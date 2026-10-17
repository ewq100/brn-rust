use brn_threads_core::*;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
fn root() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn create(store: &mut Store, key: &str, data: RecordData) -> Record {
    let op = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("fixture");
    let request = ChangeRequest {
        reason: key.into(),
        writes: vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data,
        }],
        inputs: vec![],
    };
    store.prepare_owner(&op, &request, &owner).unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!()
    };
    receipt.writes[0].after.clone()
}
fn config(note: &str) -> WritingSetup {
    WritingSetup {
        note: note.into(),
        chat: None,
        brief: Some("Brief".into()),
        references: vec![],
    }
}
fn version(path: &Path) -> i64 {
    Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}
fn downgrade6(store: Store) -> PathBuf {
    let dir = store.directory().to_owned();
    drop(store);
    let db = Connection::open(dir.join(DATABASE_FILENAME)).unwrap();
    db.execute_batch("DROP INDEX one_writing_setup; PRAGMA user_version=6;")
        .unwrap();
    drop(db);
    dir
}
fn tree(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut rows = std::fs::read_dir(dir)
        .unwrap()
        .map(|r| {
            let p = r.unwrap().path();
            let bytes = std::fs::read(&p).unwrap();
            (p, bytes)
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn setup_is_owner_only_and_stale_groups_are_atomic() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "note",
        RecordData::Note(Note::working("Draft", "Original")),
    );
    let op = store.allocate_operation("configuration").unwrap();
    let request = ChangeRequest {
        reason: "Owner config".into(),
        writes: vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::WritingSetup(config(&note.id)),
        }],
        inputs: vec![],
    };
    store.prepare(&op, &request).unwrap();
    for authority in [
        HostAuthority::maintenance("model"),
        HostAuthority::instruction(
            "model",
            "message",
            op.clone(),
            [op.creation_id(0)],
            [Capability::EditContent, Capability::Configure],
        ),
        HostAuthority::runtime("runtime", op.creation_id(0)),
    ] {
        assert!(matches!(
            store.apply(&op, &authority).unwrap(),
            ApplyOutcome::NeedsReview { .. }
        ));
        assert!(store.record(&op.creation_id(0)).unwrap().is_none());
    }
    let owner = HostAuthority::owner("owner");
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!()
    };
    let record = receipt.writes[0].after.clone();
    let stale = store.allocate_operation("stale-config").unwrap();
    let mut next = config(&note.id);
    next.brief = Some("Stale must not land".into());
    let group = ChangeRequest {
        reason: "Atomic group".into(),
        writes: vec![
            Put {
                id: record.id.clone(),
                expected_version: Some(record.version + 1),
                archived: false,
                data: RecordData::WritingSetup(next),
            },
            Put {
                id: note.id.clone(),
                expected_version: Some(note.version),
                archived: false,
                data: RecordData::Note(Note::working("Draft", "Must not land")),
            },
        ],
        inputs: vec![],
    };
    store.prepare_owner(&stale, &group, &owner).unwrap();
    assert!(matches!(
        store.apply(&stale, &owner).unwrap(),
        ApplyOutcome::Stale { .. }
    ));
    assert_eq!(store.record(&note.id).unwrap().unwrap(), note);
    assert_eq!(store.record(&record.id).unwrap().unwrap(), record);
    assert_eq!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(receipt)
    );
}

