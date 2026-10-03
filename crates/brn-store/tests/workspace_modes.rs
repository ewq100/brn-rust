use brn_store::{
    Error, WorkStore,
    workspace_mode::{WorkspaceMode, classify},
};

#[test]
fn workstore_refuses_legacy_markers_and_dangling_aliases_unchanged() {
    for name in [
        "brn.sqlite3",
        "brn.sqlite3-wal",
        "brn.sqlite3-shm",
        "brn.sqlite3-journal",
    ] {
        let data = tempfile::tempdir().unwrap();
        let marker = data.path().join(name);
        std::fs::write(&marker, b"sentinel").unwrap();
        assert_eq!(classify(data.path()).unwrap(), WorkspaceMode::Legacy);
        assert!(matches!(
            WorkStore::open(data.path()),
            Err(Error::WorkspaceModeConflict(_))
        ));
        assert_eq!(std::fs::read(&marker).unwrap(), b"sentinel");
        assert!(!data.path().join("brn.sqlite").exists());
        assert!(!data.path().join("backups").exists());
        std::fs::remove_file(&marker).unwrap();
        let absent = data.path().join("absent");
        std::os::unix::fs::symlink(&absent, &marker).unwrap();
        assert_eq!(classify(data.path()).unwrap(), WorkspaceMode::Legacy);
        assert!(matches!(
            WorkStore::open(data.path()),
            Err(Error::WorkspaceModeConflict(_))
        ));
        assert_eq!(std::fs::read_link(&marker).unwrap(), absent);
        assert!(!absent.exists());
    }
}

#[test]
fn current_sidecar_and_backup_markers_classify_without_opening_authority() {
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
        assert_eq!(classify(data.path()).unwrap(), WorkspaceMode::Simple);
        assert_eq!(std::fs::read(&marker).unwrap(), b"sentinel");
        assert!(!data.path().join("brn.owner.lock").exists());
        std::fs::remove_file(&marker).unwrap();
        let absent = data.path().join("absent");
        std::os::unix::fs::symlink(&absent, &marker).unwrap();
        assert_eq!(classify(data.path()).unwrap(), WorkspaceMode::Simple);
        assert_eq!(std::fs::read_link(&marker).unwrap(), absent);
        assert!(!absent.exists());
        std::fs::write(data.path().join("brn.sqlite3"), b"legacy").unwrap();
        assert!(matches!(
            classify(data.path()),
            Err(Error::WorkspaceModeConflict(_))
        ));
    }
}

#[test]
fn mixed_markers_are_refused_and_owner_lock_takes_precedence() {
    let data = tempfile::tempdir().unwrap();
    let (store, _) = WorkStore::open(data.path()).unwrap();
    std::fs::write(data.path().join("brn.sqlite3"), b"legacy").unwrap();
    assert!(matches!(
        classify(data.path()),
        Err(Error::WorkspaceModeConflict(_))
    ));
    assert!(matches!(
        WorkStore::open(data.path()),
        Err(Error::WorkspaceBusy(_))
    ));
    drop(store);
    let bytes = std::fs::read(data.path().join("brn.sqlite")).unwrap();
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
