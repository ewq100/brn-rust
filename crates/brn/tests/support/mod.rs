//! Shared helpers for the round-2 process tests: a private root, the built
//! `brn` (or the baseline binary named by `AGENT_CONTRACT_BASELINE_BIN`, read by
//! the test process only and never passed to children), and envelope checks.
#![allow(dead_code)]
use serde_json::{json, Value};
use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

const BASELINE: &str = "AGENT_CONTRACT_BASELINE_BIN";

/// True when the tests run against the pre-round-2 baseline binary.
pub fn baseline() -> bool {
    std::env::var_os(BASELINE).is_some()
}
pub fn brn() -> PathBuf {
    std::env::var_os(BASELINE)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_brn")))
}

/// A private test root; children get HOME, CODEX_HOME and temp dirs inside it.
pub struct Root {
    _dir: tempfile::TempDir,
    pub path: PathBuf,
}
impl Root {
    pub fn new() -> Self {
        let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let path = dir.path().canonicalize().unwrap();
        for sub in ["home", "tmp"] {
            std::fs::create_dir(path.join(sub)).unwrap();
        }
        Self { _dir: dir, path }
    }
    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
    pub fn file(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.join(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_str().unwrap().to_owned()
    }
    /// A child with a clean environment: every `BRN_*` removed, homes and temp inside the root.
    pub fn command(&self, args: &[&OsStr]) -> Command {
        let mut command = Command::new(brn());
        for (key, _) in std::env::vars_os() {
            let key = key.to_string_lossy();
            if key.starts_with("BRN_") || key == BASELINE {
                command.env_remove(&*key);
            }
        }
        let tmp = self.join("tmp");
        command
            .env("HOME", self.join("home"))
            .env("CODEX_HOME", self.join("home"))
            .env("TMPDIR", &tmp)
            .env("TMP", &tmp)
            .env("TEMP", &tmp)
            .args(args);
        command
    }
}

pub struct Run {
    pub code: i32,
    /// The terminating signal, if the child was killed by one.
    pub signal: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}
impl Run {
    /// The one envelope line, with the contract-2 shape checked (skipped on the baseline).
    pub fn envelope(&self) -> Value {
        assert_eq!(
            self.stdout.matches('\n').count(),
            1,
            "one line: {:?} (stderr {:?})",
            self.stdout,
            self.stderr
        );
        let value: Value = serde_json::from_str(&self.stdout).unwrap();
        if !baseline() {
            assert_eq!(value["contract"], 2, "{value}");
            assert_eq!(value.as_object().unwrap().len(), 15, "{value}");
        }
        assert_eq!(value["exit"], self.code, "{value}");
        assert_eq!(value["ok"], self.code == 0);
        assert!(!value["retry"].is_null());
        assert!(!self.stderr.contains("panicked"), "{}", self.stderr);
        value
    }
}

pub fn wait_output(child: Child, limit: Duration) -> Run {
    let id = child.id();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    let output = match receiver.recv_timeout(limit) {
        Ok(output) => output.unwrap(),
        Err(_) => {
            // SAFETY: signals only the child this test spawned.
            unsafe { libc::kill(id as i32, libc::SIGKILL) };
            panic!("brn did not finish within {limit:?}");
        }
    };
    use std::os::unix::process::ExitStatusExt;
    Run {
        code: output.status.code().unwrap_or(-1),
        signal: output.status.signal(),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Runs `argv` (after the program name) to completion within 60 s.
pub fn run_argv(root: &Root, argv: &[&str], stdin: Option<&[u8]>) -> Run {
    let args: Vec<&OsStr> = argv.iter().map(OsStr::new).collect();
    let mut command = root.command(&args);
    command
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(bytes) = stdin {
        let mut input = child.stdin.take().unwrap();
        let bytes = bytes.to_vec();
        std::thread::spawn(move || {
            let _ = input.write_all(&bytes);
        });
    }
    wait_output(child, Duration::from_secs(60))
}
pub fn argv<'a>(json: bool, data: &'a Path, rest: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![];
    if json {
        args.push("--json");
    }
    args.push("--data-dir");
    args.push(data.to_str().unwrap());
    args.extend_from_slice(rest);
    args
}
pub fn run(root: &Root, json: bool, data: &Path, rest: &[&str]) -> Run {
    run_argv(root, &argv(json, data, rest), None)
}
/// One machine-mode call: the envelope.
pub fn call(root: &Root, data: &Path, rest: &[&str]) -> Value {
    run(root, true, data, rest).envelope()
}
/// A machine-mode call that must succeed: the result.
pub fn ok(root: &Root, data: &Path, rest: &[&str]) -> Value {
    let envelope = call(root, data, rest);
    assert_eq!(envelope["status"], "ok", "{rest:?}: {envelope}");
    envelope["result"].clone()
}
/// A default-mode call that must exit 0: stdout parsed as JSON.
pub fn plain(root: &Root, data: &Path, rest: &[&str]) -> Value {
    let run = run(root, false, data, rest);
    assert_eq!(run.code, 0, "{rest:?}: {}", run.stderr);
    serde_json::from_str(&run.stdout).unwrap()
}
pub fn id(value: &Value) -> String {
    value["id"].as_str().unwrap().to_owned()
}
pub fn note(root: &Root, data: &Path, title: &str, text: &str) -> String {
    id(&ok(
        root,
        data,
        &["note-create", "--title", title, "--text", text],
    ))
}
pub fn assert_error(envelope: &Value, status: &str, code: &str) {
    assert_eq!(envelope["status"], status, "{envelope}");
    assert_eq!(envelope["error"]["code"], code, "{envelope}");
    assert_eq!(envelope["result"], Value::Null);
    assert_eq!(envelope["outcome"], Value::Null);
    assert_eq!(envelope["changed"], json!([]));
    if matches!(status, "usage" | "rejected") {
        assert_eq!(envelope["retry"], "refuse", "{envelope}");
    }
}
/// The number of receipts in history (unbounded default form).
pub fn receipts(root: &Root, data: &Path) -> usize {
    ok(root, data, &["history"]).as_array().unwrap().len()
}
