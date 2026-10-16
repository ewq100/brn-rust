use rig::{
    tool::rmcp::{McpClientHandler, McpTool},
    tool::{DynamicTool, ToolContext},
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
            context.ct.cancelled().await;
            self.cancelled.store(true, Ordering::SeqCst);
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
    assert!(error.is_error());
    assert!(
        error.output().render().to_lowercase().contains("tim"),
        "{error:?}"
    );
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
