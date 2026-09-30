//! `drafts`, `comments`, `revisions` — read-only review surfaces.
//! All commands only read store state; lists preserve insertion order and
//! unknown objects are rejected by construction with `NOT_FOUND`.
use crate::cli::{
    documents::sha256_hex, error::classify_workflow, error::CliError, open_workspace, CliFailure,
    Command, Invocation, Output,
};
use brn_workflow::worker::RevisionKind;
use brn_workflow::{
    AmbiguityReason, AnchorState, CommentStatus, Draft, DraftCommentView, DraftRevision, Workspace,
};
use serde_json::json;
use uuid::Uuid;

pub fn run(invocation: &Invocation) -> Result<Output, CliFailure> {
    let workspace = open_workspace(invocation)?;
    match &invocation.command {
        Command::DraftsList => drafts_list(&workspace),
        Command::DraftsShow { draft } => drafts_show(&workspace, *draft),
        Command::CommentsList { draft } => comments_list(&workspace, *draft),
        Command::RevisionsList { draft } => revisions_list(&workspace, *draft),
        Command::RevisionsShow { revision } => revisions_show(&workspace, *revision),
        Command::RevisionsDiff { draft, from, to } => {
            revisions_diff(&workspace, *draft, *from, *to)
        }
        _ => unreachable!("only review commands are routed here"),
    }
}

fn require_draft(workspace: &Workspace, id: Uuid) -> Result<Draft, CliError> {
    workspace
        .draft(id)
        .map_err(classify_workflow)?
        .ok_or_else(|| CliError::NotFound(format!("draft {id} not found")))
}

fn require_revision(workspace: &Workspace, id: Uuid) -> Result<DraftRevision, CliError> {
    workspace
        .draft_revision(id)
        .map_err(classify_workflow)?
        .ok_or_else(|| CliError::NotFound(format!("revision {id} not found")))
}

fn status_str(status: CommentStatus) -> &'static str {
    match status {
        CommentStatus::Open => "open",
        CommentStatus::Resolved => "resolved",
    }
}

fn kind_str(kind: RevisionKind) -> &'static str {
    match kind {
        RevisionKind::Checkpoint => "checkpoint",
        RevisionKind::Candidate => "candidate",
    }
}

/// Stable lowercase reason strings, aligned with the store's own wire tags.
fn reason_str(reason: AmbiguityReason) -> &'static str {
    match reason {
        AmbiguityReason::Touched => "touched",
        AmbiguityReason::Duplicate => "duplicate",
        AmbiguityReason::MissingUnsupported => "missing_unsupported",
        AmbiguityReason::BoundaryAmbiguity => "boundary_ambiguity",
        AmbiguityReason::ConflictingSnapshot => "conflicting_snapshot",
        AmbiguityReason::HistoryLimit => "history_limit",
    }
}

/// Wire form of the anchor state; `start`/`end` only when anchored, `reason`
/// only when ambiguous.
fn anchor_json(anchor: &AnchorState) -> serde_json::Value {
    match anchor {
        AnchorState::Anchored { start, end } => {
            json!({ "state": "anchored", "start": start, "end": end })
        }
        AnchorState::Deleted => json!({ "state": "deleted" }),
        AnchorState::Ambiguous { reason } => {
            json!({ "state": "ambiguous", "reason": reason_str(*reason) })
        }
    }
}

fn anchor_text(anchor: &AnchorState) -> String {
    match anchor {
        AnchorState::Anchored { start, end } => format!("anchored {start}..{end}"),
        AnchorState::Deleted => "deleted".to_string(),
        AnchorState::Ambiguous { reason } => format!("ambiguous ({})", reason_str(*reason)),
    }
}

fn draft_summary(draft: &Draft) -> serde_json::Value {
    json!({
        "id": draft.id,
        "title": draft.title,
        "base_revision": draft.stamp.base_revision,
        "generation": draft.stamp.generation,
        "sha256_hex": sha256_hex(&draft.sha256),
    })
}

fn revision_summary(revision: &DraftRevision) -> serde_json::Value {
    json!({
        "id": revision.id,
        "draft_id": revision.draft_id,
        "parent_id": revision.parent_id,
        "kind": kind_str(revision.kind),
        "origin_turn": revision.origin_turn,
        "sha256_hex": sha256_hex(&revision.sha256),
    })
}

fn comment_json(view: &DraftCommentView) -> serde_json::Value {
    let comment = &view.comment;
    json!({
        "id": comment.id,
        "original_revision_id": comment.original_revision_id,
        "original_sha256_hex": sha256_hex(&comment.original_sha256),
        "original_start": comment.original_start,
        "original_end": comment.original_end,
        "original_quote": comment.original_quote,
        "body": comment.body,
        "status": status_str(comment.status),
        "status_version": comment.status_version,
        "anchor": anchor_json(&view.anchor),
    })
}

