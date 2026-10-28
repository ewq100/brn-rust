//! Capture's persistence/review/privacy contract without providers or model admission.
use brn_threads_app::*;
fn fixture(private: bool) -> (tempfile::TempDir, Workspace, String) {
    let root = tempfile::tempdir().unwrap();
    let mut w = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    w.choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    let chat = w.new_thread("Synthetic capture").unwrap();
    if private {
        let mut record = chat.clone();
        if let RecordData::Thread(t) = &mut record.data {
            t.private_context = true;
        }
        w.owner_change(
            "private-fixture",
            &ChangeRequest {
                reason: "Private fixture".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: record.id,
                    expected_version: Some(record.version),
                    archived: false,
                    data: record.data,
                }],
            },
        )
        .unwrap();
    }
    w.post_message(
        &chat.id,
        "We chose the blue design. Mari will ask for approval; no completed result yet.",
    )
    .unwrap();
    (root, w, chat.id)
}
fn proposal(
    w: &mut Workspace,
    snapshot: &CaptureSnapshot,
    key: &str,
) -> (OperationId, ChangeRequest) {
    let op = w.store.allocate_operation(key).unwrap();
    let mut note = Note::working(
        "Design decision",
        "Proposed account of the blue design; not yet owner-approved.",
    );
    note.chat = snapshot.private_context.then_some(snapshot.thread.clone());
    let request = ChangeRequest {
        reason: "Useful outcome for review".into(),
        inputs: snapshot.current_inputs(),
        writes: vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::Note(note),
        }],
    };
    (op, request)
}
#[test]
fn capture_snapshot_replays_without_knowledge_writes_and_excludes_unposted_or_host_messages() {
    let (_root, mut w, chat) = fixture(false);
    w.queue_message("unposted", &chat, "DRAFT MUST NOT ENTER SNAPSHOT")
        .unwrap();
    let queue = w.chat_control().unwrap();
    let snapshot = w.snapshot_capture_request("capture-one", &chat).unwrap();
    assert_eq!(snapshot.messages.len(), 1);
    assert_eq!(
        snapshot,
        w.snapshot_capture_request("capture-one", &chat).unwrap()
    );
    assert_eq!(w.chat_control().unwrap(), queue);
    assert!(
        !w.records()
            .unwrap()
            .iter()
            .any(|r| matches!(r.data, RecordData::Note(_) | RecordData::Action(_)))
    );
    let other = w.new_thread("Other").unwrap();
    assert!(
        w.snapshot_capture_request("capture-one", &other.id)
            .is_err()
    );
    let fresh = w.snapshot_capture_request("capture-two", &chat).unwrap();
    assert_eq!(fresh.messages, snapshot.messages);
}
#[test]
fn capture_stays_review_only_after_reopen_ordinary_mode_unchanged_owner_apply_and_undo_work() {
    let (root, mut w, chat) = fixture(false);
    let snapshot = w.snapshot_capture_request("capture", &chat).unwrap();
    let (op, request) = proposal(&mut w, &snapshot, "candidate");
    w.store.prepare(&op, &request).unwrap();
    let maintenance = HostAuthority::maintenance("ordinary later runtime");
    assert!(matches!(
        w.store.apply(&op, &maintenance).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    drop(w);
    let mut w = Workspace::open_existing_no_upgrade(root.path().join("data")).unwrap();
    assert!(matches!(
        w.store.apply(&op, &maintenance).unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert!(w.store.receipt(&op).unwrap().is_none());
    assert!(matches!(
        w.store
            .apply(&op, &HostAuthority::owner("review selected proposal"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let undo = w.store.allocate_operation("undo-capture").unwrap();
    assert!(matches!(
        w.store
            .undo(&undo, &op, &HostAuthority::owner("undo"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        w.store
            .record(&op.creation_id(0))
            .unwrap()
            .unwrap()
            .archived
    );
    let ordinary = w.store.allocate_operation("ordinary").unwrap();
    let mut request = request;
    request.inputs.clear();
    request.writes[0].id = ordinary.creation_id(0);
    w.store.prepare(&ordinary, &request).unwrap();
    assert!(matches!(
        w.store.apply(&ordinary, &maintenance).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        matches!(w.store.record(&chat).unwrap().unwrap().data,RecordData::Thread(t) if t.state==ThreadState::Open)
    );
}
#[test]
fn replacement_cannot_remove_or_downgrade_review_binding_or_archive_chat() {
    let (_root, mut w, chat) = fixture(false);
    let snapshot = w.snapshot_capture_request("capture", &chat).unwrap();
    let (op, request) = proposal(&mut w, &snapshot, "candidate");
    w.store.prepare(&op, &request).unwrap();
    for (key, drop_marker) in [("strip", true), ("historical", false)] {
        let replacement = w.store.allocate_operation(key).unwrap();
        let mut changed = request.clone();
        changed.writes[0].id = replacement.creation_id(0);
        if drop_marker {
            changed
                .inputs
                .retain(|i| i.record != snapshot.request_message);
        } else {
            changed
                .inputs
                .iter_mut()
                .find(|i| i.record == snapshot.request_message)
                .unwrap()
                .expectation = InputExpectation::Historical;
        }
        assert!(
            w.store
                .replace_candidate(
                    &op,
                    &replacement,
                    &changed,
                    &HostAuthority::owner("review edit")
                )
                .is_err()
        );
    }
    let invalid = w.store.allocate_operation("archive").unwrap();
    let mut changed = request.clone();
    changed.writes[0].id = invalid.creation_id(0);
    changed.writes[0].archived = true;
    assert!(w.store.prepare(&invalid, &changed).is_err());
    assert_eq!(w.store.active_candidates(&[op]).unwrap().len(), 1);
}
#[test]
fn private_capture_cannot_publish_general_and_stale_posted_evidence_refuses_owner_apply() {
    let (_root, mut w, chat) = fixture(true);
    let snapshot = w.snapshot_capture_request("capture", &chat).unwrap();
    let (op, mut request) = proposal(&mut w, &snapshot, "wrong-destination");
    if let RecordData::Note(n) = &mut request.writes[0].data {
        n.chat = None;
    }
    w.store.prepare(&op, &request).unwrap();
    assert!(w.store.apply(&op, &HostAuthority::owner("review")).is_err());
    let (proper, request) = proposal(&mut w, &snapshot, "correct");
    w.store.prepare(&proper, &request).unwrap();
    let original = snapshot.messages[0].clone();
    let RecordData::Message(mut message) = original.data else {
        panic!()
    };
    message.text = "Owner corrected the discussion".into();
    w.owner_change(
        "correct-message",
        &ChangeRequest {
            reason: "Synthetic edit".into(),
            inputs: vec![],
            writes: vec![Put {
                id: original.id,
                expected_version: Some(original.version),
                archived: false,
                data: RecordData::Message(message),
            }],
        },
    )
    .unwrap();
    assert!(matches!(
        w.store
            .apply(&proper, &HostAuthority::owner("review"))
            .unwrap(),
        ApplyOutcome::Stale { .. }
    ));
    assert!(w.store.receipt(&proper).unwrap().is_none());
}
#[test]
fn empty_and_oversized_capture_refuse_without_saving_partial_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let mut w = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
    let chat = w.new_thread("Empty").unwrap();
    assert!(w.snapshot_capture_request("empty", &chat.id).is_err());
    w.post_message(&chat.id, &"x".repeat(MAX_CAPTURE_BYTES))
        .unwrap();
    assert!(w.snapshot_capture_request("large", &chat.id).is_err());
    assert!(
        !w.records()
            .unwrap()
            .iter()
            .any(|r| matches!(&r.data,RecordData::Message(m) if m.role==CAPTURE_REQUEST_ROLE))
    );
}

#[test]
fn consent_preview_is_read_only_and_cannot_change_destination_at_confirmation() {
    let (_root, mut w, chat) = fixture(false);
    let records = w.records().unwrap();
    let preview = w.preview_capture_request(&chat).unwrap();
    assert_eq!(w.records().unwrap(), records);
    let record = w.store.record(&chat).unwrap().unwrap();
    let RecordData::Thread(mut thread) = record.data else {
        panic!()
    };
    thread.private_context = true;
    w.owner_change(
        "context-changed",
        &ChangeRequest {
            reason: "Other window changed privacy".into(),
            inputs: vec![],
            writes: vec![Put {
                id: chat,
                expected_version: Some(record.version),
                archived: false,
                data: RecordData::Thread(thread),
            }],
        },
    )
    .unwrap();
    let changed = w.records().unwrap();
    assert!(w.save_capture_request("old-consent", &preview).is_err());
    assert_eq!(w.records().unwrap(), changed);
    assert!(!changed.iter().any(|r| matches!(r.data, RecordData::Run(_))));
}

#[test]
fn capture_refuses_posted_or_metadata_history_overflow_without_request_or_run() {
    for (role, count) in [("owner", 257), ("system", 1025)] {
        let root = tempfile::tempdir().unwrap();
        let mut w = Workspace::open_without_default_credentials(root.path().join("data")).unwrap();
        let chat = w.new_thread("Bounded synthetic history").unwrap();
        let op = w.store.allocate_operation("history-fixture").unwrap();
        let request = ChangeRequest {
            reason: "Synthetic bounded-history witness".into(),
            inputs: vec![],
            writes: (0..count)
                .map(|index| Put {
                    id: op.creation_id(index),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: chat.id.clone(),
                        role: role.into(),
                        text: "Synthetic history".into(),
                    }),
                })
                .collect(),
        };
        let owner = HostAuthority::owner("synthetic fixture");
        w.store.prepare_owner(&op, &request, &owner).unwrap();
        assert!(matches!(
            w.store.apply(&op, &owner).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        let before = w.records().unwrap();
        let error = w.preview_capture_request(&chat.id).unwrap_err().to_string();
        assert!(error.contains("1024 history records"), "{error}");
        assert_eq!(w.records().unwrap(), before);
        assert!(!w.records().unwrap().iter().any(|r| matches!(&r.data, RecordData::Run(_)) || matches!(&r.data, RecordData::Message(Message {role, ..}) if role == CAPTURE_REQUEST_ROLE)));
    }
}
