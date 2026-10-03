//! Pipe delivery checks with local data and durable terminal replay; never a live model.
use brn_workflow::{
    app::{App, AppConfig},
    WorkTurnStatus,
};
use serde_json::Value;
use std::{
    io::Read,
    process::{Command, Stdio},
};
use uuid::Uuid;
mod support;

#[test]
fn closed_stdout_consumer_is_quiet_success() {
    let dir = support::data_dir();
    for args in [vec!["status", "--legacy"], vec!["--help"]] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(args)
            .arg("--data-dir")
            .arg(dir.path())
            .arg("--json")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        drop(child.stdout.take());
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!String::from_utf8_lossy(&out.stderr).contains("panic"));
    }
}

#[test]
fn closed_stderr_keeps_completed_replay_and_one_stdout_envelope() {
    let dir = support::data_dir();
    let mut app = App::open(
        dir.path(),
        AppConfig {
            vault_root: None,
            credentials_dir: None,
            model_dir: None,
        },
    )
    .unwrap();
    let id = Uuid::new_v4();
    app.work_store_mut()
        .begin_turn(id, None, "q", "chatgpt", "gpt-5.5")
        .unwrap();
    app.work_store_mut()
        .finish_turn(id, WorkTurnStatus::Completed, "saved answer", None)
        .unwrap();
    drop(app);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args([
            "ask",
            "q",
            "--operation",
            &id.to_string(),
            "--json",
            "--data-dir",
        ])
        .arg(dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stderr.take());
    let mut text = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(child.wait().unwrap().code(), Some(0), "{text}");
    let envelope: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(envelope["data"]["status"], "completed");
}

#[test]
fn retired_executable_flag_is_unknown_before_opening_a_workspace() {
    let dir = support::data_dir();
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args([
            "ask",
            "q",
            "--codex",
            "/nonexistent",
            "--json",
            "--data-dir",
        ])
        .arg(dir.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["error"]["code"], "USAGE");
    assert!(envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains("unknown"));
    assert!(!dir.path().join("workspace.sqlite3").exists());
    assert!(!dir.path().join("brn.sqlite").exists());
}
