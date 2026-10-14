use brn_threads_core::*;
use rusqlite::Connection;

fn owner_record(store: &mut Store, key: &str, data: RecordData) -> Record {
    let op = store.allocate_operation(key).unwrap();
    let owner = HostAuthority::owner("synthetic fixture");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data,
                }],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = store.apply(&op, &owner).unwrap() else {
        panic!()
    };
    receipt.writes[0].after.clone()
}

#[test]
fn scoped_ids_assets_and_sources_do_not_enter_unrelated_context() {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(root.path().canonicalize().unwrap().join("data")).unwrap();
    let chat = owner_record(
        &mut store,
        "chat",
        RecordData::Thread(Thread {
            private_context: false,
            context_epoch: 0,
            title: "Private file discussion".into(),
            state: ThreadState::Open,
            attention: vec![],
        }),
    );
    let mut note = Note::working("Private", "secret attachment phrase");
    note.chat = Some(chat.id.clone());
    note.protected = true;
    let note = owner_record(&mut store, "attachment", RecordData::Note(note));
    // The checked asset boundary requires its owning Note in the group.
    let op = store.allocate_operation("figure").unwrap();
    let owner = HostAuthority::owner("attach figure");
    store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: "Retained figure".into(),
                writes: vec![
                    Put {
                        id: note.id.clone(),
                        expected_version: Some(note.version),
                        archived: false,
                        data: note.data.clone(),
                    },
                    Put {
                        id: op.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Asset(Asset {
                            note: note.id.clone(),
                            source: None,
                            media_type: "image/png".into(),
                            bytes: vec![1, 2, 3],
                        }),
                    },
                ],
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    assert!(matches!(
        store.apply(&op, &owner).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    for id in [&note.id, &op.creation_id(0)] {
        assert!(store.record_visible_in_chat(id, Some(&chat.id)).unwrap());
        assert!(
            !store
                .record_visible_in_chat(id, Some("other-chat"))
                .unwrap()
        );
        assert!(!store.record_visible_in_chat(id, None).unwrap());
    }
    assert!(store.records_for_chat(None).unwrap().is_empty());
    assert!(store.chat_has_private_context(&chat.id).unwrap());
}

#[test]
fn latest_composer_draft_survives_restart_and_delayed_clear_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let data = root.path().canonicalize().unwrap().join("data");
    let mut store = Store::open(&data).unwrap();
    store.save_chat_draft("chat-a", 1, "Earlier text").unwrap();
    store
        .save_chat_draft("chat-a", 2, "Newer õun text")
        .unwrap();
    assert!(store.save_chat_draft("chat-a", 1, "").is_err());
    assert!(store.save_chat_draft("chat-a", 2, "different").is_err());
    assert!(
        store.receipts().unwrap().is_empty(),
        "recovery typing is not permanent keystroke history"
    );
    drop(store);
    let store = Store::open(&data).unwrap();
    assert_eq!(store.chat_drafts().unwrap()[0].text, "Newer õun text");
}

fn legacy_fixture(root: &std::path::Path) -> std::path::PathBuf {
    let data = root.join("legacy");
    let mut store = Store::open(&data).unwrap();
    owner_record(
        &mut store,
        "knowledge",
        RecordData::Note(Note::working("Old", "Preserved bytes")),
    );
    drop(store);
    let conn = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    conn.execute_batch("DROP TABLE chat_drafts; DROP INDEX one_chat_control; DROP INDEX source_locator; CREATE UNIQUE INDEX source_locator ON records(json_extract(body,'$.data.Source.locator')) WHERE json_type(body,'$.data.Source') IS NOT NULL; PRAGMA user_version=3;").unwrap();
    data
}

#[test]
fn adopted_upgrade_requires_external_backup_and_preserves_the_v3_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let data = legacy_fixture(&root.path().canonicalize().unwrap());
    assert!(Store::open(&data).is_err());
    assert!(Store::upgrade(&data, &data).is_err());
    let conn = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    let old: String = conn
        .query_row("SELECT body FROM records", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        conn.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        3
    );
    drop(conn);
    let backups = root.path().canonicalize().unwrap().join("backups");
    std::fs::create_dir(&backups).unwrap();
    Store::upgrade(&data, &backups).unwrap();
    let files = std::fs::read_dir(&backups)
        .unwrap()
        .collect::<std::io::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(files.len(), 1);
    let saved = Connection::open(files[0].path()).unwrap();
    assert_eq!(
        saved
            .pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        3
    );
    assert_eq!(
        saved
            .query_row::<String, _, _>("SELECT body FROM records", [], |r| r.get(0))
            .unwrap(),
        old
    );
    let store = Store::open(&data).unwrap();
    assert_eq!(
        serde_json::to_string(&store.records().unwrap()[0]).unwrap(),
        old
    );
}

