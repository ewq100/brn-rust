//! Semantic format admission: older writers must not ignore Capture's durable fence.
use brn_threads_core::*;
use rusqlite::{Connection, OpenFlags, types::Value};
use std::path::Path;

fn version(path: &Path) -> i64 {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap()
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}
fn canonical(path: &Path) -> Vec<(String, Vec<Vec<Value>>)> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    [
        "records",
        "revisions",
        "operations",
        "receipts",
        "receipt_actors",
        "derivation_inputs",
        "candidate_targets",
    ]
    .into_iter()
    .map(|table| {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {table} ORDER BY rowid"))
            .unwrap();
        let columns = stmt.column_count();
        let rows = stmt
            .query_map([], |row| {
                (0..columns)
                    .map(|i| row.get(i))
                    .collect::<rusqlite::Result<Vec<Value>>>()
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        (table.into(), rows)
    })
    .collect()
}
fn create(store: &mut Store, key: &str, data: Vec<RecordData>) -> OperationId {
    let op = store.allocate_operation(key).unwrap();
    let request = ChangeRequest {
        reason: "Synthetic format fixture".into(),
        inputs: vec![],
        writes: data
            .into_iter()
            .enumerate()
            .map(|(i, data)| Put {
                id: op.creation_id(i),
                expected_version: None,
                archived: false,
                data,
            })
            .collect(),
    };
    let owner = HostAuthority::owner("synthetic fixture");
    store.prepare_owner(&op, &request, &owner).unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    op
}
fn pending(store: &mut Store, key: &str, inputs: Vec<RevisionInput>) -> OperationId {
    let op = store.allocate_operation(key).unwrap();
    store
        .prepare(
            &op,
            &ChangeRequest {
                reason: "Pending synthetic proposal".into(),
                inputs,
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Proposed Ω", "Exact\r\nUTF-8 α\n")),
                }],
            },
        )
        .unwrap();
    op
}

#[test]
fn schema12_upgrade_and_restore_preserve_exact_history_and_pending_requests() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    create(
        &mut store,
        "saved",
        vec![RecordData::Note(Note::working("Saved Ω", "unchanged\r\nα"))],
    );
    let candidate = pending(&mut store, "legacy-pending", vec![]);
    let expected = store.prepared(&candidate).unwrap();
    drop(store);
    let db = data.join(DATABASE_FILENAME);
    // Schema13 changes only admission semantics: this fixture contains solely
    // pre-Capture types/operations and is a representative schema12 payload.
    Connection::open(&db)
        .unwrap()
        .pragma_update(None, "user_version", 12)
        .unwrap();
    let original = canonical(&db);
    let bytes = std::fs::read(&db).unwrap();
    assert!(matches!(
        Store::open_existing_without_upgrade(&data),
        Err(Error::UpgradeRequired {
            schema_found: 12,
            schema_required: 13
        })
    ));
    assert_eq!(std::fs::read(&db).unwrap(), bytes);
    // No remembered/chosen external backup directory: fail without mutation.
    assert!(Store::open(&data).is_err());
    assert_eq!(std::fs::read(&db).unwrap(), bytes);
    let backups = root.path().join("backups");
    std::fs::create_dir(&backups).unwrap();
    Store::upgrade(&data, &backups).unwrap();
    assert_eq!(version(&db), 13);
    assert_eq!(canonical(&db), original);
    let store = Store::open_existing_without_upgrade(&data).unwrap();
    assert_eq!(store.prepared(&candidate).unwrap(), expected);
    assert_eq!(
        store
            .active_candidates(std::slice::from_ref(&candidate))
            .unwrap()
            .len(),
        1
    );
    drop(store);
    let snapshot = std::fs::read_dir(&backups)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(
        snapshot
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("brn-before-schema-13-")
    );
    assert_eq!(version(&snapshot), 12);
    assert_eq!(canonical(&snapshot), original);
    let source_bytes = std::fs::read(&snapshot).unwrap();
    let restored = Store::restore(&snapshot, root.path().join("restored12")).unwrap();
    assert_eq!(version(&restored.directory().join(DATABASE_FILENAME)), 13);
    assert_eq!(
        canonical(&restored.directory().join(DATABASE_FILENAME)),
        original
    );
    assert_eq!(restored.prepared(&candidate).unwrap(), expected);
    assert_eq!(std::fs::read(&snapshot).unwrap(), source_bytes);
}

