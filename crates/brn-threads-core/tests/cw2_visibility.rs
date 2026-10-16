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

#[test]
fn canonical_view_token_ignores_local_buffers_but_tracks_exact_candidate_pool_and_access_changes() {
    let (_root, mut store, snapshot, reference, elsewhere) = fixture();
    let before = store.change_token().unwrap();
    let operation = store.allocate_operation("token-candidate").unwrap();
    assert_eq!(
        store.change_token().unwrap(),
        before,
        "allocation alone is not a prepared candidate"
    );
    let request = snapshot.change_request("Reviewed text").unwrap();
    store.prepare(&operation, &request).unwrap();
    let prepared = store.change_token().unwrap();
    assert_ne!(prepared, before);
    assert_eq!(
        prepared.0, before.0,
        "candidate preparation has no canonical revision"
    );
    let session = store.begin_edit(&snapshot.note.id, 1).unwrap();
    let BufferOutcome::Updated(updated) = store
        .update_buffer(&session.id, 1, "Real unsaved buffer")
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(
        store.edit_session(&session.id).unwrap().markdown,
        "Real unsaved buffer"
    );
    assert_eq!(
        store.change_token().unwrap(),
        prepared,
        "local edit-buffer writes must not invalidate canonical evidence"
    );
    store.discard(&updated.id, updated.generation).unwrap();
    assert_eq!(store.change_token().unwrap(), prepared);
    let replacement = store.allocate_operation("token-replacement").unwrap();
    let replacement_request = snapshot.change_request("Owner's exact edit").unwrap();
    store
        .replace_candidate(
            &operation,
            &replacement,
            &replacement_request,
            &HostAuthority::owner("explicit exact edit"),
        )
        .unwrap();
    let replaced = store.change_token().unwrap();
    assert_ne!(
        replaced, prepared,
        "same candidate count must not hide replacement"
    );
    assert_eq!(replaced.0, before.0);
    store.retire(&replacement).unwrap();
    let retired = store.change_token().unwrap();
    assert_ne!(retired, replaced);
    assert_eq!(retired.0, before.0);
    let mut external = Store::open(store.directory()).unwrap();
    change_reference(&mut external, &reference, Some(&elsewhere), false);
    let revoked = store.change_token().unwrap();
    assert_ne!(revoked, retired);
    assert_ne!(
        revoked.1, retired.1,
        "external commit invalidates the same connection's view"
    );
    change_reference(&mut store, &reference, None, false);
    assert_ne!(
        store.change_token().unwrap(),
        revoked,
        "same-connection access restoration is canonical too"
    );
}

