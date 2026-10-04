//! Simple editor contracts across independent CLI processes and restart.
use serde_json::Value;
use std::{fs, path::Path, process::Command};
use tempfile::TempDir;

struct Fixture {
    _owner: TempDir,
    data: std::path::PathBuf,
    vault: std::path::PathBuf,
    credentials: std::path::PathBuf,
    input: std::path::PathBuf,
}

impl Fixture {
    fn new() -> Self {
        // The exclusive owner directory is outside Git and has private modes;
        // neither the fixture vault nor credentials come from user data.
        let owner = tempfile::Builder::new()
            .prefix("brn-cli-editor-")
            .tempdir_in(std::env::temp_dir().canonicalize().unwrap())
            .unwrap();
        let data = owner.path().join("data");
        let vault = owner.path().join("vault");
        fs::create_dir(&data).unwrap();
        fs::create_dir(&vault).unwrap();
        Self {
            credentials: owner.path().join("credentials"),
            input: owner.path().join("input.txt"),
            _owner: owner,
            data,
            vault,
        }
    }

    #[cfg(target_os = "macos")]
    fn open(&self) -> Value {
        ok(
            &self.data,
            &[
                "edit",
                "open",
                "plan.md",
                "--vault",
                self.vault.to_str().unwrap(),
                "--credentials-dir",
                self.credentials.to_str().unwrap(),
            ],
        )
    }
}

fn run(data: &Path, args: &[&str]) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .args(["--json", "--data-dir"])
        .arg(data)
        .output()
        .unwrap();
    let envelope: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|_| panic!("expected one envelope: {output:?}"));
    assert_eq!(envelope["schema_version"], 1);
    (output.status.code().unwrap(), envelope)
}

#[cfg(target_os = "macos")]
fn ok(data: &Path, args: &[&str]) -> Value {
    let (exit, envelope) = run(data, args);
    assert_eq!(exit, 0, "{envelope}");
    assert_eq!(envelope["ok"], true);
    envelope["data"].clone()
}

fn write_args<'a>(
    sub: &'a str,
    baseline: &'a str,
    expected: &'a str,
    generation: &'a str,
    input: &'a Path,
) -> Vec<&'a str> {
    vec![
        "edit",
        sub,
        "plan.md",
        "--baseline",
        baseline,
        "--expected-generation",
        expected,
        "--generation",
        generation,
        "--file",
        input.to_str().unwrap(),
    ]
}

#[cfg(target_os = "macos")]
#[test]
fn recovery_restart_save_exact_bytes_and_uuid_replay() {
    let f = Fixture::new();
    let original = "\u{feff}---\r\ntitle: Märkmed\r\n---\r\nAlgne.\r\n";
    fs::write(f.vault.join("plan.md"), original).unwrap();
    let view = f.open();
    assert_eq!(view["record"]["text"], original);
    let baseline = view["record"]["stamp"]["baseline"].as_str().unwrap();
    let changed = "\u{feff}---\r\ntitle: Märkmed\r\n---\r\n日本語 ja eesti.\r\n\r\n";
    fs::write(&f.input, changed).unwrap();
    let recover = write_args("recover", baseline, "0", "1", &f.input);
    let record = ok(&f.data, &recover);
    assert_eq!(record["text"], changed);
    assert_eq!(record["stamp"]["generation"], 1);
    assert_eq!(
        fs::read(f.vault.join("plan.md")).unwrap(),
        original.as_bytes()
    );

    // Every command is a new process: reopening and listing prove that the
    // acknowledged recovery buffer survives worker shutdown and restart.
    let reopened = ok(&f.data, &["edit", "open", "plan.md"]);
    assert_eq!(reopened["record"]["text"], changed);
    let records = ok(&f.data, &["edit", "list"]);
    assert_eq!(records[0]["text"], changed);
    let operation = uuid::Uuid::new_v4().to_string();
    let mut save = write_args("save", baseline, "1", "1", &f.input);
    save.extend(["--operation", &operation]);
    let receipt = ok(&f.data, &save);
    assert_eq!(receipt["operation_id"], operation);
    assert_eq!(receipt["outcome"], "Applied");
    assert_eq!(
        fs::read(f.vault.join("plan.md")).unwrap(),
        changed.as_bytes()
    );
    let saved = ok(&f.data, &["edit", "open", "plan.md"]);
    assert_eq!(saved["saved"], changed);

    fs::write(f.vault.join("plan.md"), b"external\n").unwrap();
    assert_eq!(ok(&f.data, &save), receipt);
    assert_eq!(ok(&f.data, &["edit", "reconcile", &operation]), receipt);
    assert_eq!(fs::read(f.vault.join("plan.md")).unwrap(), b"external\n");
    fs::write(&f.input, "different replay").unwrap();
    let (exit, error) = run(&f.data, &save);
    assert_eq!(exit, 1);
    assert_eq!(error["error"]["code"], "OPERATION_CONFLICT");
    assert_eq!(fs::read(f.vault.join("plan.md")).unwrap(), b"external\n");
}

