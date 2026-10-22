//! Synthetic handler and provider-history witnesses for exact local changes.
use super::*;
use note_edit::{EditArgs, prepare_edit};

fn args(
    note: &str,
    version: u64,
    start: usize,
    expected: &str,
    replacement: &str,
    key: &str,
) -> EditArgs {
    EditArgs {
        id: note.into(),
        version,
        start: Some(start),
        end: Some(start + expected.len()),
        expected_text: expected.into(),
        replacement_text: replacement.into(),
        request_key: key.into(),
        reason: "Correct the exact local passage".into(),
    }
}
fn quote_args(note: &str, version: u64, quote: &str, replacement: &str, key: &str) -> EditArgs {
    EditArgs {
        id: note.into(),
        version,
        start: None,
        end: None,
        expected_text: quote.into(),
        replacement_text: replacement.into(),
        request_key: key.into(),
        reason: "Correct the exact local passage".into(),
    }
}
fn legacy_binding(args: &EditArgs) -> String {
    use sha2::{Digest, Sha256};
    let Some(start) = args.start else {
        panic!("legacy binding needs a range")
    };
    let Some(end) = args.end else {
        panic!("legacy binding needs a range")
    };
    let bytes = format!(
        "{{\"id\":{},\"version\":{},\"start\":{},\"end\":{},\"expected_text\":{},\"replacement_text\":{},\"request_key\":{},\"reason\":{}}}",
        serde_json::to_string(&args.id).unwrap(),
        args.version,
        start,
        end,
        serde_json::to_string(&args.expected_text).unwrap(),
        serde_json::to_string(&args.replacement_text).unwrap(),
        serde_json::to_string(&args.request_key).unwrap(),
        serde_json::to_string(&args.reason).unwrap(),
    );
    format!("{:x}", Sha256::digest(bytes.as_bytes()))
}
fn owner_note(request: &ThreadRunRequest, note: &str, data: Note) -> Record {
    let mut store = Store::open(&request.data_dir).unwrap();
    let current = store.record(note).unwrap().unwrap();
    let op = store.allocate_operation(&unique_host_key()).unwrap();
    let owner = HostAuthority::owner("synthetic fixture");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Synthetic note fixture".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: note.into(),
                    expected_version: Some(current.version),
                    archived: false,
                    data: RecordData::Note(data),
                }],
            },
            &owner,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    store.record(note).unwrap().unwrap()
}
fn current(scope: &Scope, note: &str) -> Record {
    scope.store.lock().unwrap().record(note).unwrap().unwrap()
}
fn apply_op(scope: &Scope, prepared: &Value) -> Value {
    apply(
        scope,
        ApplyArgs {
            operation: prepared["operation"].as_str().unwrap().into(),
        },
    )
    .unwrap()
}
fn ordinary(scope: &Scope, note: &str, version: u64, key: &str) -> Value {
    prepare(
        scope,
        PrepareArgs {
            writing: None,
            request_key: key.into(),
            request: ChangeRequest {
                reason: "Ordinary whole replacement".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: note.into(),
                    expected_version: Some(version),
                    archived: false,
                    data: RecordData::Note(Note::working("Decision", "A whole replacement")),
                }],
            },
        },
    )
    .unwrap()
}
fn complete_read(scope: &Scope, note: &str, version: u64) {
    let mut start = 0;
    loop {
        let read = read_note(
            scope,
            ReadNoteArgs {
                id: note.into(),
                version,
                start,
                max_bytes: MAX_NOTE_READ_BYTES,
                section: false,
            },
        )
        .unwrap();
        assert_eq!(read["outcome"], "Read");
        match read["note"]["next_start"].as_u64() {
            Some(next) => start = next as usize,
            None => break,
        }
    }
}
#[test]
fn large_note_local_edit_preserves_exact_bytes_metadata_and_incomplete_coverage_on_replay_continue()
{
    let (_root, request, note) = tests::fixture();
    let prefix = "Prefix Ω\r\n".repeat(22000);
    let suffix = "\r\nSuffix e\u{301} Ω".repeat(9000);
    let markdown = format!("{prefix}Monday{suffix}");
    let mut data = Note::working("Long precise title", &markdown);
    let source = {
        let mut store = Store::open(&request.data_dir).unwrap();
        let op = store.allocate_operation("synthetic provenance").unwrap();
        let id = op.creation_id(0);
        let owner = HostAuthority::owner("fixture");
        store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "Synthetic source".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Source(Source {
                            chat: None,
                            locator: "synthetic://source".into(),
                            label: "Retained source".into(),
                            observed_at: "2026-10-19".into(),
                            external_version: None,
                            outcome: "Partial".into(),
                            gaps: vec!["qualification".into()],
                            report: None,
                        }),
                    }],
                },
                &owner,
            )
            .unwrap();
        assert!(matches!(
            store.apply(&op, &owner).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        id
    };
    data.import = Some(ImportCoverage {
        source,
        full_note: false,
        gaps: vec!["original qualification".into()],
        report: None,
    });
    let before = owner_note(&request, &note, data.clone());
    let (scope, _) = begin(&request).unwrap();
    let read = read_note(
        &scope,
        ReadNoteArgs {
            id: note.clone(),
            version: before.version,
            start: prefix.len(),
            max_bytes: 6,
            section: false,
        },
    )
    .unwrap();
    assert_eq!(read["note"]["markdown"], "Monday");
    assert!(!scope.reads.lock().unwrap().knows(&note, before.version));
    let edit = args(
        &note,
        before.version,
        prefix.len(),
        "Monday",
        "Tuesday",
        "local",
    );
    let legacy_op = scope
        .store
        .lock()
        .unwrap()
        .allocate_bound_operation(
            &format!("{}:note-edit:{}", scope.run, edit.request_key),
            &legacy_binding(&edit),
        )
        .unwrap();
    let prepared = prepare_edit(&scope, edit.clone()).unwrap();
    assert_eq!(prepared["operation"], legacy_op.as_str());
    assert!(context_budget::result_fits(&prepared));
    assert!(!prepared.to_string().contains("Suffix"));
    assert_eq!(
        ordinary(&scope, &note, before.version, "unread-base")["outcome"],
        "Invalid"
    );
    let applied = apply_op(&scope, &prepared);
    assert_eq!(applied["outcome"], "Applied");
    let after = current(&scope, &note);
    data.markdown = format!("{prefix}Tuesday{suffix}");
    assert_eq!(after.data, RecordData::Note(data));
    assert_eq!(after.version, before.version + 1);
    assert!(!scope.reads.lock().unwrap().knows(&note, after.version));
    assert_eq!(
        prepare_edit(&scope, edit.clone()).unwrap(),
        prepared,
        "immutable preparation survives canonical change"
    );
    assert_eq!(apply_op(&scope, &prepared), applied);
    assert_eq!(
        ordinary(&scope, &note, after.version, "unread-applied")["outcome"],
        "Invalid"
    );
    drop(scope);
    let (continued, input) = begin(&request).unwrap();
    assert!(
        input.contains(prepared["operation"].as_str().unwrap()),
        "Continue recovers operation identity"
    );
    assert_eq!(apply_op(&continued, &prepared)["outcome"], "Applied");
    assert!(!continued.reads.lock().unwrap().knows(&note, after.version));
    assert_eq!(
        ordinary(&continued, &note, after.version, "unread-continue")["outcome"],
        "Invalid"
    );
    // A fresh Run has its own key namespace, not a claim of old-key replay.
    assert_eq!(
        prepare_edit(&continued, edit).unwrap()["outcome"],
        "Invalid"
    );
    complete_read(&continued, &note, after.version);
    assert!(
        ordinary(&continued, &note, after.version, "read-continue")
            .get("operation")
            .is_some()
    );
}
#[test]
fn repeated_unicode_passage_changes_only_exact_occurrence_and_allows_deletion() {
    let (_root, request, note) = tests::fixture();
    let before = owner_note(
        &request,
        &note,
        Note::working("Decision", "Ω echo e\u{301}\r\nΩ echo e\u{301}\r\n"),
    );
    let (scope, _) = begin(&request).unwrap();
    let prepared = prepare_edit(
        &scope,
        args(
            &note,
            before.version,
            "Ω echo e\u{301}\r\nΩ ".len(),
            "echo",
            "",
            "second",
        ),
    )
    .unwrap();
    assert_eq!(apply_op(&scope, &prepared)["outcome"], "Applied");
    assert!(
        matches!(current(&scope, &note).data, RecordData::Note(n) if n.markdown == "Ω echo e\u{301}\r\nΩ  e\u{301}\r\n")
    );
}
#[test]
fn unique_quote_uses_exact_current_note_bytes_and_refuses_missing_or_overlapping_matches() {
    let (_root, request, note) = tests::fixture();
    let before = owner_note(
        &request,
        &note,
        Note::working("Decision", "Ω lead\r\nMonday\r\ntail aaa"),
    );
    let (scope, _) = begin(&request).unwrap();
    let sliced = read_note(
        &scope,
        ReadNoteArgs {
            id: note.clone(),
            version: before.version,
            start: "Ω lead\r\n".len(),
            max_bytes: 6,
            section: false,
        },
    )
    .unwrap();
    assert_eq!(sliced["note"]["markdown"], "Monday");
    assert!(!scope.reads.lock().unwrap().knows(&note, before.version));
    assert_eq!(
        prepare_edit(
            &scope,
            args(&note, before.version, 0, "Monday", "Wrong", "no-fallback"),
        )
        .unwrap()["outcome"],
        "Invalid",
        "an incorrect integer range must not activate unique-quote selection"
    );
    let prepared = prepare_edit(
        &scope,
        quote_args(&note, before.version, "Monday", "Tuesday", "unique"),
    )
    .unwrap();
    assert_eq!(
        prepare_edit(
            &scope,
            quote_args(&note, before.version, "Monday", "WED", "unique"),
        )
        .unwrap()["outcome"],
        "Invalid",
        "changing a prepared selector requires a fresh key"
    );
    assert_eq!(
        prepare_edit(
            &scope,
            args(
                &note,
                before.version,
                "Ω lead\r\n".len(),
                "Monday",
                "Tuesday",
                "unique",
            ),
        )
        .unwrap()["outcome"],
        "Invalid",
        "changing from quote selection to an integer range requires a fresh key"
    );
    assert_eq!(apply_op(&scope, &prepared)["outcome"], "Applied");
    assert!(!scope.reads.lock().unwrap().knows(&note, before.version + 1));
    assert_eq!(
        prepare_edit(
            &scope,
            quote_args(&note, before.version, "Monday", "Tuesday", "unique"),
        )
        .unwrap(),
        prepared,
        "a prepared null-selector edit replays before reading later canonical text"
    );
    assert!(!scope.reads.lock().unwrap().knows(&note, before.version + 1));
    assert_eq!(apply_op(&scope, &prepared)["outcome"], "Applied");
    assert!(
        matches!(current(&scope, &note).data, RecordData::Note(n) if n.markdown == "Ω lead\r\nTuesday\r\ntail aaa")
    );
    let after = current(&scope, &note);
    assert_eq!(
        ordinary(&scope, &note, after.version, "quote-still-partial")["outcome"],
        "Invalid",
        "quote search and Apply replay do not promote partial read coverage"
    );
    let candidate_count = scope.store.lock().unwrap().candidates().unwrap().len();
    for (quote, key) in [("absent", "missing"), ("aa", "overlap")] {
        let result =
            prepare_edit(&scope, quote_args(&note, after.version, quote, "X", key)).unwrap();
        assert_eq!(result["outcome"], "Invalid");
        assert!(result.get("operation").is_none());
        assert_eq!(
            scope.store.lock().unwrap().candidates().unwrap().len(),
            candidate_count
        );
    }
    assert_eq!(current(&scope, &note), after);
}
#[test]
fn nullable_offsets_are_required_and_unique_full_note_quote_keeps_full_read_guard() {
    use serde_json::json;
    let base = serde_json::to_value(args("n", 1, 0, "x", "y", "k")).unwrap();
    assert!(note_edit::validate_arguments(&base).is_ok());
    let mut missing_both = base.clone();
    missing_both.as_object_mut().unwrap().remove("start");
    missing_both.as_object_mut().unwrap().remove("end");
    let mut missing_start = base.clone();
    missing_start.as_object_mut().unwrap().remove("start");
    let mut missing_end = base.clone();
    missing_end.as_object_mut().unwrap().remove("end");
    let mut null_start = base.clone();
    null_start["start"] = json!(null);
    let mut null_end = base.clone();
    null_end["end"] = json!(null);
    let mut wrong_scalar = base.clone();
    wrong_scalar["start"] = json!("0");
    let mut unknown = base.clone();
    unknown["extra"] = json!(true);
    for invalid in [
        missing_both,
        missing_start,
        missing_end,
        wrong_scalar,
        unknown,
    ] {
        assert!(serde_json::from_value::<EditArgs>(invalid.clone()).is_err());
        assert!(note_edit::validate_arguments(&invalid).is_err());
    }
    for invalid_pair in [null_start, null_end] {
        let parsed = serde_json::from_value::<EditArgs>(invalid_pair.clone()).unwrap();
        assert!(note_edit::validate_arguments(&invalid_pair).is_err());
        let (_root, request, note) = tests::fixture();
        let (scope, _) = begin(&request).unwrap();
        let before = scope.store.lock().unwrap().record(&scope.run).unwrap();
        let mut parsed = parsed;
        parsed.id = note;
        assert!(prepare_edit(&scope, parsed).is_err());
        assert_eq!(
            scope.store.lock().unwrap().record(&scope.run).unwrap(),
            before
        );
        assert!(scope.store.lock().unwrap().candidates().unwrap().is_empty());
    }
    let mut empty_quote = base.clone();
    empty_quote["start"] = json!(null);
    empty_quote["end"] = json!(null);
    empty_quote["expected_text"] = json!("");
    assert!(note_edit::validate_arguments(&empty_quote).is_err());
    let schema = note_edit::schema();
    assert_eq!(schema["required"].as_array().unwrap().len(), 8);
    assert_eq!(
        schema["properties"]["start"]["type"],
        json!(["integer", "null"])
    );

    let (_root, request, note) = tests::fixture();
    let record = owner_note(&request, &note, Note::working("Decision", "Monday"));
    let (scope, _) = begin(&request).unwrap();
    assert_eq!(
        prepare_edit(
            &scope,
            quote_args(&note, record.version, "Monday", "Tuesday", "whole")
        )
        .unwrap()["outcome"],
        "Invalid"
    );
    complete_read(&scope, &note, record.version);
    let prepared = prepare_edit(
        &scope,
        quote_args(&note, record.version, "Monday", "Tuesday", "whole-read"),
    )
    .unwrap();
    assert!(prepared["operation"].is_string());
}
#[tokio::test]
async fn rig_typed_adapter_rejects_each_missing_nullable_key_before_effects() {
    use rig::tool::ErasedTool;
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let before = current(&scope, &note);
    let run_ops_before = match scope
        .store
        .lock()
        .unwrap()
        .record(&scope.run)
        .unwrap()
        .unwrap()
        .data
    {
        RecordData::Run(run) => run.operations,
        _ => panic!("run record missing"),
    };
    let candidate_count = scope.store.lock().unwrap().candidates().unwrap().len();
    for missing in ["start", "end"] {
        let mut raw =
            serde_json::to_value(args(&note, before.version, 0, "Mon", "Tues", "typed")).unwrap();
        raw.as_object_mut().unwrap().remove(missing);
        let mut context = ToolContext::new().with_scope(scope.clone());
        let result =
            ErasedTool::execute(&note_edit::PrepareNoteEdit, raw.to_string(), &mut context).await;
        assert!(result.is_error_kind(rig::tool::ToolErrorKind::InvalidArgs));
    }
    let mut raw =
        serde_json::to_value(args(&note, before.version, 0, "Mon", "Tues", "typed")).unwrap();
    raw.as_object_mut().unwrap().remove("start");
    raw.as_object_mut().unwrap().remove("end");
    let mut context = ToolContext::new().with_scope(scope.clone());
    let result =
        ErasedTool::execute(&note_edit::PrepareNoteEdit, raw.to_string(), &mut context).await;
    assert!(result.is_error_kind(rig::tool::ToolErrorKind::InvalidArgs));
    assert_eq!(current(&scope, &note), before);
    assert_eq!(
        scope.store.lock().unwrap().candidates().unwrap().len(),
        candidate_count
    );
    let RecordData::Run(run_ops_after) = scope
        .store
        .lock()
        .unwrap()
        .record(&scope.run)
        .unwrap()
        .unwrap()
        .data
    else {
        panic!("run record missing")
    };
    assert_eq!(run_ops_after.operations, run_ops_before);
}
#[test]
fn wrong_text_nonboundary_unicode_invalid_range_and_stale_base_never_mutate_or_relocate() {
    let (_root, request, note) = tests::fixture();
    let before = owner_note(&request, &note, Note::working("Decision", "Ω echo echo"));
    let (scope, _) = begin(&request).unwrap();
    for edit in [
        args(&note, before.version, 3, "xxxx", "X", "wrong"),
        args(&note, before.version, 1, "x", "X", "unicode"),
        args(&note, before.version, 90, "echo", "X", "bounds"),
        args(&note, before.version - 1, 3, "echo", "X", "stale"),
    ] {
        assert_eq!(prepare_edit(&scope, edit).unwrap()["outcome"], "Invalid");
    }
    let mut edit = args(&note, before.version, 3, "echo", "X", "empty");
    edit.end = edit.start;
    assert!(prepare_edit(&scope, edit).is_err());
    assert_eq!(current(&scope, &note), before);
}
#[test]
fn bound_arguments_conflict_even_before_prepare_and_after_success() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let edit = args(&note, 1, 0, "Mon", "Tues", "allocation-gap");
    let binding = legacy_binding(&edit);
    let op = scope
        .store
        .lock()
        .unwrap()
        .allocate_bound_operation(
            &format!("{}:note-edit:{}", scope.run, edit.request_key),
            &binding,
        )
        .unwrap();
    let mut changed = edit.clone();
    changed.reason = "Different exact reason".into();
    assert_eq!(prepare_edit(&scope, changed).unwrap()["outcome"], "Invalid");
    let prepared = prepare_edit(&scope, edit.clone()).unwrap();
    assert_eq!(prepared["operation"], op.as_str());
    let mut changed = edit;
    changed.replacement_text = "Wednes".into();
    assert_eq!(prepare_edit(&scope, changed).unwrap()["outcome"], "Invalid");
    assert_eq!(current(&scope, &note).version, 1);
}
#[test]
fn legacy_range_serialization_and_fingerprint_match_independent_base_golden() {
    use sha2::{Digest, Sha256};
    let args = EditArgs {
        id: "legacy-note:0".into(),
        version: 7,
        start: Some(3),
        end: Some(6),
        expected_text: "abc".into(),
        replacement_text: "XY".into(),
        request_key: "legacy-key".into(),
        reason: "Correct the exact local passage".into(),
    };
    let encoded = serde_json::to_vec(&args).unwrap();
    assert_eq!(encoded, br#"{"id":"legacy-note:0","version":7,"start":3,"end":6,"expected_text":"abc","replacement_text":"XY","request_key":"legacy-key","reason":"Correct the exact local passage"}"#);
    assert_eq!(
        format!("{:x}", Sha256::digest(&encoded)),
        "13823d0d216c0e2dad4ca192712be2acf333e123fbcbfdda00048bb6b3f0fc10"
    );
}
#[test]
fn full_range_requires_whole_read_and_known_base_promotes_after_local_apply() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let edit = args(&note, 1, 0, "Monday", "Tuesday", "full");
    assert_eq!(
        prepare_edit(&scope, edit.clone()).unwrap()["outcome"],
        "Invalid"
    );
    complete_read(&scope, &note, 1);
    let prepared = prepare_edit(&scope, edit).unwrap();
    assert_eq!(apply_op(&scope, &prepared)["outcome"], "Applied");
    assert!(scope.reads.lock().unwrap().knows(&note, 2));
    assert!(
        ordinary(&scope, &note, 2, "known")
            .get("operation")
            .is_some()
    );
}
#[test]
fn protected_local_edit_needs_exact_review_or_scoped_owner_instruction_and_undo_is_checked() {
    for direct in [false, true] {
        let (_root, mut request, note) = tests::fixture();
        let mut data = Note::working("Decision", "Monday");
        data.protected = true;
        data.confirmed = true;
        let before = owner_note(&request, &note, data);
        if direct {
            request.owner_request = Some(OwnerRequest::EditNote {
                id: note.clone(),
                version: before.version,
            });
        }
        let (scope, _) = begin(&request).unwrap();
        let prepared = prepare_edit(
            &scope,
            args(&note, before.version, 0, "Mon", "Tues", "protected"),
        )
        .unwrap();
        let outcome = apply_op(&scope, &prepared);
        assert_eq!(
            outcome["outcome"],
            if direct { "Applied" } else { "NeedsReview" }
        );
        let mut store = scope.store.lock().unwrap();
        let op = store
            .operation(prepared["operation"].as_str().unwrap())
            .unwrap();
        if !direct {
            assert_eq!(store.record(&note).unwrap().unwrap(), before);
            let review = HostAuthority::owner("exact candidate review");
            assert!(matches!(
                store.apply(&op, &review).unwrap(),
                ApplyOutcome::Applied(_)
            ));
        }
        assert!(
            matches!(store.record(&note).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Tuesday" && n.protected && n.confirmed != direct)
        );
        let undo = store.allocate_operation("undo-local").unwrap();
        assert!(matches!(
            store
                .undo(&undo, &op, &HostAuthority::owner("Undo"))
                .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        assert_eq!(store.record(&note).unwrap().unwrap().data, before.data);
    }
}
#[test]
fn dirty_writing_defers_then_saved_owner_text_stales_local_candidate() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let prepared = prepare_edit(&scope, args(&note, 1, 0, "Mon", "Tues", "dirty")).unwrap();
    let mut writer = Store::open(&request.data_dir).unwrap();
    let session = writer.begin_edit(&note, 1).unwrap();
    writer
        .update_buffer(&session.id, 1, "Owner's unsaved writing Ω")
        .unwrap();
    assert_eq!(apply_op(&scope, &prepared)["outcome"], "Deferred");
    assert_eq!(
        writer.edit_session(&session.id).unwrap().markdown,
        "Owner's unsaved writing Ω"
    );
    assert!(matches!(
        writer.save(&session.id, 1).unwrap(),
        SaveOutcome::Saved(_)
    ));
    assert_eq!(apply_op(&scope, &prepared)["outcome"], "Stale");
    assert!(
        matches!(current(&scope, &note).data, RecordData::Note(n) if n.markdown == "Owner's unsaved writing Ω")
    );
}
#[test]
fn local_candidate_cannot_refresh_into_unread_whole_replacement() {
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let local = prepare_edit(&scope, args(&note, 1, 0, "Mon", "Tues", "refresh-local")).unwrap();
    let request = ChangeRequest {
        reason: "Replace the whole unseen note".into(),
        inputs: vec![],
        writes: vec![Put {
            id: note.clone(),
            expected_version: Some(1),
            archived: false,
            data: RecordData::Note(Note::working("Decision", "Dropped unread content")),
        }],
    };
    let result = refresh_candidate(
        &scope,
        RefreshArgs {
            operation: local["operation"].as_str().unwrap().into(),
            request_key: "unread-refresh".into(),
            request,
        },
    )
    .unwrap();
    assert_eq!(result["outcome"], "Invalid");
    assert_eq!(current(&scope, &note).version, 1);
}
#[tokio::test]
async fn actual_provider_history_schema_tool_dispatch_and_frozen_rewrite_agree() {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    for unique in [false, true] {
        let (_root, request, note) = tests::fixture();
        let (scope, input) = begin(&request).unwrap();
        let (call_id, arguments) = if unique {
            (
                "unique",
                quote_args(&note, 1, "Mon", "Tues", "provider-null"),
            )
        } else {
            ("exact", args(&note, 1, 0, "Mon", "Tues", "provider-range"))
        };
        let model = MockCompletionModel::from_stream_turns([
            vec![
                MockStreamEvent::tool_call(
                    call_id,
                    "prepare_note_edit",
                    serde_json::to_value(arguments).unwrap(),
                ),
                MockStreamEvent::final_response_with_total_tokens(1),
            ],
            vec![MockStreamEvent::final_response_with_total_tokens(1)],
        ]);
        let answer = run_model(
            model.clone(),
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            4,
            true,
            CancellationToken::new(),
            Arc::new(|_| {}),
        )
        .await;
        assert!(matches!(answer.terminal, AiTerminal::Completed));
        assert_eq!(model.request_count(), 2);
        let requests = model.requests();
        assert_eq!(
            serde_json::to_value(&requests[0].tools).unwrap(),
            json!(budget_tool_definitions(&scope))
        );
        assert_eq!(
            requests[0]
                .tools
                .iter()
                .find(|t| t.name == "prepare_note_edit")
                .unwrap()
                .parameters,
            schema("prepare_note_edit")
        );
        assert_eq!(
            schema("prepare_note_edit")["required"],
            json!([
                "id",
                "version",
                "start",
                "end",
                "expected_text",
                "replacement_text",
                "request_key",
                "reason"
            ])
        );
        assert_eq!(
            schema("prepare_note_edit")["properties"]["start"]["type"],
            json!(["integer", "null"])
        );
        let dispatched = serde_json::to_string(&requests[1].chat_history).unwrap();
        if unique {
            assert!(dispatched.contains("\"start\":null") && dispatched.contains("\"end\":null"));
        }
        assert!(dispatched.contains("\"operation\""));
        assert!(dispatched.contains("Tuesday"));
        let unique_result = requests[1]
            .chat_history
            .iter()
            .find_map(|message| {
                let rig::message::Message::User { content } = message else {
                    return None;
                };
                content.iter().find_map(|item| {
                    let rig::message::UserContent::ToolResult(result) = item else {
                        return None;
                    };
                    if result.call.wire() != call_id {
                        return None;
                    }
                    result.content.iter().find_map(|block| {
                        if let rig::message::ToolResultContent::Json { value } = block {
                            Some(value.clone())
                        } else {
                            None
                        }
                    })
                })
            })
            .expect("the unique-quote call must return a structured result");
        let unique_operation = unique_result["operation"]
            .as_str()
            .unwrap_or_else(|| panic!("{call_id} must prepare an operation: {unique_result}"));
        let candidates = scope.store.lock().unwrap().candidates().unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.operation.as_str() == unique_operation)
        );
    }
    let (_root, request, snapshot) = writing_tests::rewrite_fixture(false, 0);
    let (scope, input) = begin(&request).unwrap();
    assert!(
        prepare_edit(
            &scope,
            args(
                &snapshot.note.id,
                snapshot.note.version,
                0,
                "Mon",
                "Tues",
                "refused"
            )
        )
        .is_err()
    );
    let model = MockCompletionModel::from_stream_turns([vec![
        MockStreamEvent::final_response_with_total_tokens(1),
    ]]);
    let _ = run_model(
        model.clone(),
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    let requests = model.requests();
    assert!(
        !requests[0]
            .tools
            .iter()
            .any(|t| t.name == "prepare_note_edit")
    );
    assert_eq!(
        serde_json::to_value(&requests[0].tools).unwrap(),
        json!(budget_tool_definitions(&scope))
    );
}

#[tokio::test]
async fn actual_http_provider_routes_dispatch_unique_quote_with_required_nullable_offsets() {
    use crate::provider_formats_tests::{client, definition, success, text_sse, tool_sse};

    const ROUTES: [(Provider, &str, bool); 3] = [
        (Provider::Chatgpt, "gpt-5.5", true),
        (Provider::Copilot, "gpt-5.5", false),
        (Provider::Copilot, "gpt-5.3-codex", true),
    ];

    for (provider, model_name, responses) in ROUTES {
        let (_root, request, note) = tests::fixture();
        let (scope, input) = begin(&request).unwrap();
        let canonical_before = current(&scope, &note);
        let call_id = "call_0";
        let arguments = json!({
            "id":note,
            "version":1,
            "start":null,
            "end":null,
            "expected_text":"Mon",
            "replacement_text":"Tues",
            "request_key":"http-unique-quote",
            "reason":"Correct the exact local passage"
        });
        let (_provider_root, client, http) = client(
            provider,
            model_name,
            vec![
                success(tool_sse(
                    responses,
                    &[("prepare_note_edit", arguments.clone())],
                )),
                success(text_sse(responses, "Prepared the exact local edit.")),
            ],
        )
        .await;
        let model = match client.inner {
            OwnedClient::Chatgpt(inner) => {
                rig_core::DynModel::<rig_core::operation::Completion>::from(
                    inner.completion(model_name.to_owned()),
                )
            }
            OwnedClient::Copilot(inner) => {
                rig_core::DynModel::<rig_core::operation::Completion>::from(
                    crate::auth::copilot_completion(&inner, model_name.to_owned(), responses),
                )
            }
        };

        let answer = run_model(
            model,
            scope.clone(),
            &input,
            ReasoningEffort::Medium,
            4,
            responses,
            CancellationToken::new(),
            Arc::new(|_| {}),
        )
        .await;
        assert!(
            matches!(answer.terminal, AiTerminal::Completed),
            "{answer:?}"
        );

        let bodies = http.bodies();
        assert_eq!(bodies.len(), 2, "one tool round and one final response");
        let advertised = definition(&bodies[0], responses, "prepare_note_edit")
            .unwrap_or_else(|| panic!("missing prepare_note_edit on {provider:?}/{model_name}"));
        let schema = &advertised["parameters"];
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["required"],
            json!([
                "id",
                "version",
                "start",
                "end",
                "expected_text",
                "replacement_text",
                "request_key",
                "reason"
            ])
        );
        assert_eq!(
            schema["properties"]["start"]["type"],
            json!(["integer", "null"])
        );
        assert_eq!(
            schema["properties"]["end"]["type"],
            json!(["integer", "null"])
        );

        let continuation = &bodies[1][if responses { "input" } else { "messages" }];
        let results = continuation
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["type"] == "function_call_output" || item["role"] == "tool")
            .collect::<Vec<_>>();
        assert_eq!(results.len(), 1, "the exact tool call must have one result");
        let (result_call_id, encoded_result) = if responses {
            (
                results[0]["call_id"].as_str().unwrap(),
                results[0]["output"].as_str().unwrap(),
            )
        } else {
            (
                results[0]["tool_call_id"].as_str().unwrap(),
                results[0]["content"].as_str().unwrap(),
            )
        };
        assert_eq!(result_call_id, call_id);
        let result: Value = serde_json::from_str(encoded_result).unwrap();
        let operation = result["operation"].as_str().unwrap();
        assert!(
            !operation.is_empty(),
            "the exact call must prepare an operation"
        );

        let candidates = scope.store.lock().unwrap().candidates().unwrap();
        assert_eq!(candidates.len(), 1);
        let candidate = candidates
            .iter()
            .find(|candidate| candidate.operation.as_str() == operation)
            .expect("tool result must identify the prepared candidate");
        let RecordData::Note(prepared_note) = &candidate.request.writes[0].data else {
            panic!("prepared write must remain a Note")
        };
        let RecordData::Note(mut expected_note) = canonical_before.data.clone() else {
            panic!("canonical record must be a Note")
        };
        expected_note.markdown = "Tuesday".into();
        assert_eq!(prepared_note, &expected_note);
        assert_eq!(current(&scope, &note), canonical_before);
        http.assert_consumed();
    }
}
#[test]
fn outside_private_scope_refuses_and_in_scope_local_edit_preserves_private_visibility() {
    let (_root, mut request, general) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let project = project_tests::project_command(
        &mut store,
        ProjectCommand::Create {
            title: "Private project".into(),
        },
    )
    .writes[0]
        .after
        .id
        .clone();
    let private = project_tests::project_command(
        &mut store,
        ProjectCommand::CreateDocument {
            project: project.clone(),
            title: "Private note".into(),
            markdown: "Private Monday".into(),
        },
    )
    .writes
    .into_iter()
    .find(|w| matches!(w.after.data, RecordData::Note(_)))
    .unwrap()
    .after;
    drop(store);
    let (scope, _) = begin(&request).unwrap();
    assert!(
        prepare_edit(
            &scope,
            args(
                &private.id,
                private.version,
                8,
                "Mon",
                "Tues",
                "unavailable"
            )
        )
        .is_err()
    );
    assert_eq!(current(&scope, &private.id), private);
    drop(scope);
    let mut store = Store::open(&request.data_dir).unwrap();
    project_tests::project_command(
        &mut store,
        ProjectCommand::MoveChat {
            chat: request.thread.clone(),
            project: Some(project),
        },
    );
    let version = store.record(&private.id).unwrap().unwrap().version;
    drop(store);
    request.prompt = "Change one local passage in the linked private document".into();
    let (scope, _) = begin(&request).unwrap();
    let prepared = prepare_edit(
        &scope,
        args(&private.id, version, 8, "Mon", "Tues", "private"),
    )
    .unwrap();
    assert_eq!(apply_op(&scope, &prepared)["outcome"], "Applied");
    let RecordData::Note(mut expected) = private.data else {
        panic!()
    };
    expected.markdown = "Private Tuesday".into();
    assert!(expected.chat.is_some());
    assert_eq!(
        current(&scope, &private.id).data,
        RecordData::Note(expected)
    );
    let general_before = current(&scope, &general);
    let publish = prepare_edit(
        &scope,
        args(
            &general,
            general_before.version,
            0,
            "Mon",
            "Private Tuesday",
            "private-to-general",
        ),
    )
    .unwrap();
    assert_eq!(apply_op(&scope, &publish)["outcome"], "NeedsReview");
    assert_eq!(
        current(&scope, &general),
        general_before,
        "private replacement bytes must not publish into general knowledge through maintenance"
    );
}
#[test]
fn review_first_local_edit_remains_pending_and_conflicting_undo_preserves_later_work() {
    let (_root, request, note) = tests::fixture();
    let mut store = Store::open(&request.data_dir).unwrap();
    let op = store.allocate_operation("review-first").unwrap();
    store
        .choose_delegation(
            &op,
            DelegationChoice::ReviewFirst,
            &HostAuthority::owner("choice"),
        )
        .unwrap();
    drop(store);
    let (scope, _) = begin(&request).unwrap();
    let prepared = prepare_edit(&scope, args(&note, 1, 0, "Mon", "Tues", "review-first")).unwrap();
    assert_eq!(apply_op(&scope, &prepared)["outcome"], "NeedsReview");
    assert_eq!(current(&scope, &note).version, 1);
    let op = {
        let mut store = scope.store.lock().unwrap();
        let op = store
            .operation(prepared["operation"].as_str().unwrap())
            .unwrap();
        assert!(matches!(
            store
                .apply(&op, &HostAuthority::owner("reviewed exact candidate"))
                .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        op
    };
    let later = owner_note(
        &request,
        &note,
        Note::working("Decision", "Later owner writing"),
    );
    let mut store = scope.store.lock().unwrap();
    let undo = store.allocate_operation("conflicting-undo").unwrap();
    assert!(!matches!(
        store
            .undo(&undo, &op, &HostAuthority::owner("Undo"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(store.record(&note).unwrap().unwrap(), later);
}
#[tokio::test]
async fn malformed_local_edit_skips_valid_sibling_before_bounded_correction() {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    let (_root, request, note) = tests::fixture();
    let (scope, input) = begin(&request).unwrap();
    let edit = args(&note, 1, 0, "Mon", "Tues", "corrected");
    let mut malformed = serde_json::to_value(&edit).unwrap();
    malformed["end"] = json!(0);
    assert!(!parse_valid("prepare_note_edit", &malformed));
    let mut missing_nullable = serde_json::to_value(&edit).unwrap();
    missing_nullable.as_object_mut().unwrap().remove("start");
    missing_nullable.as_object_mut().unwrap().remove("end");
    assert!(!parse_valid("prepare_note_edit", &missing_nullable));
    let mut forged = serde_json::to_value(&edit).unwrap();
    forged["host_exact"] = json!(true);
    assert!(!parse_valid("prepare_note_edit", &forged));
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call("bad", "prepare_note_edit", malformed),
            MockStreamEvent::tool_call("missing", "prepare_note_edit", missing_nullable),
            MockStreamEvent::tool_call(
                "sibling",
                "read_records",
                json!({"ids":[note],"query":null}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "fixed",
                "prepare_note_edit",
                serde_json::to_value(&edit).unwrap(),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![MockStreamEvent::final_response_with_total_tokens(1)],
    ]);
    let answer = run_model(
        model.clone(),
        scope.clone(),
        &input,
        ReasoningEffort::Medium,
        4,
        true,
        CancellationToken::new(),
        Arc::new(|_| {}),
    )
    .await;
    assert!(matches!(answer.terminal, AiTerminal::Completed));
    assert_eq!(model.request_count(), 3);
    assert!(
        !scope.reads.lock().unwrap().knows(&note, 1),
        "valid sibling was skipped too"
    );
    assert_eq!(current(&scope, &note).version, 1);
    assert_eq!(scope.store.lock().unwrap().candidates().unwrap().len(), 1);
    let RecordData::Run(run) = scope
        .store
        .lock()
        .unwrap()
        .record(&scope.run)
        .unwrap()
        .unwrap()
        .data
    else {
        panic!("run record missing")
    };
    assert_eq!(
        run.operations.len(),
        1,
        "only the corrected tool call allocated an operation"
    );
}