#[test]
fn actual_owner_candidate_apply_keeps_only_working_note_selected_conversation_exception() {
    let (_root, mut store, mut snapshot, reference, _) = fixture();
    let project_op = store.allocate_operation("strict-project").unwrap();
    let project = project_op.creation_id(0);
    commit(
        &mut store,
        "strict-project",
        vec![Put {
            id: project.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Project(Project {
                title: "Knowledge off".into(),
                instructions: String::new(),
                use_brn_knowledge: false,
                chats: vec![snapshot.thread.clone()],
                documents: vec![],
                trash: None,
            }),
        }],
    );
    let links = store.allocate_operation("strict-links").unwrap();
    commit(
        &mut store,
        "strict-links",
        [&snapshot.note.id, &reference]
            .into_iter()
            .enumerate()
            .map(|(i, id)| Put {
                id: links.creation_id(i),
                expected_version: None,
                archived: false,
                data: RecordData::Link(Link {
                    from: snapshot.thread.clone(),
                    to: id.clone(),
                    relation: "discusses".into(),
                }),
            })
            .collect(),
    );
    for id in [&snapshot.note.id, &reference] {
        assert!(
            store
                .record_visible_in_chat(id, Some(&snapshot.thread))
                .unwrap()
        );
        assert!(!store.record_visible_in_chat(id, Some(&project)).unwrap());
    }
    let setup_op = store.allocate_operation("strict-setup").unwrap();
    let setup_id = setup_op.creation_id(0);
    let thread = store.record(&snapshot.thread).unwrap().unwrap();
    let owner = HostAuthority::owner("explicit scoped setup").for_chat(&snapshot.thread, true);
    store
        .prepare_owner(
            &setup_op,
            &ChangeRequest {
                reason: "Scoped setup".into(),
                writes: vec![Put {
                    id: setup_id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::WritingSetup(WritingSetup {
                        note: snapshot.note.id.clone(),
                        chat: Some(project.clone()),
                        brief: None,
                        references: vec![],
                    }),
                }],
                inputs: vec![RevisionInput {
                    record: thread.id,
                    version: thread.version,
                    expectation: InputExpectation::Current,
                }],
            },
            &owner,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&setup_op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let marker = store.allocate_operation("strict-request").unwrap();
    snapshot.request_message = marker.creation_id(0);
    snapshot.setup = Some(WritingSnapshotSetup {
        requirements: WritingReadRequirements::default(),
        chat: Some(project),
        input: RevisionInput {
            record: setup_id,
            version: 1,
            expectation: InputExpectation::Current,
        },
        brief: None,
        references: vec![],
    });
    commit(
        &mut store,
        "strict-request",
        vec![Put {
            id: snapshot.request_message.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: snapshot.thread.clone(),
                role: WRITING_REQUEST_ROLE.into(),
                text: snapshot.message_text().unwrap(),
            }),
        }],
    );
    let denied = store.allocate_operation("strict-denied-candidate").unwrap();
    let mut request = snapshot
        .change_request("Must not use unrelated selected-chat evidence")
        .unwrap();
    request.inputs.push(RevisionInput {
        record: reference.clone(),
        version: 1,
        expectation: InputExpectation::Historical,
    });
    store.prepare(&denied, &request).unwrap();
    let records = store.records().unwrap();
    let exact = store.prepared(&denied).unwrap();
    assert!(
        matches!(store.apply(&denied, &HostAuthority::owner("exact owner review")).unwrap(), ApplyOutcome::NeedsReview { denied } if denied.contains(&reference) && !denied.contains(&snapshot.note.id))
    );
    assert_eq!(store.records().unwrap(), records);
    assert_eq!(store.prepared(&denied).unwrap(), exact);
    assert!(store.receipt(&denied).unwrap().is_none());
    store.retire(&denied).unwrap();
    let allowed = store.allocate_operation("strict-working-only").unwrap();
    store
        .prepare(
            &allowed,
            &snapshot
                .change_request("Working-Note-only positive control")
                .unwrap(),
        )
        .unwrap();
    assert!(matches!(
        store
            .apply(&allowed, &HostAuthority::owner("new explicit exact review"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
}

#[test]
fn actual_owner_apply_rejects_archived_historical_input_even_when_current_revision_is_visible() {
    let (_root, mut store, snapshot, reference, _) = fixture();
    change_reference(&mut store, &reference, None, true);
    let archived_version = store.record(&reference).unwrap().unwrap().version;
    change_reference(&mut store, &reference, None, false);
    assert!(
        store
            .record_visible_in_chat(&reference, Some(&snapshot.thread))
            .unwrap()
    );
    assert!(
        store
            .revision(&reference, archived_version)
            .unwrap()
            .unwrap()
            .archived
    );
    let operation = store
        .allocate_operation("archived-history-candidate")
        .unwrap();
    let mut request = snapshot
        .change_request("Exact owner-reviewed historical input")
        .unwrap();
    request.inputs.push(RevisionInput {
        record: reference.clone(),
        version: archived_version,
        expectation: InputExpectation::Historical,
    });
    store.prepare(&operation, &request).unwrap();
    let records = store.records().unwrap();
    let candidate = store.prepared(&operation).unwrap();
    assert!(
        matches!(store.apply(&operation, &HostAuthority::owner("exact owner review")).unwrap(), ApplyOutcome::NeedsReview { denied } if denied.contains(&reference))
    );
    assert_eq!(store.records().unwrap(), records);
    assert_eq!(store.prepared(&operation).unwrap(), candidate);
    assert!(store.receipt(&operation).unwrap().is_none());
}