#[test]
fn unavailable_backup_folder_leaves_old_format_and_records_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let data = legacy_fixture(&root.path().canonicalize().unwrap());
    assert!(Store::upgrade(&data, &root.path().canonicalize().unwrap().join("missing")).is_err());
    let conn = Connection::open(data.join(DATABASE_FILENAME)).unwrap();
    assert_eq!(
        conn.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        3
    );
    assert_eq!(
        conn.query_row::<i64, _, _>("SELECT count(*) FROM records", [], |r| r.get(0))
            .unwrap(),
        1
    );
}

#[test]
fn a_known_v3_backup_restores_as_v4_without_changing_source_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let legacy = legacy_fixture(&base);
    let source = legacy.join(DATABASE_FILENAME);
    let before = std::fs::read(&source).unwrap();
    let restored = Store::restore(&source, base.join("restored-old")).unwrap();
    assert_eq!(restored.records().unwrap().len(), 1);
    assert_eq!(std::fs::read(&source).unwrap(), before);
    restored.chat_drafts().unwrap();
}

fn with_derived_file(data: &std::path::Path) {
    let derived = data.join(DERIVED_DIRECTORY);
    std::fs::create_dir_all(&derived).unwrap();
    std::fs::write(derived.join("search.sqlite"), b"rebuildable").unwrap();
}

#[test]
fn derived_directory_is_allowed_only_as_a_real_directory() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let data = base.join("data");
    let store = Store::open(&data).unwrap();
    let derived = store.derived_directory().unwrap();
    assert_eq!(derived, data.join(DERIVED_DIRECTORY));
    assert_eq!(store.derived_directory().unwrap(), derived);
    drop(store);
    with_derived_file(&data);
    Store::open(&data).unwrap();

    std::fs::remove_dir_all(&derived).unwrap();
    std::fs::write(&derived, b"not a directory").unwrap();
    assert!(matches!(Store::open(&data), Err(Error::UnsafeDirectory(_))));

    std::fs::remove_file(&derived).unwrap();
    let elsewhere = base.join("elsewhere");
    std::fs::create_dir(&elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &derived).unwrap();
    assert!(matches!(Store::open(&data), Err(Error::UnsafeDirectory(_))));

    std::fs::remove_file(&derived).unwrap();
    let store = Store::open(&data).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &derived).unwrap();
    assert!(matches!(
        store.derived_directory(),
        Err(Error::UnsafeDirectory(_))
    ));
    assert_eq!(std::fs::read_dir(&elsewhere).unwrap().count(), 0);
}

#[test]
fn backup_restore_and_upgrade_ignore_derived_data() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().canonicalize().unwrap();
    let data = base.join("data");
    let mut store = Store::open(&data).unwrap();
    owner_record(
        &mut store,
        "knowledge",
        RecordData::Note(Note::working("Kept", "Kept bytes")),
    );
    with_derived_file(&data);
    let backups = base.join("backups");
    std::fs::create_dir(&backups).unwrap();
    let snapshot = backups.join("snapshot.sqlite3");
    store.backup(&snapshot).unwrap();
    drop(store);
    let restored = Store::restore(&snapshot, base.join("restored")).unwrap();
    assert_eq!(restored.records().unwrap().len(), 1);
    assert!(!base.join("restored").join(DERIVED_DIRECTORY).exists());
    drop(restored);

    let legacy = legacy_fixture(&base);
    with_derived_file(&legacy);
    let upgrade_backups = base.join("upgrade-backups");
    std::fs::create_dir(&upgrade_backups).unwrap();
    Store::upgrade(&legacy, &upgrade_backups).unwrap();
    let store = Store::open(&legacy).unwrap();
    assert_eq!(
        std::fs::read(legacy.join(DERIVED_DIRECTORY).join("search.sqlite")).unwrap(),
        b"rebuildable"
    );
    assert_eq!(store.records().unwrap().len(), 1);
}