#[test]
fn immutable_context_duplicate_identity_and_invalid_references_are_refused() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "note",
        RecordData::Note(Note::working("Draft", "text")),
    );
    let other = create(
        &mut store,
        "other",
        RecordData::Note(Note::working("Other", "text")),
    );
    let saved = create(
        &mut store,
        "config",
        RecordData::WritingSetup(config(&note.id)),
    );
    let owner = HostAuthority::owner("owner");
    for (key, data) in [
        ("duplicate", config(&note.id)),
        (
            "duplicate-reference",
            WritingSetup {
                references: vec![WritingReference {
                    note: note.id.clone(),
                    purpose: WritingPurpose::Evidence,
                }],
                ..config(&other.id)
            },
        ),
        (
            "self-reference",
            WritingSetup {
                references: vec![WritingReference {
                    note: other.id.clone(),
                    purpose: WritingPurpose::Evidence,
                }],
                ..config(&other.id)
            },
        ),
    ] {
        let op = store.allocate_operation(key).unwrap();
        let mut data = data;
        if key == "duplicate-reference" {
            data.references.push(data.references[0].clone());
        }
        let request = ChangeRequest {
            reason: key.into(),
            writes: vec![Put {
                id: op.creation_id(0),
                expected_version: None,
                archived: false,
                data: RecordData::WritingSetup(data),
            }],
            inputs: vec![],
        };
        let result = store
            .prepare_owner(&op, &request, &owner)
            .and_then(|_| store.apply(&op, &owner));
        assert!(result.is_err(), "{key}");
        assert!(store.record(&op.creation_id(0)).unwrap().is_none());
    }
    let op = store.allocate_operation("move-context").unwrap();
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "No automatic sharing".into(),
                writes: vec![Put {
                    id: saved.id.clone(),
                    expected_version: Some(saved.version),
                    archived: false,
                    data: RecordData::WritingSetup(config(&other.id)),
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    assert!(store.apply(&op, &owner).is_err());
    assert_eq!(store.record(&saved.id).unwrap().unwrap(), saved);
}

#[test]
fn v1_fixed_payload_and_hash_remain_exact_and_v2_limits_count_utf8_bytes() {
    let legacy = r#"{"version":1,"snapshot":{"request_message":"request:0","thread":"chat:0","note":{"id":"note:0","version":1,"archived":false,"data":{"Note":{"title":"Draft","markdown":"Õun","protected":false,"confirmed":false,"superseded_by":null,"import":null}}},"comments":[],"messages":[]}}"#;
    let record = Record {
        id: "request:0".into(),
        version: 1,
        archived: false,
        data: RecordData::Message(Message {
            thread: "chat:0".into(),
            role: WRITING_REQUEST_ROLE.into(),
            text: legacy.into(),
        }),
    };
    let old = WritingSnapshot::from_message(&record).unwrap();
    assert_eq!(old.message_text().unwrap(), legacy);
    assert_eq!(
        format!("{:x}", Sha256::digest(old.message_text().unwrap())),
        "999b01eab5147cc76e6bbf5490c8a11708dcb69743bdb36ffd59bee919328dcc"
    );
    assert_eq!(old.current_inputs().len(), 2);
    let mut new = old.clone();
    new.setup = Some(WritingSnapshotSetup {
        requirements: WritingReadRequirements::default(),
        chat: None,
        input: RevisionInput {
            record: "setup:0".into(),
            version: 4,
            expectation: InputExpectation::Current,
        },
        brief: Some("é".into()),
        references: vec![WritingReferenceVersion {
            note: "reference:0".into(),
            version: 3,
            purpose: WritingPurpose::Requirements,
        }],
    });
    let initial = new.message_text().unwrap().len();
    new.general_feedback = "x".repeat(MAX_WRITING_REQUEST_BYTES - initial - 22); // field name, quotes and comma
    let actual = new.message_text().unwrap().len();
    // Derive the final byte budget from actual encoding, preserving one exact boundary witness.
    if actual < MAX_WRITING_REQUEST_BYTES {
        new.general_feedback
            .push_str(&"x".repeat(MAX_WRITING_REQUEST_BYTES - actual));
    } else {
        new.general_feedback
            .truncate(new.general_feedback.len() - (actual - MAX_WRITING_REQUEST_BYTES));
    }
    assert_eq!(new.message_text().unwrap().len(), MAX_WRITING_REQUEST_BYTES);
    new.general_feedback.push('é');
    assert!(
        new.message_text()
            .unwrap_err()
            .to_string()
            .contains("nothing was truncated")
    );
    let mut forged = record.clone();
    let RecordData::Message(message) = &mut forged.data else {
        panic!()
    };
    let mut json: serde_json::Value = serde_json::from_str(legacy).unwrap();
    json["version"] = serde_json::json!(2);
    message.text = serde_json::to_string(&json).unwrap();
    assert!(WritingSnapshot::from_message(&forged).is_err());
}

#[test]
fn schema6_refusal_backup_first_upgrade_reopen_and_restore_preserve_v1_candidates() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "note",
        RecordData::Note(Note::working("Draft", "Legacy content")),
    );
    let thread = create(
        &mut store,
        "chat",
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "Writing".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let op = store.allocate_operation("old-request").unwrap();
    let snapshot = WritingSnapshot {
        request_message: op.creation_id(0),
        thread: thread.id.clone(),
        note: note.clone(),
        comments: vec![],
        messages: vec![],
        general_feedback: String::new(),
        setup: None,
    };
    let message = create(
        &mut store,
        "old-request",
        RecordData::Message(Message {
            thread: thread.id,
            role: WRITING_REQUEST_ROLE.into(),
            text: snapshot.message_text().unwrap(),
        }),
    );
    let candidate = store.allocate_operation("old-candidate").unwrap();
    let prepared = store
        .prepare(
            &candidate,
            &snapshot.change_request("Legacy candidate").unwrap(),
        )
        .unwrap();
    let receipt = store.receipt(&op).unwrap();
    let data = downgrade6(store);
    let before = tree(&data);
    assert!(matches!(
        Store::open_existing_without_upgrade(&data),
        Err(Error::UpgradeRequired {
            schema_found: 6,
            schema_required: 9
        })
    ));
    assert_eq!(tree(&data), before);
    assert!(Store::open_reader(&data).is_err());
    assert!(Store::open(&data).is_err());
    assert_eq!(version(&data.join(DATABASE_FILENAME)), 6);
    assert!(Store::upgrade(&data, &data).is_err());
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    Store::upgrade(&data, &backups).unwrap();
    let backup = std::fs::read_dir(&backups)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(version(&backup), 6);
    let backup_bytes = std::fs::read(&backup).unwrap();
    let mut current = Store::open_existing_without_upgrade(&data).unwrap();
    assert_eq!(version(&data.join(DATABASE_FILENAME)), 9);
    assert_eq!(current.prepared(&candidate).unwrap(), prepared);
    assert_eq!(current.receipt(&op).unwrap(), receipt);
    assert_eq!(current.record(&message.id).unwrap().unwrap(), message);
    let owner = HostAuthority::owner("exact review");
    let applied = current.apply(&candidate, &owner).unwrap();
    assert!(matches!(applied, ApplyOutcome::Applied(_)));
    drop(current);
    let mut restored = Store::restore(&backup, root.path().join("restore")).unwrap();
    assert!(!restored.directory().join(DERIVED_DIRECTORY).exists());
    assert_eq!(restored.prepared(&candidate).unwrap(), prepared);
    assert!(matches!(
        restored.apply(&candidate, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(std::fs::read(&backup).unwrap(), backup_bytes);
    Store::upgrade(&data, &backups).unwrap();
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 1);
}

#[test]
fn upgrade_failure_after_backup_rolls_back_all_schema_changes_and_can_resume() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "note",
        RecordData::Note(Note::working("Draft", "Retained")),
    );
    let data = downgrade6(store);
    // Deterministic DDL failure at the real migration seam, after its verified
    // external backup. SQLite must roll back the transaction and schema version.
    let db = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    db.execute_batch("CREATE TABLE one_writing_setup(block_upgrade INTEGER);")
        .unwrap();
    drop(db);
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    assert!(Store::upgrade(&data, &backups).is_err());
    assert_eq!(version(&data.join(DATABASE_FILENAME)), 6);
    let backup = std::fs::read_dir(&backups)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(version(&backup), 6);
    let db = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    db.execute_batch("DROP TABLE one_writing_setup;").unwrap();
    drop(db);
    Store::upgrade(&data, &backups).unwrap();
    let current = Store::open(&data).unwrap();
    assert_eq!(current.record(&note.id).unwrap().unwrap(), note);
}

