//! Exact stable-link review preparation through real synthetic CLI processes.
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};
use uuid::Uuid;

mod support;

struct Fixture {
    data: support::DataDir,
    vault: PathBuf,
    credentials: PathBuf,
    input: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let data = support::data_dir();
        let parent = data.path().parent().unwrap();
        let vault = parent.join("vault");
        fs::create_dir(&vault).unwrap();
        Self {
            vault,
            credentials: parent.join("task.credentials"),
            input: parent.join("link.json"),
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
                .unwrap_or_else(|_| panic!("missing typed envelope: {output:?}")),
        )
    }
    #[cfg(target_os = "macos")]
    fn ok(&self, args: &[&str], name: &str) -> Value {
        let (code, envelope) = self.run(args);
        assert_eq!(code, 0, "{args:?}: {envelope}");
        assert_eq!(envelope["schema_version"], 1);
        assert_eq!(envelope["command"], name);
        assert_eq!(envelope["ok"], true);
        envelope["data"].clone()
    }
    fn prepare(&self, request: &Value) -> (i32, Value) {
        fs::write(&self.input, serde_json::to_vec(request).unwrap()).unwrap();
        self.run(&["links", "prepare", "--file", self.input.to_str().unwrap()])
    }
    fn assert_unopened(&self) {
        assert_eq!(fs::read_dir(self.data.path()).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&self.vault).unwrap().count(), 0);
        assert!(!self.credentials.exists());
    }
}
fn request() -> Value {
    json!({"path":"current.md","target_note_id":Uuid::new_v4(),
        "expected_target_sha256":vec![17;32],"proposal_id":Uuid::new_v4(),
        "title":"Connect source evidence", "label":"Original õ"})
}

#[test]
fn malformed_link_preparation_refuses_before_operational_or_credential_startup() {
    let f = Fixture::new();
    for args in [
        vec!["links", "prepare"],
        vec!["links", "prepare", "extra", "--file", "missing.json"],
        vec!["links", "prepare", "--file", "one", "--file", "two"],
        vec!["links", "prepare", "--target", "bad"],
    ] {
        let (code, envelope) = f.run(&args);
        assert_eq!(code, 2, "{envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        f.assert_unopened();
    }
    for (key, value) in [
        ("path", json!("../outside.md")),
        ("path", json!("archive/current.md")),
        ("target_note_id", json!(Uuid::nil())),
        ("expected_target_sha256", json!(vec![17; 31])),
        ("expected_target_sha256", Value::Null),
        ("proposal_id", json!(Uuid::nil())),
        ("label", json!("")),
        ("label", json!("new\nline")),
        ("label", json!("õ".repeat(257))),
        ("title", json!(" ")),
        ("extra", json!(true)),
    ] {
        let mut input = request();
        input[key] = value;
        let (code, envelope) = f.prepare(&input);
        assert_eq!(code, 2, "{input}: {envelope}");
        assert_eq!(envelope["command"], "links.prepare");
        assert_eq!(envelope["error"]["code"], "USAGE");
        f.assert_unopened();
    }
    let id = Uuid::new_v4();
    let duplicate = format!("{{\"path\":\"current.md\",\"path\":\"current.md\",\"target_note_id\":\"{id}\",\"expected_target_sha256\":{},\"proposal_id\":\"{id}\",\"title\":\"Review\",\"label\":\"Source\"}}", json!(vec![17;32]));
    for bytes in [
        b"{".as_slice(),
        b"\xff".as_slice(),
        duplicate.as_bytes(),
        b"null".as_slice(),
    ] {
        fs::write(&f.input, bytes).unwrap();
        assert_eq!(
            f.run(&["links", "prepare", "--file", f.input.to_str().unwrap()])
                .0,
            2
        );
        f.assert_unopened();
    }
    fs::File::create(&f.input)
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    assert_eq!(
        f.run(&["links", "prepare", "--file", f.input.to_str().unwrap()])
            .0,
        2
    );
    f.assert_unopened();
}

#[test]
fn nonregular_link_request_refuses_without_waiting_for_a_fifo_writer() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let path = CString::new(f.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: a NUL-terminated path in this exclusively owned synthetic fixture.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["links", "prepare", "--file"])
        .arg(&f.input)
        .args(["--json", "--data-dir"])
        .arg(f.data.path())
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
    assert!(!blocked, "link input waited for a FIFO writer");
    assert_eq!(output.status.code(), Some(2));
    f.assert_unopened();
}

