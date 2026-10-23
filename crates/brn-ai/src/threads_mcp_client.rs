use super::*;
use rig::tool::{DynamicTool, ToolContext, ToolErrorKind};
use rmcp::{
    ClientHandler, RoleClient, ServiceExt,
    model::*,
    service::{NotificationContext, PeerRequestOptions, ServerSink},
    transport::{
        common::client_side_sse::NeverRetry,
        streamable_http_client::{
            StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
        },
    },
};
use serde::Serialize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub const DEADLINE: Duration = Duration::from_secs(5);
#[derive(Clone)]
struct Handler {
    changed: Arc<AtomicBool>,
    #[cfg(test)]
    retired: Arc<tokio::sync::Notify>,
}
impl ClientHandler for Handler {
    async fn on_tool_list_changed(&self, _: NotificationContext<RoleClient>) {
        // Retire the entire frozen snapshot. Never install notification-driven
        // definitions or let an SDK registry broaden the approved catalogue.
        self.changed.store(true, Ordering::SeqCst);
        #[cfg(test)]
        self.retired.notify_one();
    }
    fn get_info(&self) -> ClientConfig {
        let mut info = ClientConfig::default();
        info.client_info = Implementation::new("brn", env!("CARGO_PKG_VERSION"));
        info
    }
}
/// Bounded owner-facing metadata; it is never a runtime system instruction.
#[derive(Clone, Serialize)]
pub struct Discovery {
    pub endpoint: String,
    pub server_digest: String,
    pub tools: Vec<DiscoveredTool>,
}
#[derive(Clone, Serialize)]
pub struct DiscoveredTool {
    pub name: String,
    pub metadata_digest: String,
    pub schema_digest: Option<String>,
    pub supported: bool,
    pub metadata: Value,
}
struct CheckedChannel {
    owner: Arc<dyn http::DispatchGate>,
    changed: Arc<AtomicBool>,
    active: std::sync::Mutex<Option<RequestId>>,
}
impl http::DispatchGate for CheckedChannel {
    fn message(&self, message: &ClientJsonRpcMessage) -> Result<(), Refusal> {
        let cancelling = matches!(message,JsonRpcMessage::Notification(n) if matches!(n.notification,ClientNotification::CancelledNotification(_)));
        if self.changed.load(Ordering::SeqCst) && !cancelling {
            return Err(Refusal::Unavailable);
        }
        self.owner.message(message)?;
        if let JsonRpcMessage::Request(request) = message
            && matches!(request.request, ClientRequest::CallToolRequest(_))
        {
            *self.active.lock().map_err(|_| Refusal::Unavailable)? = Some(request.id.clone());
        }
        Ok(())
    }
    fn channel(&self) -> Result<(), Refusal> {
        if self.changed.load(Ordering::SeqCst) {
            return Err(Refusal::Unavailable);
        }
        self.owner.channel()
    }
}
pub struct Connection {
    #[cfg(test)]
    overflow: Arc<std::sync::Mutex<Option<http::RawOverflow>>>,
    #[cfg(test)]
    retired: Arc<tokio::sync::Notify>,
    endpoint: Endpoint,
    channel: Arc<CheckedChannel>,
    changed: Arc<AtomicBool>,
    peer: ServerSink,
    sdk: rig::tool::rmcp::McpConnection,
    discovery: Discovery,
    advertised: Vec<McpToolDefinition>,
    serial: Arc<tokio::sync::Semaphore>,
    active: AtomicUsize,
    drained: tokio::sync::Notify,
}
impl Connection {
    pub async fn connect(
        endpoint: Endpoint,
        gate: Arc<dyn http::DispatchGate>,
    ) -> Result<Arc<Self>, Refusal> {
        let changed = Arc::new(AtomicBool::new(false));
        let channel = Arc::new(CheckedChannel {
            owner: gate,
            changed: changed.clone(),
            active: std::sync::Mutex::new(None),
        });
        let client = http::PolicyHttpClient::new(endpoint.clone(), channel.clone())?;
        #[cfg(test)]
        let overflow = client.overflow.clone();
        let mut config = StreamableHttpClientTransportConfig::with_uri(endpoint.uri().to_owned());
        config.retry_config = Arc::new(NeverRetry::default());
        config.reinit_on_expired_session = false;
        config.channel_buffer_capacity = 2;
        let transport = StreamableHttpClientTransport::with_client(client, config);
        #[cfg(test)]
        let retired = Arc::new(tokio::sync::Notify::new());
        let handler = Handler {
            changed: changed.clone(),
            #[cfg(test)]
            retired: retired.clone(),
        };
        let service = tokio::time::timeout(DEADLINE, handler.serve(transport))
            .await
            .map_err(|_| Refusal::Timeout)?
            .map_err(|_error| {
                #[cfg(test)]
                http::setup_diagnostic("handshake", &_error);
                Refusal::Transport
            })?;
        let peer = service.peer().clone();
        let result = async {
            let info = peer.peer_info().ok_or(Refusal::Catalogue)?;
            let bytes = serde_json::to_vec(&*info).map_err(|_| Refusal::Catalogue)?;
            if bytes.len() > 8192 {
                return Err(Refusal::Catalogue);
            }
            let server_digest = digest(b"brn-mcp-server-v1\0", &bytes);
            let request = ClientRequest::ListToolsRequest(ListToolsRequest::default());
            let handle = peer
                .send_request_with_option(
                    request,
                    PeerRequestOptions::with_timeout(DEADLINE).with_max_total_timeout(DEADLINE),
                )
                .await
                .map_err(|_error| {
                    #[cfg(test)]
                    http::setup_diagnostic("list_send", &_error);
                    Refusal::Transport
                })?;
            let response = handle.await_response().await.map_err(|_error| {
                #[cfg(test)]
                http::setup_diagnostic("list_response", &_error);
                Refusal::Transport
            })?;
            let ServerResult::ListToolsResult(list) = response else {
                return Err(Refusal::Catalogue);
            };
            if list.next_cursor.is_some()
                || list.tools.len() > MAX_TOOLS
                || serde_json::to_vec(&list.tools)
                    .map_err(|_| Refusal::Catalogue)?
                    .len()
                    > MAX_CATALOGUE_BYTES
                || changed.load(Ordering::SeqCst)
            {
                return Err(Refusal::Catalogue);
            }
            let mut names = std::collections::BTreeSet::new();
            let mut tools = Vec::new();
            for tool in &list.tools {
                if !names.insert(tool.name.to_string()) {
                    return Err(Refusal::Catalogue);
                }
                let metadata_digest = metadata_digest(tool)?;
                let shape = input_shape(&tool.schema_as_json_value()).ok();
                let eligible = shape.is_some()
                    && tool.annotations.as_ref().is_some_and(|a| {
                        a.read_only_hint == Some(true) && a.destructive_hint != Some(true)
                    });
                tools.push(DiscoveredTool {
                    name: tool.name.to_string(),
                    metadata_digest,
                    schema_digest: shape.map(brn_threads_core::mcp_input_schema_digest),
                    supported: eligible,
                    metadata: serde_json::to_value(tool).map_err(|_| Refusal::Catalogue)?,
                });
            }
            Ok((
                Discovery {
                    endpoint: endpoint.uri().to_owned(),
                    server_digest,
                    tools,
                },
                list.tools,
            ))
        };
        let result = tokio::time::timeout(DEADLINE + Duration::from_secs(1), result).await;
        let (discovery, advertised) = match result {
            Ok(Ok(value)) => value,
            other => {
                let _ = tokio::time::timeout(Duration::from_secs(1), service.cancel()).await;
                return Err(match other {
                    Ok(Err(e)) => e,
                    _ => Refusal::Timeout,
                });
            }
        };
        Ok(Arc::new(Self {
            #[cfg(test)]
            overflow,
            #[cfg(test)]
            retired,
            endpoint,
            channel,
            changed,
            peer,
            sdk: rig::tool::rmcp::McpConnection::new(service),
            discovery,
            advertised,
            serial: Arc::new(tokio::sync::Semaphore::new(1)),
            active: AtomicUsize::new(0),
            drained: tokio::sync::Notify::new(),
        }))
    }
    #[cfg(test)]
    pub(super) fn raw_overflow(&self) -> Option<http::RawOverflow> {
        *self.overflow.lock().unwrap()
    }
    #[cfg(test)]
    pub(super) async fn observed_retirement(&self) {
        self.retired.notified().await;
    }
    pub fn discovery(&self) -> &Discovery {
        &self.discovery
    }
    pub fn freeze(&self, policy: &brn_threads_core::McpPolicy) -> Result<Vec<FrozenTool>, Refusal> {
        if !self.is_live()
            || self.endpoint.uri() != policy.endpoint
            || self.discovery.server_digest != policy.server_digest
        {
            return Err(Refusal::Catalogue);
        }
        let approved = policy
            .tools
            .iter()
            .map(|a| ApprovedTool {
                name: a.name.clone(),
                digest: a.metadata_digest.clone(),
            })
            .collect::<Vec<_>>();
        let frozen = freeze_tools(&self.endpoint, &self.advertised, &approved)?;
        if frozen.iter().zip(&policy.tools).any(|(f, p)| {
            f.shape != p.contract
                || brn_threads_core::mcp_input_schema_digest(f.shape) != p.schema_digest
        }) {
            return Err(Refusal::Catalogue);
        }
        Ok(frozen)
    }
    pub fn is_live(&self) -> bool {
        !self.changed.load(Ordering::SeqCst) && !self.peer.is_transport_closed()
    }
    /// Keep the real Rig adapter future polled even if its caller stops waiting.
    /// The SDK owns its absolute deadline and explicit cancellation; this task
    /// never returns a late accepted result after cancellation or retirement.
    pub async fn invoke(
        self: &Arc<Self>,
        tool: &FrozenTool,
        args: Value,
        cancel: CancellationToken,
    ) -> Result<Value, Refusal> {
        validate_arguments(tool.shape, &args)?;
        if cancel.is_cancelled() || !self.is_live() {
            return Err(Refusal::Unavailable);
        }
        let permit = self
            .serial
            .clone()
            .try_acquire_owned()
            .map_err(|_| Refusal::Unavailable)?;
        let connection = self.clone();
        let tool = tool.clone();
        let started = tokio::time::Instant::now();
        self.active.fetch_add(1, Ordering::SeqCst);
        let guard = ActiveCall(self.clone());
        tokio::spawn(async move {
            let _guard = guard;
            if cancel.is_cancelled() || !connection.is_live() {
                return Err(Refusal::Unavailable);
            }
            let remaining = DEADLINE
                .checked_sub(started.elapsed())
                .ok_or(Refusal::Timeout)?;
            let adapter: DynamicTool =
                rig::tool::rmcp::McpTool::new(tool.definition.clone(), &connection.sdk)
                    .with_timeout(remaining)
                    .try_into()
                    .map_err(|_| Refusal::Catalogue)?;
            let mut context = ToolContext::new();
            let output = {
                let call = adapter.run_with(&mut context, args);
                tokio::pin!(call);
                tokio::select! {
                    result = call.as_mut() => result,
                    _ = cancel.cancelled() => {
                        connection.cancel_active().await;
                        // Keep Rig's SDK deadline/cancellation future polled.
                        let _ = call.as_mut().await;
                        return Err(Refusal::Cancelled);
                    }
                }
            };
            drop(permit);
            if cancel.is_cancelled() {
                return Err(Refusal::Cancelled);
            }
            if !connection.is_live() {
                return Err(Refusal::Unavailable);
            }
            if let Err(error) = output {
                if error.kind() == ToolErrorKind::Timeout {
                    connection.cancel_active().await;
                    return Err(Refusal::Timeout);
                }
                return Err(Refusal::Transport);
            }
            let raw = context
                .result::<rig::tool::rmcp::McpCallToolResult>()
                .map_err(|_| Refusal::InvalidResult)?
                .ok_or(Refusal::InvalidResult)?
                .0;
            if raw.is_error == Some(true)
                || raw.structured_content.is_some()
                || raw.content.is_empty()
                || raw.content.len() > 8
            {
                return Err(Refusal::InvalidResult);
            }
            let mut text = String::new();
            for content in &raw.content {
                let part = content.as_text().ok_or(Refusal::InvalidResult)?;
                let separator = usize::from(!text.is_empty());
                if part.text.len() + separator > MAX_TEXT_BYTES - text.len() {
                    return Err(Refusal::ResultLimit);
                }
                if separator != 0 {
                    text.push('\n');
                }
                text.push_str(&part.text);
            }
            if text.trim().is_empty() {
                return Err(Refusal::InvalidResult);
            }
            let identity = digest(b"brn-mcp-result-v1\0", text.as_bytes());
            Ok(serde_json::json!({
                "kind":"untrusted_external_data", "connection":connection.endpoint.identity(),
                "tool_metadata":tool.metadata_digest, "source_identity":identity,
                "coverage":"complete_tool_result_not_whole_source", "text":text,
            }))
        })
        .await
        .map_err(|_| Refusal::Transport)?
    }
    async fn cancel_active(&self) {
        let active = self.channel.active.lock().ok().and_then(|id| id.clone());
        if let Some(id) = active {
            let request = self.peer.notify_cancelled(CancelledNotificationParam::new(
                Some(id),
                Some("BRN request ended; remote processing may have occurred".into()),
            ));
            let _ = tokio::time::timeout(Duration::from_secs(1), request).await;
        }
    }
    /// Drain supervised SDK deadlines before the host worker runtime exits.
    pub async fn drain(&self) {
        let wait = async {
            loop {
                let changed = self.drained.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if self.active.load(Ordering::SeqCst) == 0 {
                    break;
                }
                changed.await;
            }
        };
        let _ = tokio::time::timeout(DEADLINE + Duration::from_secs(2), wait).await;
    }
    pub async fn disconnect(&self) {
        self.changed.store(true, Ordering::SeqCst);
        let _ = tokio::time::timeout(Duration::from_secs(1), self.sdk.close()).await;
    }
}

struct ActiveCall(Arc<Connection>);
impl Drop for ActiveCall {
    fn drop(&mut self) {
        self.0.active.fetch_sub(1, Ordering::SeqCst);
        self.0.drained.notify_waiters();
    }
}
