use brn_threads_app::*;

fn fixture(role: &str) -> (tempfile::TempDir, Workspace, Record) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut w = Workspace::open(root.path().join("data")).unwrap();
    let thread = w.new_thread("Choose route").unwrap();
    let op = w.store.allocate_operation("saved-reply").unwrap();
    let outcome = w
        .owner_change(
            "saved-reply",
            &ChangeRequest {
                reason: "Synthetic saved answer".into(),
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: thread.id,
                        role: role.into(),
                        text: "Use route Alpha · õun".into(),
                    }),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = outcome else {
        panic!()
    };
    (root, w, receipt.writes[0].after.clone())
}

#[test]
fn reviewed_decision_is_exact_protected_linked_replayable_and_undoable() {
    let (root, mut w, reply) = fixture("assistant");
    let text = "We choose **Gamma** instead · õun\n\nOwner-reviewed decision.";
    let note = w
        .save_chat_decision("capture", &reply, "Route decision", text)
        .unwrap();
    assert!(
        matches!(&note.data, RecordData::Note(n) if n.title == "Route decision" && n.markdown == text && n.protected && n.confirmed && n.chat.is_none())
    );
    assert!(w.notes().unwrap().iter().any(|n| n.id == note.id));
    let receipt = w
        .history()
        .unwrap()
        .into_iter()
        .find(|r| r.writes.iter().any(|wr| wr.after.id == note.id))
        .unwrap();
    assert_eq!(receipt.writes.len(), 2);
    assert!(
        matches!(&receipt.writes[1].after.data, RecordData::Link(l) if l.to == note.id && l.relation == "decision")
    );
    let prepared = w.store.prepared(&receipt.operation).unwrap();
    assert!(
        prepared
            .request
            .inputs
            .iter()
            .any(|i| i.record == reply.id && i.version == reply.version)
    );
    let RecordData::Message(message) = &reply.data else {
        panic!()
    };
    w.archive_chat("archive", &message.thread).unwrap();
    drop(w);
    let mut w = Workspace::open(root.path().join("data")).unwrap();
    assert_eq!(
        w.save_chat_decision("capture", &reply, "Route decision", text)
            .unwrap()
            .id,
        note.id
    );
    assert!(
        w.save_chat_decision("capture", &reply, "Different decision", text)
            .is_err()
    );
    assert!(matches!(
        w.undo(receipt.operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(!w.notes().unwrap().iter().any(|n| n.id == note.id));
    let history = w.history().unwrap().len();
    assert_eq!(
        w.save_chat_decision("capture", &reply, "Route decision", text)
            .unwrap()
            .id,
        note.id
    );
    assert_eq!(
        w.history().unwrap().len(),
        history,
        "replay cannot undo Undo or create a second decision"
    );
}

#[test]
fn changed_partial_owner_or_fabricated_replies_cannot_be_captured() {
    for role in ["assistant-partial", "owner"] {
        let (_, mut w, reply) = fixture(role);
        assert!(
            w.save_chat_decision("capture", &reply, "Decision", "Text")
                .is_err()
        );
        assert!(w.notes().unwrap().is_empty());
    }
    let (_, mut w, reply) = fixture("assistant");
    let mut fake = reply.clone();
    if let RecordData::Message(m) = &mut fake.data {
        m.thread = "unrelated-chat".into();
    }
    assert!(
        w.save_chat_decision("foreign", &fake, "Decision", "Text")
            .is_err()
    );
    let mut changed = reply.clone();
    if let RecordData::Message(m) = &mut changed.data {
        m.text = "Updated answer".into();
    }
    w.owner_change(
        "update",
        &ChangeRequest {
            reason: "Synthetic reply correction".into(),
            writes: vec![Put {
                id: reply.id.clone(),
                expected_version: Some(reply.version),
                archived: false,
                data: changed.data,
            }],
            inputs: vec![],
        },
    )
    .unwrap();
    assert!(
        w.save_chat_decision("stale", &reply, "Decision", "Text")
            .is_err()
    );
    assert!(w.notes().unwrap().is_empty());
}

#[test]
fn decision_capture_does_not_promote_private_documents_or_figures() {
    let (_, mut w, reply) = fixture("assistant");
    let RecordData::Message(message) = &reply.data else {
        panic!()
    };
    let op = w.store.allocate_operation("private-doc").unwrap();
    let mut private = Note::working("Private attachment", "Private full document");
    private.chat = Some(message.thread.clone());
    private.protected = true;
    w.owner_change(
        "private-doc",
        &ChangeRequest {
            reason: "Synthetic attachment".into(),
            inputs: vec![],
            writes: vec![
                Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(private),
                },
                Put {
                    id: op.creation_id(1),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Asset(Asset {
                        note: op.creation_id(0),
                        source: None,
                        media_type: "image/png".into(),
                        bytes: vec![1, 2, 3],
                    }),
                },
            ],
        },
    )
    .unwrap();
    let markdown = format!("Decision with ![figure](brn-asset://{})", op.creation_id(1));
    assert!(
        w.save_chat_decision("private-figure", &reply, "Decision", &markdown)
            .is_err()
    );
    assert!(w.notes().unwrap().is_empty());
    let note = w
        .save_chat_decision("text-only", &reply, "Decision", "Reviewed conclusion only")
        .unwrap();
    assert_eq!(w.notes().unwrap().len(), 1);
    assert_eq!(w.notes().unwrap()[0].id, note.id);
    assert!(
        matches!(w.store.record(&op.creation_id(0)).unwrap().unwrap().data, RecordData::Note(n) if n.chat.as_deref() == Some(message.thread.as_str()))
    );
    assert_eq!(
        w.store.record(&op.creation_id(1)).unwrap().unwrap().version,
        1
    );
}

#[test]
fn reviewed_decision_keeps_general_figure_references_and_their_versions() {
    let (_, mut w, reply) = fixture("assistant");
    let op = w.store.allocate_operation("general-figure").unwrap();
    w.owner_change(
        "general-figure",
        &ChangeRequest {
            reason: "Synthetic retained figure".into(),
            inputs: vec![],
            writes: vec![
                Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("General reference", "A retained figure")),
                },
                Put {
                    id: op.creation_id(1),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Asset(Asset {
                        note: op.creation_id(0),
                        source: None,
                        media_type: "image/png".into(),
                        bytes: vec![1, 2, 3],
                    }),
                },
            ],
        },
    )
    .unwrap();
    let markdown = format!(
        "Reviewed decision\n\n![Retained figure](brn-asset://{})",
        op.creation_id(1)
    );
    let note = w
        .save_chat_decision("with-figure", &reply, "Decision", &markdown)
        .unwrap();
    assert!(matches!(&note.data, RecordData::Note(n) if n.markdown == markdown));
    let receipt = w
        .history()
        .unwrap()
        .into_iter()
        .find(|r| r.writes.iter().any(|wr| wr.after.id == note.id))
        .unwrap();
    let prepared = w.store.prepared(&receipt.operation).unwrap();
    for i in [0, 1] {
        assert!(
            prepared
                .request
                .inputs
                .iter()
                .any(|input| input.record == op.creation_id(i) && input.version == 1)
        );
        assert_eq!(
            w.store.record(&op.creation_id(i)).unwrap().unwrap().version,
            1
        );
    }
}
