//! `drafts create --title TITLE --text-file PATH [--operation UUID]`: the
//! draft-creation mutation over the shared workflow. Input is fully prepared
//! and validated BEFORE the workspace is opened, so invalid independent
//! input never initializes it; the text file is read with a bounded read
//! under the existing draft-size limit and the exact bytes are carried in
//! memory into `Workspace::create_draft` (no reread, no Unicode or
//! line-ending normalization). Empty text is permitted; the blank-title rule
//! is the workflow's own and is re-checked by the store.
use crate::cli::{
    error::classify_workflow, error::CliError, open_workspace, review::draft_summary, CliFailure,
    Command, Invocation, Output,
};
use brn_workflow::{Workspace, MAX_DRAFT_BYTES};
use serde_json::json;
use std::{fs, io::Read, path::Path, sync::atomic::Ordering};
use uuid::Uuid;

pub fn run(invocation: &Invocation) -> Result<Output, CliFailure> {
    let Command::DraftsCreate {
        title,
        text_file,
        operation,
    } = &invocation.command
    else {
        unreachable!("drafts module handles drafts create only");
    };
    // Input preparation happens before anything is opened: an invalid title
    // or input file must never initialize a workspace. The title rule is the
    // store's own create_draft rule; the store re-checks it at the source of
    // truth, so persistence semantics are unchanged.
    if title.trim().is_empty() {
        return Err(CliError::Workflow("draft title is required".into()).into());
    }
    // The exact validated text is held in memory and passed on unchanged.
    let text = read_text_file(text_file)?;
    // A signal that has arrived by the end of input preparation must prevent
    // the workspace from even being opened.
    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::Interrupted(
            "interrupted during input preparation; the command was not run".into(),
        )
        .into());
    }
    let mut workspace = open_workspace(invocation)?;
    // Opening the workspace can block up to the store's lock retry window;
    // a signal arriving during that wait must still prevent the mutation.
    // This is the last check before the mutation starts.
    if crate::CANCEL.load(Ordering::SeqCst) {
        return Err(CliError::Interrupted(
            "interrupted while acquiring the workspace; the command was not run".into(),
        )
        .into());
    }
    create(&mut workspace, title, &text, *operation)
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

/// Bounded read of a regular UTF-8 file at most `MAX_DRAFT_BYTES` long,
/// following the shared `import_file` read pattern. Exact bytes are kept:
/// no newline or Unicode normalization. Empty text stays permitted (the
/// import-only nonempty rule is deliberately not imported here).
fn read_text_file(path: &Path) -> Result<String, CliError> {
    fn io(error: std::io::Error) -> CliError {
        CliError::Workflow(error.to_string())
    }
    let meta = fs::metadata(path).map_err(io)?;
    if !meta.is_file() || meta.len() > MAX_DRAFT_BYTES as u64 {
        return Err(CliError::Workflow(
            "drafts create requires a regular UTF-8 text file up to 1 MiB".into(),
        ));
    }
    let mut file = fs::File::open(path).map_err(io)?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((MAX_DRAFT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() > MAX_DRAFT_BYTES {
        return Err(CliError::Workflow("draft text exceeds 1 MiB".into()));
    }
    String::from_utf8(bytes)
        .map_err(|_| CliError::Workflow("draft text file is not valid UTF-8".into()))
}
