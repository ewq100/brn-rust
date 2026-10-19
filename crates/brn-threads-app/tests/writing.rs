use brn_threads_app::*;

fn setup() -> (tempfile::TempDir, Workspace, Record, Record) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open(dir.path().join("data")).unwrap();
    let note = app.new_note("Draft", "Tere maailm. Tere maailm.").unwrap();
    let thread = app.new_thread("Refine draft").unwrap();
    app.link_writing_thread("link", &note.id, note.version, &thread.id)
        .unwrap();
    (dir, app, note, thread)
}
fn comment(
    app: &mut Workspace,
    note: &Record,
    thread: &Record,
    key: &str,
    range: (usize, usize),
) -> Record {
    app.add_writing_comment(
        key,
        &WritingCommentInput {
            note: note.id.clone(),
            version: note.version,
            thread: thread.id.clone(),
            range,
            quote: "Tere maailm.".into(),
            body: "Make this clearer".into(),
        },
    )
    .unwrap()
}
#[test]
fn multiple_comments_snapshot_replay_and_explicit_addressed_undo() {
    let (_dir, mut app, note, thread) = setup();
    let first = comment(&mut app, &note, &thread, "first", (0, 12));
    let second = comment(&mut app, &note, &thread, "second", (13, 25));
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
    assert_eq!(app.store.record(&thread.id).unwrap().unwrap(), thread);
    app.post_message(&thread.id, "Earlier instruction").unwrap();
    let snapshot = app
        .snapshot_writing_request(
            "rewrite",
            &note.id,
            note.version,
            &thread.id,
            std::slice::from_ref(&first.id),
        )
        .unwrap();
    app.post_message(&thread.id, "Later instruction").unwrap();
    assert_eq!(
        app.snapshot_writing_request(
            "rewrite",
            &note.id,
            note.version,
            &thread.id,
            std::slice::from_ref(&first.id)
        )
        .unwrap(),
        snapshot
    );
    assert_eq!(snapshot.messages.len(), 1);
    let operation = app.store.allocate_operation("proposal").unwrap();
    let request = snapshot.change_request("A clearer whole draft.").unwrap();
    app.store.prepare(&operation, &request).unwrap();
    assert!(
        app.writing_comments(&note.id)
            .unwrap()
            .iter()
            .all(|c| !c.addressed)
    );
    let applied = app
        .store
        .apply(&operation, &HostAuthority::owner("reviewed"))
        .unwrap();
    assert!(matches!(applied, ApplyOutcome::Applied(_)));
    assert!(
        app.writing_comments(&note.id)
            .unwrap()
            .iter()
            .all(|c| !c.addressed)
    );
    app.mark_writing_comments_addressed(
        "addressed",
        &snapshot.request_message,
        operation.as_str(),
        std::slice::from_ref(&first.id),
    )
    .unwrap();
    let comments = app.writing_comments(&note.id).unwrap();
    assert!(
        comments
            .iter()
            .find(|c| c.record.id == first.id)
            .unwrap()
            .addressed
    );
    assert!(
        !comments
            .iter()
            .find(|c| c.record.id == second.id)
            .unwrap()
            .addressed
    );
    app.undo(operation.as_str()).unwrap();
    assert!(
        app.mark_writing_comments_addressed(
            "after-undo",
            &snapshot.request_message,
            operation.as_str(),
            std::slice::from_ref(&first.id)
        )
        .is_err()
    );
    assert!(
        app.writing_comments(&note.id)
            .unwrap()
            .iter()
            .all(|c| !c.addressed)
    );
}
#[test]
fn exact_anchor_stale_note_bad_selection_and_bound_key_refused() {
    let (_dir, mut app, note, thread) = setup();
    let first = comment(&mut app, &note, &thread, "first", (13, 25));
    let invalid = WritingCommentInput {
        note: note.id.clone(),
        version: note.version,
        thread: thread.id.clone(),
        range: (1, 13),
        quote: "Tere maailm.".into(),
        body: "Bad".into(),
    };
    assert!(app.add_writing_comment("bad", &invalid).is_err());
    assert!(
        app.snapshot_writing_request(
            "bad-note-version",
            &note.id,
            note.version + 1,
            &thread.id,
            std::slice::from_ref(&first.id)
        )
        .is_err()
    );
    app.snapshot_writing_request(
        "rewrite",
        &note.id,
        note.version,
        &thread.id,
        std::slice::from_ref(&first.id),
    )
    .unwrap();
    assert!(
        app.snapshot_writing_request("rewrite", &note.id, note.version, &thread.id, &[])
            .is_err()
    );
    assert!(
        app.mark_writing_comments_addressed("no-apply", "missing", "missing", &[first.id])
            .is_err()
    );
}
#[test]
fn oversize_context_fails_without_truncation_and_fake_snapshot_refused() {
    let (_dir, mut app, note, thread) = setup();
    app.post_message(&thread.id, &"x".repeat(128 * 1024))
        .unwrap();
    assert!(
        app.snapshot_writing_request("large", &note.id, note.version, &thread.id, &[])
            .is_err()
    );
    let operation = app.store.allocate_operation("fake").unwrap();
    let mut fake_note = note.clone();
    if let RecordData::Note(n) = &mut fake_note.data {
        n.markdown = "Invented evidence".into();
    }
    let fake = WritingSnapshot {
        setup: None,
        request_message: operation.creation_id(0),
        thread: thread.id.clone(),
        note: fake_note,
        comments: vec![],
        messages: vec![],
        general_feedback: String::new(),
    };
    app.owner_change(
        "fake",
        &ChangeRequest {
            reason: "fixture".into(),
            writes: vec![Put {
                id: fake.request_message.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: thread.id,
                    role: WRITING_REQUEST_ROLE.into(),
                    text: fake.message_text().unwrap(),
                }),
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    assert!(app.writing_request(&fake.request_message).is_err());
}

#[test]
fn utf8_cross_paragraph_exact_selection_and_restart_keep_feedback() {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = dir.path().join("data");
    let mut app = Workspace::open(&data).unwrap();
    let text = "Õun **küps**.\n\nTeine lõik.";
    let note = app.new_note("Estonian", text).unwrap();
    let thread = app.new_thread("Draft").unwrap();
    app.link_writing_thread("link", &note.id, note.version, &thread.id)
        .unwrap();
    let input = WritingCommentInput {
        note: note.id.clone(),
        version: note.version,
        thread: thread.id.clone(),
        range: (0, text.len()),
        quote: text.into(),
        body: "Clarify both paragraphs".into(),
    };
    let comment = app.add_writing_comment("comment", &input).unwrap();
    let mut bad = input.clone();
    bad.range = (1, text.len());
    assert!(app.add_writing_comment("broken-char", &bad).is_err());
    let snapshot = app
        .snapshot_writing_request(
            "rewrite",
            &note.id,
            note.version,
            &thread.id,
            std::slice::from_ref(&comment.id),
        )
        .unwrap();
    drop(app);
    let mut reopened = Workspace::open(&data).unwrap();
    assert_eq!(
        reopened.add_writing_comment("comment", &input).unwrap(),
        comment
    );
    assert_eq!(
        reopened
            .snapshot_writing_request("rewrite", &note.id, note.version, &thread.id, &[comment.id])
            .unwrap(),
        snapshot
    );
    assert_eq!(reopened.writing_comments(&note.id).unwrap().len(), 1);
}
#[test]
fn feedback_edit_stales_generated_candidate_and_only_matching_apply_can_address() {
    let (_dir, mut app, note, thread) = setup();
    let comment = comment(&mut app, &note, &thread, "comment", (0, 12));
    let snapshot = app
        .snapshot_writing_request(
            "rewrite",
            &note.id,
            note.version,
            &thread.id,
            std::slice::from_ref(&comment.id),
        )
        .unwrap();
    let proposal = app.store.allocate_operation("proposal").unwrap();
    app.store
        .prepare(&proposal, &snapshot.change_request("Proposed").unwrap())
        .unwrap();
    let mut changed = comment.clone();
    if let RecordData::Comment(comment) = &mut changed.data {
        comment.body = "A newer feedback instruction".into();
    }
    app.owner_change(
        "edit-comment",
        &ChangeRequest {
            reason: "Owner updated feedback".into(),
            writes: vec![Put {
                id: changed.id.clone(),
                expected_version: Some(changed.version),
                archived: false,
                data: changed.data,
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    assert!(matches!(
        app.store
            .apply(&proposal, &HostAuthority::owner("review"))
            .unwrap(),
        ApplyOutcome::Stale { .. }
    ));
    let unrelated = app.post_message(&thread.id, "Only chat changed").unwrap();
    assert!(
        app.mark_writing_comments_addressed(
            "wrong-receipt",
            &snapshot.request_message,
            unrelated.operation.as_str(),
            std::slice::from_ref(&comment.id)
        )
        .is_err()
    );
    assert!(
        app.writing_comments(&note.id)
            .unwrap()
            .iter()
            .all(|comment| !comment.addressed)
    );
    // Stale exact context remains retained for an explicit new invocation.
    assert_eq!(
        app.writing_request(&snapshot.request_message).unwrap(),
        snapshot
    );
}

#[test]
fn manual_save_mapping_preserves_addressed_feedback_and_allows_later_confirmation() {
    for confirm_before_save in [true, false] {
        let (_dir, mut app, note, thread) = setup();
        let comment = comment(&mut app, &note, &thread, "comment", (0, 12));
        let snapshot = app
            .snapshot_writing_request(
                "rewrite",
                &note.id,
                note.version,
                &thread.id,
                std::slice::from_ref(&comment.id),
            )
            .unwrap();
        let operation = app.store.allocate_operation("proposal").unwrap();
        app.store
            .prepare(
                &operation,
                &snapshot
                    .change_request("Tere maailm. Tere maailm. Added.")
                    .unwrap(),
            )
            .unwrap();
        assert!(matches!(
            app.store
                .apply(&operation, &HostAuthority::owner("review"))
                .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        if confirm_before_save {
            app.mark_writing_comments_addressed(
                "addressed",
                &snapshot.request_message,
                operation.as_str(),
                std::slice::from_ref(&comment.id),
            )
            .unwrap();
        }
        let current = app.store.record(&note.id).unwrap().unwrap();
        let edit = app.store.begin_edit(&note.id, current.version).unwrap();
        app.store
            .update_buffer(&edit.id, 1, "Tere maailm. Tere maailm. Added. Manual edit.")
            .unwrap();
        assert!(matches!(
            app.store.save(&edit.id, 1).unwrap(),
            SaveOutcome::Saved(_)
        ));
        let mapped = app.store.record(&comment.id).unwrap().unwrap();
        assert!(
            mapped.version > comment.version,
            "Save must exercise the real comment mapping write"
        );
        if !confirm_before_save {
            assert!(!app.writing_comments(&note.id).unwrap()[0].addressed);
            app.mark_writing_comments_addressed(
                "addressed",
                &snapshot.request_message,
                operation.as_str(),
                std::slice::from_ref(&comment.id),
            )
            .unwrap();
        }
        assert!(app.writing_comments(&note.id).unwrap()[0].addressed);
        let mut feedback = mapped.data;
        if let RecordData::Comment(comment) = &mut feedback {
            comment.body = "Different feedback after the rewrite".into();
        }
        app.owner_change(
            "body-edit",
            &ChangeRequest {
                reason: "Owner changed feedback".into(),
                writes: vec![Put {
                    id: comment.id.clone(),
                    expected_version: Some(mapped.version),
                    archived: false,
                    data: feedback,
                }],
                inputs: vec![],
            },
        )
        .unwrap();
        assert!(!app.writing_comments(&note.id).unwrap()[0].addressed);
        assert!(
            app.mark_writing_comments_addressed(
                "old-feedback",
                &snapshot.request_message,
                operation.as_str(),
                std::slice::from_ref(&comment.id)
            )
            .is_err()
        );
    }
}

#[test]
fn general_feedback_is_immutable_replay_bound_and_never_posted_as_chat() {
    let (dir, mut app, note, thread) = setup();
    let first = comment(&mut app, &note, &thread, "passage", (0, 12));
    app.post_message(&thread.id, "Earlier context").unwrap();
    let feedback = "Use my voice. Keep Õun and the factual qualifications.\nDo not add new claims.";
    let snapshot = app
        .snapshot_writing_request_with_feedback(
            "general-rewrite",
            &note.id,
            note.version,
            &thread.id,
            std::slice::from_ref(&first.id),
            feedback,
        )
        .unwrap();
    assert_eq!(snapshot.general_feedback, feedback);
    assert_eq!(snapshot.comments, vec![first.clone()]);
    assert_eq!(snapshot.messages.len(), 1);
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
    assert_eq!(
        app.writing_request(&snapshot.request_message).unwrap(),
        snapshot
    );
    let messages = app.messages(&thread.id).unwrap();
    assert_eq!(
        messages.len(),
        2,
        "one prior chat message and one request only"
    );
    assert!(messages.iter().all(|record| !matches!(&record.data, RecordData::Message(message) if message.role == "owner" && message.text == feedback)));
    for different in ["Different instruction", ""] {
        assert!(
            app.snapshot_writing_request_with_feedback(
                "general-rewrite",
                &note.id,
                note.version,
                &thread.id,
                std::slice::from_ref(&first.id),
                different,
            )
            .is_err()
        );
    }
    app.post_message(&thread.id, "Later context outside this request")
        .unwrap();
    drop(app);
    let mut reopened = Workspace::open(dir.path().join("data")).unwrap();
    assert_eq!(
        reopened
            .snapshot_writing_request_with_feedback(
                "general-rewrite",
                &note.id,
                note.version,
                &thread.id,
                std::slice::from_ref(&first.id),
                feedback,
            )
            .unwrap(),
        snapshot
    );
}

#[test]
fn empty_general_feedback_preserves_existing_replay_binding() {
    let (_dir, mut app, note, thread) = setup();
    let snapshot = app
        .snapshot_writing_request("existing-request", &note.id, note.version, &thread.id, &[])
        .unwrap();
    assert!(snapshot.general_feedback.is_empty());
    assert!(
        !snapshot
            .message_text()
            .unwrap()
            .contains("general_feedback")
    );
    assert_eq!(
        app.snapshot_writing_request_with_feedback(
            "existing-request",
            &note.id,
            note.version,
            &thread.id,
            &[],
            "",
        )
        .unwrap(),
        snapshot
    );
    assert!(
        app.snapshot_writing_request_with_feedback(
            "existing-request",
            &note.id,
            note.version,
            &thread.id,
            &[],
            "New feedback needs a new operation identity",
        )
        .is_err()
    );
}

#[test]
fn oversized_general_feedback_refuses_without_partial_request_or_note_write() {
    let (_dir, mut app, note, thread) = setup();
    let before = app.records().unwrap();
    assert!(
        app.snapshot_writing_request_with_feedback(
            "too-large",
            &note.id,
            note.version,
            &thread.id,
            &[],
            &"é".repeat(MAX_WRITING_REQUEST_BYTES),
        )
        .is_err()
    );
    assert_eq!(app.records().unwrap(), before);
    assert!(app.messages(&thread.id).unwrap().is_empty());
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
}

#[test]
fn new_requests_capture_v2_setup_without_changing_the_note() {
    let (_dir, mut app, note, thread) = setup();
    let snapshot = app
        .snapshot_writing_request("v2", &note.id, note.version, &thread.id, &[])
        .unwrap();
    let text = snapshot.message_text().unwrap();
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["version"], 2);
    assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
}

#[test]
fn combined_review_selects_only_captured_subset_replays_and_reopens_after_undo() {
    let (dir, mut app, note, thread) = setup();
    let first = comment(&mut app, &note, &thread, "first", (0, 12));
    let second = comment(&mut app, &note, &thread, "second", (13, 25));
    let ids = vec![first.id.clone(), second.id.clone()];
    let snapshot = app
        .snapshot_writing_request("request", &note.id, note.version, &thread.id, &ids)
        .unwrap();
    let operation = app.store.allocate_operation("proposal").unwrap();
    app.store
        .prepare(
            &operation,
            &snapshot.change_request("Exact accepted text").unwrap(),
        )
        .unwrap();
    // Unknown or duplicated selections fail before Apply.
    for selected in [
        vec!["unknown-comment".into()],
        vec![first.id.clone(), first.id.clone()],
    ] {
        assert!(
            app.review_apply_selected_comments(operation.as_str(), None, &selected)
                .is_err()
        );
        assert!(app.store.receipt(&operation).unwrap().is_none());
    }
    let selected = std::slice::from_ref(&second.id);
    let result = app
        .review_apply_selected_comments(operation.as_str(), None, selected)
        .unwrap();
    assert!(matches!(result.outcome, ApplyOutcome::Applied(_)));
    assert!(result.pending_updates().is_none());
    let comments = app.writing_comments(&note.id).unwrap();
    assert!(
        !comments
            .iter()
            .find(|c| c.record.id == first.id)
            .unwrap()
            .addressed
    );
    assert!(
        comments
            .iter()
            .find(|c| c.record.id == second.id)
            .unwrap()
            .addressed
    );
    let history = app.history().unwrap().len();
    assert_eq!(
        app.review_apply_selected_comments(operation.as_str(), None, selected)
            .unwrap(),
        result
    );
    assert_eq!(app.history().unwrap().len(), history);
    drop(app);
    let mut app = Workspace::open(dir.path().join("data")).unwrap();
    assert!(
        app.writing_comments(&note.id)
            .unwrap()
            .iter()
            .find(|c| c.record.id == second.id)
            .unwrap()
            .addressed
    );
    let proof = app.history().unwrap().into_iter().find(|receipt| receipt.writes.iter().any(|write| matches!(&write.after.data, RecordData::Message(message) if message.role == "writing-addressed"))).unwrap();
    app.undo(proof.operation.as_str()).unwrap();
    assert!(
        app.writing_comments(&note.id)
            .unwrap()
            .iter()
            .all(|comment| !comment.addressed)
    );
    // A fresh, explicit individual confirmation remains available after reopening.
    app.mark_writing_comments_addressed(
        "new-owner-confirmation",
        &snapshot.request_message,
        operation.as_str(),
        selected,
    )
    .unwrap();
    assert!(
        app.writing_comments(&note.id)
            .unwrap()
            .iter()
            .find(|c| c.record.id == second.id)
            .unwrap()
            .addressed
    );
    app.undo(operation.as_str()).unwrap();
    assert!(
        app.writing_comments(&note.id)
            .unwrap()
            .iter()
            .all(|comment| !comment.addressed)
    );
    assert!(
        matches!(app.store.note(&note.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Tere maailm. Tere maailm.")
    );
}

#[test]
fn combined_apply_only_and_order_independent_feedback_replay() {
    for selected in [false, true] {
        let (_dir, mut app, note, thread) = setup();
        let first = comment(&mut app, &note, &thread, "first", (0, 12));
        let second = comment(&mut app, &note, &thread, "second", (13, 25));
        let mut ids = vec![first.id, second.id];
        let snapshot = app
            .snapshot_writing_request("request", &note.id, note.version, &thread.id, &ids)
            .unwrap();
        let op = app.store.allocate_operation("proposal").unwrap();
        app.store
            .prepare(&op, &snapshot.change_request("Applied text").unwrap())
            .unwrap();
        let chosen = if selected { ids.as_slice() } else { &[] };
        let result = app
            .review_apply_selected_comments(op.as_str(), None, chosen)
            .unwrap();
        assert!(result.pending_updates().is_none());
        assert!(
            app.writing_comments(&note.id)
                .unwrap()
                .iter()
                .all(|comment| comment.addressed == selected)
        );
        let history = app.history().unwrap().len();
        ids.reverse();
        let chosen = if selected { ids.as_slice() } else { &[] };
        assert_eq!(
            app.review_apply_selected_comments(op.as_str(), None, chosen)
                .unwrap(),
            result
        );
        assert_eq!(app.history().unwrap().len(), history);
    }
}

fn suggested_request(
    snapshot: &WritingSnapshot,
    operation: &OperationId,
    ids: Vec<String>,
) -> ChangeRequest {
    let evidence = WritingCandidateEvidence {
        version: 1,
        request_message: snapshot.request_message.clone(),
        thread: snapshot.thread.clone(),
        chat: None,
        reads: vec![],
        quotations: vec![],
        requirements_reported: vec![],
        proposed_addressed_comment_ids: ids,
        explanation: "Model reports feedback addressed".into(),
    };
    let mut request = snapshot
        .change_request("A proposed clearer whole draft.")
        .unwrap();
    request.reason = WRITING_PUBLIC_REASON.into();
    request.writes.push(Put {
        id: operation.creation_id(0),
        expected_version: None,
        archived: false,
        data: RecordData::Message(Message {
            thread: snapshot.thread.clone(),
            role: WRITING_EVIDENCE_ROLE.into(),
            text: serde_json::to_string(&evidence).unwrap(),
        }),
    });
    request
}

#[test]
fn proposed_comment_evidence_refuses_forgery_and_preserves_old_empty_hashes() {
    let (_dir, mut app, note, thread) = setup();
    let first = comment(&mut app, &note, &thread, "first", (0, 12));
    let excluded = comment(&mut app, &note, &thread, "excluded", (13, 25));
    let snapshot = app
        .snapshot_writing_request(
            "suggestions",
            &note.id,
            note.version,
            &thread.id,
            std::slice::from_ref(&first.id),
        )
        .unwrap();
    for (i, ids) in [
        vec!["unknown".into()],
        vec![excluded.id],
        vec![first.id.clone(), first.id.clone()],
        vec![first.id.clone(); 129],
    ]
    .into_iter()
    .enumerate()
    {
        let operation = app
            .store
            .allocate_operation(&format!("forged-suggestions-{i}"))
            .unwrap();
        let request = suggested_request(&snapshot, &operation, ids);
        assert!(app.store.prepare(&operation, &request).is_err());
        assert!(app.store.candidates().unwrap().is_empty());
        assert_eq!(app.store.record(&note.id).unwrap().unwrap(), note);
    }
    let operation = app.store.allocate_operation("old-empty-evidence").unwrap();
    let request = suggested_request(&snapshot, &operation, vec![]);
    let first = app.store.prepare(&operation, &request).unwrap();
    let mut roundtrip = request.clone();
    let RecordData::Message(message) = &mut roundtrip.writes[1].data else {
        panic!()
    };
    assert!(!message.text.contains("proposed_addressed_comment_ids"));
    let old_bytes = message.text.clone();
    let mut explicit: serde_json::Value = serde_json::from_str(&message.text).unwrap();
    explicit["proposed_addressed_comment_ids"] = serde_json::json!([]);
    let evidence: WritingCandidateEvidence = serde_json::from_value(explicit).unwrap();
    message.text = serde_json::to_string(&evidence).unwrap();
    assert_eq!(message.text, old_bytes);
    assert_eq!(
        app.store.prepare(&operation, &roundtrip).unwrap(),
        first,
        "same immutable hash and bytes"
    );
}

#[test]
fn proposed_comments_reopen_checked_review_and_only_clear_on_owner_replacement() {
    let (dir, mut app, note, thread) = setup();
    let first = comment(&mut app, &note, &thread, "first", (0, 12));
    let snapshot = app
        .snapshot_writing_request(
            "suggestions",
            &note.id,
            note.version,
            &thread.id,
            std::slice::from_ref(&first.id),
        )
        .unwrap();
    let operation = app
        .store
        .allocate_operation("suggestions-original")
        .unwrap();
    let original = app
        .store
        .prepare(
            &operation,
            &suggested_request(&snapshot, &operation, vec![first.id.clone()]),
        )
        .unwrap();
    drop(app);
    let mut app = Workspace::open_without_default_credentials(dir.path().join("data")).unwrap();
    assert_eq!(
        app.writing_candidate_review(operation.as_str(), None)
            .unwrap()
            .proposed_addressed_comment_ids,
        vec![first.id]
    );
    // Clearing suggestions never permits editing other evidence or its bindings.
    for forged in 0..5 {
        let replacement = app
            .store
            .allocate_operation(&format!("forged-replacement-{forged}"))
            .unwrap();
        let mut request = original.request.clone();
        request.writes[1].id = replacement.creation_id(0);
        let RecordData::Note(note) = &mut request.writes[0].data else {
            panic!()
        };
        note.markdown = "Owner changed the candidate".into();
        let RecordData::Message(message) = &mut request.writes[1].data else {
            panic!()
        };
        let mut evidence: WritingCandidateEvidence = serde_json::from_str(&message.text).unwrap();
        if forged != 4 {
            evidence.proposed_addressed_comment_ids.clear();
        }
        match forged {
            0 => evidence.explanation.push_str(" forged"),
            1 => evidence.thread = "foreign".into(),
            2 => evidence
                .requirements_reported
                .push(WritingRequirementsClaim {
                    claim: "forged".into(),
                    quotations: vec![],
                }),
            3 => {
                request.inputs.pop();
            }
            _ => {}
        }
        message.text = serde_json::to_string(&evidence).unwrap();
        let error = app
            .store
            .replace_candidate(
                &operation,
                &replacement,
                &request,
                &HostAuthority::owner("exact review"),
            )
            .expect_err("forged replacement must fail the checked evidence boundary");
        assert!(
            error
                .to_string()
                .contains("evidence and input bindings must survive replacement"),
            "forgery {forged}: {error}"
        );
        assert!(
            app.store
                .candidates()
                .unwrap()
                .iter()
                .any(|candidate| candidate.operation == operation)
        );
    }
    let replacement = app
        .review_replace(operation.as_str(), &note.id, "Owner revised candidate")
        .unwrap();
    let checked = app
        .writing_candidate_review(replacement.operation.as_str(), None)
        .unwrap();
    assert!(checked.proposed_addressed_comment_ids.is_empty());
    assert_eq!(
        checked.model_explanation,
        "Model reports feedback addressed"
    );
    assert!(
        original
            .request
            .inputs
            .iter()
            .all(|input| replacement.request.inputs.contains(input))
    );
    assert_eq!(app.store.prepared(&operation).unwrap(), original);
    let mut cleared_original = original.request.clone();
    let RecordData::Message(message) = &mut cleared_original.writes[1].data else {
        panic!()
    };
    let mut evidence: WritingCandidateEvidence = serde_json::from_str(&message.text).unwrap();
    evidence.proposed_addressed_comment_ids.clear();
    message.text = serde_json::to_string(&evidence).unwrap();
    assert_eq!(
        replacement.request.writes[1].data,
        cleared_original.writes[1].data
    );
}

#[test]
fn owner_selected_replacement_also_clears_model_suggestions() {
    let (_dir, mut app, note, thread) = setup();
    let first = comment(&mut app, &note, &thread, "first", (0, 12));
    let snapshot = app
        .snapshot_writing_request(
            "suggestions",
            &note.id,
            note.version,
            &thread.id,
            std::slice::from_ref(&first.id),
        )
        .unwrap();
    let operation = app
        .store
        .allocate_operation("suggestions-original")
        .unwrap();
    let original = app
        .store
        .prepare(
            &operation,
            &suggested_request(&snapshot, &operation, vec![first.id]),
        )
        .unwrap();
    let replacement = app
        .review_select(operation.as_str(), std::slice::from_ref(&note.id))
        .unwrap();
    assert!(
        app.writing_candidate_review(replacement.operation.as_str(), None)
            .unwrap()
            .proposed_addressed_comment_ids
            .is_empty()
    );
    assert_eq!(app.store.prepared(&operation).unwrap(), original);
}
