//! Typed citation requests and exact approval through real synthetic CLI processes.
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
use uuid::Uuid;

mod support;

struct Fixture {
    data: support::DataDir,
    vault: PathBuf,
    input: PathBuf,
    credentials: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let data = support::data_dir();
        let parent = data.path().parent().unwrap();
        let vault = parent.join("vault");
        fs::create_dir(&vault).unwrap();
        Self {
            vault,
            input: parent.join("request.json"),
            credentials: parent.join("task.credentials"),
            data,
        }
    }

    fn process(&self, args: &[&str], machine: bool) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_brn"));
        command
            .args(args)
            .arg("--data-dir")
            .arg(self.data.path())
            .arg("--vault")
            .arg(&self.vault)
            .arg("--credentials-dir")
            .arg(&self.credentials);
        if machine {
            command.arg("--json");
        }
        command.output().unwrap()
    }

    fn run(&self, args: &[&str]) -> (i32, Value) {
        let output = self.process(args, true);
        (
            output.status.code().unwrap(),
            serde_json::from_slice(&output.stdout)
                .unwrap_or_else(|_| panic!("missing typed CLI envelope: {output:?}")),
        )
    }

    fn input(&self, subcommand: &str, value: &Value) -> (i32, Value) {
        fs::write(&self.input, serde_json::to_vec(value).unwrap()).unwrap();
        self.run(&[
            "provenance",
            subcommand,
            "--file",
            self.input.to_str().unwrap(),
        ])
    }

    fn assert_unopened(&self) {
        assert_eq!(fs::read_dir(self.data.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&self.vault).unwrap().count(), 0);
        assert!(!self.credentials.exists());
        assert!(!self.data.path().with_file_name("data.credentials").exists());
    }
}

fn ok(result: (i32, Value), command: &str) -> Value {
    assert_eq!(result.0, 0, "{}", result.1);
    assert_eq!(result.1["schema_version"], 1);
    assert_eq!(result.1["command"], command);
    assert_eq!(result.1["ok"], true);
    result.1["data"].clone()
}

fn capture_request() -> Value {
    json!({"note_id":Uuid::new_v4(), "expected_sha256":vec![17;32],
        "start_byte":0, "end_byte":1})
}