#[cfg(target_os = "macos")]
#[test]
fn external_conflict_preserves_disk_and_recovered_work() {
    let f = Fixture::new();
    fs::write(f.vault.join("plan.md"), "base").unwrap();
    let view = f.open();
    let baseline = view["record"]["stamp"]["baseline"].as_str().unwrap();
    fs::write(&f.input, "my unfinished work\r\n").unwrap();
    fs::write(f.vault.join("plan.md"), "external").unwrap();
    let operation = uuid::Uuid::new_v4().to_string();
    let mut save = write_args("save", baseline, "0", "1", &f.input);
    save.extend(["--operation", &operation]);
    let (exit, error) = run(&f.data, &save);
    assert_eq!(exit, 1);
    assert_eq!(error["error"]["code"], "CONTEXT_STALE");
    assert_eq!(fs::read(f.vault.join("plan.md")).unwrap(), b"external");
    let records = ok(&f.data, &["edit", "list"]);
    assert_eq!(records[0]["text"], "my unfinished work\r\n");
    let reopened = ok(&f.data, &["edit", "open", "plan.md"]);
    assert_eq!(reopened["conflict"], true);
    assert_eq!(reopened["record"]["text"], "my unfinished work\r\n");
    assert_eq!(run(&f.data, &save).1["error"]["code"], "CONTEXT_STALE");
    assert_eq!(fs::read(f.vault.join("plan.md")).unwrap(), b"external");
}

#[cfg(target_os = "macos")]
#[test]
fn copy_is_exclusive_and_empty_utf8_can_be_saved() {
    let f = Fixture::new();
    fs::write(f.vault.join("plan.md"), "original").unwrap();
    fs::write(f.vault.join("occupied.md"), "occupant").unwrap();
    let view = f.open();
    let baseline = view["record"]["stamp"]["baseline"].as_str().unwrap();
    fs::write(&f.input, b"").unwrap();
    let operation = uuid::Uuid::new_v4().to_string();
    let mut copy = write_args("save", baseline, "0", "1", &f.input);
    copy.extend(["--operation", &operation, "--copy", "occupied.md"]);
    let (exit, error) = run(&f.data, &copy);
    assert_eq!(exit, 1, "{error}");
    assert_eq!(fs::read(f.vault.join("occupied.md")).unwrap(), b"occupant");
    assert_eq!(fs::read(f.vault.join("plan.md")).unwrap(), b"original");
    let operation = uuid::Uuid::new_v4().to_string();
    let mut copy = write_args("save", baseline, "1", "1", &f.input);
    copy.extend(["--operation", &operation, "--copy", "copy.md"]);
    ok(&f.data, &copy);
    assert_eq!(fs::read(f.vault.join("copy.md")).unwrap(), b"");
    assert_eq!(fs::read(f.vault.join("plan.md")).unwrap(), b"original");
}