#[test]
fn schema6_upgrade_refuses_active_edits_and_missing_backup_before_mutation() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "note",
        RecordData::Note(Note::working("Draft", "Original")),
    );
    store.begin_edit(&note.id, 1).unwrap();
    let data = downgrade6(store);
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    assert!(Store::upgrade(&data, &backups).is_err());
    assert_eq!(version(&data.join(DATABASE_FILENAME)), 6);
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 0);
    let db = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    db.execute("UPDATE edit_sessions SET closed=1", []).unwrap();
    drop(db);
    assert!(Store::upgrade(&data, &root.path().join("missing")).is_err());
    assert_eq!(version(&data.join(DATABASE_FILENAME)), 6);
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 0);
}

#[test]
fn v2_candidate_replacement_cannot_drop_or_downgrade_setup_current_input() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "note",
        RecordData::Note(Note::working("Draft", "Saved")),
    );
    let thread = create(
        &mut store,
        "chat",
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "Writing".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let setup = create(
        &mut store,
        "setup",
        RecordData::WritingSetup(config(&note.id)),
    );
    let marker = store.allocate_operation("request").unwrap();
    let capture = WritingSnapshot {
        request_message: marker.creation_id(0),
        thread: thread.id.clone(),
        note,
        comments: vec![],
        messages: vec![],
        general_feedback: String::new(),
        setup: Some(WritingSnapshotSetup {
            requirements: WritingReadRequirements::default(),
            chat: None,
            input: RevisionInput {
                record: setup.id.clone(),
                version: setup.version,
                expectation: InputExpectation::Current,
            },
            brief: Some("Brief".into()),
            references: vec![],
        }),
    };
    create(
        &mut store,
        "request",
        RecordData::Message(Message {
            thread: thread.id,
            role: WRITING_REQUEST_ROLE.into(),
            text: capture.message_text().unwrap(),
        }),
    );
    let op = store.allocate_operation("candidate").unwrap();
    let request = capture.change_request("Result").unwrap();
    store.prepare(&op, &request).unwrap();
    let owner = HostAuthority::owner("exact review");
    for downgrade in [false, true] {
        let replacement = store
            .allocate_operation(&format!("replacement-{downgrade}"))
            .unwrap();
        let mut unsafe_request = request.clone();
        if downgrade {
            unsafe_request
                .inputs
                .iter_mut()
                .find(|i| i.record == setup.id)
                .unwrap()
                .expectation = InputExpectation::Historical;
        } else {
            unsafe_request.inputs.retain(|i| i.record != setup.id);
        }
        assert!(
            store
                .replace_candidate(&op, &replacement, &unsafe_request, &owner)
                .is_err()
        );
    }
}

