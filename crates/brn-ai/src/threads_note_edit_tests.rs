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
        start,
        end: start + expected.len(),
        expected_text: expected.into(),
        replacement_text: replacement.into(),
        request_key: key.into(),
        reason: "Correct the exact local passage".into(),
    }
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
    let prepared = prepare_edit(&scope, edit.clone()).unwrap();
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
    use sha2::{Digest, Sha256};
    let (_root, request, note) = tests::fixture();
    let (scope, _) = begin(&request).unwrap();
    let edit = args(&note, 1, 0, "Mon", "Tues", "allocation-gap");
    let binding = format!("{:x}", Sha256::digest(serde_json::to_vec(&edit).unwrap()));
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
    let (_root, request, note) = tests::fixture();
    let (scope, input) = begin(&request).unwrap();
    let edit = args(&note, 1, 0, "Mon", "Tues", "provider");
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call(
                "exact",
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
    assert!(
        serde_json::to_string(&requests[1].chat_history)
            .unwrap()
            .contains("Tuesday")
    );
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
    let mut forged = serde_json::to_value(&edit).unwrap();
    forged["host_exact"] = json!(true);
    assert!(!parse_valid("prepare_note_edit", &forged));
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call("bad", "prepare_note_edit", malformed),
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
}
