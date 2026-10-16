//! Round 3, schema upgrade option B: the CLI never upgrades a workspace as a
//! side effect. Any command on an older store, reads included, is refused
//! with `upgrade_required` before anything in the folder changes; only
//! `upgrade --backup-directory` upgrades, backup-first.
//!
//! The schema-5 fixtures are built by a `fb1b2100` `brn`, named by
//! `AGENT_CONTRACT_SCHEMA5_BIN`, through documented commands only (`init`,
//! `note-create`, `begin-edit`): no test writes SQLite. Without the variable
//! these tests print a skip line; the core tests cover the upgrade logic.
#![cfg(unix)]
mod support;
use serde_json::Value;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use support::*;

fn schema5() -> Option<PathBuf> {
    let binary = schema5_bin();
    if binary.is_none() {
        eprintln!("skipped: AGENT_CONTRACT_SCHEMA5_BIN unset");
    }
    binary
}

/// Runs the schema-5 binary with the root's clean environment.
fn old(root: &Root, binary: &Path, json: bool, data: &Path, rest: &[&str]) -> Run {
    let argv = argv(json, data, rest);
    let args: Vec<&OsStr> = argv.iter().map(OsStr::new).collect();
    let child = root
        .command_with(binary, &args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_output(child, Duration::from_secs(60))
}
fn old_ok(root: &Root, binary: &Path, data: &Path, rest: &[&str]) -> Value {
    let run = old(root, binary, false, data, rest);
    assert_eq!(run.code, 0, "{rest:?}: {}", run.stderr);
    serde_json::from_str(&run.stdout).unwrap()
}

/// A schema-5 workspace with one note, made by the schema-5 binary.
fn fixture(root: &Root, binary: &Path, name: &str) -> (PathBuf, String) {
    let data = root.join(name);
    old_ok(root, binary, &data, &["init"]);
    let note = old_ok(
        root,
        binary,
        &data,
        &["note-create", "--title", "Old", "--text", "schema five"],
    );
    assert_eq!(user_version(&data), 5);
    (data, id(&note))
}

/// SQLite's `user_version`: big-endian bytes 60..64 of the database header,
/// read from the file without opening it as a database.
fn user_version(data: &Path) -> u32 {
    let bytes = std::fs::read(data.join("threads.sqlite3")).unwrap();
    u32::from_be_bytes(bytes[60..64].try_into().unwrap())
}

/// Every file under `dir` with its bytes, sorted by path.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = vec![];
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                out.push((path.clone(), vec![]));
                stack.push(path);
            } else {
                out.push((path.clone(), std::fs::read(&path).unwrap()));
            }
        }
    }
    out.sort();
    out
}

fn assert_upgrade_required(envelope: &Value) {
    assert_error(envelope, "rejected", "upgrade_required");
    assert_eq!(envelope["exit"], 6);
    assert_eq!(envelope["retry"], "refuse");
    assert_eq!(
        envelope["error"]["details"],
        serde_json::json!({"schema_found": 5, "schema_required": 6})
    );
    let message = envelope["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("brn upgrade --backup-directory ABSOLUTE_DIR"),
        "{message}"
    );
}

#[test]
fn a_read_on_a_schema_5_store_is_refused_and_leaves_it_byte_unchanged() {
    let Some(binary) = schema5() else { return };
    let root = Root::new();
    let (data, _note) = fixture(&root, &binary, "data");
    let before = snapshot(&data);
    let read = call(&root, &data, &["notes"]);
    assert_upgrade_required(&read);
    assert_eq!(read["authority"]["workspace_purpose"], Value::Null);
    assert_eq!(snapshot(&data), before);
    let paged = call(&root, &data, &["history", "--limit", "5"]);
    assert_upgrade_required(&paged);
    let default = run(&root, false, &data, &["notes"]);
    assert_eq!(default.code, 1);
    assert_eq!(default.stdout, "");
    assert_eq!(default.stderr.lines().count(), 1, "{}", default.stderr);
    assert!(default
        .stderr
        .contains("brn upgrade --backup-directory ABSOLUTE_DIR"));
    assert_eq!(snapshot(&data), before);
    assert_eq!(user_version(&data), 5);
}