/// Human text must be self-terminated; only the terminator is ever added.
fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn drafts_list(workspace: &Workspace) -> Result<Output, CliFailure> {
    let drafts = workspace.drafts().map_err(classify_workflow)?;
    let mut text = String::new();
    for draft in &drafts {
        text.push_str(&format!(
            "{} generation={} base={} {}\n",
            draft.id, draft.stamp.generation, draft.stamp.base_revision, draft.title
        ));
    }
    Ok(Output {
        text,
        data: json!({ "drafts": drafts.iter().map(draft_summary).collect::<Vec<_>>() }),
    })
}

fn drafts_show(workspace: &Workspace, id: Uuid) -> Result<Output, CliFailure> {
    let draft = require_draft(workspace, id)?;
    let mut data = draft_summary(&draft);
    if let serde_json::Value::Object(map) = &mut data {
        map.insert("content".into(), json!(draft.text));
    }
    let text = ensure_trailing_newline(format!(
        "id: {}\ntitle: {}\nbase_revision: {}\ngeneration: {}\nsha256: {}\n{}",
        draft.id,
        draft.title,
        draft.stamp.base_revision,
        draft.stamp.generation,
        sha256_hex(&draft.sha256),
        draft.text
    ));
    Ok(Output { text, data })
}

fn comments_list(workspace: &Workspace, draft_id: Uuid) -> Result<Output, CliFailure> {
    require_draft(workspace, draft_id)?;
    let comments = workspace
        .draft_comments(draft_id)
        .map_err(classify_workflow)?;
    let mut text = String::new();
    for view in &comments.comments {
        let comment = &view.comment;
        text.push_str(&format!(
            "comment: {}\n  status: {} (version {})\n  original_revision: {}\n  original_sha256: {}\n  original_range: {}..{}\n  original_quote: {}\n  body: {}\n  anchor: {}\n",
            comment.id,
            status_str(comment.status),
            comment.status_version,
            comment.original_revision_id,
            sha256_hex(&comment.original_sha256),
            comment.original_start,
            comment.original_end,
            comment.original_quote,
            comment.body,
            anchor_text(&view.anchor),
        ));
    }
    Ok(Output {
        text,
        data: json!({
            "draft": draft_summary(&comments.draft),
            "comments": comments.comments.iter().map(comment_json).collect::<Vec<_>>(),
        }),
    })
}

fn revisions_list(workspace: &Workspace, draft_id: Uuid) -> Result<Output, CliFailure> {
    require_draft(workspace, draft_id)?;
    let revisions = workspace
        .draft_revisions(draft_id)
        .map_err(classify_workflow)?;
    let mut text = String::new();
    for revision in &revisions {
        text.push_str(&format!(
            "{} kind={} parent={}{}\n",
            revision.id,
            kind_str(revision.kind),
            revision
                .parent_id
                .map_or_else(|| "none".to_string(), |p| p.to_string()),
            revision
                .origin_turn
                .map_or_else(String::new, |t| format!(" origin={t}"))
        ));
    }
    Ok(Output {
        text,
        data: json!({
            "revisions": revisions.iter().map(revision_summary).collect::<Vec<_>>(),
        }),
    })
}

fn revisions_show(workspace: &Workspace, id: Uuid) -> Result<Output, CliFailure> {
    let revision = require_revision(workspace, id)?;
    let mut data = revision_summary(&revision);
    if let serde_json::Value::Object(map) = &mut data {
        map.insert("content".into(), json!(revision.text));
    }
    let text = ensure_trailing_newline(format!(
        "id: {}\ndraft_id: {}\nparent_id: {}\nkind: {}\norigin_turn: {}\nsha256: {}\n{}",
        revision.id,
        revision.draft_id,
        revision
            .parent_id
            .map_or_else(|| "none".to_string(), |p| p.to_string()),
        kind_str(revision.kind),
        revision
            .origin_turn
            .map_or_else(|| "none".to_string(), |t| t.to_string()),
        sha256_hex(&revision.sha256),
        revision.text
    ));
    Ok(Output { text, data })
}

fn revisions_diff(
    workspace: &Workspace,
    draft: Uuid,
    from: Uuid,
    to: Uuid,
) -> Result<Output, CliFailure> {
    require_draft(workspace, draft)?;
    require_revision(workspace, from)?;
    require_revision(workspace, to)?;
    let diff = workspace
        .compare_draft_revisions(draft, from, to)
        .map_err(classify_workflow)?;
    let text = ensure_trailing_newline(format!("draft {draft} diff {from} -> {to}\n{diff}"));
    Ok(Output {
        text,
        data: json!({ "draft_id": draft, "from": from, "to": to, "diff": diff }),
    })
}
