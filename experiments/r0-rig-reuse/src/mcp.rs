use rig::{
    tool::rmcp::{McpClientHandler, McpTool},
    tool::{DynamicTool, ToolContext, ToolErrorKind},
};
use rmcp::{RoleServer, ServerHandler, ServiceExt, model::*, service::RequestContext};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
#[derive(Clone)]
struct Server {
    generation: Arc<AtomicUsize>,
    cancelled: Arc<AtomicBool>,
    started: Arc<tokio::sync::Notify>,
    dropped: Arc<AtomicBool>,
    release: Arc<tokio::sync::Notify>,
}
fn tool(name: &str) -> Tool {
    Tool::new(
        name.to_owned(),
        "synthetic read-only",
        Arc::new(serde_json::Map::new()),
    )
}
impl ServerHandler for Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
    }
    async fn list_tools(
        &self,
        _: Option<PaginatedRequestParams>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(vec![
            tool(if self.generation.load(Ordering::SeqCst) == 0 {
                "read"
            } else {
                "new_read"
            }),
            tool("wait"),
        ]))
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        if request.name == "wait" {
            struct DropWitness(Arc<AtomicBool>);
            impl Drop for DropWitness {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::SeqCst);
                }
            }
            let _drop_witness = DropWitness(self.dropped.clone());
            self.started.notify_one();
            tokio::select! {
                _ = context.ct.cancelled() => { self.cancelled.store(true, Ordering::SeqCst); }
                _ = self.release.notified() => {}
            }
            return Ok(CallToolResult::error(vec![ContentBlock::text("cancelled")]));
        }
        Ok(CallToolResult::success(vec![ContentBlock::text(
            "synthetic source",
        )]))
    }
}
#[tokio::test]
async fn conversion_timeout_cancellation_and_list_change_refresh() {
    let generation = Arc::new(AtomicUsize::new(0));
    let cancelled = Arc::new(AtomicBool::new(false));
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server = Server {
        generation: generation.clone(),
        cancelled: cancelled.clone(),
        started: Arc::new(tokio::sync::Notify::new()),
        dropped: Arc::new(AtomicBool::new(false)),
        release: Arc::new(tokio::sync::Notify::new()),
    };
    let (tx, rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let service = server.serve(server_io).await.unwrap();
        tx.send(service.peer().clone()).unwrap();
        service.waiting().await.unwrap();
    });
    let registry = rig::tool::server::ToolServer::new().run();
    let client = McpClientHandler::new(ClientInfo::default(), registry.clone())
        .with_timeout(Duration::from_millis(100))
        .connect(client_io)
        .await
        .unwrap();
    let server_peer = rx.await.unwrap();
    assert!(registry.static_tool_defs().iter().any(|d| d.name == "read"));
    let direct: DynamicTool = McpTool::from_mcp_server(tool("read"), client.peer().clone())
        .try_into()
        .unwrap();
    assert!(
        direct
            .execute(serde_json::json!({}))
            .await
            .unwrap()
            .render()
            .contains("synthetic source")
    );
    let mut context = ToolContext::default();
    let error = registry.execute("wait", "{}", &mut context).await;
    assert!(error.is_error_kind(ToolErrorKind::Timeout), "{error:?}");
    tokio::time::timeout(Duration::from_secs(2), async {
        while !cancelled.load(Ordering::SeqCst) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    generation.store(1, Ordering::SeqCst);
    server_peer.notify_tool_list_changed().await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let defs = registry.static_tool_defs();
            if defs.iter().any(|d| d.name == "new_read") && !defs.iter().any(|d| d.name == "read") {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        registry
            .execute("read", "{}", &mut context)
            .await
            .is_error()
    );
    client.cancel().await.unwrap();
    assert!(
        registry
            .static_tool_defs()
            .iter()
            .all(|d| d.name != "new_read")
    );
    task.abort();
    println!(
        "MCP: conversion result; timeout error; remote cancellation observed; list-change removes old/adds new; disconnect retires availability"
    );
}

#[tokio::test]
async fn local_abort_and_disconnect_do_not_guarantee_remote_cancellation() {
    let started = Arc::new(tokio::sync::Notify::new());
    let cancelled = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let release = Arc::new(tokio::sync::Notify::new());
    let server = Server {
        generation: Arc::new(AtomicUsize::new(0)),
        cancelled: cancelled.clone(),
        started: started.clone(),
        dropped: dropped.clone(),
        release: release.clone(),
    };
    let (client_io, server_io) = tokio::io::duplex(65536);
    let server_task = tokio::spawn(async move {
        let service = server.serve(server_io).await.unwrap();
        service.waiting().await.unwrap()
    });
    let registry = rig::tool::server::ToolServer::new().run();
    let client = McpClientHandler::new(ClientInfo::default(), registry.clone())
        .with_timeout(Duration::from_secs(60))
        .connect(client_io)
        .await
        .unwrap();
    let worker_registry = registry.clone();
    let call = tokio::spawn(async move {
        worker_registry
            .execute("wait", "{}", &mut ToolContext::default())
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    call.abort();
    assert!(call.await.unwrap_err().is_cancelled());
    // The server-start barrier precedes abort. This bounded observation is far
    // shorter than the 60-second deadline, so it isolates local future drop.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !cancelled.load(Ordering::SeqCst),
        "local drop sent no cancel notification"
    );
    assert!(
        !dropped.load(Ordering::SeqCst),
        "remote handler is still pending"
    );
    assert!(registry.static_tool_defs().iter().any(|d| d.name == "wait"));
    client.cancel().await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !cancelled.load(Ordering::SeqCst),
        "disconnect did not deliver request cancellation in this server"
    );
    assert!(
        !dropped.load(Ordering::SeqCst),
        "disconnected server still drains pending handler"
    );
    assert!(!server_task.is_finished());
    assert!(
        registry.static_tool_defs().is_empty(),
        "local tools retire despite remote work still pending"
    );
    // Server-controlled release is test cleanup, not a claimed Stop mechanism.
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), server_task)
        .await
        .unwrap()
        .unwrap();
    assert!(dropped.load(Ordering::SeqCst));
    assert!(!cancelled.load(Ordering::SeqCst));
    println!(
        "MCP abort: server-start barrier; local abort leaves remote pending/no cancel after 200ms (deadline 60s); explicit disconnect retires tools but no request cancel/handler termination after another 200ms; server-controlled release drains handler"
    );
}
