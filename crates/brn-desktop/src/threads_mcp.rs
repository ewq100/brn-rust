//! Isolated owner MCP view. Rendering uses refreshed bounded snapshots only.
//! Exact input is transient and consumed once at the existing prepare handoff.
use super::*;
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Stamp {
    policy: Option<McpPolicy>,
    chat: ChatTarget,
    project: Option<String>,
    records: Vec<(String, u64, bool)>,
}
pub(super) struct ReviewedInput {
    stamp: Stamp,
    snapshot: brn_ai::threads::ExternalInputSnapshot,
    prompt: String,
    pub tool: String,
    pub input: serde_json::Value,
}
enum McpViewError {
    Status(String),
    Discovery { endpoint: String, message: String },
}
impl ReviewedInput {
    pub(super) fn attach(
        self,
        prepared: brn_ai::threads::PreparedThreadRun,
    ) -> brn_ai::AiResult<brn_ai::threads::PreparedThreadRun> {
        prepared.with_external_input_at_snapshot(&self.tool, &self.input, &self.snapshot)
    }
}
pub(super) struct McpView {
    stamp: Option<Stamp>,
    pub endpoint: Entity<InputState>,
    pub query: Option<Entity<InputState>>,
    pub review_tool: Option<String>,
    pub pending: Option<ReviewedInput>,
    pub status: Option<McpStatus>,
    pub discovery: Option<brn_ai::mcp::Discovery>,
    query_subscription: Option<Subscription>,
    discovering: bool,
    error: Option<McpViewError>,
}
impl McpView {
    pub(super) fn new(window: &mut Window, cx: &mut Context<ThreadsUi>) -> Self {
        let endpoint = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("http://127.0.0.1:8077/mcp")
                .placeholder("Anonymous local HTTP endpoint")
        });
        Self {
            stamp: None,
            endpoint,
            query: None,
            review_tool: None,
            pending: None,
            status: None,
            discovery: None,
            query_subscription: None,
            discovering: false,
            error: None,
        }
    }
    fn clear_input(&mut self) {
        self.pending = None;
        self.query = None;
        self.query_subscription = None;
        self.review_tool = None;
    }
}
impl ThreadsUi {
    fn endpoint_matches(expected: &str, current: &str) -> bool {
        brn_ai::mcp::Endpoint::parse(current).is_ok_and(|endpoint| endpoint.uri() == expected)
    }

    fn discovery_matches_endpoint(&self, cx: &Context<Self>) -> bool {
        let Some(discovery) = &self.mcp_view.discovery else {
            return false;
        };
        let current = self.mcp_view.endpoint.read(cx).value().to_string();
        Self::endpoint_matches(&discovery.endpoint, &current)
    }

