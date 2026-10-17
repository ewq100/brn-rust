//! Test-only maintained SDK server for actual CLI process qualification.
use rmcp::{
    model::*,
    service::RequestContext,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    RoleServer, ServerHandler,
};
use serde_json::Value;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tokio_util::sync::CancellationToken;
#[derive(Default)]
struct State {
    requests: AtomicUsize,
    calls: AtomicUsize,
    credential_headers: AtomicUsize,
    inputs: Mutex<Vec<Value>>,
}
#[derive(Clone)]
struct Handler(Arc<State>);
impl ServerHandler for Handler {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::new(ServerCapabilities::builder().enable_tools().build());
        info.server_info.version = "synthetic-cli-1".into();
        info
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult {
            tools: vec![Tool::new(
                "public_read",
                "Read one synthetic public fixture",
                brn_ai::mcp::input_schema(brn_ai::mcp::InputShape::Query)
                    .as_object()
                    .unwrap()
                    .clone(),
            )
            .with_annotations(ToolAnnotations::new().read_only(true).destructive(false))],
            ..Default::default()
        })
    }
    async fn call_tool(
        &self,
        params: CallToolRequestParams,
        _: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.0.calls.fetch_add(1, Ordering::SeqCst);
        let input = params.arguments.map(Value::Object).unwrap_or(Value::Null);
        self.0.inputs.lock().unwrap().push(input);
        if params.name != "public_read" {
            return Err(ErrorData::invalid_params("Unknown synthetic tool", None));
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(
            "PUBLIC_RAW_MCP_RESPONSE_CANARY https://example.invalid/mcp-process: source value 23.",
        )]))
    }
}
pub struct Server {
    pub endpoint: String,
    state: Arc<State>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    finished: mpsc::Receiver<()>,
    task: Option<thread::JoinHandle<()>>,
}
impl Server {
    pub fn start() -> Self {
        let state = Arc::new(State::default());
        let worker = state.clone();
        let (ready, address) = mpsc::sync_channel(1);
        let (done, finished) = mpsc::sync_channel(1);
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let task = thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let cancellation = CancellationToken::new();
                let mut config = StreamableHttpServerConfig::default();
                config.cancellation_token = cancellation.clone();
                let handler = worker.clone();
                let service = StreamableHttpService::new(
                    move || Ok(Handler(handler.clone())),
                    Arc::new(LocalSessionManager::default()),
                    config,
                );
                let router = axum::Router::new().nest_service("/mcp", service).layer(
                    axum::middleware::from_fn(
                        move |request: axum::extract::Request, next: axum::middleware::Next| {
                            let state = worker.clone();
                            async move {
                                state.requests.fetch_add(1, Ordering::SeqCst);
                                if request.headers().contains_key("authorization")
                                    || request.headers().contains_key("cookie")
                                {
                                    state.credential_headers.fetch_add(1, Ordering::SeqCst);
                                }
                                next.run(request).await
                            }
                        },
                    ),
                );
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                ready
                    .send(format!("http://{}/mcp", listener.local_addr().unwrap()))
                    .unwrap();
                axum::serve(listener, router)
                    .with_graceful_shutdown(async move {
                        let _ = stopped.await;
                        cancellation.cancel();
                    })
                    .await
                    .unwrap();
            });
            drop(runtime);
            done.send(()).unwrap();
        });
        let endpoint = address
            .recv_timeout(Duration::from_secs(5))
            .expect("bounded SDK readiness");
        Self {
            endpoint,
            state,
            stop: Some(stop),
            finished,
            task: Some(task),
        }
    }
    pub fn requests(&self) -> usize {
        self.state.requests.load(Ordering::SeqCst)
    }
    pub fn calls(&self) -> usize {
        self.state.calls.load(Ordering::SeqCst)
    }
    pub fn credential_headers(&self) -> usize {
        self.state.credential_headers.load(Ordering::SeqCst)
    }
    pub fn inputs(&self) -> Vec<Value> {
        self.state.inputs.lock().unwrap().clone()
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.finished
            .recv_timeout(Duration::from_secs(5))
            .expect("bounded SDK server shutdown");
        self.task.take().unwrap().join().unwrap();
    }
}
