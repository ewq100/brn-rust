//! BRN's single outbound MCP boundary. SDK protocol and Rig dispatch are reused;
//! owner policy/grants remain host authority, never server/model instructions.
use rmcp::model::Tool as McpToolDefinition;
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const MAX_TOOLS: usize = 8;
pub const MAX_CATALOGUE_BYTES: usize = 32 * 1024;
pub const MAX_TOOL_BYTES: usize = 4 * 1024;
pub const MAX_SCHEMA_BYTES: usize = 2 * 1024;
pub const MAX_ARGUMENT_BYTES: usize = 8 * 1024;
pub const MAX_QUERY_BYTES: usize = 2048;
pub const MAX_WIRE_BYTES: usize = 256 * 1024;
pub const MAX_TEXT_BYTES: usize = 32 * 1024;

/// Static diagnostics only. SDK/server payloads never become owner error text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Endpoint,
    Catalogue,
    Schema,
    Arguments,
    Unavailable,
    Transport,
    Timeout,
    Cancelled,
    ResultLimit,
    InvalidResult,
}
impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Endpoint => "MCP supports only an explicit anonymous literal-loopback HTTP destination without credentials, query or fragment.",
            Self::Catalogue => "The external tool catalogue does not match its exact owner approval.",
            Self::Schema => "This external tool input schema is unsupported by the bounded MCP connection.",
            Self::Arguments => "External tool input does not match its supported typed contract and limits.",
            Self::Unavailable => "The external connection or its current data grant is unavailable.",
            Self::Transport => "The external MCP request failed. No successful result was accepted.",
            Self::Timeout => "The external MCP request timed out. Remote processing may have occurred.",
            Self::Cancelled => "The external MCP request was stopped. Remote processing may have occurred.",
            Self::ResultLimit => "The external MCP response exceeded BRN's bounded admission limit.",
            Self::InvalidResult => "The external MCP response is not a supported nonempty text result.",
        })
    }
}
impl std::error::Error for Refusal {}

/// An intentionally small URI subset: no DNS, proxies, credentials or redirects.
/// This is an anonymous local connection, not qualified remote authentication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint(String);
impl Endpoint {
    pub fn parse(value: &str) -> Result<Self, Refusal> {
        brn_threads_core::canonical_mcp_endpoint(value)
            .map(Self)
            .map_err(|_| Refusal::Endpoint)
    }
    pub fn uri(&self) -> &str {
        &self.0
    }
    pub fn identity(&self) -> String {
        digest(b"brn-mcp-destination-v1\0", self.0.as_bytes())
    }
}

/// Two fixed typed contracts, not a partial JSON Schema evaluator. Any other
/// schema is refused before approval/registration; references are never fetched.
pub use brn_threads_core::McpInputContract as InputShape;
pub fn input_schema(shape: InputShape) -> Value {
    brn_threads_core::mcp_input_schema(shape)
}
pub fn input_shape(schema: &Value) -> Result<InputShape, Refusal> {
    for shape in [InputShape::Empty, InputShape::Query] {
        if *schema == input_schema(shape) {
            return Ok(shape);
        }
    }
    Err(Refusal::Schema)
}
pub fn validate_arguments(shape: InputShape, args: &Value) -> Result<(), Refusal> {
    brn_threads_core::canonical_mcp_input(shape, args)
        .map(|_| ())
        .map_err(|_| Refusal::Arguments)
}

