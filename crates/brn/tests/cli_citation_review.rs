//! Citation review through fresh owner CLI processes, using synthetic files only.
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

    fn input(&self, category: &str, subcommand: &str, value: &Value) -> (i32, Value) {
        fs::write(&self.input, serde_json::to_vec(value).unwrap()).unwrap();
        self.run(&[category, subcommand, "--file", self.input.to_str().unwrap()])
    }

    fn assert_unopened(&self) {
        assert_eq!(fs::read_dir(self.data.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&self.vault).unwrap().count(), 0);
        assert!(!self.credentials.exists());
        assert!(!self.data.path().with_file_name("data.credentials").exists());
    }
}

fn ok(result: (i32, Value)) -> Value {
    assert_eq!(result.0, 0, "{}", result.1);
    assert_eq!(result.1["ok"], true);
    result.1["data"].clone()
}

#[test]
fn invalid_requests_refuse_before_operational_or_credential_storage() {
    let f = Fixture::new();
    for args in [
        vec!["needs-review"],
        vec!["needs-review", "unknown"],
        vec!["needs-review", "show"],
        vec!["needs-review", "citations", "extra"],
        vec!["needs-review", "citations", "--limit", "0"],
        vec!["needs-review", "citations", "--limit", "101"],
        vec!["needs-review", "citations", "--limit", "x"],
        vec!["needs-review", "citations", "--file", "missing"],
    ] {
        let result = f.run(&args);
        assert_eq!(result.0, 2, "{args:?}: {}", result.1);
        assert_eq!(result.1["error"]["code"], "USAGE");
        f.assert_unopened();
    }
    for request in [
        json!({}),
        json!({"path":"../escape.md","expected_sha256":vec![0;32]}),
        json!({"path":"note.md","expected_sha256":vec![0;31]}),
        json!({"path":"note.md","expected_sha256":vec![0;32],"extra":true}),
    ] {
        assert_eq!(f.input("needs-review", "show", &request).0, 2);
        f.assert_unopened();
    }
    fs::write(&f.input, b"{\"unexpected\":true}").unwrap();
    assert_eq!(
        f.run(&[
            "needs-review",
            "citations",
            "--cursor",
            f.input.to_str().unwrap()
        ])
        .0,
        2
    );
    f.assert_unopened();
    fs::File::create(&f.input).unwrap().set_len(65537).unwrap();
    assert_eq!(
        f.run(&["needs-review", "show", "--file", f.input.to_str().unwrap()])
            .0,
        2
    );
    f.assert_unopened();
}

#[cfg(unix)]
#[test]
fn nonregular_request_and_cursor_refuse_without_waiting_for_a_fifo_writer() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let path = CString::new(f.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: exclusively owned synthetic temporary path, NUL-terminated above.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    for args in [
        vec!["needs-review", "show", "--file"],
        vec!["needs-review", "citations", "--cursor"],
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
            .args(args)
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
        assert!(!blocked, "citation input waited for a FIFO writer");
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        f.assert_unopened();
    }
}

#[test]
fn paging_restart_exact_detail_and_stale_proof_have_no_review_effects() {
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    let source = format!("---\r\nbrn_id: {source_id}\r\nbrn_kind: source\r\n---\r\n# Evidence\r\nExact õ 日本語 🦀 quote\r\n");
    fs::write(f.vault.join("source.md"), &source).unwrap();
    fs::write(f.vault.join("a-healthy.md"), "# No citations\n").unwrap();
    fs::write(
        f.vault.join("z-review.md"),
        "# Consumer\r\nOwner preserved text õ\r\n",
    )
    .unwrap();
    let inventory = ok(f.run(&["identity", "inventory"]));
    let hash = inventory["notes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["path"] == "source.md")
        .unwrap()["sha256"]
        .clone();
    let quote = "Exact õ 日本語 🦀 quote\r\n";
    let start = source.find(quote).unwrap();
    // Pure portable fixture construction: capture/preparation commands have a
    // macOS Save dependency; this test qualifies read-only review on Unix.
    let citation = serde_json::from_value::<brn_store::note_provenance::VaultCitation>(json!({
        "note_id": source_id, "sha256": hash, "start_byte": start,
        "end_byte": start + quote.len(), "quote": quote
    }))
    .unwrap();
    let consumer =
        brn_store::note_provenance::write("# Consumer\r\nOwner preserved text õ\r\n", &[citation])
            .unwrap();
    fs::write(f.vault.join("z-review.md"), &consumer).unwrap();
    fs::write(
        f.vault.join("0-malformed-intake.md"),
        "---\nbrn_inbox_source: invalid\n---\n# Unavailable metadata\n",
    )
    .unwrap();
    // Synthetic owner file edit changes source evidence after the durable quote was captured.
    fs::write(f.vault.join("source.md"), source.replace("Exact", "Other")).unwrap();
    let first = ok(f.run(&["needs-review", "citations", "--limit", "1"]));
    assert_eq!(first["entries"], json!([]));
    assert_eq!(first["inspected_count"], 1);
    assert_eq!(first["coverage"]["incomplete"], true);
    assert_eq!(first["coverage"]["diagnostic_count"], 1);
    assert_eq!(
        first["coverage"]["diagnostics"][0]["path"],
        "0-malformed-intake.md"
    );
    assert!(first["next_cursor"].is_object());
    fs::write(&f.input, serde_json::to_vec(&first["next_cursor"]).unwrap()).unwrap();
    let second = ok(f.run(&[
        "needs-review",
        "citations",
        "--limit",
        "1",
        "--cursor",
        f.input.to_str().unwrap(),
    ]));
    assert_eq!(second["entries"][0]["path"], "z-review.md");
    assert_eq!(second["entries"][0]["citations"][0]["outcome"], "changed");
    assert_eq!(second["next_cursor"], Value::Null);
    let request = json!({"path":"z-review.md","expected_sha256":second["entries"][0]["sha256"]});
    let detail = ok(f.input("needs-review", "show", &request));
    assert_eq!(detail["text"], consumer);
    assert_eq!(
        detail["provenance"]["citations"][0]["citation"]["quote"],
        quote
    );
    assert_eq!(detail["provenance"]["citations"][0]["outcome"], "changed");
    fs::remove_file(f.data.path().join("index.sqlite")).unwrap();
    assert_eq!(ok(f.input("needs-review", "show", &request)), detail);
    // Cursor/source digest and detail/consumer hash fence independent fresh processes.
    fs::write(f.vault.join("source.md"), &source).unwrap();
    fs::write(&f.input, serde_json::to_vec(&first["next_cursor"]).unwrap()).unwrap();
    assert_eq!(
        f.run(&[
            "needs-review",
            "citations",
            "--cursor",
            f.input.to_str().unwrap()
        ])
        .1["error"]["code"],
        "CONTEXT_STALE"
    );
    fs::write(
        f.vault.join("z-review.md"),
        format!("{consumer}Owner later edit\n"),
    )
    .unwrap();
    assert_eq!(
        f.input("needs-review", "show", &request).1["error"]["code"],
        "CONTEXT_STALE"
    );
    assert_eq!(ok(f.run(&["proposals", "list"])), json!([]));
    assert_eq!(ok(f.run(&["findings", "list"]))["entries"], json!([]));
    assert_eq!(ok(f.run(&["edit", "list"])), json!([]));
    assert_eq!(
        fs::read_to_string(f.vault.join("source.md")).unwrap(),
        source
    );
}
