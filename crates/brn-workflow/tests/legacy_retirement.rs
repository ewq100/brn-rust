use brn_store::{OperationStatus, Store};
use brn_workflow::{Config, ErrorKind, SearchProfile, Workspace};
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

#[test]
fn all_legacy_ask_entries_refuse_before_callbacks_or_storage_and_history_is_readable() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let session = store
        .create_session(
            Uuid::new_v4(),
            "codex",
            "historical-store",
            None,
            Some("old-thread"),
            b"{}",
        )
        .unwrap();
    let saved = Uuid::new_v4();
    store
        .prepare_turn(saved, session, "old question", "keyword", "[]")
        .unwrap();
    store.record_turn_started(saved, "old-turn").unwrap();
    store
        .complete_turn(saved, OperationStatus::Completed, "old answer", Some("{}"))
        .unwrap();
    drop(store);
    let mut workspace = Workspace::open(dir.path(), Config::default()).unwrap();
    let before = serde_json::to_value(workspace.history(session).unwrap()).unwrap();
    let cancel = AtomicBool::new(true);
    assert_eq!(
        workspace
            .ask(
                saved,
                Some(session),
                "different",
                SearchProfile::Hybrid,
                &cancel,
                |_| panic!()
            )
            .unwrap_err()
            .kind,
        ErrorKind::LegacyAiRetired
    );
    assert_eq!(
        workspace
            .ask_guarded(
                Uuid::new_v4(),
                Some(session),
                "",
                SearchProfile::Semantic,
                &cancel,
                || panic!(),
                |_| panic!()
            )
            .unwrap_err()
            .kind,
        ErrorKind::LegacyAiRetired
    );
    let fresh = Uuid::new_v4();
    let failure = workspace
        .ask_detailed(
            fresh,
            None,
            "",
            SearchProfile::Keyword,
            &cancel,
            |_| panic!(),
        )
        .unwrap_err();
    assert_eq!(failure.kind, ErrorKind::LegacyAiRetired);
    assert_eq!(failure.operation_id, fresh);
    assert!(failure.recorded_status.is_none());
    assert!(failure.receipt.is_none());
    assert_eq!(workspace.sessions().unwrap().len(), 1);
    let after = serde_json::to_value(workspace.history(session).unwrap()).unwrap();
    assert_eq!(before, after);
    assert_eq!(after[0]["provider_turn_id"], "old-turn");
    drop(workspace);
    let (store, _) = Store::open(dir.path()).unwrap();
    assert!(store.operation(fresh).unwrap().is_none());
    assert_eq!(
        store
            .session(session)
            .unwrap()
            .unwrap()
            .thread_id
            .as_deref(),
        Some("old-thread")
    );
}

#[test]
fn headless_retired_ask_and_unknown_executable_flag_do_not_initialize_storage() {
    let dir = tempfile::tempdir().unwrap();
    for (args, message) in [
        (vec!["ask", "--query", "question"], "LEGACY_AI_RETIRED"),
        (vec!["sources", "--codex", "/nonexistent"], "unknown option"),
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_brn-flow"))
            .args(args)
            .arg("--data-dir")
            .arg(dir.path())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(message));
        assert!(!dir.path().join("workspace.sqlite3").exists());
        assert!(!dir.path().join("brn.sqlite").exists());
    }
}
