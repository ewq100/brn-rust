//! Comment mutations over the shared workflow.
//!
//! `comments add` deliberately captures only already-saved, unchanged draft
//! text in this first CLI slice. The store remains responsible for the
//! operation receipt, expected-state check, exact anchor validation and the
//! checkpoint/comment transaction.
use crate::cli::{
    error::classify_workflow, error::CliError, open_workspace, review::comment_fields,
    review::comment_json, review::draft_summary, CliFailure, Command, Invocation, Output,
};
use brn_workflow::{
    CommentCapture, CommentStatus, CommentStatusChange, EditTrace, MAX_COMMENT_BODY_BYTES,
    MAX_DRAFT_BYTES,
};
use serde_json::json;
use std::{fs, io::Read, ops::Range, path::Path, sync::atomic::Ordering};
use uuid::Uuid;

pub fn run(invocation: &Invocation) -> Result<Output, CliFailure> {
    match &invocation.command {
        Command::CommentsAdd { .. } => add(invocation),
        Command::CommentsResolve {
            comment,
            draft,
            expected_status_version,
            operation,
        } => resolve(
            invocation,
            *comment,
            *draft,
            *expected_status_version,
            *operation,
        ),
        Command::CommentsReopen {
            comment,
            draft,
            expected_status_version,
            operation,
        } => reopen(
            invocation,
            *comment,
            *draft,
            *expected_status_version,
            *operation,
        ),
        _ => unreachable!("comments module handles mutations only"),
    }
}

fn add(invocation: &Invocation) -> Result<Output, CliFailure> {
    let Command::CommentsAdd {
        draft,
        base_revision,
        expected_generation,
        generation,
        text_file,
        start_byte,
        end_byte,
        quote_file,
        body_file,
        operation,
    } = &invocation.command
    else {
        unreachable!("comments module handles add")
    };

    let text = read_utf8_file(
        text_file,
        MAX_DRAFT_BYTES,
        "comment text requires a regular UTF-8 text file up to 1 MiB",
    )?;
    let quote = read_utf8_file(
        quote_file,
        MAX_DRAFT_BYTES,
        "comment quote requires a regular UTF-8 text file up to 1 MiB",
    )?;
    let body = read_utf8_file(
        body_file,
        MAX_COMMENT_BODY_BYTES,
        "comment body requires a regular UTF-8 text file up to 64 KiB",
    )?;
    if body.trim().is_empty() {
        return Err(
            CliError::Workflow("comment body must contain text within 64 KiB".into()).into(),
        );
    }
    validate_quote(&text, *start_byte..*end_byte, &quote)?;

    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::Interrupted(
            "interrupted during input preparation; the command was not run".into(),
        )
        .into());
    }
    let mut workspace = open_workspace(invocation)?;
    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::Interrupted(
            "interrupted while acquiring the workspace; the command was not run".into(),
        )
        .into());
    }

    let op = operation.unwrap_or_else(Uuid::new_v4);
    let result = workspace
        .create_draft_comment(CommentCapture {
            op,
            draft_id: *draft,
            expected: brn_workflow::DraftStamp {
                base_revision: *base_revision,
                generation: *expected_generation,
            },
            generation: *generation,
            text,
            edits: EditTrace::Steps(Vec::new()),
            range: *start_byte..*end_byte,
            quote,
            body,
        })
        .map_err(classify_workflow)?;

    Ok(Output {
        text: format!(
            "added comment {} to draft {} generation={} operation={}\n",
            result.comment_id, result.saved.draft.id, result.submitted_generation, result.op
        ),
        data: json!({
            "operation_id": result.op,
            "comment_id": result.comment_id,
            "submitted_generation": result.submitted_generation,
            "draft": draft_summary(&result.saved.draft),
            "comments": result.saved.comments.iter().map(comment_json).collect::<Vec<_>>(),
        }),
    })
}

fn resolve(
    invocation: &Invocation,
    comment_id: Uuid,
    draft_id: Uuid,
    expected_status_version: u64,
    operation: Option<Uuid>,
) -> Result<Output, CliFailure> {
    let mut workspace = open_workspace(invocation)?;
    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::Interrupted(
            "interrupted while acquiring the workspace; the command was not run".into(),
        )
        .into());
    }

    let op = operation.unwrap_or_else(Uuid::new_v4);
    let result = workspace
        .set_comment_status(CommentStatusChange {
            op,
            draft_id,
            comment_id,
            expected_status_version,
            status: CommentStatus::Resolved,
        })
        .map_err(classify_workflow)?;

    Ok(Output {
        text: format!(
            "resolved comment {} status_version={} operation={}\n",
            result.comment.id, result.comment.status_version, result.op
        ),
        data: json!({
            "operation_id": result.op,
            "comment": comment_fields(&result.comment),
        }),
    })
}

fn reopen(
    invocation: &Invocation,
    comment_id: Uuid,
    draft_id: Uuid,
    expected_status_version: u64,
    operation: Option<Uuid>,
) -> Result<Output, CliFailure> {
    let mut workspace = open_workspace(invocation)?;
    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::Interrupted(
            "interrupted while acquiring the workspace; the command was not run".into(),
        )
        .into());
    }

    let op = operation.unwrap_or_else(Uuid::new_v4);
    let result = workspace
        .set_comment_status(CommentStatusChange {
            op,
            draft_id,
            comment_id,
            expected_status_version,
            status: CommentStatus::Open,
        })
        .map_err(classify_workflow)?;

    Ok(Output {
        text: format!(
            "reopened comment {} status_version={} operation={}\n",
            result.comment.id, result.comment.status_version, result.op
        ),
        data: json!({
            "operation_id": result.op,
            "comment": comment_fields(&result.comment),
        }),
    })
}

fn read_utf8_file(path: &Path, max_bytes: usize, message: &str) -> Result<String, CliError> {
    let metadata = fs::metadata(path).map_err(|error| CliError::Workflow(error.to_string()))?;
    if !metadata.is_file() || metadata.len() > max_bytes as u64 {
        return Err(CliError::Workflow(message.into()));
    }
    let mut file = fs::File::open(path).map_err(|error| CliError::Workflow(error.to_string()))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| CliError::Workflow(error.to_string()))?;
    if bytes.len() > max_bytes {
        return Err(CliError::Workflow(message.into()));
    }
    String::from_utf8(bytes).map_err(|_| CliError::Workflow(message.into()))
}

fn validate_quote(text: &str, range: Range<usize>, quote: &str) -> Result<(), CliFailure> {
    if quote.is_empty()
        || range.start >= range.end
        || range.end > text.len()
        || !text.is_char_boundary(range.start)
        || !text.is_char_boundary(range.end)
        || text.get(range) != Some(quote)
    {
        return Err(
            CliError::Workflow("comment selection does not match exact quote".into()).into(),
        );
    }
    Ok(())
}
