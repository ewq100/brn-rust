//! Policy adapter at rmcp's supported HTTP-client seam. rmcp continues to own
//! JSON-RPC types, stream parsing, sessions, request handles and cancellation.
//! Unlike its stock reqwest adapter, bytes are bounded before typed decoding.
use super::{Endpoint, MAX_ARGUMENT_BYTES, MAX_WIRE_BYTES, Refusal};
use futures::StreamExt;
use rig_reqwest::reqwest::{
    self, Method, Response,
    header::{HeaderName, HeaderValue},
};
use rmcp::{
    model::{ClientJsonRpcMessage, JsonRpcMessage, ServerJsonRpcMessage},
    transport::{
        common::client_side_sse::BoxedSseResponse,
        streamable_http_client::{
            StreamableHttpClient, StreamableHttpError, StreamableHttpPostResponse,
        },
    },
};
use std::{collections::HashMap, sync::Arc, time::Duration};

// Error classes only; never format an error or retain its payload/destination.
#[cfg(test)]
pub(super) fn setup_diagnostic(stage: &'static str, error: &(dyn std::error::Error + 'static)) {
    use std::io::Write as _;
    let mut source = Some(error);
    let mut reqwest = None;
    let mut os = None;
    for _ in 0..8 {
        let Some(error) = source else { break };
        if let Some(error) = error.downcast_ref::<reqwest::Error>() {
            reqwest = Some((
                error.is_connect(),
                error.is_timeout(),
                error.is_body(),
                error.is_request(),
            ));
        }
        if let Some(error) = error.downcast_ref::<std::io::Error>() {
            os = error.raw_os_error();
        }
        source = error.source();
    }
    let (connect, timeout, body, request) = reqwest.unwrap_or_default();
    let _ = writeln!(
        std::io::stderr().lock(),
        "BRN SDK test setup: stage={stage} reqwest={} connect={connect} timeout={timeout} body={body} request={request} os_present={} os={}",
        reqwest.is_some(),
        os.is_some(),
        os.unwrap_or(0)
    );
}

