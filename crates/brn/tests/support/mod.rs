//! Shared helpers for the round-2 process tests: a private root, the built
//! `brn` (or the baseline binary named by `AGENT_CONTRACT_BASELINE_BIN`, read by
//! the test process only and never passed to children), and envelope checks.
#![allow(dead_code)]
#[cfg(unix)]
pub mod stub;
use serde_json::{json, Value};
use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};

const BASELINE: &str = "AGENT_CONTRACT_BASELINE_BIN";
/// A `fb1b2100` (schema 5) `brn`, for building older-schema fixtures through
/// documented commands. Read by the test process only, never passed on.
const SCHEMA5: &str = "AGENT_CONTRACT_SCHEMA5_BIN";

/// The schema-5 binary, when `AGENT_CONTRACT_SCHEMA5_BIN` names one.
pub fn schema5_bin() -> Option<PathBuf> {
    std::env::var_os(SCHEMA5).map(PathBuf::from)
}

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
    /// Extra variables for every child (the stub's proxy settings).
    pub env: Vec<(String, String)>,
}
impl Root {
    pub fn new() -> Self {
        let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let path = dir.path().canonicalize().unwrap();
        for sub in ["home", "tmp"] {
            std::fs::create_dir(path.join(sub)).unwrap();
        }
        Self {
            _dir: dir,
            path,
            env: vec![],
        }
    }
    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
    /// A fresh test workspace (`init --purpose test`) at `name`: the folder
    /// every writing test uses. Only test workspaces keep full CLI authority
    /// once owner workspaces take proposals (B1 phase 2).
    pub fn workspace(&self, name: &str) -> PathBuf {
        let data = self.join(name);
        let run = run(self, true, &data, &["init", "--purpose", "test"]);
        assert_eq!(run.code, 0, "init --purpose test: {}", run.stdout);
        data
    }
    pub fn file(&self, name: &str, bytes: &[u8]) -> String {
        let path = self.join(name);
        std::fs::write(&path, bytes).unwrap();
        path.to_str().unwrap().to_owned()
    }
    /// A child with a clean environment: every `BRN_*` removed, homes and temp inside the root.
    pub fn command(&self, args: &[&OsStr]) -> Command {
        self.command_with(&brn(), args)
    }
    /// `command` for another `brn` binary (the schema-5 fixture builder).
    pub fn command_with(&self, binary: &Path, args: &[&OsStr]) -> Command {
        let mut command = Command::new(binary);
        for (key, _) in std::env::vars_os() {
            let key = key.to_string_lossy();
            if key.starts_with("BRN_") || key == BASELINE || key == SCHEMA5 {
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
            .envs(self.env.iter().map(|(k, v)| (k, v)))
            // A relative path a binary accepts (the baseline did) lands in the root.
            .current_dir(&self.path)
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
            assert_eq!(value["contract"], 3, "{value}");
            assert_eq!(value.as_object().unwrap().len(), 16, "{value}");
            assert_eq!(value["authority"]["enforced"], false, "{value}");
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
/// `begin-edit --token-out FILE`: the session and the token file's path.
pub fn begin_edit(root: &Root, data: &Path, note: &str, base: u64) -> (Value, String) {
    let token = root.join(&format!("token-{}", uuid_like()));
    let token = token.to_str().unwrap().to_owned();
    let session = ok(
        root,
        data,
        &[
            "begin-edit",
            "--note",
            note,
            "--base",
            &base.to_string(),
            "--token-out",
            &token,
        ],
    );
    assert_eq!(session["token"], Value::Null, "{session}");
    assert_eq!(session["holder"], "caller", "{session}");
    (session, token)
}
/// A name unique within this test process.
pub fn uuid_like() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    )
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
/// Makes a child's signal delivery independent of the parent shell: its own
/// process group (a terminal Ctrl-C reaches only the test), and SIGINT and
/// SIGTERM at their default disposition and unblocked, whatever the parent
/// inherited (some agent shells block SIGINT, and the mask survives exec).
/// Tests signal the child directly with `kill(child.id(), …)`.
pub fn isolate(command: &mut Command) -> &mut Command {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
    // SAFETY: the hook runs in the forked child before exec and calls only
    // async-signal-safe functions (sigemptyset, sigaddset, sigprocmask, signal).
    unsafe {
        command.pre_exec(|| {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGINT);
            libc::sigaddset(&mut set, libc::SIGTERM);
            libc::sigprocmask(libc::SIG_UNBLOCK, &set, std::ptr::null_mut());
            libc::signal(libc::SIGINT, libc::SIG_DFL);
            libc::signal(libc::SIGTERM, libc::SIG_DFL);
            Ok(())
        });
    }
    command
}
/// Starts `argv` with piped stdin, stdout and stderr; finish with `wait_output`.
/// The child is isolated (see `isolate`) so signal tests pass from any shell.
pub fn spawn(root: &Root, argv: &[&str]) -> Child {
    let args: Vec<&OsStr> = argv.iter().map(OsStr::new).collect();
    let mut command = root.command(&args);
    isolate(&mut command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}
/// Sends `signal` to a child this test spawned.
pub fn signal(child: &Child, signal: i32) {
    // SAFETY: signals only the child this test spawned.
    assert_eq!(unsafe { libc::kill(child.id() as i32, signal) }, 0);
}
/// A FIFO in the root. `ready_writer` waits until a reader (the child) has it
/// open, which proves the child is past startup, then returns the writer.
pub struct Fifo {
    pub path: PathBuf,
}
impl Fifo {
    pub fn new(root: &Root, name: &str) -> Self {
        let path = root.join(name);
        let c = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        // SAFETY: creates a FIFO inside the test root.
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
        Self { path }
    }
    pub fn arg(&self) -> &str {
        self.path.to_str().unwrap()
    }
    /// Opens the write end once the child has opened the read end (within 20 s).
    pub fn ready_writer(&self) -> std::fs::File {
        use std::os::unix::fs::OpenOptionsExt;
        let started = std::time::Instant::now();
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&self.path)
            {
                Ok(file) => return file,
                Err(_) if started.elapsed() < Duration::from_secs(20) => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("the child never opened the FIFO: {error}"),
            }
        }
    }
}