#[test]
fn owner_host_schema6_open_uses_remembered_external_backup_first() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "note",
        RecordData::Note(Note::working("Draft", "Retained")),
    );
    let folder = root.path().join("remembered");
    std::fs::create_dir(&folder).unwrap();
    create(
        &mut store,
        "settings",
        RecordData::Settings(WorkspaceSettings {
            mcp: None,
            skills: vec![],
            provider: "offline".into(),
            model: "fixture".into(),
            effort: "medium".into(),
            auth_file: None,
            credentials_dir: None,
            maintenance: false,
            review_first: true,
            delegation_chosen: true,
            backup_directory: None,
            theme: None,
            topics_note: None,
            helper: None,
        }),
    );
    let op = store.allocate_operation("backup-selection").unwrap();
    assert!(matches!(
        store
            .choose_backup_directory(&op, &folder, &HostAuthority::owner("owner"))
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let data = downgrade6(store);
    let reopened = Store::open(&data).unwrap();
    assert_eq!(reopened.record(&note.id).unwrap().unwrap(), note);
    let files = std::fs::read_dir(&folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1);
    assert_eq!(version(&files[0]), 6);
    assert_eq!(version(&data.join(DATABASE_FILENAME)), 9);
}

#[test]
fn knowledge_off_working_note_uses_bound_conversation_and_references_keep_project_scope() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "linked-general-note",
        RecordData::Note(Note::working("Draft", "General")),
    );
    let reference = create(
        &mut store,
        "general-reference",
        RecordData::Note(Note::working("Reference", "General evidence")),
    );
    let chat = create(
        &mut store,
        "project-chat",
        RecordData::Thread(Thread {
            private_context: true,
            context_epoch: 0,
            title: "Project".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let project = create(
        &mut store,
        "knowledge-off-project",
        RecordData::Project(Project {
            title: "Knowledge off".into(),
            instructions: String::new(),
            use_brn_knowledge: false,
            chats: vec![chat.id.clone()],
            documents: vec![],
            trash: None,
        }),
    );
    for (key, target) in [
        ("working-link", &note.id),
        ("reference-link", &reference.id),
    ] {
        create(
            &mut store,
            key,
            RecordData::Link(Link {
                from: chat.id.clone(),
                to: target.clone(),
                relation: "discusses".into(),
            }),
        );
    }
    assert!(
        store
            .record_visible_in_chat(&note.id, Some(&chat.id))
            .unwrap()
    );
    assert!(
        !store
            .record_visible_in_chat(&note.id, Some(&project.id))
            .unwrap()
    );
    let owner = HostAuthority::owner("explicit conversation context").for_chat(&chat.id, true);
    let op = store.allocate_operation("checked-project-setup").unwrap();
    let mut setup = config(&note.id);
    setup.chat = Some(project.id.clone());
    let request = ChangeRequest {
        reason: "Project writing setup".into(),
        writes: vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::WritingSetup(setup.clone()),
        }],
        inputs: vec![RevisionInput {
            record: chat.id.clone(),
            version: chat.version,
            expectation: InputExpectation::Current,
        }],
    };
    store.prepare_owner(&op, &request, &owner).unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    setup.references.push(WritingReference {
        note: reference.id.clone(),
        purpose: WritingPurpose::Evidence,
    });
    let edit = store
        .allocate_operation("must-not-widen-references")
        .unwrap();
    let mut request = request;
    request.writes[0].expected_version = Some(1);
    request.writes[0].data = RecordData::WritingSetup(setup);
    store.prepare_owner(&edit, &request, &owner).unwrap();
    assert!(store.apply(&edit, &owner).is_err());
    let wrong = store.allocate_operation("unbound-project-setup").unwrap();
    let mut unbound = config(&reference.id);
    unbound.chat = Some(project.id);
    let req = ChangeRequest {
        reason: "Missing conversation proof".into(),
        writes: vec![Put {
            id: wrong.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::WritingSetup(unbound),
        }],
        inputs: vec![],
    };
    store
        .prepare_owner(&wrong, &req, &HostAuthority::owner("unbound"))
        .unwrap();
    assert!(
        store
            .apply(&wrong, &HostAuthority::owner("unbound"))
            .is_err()
    );
}

