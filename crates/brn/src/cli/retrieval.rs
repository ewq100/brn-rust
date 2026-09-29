//! `import`, `documents set-search-approval`, `index build`, `search`.
use crate::cli::{
    documents::approval_str,
    error::{classify_workflow, CliError},
    open_workspace, Command, Invocation, Output,
};
use brn_workflow::{profile_name, SearchApproval, Workspace};
use serde_json::json;
use std::{io::Write as _, sync::atomic::Ordering};
use uuid::Uuid;

pub fn run(invocation: &Invocation) -> Result<Output, CliError> {
    let mut workspace = open_workspace(invocation)?;
    match &invocation.command {
        Command::Import {
            path,
            approve_for_search,
            operation,
        } => import(&mut workspace, path, *approve_for_search, *operation),
        Command::DocumentsSetApproval {
            source,
            version,
            state,
            operation,
        } => set_approval(&mut workspace, *source, *version, *state, *operation),
        Command::IndexBuild => build(&mut workspace),
        Command::Search { query, profile } => search(&mut workspace, query, *profile),
        _ => unreachable!("retrieval module handles import, approval, build and search only"),
    }
}

/// Approval defaults to Draft; `--approve-for-search` is the only path to Approved.
fn import(
    workspace: &mut Workspace,
    path: &std::path::Path,
    approve_for_search: bool,
    operation: Option<Uuid>,
) -> Result<Output, CliError> {
    let approval = if approve_for_search {
        SearchApproval::Approved
    } else {
        SearchApproval::Draft
    };
    let op = operation.unwrap_or_else(Uuid::new_v4);
    let result = workspace
        .import_file(op, path, approval)
        .map_err(classify_workflow)?;
    Ok(Output {
        text: format!(
            "imported {} {} {} approval={} operation={}\n",
            result.source_id,
            result.version_id,
            if result.changed {
                "changed"
            } else {
                "unchanged"
            },
            approval_str(approval),
            op
        ),
        data: json!({
            "source_id": result.source_id,
            "version_id": result.version_id,
            "changed": result.changed,
            "approval": approval_str(approval),
            "operation_id": op,
        }),
    })
}

/// Approval binds the exact version: the requested version must be the
/// source's current version, checked structurally against `sources()`.
fn set_approval(
    workspace: &mut Workspace,
    source: Uuid,
    version: Uuid,
    state: SearchApproval,
    operation: Option<Uuid>,
) -> Result<Output, CliError> {
    let docs = workspace.sources().map_err(classify_workflow)?;
    let doc = docs
        .iter()
        .find(|d| d.source_id == source)
        .ok_or_else(|| CliError::NotFound(format!("source {source} not found")))?;
    if doc.version_id != version {
        return Err(CliError::NotFound(format!(
            "version {version} is not the current version of source {source}"
        )));
    }
    let op = operation.unwrap_or_else(Uuid::new_v4);
    workspace
        .set_approval(op, source, version, state)
        .map_err(classify_workflow)?;
    Ok(Output {
        text: format!(
            "source {source} version {version} search approval set to {} operation={op}\n",
            approval_str(state)
        ),
        data: json!({
            "source_id": source,
            "version_id": version,
            "state": approval_str(state),
            "operation_id": op,
        }),
    })
}

fn build(workspace: &mut Workspace) -> Result<Output, CliError> {
    let generation = workspace
        .build_index(&crate::CANCEL, |p| {
            // Best-effort progress: a closed stderr never affects the build.
            let _ = writeln!(std::io::stderr(), "index: {p}");
        })
        .map_err(cancelled_or_classified)?;
    Ok(Output {
        text: format!("index generation {generation}\n"),
        data: json!({ "generation": generation }),
    })
}

/// Errors after a SIGINT are reported as interruptions, not workflow faults.
fn cancelled_or_classified(message: String) -> CliError {
    if crate::CANCEL.load(Ordering::SeqCst) {
        CliError::Interrupted(message)
    } else {
        classify_workflow(message)
    }
}

fn search(
    workspace: &mut Workspace,
    query: &str,
    profile: brn_workflow::SearchProfile,
) -> Result<Output, CliError> {
    let result = workspace
        .search(query, profile)
        .map_err(cancelled_or_classified)?;
    let mut text = format!("{} {}\n", profile_name(result.profile), result.query);
    for evidence in &result.evidence {
        text.push_str(&format!(
            "{} {} {}..{}\n  {}\n",
            evidence.source_id,
            evidence.version_id,
            evidence.start_byte,
            evidence.end_byte,
            evidence.quote
        ));
    }
    Ok(Output {
        text,
        data: json!({
            "query": result.query,
            "profile": profile_name(result.profile),
            "evidence": result.evidence,
        }),
    })
}