#[test]
fn schema13_capture_backup_restore_retains_review_fence_and_owner_undo() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut store = Store::open(&data).unwrap();
    create(
        &mut store,
        "standing-maintenance",
        vec![RecordData::Settings(WorkspaceSettings {
            document_conversion: Default::default(),
            mcp: None,
            skills: vec![],
            provider: "synthetic".into(),
            model: "synthetic".into(),
            effort: "medium".into(),
            auth_file: None,
            credentials_dir: None,
            maintenance: true,
            review_first: false,
            delegation_chosen: true,
            backup_directory: None,
            theme: None,
            topics_note: None,
            helper: None,
        })],
    );
    let chat = create(
        &mut store,
        "chat",
        vec![RecordData::Thread(Thread {
            title: "Synthetic Capture".into(),
            state: ThreadState::Open,
            attention: vec![],
            private_context: false,
            context_epoch: 0,
        })],
    )
    .creation_id(0);
    let message = create(
        &mut store,
        "posted",
        vec![RecordData::Message(Message {
            thread: chat.clone(),
            role: "owner".into(),
            text: "A proposal, not approval".into(),
        })],
    )
    .creation_id(0);
    let marker = store.allocate_operation("capture-request").unwrap();
    let snapshot = CaptureSnapshot {
        version: 1,
        request_message: marker.creation_id(0),
        thread: chat.clone(),
        private_context: false,
        project: None,
        messages: vec![store.record(&message).unwrap().unwrap()],
        context: vec![RevisionInput {
            record: chat,
            version: 1,
            expectation: InputExpectation::ChatContext,
        }],
    };
    let request = ChangeRequest {
        reason: "Synthetic explicit Capture".into(),
        inputs: vec![],
        writes: vec![Put {
            id: snapshot.request_message.clone(),
            expected_version: None,
            archived: false,
            data: RecordData::Message(Message {
                thread: snapshot.thread.clone(),
                role: CAPTURE_REQUEST_ROLE.into(),
                text: snapshot.message_text().unwrap(),
            }),
        }],
    };
    let owner = HostAuthority::owner("owner review");
    store.prepare_owner(&marker, &request, &owner).unwrap();
    store.apply(&marker, &owner).unwrap();
    let candidate = pending(&mut store, "capture-pending", snapshot.current_inputs());
    let ordinary = pending(&mut store, "ordinary-positive-control", vec![]);
    let expected = store.prepared(&candidate).unwrap();
    let before = canonical(&data.join(DATABASE_FILENAME));
    let backup = root.path().join("capture13.sqlite3");
    store.backup(&backup).unwrap();
    drop(store);
    assert_eq!(version(&backup), 13);
    let source_bytes = std::fs::read(&backup).unwrap();
    let mut restored = Store::restore(&backup, root.path().join("restored13")).unwrap();
    assert_eq!(
        canonical(&restored.directory().join(DATABASE_FILENAME)),
        before
    );
    assert_eq!(restored.prepared(&candidate).unwrap(), expected);
    assert!(matches!(
        restored
            .apply(
                &ordinary,
                &HostAuthority::maintenance("same authority ordinary control")
            )
            .unwrap(),
        ApplyOutcome::Applied(_)
    ));

    assert!(matches!(
        restored
            .apply(
                &candidate,
                &HostAuthority::maintenance("later ordinary run")
            )
            .unwrap(),
        ApplyOutcome::NeedsReview { .. }
    ));
    assert!(restored.receipt(&candidate).unwrap().is_none());
    // Optional retained synthetic workspace for a separately source-qualified
    // historical schema12 executable refusal witness. Never a real owner DB.
    if let Some(path) = std::env::var_os("BRN_CAPTURE_FORMAT_EVIDENCE") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        let retained = Store::restore(&backup, path.join("schema13-data")).unwrap();
        std::fs::write(path.join("candidate-operation.txt"), candidate.as_str()).unwrap();
        assert_eq!(retained.prepared(&candidate).unwrap(), expected);
    }
    assert!(matches!(
        restored.apply(&candidate, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    let undo = restored.allocate_operation("undo-capture").unwrap();
    assert!(matches!(
        restored.undo(&undo, &candidate, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert!(
        restored
            .record(&candidate.creation_id(0))
            .unwrap()
            .unwrap()
            .archived
    );
    assert_eq!(std::fs::read(&backup).unwrap(), source_bytes);
}
