//! `brn documents list|show`: read-only source document surfaces.
use crate::cli::{error::classify_workflow, error::CliError, Invocation, Output};
use brn_workflow::{SearchApproval, SourceDocument, Workspace};
use serde_json::json;
use uuid::Uuid;

pub fn approval_str(approval: SearchApproval) -> &'static str {
    match approval {
        SearchApproval::Approved => "approved",
        SearchApproval::Draft => "draft",
        SearchApproval::Withdrawn => "withdrawn",
    }
}

pub fn sha256_hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn summary(doc: &SourceDocument) -> serde_json::Value {
    json!({
        "source_id": doc.source_id,
        "version_id": doc.version_id,
        "title": doc.title,
        "origin": doc.origin,
        "approval": approval_str(doc.approval),
        "sha256_hex": sha256_hex(&doc.sha256),
    })
}

pub fn list(_invocation: &Invocation, workspace: &Workspace) -> Result<Output, CliError> {
    let docs = workspace.sources().map_err(classify_workflow)?;
    let summaries: Vec<_> = docs.iter().map(summary).collect();
    let mut text = String::new();
    for doc in &docs {
        text.push_str(&format!(
            "{} {} {} {}\n",
            doc.source_id,
            doc.version_id,
            approval_str(doc.approval),
            doc.title,
        ));
    }
    Ok(Output {
        text,
        data: json!({ "documents": summaries }),
    })
}

pub fn show(
    _invocation: &Invocation,
    workspace: &Workspace,
    source: Uuid,
) -> Result<Output, CliError> {
    let docs = workspace.sources().map_err(classify_workflow)?;
    let doc = docs.iter().find(|d| d.source_id == source).ok_or_else(|| {
        crate::cli::error::CliError::NotFound(format!("source {source} not found"))
    })?;
    let content = std::str::from_utf8(&doc.bytes)
        .map_err(|_| {
            crate::cli::error::CliError::Workflow("stored source bytes are not valid UTF-8".into())
        })?
        .to_string();
    let mut data = summary(doc);
    if let serde_json::Value::Object(map) = &mut data {
        map.insert("content".into(), json!(content));
    }
    let text = format!(
        "source_id: {}\nversion_id: {}\ntitle: {}\norigin: {}\napproval: {}\nsha256: {}\n{}",
        doc.source_id,
        doc.version_id,
        doc.title,
        doc.origin,
        approval_str(doc.approval),
        sha256_hex(&doc.sha256),
        content,
    );
    Ok(Output { text, data })
}