#[test]
fn deferred_setup_undo_keeps_identity_after_unrelated_conversation_attention_and_restart() {
    for case in [
        "general-attention",
        "project-attention",
        "project-knowledge",
    ] {
        let needs_conversation = case != "general-attention";
        let root = root();
        let data = root.path().join("data");
        let mut store = Store::open(&data).unwrap();
        let owner = HostAuthority::owner("owner");
        let note = create(
            &mut store,
            "note",
            RecordData::Note(Note::working("Draft", "Before")),
        );
        let chat = create(
            &mut store,
            "ordinary-chat",
            RecordData::Thread(Thread {
                private_context: needs_conversation,
                context_epoch: 0,
                title: "General".into(),
                state: ThreadState::Open,
                attention: vec![],
            }),
        );
        let setup = if needs_conversation {
            let project = create(
                &mut store,
                "project",
                RecordData::Project(Project {
                    title: "Knowledge off".into(),
                    instructions: String::new(),
                    use_brn_knowledge: false,
                    chats: vec![chat.id.clone()],
                    documents: vec![],
                    trash: None,
                }),
            );
            create(
                &mut store,
                "working-link",
                RecordData::Link(Link {
                    from: chat.id.clone(),
                    to: note.id.clone(),
                    relation: "discusses".into(),
                }),
            );
            let mut data = config(&note.id);
            data.chat = Some(project.id);
            let op = store.allocate_operation("setup").unwrap();
            store
                .prepare_owner(
                    &op,
                    &ChangeRequest {
                        reason: "Project setup with actual conversation access".into(),
                        writes: vec![Put {
                            id: op.creation_id(0),
                            expected_version: None,
                            archived: false,
                            data: RecordData::WritingSetup(data),
                        }],
                        inputs: vec![RevisionInput {
                            record: chat.id.clone(),
                            version: chat.version,
                            expectation: InputExpectation::Current,
                        }],
                    },
                    &owner,
                )
                .unwrap();
            let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
                panic!()
            };
            receipt.writes[0].after.clone()
        } else {
            create(
                &mut store,
                "setup",
                RecordData::WritingSetup(config(&note.id)),
            )
        };
        let original = store.allocate_operation("owner-group").unwrap();
        let RecordData::WritingSetup(mut changed) = setup.data.clone() else {
            panic!()
        };
        changed.brief = Some("Changed".into());
        store
            .prepare_owner(
                &original,
                &ChangeRequest {
                    reason: "Owner group".into(),
                    writes: vec![
                        Put {
                            id: setup.id,
                            expected_version: Some(1),
                            archived: false,
                            data: RecordData::WritingSetup(changed),
                        },
                        Put {
                            id: note.id.clone(),
                            expected_version: Some(1),
                            archived: false,
                            data: RecordData::Note(Note::working("Draft", "After")),
                        },
                    ],
                    inputs: vec![RevisionInput {
                        record: chat.id.clone(),
                        version: chat.version,
                        expectation: InputExpectation::Current,
                    }],
                },
                &owner,
            )
            .unwrap();
        assert!(matches!(
            store.apply(&original, &owner).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        let edit = store.begin_edit(&note.id, 2).unwrap();
        let compensation = store.allocate_operation("stable-undo").unwrap();
        assert!(matches!(
            store.undo(&compensation, &original, &owner).unwrap(),
            ApplyOutcome::Deferred { .. }
        ));
        let prepared = store.prepared(&compensation).unwrap();
        let mut data_chat = chat.data;
        if let RecordData::Thread(t) = &mut data_chat {
            if case == "project-knowledge" {
                t.context_epoch += 1;
            }
            t.attention.push(Attention {
                kind: AttentionKind::Question,
                reason: "Unrelated attention".into(),
                record: None,
            });
        }
        let mut changed_context = vec![Put {
            id: chat.id.clone(),
            expected_version: Some(1),
            archived: false,
            data: data_chat,
        }];
        if case == "project-knowledge" {
            let project = store
                .records()
                .unwrap()
                .into_iter()
                .find(|r| matches!(r.data, RecordData::Project(_)))
                .unwrap();
            let mut data = project.data;
            if let RecordData::Project(p) = &mut data {
                p.use_brn_knowledge = true;
            }
            changed_context.push(Put {
                id: project.id,
                expected_version: Some(project.version),
                archived: false,
                data,
            });
        }
        let attention = store.allocate_operation("attention").unwrap();
        store
            .prepare_owner(
                &attention,
                &ChangeRequest {
                    reason: "Unrelated attention".into(),
                    writes: changed_context,
                    inputs: vec![],
                },
                &owner,
            )
            .unwrap();
        store.apply(&attention, &owner).unwrap();
        store.discard(&edit.id, edit.generation).unwrap();
        drop(store);
        let mut reopened = Store::open(root.path().join("data")).unwrap();
        let outcome = reopened.undo(&compensation, &original, &owner).unwrap();
        let expected = if case == "project-knowledge" {
            assert!(matches!(outcome, ApplyOutcome::Stale { .. }));
            assert!(reopened.receipt(&compensation).unwrap().is_none());
            "After"
        } else {
            assert!(matches!(outcome, ApplyOutcome::Applied(_)));
            "Before"
        };
        assert_eq!(reopened.prepared(&compensation).unwrap(), prepared);
        assert!(
            matches!(reopened.record(&note.id).unwrap().unwrap().data,RecordData::Note(n) if n.markdown==expected)
        );
    }
}

#[test]
fn raw_ordinary_conversation_setup_is_rejected_without_creating_private_scope() {
    let root = root();
    let mut store = Store::open(root.path().join("data")).unwrap();
    let note = create(
        &mut store,
        "note",
        RecordData::Note(Note::working("Draft", "General")),
    );
    let chat = create(
        &mut store,
        "ordinary",
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "Ordinary".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let before = store.records().unwrap();
    let op = store.allocate_operation("raw-thread-scope").unwrap();
    let mut setup = config(&note.id);
    setup.chat = Some(chat.id.clone());
    let request = ChangeRequest {
        reason: "Raw noncanonical conversation setup".into(),
        writes: vec![Put {
            id: op.creation_id(0),
            expected_version: None,
            archived: false,
            data: RecordData::WritingSetup(setup),
        }],
        inputs: vec![],
    };
    let owner = HostAuthority::owner("owner");
    store.prepare_owner(&op, &request, &owner).unwrap();
    assert!(store.apply(&op, &owner).is_err());
    assert_eq!(store.records().unwrap(), before);
    assert_eq!(store.record(&chat.id).unwrap().unwrap(), chat);
    assert_eq!(
        writing_setup_scope(&store.records().unwrap(), Some(&chat.id)).unwrap(),
        None
    );
    assert!(store.receipt(&op).unwrap().is_none());
    for invalid in ["missing-scope", note.id.as_str()] {
        assert!(writing_setup_scope(&store.records().unwrap(), Some(invalid)).is_err());
    }
}

#[test]
fn checked_setup_mutation_requires_active_context_even_with_current_thread_input() {
    for private in [false, true] {
        let root = root();
        let mut store = Store::open(root.path().join("data")).unwrap();
        let note = create(
            &mut store,
            "note",
            RecordData::Note(Note::working("Draft", "General")),
        );
        let chat = create(
            &mut store,
            "resolved",
            RecordData::Thread(Thread {
                private_context: private,
                context_epoch: 0,
                title: "Resolved".into(),
                state: ThreadState::Resolved,
                attention: vec![],
            }),
        );
        let before = store.records().unwrap();
        let op = store.allocate_operation("inactive-config").unwrap();
        let mut setup = config(&note.id);
        if private {
            setup.chat = Some(chat.id.clone());
        }
        let owner = HostAuthority::owner("selected inactive context").for_chat(&chat.id, private);
        store
            .prepare_owner(
                &op,
                &ChangeRequest {
                    reason: "New configuration in inactive context".into(),
                    writes: vec![Put {
                        id: op.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::WritingSetup(setup),
                    }],
                    inputs: vec![RevisionInput {
                        record: chat.id,
                        version: chat.version,
                        expectation: InputExpectation::Current,
                    }],
                },
                &owner,
            )
            .unwrap();
        assert!(store.apply(&op, &owner).is_err());
        assert_eq!(store.records().unwrap(), before);
        assert!(store.receipt(&op).unwrap().is_none());
    }
}