/// Implemented by the checked BRN host. A model cannot deserialize this gate.
/// It must check current policy/fence/context and exact reviewed tool input at
/// the last synchronous boundary before each actual HTTP execute future.
pub trait DispatchGate: Send + Sync + 'static {
    fn message(&self, message: &ClientJsonRpcMessage) -> Result<(), Refusal>;
    fn channel(&self) -> Result<(), Refusal>;
}
#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) struct RawOverflow {
    pub(super) at: std::time::Instant,
    pub(super) admitted_bytes: usize,
    pub(super) rejected_chunk_bytes: usize,
}
#[cfg(test)]
type OverflowObserver = Arc<std::sync::Mutex<Option<RawOverflow>>>;
#[derive(Clone)]
pub struct PolicyHttpClient {
    client: reqwest::Client,
    endpoint: Endpoint,
    gate: Arc<dyn DispatchGate>,
    #[cfg(test)]
    pub(super) overflow: OverflowObserver,
}
impl PolicyHttpClient {
    pub fn new(endpoint: Endpoint, gate: Arc<dyn DispatchGate>) -> Result<Self, Refusal> {
        let client = reqwest::Client::builder()
            .no_proxy()
            .retry(reqwest::retry::never())
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|_| Refusal::Transport)?;
        Ok(Self {
            client,
            endpoint,
            gate,
            #[cfg(test)]
            overflow: Arc::default(),
        })
    }
    fn request(
        &self,
        method: Method,
        uri: &str,
        session: Option<&str>,
        auth: Option<&str>,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<reqwest::RequestBuilder, StreamableHttpError<Refusal>> {
        if uri != self.endpoint.uri() || auth.is_some() {
            return Err(StreamableHttpError::Client(Refusal::Endpoint));
        }
        let mut request = self.client.request(method, self.endpoint.uri()).header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        );
        if let Some(session) = session {
            if session.is_empty()
                || session.len() > 256
                || !session.bytes().all(|b| b.is_ascii_graphic())
            {
                return Err(StreamableHttpError::Client(Refusal::Transport));
            }
            request = request.header("mcp-session-id", session);
        }
        for (name, value) in headers {
            // Only the SDK's negotiated protocol header. No credentials or
            // model/server-chosen generic custom headers cross this boundary.
            if name.as_str() != "mcp-protocol-version"
                || value.as_bytes().len() > 32
                || !value
                    .as_bytes()
                    .iter()
                    .all(|b| b.is_ascii_digit() || *b == b'-')
            {
                return Err(StreamableHttpError::Client(Refusal::Transport));
            }
            request = request.header(name, value);
        }
        Ok(request)
    }
    fn session(response: &Response) -> Result<Option<String>, StreamableHttpError<Refusal>> {
        response
            .headers()
            .get("mcp-session-id")
            .map(|h| {
                let s = h
                    .to_str()
                    .map_err(|_| StreamableHttpError::Client(Refusal::Transport))?;
                if s.is_empty() || s.len() > 256 || !s.bytes().all(|b| b.is_ascii_graphic()) {
                    return Err(StreamableHttpError::Client(Refusal::Transport));
                }
                Ok(s.to_owned())
            })
            .transpose()
    }
    fn response_policy(response: &Response) -> Result<(), StreamableHttpError<Refusal>> {
        if response.headers().contains_key(reqwest::header::LOCATION)
            || response
                .headers()
                .contains_key(reqwest::header::CONTENT_LOCATION)
            || response
                .headers()
                .get(reqwest::header::CONTENT_ENCODING)
                .is_some_and(|h| h.as_bytes() != b"identity")
        {
            return Err(StreamableHttpError::Client(Refusal::InvalidResult));
        }
        Ok(())
    }
    fn content_type(response: &Response) -> Result<&str, StreamableHttpError<Refusal>> {
        Self::response_policy(response)?;
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').next())
            .map(str::trim)
            .ok_or(StreamableHttpError::Client(Refusal::InvalidResult))
    }
    async fn bytes(response: Response) -> Result<Vec<u8>, StreamableHttpError<Refusal>> {
        if response
            .content_length()
            .is_some_and(|n| n > MAX_WIRE_BYTES as u64)
        {
            return Err(StreamableHttpError::Client(Refusal::ResultLimit));
        }
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| StreamableHttpError::Client(Refusal::Transport))?;
            if chunk.len() > MAX_WIRE_BYTES - bytes.len() {
                return Err(StreamableHttpError::Client(Refusal::ResultLimit));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
    fn stream(
        response: Response,
        #[cfg(test)] overflow: OverflowObserver,
    ) -> Result<BoxedSseResponse, StreamableHttpError<Refusal>> {
        if response
            .content_length()
            .is_some_and(|n| n > MAX_WIRE_BYTES as u64)
        {
            return Err(StreamableHttpError::Client(Refusal::ResultLimit));
        }
        let stream = response
            .bytes_stream()
            .scan((0usize, false), move |(used, failed), item| {
                #[cfg(test)]
                if !*failed
                    && let Ok(chunk) = &item
                    && chunk.len() > MAX_WIRE_BYTES - *used
                {
                    overflow.lock().unwrap().get_or_insert(RawOverflow {
                        at: std::time::Instant::now(),
                        admitted_bytes: *used,
                        rejected_chunk_bytes: chunk.len(),
                    });
                }
                let next = if *failed {
                    None
                } else {
                    match item {
                        Ok(chunk) if chunk.len() <= MAX_WIRE_BYTES - *used => {
                            *used += chunk.len();
                            Some(Ok(chunk))
                        }
                        _ => {
                            *failed = true;
                            Some(Err(std::io::Error::other(
                                "MCP response stream unavailable or over limit",
                            )))
                        }
                    }
                };
                futures::future::ready(next)
            });
        Ok(sse_stream::SseStream::from_bytes_stream(stream).boxed())
    }
}
impl StreamableHttpClient for PolicyHttpClient {
    type Error = Refusal;
    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session: Option<Arc<str>>,
        auth: Option<String>,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        let notification = matches!(message, JsonRpcMessage::Notification(_));
        let request = self.request(
            Method::POST,
            &uri,
            session.as_deref(),
            auth.as_deref(),
            headers,
        )?;
        let body = serde_json::to_vec(&message)
            .map_err(|_| StreamableHttpError::Client(Refusal::Arguments))?;
        // Allow bounded SDK envelopes in addition to the separately bounded
        // tool arguments; no arbitrary history or assets are transport inputs.
        if body.len() > MAX_ARGUMENT_BYTES + 4096 {
            return Err(StreamableHttpError::Client(Refusal::Arguments));
        }
        let request = request
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .build()
            .map_err(|_| StreamableHttpError::Client(Refusal::Transport))?;
        self.gate
            .message(&message)
            .map_err(StreamableHttpError::Client)?;
        let response = self.client.execute(request).await.map_err(|_error| {
            #[cfg(test)]
            setup_diagnostic("post_execute", &_error);
            StreamableHttpError::Client(Refusal::Transport)
        })?;
        Self::response_policy(&response)?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND && session.is_some() {
            return Err(StreamableHttpError::SessionExpired);
        }
        // Never retain/display an untrusted error body or authentication header.
        if !status.is_success() {
            return Err(StreamableHttpError::Client(Refusal::Transport));
        }
        if status == reqwest::StatusCode::ACCEPTED || status == reqwest::StatusCode::NO_CONTENT {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        let session = Self::session(&response)?;
        match Self::content_type(&response)? {
            "application/json" => {
                let bytes = Self::bytes(response).await?;
                if notification && bytes.is_empty() {
                    return Ok(StreamableHttpPostResponse::Accepted);
                }
                let message: ServerJsonRpcMessage = serde_json::from_slice(&bytes)
                    .map_err(|_| StreamableHttpError::Client(Refusal::InvalidResult))?;
                Ok(StreamableHttpPostResponse::Json(message, session))
            }
            "text/event-stream" => Ok(StreamableHttpPostResponse::Sse(
                Self::stream(
                    response,
                    #[cfg(test)]
                    self.overflow.clone(),
                )?,
                session,
            )),
            _ => Err(StreamableHttpError::Client(Refusal::InvalidResult)),
        }
    }
    async fn get_stream(
        &self,
        uri: Arc<str>,
        session: Arc<str>,
        last_id: Option<String>,
        auth: Option<String>,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<BoxedSseResponse, StreamableHttpError<Self::Error>> {
        // Replay/reconnection is disabled in the SDK config. Never forward a
        // server event ID in a new request; it is not a destination or grant.
        if last_id.is_some() {
            return Err(StreamableHttpError::Client(Refusal::Unavailable));
        }
        let request = self
            .request(Method::GET, &uri, Some(&session), auth.as_deref(), headers)?
            .build()
            .map_err(|_| StreamableHttpError::Client(Refusal::Transport))?;
        self.gate.channel().map_err(StreamableHttpError::Client)?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|_| StreamableHttpError::Client(Refusal::Transport))?;
        Self::response_policy(&response)?;
        if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }
        if !response.status().is_success() || Self::content_type(&response)? != "text/event-stream"
        {
            return Err(StreamableHttpError::Client(Refusal::Transport));
        }
        Self::stream(
            response,
            #[cfg(test)]
            self.overflow.clone(),
        )
    }
    async fn delete_session(
        &self,
        uri: Arc<str>,
        session: Arc<str>,
        auth: Option<String>,
        headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        // Exact-destination cleanup carries no reviewed input and remains
        // available after revocation. It never claims remote work was undone.
        let request = self
            .request(
                Method::DELETE,
                &uri,
                Some(&session),
                auth.as_deref(),
                headers,
            )?
            .build()
            .map_err(|_| StreamableHttpError::Client(Refusal::Transport))?;
        let response = self
            .client
            .execute(request)
            .await
            .map_err(|_| StreamableHttpError::Client(Refusal::Transport))?;
        Self::response_policy(&response)?;
        if response.status().is_success()
            || response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED
        {
            Ok(())
        } else {
            Err(StreamableHttpError::Client(Refusal::Transport))
        }
    }
}
