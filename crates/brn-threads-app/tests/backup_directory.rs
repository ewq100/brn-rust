use brn_threads_app::*;

#[test]
fn saved_folder_persists_across_reopen_and_backup_restore_and_each_backup_is_fresh() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("workspace");
    let folder = root.path().join("backups");
    std::fs::create_dir(&folder).unwrap();
    let mut app = Workspace::open(&data).unwrap();
    let mut settings = app.settings().unwrap();
    settings.auth_file = None;
    settings.credentials_dir = None;
    settings.provider = "offline-fixture".into();
    app.configure(settings).unwrap();
    let note = app.new_note("Synthetic note", "Saved content").unwrap();
    app.choose_backup_directory(&folder).unwrap();
    drop(app);
    let app = Workspace::open(&data).unwrap();
    assert_eq!(
        app.settings().unwrap().backup_directory.as_deref(),
        folder.to_str()
    );
    let first = app.backup_to_saved_directory().unwrap();
    let second = app.backup_to_saved_directory().unwrap();
    assert_ne!(first, second);
    assert!(first.is_file() && second.is_file());
    assert_eq!(first.parent(), Some(folder.as_path()));
    let restored = root.path().join("restored");
    Store::restore(&first, &restored).unwrap();
    let restored = Workspace::open(&restored).unwrap();
    assert_eq!(restored.settings().unwrap(), app.settings().unwrap());
    assert_eq!(
        restored.store.note(&note.id).unwrap(),
        app.store.note(&note.id).unwrap()
    );
    let third = restored.backup_to_saved_directory().unwrap();
    assert_ne!(third, first);
    assert!(third.is_file());
    std::fs::remove_file(first).unwrap();
    std::fs::remove_file(second).unwrap();
    std::fs::remove_file(third).unwrap();
    std::fs::remove_dir(&folder).unwrap();
    assert!(app.backup_to_saved_directory().is_err());
    assert!(!folder.exists(), "missing folder must not be recreated");
    assert_eq!(
        Workspace::open(&data)
            .unwrap()
            .settings()
            .unwrap()
            .backup_directory
            .as_deref(),
        folder.to_str()
    );
}

#[test]
fn saved_destination_revalidated_against_restored_workspace_and_alias() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let folder = root.path().join("future-workspace");
    std::fs::create_dir(&folder).unwrap();
    let mut app = Workspace::open(root.path().join("workspace")).unwrap();
    app.choose_backup_directory(&folder).unwrap();
    let backup = root.path().join("synthetic-backup");
    app.backup(&backup).unwrap();
    std::fs::remove_dir(&folder).unwrap();
    Store::restore(&backup, &folder).unwrap();
    let restored = Workspace::open(&folder).unwrap();
    assert!(restored.backup_to_saved_directory().is_err());
    assert!(std::fs::read_dir(&folder).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("BRN-backup-")
    }));
    #[cfg(unix)]
    {
        let alias = root.path().join("retargetable-folder");
        let external = root.path().join("external");
        std::fs::create_dir(&external).unwrap();
        std::os::unix::fs::symlink(&external, &alias).unwrap();
        // Persisted paths normally normalize aliases; a restored preference may come from an older host.
        let op = app
            .store
            .allocate_operation("synthetic alias preference")
            .unwrap();
        let record = app
            .records()
            .unwrap()
            .into_iter()
            .find(|r| matches!(r.data, RecordData::Settings(_)))
            .unwrap();
        let RecordData::Settings(mut settings) = record.data else {
            panic!("settings")
        };
        settings.backup_directory = Some(alias.to_str().unwrap().into());
        app.owner_change(
            "synthetic alias preference",
            &ChangeRequest {
                reason: "Synthetic restored host path".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: record.id,
                    expected_version: Some(record.version),
                    archived: false,
                    data: RecordData::Settings(settings),
                }],
            },
        )
        .unwrap();
        assert!(app.store.receipt(&op).unwrap().is_some());
        std::fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink(app.store.directory(), &alias).unwrap();
        assert!(app.backup_to_saved_directory().is_err());
    }
}

#[test]
fn explicit_app_backup_refuses_storage_and_saved_parent_folder_remains_supported() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("workspace");
    let mut app = Workspace::open(&data).unwrap();
    let records = app.records().unwrap();
    let inside = data.join("backup.sqlite3");
    assert!(app.backup(&inside).is_err());
    assert!(!inside.exists());
    assert_eq!(Workspace::open(&data).unwrap().records().unwrap(), records);
    app.choose_backup_directory(root.path()).unwrap();
    let backup = app.backup_to_saved_directory().unwrap();
    assert_eq!(backup.parent(), Some(root.path()));
    assert!(backup.is_file());
}

#[test]
fn restored_saved_descendant_is_refused_at_dispatch() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let future = root.path().join("future-workspace");
    let folder = future.join("child");
    std::fs::create_dir_all(&folder).unwrap();
    let mut app = Workspace::open(root.path().join("workspace")).unwrap();
    app.choose_backup_directory(&folder).unwrap();
    let backup = root.path().join("backup");
    app.backup(&backup).unwrap();
    std::fs::remove_dir_all(&future).unwrap();
    Store::restore(&backup, &future).unwrap();
    let restored = Workspace::open(&future).unwrap();
    std::fs::create_dir(&folder).unwrap();
    assert!(restored.backup_to_saved_directory().is_err());
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
    std::fs::remove_dir(&folder).unwrap();
    assert_eq!(
        Workspace::open(&future).unwrap().settings().unwrap(),
        app.settings().unwrap()
    );
}
