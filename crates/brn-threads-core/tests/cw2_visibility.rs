use brn_threads_core::*;

fn commit(store: &mut Store, key: &str, writes: Vec<Put>) -> Receipt {
    let op = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("synthetic owner");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                writes,
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!("fixture commit")
    };
    receipt
}
fn fixture() -> (tempfile::TempDir, Store, WritingSnapshot, String, String) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let op = store.allocate_operation("fixture").unwrap();
    let draft = op.creation_id(0);
    let reference = op.creation_id(1);
    let thread = op.creation_id(2);
    let elsewhere = op.creation_id(3);
    let mut writes = vec![];
    for (id, title) in [(&draft, "Working"), (&reference, "Evidence")] {
        writes.push(Put {
            id: id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Note(Note::working(title, "Original")),
        });
    }
    for id in [&thread, &elsewhere] {
        writes.push(Put {
            id: id.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Thread(Thread {
                title: "Writing".into(),
                state: ThreadState::Open,
                attention: vec![],
                context_epoch: 0,
                private_context: id == &elsewhere,
            }),
        });
    }
    commit(&mut store, "fixture", writes);
    let saved = store.allocate_operation("saved-request").unwrap();
    let snapshot = WritingSnapshot {
        setup: None,
        request_message: saved.creation_id(0),
        thread: thread.clone(),
        note: store.record(&draft).unwrap().unwrap(),
        comments: vec![],
        messages: vec![],
        general_feedback: String::new(),
    };
    commit(
        &mut store,
        "saved-request",
        vec![Put {
            id: snapshot.request_message.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread,
                role: WRITING_REQUEST_ROLE.into(),
                text: snapshot.message_text().unwrap(),
            }),
        }],
    );
    (root, store, snapshot, reference, elsewhere)
}
fn candidate(
    store: &mut Store,
    snapshot: &WritingSnapshot,
    reference: &str,
    expectation: InputExpectation,
) -> OperationId {
    let op = store.allocate_operation("writing-proposal").unwrap();
    let mut request = snapshot.change_request("Exact reviewed rewrite").unwrap();
    request.inputs.push(RevisionInput {
        record: reference.into(),
        version: 1,
        expectation,
    });
    store.prepare(&op, &request).unwrap();
    op
}
fn change_reference(store: &mut Store, reference: &str, chat: Option<&str>, archived: bool) {
    let record = store.record(reference).unwrap().unwrap();
    let RecordData::Note(mut note) = record.data else {
        unreachable!()
    };
    note.chat = chat.map(str::to_owned);
    note.markdown = "Later evidence".into();
    commit(
        store,
        &format!("reference-{}", record.version),
        vec![Put {
            id: reference.into(),
            expected_version: Some(record.version),
            archived,
            data: RecordData::Note(note),
        }],
    );
}

#[test]
fn owner_apply_rechecks_historical_visibility_and_restored_access_needs_explicit_review() {
    let (_root, mut store, snapshot, reference, elsewhere) = fixture();
    let op = candidate(
        &mut store,
        &snapshot,
        &reference,
        InputExpectation::Historical,
    );
    change_reference(&mut store, &reference, Some(&elsewhere), false);
    assert!(
        !store
            .record_visible_in_chat(&reference, Some(&snapshot.thread))
            .unwrap()
    );
    let before = store.record(&snapshot.note.id).unwrap();
    for authority in [
        HostAuthority::owner("earlier exact review"),
        HostAuthority::owner("retry earlier review").for_chat(&snapshot.thread, false),
    ] {
        assert!(
            matches!(store.apply(&op, &authority).unwrap(), ApplyOutcome::NeedsReview { denied } if denied.contains(&reference))
        );
        assert_eq!(store.record(&snapshot.note.id).unwrap(), before);
        assert!(store.receipt(&op).unwrap().is_none());
    }
    change_reference(&mut store, &reference, None, false);
    assert_eq!(
        store.record(&snapshot.note.id).unwrap(),
        before,
        "access restoration does not apply anything"
    );
    let ApplyOutcome::Applied(receipt) = store
        .apply(
            &op,
            &HostAuthority::owner("new explicit exact review after access restoration"),
        )
        .unwrap()
    else {
        panic!("restored visible Historical evidence remains applicable")
    };
    // Lost-response replay reports this exact receipt; it never writes again.
    change_reference(&mut store, &reference, Some(&elsewhere), false);
    assert_eq!(
        store
            .apply(&op, &HostAuthority::owner("lost response"))
            .unwrap(),
        ApplyOutcome::Applied(receipt)
    );
    assert_eq!(store.record(&snapshot.note.id).unwrap().unwrap().version, 2);
}