#[test]
fn malformed_syntax_and_typed_requests_refuse_before_storage_or_credentials() {
    let f = Fixture::new();
    for args in [
        vec!["provenance"],
        vec!["provenance", "unknown"],
        vec!["provenance", "show"],
        vec!["provenance", "show", "../outside.md"],
        vec!["provenance", "show", "/absolute.md"],
        vec!["provenance", "show", ".hidden.md"],
        vec!["provenance", "show", "source.txt"],
        vec!["provenance", "show", "note.md", "extra"],
        vec!["provenance", "capture"],
        vec!["provenance", "prepare", "extra", "--file", "missing.json"],
        vec!["provenance", "capture", "--file", "one", "--file", "two"],
        vec!["provenance", "capture", "--unknown", "value"],
    ] {
        let result = f.run(&args);
        assert_eq!(result.0, 2, "{args:?}: {}", result.1);
        assert_eq!(result.1["error"]["code"], "USAGE");
        f.assert_unopened();
    }

    let capture = capture_request();
    for (key, value) in [
        ("note_id", json!(Uuid::nil())),
        ("note_id", json!("bad UUID")),
        ("expected_sha256", json!(vec![17; 31])),
        ("expected_sha256", Value::Null),
        ("start_byte", Value::Null),
        ("start_byte", json!(2)),
        ("end_byte", json!(0)),
        ("end_byte", json!(brn_workflow::MAX_NOTE_BYTES + 1)),
        ("unknown", json!(true)),
    ] {
        let mut request = capture.clone();
        request[key] = value;
        let result = f.input("capture", &request);
        assert_eq!(result.0, 2, "{request}: {}", result.1);
        assert_eq!(result.1["command"], "provenance.capture");
        assert_eq!(result.1["error"]["code"], "USAGE");
        f.assert_unopened();
    }

    for value in [json!({}), Value::Null, json!([])] {
        for subcommand in ["capture", "prepare"] {
            let result = f.input(subcommand, &value);
            assert_eq!(result.0, 2, "{value}: {}", result.1);
            assert_eq!(result.1["error"]["code"], "USAGE");
            f.assert_unopened();
        }
    }

    let prepare = json!({"path":"note.md","proposal_id":Uuid::new_v4(),"title":"Cite source",
        "citations":[{"note_id":Uuid::new_v4(),"sha256":vec![17;32],
            "start_byte":0,"end_byte":1,"quote":"x"}]});
    for (key, value) in [
        ("path", json!("archive/note.md")),
        ("path", json!("../outside.md")),
        ("path", Value::Null),
        ("proposal_id", json!(Uuid::nil())),
        ("title", json!(" ")),
        ("title", json!("õ".repeat(257))),
        ("citations", json!([])),
        ("citations", Value::Null),
        ("unknown", json!(true)),
    ] {
        let mut request = prepare.clone();
        request[key] = value;
        let result = f.input("prepare", &request);
        assert_eq!(result.0, 2, "{request}: {}", result.1);
        assert_eq!(result.1["command"], "provenance.prepare");
        assert_eq!(result.1["error"]["code"], "USAGE");
        f.assert_unopened();
    }
    for (key, value) in [
        ("note_id", json!(Uuid::nil())),
        ("sha256", json!(vec![17; 31])),
        ("quote", Value::Null),
        ("quote", json!("xx")),
        ("unknown", json!(true)),
    ] {
        let mut request = prepare.clone();
        request["citations"][0][key] = value;
        let result = f.input("prepare", &request);
        assert_eq!(result.0, 2, "{request}: {}", result.1);
        f.assert_unopened();
    }

    // Duplicate fields are invalid typed input even when both values agree.
    let raw = format!(
        "{{\"note_id\":\"{}\",\"note_id\":\"{}\",\"expected_sha256\":{},\"start_byte\":0,\"end_byte\":1}}",
        capture["note_id"].as_str().unwrap(),
        capture["note_id"].as_str().unwrap(),
        capture["expected_sha256"]
    );
    for bytes in [b"{".as_slice(), b"\xff".as_slice(), raw.as_bytes()] {
        fs::write(&f.input, bytes).unwrap();
        assert_eq!(
            f.run(&["provenance", "capture", "--file", f.input.to_str().unwrap()])
                .0,
            2
        );
        f.assert_unopened();
    }
}

#[test]
fn oversized_and_nonregular_input_refuse_before_startup_without_a_fifo_writer() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    fs::File::create(&f.input)
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    for subcommand in ["capture", "prepare"] {
        assert_eq!(
            f.run(&[
                "provenance",
                subcommand,
                "--file",
                f.input.to_str().unwrap()
            ])
            .0,
            2
        );
        f.assert_unopened();
    }
    fs::remove_file(&f.input).unwrap();
    let path = CString::new(f.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: a NUL-terminated path in the exclusively owned synthetic fixture.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["provenance", "capture", "--file"])
        .arg(&f.input)
        .args(["--json", "--data-dir"])
        .arg(f.data.path())
        .arg("--credentials-dir")
        .arg(&f.credentials)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut blocked = false;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            blocked = true;
            child.kill().unwrap();
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!blocked, "provenance input waited for a FIFO writer");
    assert_eq!(output.status.code(), Some(2));
    f.assert_unopened();
}