#[cfg(target_os = "macos")]
#[test]
fn reload_requires_discard_and_exact_fresh_disk_observation() {
    let f = Fixture::new();
    fs::write(f.vault.join("plan.md"), "original").unwrap();
    let view = f.open();
    let baseline = view["record"]["stamp"]["baseline"].as_str().unwrap();
    fs::write(&f.input, "unfinished").unwrap();
    ok(
        &f.data,
        &write_args("recover", baseline, "0", "1", &f.input),
    );
    fs::write(f.vault.join("plan.md"), "first external version").unwrap();
    let viewed = ok(&f.data, &["edit", "open", "plan.md"]);
    fs::write(&f.input, serde_json::to_vec(&viewed["observed"]).unwrap()).unwrap();
    let mut reload = vec![
        "edit",
        "reload",
        "plan.md",
        "--baseline",
        baseline,
        "--expected-generation",
        "1",
        "--observed-file",
        f.input.to_str().unwrap(),
    ];
    assert_eq!(run(&f.data, &reload).0, 1, "discard must be explicit");
    assert_eq!(ok(&f.data, &["edit", "list"])[0]["text"], "unfinished");

    fs::write(f.vault.join("plan.md"), "new external version\r\n").unwrap();
    reload.push("--discard");
    let (exit, error) = run(&f.data, &reload);
    assert_eq!(exit, 1, "{error}");
    assert_eq!(error["error"]["code"], "CONTEXT_STALE");
    assert_eq!(ok(&f.data, &["edit", "list"])[0]["text"], "unfinished");
    let viewed = ok(&f.data, &["edit", "open", "plan.md"]);
    fs::write(&f.input, serde_json::to_vec(&viewed["observed"]).unwrap()).unwrap();
    let record = ok(&f.data, &reload);
    assert_eq!(record["text"], "new external version\r\n");
    assert_ne!(record["stamp"]["baseline"], baseline);
    assert_eq!(
        fs::read(f.vault.join("plan.md")).unwrap(),
        b"new external version\r\n"
    );
}

#[test]
fn invalid_editor_inputs_and_legacy_mode_do_not_initialize_authority() {
    let id = "00000000-0000-0000-0000-000000000001";
    for mut args in [
        vec!["edit", "open", "../plan.md"],
        vec!["edit", "open", "plan.md", "--legacy"],
        vec!["edit", "save", "plan.md", "--baseline", id],
        vec!["edit", "reconcile", "bad-uuid"],
    ] {
        let f = Fixture::new();
        args.extend([
            "--vault",
            f.vault.to_str().unwrap(),
            "--credentials-dir",
            f.credentials.to_str().unwrap(),
        ]);
        let (exit, error) = run(&f.data, &args);
        assert_eq!(exit, 2, "{error}");
        assert_eq!(error["error"]["code"], "USAGE");
        assert!(!f.data.join("brn.sqlite").exists());
        assert!(!f.data.join("brn.sqlite3").exists());
    }
    for bytes in [vec![0xff], vec![b'a'; 1024 * 1024 + 1]] {
        let f = Fixture::new();
        fs::write(&f.input, bytes).unwrap();
        let mut args = write_args("save", id, "0", "1", &f.input);
        args.extend(["--operation", id]);
        let (exit, error) = run(&f.data, &args);
        assert_eq!(exit, 1, "{error}");
        assert!(!f.data.join("brn.sqlite").exists());
        assert!(!f.data.join("brn.sqlite3").exists());
    }
    let f = Fixture::new();
    fs::write(f.data.join("brn.sqlite3-journal"), []).unwrap();
    let (exit, error) = run(&f.data, &["edit", "open", "plan.md"]);
    assert_eq!(exit, 1);
    assert_eq!(error["error"]["code"], "WORKSPACE_MODE_CONFLICT");
    assert!(!f.data.join("brn.sqlite").exists());
}
