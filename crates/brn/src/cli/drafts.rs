//! Draft mutations over the shared workflow. Input is fully prepared and
//! validated BEFORE the workspace is opened, so invalid independent input
//! never initializes it; text files are read with a bounded read under the
//! existing draft-size limit and the exact bytes are carried in memory into
//! the workflow (no reread, no Unicode or line-ending normalization).
use crate::cli::{
    error::classify_workflow, error::CliError, input::prepared_workspace, review::draft_summary,
    CliFailure, Command, Invocation, Output,
};
use brn_workflow::Workspace;
use serde_json::json;
use std::path::Path;
use uuid::Uuid;

pub fn run(invocation: &Invocation) -> Result<Output, CliFailure> {
    match &invocation.command {
        Command::DraftsCreate {
            title,
            text_file,
            operation,
        } => create_run(invocation, title, text_file, *operation),
        Command::DraftsCheckpoint {
            draft,
            base_revision,
            expected_generation,
            generation,
            text_file,
            operation,
        } => checkpoint_run(
            invocation,
            *draft,
            *base_revision,
            *expected_generation,
            *generation,
            text_file,
            *operation,
        ),
        Command::DraftsSave {
            draft,
            base_revision,
            expected_generation,
            generation,
            text_file,
            operation,
        } => save_run(
            invocation,
            *draft,
            *base_revision,
            *expected_generation,
            *generation,
            text_file,
            *operation,
        ),
        _ => unreachable!("drafts module handles mutations only"),
    }
}

fn create_run(
    invocation: &Invocation,
    title: &str,
    text_file: &Path,
    operation: Option<Uuid>,
) -> Result<Output, CliFailure> {
    // Input preparation happens before anything is opened: an invalid title
    // or input file must never initialize a workspace. The title rule is the
    // store's own create_draft rule; the store re-checks it at the source of
    // truth, so persistence semantics are unchanged.
    if title.trim().is_empty() {
        return Err(CliError::Workflow("draft title is required".into()).into());
    }
    let text = read_text_file(text_file)?;
    let mut workspace = prepared_workspace(invocation)?;
    create(&mut workspace, title, &text, operation)
}

fn checkpoint_run(
    invocation: &Invocation,
    draft: Uuid,
    base_revision: Uuid,
    expected_generation: u64,
    generation: u64,
    text_file: &Path,
    operation: Option<Uuid>,
) -> Result<Output, CliFailure> {
    // The submitted text is explicit input captured before opening the
    // workspace; replay never rereads current draft content as its payload.
    let text = read_text_file(text_file)?;
    let mut workspace = prepared_workspace(invocation)?;
    checkpoint(
        &mut workspace,
        draft,
        brn_workflow::DraftStamp {
            base_revision,
            generation: expected_generation,
        },
        generation,
        &text,
        operation,
    )
}

fn save_run(
    invocation: &Invocation,
    draft: Uuid,
    base_revision: Uuid,
    expected_generation: u64,
    generation: u64,
    text_file: &Path,
    operation: Option<Uuid>,
) -> Result<Output, CliFailure> {
    // The submitted text is captured before opening the workspace; operation
    // replay therefore remains bound to the caller's original payload.
    let text = read_text_file(text_file)?;
    let mut workspace = prepared_workspace(invocation)?;
    save(
        &mut workspace,
        draft,
        brn_workflow::DraftStamp {
            base_revision,
            generation: expected_generation,
        },
        generation,
        &text,
        operation,
    )
}

fn create(
    workspace: &mut Workspace,
    title: &str,
    text: &str,
    operation: Option<Uuid>,
) -> Result<Output, CliFailure> {
    let op = operation.unwrap_or_else(Uuid::new_v4);
    let draft = workspace
        .create_draft(op, title, text)
        .map_err(classify_workflow)?;
    Ok(Output {
        text: format!(
            "created draft {} title={} generation={} base={} operation={}\n",
            draft.id, draft.title, draft.stamp.generation, draft.stamp.base_revision, op
        ),
        data: {
            let mut data = draft_summary(&draft);
            if let serde_json::Value::Object(map) = &mut data {
                map.insert("operation_id".into(), json!(op));
            }
            data
        },
    })
}

fn checkpoint(
    workspace: &mut Workspace,
    draft_id: Uuid,
    expected: brn_workflow::DraftStamp,
    generation: u64,
    text: &str,
    operation: Option<Uuid>,
) -> Result<Output, CliFailure> {
    let op = operation.unwrap_or_else(Uuid::new_v4);
    let draft = workspace
        .checkpoint_draft(op, draft_id, expected, generation, text)
        .map_err(classify_workflow)?;
    let checkpoint_id = draft.stamp.base_revision;
    Ok(Output {
        text: format!(
            "checkpointed draft {} generation={} checkpoint={} operation={}\n",
            draft.id, draft.stamp.generation, checkpoint_id, op
        ),
        data: {
            let mut data = draft_summary(&draft);
            if let serde_json::Value::Object(map) = &mut data {
                map.insert("checkpoint_id".into(), json!(checkpoint_id));
                map.insert("operation_id".into(), json!(op));
            }
            data
        },
    })
}

fn save(
    workspace: &mut Workspace,
    draft_id: Uuid,
    expected: brn_workflow::DraftStamp,
    generation: u64,
    text: &str,
    operation: Option<Uuid>,
) -> Result<Output, CliFailure> {
    let op = operation.unwrap_or_else(Uuid::new_v4);
    let draft = workspace
        .save_draft(op, draft_id, expected, generation, text)
        .map_err(classify_workflow)?;
    Ok(Output {
        text: format!(
            "saved draft {} generation={} base={} operation={}\n",
            draft.id, draft.stamp.generation, draft.stamp.base_revision, op
        ),
        data: {
            let mut data = draft_summary(&draft);
            if let serde_json::Value::Object(map) = &mut data {
                map.insert("operation_id".into(), json!(op));
            }
            data
        },
    })
}

fn read_text_file(path: &Path) -> Result<String, CliError> {
    super::input::read_text_file(path, "draft")
}
