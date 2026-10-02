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
