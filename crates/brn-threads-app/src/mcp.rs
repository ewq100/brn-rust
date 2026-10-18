//! Explicit owner connection controls, never registered as model tools.
use crate::*;
use brn_ai::mcp;

pub const MCP_LIMITATIONS: &str = "Anonymous literal-loopback HTTP only; remote and authenticated servers are unsupported. Approvals pin self-reported metadata, not server implementation or read-only behavior. Each invocation needs one exact transient owner-reviewed input; enablement alone grants no data egress. Stop cannot undo remote processing.";

#[derive(Clone, Serialize)]
pub struct McpStatus {
    pub configured: bool,
    pub connection_enabled: bool,
    pub scope_authorized: bool,
    pub endpoint: Option<String>,
    pub tools: Vec<McpApprovedTool>,
    pub grant_required: bool,
    pub limitations: &'static str,
}
fn refusal(error: mcp::Refusal) -> AppError {
    match error {
        mcp::Refusal::Endpoint
        | mcp::Refusal::Catalogue
        | mcp::Refusal::Schema
        | mcp::Refusal::Arguments => AppError::Invalid(error.to_string()),
        _ => AppError::Unavailable(error.to_string()),
    }
}
impl Workspace {
    /// Non-authoritative snapshot for an explicit owner input-review surface.
    /// Native carries this unchanged through its later durable preparation.
    pub fn external_input_snapshot(
        &self,
        chat: &str,
    ) -> brn_ai::AiResult<brn_ai::threads::ExternalInputSnapshot> {
        brn_ai::threads::ExternalInputSnapshot::capture(&self.store, chat)
    }
    /// No canonical changes or tool invocation; owner action only. No workspace
    /// lock/transaction survives this network await. Metadata is untrusted.
    pub async fn discover_mcp(endpoint: &str) -> AppResult<mcp::Discovery> {
        mcp::discover(endpoint).await.map_err(refusal)
    }
    /// Only currently permitted tool metadata is projected, without hidden
    /// access rows/counts. Status never connects or reconstructs a grant.
    pub fn mcp_status(&self, chat: Option<&str>) -> AppResult<McpStatus> {
        let policy = self.settings()?.mcp;
        let mut allowed = false;
        if let Some(policy) = &policy {
            allowed = if let Some(chat) = chat {
                let record = self.store.record(chat)?.ok_or_else(|| {
                    AppError::Invalid("MCP context must name a current chat".into())
                })?;
                let active = !record.archived
                    && matches!(&record.data, RecordData::Thread(t) if t.state == ThreadState::Open);
                let project = self.store.project_context_record(chat)?;
                active
                    && !project.as_ref().is_some_and(|p| p.archived)
                    && policy.access.iter().any(|a| match a {
                        McpAccess::Chat(id) => id == chat,
                        McpAccess::Project(id) => project.as_ref().is_some_and(|p| &p.id == id),
                        McpAccess::General => {
                            project.is_none()
                                && !self.store.chat_has_private_context(chat).unwrap_or(true)
                        }
                    })
            } else {
                policy.access.contains(&McpAccess::General)
            };
        }
        Ok(McpStatus {
            configured: policy.is_some(),
            connection_enabled: policy.as_ref().is_some_and(|p| p.enabled),
            scope_authorized: allowed,
            endpoint: policy.as_ref().map(|p| p.endpoint.clone()),
            tools: policy
                .as_ref()
                .filter(|p| p.enabled && allowed)
                .map(|p| p.tools.clone())
                .unwrap_or_default(),
            grant_required: true,
            limitations: MCP_LIMITATIONS,
        })
    }
    /// Supplied identities are the owner's exact reviewed metadata. The model
    /// cannot reach this service. Raw checked-owner replacements validate too.
    pub fn configure_mcp(&mut self, policy: Option<McpPolicy>) -> AppResult<ApplyOutcome> {
        let op = self
            .store
            .allocate_operation(&format!("mcp-policy:{}", uuid::Uuid::new_v4()))?;
        let outcome = self.store.change_mcp_policy(
            &op,
            policy.as_ref(),
            &HostAuthority::owner("owner MCP policy review"),
        )?;
        Ok(self.remove_leases_ended_by(outcome))
    }
    pub fn disable_mcp(&mut self) -> AppResult<Option<ApplyOutcome>> {
        let Some(mut policy) = self.settings()?.mcp else {
            return Ok(None);
        };
        policy.enabled = false;
        self.configure_mcp(Some(policy)).map(Some)
    }
}
impl PreparedWorkspaceRun {
    /// Consumes the one durable prepared run. On refusal the AI seam releases
    /// the same lease and terminalizes it; never prepare/retry behind the owner.
    pub fn with_external_input(
        self,
        tool: &str,
        input: &serde_json::Value,
    ) -> brn_ai::AiResult<Self> {
        Ok(Self {
            prepared: self.prepared.with_external_input(tool, input)?,
            search_status: self.search_status,
        })
    }
}
