use serde_json::Value;
use std::process::Command;
mod support;

fn run(data: &std::path::Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .args(["--json", "--data-dir"])
        .arg(data)
        .output()
        .unwrap();
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[test]
fn fresh_status_and_history_use_current_operational_storage_without_vault() {
    let data = support::data_dir();
    let (code, status) = run(data.path(), &["status"]);
    assert_eq!(code, 0, "{status}");
    assert!(status["data"]["vault_root"].is_null());
    let (code, history) = run(data.path(), &["conversations", "list"]);
    assert_eq!(code, 0, "{history}");
    assert_eq!(history["data"]["conversations"], serde_json::json!([]));
    let (code, note) = run(data.path(), &["notes", "show", "plan.md"]);
    assert_eq!(code, 1);
    assert_eq!(note["error"]["code"], "VAULT_NOT_BOUND");
    assert!(data.path().join("brn.sqlite").exists());
    assert!(!data.path().join("brn.sqlite3").exists());
}

#[test]
fn session_timestamp_json_preserves_completion_and_interruption_times_across_reads() {
    use brn_store::work::{WorkStore, WorkTurnStatus};
    let data = support::data_dir();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let first = store
        .begin_turn(
            uuid::Uuid::new_v4(),
            None,
            "Exact question õ\r\n",
            "chatgpt",
            "synthetic-model",
        )
        .unwrap();
    let completed = store
        .finish_turn(
            first.id,
            WorkTurnStatus::Completed,
            "Exact answer λ\r\n",
            None,
        )
        .unwrap();
    let running = store
        .begin_turn(
            uuid::Uuid::new_v4(),
            Some(first.conversation_id),
            "Second question\r\n",
            "copilot",
            "synthetic-model",
        )
        .unwrap();
    let conversation = serde_json::to_value(store.conversations().unwrap().remove(0)).unwrap();
    let started = running.started_at_ms.unwrap();
    drop(store);
    let (code, listed) = run(data.path(), &["conversations", "list"]);
    assert_eq!(code, 0, "{listed}");
    assert_eq!(listed["data"]["conversations"][0], conversation);
    let (code, shown) = run(
        data.path(),
        &["conversations", "show", &first.conversation_id.to_string()],
    );
    assert_eq!(code, 0, "{shown}");
    assert_eq!(
        shown["data"]["turns"][0]["operation_id"],
        completed.id.to_string()
    );
    assert_eq!(
        shown["data"]["turns"][0]["session_id"],
        completed.conversation_id.to_string()
    );
    assert_eq!(shown["data"]["turns"][0]["question"], completed.question);
    assert_eq!(shown["data"]["turns"][0]["answer"], completed.answer);
    assert_eq!(
        shown["data"]["turns"][0]["started_at_ms"],
        completed.started_at_ms.unwrap()
    );
    assert_eq!(
        shown["data"]["turns"][0]["finished_at_ms"],
        completed.finished_at_ms.unwrap()
    );
    assert_eq!(shown["data"]["turns"][1]["status"], "interrupted");
    assert_eq!(shown["data"]["turns"][1]["started_at_ms"], started);
    assert!(shown["data"]["turns"][1]["finished_at_ms"].is_null());
    assert!(shown["data"]["turns"][1]
        .as_object()
        .unwrap()
        .contains_key("finished_at_ms"));
    let (code, listed_again) = run(data.path(), &["conversations", "list"]);
    assert_eq!(code, 0, "{listed_again}");
    assert_eq!(listed_again["data"], listed["data"]);
    let (code, shown_again) = run(
        data.path(),
        &["conversations", "show", &first.conversation_id.to_string()],
    );
    assert_eq!(code, 0, "{shown_again}");
    assert_eq!(shown_again["data"], shown["data"]);
}

#[test]
fn markdown_library_preserves_bytes_and_keyword_only_search_shape() {
    let data = support::data_dir();
    let vault = tempfile::tempdir().unwrap();
    std::fs::write(
        vault.path().join("plan.md"),
        "# Aurora\r\nSynthetic launch Tuesday.\r\n",
    )
    .unwrap();
    let root = vault.path().to_str().unwrap();
    let (code, page) = run(data.path(), &["notes", "list", "--vault", root]);
    assert_eq!(code, 0, "{page}");
    assert_eq!(page["data"]["notes"][0]["path"], "plan.md");
    let (code, note) = run(data.path(), &["notes", "show", "plan.md"]);
    assert_eq!(code, 0, "{note}");
    assert_eq!(
        note["data"]["text"],
        "# Aurora\r\nSynthetic launch Tuesday.\r\n"
    );
    let (code, search) = run(data.path(), &["search", "launch"]);
    assert_eq!(code, 0, "{search}");
    assert_eq!(search["data"]["keyword_only"], true);
    assert_eq!(search["data"]["hits"][0]["path"], "plan.md");
    let (code, ask) = run(data.path(), &["ask", "q"]);
    assert_eq!(code, 1);
    assert_eq!(ask["error"]["code"], "AI_SELECTION_REQUIRED");
}

#[test]
fn markers_and_backup_sidecars_route_without_creating_other_authority() {
    for marker in ["brn.sqlite3-wal", "brn.sqlite3-shm", "brn.sqlite3-journal"] {
        let data = support::data_dir();
        std::fs::write(data.path().join(marker), []).unwrap();
        let (code, value) = run(data.path(), &["notes", "list"]);
        assert_eq!(code, 1);
        assert_eq!(value["error"]["code"], "WORKSPACE_MODE_CONFLICT");
        assert!(!data.path().join("brn.sqlite").exists());
        let (code, value) = run(data.path(), &["ask", "q"]);
        assert_eq!(code, 1);
        assert_eq!(value["error"]["code"], "WORKSPACE_MODE_CONFLICT");
    }
}

#[test]
fn recognized_valid_backup_restores_simple_history_not_legacy_authority() {
    use brn_workflow::app::{App, AppConfig};
    let data = support::data_dir();
    let config = || AppConfig {
        vault_root: None,
        credentials_dir: None,
        model_dir: None,
    };
    let mut app = App::open(data.path(), config()).unwrap();
    let turn = app
        .work_store_mut()
        .begin_turn(uuid::Uuid::new_v4(), None, "q", "chatgpt", "gpt-5.5")
        .unwrap();
    app.work_store_mut()
        .finish_turn(
            turn.id,
            brn_workflow::WorkTurnStatus::Completed,
            "durable",
            None,
        )
        .unwrap();
    drop(app);
    drop(App::open(data.path(), config()).unwrap());
    std::fs::remove_file(data.path().join("brn.sqlite")).unwrap();
    let (code, history) = run(
        data.path(),
        &["conversations", "show", &turn.conversation_id.to_string()],
    );
    assert_eq!(code, 0, "{history}");
    assert_eq!(history["data"]["turns"][0]["answer"], "durable");
    assert!(!data.path().join("brn.sqlite3").exists());
}

#[test]
fn mixed_markers_refuse_current_startup_without_modification() {
    for marker in [
        "brn.sqlite-wal",
        "brn.sqlite-shm",
        "brn.sqlite-journal",
        "backups/brn-123.sqlite-wal",
    ] {
        let data = support::data_dir();
        std::fs::create_dir_all(data.path().join("backups")).unwrap();
        std::fs::write(data.path().join(marker), b"current marker").unwrap();
        let legacy = data.path().join("brn.sqlite3-journal");
        std::fs::write(&legacy, b"legacy marker").unwrap();
        let (code, value) = run(data.path(), &["status"]);
        assert_eq!(code, 1);
        assert_eq!(value["error"]["code"], "WORKSPACE_MODE_CONFLICT");
        assert_eq!(std::fs::read(&legacy).unwrap(), b"legacy marker");
        assert_eq!(
            std::fs::read(data.path().join(marker)).unwrap(),
            b"current marker"
        );
        assert!(!data.path().join("brn.sqlite").exists());
    }
}
