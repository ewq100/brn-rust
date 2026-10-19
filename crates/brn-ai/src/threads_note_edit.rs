//! One exact local replacement. The host keeps the complete base out of model
//! context while reusing checked preparation, application, review and Undo.
use super::*;
use sha2::{Digest, Sha256};

#[derive(Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct EditArgs {
    pub id: String,
    pub version: u64,
    pub start: usize,
    pub end: usize,
    pub expected_text: String,
    pub replacement_text: String,
    pub request_key: String,
    pub reason: String,
}
pub(super) fn validate_arguments(raw: &Value) -> std::result::Result<(), ArgumentError> {
    let args = serde_json::from_value::<EditArgs>(raw.clone()).map_err(|_| {
        ArgumentError::Shape("Supply only id, version, start, end, expected_text, replacement_text, request_key and reason with the offered types.")
    })?;
    argument_shape(
        !args.id.is_empty() && args.id.len() <= 256 && args.version > 0,
        "id must contain 1..256 UTF-8 bytes and version must be positive.",
    )?;
    argument_shape(
        args.start < args.end && args.end - args.start == args.expected_text.len(),
        "Use one nonempty UTF-8 byte range start < end; expected_text must have exactly end-start bytes.",
    )?;
    argument_shape(
        !args.request_key.is_empty() && args.request_key.len() <= 128,
        "request_key must contain 1..128 UTF-8 bytes.",
    )?;
    argument_shape(!args.reason.trim().is_empty(), "reason must be nonblank.")
}
pub(super) fn schema() -> Value {
    object(
        json!({
            "id":{"type":"string","description":"Exact current Note ID."},
            "version":{"type":"integer","minimum":1},
            "start":{"type":"integer","minimum":0,"description":"Inclusive UTF-8 byte offset, never a character index."},
            "end":{"type":"integer","minimum":1,"description":"Exclusive UTF-8 byte offset; start < end."},
            "expected_text":{"type":"string","description":"Exact bytes at this range; no search, normalization or relocation."},
            "replacement_text":{"type":"string","description":"Replacement passage; empty deletes the range."},
            "request_key":{"type":"string","description":"1..128 UTF-8 bytes. Reuse only for identical arguments in this Run."},
            "reason":{"type":"string","description":"Nonblank explanation of this local change."}
        }),
        json!([
            "id",
            "version",
            "start",
            "end",
            "expected_text",
            "replacement_text",
            "request_key",
            "reason"
        ]),
    )
}
#[derive(Clone)]
pub(super) struct PrepareNoteEdit;
impl Tool for PrepareNoteEdit {
    const NAME: &'static str = "prepare_note_edit";
    type Args = EditArgs;
    type Output = Value;
    type Error = AiError;
    fn description(&self) -> String {
        "Prepare one exact local Note passage replacement or deletion at current version and UTF-8 byte range. Read the relevant passage, supply its exact expected_text and replacement_text; the host preserves untouched bytes and metadata. No fuzzy matching or write occurs. Full-document ranges require an actual whole read. Use apply_changes with its operation; all existing scope, protection, review, dirty-writing, stale and Undo rules remain. Unavailable for Rewrite or research helpers.".into()
    }
    fn parameters(&self) -> Value {
        schema()
    }
    async fn call(&self, cx: &mut ToolContext, args: EditArgs) -> AiResult<Value> {
        prepare_edit(scope(cx)?.as_ref(), args)
    }
}
/// Only this module can construct a local-edit proof. It contains the complete
/// host-built request, never an externally supplied flag or mutation mode.
pub(super) struct ExactEdit(ChangeRequest);
impl ExactEdit {
    pub(super) fn into_request(self) -> ChangeRequest {
        self.0
    }
}
fn invalid(reason: &str) -> Value {
    json!({"outcome":"Invalid","reason":reason})
}
pub(super) fn prepare_edit(scope: &Scope, args: EditArgs) -> AiResult<Value> {
    if scope.writing_snapshot.is_some() || scope.scout.is_some() {
        return Err(rejected());
    }
    validate_arguments(&serde_json::to_value(&args).map_err(storage)?).map_err(|_| rejected())?;
    let mut store = scope.store.lock().map_err(storage)?;
    live(scope, &store)?;
    // Check current access before exposing any target/version or replay output.
    if !store
        .record_visible_in_chat(&args.id, Some(&scope.chat))
        .map_err(storage)?
    {
        return Err(rejected());
    }
    let binding = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&args).map_err(storage)?)
    );
    let op = match store.allocate_bound_operation(
        &format!("{}:note-edit:{}", scope.run, args.request_key),
        &binding,
    ) {
        Ok(op) => op,
        Err(Error::ImmutableRequest) => {
            return Ok(invalid(
                "request_key is already bound to different exact-edit arguments; use a fresh key.",
            ));
        }
        Err(error) => return Err(storage(error)),
    };
    // Track even an interrupted allocation-to-prepare; retries recover the same
    // operation before looking at later canonical Markdown.
    update_run(scope, &mut store, |run| {
        if !run.operations.contains(&op) {
            run.operations.push(op.clone());
        }
    })?;
    if let Some(prepared) = store.operation_info(op.as_str()).map_err(storage)?.prepared {
        check_scope(scope, &store, &prepared.request)?;
        return prepared_output(&prepared);
    }
    let Some(record) = store.note(&args.id).map_err(storage)? else {
        return Err(rejected());
    };
    let RecordData::Note(mut note) = record.data else {
        return Err(rejected());
    };
    if record.archived || note.superseded_by.is_some() {
        return Err(rejected());
    }
    if record.version != args.version {
        return Ok(invalid(
            "The Note version changed; read the current passage and use a fresh request_key.",
        ));
    }
    if note.markdown.get(args.start..args.end) != Some(args.expected_text.as_str()) {
        return Ok(invalid(
            "The exact UTF-8 byte range does not match expected_text; no search or relocation was attempted.",
        ));
    }
    if args.start == 0
        && args.end == note.markdown.len()
        && !scope
            .reads
            .lock()
            .map_err(storage)?
            .knows(&args.id, args.version)
    {
        return Ok(invalid(
            "A full-document range requires a complete read at this version; use prepare_changes after reading the whole Note.",
        ));
    }
    note.markdown
        .replace_range(args.start..args.end, &args.replacement_text);
    let mut request = ChangeRequest {
        reason: args.reason,
        writes: vec![Put {
            id: args.id,
            expected_version: Some(args.version),
            archived: record.archived,
            data: RecordData::Note(note),
        }],
        inputs: vec![],
    };
    scope_request(scope, &store, &mut request)?;
    bind_project_inputs(scope, &store, &mut request)?;
    match prepare_checked(
        scope,
        &mut store,
        &op,
        PreparationInput::Exact(ExactEdit(request)),
    )? {
        Ok(prepared) => prepared_output(&prepared),
        Err(invalid) => Ok(invalid),
    }
}
