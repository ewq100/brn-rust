use brn_store::{Error, WorkStore};

#[test]
fn settings_survive_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, None);
    assert_eq!(report.corrupt_moved_to, None);
    store.set_setting("vault", "/synthetic/vault").unwrap();
    store.set_setting("vault", "/synthetic/vault2").unwrap();
    store.set_setting("model", "gpt").unwrap();
    store.remove_setting("model").unwrap();
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.setting("vault").unwrap().as_deref(),
        Some("/synthetic/vault2")
    );
    assert_eq!(store.setting("model").unwrap(), None);
    assert_eq!(store.data_dir(), dir.path());
}

#[test]
fn second_owner_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (_store, _) = WorkStore::open(dir.path()).unwrap();
    assert!(matches!(
        WorkStore::open(dir.path()),
        Err(Error::WorkspaceBusy(_))
    ));
}

#[test]
fn missing_data_dir_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        WorkStore::open(&dir.path().join("absent")),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn newer_schema_is_refused_and_left_in_place() {
    let dir = tempfile::tempdir().unwrap();
    drop(WorkStore::open(dir.path()).unwrap());
    let raw = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    raw.pragma_update(None, "user_version", 99).unwrap();
    drop(raw);
    assert!(matches!(
        WorkStore::open(dir.path()),
        Err(Error::Invalid(_))
    ));
    assert!(dir.path().join("brn.sqlite").exists());
    let moved = std::fs::read_dir(dir.path())
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .contains("corrupt")
        })
        .count();
    assert_eq!(moved, 0);
}

#[test]
fn foreign_database_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let raw = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    raw.pragma_update(None, "application_id", 12345).unwrap();
    drop(raw);
    assert!(matches!(
        WorkStore::open(dir.path()),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn unbranded_database_is_refused_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let raw = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    raw.execute_batch("CREATE TABLE other (x INTEGER)").unwrap();
    drop(raw);
    assert!(matches!(
        WorkStore::open(dir.path()),
        Err(Error::Invalid(_))
    ));
    let raw = rusqlite::Connection::open(dir.path().join("brn.sqlite")).unwrap();
    let tables: Vec<String> = raw
        .prepare("SELECT name FROM sqlite_schema WHERE type = 'table'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tables, vec!["other".to_string()]);
}

fn backups(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found: Vec<_> = std::fs::read_dir(dir.join("backups"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with("brn-") && name.ends_with(".sqlite")
        })
        .collect();
    found.sort();
    found
}

fn corrupt(dir: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(dir.join(format!("brn.sqlite{suffix}")));
    }
    std::fs::write(dir.join("brn.sqlite"), vec![0x42u8; 8192]).unwrap();
}

#[test]
fn each_open_backs_up_and_keeps_five() {
    let dir = tempfile::tempdir().unwrap();
    let mut made = Vec::new();
    for _ in 0..7 {
        let (_store, report) = WorkStore::open(dir.path()).unwrap();
        assert!(report.backup.exists());
        made.push(report.backup);
    }
    assert_eq!(backups(dir.path()), made[2..].to_vec());
}

#[test]
fn corrupt_database_is_restored_from_newest_backup() {
    let dir = tempfile::tempdir().unwrap();
    {
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("k", "first").unwrap();
    }
    {
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("k", "second").unwrap();
    }
    let newest = WorkStore::open(dir.path()).unwrap().1.backup;
    corrupt(dir.path());
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.setting("k").unwrap().as_deref(), Some("second"));
    assert_eq!(report.restored_from, Some(newest));
    let moved = report.corrupt_moved_to.unwrap();
    assert!(moved.exists());
    assert!(
        moved
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("brn.sqlite.corrupt-")
    );
}

#[test]
fn corrupt_database_without_backups_starts_fresh() {
    let dir = tempfile::tempdir().unwrap();
    corrupt(dir.path());
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, None);
    assert!(report.corrupt_moved_to.unwrap().exists());
    assert_eq!(store.setting("k").unwrap(), None);
}

#[test]
fn unusable_newest_backup_falls_back_to_older_one() {
    for damage in [vec![0x42u8; 8192], Vec::new()] {
        let dir = tempfile::tempdir().unwrap();
        {
            let (mut store, _) = WorkStore::open(dir.path()).unwrap();
            store.set_setting("k", "kept").unwrap();
        }
        let older = WorkStore::open(dir.path()).unwrap().1.backup;
        let newest = WorkStore::open(dir.path()).unwrap().1.backup;
        std::fs::write(&newest, &damage).unwrap();
        corrupt(dir.path());
        let (store, report) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(
            report.restored_from,
            Some(older),
            "damage of {} bytes",
            damage.len()
        );
        assert_eq!(store.setting("k").unwrap().as_deref(), Some("kept"));
    }
}

#[test]
fn missing_database_is_restored_from_newest_backup() {
    let dir = tempfile::tempdir().unwrap();
    {
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        store.set_setting("k", "kept").unwrap();
    }
    let newest = WorkStore::open(dir.path()).unwrap().1.backup;
    for suffix in ["", "-wal", "-shm"] {
        let _ = std::fs::remove_file(dir.path().join(format!("brn.sqlite{suffix}")));
    }
    let (store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, Some(newest));
    assert_eq!(report.corrupt_moved_to, None);
    assert_eq!(store.setting("k").unwrap().as_deref(), Some("kept"));
}

#[test]
fn new_backup_sorts_after_existing_ones_even_with_future_names() {
    let dir = tempfile::tempdir().unwrap();
    let first = WorkStore::open(dir.path()).unwrap().1.backup;
    let future = dir.path().join("backups/brn-9000000000000.sqlite");
    std::fs::copy(&first, &future).unwrap();
    let (_store, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        report.backup.file_name().unwrap().to_string_lossy(),
        "brn-9000000000001.sqlite"
    );
    assert!(report.backup.exists());
    assert_eq!(backups(dir.path()).last(), Some(&report.backup));
}

#[test]
fn unsaved_edits_round_trip_replace_and_clear() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    store
        .put_unsaved_edit("notes/a.md", [7; 32], "draft é\r\n")
        .unwrap();
    store.put_unsaved_edit("b.md", [1; 32], "").unwrap();
    let edit = store.unsaved_edit("notes/a.md").unwrap().unwrap();
    assert_eq!(edit.path, "notes/a.md");
    assert_eq!(edit.base_sha256, [7; 32]);
    assert_eq!(edit.text, "draft é\r\n");
    assert!(edit.updated_at_ms > 0);
    store
        .put_unsaved_edit("notes/a.md", [8; 32], "newer")
        .unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let all = store.unsaved_edits().unwrap();
    assert_eq!(
        all.iter()
            .map(|e| (e.path.as_str(), e.text.as_str()))
            .collect::<Vec<_>>(),
        vec![("b.md", ""), ("notes/a.md", "newer")]
    );
    store.clear_unsaved_edit("notes/a.md").unwrap();
    assert_eq!(store.unsaved_edit("notes/a.md").unwrap(), None);
}

#[test]
fn unsaved_edit_limits() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let at_limit = "a".repeat(brn_store::MAX_NOTE_BYTES);
    store.put_unsaved_edit("a.md", [0; 32], &at_limit).unwrap();
    let over = "a".repeat(brn_store::MAX_NOTE_BYTES + 1);
    assert!(matches!(
        store.put_unsaved_edit("a.md", [0; 32], &over),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        store.put_unsaved_edit("", [0; 32], "x"),
        Err(Error::Invalid(_))
    ));
}