/// Exact owner approval is independent of untrusted server annotations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovedTool {
    pub name: String,
    pub digest: String,
}
#[derive(Clone, Debug)]
pub struct FrozenTool {
    pub model_name: String,
    pub metadata_digest: String,
    pub shape: InputShape,
    pub definition: McpToolDefinition,
}
fn digest(domain: &[u8], bytes: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(bytes);
    format!("{:x}", hash.finalize())
}
pub fn model_name(endpoint: &Endpoint, metadata_digest: &str) -> String {
    let identity = digest(
        b"brn-mcp-model-name-v1\0",
        format!("{}:{metadata_digest}", endpoint.identity()).as_bytes(),
    );
    format!("mcp_{}", &identity[..60])
}
pub fn metadata_digest(tool: &McpToolDefinition) -> Result<String, Refusal> {
    let bytes = serde_json::to_vec(tool).map_err(|_| Refusal::Catalogue)?;
    if bytes.len() > MAX_TOOL_BYTES
        || tool.schema_as_json_value().to_string().len() > MAX_SCHEMA_BYTES
    {
        return Err(Refusal::Catalogue);
    }
    Ok(digest(b"brn-mcp-tool-metadata-v1\0", &bytes))
}
/// Never register newly advertised tools implicitly. Duplicate names or changed
/// approved metadata fail closed; unknown tools remain absent, not enabled.
pub fn freeze_tools(
    endpoint: &Endpoint,
    advertised: &[McpToolDefinition],
    approved: &[ApprovedTool],
) -> Result<Vec<FrozenTool>, Refusal> {
    if advertised.len() > MAX_TOOLS
        || approved.len() > MAX_TOOLS
        || serde_json::to_vec(advertised)
            .map_err(|_| Refusal::Catalogue)?
            .len()
            > MAX_CATALOGUE_BYTES
    {
        return Err(Refusal::Catalogue);
    }
    let mut names = std::collections::BTreeSet::new();
    for tool in advertised {
        if !names.insert(tool.name.as_ref()) {
            return Err(Refusal::Catalogue);
        }
    }
    let mut approved_names = std::collections::BTreeSet::new();
    let mut frozen = Vec::new();
    for approval in approved {
        if !approved_names.insert(&approval.name) {
            return Err(Refusal::Catalogue);
        }
        let tool = advertised
            .iter()
            .find(|t| t.name == approval.name)
            .ok_or(Refusal::Catalogue)?;
        let annotations = tool.annotations.as_ref().ok_or(Refusal::Catalogue)?;
        if annotations.read_only_hint != Some(true)
            || annotations.destructive_hint == Some(true)
            || tool.name.is_empty()
            || tool.name.len() > 64
            || !tool
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        {
            return Err(Refusal::Catalogue);
        }
        let actual = metadata_digest(tool)?;
        if actual != approval.digest {
            return Err(Refusal::Catalogue);
        }
        let shape = input_shape(&tool.schema_as_json_value())?;

        frozen.push(FrozenTool {
            model_name: model_name(endpoint, &actual),
            metadata_digest: actual,
            shape,
            definition: tool.clone(),
        });
    }
    Ok(frozen)
}

#[cfg(test)]
#[path = "threads_mcp_tests.rs"]
mod tests;

#[path = "threads_mcp_http.rs"]
pub mod http;

#[path = "threads_mcp_client.rs"]
mod client;
pub use client::{Connection, DiscoveredTool, Discovery};

#[cfg(test)]
#[path = "threads_mcp_http_tests.rs"]
pub(crate) mod http_tests;

/// Deliberate owner discovery, never invoked by an unrelated model run. This
/// gate permits only the SDK handshake/list/ping, with no tool-call authority.
struct DiscoveryGate;
impl http::DispatchGate for DiscoveryGate {
    fn message(&self, message: &rmcp::model::ClientJsonRpcMessage) -> Result<(), Refusal> {
        use rmcp::model::{ClientNotification, ClientRequest, JsonRpcMessage};
        match message {
            JsonRpcMessage::Request(r)
                if matches!(
                    r.request,
                    ClientRequest::InitializeRequest(_)
                        | ClientRequest::ListToolsRequest(_)
                        | ClientRequest::PingRequest(_)
                ) =>
            {
                Ok(())
            }
            JsonRpcMessage::Notification(n)
                if matches!(
                    n.notification,
                    ClientNotification::InitializedNotification(_)
                        | ClientNotification::CancelledNotification(_)
                ) =>
            {
                Ok(())
            }
            _ => Err(Refusal::Unavailable),
        }
    }
    fn channel(&self) -> Result<(), Refusal> {
        Ok(())
    }
}
pub async fn discover(endpoint: &str) -> Result<Discovery, Refusal> {
    let connection = Connection::connect(
        Endpoint::parse(endpoint)?,
        std::sync::Arc::new(DiscoveryGate),
    )
    .await?;
    let result = connection.discovery().clone();
    connection.disconnect().await;
    Ok(result)
}
