//! Explicit raw saved evidence through fresh owner CLI processes, synthetic only.
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

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
fn raw_input_validation_leaves_workspace_and_credentials_unopened() {
    let f = Fixture::new();
    for args in [
        vec!["evidence", "read-raw"],
        vec!["evidence", "read-raw", "extra"],
        vec!["evidence", "read-raw", "--unknown", "x"],
    ] {
        assert_eq!(f.run(&args).0, 2);
        f.assert_unopened();
    }
    for value in [
        json!({}),
        json!({"path":"../outside.md"}),
        json!({"path":".hidden.md"}),
        json!({"path":"/absolute.md"}),
        json!({"path":"note.md","extra":true}),
        json!({"path":"note.md","start_byte":1}),
        json!({"path":"note.md","end_byte":0}),
        json!({"path":"note.md","expected_sha256":vec![0;31]}),
        json!({"path":"note.md","expected_sha256":vec![0;32],"end_byte":50001}),
    ] {
        let result = f.input("evidence", "read-raw", &value);
        assert_eq!(result.0, 2, "{value}: {}", result.1);
        assert_eq!(result.1["error"]["code"], "USAGE");
        f.assert_unopened();
    }
    fs::File::create(&f.input).unwrap().set_len(65537).unwrap();
    assert_eq!(
        f.run(&["evidence", "read-raw", "--file", f.input.to_str().unwrap()])
            .0,
        2
    );
    f.assert_unopened();
}

#[test]
fn raw_prefix_hash_bound_tail_restart_and_index_loss_preserve_unclassified_bytes() {
    let f = Fixture::new();
    let mut text = "\u{feff}---\r\nbrn_id: invalid\r\n---\r\n# Unclassified\r\n".to_owned();
    text.push_str(&"x".repeat(49_999 - text.len()));
    text.push('🦀');
    text.push_str(&"x".repeat(949_000 - text.len()));
    text.push_str("\r\nTAIL õ 日本語 🦀\r\n");
    fs::write(f.vault.join("raw.md"), &text).unwrap();
    let prefix = ok(f.input("evidence", "read-raw", &json!({"path":"raw.md"})));
    assert_eq!(prefix["facts"], Value::Null);
    assert!(!prefix["metadata_issue"].as_str().unwrap().is_empty());
    assert_eq!(prefix["partial"], true);
    assert_eq!(prefix["total_bytes"], text.len());
    assert_eq!(prefix["end_byte"], 49_999);
    assert_eq!(prefix["text"], &text[..49_999]);
    let start = text.find("TAIL").unwrap();
    let request = json!({"path":"raw.md","start_byte":start,"end_byte":text.len(),"expected_sha256":prefix["sha256"]});
    let tail = ok(f.input("evidence", "read-raw", &request));
    assert_eq!(tail["text"], &text[start..]);
    assert_eq!(tail["sha256"], prefix["sha256"]);
    assert_eq!(tail["facts"], Value::Null);
    fs::remove_file(f.data.path().join("index.sqlite")).unwrap();
    assert_eq!(ok(f.input("evidence", "read-raw", &request)), tail);
    let empty=ok(f.input("evidence","read-raw",&json!({"path":"raw.md","start_byte":text.len(),"end_byte":text.len(),"expected_sha256":prefix["sha256"]})));
    assert_eq!(empty["text"], "");
    assert_ne!(f.run(&["notes", "show", "raw.md", "--scope", "all"]).0, 0);
    assert_eq!(fs::read_to_string(f.vault.join("raw.md")).unwrap(), text);
    let modified = fs::metadata(f.vault.join("raw.md"))
        .unwrap()
        .modified()
        .unwrap();
    let changed = text.replacen("xxxxx", "yyyyy", 1);
    fs::write(f.vault.join("raw.md"), &changed).unwrap();
    fs::File::options()
        .write(true)
        .open(f.vault.join("raw.md"))
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let refused = f.input("evidence", "read-raw", &request);
    assert_eq!(refused.0, 1);
    assert!(refused.1["error"]["code"]
        .as_str()
        .unwrap()
        .contains("STALE"));
    assert_eq!(ok(f.run(&["proposals", "list"])), json!([]));
    assert_eq!(ok(f.run(&["findings", "list"]))["entries"], json!([]));
    assert_eq!(ok(f.run(&["actions", "list"]))["entries"], json!([]));
    assert_eq!(fs::read_to_string(f.vault.join("raw.md")).unwrap(), changed);
}

#[cfg(unix)]
#[test]
fn raw_request_fifo_refuses_without_waiting_and_vault_symlink_is_not_read() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        process::Stdio,
        time::{Duration, Instant},
    };
    let f = Fixture::new();
    let path = CString::new(f.input.as_os_str().as_bytes()).unwrap();
    // SAFETY: exclusively owned synthetic NUL-terminated temporary path.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let mut child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["evidence", "read-raw", "--file"])
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
    assert!(!blocked);
    assert_eq!(output.status.code(), Some(2));
    f.assert_unopened();
    fs::remove_file(&f.input).unwrap();
    fs::write(f.vault.join("valid.md"), "# Original\n").unwrap();
    std::os::unix::fs::symlink(f.vault.join("valid.md"), f.vault.join("link.md")).unwrap();
    assert_eq!(
        f.input("evidence", "read-raw", &json!({"path":"link.md"}))
            .0,
        1
    );
    assert_eq!(
        fs::read_to_string(f.vault.join("valid.md")).unwrap(),
        "# Original\n"
    );
}
