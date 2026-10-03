use brn_store::{Error, Store, WorkStore};

#[test]
fn both_owners_refuse_every_opposite_mode_marker_unchanged() {
    for name in [
        "brn.sqlite3",
        "brn.sqlite3-wal",
        "brn.sqlite3-shm",
        "brn.sqlite3-journal",
    ] {
        let data = tempfile::tempdir().unwrap();
        let marker = data.path().join(name);
        std::fs::write(&marker, b"sentinel").unwrap();
        assert!(matches!(
            WorkStore::open(data.path()),
            Err(Error::WorkspaceModeConflict(_))
        ));
        assert_eq!(std::fs::read(&marker).unwrap(), b"sentinel");
        assert!(!data.path().join("brn.sqlite").exists());
        std::fs::remove_file(&marker).unwrap();
        let absent = data.path().join("absent");
        std::os::unix::fs::symlink(&absent, &marker).unwrap();
        assert!(matches!(
            WorkStore::open(data.path()),
            Err(Error::WorkspaceModeConflict(_))
        ));
        assert_eq!(std::fs::read_link(&marker).unwrap(), absent);
        assert!(!absent.exists());
    }
    for name in [
        "brn.sqlite",
        "brn.sqlite-wal",
        "brn.sqlite-shm",
        "brn.sqlite-journal",
        "backups/brn-123.sqlite",
        "backups/brn-123.sqlite-wal",
        "backups/brn-123.sqlite-shm",
        "backups/brn-123.sqlite-journal",
    ] {
        let data = tempfile::tempdir().unwrap();
        let marker = data.path().join(name);
        std::fs::create_dir_all(marker.parent().unwrap()).unwrap();
        std::fs::write(&marker, b"sentinel").unwrap();
        assert!(matches!(
            Store::open(data.path()),
            Err(Error::WorkspaceModeConflict(_))
        ));
        assert_eq!(std::fs::read(&marker).unwrap(), b"sentinel");
        assert!(!data.path().join("brn.sqlite3").exists());
        std::fs::remove_file(&marker).unwrap();
        let absent = data.path().join("absent");
        std::os::unix::fs::symlink(&absent, &marker).unwrap();
        assert!(matches!(
            Store::open(data.path()),
            Err(Error::WorkspaceModeConflict(_))
        ));
        assert_eq!(std::fs::read_link(&marker).unwrap(), absent);
        assert!(!absent.exists());
    }
}

#[test]
fn both_authorities_are_refused_and_shared_lock_takes_precedence() {
    let data = tempfile::tempdir().unwrap();
    let (store, _) = WorkStore::open(data.path()).unwrap();
    std::fs::write(data.path().join("brn.sqlite3"), b"legacy").unwrap();
    assert!(matches!(
        Store::open(data.path()),
        Err(Error::WorkspaceBusy(_))
    ));
    assert!(matches!(
        WorkStore::open(data.path()),
        Err(Error::WorkspaceBusy(_))
    ));
    drop(store);
    let bytes = std::fs::read(data.path().join("brn.sqlite")).unwrap();
    assert!(matches!(
        Store::open(data.path()),
        Err(Error::WorkspaceModeConflict(_))
    ));
    assert!(matches!(
        WorkStore::open(data.path()),
        Err(Error::WorkspaceModeConflict(_))
    ));
    assert_eq!(
        std::fs::read(data.path().join("brn.sqlite")).unwrap(),
        bytes
    );
    assert_eq!(
        std::fs::read(data.path().join("brn.sqlite3")).unwrap(),
        b"legacy"
    );
}
