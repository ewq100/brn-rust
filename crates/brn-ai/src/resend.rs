//! One identical re-send of a streamed model request whose first send was
//! provably not processed: the connection was never established, or the
//! provider answered 503 (unavailable, the request was not served). The same
//! request goes to the same transport, so provider, model, account, session
//! and body are unchanged; there is no fallback. Failures after the request
//! may have reached the model (read timeouts, resets, other statuses, a
//! started stream) are never re-sent, because the provider may already be
//! generating and billing; no tool of the failed turn has run either way.
use crate::AiFailureDetail;
use crate::error::transport_detail;
use rig::http_client::{
    self, DynHttpClient, HttpClientExt, LazyBody, MultipartForm, Request, Response,
    StreamingResponse,
};
use rig_core::wasm_compat::WasmCompatSend;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;
use tokio_util::bytes::Bytes;

/// Re-sends allowed in one run.
pub(crate) const RUN_RESENDS: u8 = 2;
/// Pause before a re-send.
pub(crate) const RESEND_DELAY: Duration = if cfg!(test) {
    Duration::ZERO
} else {
    Duration::from_millis(1500)
};

pub(crate) type OnResend = Arc<dyn Fn(AiFailureDetail) + Send + Sync>;

pub(crate) struct ResendOnce {
    inner: DynHttpClient,
    delay: Duration,
    budget: Arc<AtomicU8>,
    on_resend: OnResend,
}

impl ResendOnce {
    pub(crate) fn wrap(
        inner: DynHttpClient,
        delay: Duration,
        on_resend: OnResend,
    ) -> DynHttpClient {
        DynHttpClient::new(Self {
            inner,
            delay,
            budget: Arc::new(AtomicU8::new(RUN_RESENDS)),
            on_resend,
        })
    }
}

/// Only failures where the request provably was not processed: a 503, or a
/// transport error whose reqwest source failed while connecting (DNS, TCP or
/// TLS), before any request byte reached the provider. 408, 500, 502 and 504
/// can follow a request the model already started, so they are not re-sent.
fn resendable(error: &http_client::Error) -> bool {
    match error {
        http_client::Error::InvalidStatusCodeWithDetails { status, .. } => status.as_u16() == 503,
        http_client::Error::Instance(source) => connect_failure(source.as_ref()),
        _ => false,
    }
}

