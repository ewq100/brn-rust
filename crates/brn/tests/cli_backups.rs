//! Real-process checkpoint inspection, strict preflight and durable state.
use serde_json::Value;
use std::{path::Path, process::Command};
mod support;

fn run(data: &Path, args: &[&str]) -> (i32, Value) {
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
fn checkpoint_and_status_are_local_and_preserve_work_after_restart() {
    let data = support::data_dir();
    let (mut store, _) = brn_store::WorkStore::open(data.path()).unwrap();
    store
        .put_unsaved_edit("pending.md", [9; 32], "Saved recovery õ\r\n")
        .unwrap();
    drop(store);
    let (code, status) = run(data.path(), &["backups", "status"]);
    assert_eq!(code, 0, "{status}");
    assert!(Path::new(status["data"]["latest_path"].as_str().unwrap()).exists());
    let (code, checkpoint) = run(data.path(), &["backups", "checkpoint"]);
    assert_eq!(code, 0, "{checkpoint}");
    assert!(checkpoint["data"]["last_error"].is_null());
    let (store, _) = brn_store::WorkStore::open(data.path()).unwrap();
    assert_eq!(
        store.unsaved_edit("pending.md").unwrap().unwrap().text,
        "Saved recovery õ\r\n"
    );
    assert!(store.conversations().unwrap().is_empty());
}

#[test]
fn malformed_backup_commands_refuse_before_opening_state() {
    let data = support::data_dir();
    for args in [
        vec!["backups"],
        vec!["backups", "restore"],
        vec!["backups", "status", "extra"],
        vec!["backups", "checkpoint", "--force"],
    ] {
        let (code, failure) = run(data.path(), &args);
        assert_eq!(code, 2, "{failure}");
        assert!(!data.path().join("brn.sqlite").exists());
    }
}