#[test]
fn archived_historical_input_is_not_apply_authority() {
    let (_root, mut store, snapshot, reference, _) = fixture();
    let op = candidate(
        &mut store,
        &snapshot,
        &reference,
        InputExpectation::Historical,
    );
    change_reference(&mut store, &reference, None, true);
    assert!(
        matches!(store.apply(&op, &HostAuthority::owner("exact review")).unwrap(), ApplyOutcome::NeedsReview { denied } if denied.contains(&reference))
    );
    assert!(store.receipt(&op).unwrap().is_none());
}

#[test]
fn visible_historical_edit_remains_applicable_while_current_edit_stales() {
    for expectation in [InputExpectation::Historical, InputExpectation::Current] {
        let (_root, mut store, snapshot, reference, _) = fixture();
        let op = candidate(&mut store, &snapshot, &reference, expectation);
        change_reference(&mut store, &reference, None, false);
        let result = store
            .apply(&op, &HostAuthority::owner("exact review"))
            .unwrap();
        match expectation {
            InputExpectation::Historical => assert!(matches!(result, ApplyOutcome::Applied(_))),
            InputExpectation::Current => assert!(
                matches!(result, ApplyOutcome::Stale { records } if records.contains(&reference))
            ),
            _ => unreachable!(),
        }
    }
}

#[test]
fn revoked_v1_evidence_allows_safe_undo_but_fences_reapplying_candidate_content() {
    let (_root, mut store, snapshot, reference, elsewhere) = fixture();
    assert!(
        snapshot.setup.is_none(),
        "actual saved v1 compatibility fixture"
    );
    let saved_marker = store.record(&snapshot.request_message).unwrap();
    let op = candidate(
        &mut store,
        &snapshot,
        &reference,
        InputExpectation::Historical,
    );
    let original = store.prepared(&op).unwrap();
    assert!(matches!(
        store
            .apply(&op, &HostAuthority::owner("exact review"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    change_reference(&mut store, &reference, Some(&elsewhere), false);
    let undo = store.allocate_operation("undo-writing").unwrap();
    let ApplyOutcome::Applied(undone) = store
        .undo(
            &undo,
            &op,
            &HostAuthority::owner("safe Undo after revocation"),
        )
        .unwrap()
    else {
        panic!("restoring the pre-AI draft must not depend on the AI evidence still being visible")
    };
    assert!(
        matches!(store.note(&snapshot.note.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Original")
    );
    assert_eq!(undone.undo_of.as_ref(), Some(&op));
    let redo = store.allocate_operation("undo-the-undo").unwrap();
    assert!(
        matches!(store.undo(&redo, &undo, &HostAuthority::owner("explicit reapply")).unwrap(), ApplyOutcome::NeedsReview { denied } if denied.contains(&reference))
    );
    assert!(store.receipt(&redo).unwrap().is_none());
    assert_eq!(store.record(&snapshot.note.id).unwrap().unwrap().version, 3);
    change_reference(&mut store, &reference, None, false);
    let reapplied = store
        .undo(
            &redo,
            &undo,
            &HostAuthority::owner("new exact review after access restoration"),
        )
        .unwrap();
    assert!(matches!(&reapplied, ApplyOutcome::Applied(_)));
    assert!(
        matches!(store.note(&snapshot.note.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Exact reviewed rewrite")
    );
    change_reference(&mut store, &reference, Some(&elsewhere), false);
    assert_eq!(
        store
            .undo(&redo, &undo, &HostAuthority::owner("lost response replay"))
            .unwrap(),
        reapplied
    );
    let third = store.allocate_operation("undo-restored-rewrite").unwrap();
    assert!(matches!(
        store
            .undo(
                &third,
                &redo,
                &HostAuthority::owner("safe compensation again")
            )
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        matches!(store.note(&snapshot.note.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Original")
    );
    assert_eq!(
        store.prepared(&op).unwrap(),
        original,
        "saved v1 candidate/hash is not rewritten"
    );
    assert_eq!(
        store.record(&snapshot.request_message).unwrap(),
        saved_marker
    );
}