#[cfg(target_os = "macos")]
#[test]
fn archived_source_capture_and_full_proposal_remain_durable_after_approval_and_rebuild() {
    let f = Fixture::new();
    let note_id = Uuid::new_v4();
    let source = format!(
        "\u{feff}---\r\nbrn_id: {note_id}\r\nbrn_kind: source\r\n---\r\n# Eesti\r\nOriginal õäöü wording\r\n"
    );
    let target = "\u{feff}---\r\ncustom: preserved\r\n---\r\n# Knowledge\r\nInterpreted result\r\n";
    fs::create_dir(f.vault.join("archive")).unwrap();
    fs::write(f.vault.join("archive/source.md"), &source).unwrap();
    fs::write(f.vault.join("knowledge.md"), target).unwrap();
    let inventory = ok(f.run(&["identity", "inventory"]), "identity.inventory");
    let source_hash = inventory["notes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|note| note["path"] == "archive/source.md")
        .unwrap()["sha256"]
        .clone();
    let quote = "Original õäöü wording\r\n";
    let start = source.find(quote).unwrap();
    let capture = ok(
        f.input(
            "capture",
            &json!({"note_id":note_id,
        "expected_sha256":source_hash,"start_byte":start,"end_byte":start+quote.len()}),
        ),
        "provenance.capture",
    );
    let citation = capture["citation"].clone();
    assert_eq!(citation["note_id"], note_id.to_string());
    assert_eq!(citation["quote"], quote);
    let proposal_id = Uuid::new_v4();
    let request = json!({"path":"knowledge.md","proposal_id":proposal_id,
        "title":"Preserve original source evidence", "citations":[citation]});
    let draft = ok(f.input("prepare", &request), "provenance.prepare");
    assert_eq!(draft["id"], proposal_id.to_string());
    assert_eq!(draft["changes"][0]["kind"], "replace");
    let proposed = draft["changes"][0]["text"].as_str().unwrap();
    assert!(proposed.ends_with("# Knowledge\r\nInterpreted result\r\n"));
    assert!(proposed.contains("custom: preserved\r\n"));
    assert!(proposed.contains("brn_provenance:"));
    assert_eq!(
        fs::read(f.vault.join("knowledge.md")).unwrap(),
        target.as_bytes()
    );
    assert_eq!(
        ok(f.run(&["proposals", "list"]), "proposals.list"),
        json!([])
    );
    assert_eq!(ok(f.run(&["edit", "list"]), "edit.list"), json!([]));
    fs::write(&f.input, serde_json::to_vec(&draft).unwrap()).unwrap();
    let created = ok(
        f.run(&["proposals", "create", "--file", f.input.to_str().unwrap()]),
        "proposals.create",
    );
    assert_eq!(created["draft"]["id"], proposal_id.to_string());
    let operation = Uuid::new_v4();
    let approved = ok(
        f.run(&[
            "proposals",
            "approve",
            &proposal_id.to_string(),
            "--review-version",
            "1",
            "--operation",
            &operation.to_string(),
        ]),
        "proposals.approve",
    );
    assert_eq!(approved["outcome"], "applied");
    assert_eq!(
        fs::read(f.vault.join("knowledge.md")).unwrap(),
        proposed.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("archive/source.md")).unwrap(),
        source.as_bytes()
    );

    // Each command restarts the application. Derived-index removal does not
    // remove provenance or change its exact stored citation snapshot.
    let shown = ok(
        f.run(&["provenance", "show", "knowledge.md"]),
        "provenance.show",
    );
    assert_eq!(shown["citations"][0]["citation"], request["citations"][0]);
    assert_eq!(shown["citations"][0]["outcome"], "matched");
    assert_eq!(
        shown["citations"][0]["matches"][0]["path"],
        "archive/source.md"
    );
    fs::remove_file(f.data.path().join("index.sqlite")).unwrap();
    assert_eq!(
        ok(
            f.run(&["provenance", "show", "knowledge.md"]),
            "provenance.show"
        ),
        shown
    );
    let plain = f.process(&["provenance", "show", "knowledge.md"], false);
    assert!(plain.status.success(), "{plain:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&plain.stdout).unwrap(),
        shown
    );
    assert_eq!(
        fs::read(f.vault.join("knowledge.md")).unwrap(),
        proposed.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("archive/source.md")).unwrap(),
        source.as_bytes()
    );
}