#[test]
fn a_write_on_a_schema_5_store_is_refused_and_changes_nothing() {
    let Some(binary) = schema5() else { return };
    let root = Root::new();
    let (data, _note) = fixture(&root, &binary, "data");
    let before = snapshot(&data);
    for argv in [
        &["note-create", "--title", "New", "--text", "refused"][..],
        &["init"][..],
        &["init", "--purpose", "test"][..],
    ] {
        let envelope = call(&root, &data, argv);
        if argv == ["init", "--purpose", "test"] {
            // Creation only: an existing store is never a new folder.
            assert_error(&envelope, "rejected", "invalid_request");
            assert_eq!(envelope["error"]["details"]["reason"], "workspace_exists");
        } else {
            assert_upgrade_required(&envelope);
        }
        assert_eq!(snapshot(&data), before, "{argv:?}");
    }
    // The schema-5 binary still reads its own store.
    assert_eq!(
        old_ok(&root, &binary, &data, &["notes"])
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn upgrade_is_refused_while_a_schema_5_edit_is_open() {
    let Some(binary) = schema5() else { return };
    let root = Root::new();
    let (data, note) = fixture(&root, &binary, "data");
    old_ok(
        &root,
        &binary,
        &data,
        &["begin-edit", "--note", &note, "--base", "1"],
    );
    let backups = root.join("backups");
    std::fs::create_dir(&backups).unwrap();
    let refused = call(
        &root,
        &data,
        &["upgrade", "--backup-directory", backups.to_str().unwrap()],
    );
    assert_error(&refused, "rejected", "unsafe_directory");
    assert!(refused["error"]["message"]
        .as_str()
        .unwrap()
        .contains("save/discard edits"));
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 0);
    assert_eq!(user_version(&data), 5);
    let read = call(&root, &data, &["notes"]);
    assert_upgrade_required(&read);
}

#[test]
fn upgrade_backs_up_first_and_the_upgraded_store_works() {
    let Some(binary) = schema5() else { return };
    let root = Root::new();
    let (data, note) = fixture(&root, &binary, "data");
    let backups = root.join("backups");
    std::fs::create_dir(&backups).unwrap();
    let upgraded = call(
        &root,
        &data,
        &["upgrade", "--backup-directory", backups.to_str().unwrap()],
    );
    assert_eq!(upgraded["status"], "ok", "{upgraded}");
    assert_eq!(upgraded["authority"]["workspace_purpose"], Value::Null);
    assert_eq!(user_version(&data), 6);
    let files: Vec<PathBuf> = std::fs::read_dir(&backups)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1, "{files:?}");
    let name = files[0].file_name().unwrap().to_str().unwrap();
    assert!(
        name.starts_with("brn-before-schema-6-") && name.ends_with(".sqlite3"),
        "{name}"
    );
    // The verified backup is the schema-5 store: the old binary restores it.
    let restored = root.join("restored");
    old_ok(
        &root,
        &binary,
        &restored,
        &["restore", "--source", files[0].to_str().unwrap()],
    );
    assert_eq!(user_version(&restored), 5);
    let old_notes = old_ok(&root, &binary, &restored, &["notes"]);
    assert_eq!(old_notes[0]["id"], note.as_str());
    // After the upgrade the CLI works normally on the owner workspace.
    let notes = call(&root, &data, &["notes"]);
    assert_eq!(notes["status"], "ok", "{notes}");
    assert_eq!(notes["authority"]["workspace_purpose"], "owner");
    assert_eq!(notes["result"][0]["id"], note.as_str());
    let (session, token) = begin_edit(&root, &data, &note, 1);
    let saved = call(
        &root,
        &data,
        &[
            "save",
            "--session",
            &id(&session),
            "--generation",
            "0",
            "--token-file",
            &token,
        ],
    );
    assert_eq!(saved["outcome"], "Saved", "{saved}");
    // A second upgrade records nothing and backs up nothing.
    let again = call(
        &root,
        &data,
        &["upgrade", "--backup-directory", backups.to_str().unwrap()],
    );
    assert_eq!(again["status"], "ok");
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 1);
    // A schema-5 build refuses schema 6: the upgraded store and a fresh one.
    let refused = old(&root, &binary, false, &data, &["notes"]);
    assert_ne!(refused.code, 0, "{}", refused.stdout);
    let fresh = root.join("fresh");
    ok(&root, &fresh, &["init"]);
    assert_eq!(user_version(&fresh), 6);
    let refused = old(&root, &binary, false, &fresh, &["notes"]);
    assert_ne!(refused.code, 0, "{}", refused.stdout);
}
