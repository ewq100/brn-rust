//! One exact local replacement. The host keeps the complete base out of model
//! context while reusing checked preparation, application, review and Undo.
use super::*;
use sha2::{Digest, Sha256};

#[derive(Deserialize, serde::Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub(super) struct EditArgs {
    pub id: String,
    pub version: u64,
    #[serde(deserialize_with = "required_nullable_usize")]
    pub start: Option<usize>,
    #[serde(deserialize_with = "required_nullable_usize")]
    pub end: Option<usize>,
    pub expected_text: String,
    pub replacement_text: String,
    pub request_key: String,
    pub reason: String,
}
fn required_nullable_usize<'de, D>(deserializer: D) -> std::result::Result<Option<usize>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<usize>::deserialize(deserializer)
}
pub(super) fn validate_arguments(raw: &Value) -> std::result::Result<(), ArgumentError> {
    let args = serde_json::from_value::<EditArgs>(raw.clone()).map_err(|_| {
        ArgumentError::Shape("Supply only id, version, start, end, expected_text, replacement_text, request_key and reason with the offered types.")
    })?;
    argument_shape(
        !args.id.is_empty() && args.id.len() <= 256 && args.version > 0,
        "id must contain 1..256 UTF-8 bytes and version must be positive.",
    )?;
    match (args.start, args.end) {
        (Some(start), Some(end)) => argument_shape(
            start < end && end - start == args.expected_text.len(),
            "Use one nonempty UTF-8 byte range start < end; expected_text must have exactly end-start bytes.",
        )?,
        (None, None) => argument_shape(
            !args.expected_text.is_empty(),
            "Quote selection requires nonempty expected_text.",
        )?,
        _ => {
            return Err(ArgumentError::Shape(
                "Set both start and end to byte offsets, or set both to null for one unique exact quote.",
            ));
        }
    }
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
            "start":{"type":["integer","null"],"minimum":0,"description":"Inclusive UTF-8 byte offset; use null together with end for unique exact-quote selection."},
            "end":{"type":["integer","null"],"minimum":1,"description":"Exclusive UTF-8 byte offset; use null together with start for unique exact-quote selection."},
            "expected_text":{"type":"string","description":"Exact bytes at the range, or a nonempty exact quote occurring once in the current Note when both offsets are null."},
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
        "Prepare one exact local Note passage replacement or deletion at current version. Supply a UTF-8 byte range or set both offsets null for a nonempty exact quote occurring once in the current Note; duplicates require an explicit range. The host preserves untouched bytes and metadata. No fuzzy matching or write occurs. Full-document ranges require an actual whole read. Use a fresh request_key whenever arguments change. Use apply_changes with its operation; all existing scope, protection, review, dirty-writing, stale and Undo rules remain. Unavailable for Rewrite or research helpers.".into()
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
    let range = match (args.start, args.end) {
        (Some(start), Some(end)) => {
            if note.markdown.get(start..end) != Some(args.expected_text.as_str()) {
                return Ok(invalid(
                    "The exact UTF-8 byte range does not match expected_text; no search or relocation was attempted.",
                ));
            }
            start..end
        }
        (None, None) => {
            match super::threads_text::unique_range(&note.markdown, &args.expected_text) {
                Some(range) => range,
                None => {
                    return Ok(invalid(
                        "expected_text must occur exactly once as a nonempty exact quote in the current Note; no candidate was prepared.",
                    ));
                }
            }
        }
        _ => {
            return Ok(invalid(
                "Set both start and end to byte offsets, or set both to null for unique exact-quote selection.",
            ));
        }
    };
    if range.start == 0
        && range.end == note.markdown.len()
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
    note.markdown.replace_range(range, &args.replacement_text);
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