#[cfg(target_os = "macos")]
#[test]
fn archived_target_preparation_preserves_exact_prefix_and_uses_ordinary_approval() {
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let source = format!("\u{feff}---\r\nbrn_id: {source_id}\r\ncustom: unchanged õ\r\n---\r\n# Consumer\r\nOriginal bytes without final newline");
    let target = format!(
        "\u{feff}---\r\nbrn_id: {target_id}\r\nbrn_kind: source\r\n---\r\nOriginal target õ\r\n"
    );
    fs::write(f.vault.join("current.md"), &source).unwrap();
    fs::create_dir(f.vault.join("archive")).unwrap();
    fs::write(f.vault.join("archive/target.md"), &target).unwrap();
    let inventory = f.ok(&["identity", "inventory"], "identity.inventory");
    let hash = inventory["notes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|note| note["path"] == "archive/target.md")
        .unwrap()["sha256"]
        .clone();
    let proposal = Uuid::new_v4();
    let input = json!({"path":"current.md","target_note_id":target_id,
        "expected_target_sha256":hash,"proposal_id":proposal,
        "title":"Connect archived evidence", "label":"Õ [õ] *source* \\ &"});
    let (code, envelope) = f.prepare(&input);
    assert_eq!(code, 0, "{envelope}");
    assert_eq!(envelope["command"], "links.prepare");
    let draft = &envelope["data"];
    assert_eq!(draft["id"], proposal.to_string());
    assert_eq!(draft["title"], input["title"]);
    let change = &draft["changes"][0];
    assert_eq!(change["kind"], "replace");
    assert_eq!(change["path"], "current.md");
    let proposed = change["text"].as_str().unwrap();
    assert!(proposed.starts_with(&source));
    assert!(
        proposed.contains(&format!(
            "[Õ \\[õ\\] \\*source\\* \\\\ \\&](brn://note/{target_id})"
        )),
        "{proposed:?}"
    );
    let proofs = draft["sources"].as_array().unwrap();
    assert_eq!(proofs.len(), 2);
    let before = proofs
        .iter()
        .find(|proof| proof["path"] == "current.md")
        .unwrap();
    assert_eq!(before["fingerprint"], change["expected"]);
    assert_eq!(
        proofs
            .iter()
            .find(|proof| proof["path"] == "archive/target.md")
            .unwrap()["fingerprint"]["sha256"],
        hash
    );
    assert_eq!(
        fs::read(f.vault.join("current.md")).unwrap(),
        source.as_bytes()
    );
    assert_eq!(
        f.ok(&["links", "show", "current.md"], "links.show")["links"],
        json!([])
    );
    assert_eq!(f.ok(&["proposals", "list"], "proposals.list"), json!([]));
    assert_eq!(f.ok(&["edit", "list"], "edit.list"), json!([]));
    let plain = f.process(
        &["links", "prepare", "--file", f.input.to_str().unwrap()],
        false,
    );
    assert!(plain.status.success(), "{plain:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&plain.stdout).unwrap(),
        *draft
    );

    fs::write(&f.input, serde_json::to_vec(draft).unwrap()).unwrap();
    let created = f.ok(
        &["proposals", "create", "--file", f.input.to_str().unwrap()],
        "proposals.create",
    );
    assert_eq!(created["version"], 1);
    assert_eq!(created["draft"]["changes"][0]["before_text"], source);
    let operation = Uuid::new_v4();
    let receipt = f.ok(
        &[
            "proposals",
            "approve",
            &proposal.to_string(),
            "--review-version",
            "1",
            "--operation",
            &operation.to_string(),
        ],
        "proposals.approve",
    );
    assert_eq!(receipt["outcome"], "applied");
    let shown = f.ok(&["links", "show", "current.md"], "links.show");
    assert_eq!(shown["links"].as_array().unwrap().len(), 1);
    assert_eq!(
        shown["links"][0]["destination"],
        format!("brn://note/{target_id}")
    );
    assert_eq!(shown["links"][0]["outcome"], "resolved");
    assert_eq!(shown["links"][0]["target_path"], "archive/target.md");
    fs::remove_file(f.data.path().join("index.sqlite")).unwrap();
    assert_eq!(f.ok(&["links", "show", "current.md"], "links.show"), shown);
    assert_eq!(
        fs::read(f.vault.join("current.md")).unwrap(),
        proposed.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("archive/target.md")).unwrap(),
        target.as_bytes()
    );
}

#[cfg(target_os = "macos")]
#[test]
fn changed_selected_hash_and_new_uuid_alias_refuse_without_admitting_review() {
    let f = Fixture::new();
    let source_id = Uuid::new_v4();
    let target_id = Uuid::new_v4();
    let source = format!("---\nbrn_id: {source_id}\n---\nConsumer õ\n");
    let target = format!("---\nbrn_id: {target_id}\n---\nOriginal target\n");
    fs::write(f.vault.join("current.md"), &source).unwrap();
    fs::write(f.vault.join("target.md"), &target).unwrap();
    let inventory = f.ok(&["identity", "inventory"], "identity.inventory");
    let hash = inventory["notes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|note| note["path"] == "target.md")
        .unwrap()["sha256"]
        .clone();
    let input = json!({"path":"current.md","target_note_id":target_id,
        "expected_target_sha256":hash,"proposal_id":Uuid::new_v4(),"title":"Connect evidence","label":"Source"});
    fs::write(
        f.vault.join("target.md"),
        target.replace("Original", "Changed"),
    )
    .unwrap();
    let stale = f.prepare(&input);
    assert_eq!(stale.0, 1, "{}", stale.1);
    assert_eq!(stale.1["error"]["code"], "CONTEXT_STALE");
    fs::write(f.vault.join("target.md"), &target).unwrap();
    fs::write(f.vault.join("alias.md"), &target).unwrap();
    let alias = f.prepare(&input);
    assert_eq!(alias.0, 1, "{}", alias.1);
    assert_eq!(alias.1["ok"], false);
    assert_eq!(f.ok(&["proposals", "list"], "proposals.list"), json!([]));
    assert_eq!(
        fs::read(f.vault.join("current.md")).unwrap(),
        source.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("target.md")).unwrap(),
        target.as_bytes()
    );
    assert_eq!(
        fs::read(f.vault.join("alias.md")).unwrap(),
        target.as_bytes()
    );
}
