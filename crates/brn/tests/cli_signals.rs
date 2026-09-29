//! Subprocess tests for signal/pipe robustness: a broken provider stdin pipe
//! must be a recoverable workflow error (not a process-wide SIGPIPE death) and
//! must still reap the provider child; closed stdout/stderr consumers must not
//! panic or corrupt a completed outcome. The fake provider is a python script
//! whose mode is selected by executable-name suffix (pattern from
//! tests/cli_ask.rs, kept self-contained here). Workspaces are disposable
//! temp dirs; CODEX_HOME is redirected to a synthetic home.
use brn_workflow::{Config, SearchApproval, Workspace};
use serde_json::Value;
use std::{
    fs,
    io::Read,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
use tempfile::tempdir;
use uuid::Uuid;

const FIXTURE: &str =
    "The synthetic Aurora mission launches on Tuesday. Its crew includes Mira and Niko.\r\n";
const QUESTION: &str = "When does the Aurora mission launch?";

/// Write the fake provider executable for `mode` (success|close-stdin).
/// `close-stdin` answers `initialize`, then closes its stdin read end
/// (`os.close(0)` — `sys.stdin.close()` does not close the fd), writes the
/// `../stdin-closed.log` marker and sleeps forever: brn's next protocol write
/// hits a broken pipe.
fn fake(dir: &Path, mode: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let exe = dir.join(format!("fake-{mode}"));
    let body = if mode == "close-stdin" {
        r#"#!/usr/bin/env python3
import json,os,sys,time
line=sys.stdin.readline()
q=json.loads(line)
if q.get('method')=='initialize':
 sys.stdout.write(json.dumps({'id':q['id'],'result':{'userAgent':'codex/0.155.0-alpha.16.4','codexHome':os.environ.get('CODEX_HOME','/tmp/fake'),'platformFamily':'unix','platformOs':'macos'}})+'\n')
 sys.stdout.flush()
os.close(0)
open('../stdin-closed.log','w').write('closed\n')
time.sleep(300)
"#
    } else {
        r#"#!/usr/bin/env python3
import json,sys,os,sqlite3
if '--version' in sys.argv:
 print('codex-cli 0.155.0-alpha.16.4');sys.exit(0)
def send(x):print(json.dumps(x),flush=True)
for line in sys.stdin:
 q=json.loads(line);m=q.get('method');rid=q.get('id')
 if m=='initialize':send({'id':rid,'result':{'userAgent':'codex/0.155.0-alpha.16.4','codexHome':os.environ.get('CODEX_HOME','/tmp/fake'),'platformFamily':'unix','platformOs':'macos'}})
 elif m=='account/read':send({'id':rid,'result':{'account':{'type':'chatgpt'},'workspaceRouting':{'chatgptAccountId':'synthetic-account'}}})
 elif m in ['thread/start','thread/resume']:
  tid=q.get('params',{}).get('threadId','thr_synthetic');send({'id':rid,'result':{'thread':{'id':tid,'sessionId':tid}}})
 elif m=='turn/start':
  tid=q['params']['threadId'];db=sqlite3.connect('file:../brn.sqlite3?mode=ro',uri=True)
  assert db.execute('SELECT count(*) FROM sessions WHERE thread_id=?',(tid,)).fetchone()[0]==1
  assert db.execute("SELECT count(*) FROM chat_turns t JOIN operations o ON t.operation_id=o.id WHERE o.status='pending'").fetchone()[0]==1
  assert 'SOURCE [1]' in q['params']['input'][0]['text']
  with open('../submitted.log','a') as f:f.write('turn\n')
  send({'id':rid,'result':{'turn':{'id':'turn_synthetic'}}})
  send({'method':'item/agentMessage/delta','params':{'threadId':tid,'turnId':'turn_synthetic','delta':'Aurora launches Tuesday [1].'}})
  send({'method':'turn/completed','params':{'threadId':tid,'turn':{'id':'turn_synthetic','status':'completed'}}})
"#
    };
    fs::write(&exe, format!("{}\n", body.trim_end())).unwrap();
    fs::set_permissions(&exe, fs::Permissions::from_mode(0o700)).unwrap();
    exe
}

/// Seed one approved source and build the index through the in-process
/// workflow, so the tests exercise the CLI subprocess against a ready store.
fn seed(root: &Path) {
    let file = root.join("launch.md");
    fs::write(&file, FIXTURE).unwrap();
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    workspace
        .import_file(Uuid::new_v4(), &file, SearchApproval::Approved)
        .unwrap();
    workspace
        .build_index(&AtomicBool::new(false), |_| {})
        .unwrap();
}

/// Block until `name` appears under `root` (bounded polling, no fixed sleeps).
fn wait_for_log(root: &Path, name: &str) {
    let log = root.join(name);
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if log.exists() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("{name} never appeared: {}", log.display());
}

/// Poll until no process matches the fake executable path any more.
fn assert_provider_gone(exe: &Path) {
    let needle = exe.to_str().unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let out = Command::new("pgrep")
            .arg("-f")
            .arg(needle)
            .output()
            .expect("pgrep runs");
        if out.stdout.is_empty() {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("provider child still running: {needle}");
}

/// Spawn brn with all three stdio pipes and synthetic CODEX_HOME.
fn spawn_brn(root: &Path, args: &[&str]) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .arg("--data-dir")
        .arg(root)
        .arg("--json")
        .env("CODEX_HOME", root.join("synthetic-home"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("brn binary spawns")
}

/// A broken provider stdin pipe is a recoverable workflow error, not a
/// SIGPIPE death, and the provider child is reaped.
#[test]
fn broken_provider_pipe_is_recoverable_and_reaps_child() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "close-stdin");
    let child = spawn_brn(root, &["ask", QUESTION, "--codex", exe.to_str().unwrap()]);
    wait_for_log(root, "stdin-closed.log");
    let out = child.wait_with_output().expect("brn ask waits");
    assert_eq!(
        out.status.code(),
        Some(1),
        "brn exits with a structured error, not a signal"
    );
    assert!(
        out.status.signal().is_none(),
        "brn must not die by signal, got {:?}",
        out.status.signal()
    );
    let envelope: Value =
        serde_json::from_str(&String::from_utf8(out.stdout).expect("stdout is UTF-8"))
            .expect("exactly one JSON envelope on stdout");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR", "{envelope}");
    assert_provider_gone(&exe);
}

/// A consumer that closed stdout before the envelope is written gets quiet
/// success: write errors are not a domain failure.
#[test]
fn closed_stdout_consumer_is_quiet_success() {
    let dir = tempdir().unwrap();
    let data = dir.path().join("status-data");
    fs::create_dir_all(&data).unwrap();
    let mut child = spawn_brn(&data, &["status"]);
    drop(child.stdout.take()); // close the read end: envelope write hits EPIPE
    let out = child.wait_with_output().expect("brn status waits");
    assert_eq!(out.status.code(), Some(0), "completed work still exits 0");
    assert!(
        !String::from_utf8_lossy(&out.stderr).contains("panic"),
        "no panic on closed stdout"
    );

    let mut help = Command::new(env!("CARGO_BIN_EXE_brn"))
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("brn --help spawns");
    drop(help.stdout.take());
    let out = help.wait_with_output().expect("brn --help waits");
    assert_eq!(out.status.code(), Some(0), "--help with closed stdout");
}

/// Closed stderr (delta stream) must not break the ask lifecycle: the turn
/// still completes and stdout carries the success envelope.
#[test]
fn closed_stderr_does_not_break_ask_lifecycle() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "success");
    let mut child = spawn_brn(root, &["ask", QUESTION, "--codex", exe.to_str().unwrap()]);
    drop(child.stderr.take()); // delta writes hit EPIPE
    let mut stdout = child.stdout.take().expect("stdout piped");
    let mut buf = String::new();
    stdout.read_to_string(&mut buf).expect("stdout reads");
    let status = child.wait().expect("brn ask waits");
    assert_eq!(status.code(), Some(0), "stdout: {buf}");
    let envelope: Value = serde_json::from_str(&buf).expect("exactly one JSON envelope");
    assert_eq!(envelope["ok"], true, "{envelope}");
    assert_eq!(envelope["data"]["status"], "completed", "{envelope}");
}
