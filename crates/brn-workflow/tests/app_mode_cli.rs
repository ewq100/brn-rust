#[test]
fn legacy_brn_flow_sessions_refuses_simple_folder_without_modifying_work() {
    let data = tempfile::tempdir().unwrap();
    let (store, _) = brn_store::WorkStore::open(data.path()).unwrap();
    drop(store);
    let before = std::fs::read(data.path().join("brn.sqlite")).unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_brn-flow"))
        .args(["sessions", "--data-dir"])
        .arg(data.path())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("workspace mode")
    );
    assert!(!data.path().join("brn.sqlite3").exists());
    assert_eq!(
        std::fs::read(data.path().join("brn.sqlite")).unwrap(),
        before
    );
}