    pub(super) fn mcp_stamp(&self) -> Stamp {
        let mut ids = Vec::new();
        if let ChatTarget::Thread(chat) = &self.chat {
            ids.push(chat.clone());
            if let Some(project) = project_for_chat(&self.records, chat) {
                ids.push(project.id.clone());
            }
            if let Some(run) = latest_run(&self.records, &self.history, chat) {
                ids.push(run.id.clone());
            }
        }
        if !matches!(self.chat, ChatTarget::Thread(_))
            && let Some(project) = &self.selected_project
        {
            ids.push(project.clone());
        }
        Stamp {
            policy: self.settings.mcp.clone(),
            chat: self.chat.clone(),
            project: self.selected_project.clone(),
            records: ids
                .iter()
                .filter_map(|id| {
                    self.records
                        .iter()
                        .find(|r| &r.id == id)
                        .map(|r| (r.id.clone(), r.version, r.archived))
                })
                .collect(),
        }
    }
    pub(super) fn refresh_mcp(&mut self) {
        let stamp = self.mcp_stamp();
        if self.mcp_view.stamp.as_ref() == Some(&stamp) {
            return;
        }
        let cleared = self.mcp_view.pending.is_some() || self.mcp_view.query.is_some();
        self.mcp_view.clear_input();
        self.mcp_view.discovery = None;
        self.mcp_view.status = None;
        let chat = if let ChatTarget::Thread(chat) = &self.chat {
            Some(chat.as_str())
        } else {
            None
        };
        match self.workspace.mcp_status(chat) {
            Ok(mut status) => {
                if !matches!(self.chat, ChatTarget::Thread(_)) && self.selected_project.is_some() {
                    status.scope_authorized = false;
                    status.tools.clear();
                }
                self.mcp_view.status = Some(status);
                self.mcp_view.error = None;
            }
            Err(error) => {
                self.mcp_view.error = Some(McpViewError::Status(error.to_string()));
            }
        }
        self.mcp_view.stamp = Some(stamp);
        if cleared {
            self.warn("MCP input review cleared after a context, policy or run change. Authorize the exact input again if needed.");
        }
    }
    fn discover_mcp(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.mcp_view.discovering {
            return;
        }
        let endpoint_input = self.mcp_view.endpoint.read(cx);
        let endpoint = match brn_ai::mcp::Endpoint::parse(endpoint_input.value().as_ref()) {
            Ok(endpoint) => endpoint.uri().to_owned(),
            Err(error) => {
                self.fail(error.to_string());
                cx.notify();
                return;
            }
        };
        self.refresh_mcp();
        let stamp = self.mcp_view.stamp.clone();
        self.mcp_view.discovering = true;
        self.mcp_view.discovery = None;
        // One bounded control action on a background executor. No network or
        // runtime construction in render or on the GUI thread.
        let worker_endpoint = endpoint.clone();
        let work = cx.background_executor().spawn(async move {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| "Could not start local MCP discovery".to_owned())?
                .block_on(Workspace::discover_mcp(&worker_endpoint))
                .map_err(|e| e.to_string())
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = work.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.refresh();
                this.finish_mcp_discovery(stamp, &endpoint, result, cx);
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn finish_mcp_discovery(
        &mut self,
        request_stamp: Option<Stamp>,
        request_endpoint: &str,
        result: Result<brn_ai::mcp::Discovery, String>,
        cx: &mut Context<Self>,
    ) {
        self.mcp_view.discovering = false;
        if self.mcp_view.stamp != request_stamp {
            self.warn("MCP context changed during discovery; review again.");
            cx.notify();
            return;
        }
        let current_endpoint = self.mcp_view.endpoint.read(cx).value().to_string();
        if !Self::endpoint_matches(request_endpoint, &current_endpoint) {
            self.warn("MCP destination changed during discovery; review again.");
            cx.notify();
            return;
        }
        match result {
            Ok(discovery) if discovery.endpoint == request_endpoint => {
                self.mcp_view.discovery = Some(discovery);
                if matches!(
                    self.mcp_view.error.as_ref(),
                    Some(McpViewError::Discovery { .. })
                ) {
                    self.mcp_view.error = None;
                }
            }
            Ok(_) => {
                self.warn("MCP discovery returned metadata for a different destination.");
            }
            Err(error) => {
                self.mcp_view.error = Some(McpViewError::Discovery {
                    endpoint: request_endpoint.to_owned(),
                    message: error,
                });
            }
        }
        cx.notify();
    }

    pub(super) fn approve_mcp_tool(&mut self, name: &str, digest: &str, cx: &mut Context<Self>) {
        self.refresh_mcp();
        if !matches!(self.chat, ChatTarget::Thread(_)) && self.selected_project.is_some() {
            self.warn("Open the intended Project chat before approving its connection access.");
            cx.notify();
            return;
        }
        if self.mcp_view.discovery.is_some() && !self.discovery_matches_endpoint(cx) {
            self.mcp_view.discovery = None;
            self.warn("MCP destination changed after discovery; discover and inspect it again.");
            cx.notify();
            return;
        }
        let Some(discovery) = &self.mcp_view.discovery else {
            self.warn("Discover and inspect the current local metadata first.");
            cx.notify();
            return;
        };
        let Some(tool) = discovery
            .tools
            .iter()
            .find(|t| t.name == name && t.metadata_digest == digest && t.supported)
        else {
            self.warn("That exact supported tool is no longer available for approval.");
            cx.notify();
            return;
        };
        let Some(schema) = tool.schema_digest.clone() else {
            return;
        };
        let Some(contract) = tool
            .metadata
            .get("inputSchema")
            .and_then(|s| brn_ai::mcp::input_shape(s).ok())
        else {
            self.warn("This exact tool schema is unsupported.");
            cx.notify();
            return;
        };
        let access = match &self.chat {
            ChatTarget::Thread(chat) => McpAccess::Chat(chat.clone()),
            _ => McpAccess::General,
        };
        let policy = McpPolicy {
            endpoint: discovery.endpoint.clone(),
            server_digest: discovery.server_digest.clone(),
            enabled: true,
            tools: vec![McpApprovedTool {
                name: name.into(),
                metadata_digest: digest.into(),
                schema_digest: schema,
                contract,
            }],
            access: vec![access],
        };
        match self.workspace.configure_mcp(Some(policy)) {
            Ok(_) => {
                self.refresh();
                self.warn("Approved one known-behavior tool for this scope. Exact transient input review is still required before any data egress.");
            }
            Err(error) => self.fail(error.to_string()),
        };
        cx.notify();
    }
    pub(super) fn disable_mcp(&mut self, cx: &mut Context<Self>) {
        self.mcp_view.clear_input();
        match self.workspace.disable_mcp() {
            Ok(_) => {
                self.refresh();
                self.warn("MCP disabled; subsequent dispatch is fenced. Already-sent remote processing may still occur.");
            }
            Err(error) => self.fail(error.to_string()),
        };
        cx.notify();
    }
    pub(super) fn begin_mcp_input(
        &mut self,
        tool: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_mcp();
        if !matches!(self.chat, ChatTarget::Thread(_))
            || self.worker.is_some()
            || !self
                .mcp_view
                .status
                .as_ref()
                .is_some_and(|s| s.tools.iter().any(|t| t.name == tool))
        {
            self.warn("Choose an idle permitted chat and an approved enabled tool first.");
            cx.notify();
            return;
        }
        self.mcp_view.clear_input();
        self.mcp_view.review_tool = Some(tool.into());
        let query = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Exact query to send once; never saved as a draft")
        });
        self.mcp_view.query_subscription =
            Some(
                cx.subscribe_in(&query, window, |this, _, event: &InputEvent, _, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.mcp_view.pending = None;
                        cx.notify();
                    }
                }),
            );
        self.mcp_view.query = Some(query);
        cx.notify();
    }
    pub(super) fn review_mcp_input(&mut self, cx: &mut Context<Self>) {
        self.refresh_mcp();
        let Some(tool) = self.mcp_view.review_tool.clone() else {
            return;
        };
        let Some(approved) = self
            .mcp_view
            .status
            .as_ref()
            .and_then(|s| s.tools.iter().find(|t| t.name == tool))
        else {
            return;
        };
        let query = self
            .mcp_view
            .query
            .as_ref()
            .map(|q| q.read(cx).value().to_string())
            .unwrap_or_default();
        let input = match approved.contract {
            McpInputContract::Empty => serde_json::json!({}),
            McpInputContract::Query => serde_json::json!({"query":query}),
        };
        if canonical_mcp_input(approved.contract, &input).is_err() {
            self.warn("Exact MCP input exceeds its typed UTF-8/encoded limits.");
            cx.notify();
            return;
        }
        let prompt = self.composer.read(cx).value().to_string();
        if prompt.trim().is_empty() {
            self.warn(
                "Write the intended owner request before authorizing this exact external input.",
            );
            cx.notify();
            return;
        }
        let ChatTarget::Thread(chat) = &self.chat else {
            return;
        };
        let snapshot = match self.workspace.external_input_snapshot(chat) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.mcp_view.clear_input();
                self.fail(error.to_string());
                cx.notify();
                return;
            }
        };
        let displayed_epoch = self.records.iter().find_map(|r| {
            if r.id == *chat
                && !r.archived
                && let RecordData::Thread(t) = &r.data
            {
                Some(t.context_epoch)
            } else {
                None
            }
        });
        let displayed_project =
            project_for_chat(&self.records, chat).map(|p| (p.id.clone(), p.version));
        if self
            .settings
            .mcp
            .as_ref()
            .and_then(|p| p.identity().ok())
            .as_ref()
            != Some(&snapshot.policy_identity)
            || displayed_epoch != Some(snapshot.context_epoch)
            || displayed_project != snapshot.project_context
        {
            self.mcp_view.clear_input();
            self.warn("MCP review no longer matches the displayed destination and context. Refresh and review them again.");
            cx.notify();
            return;
        }
        self.mcp_view.pending = Some(ReviewedInput {
            stamp: self.mcp_stamp(),
            snapshot,
            prompt,
            tool,
            input,
        });
        cx.notify();
    }
    pub(super) fn take_mcp_review(
        &mut self,
        thread: &str,
        prompt: &str,
        queued: bool,
    ) -> Result<Option<ReviewedInput>, String> {
        let pending = self.mcp_view.pending.take();
        self.mcp_view.query = None;
        self.mcp_view.query_subscription = None;
        self.mcp_view.review_tool = None;
        let Some(pending) = pending else {
            return Ok(None);
        };
        if queued
            || pending.stamp != self.mcp_stamp()
            || pending.stamp.chat != ChatTarget::Thread(thread.into())
            || pending.prompt != prompt
        {
            return Err("MCP input review no longer matches this exact owner request. No new run was started; review the destination, scope and input again.".into());
        }
        Ok(Some(pending))
    }
    pub(super) fn mcp_settings(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let current_endpoint = self.mcp_view.endpoint.read(cx).value().to_string();
        let mut panel = div()
            .id("mcp-settings")
            .flex()
            .flex_col()
            .gap_3()
            .child(section("Anonymous local MCP client"))
            .child(metadata(MCP_LIMITATIONS))
            .child(
                Input::new(&self.mcp_view.endpoint)
                    .id("mcp-endpoint")
                    .w_full(),
            )
            .child(
                button("mcp-discover")
                    .label("Discover local metadata (no tool call)")
                    .disabled(self.mcp_view.discovering)
                    .on_click(cx.listener(|this, _, window, cx| this.discover_mcp(window, cx))),
            )
            .child(
                button("mcp-disable")
                    .label("Disable / disconnect")
                    .on_click(cx.listener(|this, _, _, cx| this.disable_mcp(cx))),
            );
        if let Some(error) = &self.mcp_view.error {
            let message = match error {
                McpViewError::Status(message) => Some(message),
                McpViewError::Discovery { endpoint, message }
                    if Self::endpoint_matches(endpoint, &current_endpoint) =>
                {
                    Some(message)
                }
                McpViewError::Discovery { .. } => None,
            };
            if let Some(message) = message {
                panel = panel.child(
                    metadata(message.clone())
                        .id("mcp-error")
                        .test_support()
                        .into_any_element(),
                );
            }
        }
        if self.mcp_view.stamp.as_ref() == Some(&self.mcp_stamp()) {
            if let Some(discovery) = &self.mcp_view.discovery
                && self.discovery_matches_endpoint(cx)
            {
                panel=panel.child(metadata(format!("Destination: {} · self-reported metadata pin {}",discovery.endpoint,discovery.server_digest)))
                    .child(metadata("Untrusted metadata below. Approve only independently known read-only behavior. Approval replaces the existing connection policy with this one tool/current scope; it does not authorize a query."));
                for tool in &discovery.tools {
                    let (name, digest) = (tool.name.clone(), tool.metadata_digest.clone());
                    panel = panel
                        .child(metadata(format!(
                            "{} · {} · {}",
                            tool.name, tool.metadata_digest, tool.metadata
                        )))
                        .child(
                            button(format!("mcp-approve-{}", tool.name))
                                .label("Approve known read-only behavior for this scope")
                                .disabled(!tool.supported)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.approve_mcp_tool(&name, &digest, cx)
                                })),
                        );
                }
            }
            if let Some(status) = &self.mcp_view.status {
                panel = panel.child(metadata(format!(
                    "Enabled: {} · current scope authorized: {} · transient grant required",
                    status.connection_enabled, status.scope_authorized
                )));
                for tool in &status.tools {
                    let name = tool.name.clone();
                    panel = panel.child(
                        button(format!("mcp-review-{}", tool.name))
                            .label(format!("Review exact input for {}", tool.name))
                            .disabled(self.worker.is_some())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.begin_mcp_input(&name, window, cx)
                            })),
                    );
                }
            }
            if let Some(query) = &self.mcp_view.query {
                panel=panel.child(Input::new(query).id("mcp-reviewed-query").w_full())
                    .child(metadata("The displayed destination/current chat and exact input are sent only for the unchanged owner prompt. This transient field is never autosaved."))
                    .child(button("mcp-authorize-input").label("Authorize this exact input ONCE for the current owner request").on_click(cx.listener(|this,_,_,cx|this.review_mcp_input(cx))));
            }
            if let Some(pending) = &self.mcp_view.pending {
                panel = panel
                    .child(metadata(format!(
                        "Pending exact input: {} · {} · {}",
                        self.mcp_view
                            .status
                            .as_ref()
                            .and_then(|s| s.endpoint.as_deref())
                            .unwrap_or("unavailable"),
                        pending.tool,
                        pending.input
                    )))
                    .child(metadata(format!(
                        "Current owner request: {}",
                        pending.prompt
                    )));
            }
        }
        panel.test_support().into_any_element()
    }
    pub(super) fn mcp_detail(&self, chat: &str) -> gpui_kit::AnyElement {
        let mut panel = div().id("mcp-status-detail").flex().flex_col().gap_1();
        if self.chat == ChatTarget::Thread(chat.into())
            && self.mcp_view.stamp.as_ref() == Some(&self.mcp_stamp())
        {
            if let Some(status) = &self.mcp_view.status {
                panel = panel.child(metadata(format!(
                    "Local MCP: {} · {}. No automatic grant; one reviewed input per invocation.",
                    if status.connection_enabled {
                        "enabled"
                    } else {
                        "disabled"
                    },
                    if status.scope_authorized {
                        "current scope approved"
                    } else {
                        "scope unavailable"
                    }
                )));
            }
            if self.mcp_view.pending.is_some() {
                panel = panel.child(metadata(
                    "One transient input review is pending for this unchanged owner request.",
                ));
            }
        }
        panel.test_support().into_any_element()
    }
}
