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
        request_message: operation.creation_id(0),
        thread: thread.id.clone(),
        note: fake_note,
        comments: vec![],
        messages: vec![],
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
