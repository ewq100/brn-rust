use brn_threads_app::*;
#[test]
fn owner_package_reopen_and_real_backup_export_retention_exclusion() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let source = root.path().join("package");
    std::fs::create_dir(&source).unwrap();
    let sentinel = "instructiononlycanaryd3a";
    std::fs::create_dir(source.join("references")).unwrap();
    let example = include_str!("../../../agent/skill-examples/document-review/SKILL.md");
    std::fs::write(source.join("SKILL.md"), format!("{example}\n{sentinel}\n")).unwrap();
    std::fs::write(
        source.join("references/checklist.md"),
        include_str!("../../../agent/skill-examples/document-review/references/checklist.md"),
    )
    .unwrap();
    let mut app =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    let note = app
        .new_note("canonicalpositivecanaryd3a", "Retained canonical work")
        .unwrap();
    let records = app.records().unwrap();
    let history = app.history().unwrap();
    let info = app.install_skill(&source).unwrap();
    assert_eq!(info.file_count, 2);
    assert!(!info.enabled);
    assert!(!info.included_in_backup);
    assert!(!info.included_in_export);
    assert!(info.retention_notice.contains("Preserve original"));
    assert_eq!(app.records().unwrap(), records);
    assert_eq!(app.history().unwrap(), history);
    drop(app);
    // Remove the caller source only in this synthetic fixture: installed bytes
    // must survive, rather than being an on-demand external-folder lookup.
    std::fs::remove_dir_all(&source).unwrap();
    let mut app = Workspace::open_existing_no_upgrade(&data).unwrap();
    assert_eq!(app.inspect_skill("document-review").unwrap().info, info);
    assert!(app.inspect_skill("document-review").unwrap().files["SKILL.md"].contains(sentinel));
    assert!(app.search(sentinel).unwrap().is_empty());
    assert!(!app.search("canonicalpositivecanaryd3a").unwrap().is_empty());
    let export = root.path().join("export");
    let manifest = app.export_now(&export).unwrap();
    assert!(manifest.files.iter().any(|f| f.record == note.id));
    for f in &manifest.files {
        assert!(
            !std::fs::read_to_string(export.join(&f.path))
                .unwrap()
                .contains(sentinel)
        );
    }
    assert!(!serde_json::to_string(&manifest).unwrap().contains(sentinel));
    let backup = root.path().join("backup.sqlite3");
    app.store.backup(&backup).unwrap();
    let restored = root.path().join("restored");
    drop(Store::restore(&backup, &restored).unwrap());
    let mut restored = Workspace::open_existing_no_upgrade(&restored).unwrap();
    assert_eq!(
        restored.store.record(&note.id).unwrap(),
        app.store.record(&note.id).unwrap()
    );
    assert_eq!(restored.skills().unwrap().len(), 4);
    assert!(restored.inspect_skill("document-review").is_err());
    assert!(
        !restored
            .search("canonicalpositivecanaryd3a")
            .unwrap()
            .is_empty()
    );
    assert!(restored.search(sentinel).unwrap().is_empty());
    assert_eq!(app.inspect_skill("document-review").unwrap().info, info);
}
