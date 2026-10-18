//! Invocation-local authority for one exact reviewed external input. Nothing in
//! this module serializes a grant or keeps the input/remote payload in SQLite.
use crate::*;
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
    sync::{Arc, Weak},
};

fn invalid() -> Error {
    Error::Invalid("external-data authorization is unavailable or no longer current".into())
}
pub fn canonical_mcp_endpoint(value: &str) -> Result<String> {
    if value.len() > 2048 || !value.is_ascii() {
        return Err(invalid());
    }
    let rest = value.strip_prefix("http://").ok_or_else(invalid)?;
    let (authority, path) = rest.split_once('/').ok_or_else(invalid)?;
    let socket: SocketAddr = authority.parse().map_err(|_| invalid())?;
    if socket.port() == 0
        || !matches!(socket.ip(), IpAddr::V4(ip) if ip == Ipv4Addr::LOCALHOST)
            && !matches!(socket.ip(), IpAddr::V6(ip) if ip == Ipv6Addr::LOCALHOST)
        || path
            .bytes()
            .any(|b| !b.is_ascii_alphanumeric() && !b"/-_.~".contains(&b))
        || path.split('/').any(|p| p == "." || p == "..")
    {
        return Err(invalid());
    }
    Ok(format!("http://{socket}/{path}"))
}
pub fn mcp_input_schema(contract: McpInputContract) -> serde_json::Value {
    match contract {
        McpInputContract::Empty => {
            serde_json::json!({"type":"object","properties":{},"additionalProperties":false})
        }
        McpInputContract::Query => {
            serde_json::json!({"type":"object","properties":{"query":{"type":"string","maxLength":2048}},"required":["query"],"additionalProperties":false})
        }
    }
}
#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryInput {
    query: String,
}
#[derive(serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyInput {}
/// Parses a fixed typed contract. This is not a JSON Schema keyword evaluator.
/// Canonical serialization is also the exact host-reviewed input hash source.
pub fn canonical_mcp_input(
    contract: McpInputContract,
    value: &serde_json::Value,
) -> Result<String> {
    if !value.is_object() || serde_json::to_vec(value)?.len() > 8192 {
        return Err(invalid());
    }
    let text = match contract {
        McpInputContract::Empty => serde_json::to_string(
            &serde_json::from_value::<EmptyInput>(value.clone()).map_err(|_| invalid())?,
        )?,
        McpInputContract::Query => {
            let input =
                serde_json::from_value::<QueryInput>(value.clone()).map_err(|_| invalid())?;
            if input.query.len() > 2048 {
                return Err(invalid());
            }
            serde_json::to_string(&input)?
        }
    };
    if text.len() > 8192 {
        return Err(invalid());
    }
    Ok(text)
}
pub fn mcp_input_schema_digest(contract: McpInputContract) -> String {
    hash(
        b"brn-mcp-input-schema-v1\0",
        mcp_input_schema(contract).to_string().as_bytes(),
    )
}
fn hash(domain: &[u8], bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(domain);
    h.update(bytes);
    format!("{:x}", h.finalize())
}
impl McpPolicy {
    pub fn validate(&self) -> Result<()> {
        if ((!self.server_digest.is_empty() || self.enabled)
            && !skill_digest_valid(&self.server_digest))
            || canonical_mcp_endpoint(&self.endpoint)? != self.endpoint
            || self.tools.len() > 8
            || self.access.len() > 32
            || self.enabled && (self.tools.is_empty() || self.access.is_empty())
            || !self.tools.windows(2).all(|p| p[0].name < p[1].name)
            || !self.access.windows(2).all(|p| p[0] < p[1])
            || serde_json::to_vec(self)?.len() > 32 * 1024
        {
            return Err(invalid());
        }
        for tool in &self.tools {
            if tool.name.is_empty()
                || tool.name.len() > 64
                || !tool
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                || !skill_digest_valid(&tool.metadata_digest)
                || tool.schema_digest != mcp_input_schema_digest(tool.contract)
            {
                return Err(invalid());
            }
        }
        for access in &self.access {
            if let McpAccess::Chat(id) | McpAccess::Project(id) = access
                && (id.trim().is_empty() || id.len() > 256 || id.chars().any(char::is_control))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<String> {
        self.validate()?;
        Ok(hash(b"brn-mcp-policy-v1\0", &serde_json::to_vec(self)?))
    }
}
/// Validate new authority in the same checked writer as raw owner replacements.
/// Exact unavailable rows may be retained; removing/disabling never grants.
pub(crate) fn validate_policy_context(
    conn: &rusqlite::Connection,
    next: Option<&McpPolicy>,
    prior: Option<&McpPolicy>,
) -> Result<()> {
    let Some(next) = next else {
        return Ok(());
    };
    next.validate()?;
    if !next.enabled {
        return Ok(());
    }
    for access in &next.access {
        let active = match access {
            McpAccess::General => true,
            McpAccess::Project(id) => matches!(
                crate::store::record_on(conn, id)?,
                Some(Record {
                    archived: false,
                    data: RecordData::Project(_),
                    ..
                })
            ),
            McpAccess::Chat(id) => matches!(
                crate::store::record_on(conn, id)?,
                Some(Record {
                    archived: false,
                    data: RecordData::Thread(Thread {
                        state: ThreadState::Open,
                        ..
                    }),
                    ..
                })
            ),
        };
        if !active
            && !prior.is_some_and(|old| {
                old.enabled
                    && old.endpoint == next.endpoint
                    && old.server_digest == next.server_digest
                    && old.tools == next.tools
                    && old.access.contains(access)
            })
        {
            return Err(invalid());
        }
    }
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
struct GrantContext {
    run: String,
    fence: u64,
    instruction: (String, u64, String),
    chat: String,
    context_epoch: u64,
    private: bool,
    project: Option<(String, u64)>,
    settings: (String, u64),
    policy: String,
    guide: String,
}
/// Only checked core host-owner construction can create this capability. It has
/// no Serialize/Deserialize/Debug implementation and no write authority.
pub struct ExternalDataGrant {
    epoch: Weak<()>,
    context: GrantContext,
    endpoint: String,
    tool: McpApprovedTool,
    input_hash: String,
}
impl ExternalDataGrant {
    /// Opaque identity for checking the prepared snapshot, not grant authority.
    pub fn policy_identity(&self) -> &str {
        &self.context.policy
    }
}
impl Store {
    fn external_context(&self, run_id: &str) -> Result<(GrantContext, McpPolicy)> {
        let run_record = self.record(run_id)?.ok_or_else(invalid)?;
        let RecordData::Run(run) = run_record.data else {
            return Err(invalid());
        };
        if run_record.archived || run.state != RunState::Working {
            return Err(invalid());
        }
        let chat = self.record(&run.thread)?.ok_or_else(invalid)?;
        let RecordData::Thread(thread) = chat.data else {
            return Err(invalid());
        };
        if chat.archived || thread.state != ThreadState::Open {
            return Err(invalid());
        }
        let instruction = self
            .record(run.instruction.as_deref().ok_or_else(invalid)?)?
            .ok_or_else(invalid)?;
        if instruction.archived
            || !matches!(&instruction.data,RecordData::Message(m) if m.thread == run.thread && m.role == "owner")
        {
            return Err(invalid());
        }
        let instruction_hash = hash(
            b"brn-external-instruction-v1\0",
            &serde_json::to_vec(&instruction.data)?,
        );
        let settings_body: Option<String> = self.conn.query_row("SELECT body FROM records WHERE json_type(body,'$.data.Settings') IS NOT NULL AND json_extract(body,'$.archived')=0", [], |r| r.get(0)).optional()?;
        let settings: Record = serde_json::from_str(&settings_body.ok_or_else(invalid)?)?;
        let RecordData::Settings(config) = settings.data else {
            return Err(invalid());
        };
        if !crate::store::run_matches_settings(&run, &config) {
            return Err(invalid());
        }
        let policy = config.mcp.ok_or_else(invalid)?;
        policy.validate()?;
        if !policy.enabled {
            return Err(invalid());
        }
        let project = self.project_context_record(&run.thread)?;
        if project.as_ref().is_some_and(|p| p.archived) {
            return Err(invalid());
        }
        let private = self.chat_has_private_context(&run.thread)?;
        let allowed = policy.access.iter().any(|a| match a {
            McpAccess::Chat(id) => id == &run.thread,
            McpAccess::Project(id) => project.as_ref().is_some_and(|p| &p.id == id),
            McpAccess::General => project.is_none() && !private,
        });
        if !allowed {
            return Err(invalid());
        }
        Ok((
            GrantContext {
                run: run_id.to_owned(),
                fence: run.fence,
                instruction: (instruction.id, instruction.version, instruction_hash),
                chat: run.thread,
                context_epoch: thread.context_epoch,
                private,
                project: project.map(|p| (p.id, p.version)),
                settings: (settings.id, settings.version),
                policy: policy.identity()?,
                guide: run.guide_identity,
            },
            policy,
        ))
    }
    /// The owner reviews the exact parsed typed input after durable prepare.
    /// No prose/message ID/hash alone can manufacture this authority.
    pub fn grant_external_data(
        &self,
        run_id: &str,
        tool: &str,
        reviewed_input: &serde_json::Value,
        authority: &HostAuthority,
    ) -> Result<ExternalDataGrant> {
        if !matches!(authority.scope, crate::types::Scope::Owner) {
            return Err(invalid());
        }
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Deferred)?;
        let (context, policy) = self.external_context(run_id)?;
        let tool = policy
            .tools
            .iter()
            .find(|t| t.name == tool)
            .ok_or_else(invalid)?
            .clone();
        let input = canonical_mcp_input(tool.contract, reviewed_input)?;
        let grant = ExternalDataGrant {
            epoch: Arc::downgrade(&self.external_grant_epoch),
            context,
            endpoint: policy.endpoint,
            tool,
            input_hash: hash(b"brn-external-input-v1\0", input.as_bytes()),
        };
        tx.commit()?;
        Ok(grant)
    }
    pub fn validate_external_dispatch(
        &self,
        grant: &ExternalDataGrant,
        endpoint: &str,
        tool: &str,
        metadata_digest: &str,
        input: &serde_json::Value,
    ) -> Result<()> {
        if !grant
            .epoch
            .upgrade()
            .is_some_and(|epoch| Arc::ptr_eq(&epoch, &self.external_grant_epoch))
        {
            return Err(invalid());
        }
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Deferred)?;
        let (current, policy) = self.external_context(&grant.context.run)?;
        let canonical = canonical_mcp_input(grant.tool.contract, input)?;
        if current != grant.context
            || endpoint != grant.endpoint
            || tool != grant.tool.name
            || metadata_digest != grant.tool.metadata_digest
            || !policy.tools.contains(&grant.tool)
            || hash(b"brn-external-input-v1\0", canonical.as_bytes()) != grant.input_hash
        {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(())
    }
    /// Control-channel liveness only; this does not authorize a tool input.
    pub fn validate_external_channel(&self, grant: &ExternalDataGrant) -> Result<()> {
        if !grant
            .epoch
            .upgrade()
            .is_some_and(|epoch| Arc::ptr_eq(&epoch, &self.external_grant_epoch))
        {
            return Err(invalid());
        }
        let tx = Transaction::new_unchecked(&self.conn, TransactionBehavior::Deferred)?;
        let (current, _) = self.external_context(&grant.context.run)?;
        if current != grant.context {
            return Err(invalid());
        }
        tx.commit()?;
        Ok(())
    }
}