fn connect_failure(error: &(dyn std::error::Error + 'static)) -> bool {
    let mut next = Some(error);
    while let Some(error) = next {
        if let Some(error) = error.downcast_ref::<rig::rig_reqwest::reqwest::Error>() {
            return error.is_connect();
        }
        next = error.source();
    }
    false
}

fn copy(original: &Request<Bytes>) -> Request<Bytes> {
    let mut request = Request::new(original.body().clone());
    *request.method_mut() = original.method().clone();
    *request.uri_mut() = original.uri().clone();
    *request.version_mut() = original.version();
    *request.headers_mut() = original.headers().clone();
    request
}

impl HttpClientExt for ResendOnce {
    fn send<T, U>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        T: Into<Bytes> + WasmCompatSend,
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        self.inner.send(req)
    }

    fn send_multipart<U>(
        &self,
        req: Request<MultipartForm>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + WasmCompatSend + 'static
    where
        U: From<Bytes> + WasmCompatSend + 'static,
    {
        self.inner.send_multipart(req)
    }

    fn send_streaming<T>(
        &self,
        req: Request<T>,
    ) -> impl Future<Output = http_client::Result<StreamingResponse>> + WasmCompatSend
    where
        T: Into<Bytes> + WasmCompatSend,
    {
        let req: Request<Bytes> = req.map(Into::into);
        async move {
            let again = copy(&req);
            let error = match self.inner.send_streaming(req).await {
                Ok(response) => return Ok(response),
                Err(error) => error,
            };
            if !resendable(&error)
                || self
                    .budget
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
                    .is_err()
            {
                return Err(error);
            }
            (self.on_resend)(transport_detail(&error));
            tokio::time::sleep(self.delay).await;
            self.inner.send_streaming(again).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig::test_utils::{MockHttpResponse, SequencedHttpClient};
    use std::sync::Mutex;

    fn request() -> Request<Bytes> {
        Request::builder()
            .method("POST")
            .uri("https://example.invalid/responses")
            .header("session_id", "brn-synthetic")
            .body(Bytes::from_static(b"{\"input\":\"synthetic\"}"))
            .unwrap()
    }

    fn client(
        replies: Vec<MockHttpResponse>,
    ) -> (
        SequencedHttpClient,
        DynHttpClient,
        Arc<Mutex<Vec<AiFailureDetail>>>,
    ) {
        let http = SequencedHttpClient::new(replies);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        let wrapped = ResendOnce::wrap(
            DynHttpClient::new(http.clone()),
            Duration::ZERO,
            Arc::new(move |d| log.lock().unwrap().push(d)),
        );
        (http, wrapped, seen)
    }

    fn ok() -> MockHttpResponse {
        MockHttpResponse::success_typed("data: [DONE]\n\n", "text/event-stream")
    }

    fn status(code: u16) -> MockHttpResponse {
        MockHttpResponse::error(
            http_client::StatusCode::from_u16(code).unwrap(),
            r#"{"error":{"type":"server_error","message":"SYNTHETIC_SECRET"}}"#,
        )
    }

    #[tokio::test]
    async fn a_transient_status_is_resent_once_byte_for_byte_and_recorded() {
        let (http, wrapped, seen) = client(vec![status(503), ok()]);
        assert!(wrapped.send_streaming(request()).await.is_ok());
        let sent = http.requests();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0].uri, sent[1].uri);
        assert_eq!(sent[0].body, sent[1].body);
        assert_eq!(sent[0].headers, sent[1].headers);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            (
                seen[0].origin.as_str(),
                seen[0].status,
                seen[0].error_type.as_deref()
            ),
            ("http_status", Some(503), Some("server_error"))
        );
        assert!(!format!("{seen:?}").contains("SYNTHETIC_SECRET"));
    }

    #[tokio::test]
    async fn a_second_transient_failure_of_the_same_request_is_returned() {
        let (http, wrapped, seen) = client(vec![status(503), status(503), ok()]);
        let error = wrapped.send_streaming(request()).await.err().unwrap();
        assert!(matches!(
            error,
            http_client::Error::InvalidStatusCodeWithDetails { status, .. } if status == 503
        ));
        assert_eq!(http.requests().len(), 2);
        assert_eq!(seen.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn rejections_and_limits_are_never_resent() {
        for code in [400, 401, 403, 404, 408, 413, 429, 500, 501, 502, 504] {
            let (http, wrapped, seen) = client(vec![status(code), ok()]);
            assert!(wrapped.send_streaming(request()).await.is_err(), "{code}");
            assert_eq!(http.requests().len(), 1, "{code}");
            assert!(seen.lock().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn a_run_resends_at_most_its_budget() {
        let (http, wrapped, seen) = client(vec![
            status(503),
            ok(),
            status(503),
            ok(),
            status(503),
            ok(),
        ]);
        assert!(wrapped.send_streaming(request()).await.is_ok());
        assert!(wrapped.send_streaming(request()).await.is_ok());
        assert!(wrapped.send_streaming(request()).await.is_err());
        assert_eq!(http.requests().len(), 5);
        assert_eq!(seen.lock().unwrap().len(), usize::from(RUN_RESENDS));
    }

    fn real_transport() -> (DynHttpClient, Arc<Mutex<Vec<AiFailureDetail>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        let client = rig::rig_reqwest::reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_millis(300))
            .build()
            .unwrap();
        let wrapped = ResendOnce::wrap(
            DynHttpClient::new(rig::rig_reqwest::ReqwestClient::from(client)),
            Duration::ZERO,
            Arc::new(move |d| log.lock().unwrap().push(d)),
        );
        (wrapped, seen)
    }

    fn to(uri: String) -> Request<Bytes> {
        let mut request = request();
        *request.uri_mut() = uri.parse().unwrap();
        request
    }

    /// A loopback listener that accepts `connections` and lets `serve` treat
    /// each one; it reports how many requests arrived.
    fn listener(serve: fn(std::net::TcpStream)) -> (String, Arc<Mutex<usize>>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let uri = format!("http://{}/responses", listener.local_addr().unwrap());
        let count = Arc::new(Mutex::new(0));
        let seen = count.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                *seen.lock().unwrap() += 1;
                serve(stream);
            }
        });
        (uri, count)
    }

    fn read_request(stream: &mut std::net::TcpStream) {
        use std::io::Read;
        let mut buffer = [0u8; 4096];
        let _ = stream.read(&mut buffer);
    }

    #[tokio::test]
    async fn a_refused_connection_is_resent_once() {
        let port = {
            let unused = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            unused.local_addr().unwrap().port()
        };
        let (wrapped, seen) = real_transport();
        let error = wrapped
            .send_streaming(to(format!("http://127.0.0.1:{port}/responses")))
            .await
            .err()
            .unwrap();
        assert!(resendable(&error));
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(
            (seen[0].origin.as_str(), seen[0].code.as_deref()),
            ("transport", Some("instance"))
        );
    }

    #[tokio::test]
    async fn a_reset_after_the_request_was_sent_is_not_resent() {
        let (uri, arrived) = listener(|mut stream| {
            read_request(&mut stream);
            drop(stream);
        });
        let (wrapped, seen) = real_transport();
        let error = wrapped.send_streaming(to(uri)).await.err().unwrap();
        assert!(!resendable(&error));
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(*arrived.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn a_read_timeout_after_the_request_was_sent_is_not_resent() {
        let (uri, arrived) = listener(|mut stream| {
            read_request(&mut stream);
            std::thread::sleep(Duration::from_secs(2));
            drop(stream);
        });
        let (wrapped, seen) = real_transport();
        let error = wrapped.send_streaming(to(uri)).await.err().unwrap();
        assert!(!resendable(&error));
        assert!(seen.lock().unwrap().is_empty());
        assert_eq!(*arrived.lock().unwrap(), 1);
    }
}
