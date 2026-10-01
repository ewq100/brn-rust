//! Note contracts exercised across independent CLI processes.
#![cfg(target_os = "macos")]
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};
use tempfile::{Builder, TempDir};

fn directory() -> TempDir {
    Builder::new()
        .prefix("cli-notes-")
        .tempdir_in(std::env::current_dir().unwrap())
        .unwrap()
}

fn run(root: &Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .args(["--data-dir", root.to_str().unwrap(), "--json"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    if args.first() == Some(&"notes") {
        assert!(output.stderr.is_empty(), "{:?}", output);
    }
    let envelope: Value =
        serde_json::from_slice(&output.stdout).expect("exactly one JSON envelope");
    assert_eq!(envelope["schema_version"], 1);
    (output.status.code().unwrap(), envelope)
}

fn ok(root: &Path, args: &[&str]) -> Value {
    let (exit, envelope) = run(root, args);
    assert_eq!(exit, 0, "{envelope}");
    assert_eq!(envelope["ok"], true);
    envelope["data"].clone()
}

#[test]
fn open_exact_bytes_buffer_save_restart_and_replay() {
    let data = directory();
    let vault = directory();
    let path = vault.path().join("plan.md");
    fs::write(&path, b"\xef\xbb\xbf# plan\r\n").unwrap();
    let view = ok(
        data.path(),
        &[
            "notes",
            "open",
            path.to_str().unwrap(),
            "--vault",
            vault.path().to_str().unwrap(),
        ],
    );
    assert_eq!(view["saved"], "\u{feff}# plan\r\n");
    let id = view["id"].as_str().unwrap();
    let state = view["stamp"]["file_state"].as_str().unwrap();
    let text = data.path().join("text.txt");
    fs::write(&text, b"# updated\r\n").unwrap();
    let buffer = ok(
        data.path(),
        &[
            "notes",
            "buffer",
            "save",
            id,
            "--base-file-state",
            state,
            "--expected-generation",
            "0",
            "--generation",
            "1",
            "--text-file",
            text.to_str().unwrap(),
        ],
    );
    assert_eq!(buffer["stamp"]["generation"], 1);
    assert_eq!(fs::read(&path).unwrap(), b"\xef\xbb\xbf# plan\r\n");
    let operation = uuid::Uuid::new_v4().to_string();
    let save = [
        "notes",
        "save",
        id,
        "--base-file-state",
        state,
        "--expected-generation",
        "1",
        "--generation",
        "1",
        "--text-file",
        text.to_str().unwrap(),
        "--operation",
        &operation,
    ];
    let receipt = ok(data.path(), &save);
    assert_eq!(receipt["filesystem_outcome"], "Applied");
    assert_eq!(receipt["note_id"], id);
    assert_eq!(fs::read(&path).unwrap(), b"# updated\r\n");
    assert_eq!(
        ok(data.path(), &["notes", "show", id])["saved"],
        "# updated\r\n"
    );
    assert_eq!(
        ok(data.path(), &["notes", "recovery", "show", id])["working"],
        "# updated\r\n"
    );
    assert_eq!(
        ok(data.path(), &["notes", "recovery", "list"])
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fs::write(&path, b"# external\n").unwrap();
    assert_eq!(ok(data.path(), &save), receipt);
    assert_eq!(
        ok(
            data.path(),
            &["notes", "recovery", "reconcile", "--operation", &operation]
        ),
        receipt
    );
    assert_eq!(fs::read(&path).unwrap(), b"# external\n");
}

#[test]
fn compare_reload_relink_copy_and_approval_use_explicit_tokens() {
    let data = directory();
    let vault = directory();
    let path = vault.path().join("plan.md");
    fs::write(&path, "oldterm").unwrap();
    let view = ok(
        data.path(),
        &[
            "notes",
            "open",
            path.to_str().unwrap(),
            "--vault",
            vault.path().to_str().unwrap(),
        ],
    );
    let id = view["id"].as_str().unwrap();
    let state = view["stamp"]["file_state"].as_str().unwrap();
    ok(
        data.path(),
        &["notes", "approve-for-search", id, "--file-state", state],
    );
    ok(data.path(), &["index", "build"]);
    assert!(!ok(data.path(), &["search", "oldterm"])["evidence"]
        .as_array()
        .unwrap()
        .is_empty());
    fs::write(&path, "newterm").unwrap();
    let compare = ok(data.path(), &["notes", "compare", id]);
    assert_eq!(compare["baseline"], "oldterm");
    assert_eq!(compare["observed"], "newterm");
    let (exit, failure) = run(
        data.path(),
        &["notes", "approve-for-search", id, "--file-state", state],
    );
    assert_eq!(exit, 1);
    assert_eq!(failure["error"]["code"], "NOTE_STATE_CHANGED");
    let reloaded = ok(
        data.path(),
        &[
            "notes",
            "reload",
            id,
            "--base-file-state",
            state,
            "--expected-generation",
            "0",
            "--discard-local-edits",
        ],
    );
    assert_eq!(reloaded["buffer"], "newterm");
    let state = reloaded["stamp"]["file_state"].as_str().unwrap();
    fs::rename(&path, vault.path().join("moved.md")).unwrap();
    let relink = ok(
        data.path(),
        &[
            "notes",
            "relink",
            id,
            "--path",
            "moved.md",
            "--base-file-state",
            state,
            "--expected-generation",
            "0",
            "--confirm-identity",
        ],
    );
    assert_eq!(relink["id"], id);
    assert_eq!(relink["relative_path"], "moved.md");
    let state = relink["stamp"]["file_state"].as_str().unwrap();
    let text = data.path().join("text.txt");
    fs::write(&text, "copyterm\r\n").unwrap();
    let copy = ok(
        data.path(),
        &[
            "notes",
            "save-copy",
            id,
            "--path",
            "copy.md",
            "--base-file-state",
            state,
            "--expected-generation",
            "0",
            "--generation",
            "1",
            "--text-file",
            text.to_str().unwrap(),
        ],
    );
    assert_eq!(copy["source_note_id"], id);
    assert_ne!(copy["note_id"], id);
    assert_eq!(
        ok(
            data.path(),
            &["notes", "show", copy["note_id"].as_str().unwrap()]
        )["saved"],
        "copyterm\r\n"
    );
    assert_eq!(
        fs::read(vault.path().join("copy.md")).unwrap(),
        b"copyterm\r\n"
    );
    assert_eq!(fs::read(vault.path().join("moved.md")).unwrap(), b"newterm");
    let view = ok(data.path(), &["notes", "show", id]);
    ok(
        data.path(),
        &[
            "notes",
            "approve-for-search",
            id,
            "--file-state",
            view["current_file_state"].as_str().unwrap(),
        ],
    );
    ok(data.path(), &["index", "build"]);
    assert!(!ok(data.path(), &["search", "newterm"])["evidence"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn invalid_inputs_and_help_never_initialize_workspace() {
    let data = directory();
    let id = uuid::Uuid::new_v4().to_string();
    for args in [
        vec!["notes", "open", "relative.md", "--vault", "/nonexistent"],
        vec![
            "notes",
            "reload",
            &id,
            "--base-file-state",
            &id,
            "--expected-generation",
            "0",
        ],
        vec![
            "notes",
            "relink",
            &id,
            "--path",
            "../bad.md",
            "--base-file-state",
            &id,
            "--expected-generation",
            "0",
            "--confirm-identity",
        ],
        vec![
            "notes",
            "recovery",
            "accept-current",
            "--save-operation",
            &id,
            "--file-state",
            &id,
        ],
        vec![
            "notes",
            "save",
            &id,
            "--base-file-state",
            &id,
            "--expected-generation",
            "9223372036854775808",
            "--generation",
            "0",
            "--text-file",
            "/nonexistent",
        ],
    ] {
        assert_eq!(run(data.path(), &args).0, 2);
    }
    let input = data.path().join("bad.txt");
    fs::write(&input, [0xff]).unwrap();
    assert_eq!(
        run(
            data.path(),
            &[
                "notes",
                "save",
                &id,
                "--base-file-state",
                &id,
                "--expected-generation",
                "0",
                "--generation",
                "0",
                "--text-file",
                input.to_str().unwrap()
            ]
        )
        .0,
        1
    );
    for words in [
        vec!["open"],
        vec!["show"],
        vec!["save"],
        vec!["compare"],
        vec!["reload"],
        vec!["relink"],
        vec!["save-copy"],
        vec!["approve-for-search"],
        vec!["buffer", "save"],
        vec!["recovery", "list"],
        vec!["recovery", "show"],
        vec!["recovery", "reconcile"],
        vec!["recovery", "accept-current"],
    ] {
        let out = Command::new(env!("CARGO_BIN_EXE_brn"))
            .arg("notes")
            .args(words)
            .args(["--nonsense", "--help"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert!(String::from_utf8(out.stdout).unwrap().contains("brn notes"));
        assert!(out.stderr.is_empty());
    }
    assert_eq!(fs::read_dir(data.path()).unwrap().count(), 1);
}

#[test]
fn vault_ownership_and_missing_root_preserve_inspectable_recovery() {
    use brn_workflow::{notes::NoteSubmission, Config, Workspace};
    let data = directory();
    let other = directory();
    let vault = directory();
    fs::write(vault.path().join("plan.md"), "baseline").unwrap();
    let mut workspace = Workspace::open(data.path(), Config::default()).unwrap();
    let note = workspace
        .open_note(uuid::Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    workspace
        .save_note_buffer(NoteSubmission {
            operation_id: uuid::Uuid::new_v4(),
            note_id: note.id,
            expected: note.stamp,
            generation: 1,
            text: "recover me".into(),
        })
        .unwrap();
    drop(workspace);
    let id = note.id.to_string();
    let state = note.stamp.file_state.to_string();
    let mut owner = Workspace::open(other.path(), Config::default()).unwrap();
    owner
        .open_note(uuid::Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    let view = ok(data.path(), &["notes", "show", &id]);
    assert_eq!(view["availability"], "OwnedElsewhere");
    assert!(view["saved"].is_null());
    assert!(view["current_file_state"].is_null());
    assert!(view["availability_message"].is_string());
    let text = data.path().join("text.txt");
    fs::write(&text, "recover me").unwrap();
    let op = uuid::Uuid::new_v4().to_string();
    let args = [
        "notes",
        "save",
        &id,
        "--base-file-state",
        &state,
        "--expected-generation",
        "1",
        "--generation",
        "1",
        "--text-file",
        text.to_str().unwrap(),
        "--operation",
        &op,
    ];
    let (exit, error) = run(data.path(), &args);
    assert_eq!(exit, 1);
    assert_eq!(error["error"]["code"], "VAULT_BUSY");
    assert_eq!(error["error"]["context"]["operation_id"], op);
    assert_eq!(error["error"]["context"]["note_id"], id);
    assert_eq!(
        error["error"]["context"]["filesystem_outcome"],
        "NotApplied"
    );
    assert_eq!(error["error"]["context"]["recovery_available"], true);
    assert!(error["error"]["context"]["phase"].is_null());
    assert_eq!(
        ok(data.path(), &["notes", "recovery", "show", &id])["working"],
        "recover me"
    );
    drop(owner);
    fs::remove_dir_all(vault.path()).unwrap();
    let view = ok(data.path(), &["notes", "show", &id]);
    assert_eq!(view["availability"], "Unavailable");
    assert!(view["saved"].is_null());
    assert_eq!(view["buffer"], "recover me");
    assert_eq!(
        run(data.path(), &args).1,
        error,
        "recorded refusal replay without vault"
    );
    let new_op = uuid::Uuid::new_v4().to_string();
    let mut new_args = args.to_vec();
    *new_args.last_mut().unwrap() = &new_op;
    let (exit, error) = run(data.path(), &new_args);
    assert_eq!(exit, 1);
    assert_eq!(error["error"]["code"], "VAULT_UNAVAILABLE");
    assert_eq!(error["error"]["context"]["recovery_available"], true);
    assert_eq!(
        ok(data.path(), &["notes", "recovery", "list"])[0]["working"],
        "recover me"
    );
}

#[test]
fn stale_generation_conflicts_missing_and_unsupported_are_typed() {
    let data = directory();
    let vault = directory();
    let path = vault.path().join("plan.md");
    fs::write(&path, "base").unwrap();
    let view = ok(
        data.path(),
        &[
            "notes",
            "open",
            path.to_str().unwrap(),
            "--vault",
            vault.path().to_str().unwrap(),
        ],
    );
    let id = view["id"].as_str().unwrap();
    let state = view["stamp"]["file_state"].as_str().unwrap();
    let text = data.path().join("text.txt");
    fs::write(&text, "mine").unwrap();
    let op = uuid::Uuid::new_v4().to_string();
    let args = [
        "notes",
        "buffer",
        "save",
        id,
        "--base-file-state",
        state,
        "--expected-generation",
        "0",
        "--generation",
        "1",
        "--text-file",
        text.to_str().unwrap(),
        "--operation",
        &op,
    ];
    ok(data.path(), &args);
    fs::write(&text, "different").unwrap();
    let (exit, error) = run(data.path(), &args);
    assert_eq!(exit, 1);
    assert_eq!(error["error"]["code"], "OPERATION_CONFLICT");
    let save = [
        "notes",
        "save",
        id,
        "--base-file-state",
        state,
        "--expected-generation",
        "0",
        "--generation",
        "1",
        "--text-file",
        text.to_str().unwrap(),
    ];
    assert_eq!(
        run(data.path(), &save).1["error"]["code"],
        "NOTE_STATE_CHANGED"
    );
    fs::write(&text, "mine").unwrap();
    fs::write(&path, "external").unwrap();
    let save = [
        "notes",
        "save",
        id,
        "--base-file-state",
        state,
        "--expected-generation",
        "1",
        "--generation",
        "1",
        "--text-file",
        text.to_str().unwrap(),
    ];
    let error = run(data.path(), &save).1;
    assert_eq!(error["error"]["code"], "NOTE_CONFLICT");
    assert_eq!(error["error"]["context"]["recovery_available"], true);
    assert_eq!(fs::read(&path).unwrap(), b"external");
    fs::remove_file(&path).unwrap();
    assert_eq!(run(data.path(), &save).1["error"]["code"], "NOTE_MISSING");
    fs::write(&path, [0xff]).unwrap();
    assert_eq!(
        run(data.path(), &save).1["error"]["code"],
        "NOTE_UNSUPPORTED"
    );
    assert_eq!(
        ok(data.path(), &["notes", "recovery", "show", id])["working"],
        "mine"
    );
}

#[test]
fn closed_stdout_after_note_save_is_quiet_and_keeps_durable_receipt() {
    let data = directory();
    let vault = directory();
    let path = vault.path().join("plan.md");
    fs::write(&path, "base").unwrap();
    let view = ok(
        data.path(),
        &[
            "notes",
            "open",
            path.to_str().unwrap(),
            "--vault",
            vault.path().to_str().unwrap(),
        ],
    );
    let id = view["id"].as_str().unwrap();
    let state = view["stamp"]["file_state"].as_str().unwrap();
    let text = data.path().join("text.txt");
    fs::write(&text, "saved").unwrap();
    let op = uuid::Uuid::new_v4().to_string();
    let args = [
        "notes",
        "save",
        id,
        "--base-file-state",
        state,
        "--expected-generation",
        "0",
        "--generation",
        "1",
        "--text-file",
        text.to_str().unwrap(),
        "--operation",
        &op,
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .args(["--data-dir", data.path().to_str().unwrap(), "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(fs::read(&path).unwrap(), b"saved");
    assert_eq!(ok(data.path(), &args)["filesystem_outcome"], "Applied");
}

#[test]
fn bounded_input_rejects_oversize_nonregular_and_missing_before_workspace_open() {
    let data = directory();
    let input = directory();
    let id = uuid::Uuid::new_v4().to_string();
    let file = input.path().join("text.txt");
    fs::write(&file, vec![b'a'; brn_workflow::MAX_DRAFT_BYTES + 1]).unwrap();
    for path in [&file, input.path(), &input.path().join("missing.txt")] {
        assert_eq!(
            run(
                data.path(),
                &[
                    "notes",
                    "save",
                    &id,
                    "--base-file-state",
                    &id,
                    "--expected-generation",
                    "0",
                    "--generation",
                    "0",
                    "--text-file",
                    path.to_str().unwrap()
                ]
            )
            .0,
            1
        );
        assert_eq!(fs::read_dir(data.path()).unwrap().count(), 0);
    }
}
#[test]
fn accept_current_unknown_save_operation_returns_typed_missing_failure() {
    let data = directory();
    let vault = directory();
    let path = vault.path().join("plan.md");
    fs::write(&path, "base").unwrap();
    let view = ok(
        data.path(),
        &[
            "notes",
            "open",
            path.to_str().unwrap(),
            "--vault",
            vault.path().to_str().unwrap(),
        ],
    );
    let save_op = uuid::Uuid::new_v4().to_string();
    let ack_op = uuid::Uuid::new_v4().to_string();
    let state = view["current_file_state"].as_str().unwrap();
    let (exit, envelope) = run(
        data.path(),
        &[
            "notes",
            "recovery",
            "accept-current",
            "--save-operation",
            &save_op,
            "--file-state",
            state,
            "--keep-recovery",
            "--operation",
            &ack_op,
        ],
    );
    assert_eq!(exit, 1);
    assert_eq!(envelope["command"], "notes.recovery.accept-current");
    assert_eq!(envelope["error"]["code"], "NOTE_MISSING");
    assert_eq!(envelope["error"]["context"]["operation_id"], ack_op);
    assert_eq!(
        envelope["error"]["context"]["filesystem_outcome"],
        "NotApplied"
    );
    assert!(envelope.get("data").is_none());
    assert_eq!(fs::read(&path).unwrap(), b"base");
    assert_eq!(
        ok(data.path(), &["notes", "recovery", "list"])[0]["working"],
        "base"
    );
}

#[test]
fn empty_note_noop_save_returns_verified_not_applied_receipt() {
    let data = directory();
    let vault = directory();
    let path = vault.path().join("empty.md");
    fs::write(&path, []).unwrap();
    let view = ok(
        data.path(),
        &[
            "notes",
            "open",
            path.to_str().unwrap(),
            "--vault",
            vault.path().to_str().unwrap(),
        ],
    );
    let input = data.path().join("empty.txt");
    fs::write(&input, []).unwrap();
    let receipt = ok(
        data.path(),
        &[
            "notes",
            "save",
            view["id"].as_str().unwrap(),
            "--base-file-state",
            view["stamp"]["file_state"].as_str().unwrap(),
            "--expected-generation",
            "0",
            "--generation",
            "0",
            "--text-file",
            input.to_str().unwrap(),
        ],
    );
    assert_eq!(receipt["filesystem_outcome"], "NotApplied");
    assert_eq!(receipt["stamp"], view["stamp"]);
    assert_eq!(receipt["recovery_available"], true);
    assert_eq!(fs::read(&path).unwrap(), b"");
}

#[test]
fn save_copy_replay_after_destination_and_source_edits_preserves_original_receipt() {
    let data = directory();
    let vault = directory();
    let path = vault.path().join("plan.md");
    fs::write(&path, "source baseline\r\n").unwrap();
    let view = ok(
        data.path(),
        &[
            "notes",
            "open",
            path.to_str().unwrap(),
            "--vault",
            vault.path().to_str().unwrap(),
        ],
    );
    let id = view["id"].as_str().unwrap();
    let state = view["stamp"]["file_state"].as_str().unwrap();
    let text = data.path().join("copy.txt");
    fs::write(&text, "submitted copy\r\n").unwrap();
    let operation = uuid::Uuid::new_v4().to_string();
    let args = [
        "notes",
        "save-copy",
        id,
        "--path",
        "copy.md",
        "--base-file-state",
        state,
        "--expected-generation",
        "0",
        "--generation",
        "1",
        "--text-file",
        text.to_str().unwrap(),
        "--operation",
        &operation,
    ];
    let receipt = ok(data.path(), &args);
    assert_eq!(receipt["operation_id"], operation);
    assert_eq!(receipt["source_note_id"], id);
    assert_ne!(receipt["note_id"], id);
    let copy_path = vault.path().join("copy.md");
    assert_eq!(fs::read(&copy_path).unwrap(), b"submitted copy\r\n");
    fs::write(&copy_path, "external copy edit\r\n").unwrap();
    let copy_before = file_snapshot(&copy_path);
    let source = ok(data.path(), &["notes", "show", id]);
    let later = data.path().join("later.txt");
    fs::write(&later, "later source buffer\r\n").unwrap();
    let generation = source["stamp"]["generation"].as_u64().unwrap();
    let expected_generation = generation.to_string();
    let next_generation = (generation + 1).to_string();
    ok(
        data.path(),
        &[
            "notes",
            "buffer",
            "save",
            id,
            "--base-file-state",
            source["stamp"]["file_state"].as_str().unwrap(),
            "--expected-generation",
            &expected_generation,
            "--generation",
            &next_generation,
            "--text-file",
            later.to_str().unwrap(),
        ],
    );
    let source_before = ok(data.path(), &["notes", "recovery", "show", id]);
    assert_eq!(source_before["working"], "later source buffer\r\n");
    assert_eq!(ok(data.path(), &args), receipt);
    assert_eq!(
        ok(
            data.path(),
            &["notes", "recovery", "reconcile", "--operation", &operation,]
        ),
        receipt
    );
    assert_eq!(file_snapshot(&copy_path), copy_before);
    assert_eq!(fs::read(&path).unwrap(), b"source baseline\r\n");
    assert_eq!(
        ok(data.path(), &["notes", "recovery", "show", id]),
        source_before
    );
}

fn file_snapshot(path: &Path) -> (u64, u64, i64, i64, Vec<u8>) {
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::metadata(path).unwrap();
    (
        metadata.dev(),
        metadata.ino(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        fs::read(path).unwrap(),
    )
}

#[test]
fn interrupted_save_accept_current_revalidates_token_preserves_recovery_and_allows_new_save() {
    use brn_store::{
        notes::{DestinationPrecondition, NoteResolution, NoteWriteKind},
        Store,
    };
    use brn_workflow::{notes::NoteSubmission, Config, Workspace};
    use uuid::Uuid;

    let data = directory();
    let vault = directory();
    let path = vault.path().join("plan.md");
    fs::write(&path, "baseline\r\n").unwrap();
    let mut workspace = Workspace::open(data.path(), Config::default()).unwrap();
    let view = workspace
        .open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md"))
        .unwrap();
    workspace
        .approve_note_snapshot(Uuid::new_v4(), view.id, view.current_file_state.unwrap())
        .unwrap();
    let request = NoteSubmission {
        operation_id: Uuid::new_v4(),
        note_id: view.id,
        expected: view.stamp,
        generation: 1,
        text: "submitted recovery\r\n".into(),
    };
    drop(workspace);

    // R19: public store intent commit simulates a crash before workflow file execution.
    let (mut store, _) = Store::open(data.path()).unwrap();
    let record = store.note_record(view.id).unwrap();
    let intent = store
        .begin_note_save(
            &request,
            &record.relative_path,
            NoteWriteKind::Replace,
            &DestinationPrecondition::Existing {
                fingerprint: record.baseline,
                baseline_text: "baseline\r\n".into(),
            },
        )
        .unwrap();
    assert_eq!(intent.resolution, NoteResolution::Unresolved);
    drop(store);
    let stage = vault.path().join(&intent.staging_relative);
    fs::write(&stage, "unproven interrupted artifact\r\n").unwrap();
    fs::write(&path, "first external state\r\n").unwrap();
    let id = view.id.to_string();
    let save_op = request.operation_id.to_string();
    let reconcile = ["notes", "recovery", "reconcile", "--operation", &save_op];
    let (exit, original) = run(data.path(), &reconcile);
    assert_eq!(exit, 1);
    assert_eq!(original["ok"], false);
    assert_eq!(original["error"]["code"], "NOTE_CONFLICT");
    assert_eq!(original["error"]["context"]["operation_id"], save_op);
    assert_eq!(original["error"]["context"]["note_id"], id);
    assert_eq!(original["error"]["context"]["phase"], "Intent");
    assert_eq!(
        original["error"]["context"]["filesystem_outcome"],
        "Unknown"
    );
    assert_eq!(original["error"]["context"]["recovery_available"], true);
    assert!(original.get("data").is_none());
    let (store, _) = Store::open(data.path()).unwrap();
    assert_eq!(
        store
            .note_save_intent(request.operation_id)
            .unwrap()
            .unwrap()
            .resolution,
        NoteResolution::Unresolved
    );
    drop(store);

    let compared = ok(data.path(), &["notes", "compare", &id]);
    assert_eq!(compared["baseline"], "baseline\r\n");
    assert_eq!(compared["working"], request.text);
    assert_eq!(compared["observed"], "first external state\r\n");
    let stale_token = compared["observed_file_state"].as_str().unwrap();
    fs::write(&path, "reviewed external state\r\n").unwrap();
    let before = (file_snapshot(&path), file_snapshot(&stage));
    let stale_ack = Uuid::new_v4().to_string();
    let (exit, stale) = run(
        data.path(),
        &[
            "notes",
            "recovery",
            "accept-current",
            "--save-operation",
            &save_op,
            "--file-state",
            stale_token,
            "--keep-recovery",
            "--operation",
            &stale_ack,
        ],
    );
    assert_eq!(exit, 1);
    assert_eq!(stale["error"]["code"], "NOTE_STATE_CHANGED");
    assert_eq!(stale["error"]["context"]["operation_id"], stale_ack);
    assert!(stale.get("data").is_none());
    assert_eq!((file_snapshot(&path), file_snapshot(&stage)), before);

    let compared = ok(data.path(), &["notes", "compare", &id]);
    let fresh_token = compared["observed_file_state"].as_str().unwrap();
    assert_ne!(fresh_token, stale_token);
    let ack_id = Uuid::new_v4();
    let ack_op = ack_id.to_string();
    let acceptance = [
        "notes",
        "recovery",
        "accept-current",
        "--save-operation",
        &save_op,
        "--file-state",
        fresh_token,
        "--keep-recovery",
        "--operation",
        &ack_op,
    ];
    let accepted = ok(data.path(), &acceptance);
    assert_eq!(accepted["id"], id);
    assert_eq!(accepted["operation_id"], ack_op);
    assert_eq!(accepted["save_operation_id"], save_op);
    assert_eq!(accepted["saved"], "reviewed external state\r\n");
    assert_eq!(accepted["buffer"], request.text);
    assert_eq!(accepted["availability"], "Available");
    assert_eq!(accepted["search_approval"], "Draft");
    assert_eq!(accepted["stamp"]["generation"], 1);
    assert_ne!(accepted["stamp"]["file_state"], fresh_token);
    assert_ne!(
        accepted["stamp"]["file_state"],
        view.stamp.file_state.to_string()
    );
    assert_eq!(
        accepted["stamp"]["file_state"],
        accepted["current_file_state"]
    );
    assert!(accepted.get("filesystem_outcome").is_none());
    assert_eq!((file_snapshot(&path), file_snapshot(&stage)), before);
    assert_eq!(ok(data.path(), &acceptance), accepted);
    let recovery = ok(data.path(), &["notes", "recovery", "show", &id]);
    assert_eq!(recovery["working"], request.text);
    assert_eq!(recovery["baseline"], "reviewed external state\r\n");
    assert_eq!(ok(data.path(), &["notes", "recovery", "list"])[0], recovery);
    let (store, _) = Store::open(data.path()).unwrap();
    let retained = store
        .note_save_intent(request.operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(retained.resolution, NoteResolution::AcceptedCurrent);
    assert_eq!(retained.acknowledged_by, Some(ack_id));
    let protected = store.note_decision_recovery(ack_id).unwrap().unwrap();
    assert_eq!(protected.working, request.text);
    assert_eq!(protected.baseline, "baseline\r\n");
    assert_eq!(protected.stamp.file_state, view.stamp.file_state);
    drop(store);
    assert_eq!(run(data.path(), &reconcile), (1, original.clone()));
    let input = data.path().join("submitted.txt");
    fs::write(&input, &request.text).unwrap();
    let baseline = request.expected.file_state.to_string();
    let save = [
        "notes",
        "save",
        &id,
        "--base-file-state",
        &baseline,
        "--expected-generation",
        "0",
        "--generation",
        "1",
        "--text-file",
        input.to_str().unwrap(),
        "--operation",
        &save_op,
    ];
    let (exit, replay) = run(data.path(), &save);
    assert_eq!(exit, 1);
    assert_eq!(replay["error"], original["error"]);
    assert!(replay.get("data").is_none());
    assert_eq!((file_snapshot(&path), file_snapshot(&stage)), before);
    let new_save = ok(
        data.path(),
        &[
            "notes",
            "save",
            &id,
            "--base-file-state",
            accepted["stamp"]["file_state"].as_str().unwrap(),
            "--expected-generation",
            "1",
            "--generation",
            "1",
            "--text-file",
            input.to_str().unwrap(),
        ],
    );
    assert_eq!(new_save["filesystem_outcome"], "Applied");
    assert_eq!(new_save["source_note_id"], id);
    assert_eq!(new_save["note_id"], id);
    assert_eq!(new_save["submitted_generation"], 1);
    assert_eq!(new_save["recovery_available"], true);
    assert_eq!(fs::read(&path).unwrap(), request.text.as_bytes());
    assert_eq!(file_snapshot(&stage), before.1);
    assert_eq!(run(data.path(), &reconcile), (1, original));
}
