//! Subprocess tests for the Phase D provider/session commands: `ask`,
//! `conversations list/show`. The provider is a fake Codex app-server (a
//! python script whose mode is selected by executable-name suffix, adapted
//! from brn-workflow tests/flow.rs). Workspaces are disposable temp dirs;
//! CODEX_HOME is redirected to a synthetic home and no real provider runs.
use brn_workflow::{Config, SearchApproval, Workspace};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
use tempfile::tempdir;
use uuid::Uuid;

const FIXTURE: &str =
    "The synthetic Aurora mission launches on Tuesday. Its crew includes Mira and Niko.\r\n";
const QUESTION: &str = "When does the Aurora mission launch?";

/// Write the fake provider executable for `mode` (success|fail|stall|cancel).
fn fake(dir: &Path, mode: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let exe = dir.join(format!("fake-{mode}"));
    fs::write(
        &exe,
        r#"#!/usr/bin/env python3
import json,sys,os,sqlite3
if '--version' in sys.argv:
 print('codex-cli 0.155.0-alpha.16.4');sys.exit(0)
mode=os.path.basename(sys.argv[0]).split('-')[-1]
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
  if mode=='fail':sys.exit(0)
  if mode=='stall':continue
  send({'method':'item/agentMessage/delta','params':{'threadId':tid,'turnId':'turn_synthetic','delta':'Aurora launches Tuesday [1].'}})
  if mode=='cancel':continue
  send({'method':'turn/completed','params':{'threadId':tid,'turn':{'id':'turn_synthetic','status':'completed'}}})
 elif m=='turn/interrupt':
  if mode=='stall':continue
  send({'id':rid,'result':{}});send({'method':'turn/completed','params':{'threadId':q['params']['threadId'],'turn':{'id':'turn_synthetic','status':'interrupted'}}})
"#,
    )
    .unwrap();
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

/// Run `brn` with `--data-dir root --json` and a synthetic CODEX_HOME.
fn brn_json(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .arg("--data-dir")
        .arg(root)
        .arg("--json")
        .env("CODEX_HOME", root.join("synthetic-home"))
        .stdin(Stdio::null())
        .output()
        .expect("brn binary runs")
}

/// Run `ask QUESTION --codex fake` with extra flags.
fn ask(root: &Path, exe: &Path, extra: &[&str]) -> Output {
    brn_json(
        root,
        std::iter::once("ask")
            .chain(std::iter::once(QUESTION))
            .chain(std::iter::once("--codex"))
            .chain(std::iter::once(exe.to_str().unwrap()))
            .chain(extra.iter().copied())
            .collect::<Vec<&str>>()
            .as_slice(),
    )
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("exit code")
}

fn text(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8")
}

/// Assert stdout holds exactly one JSON value and return it.
fn one_json(out: &Output) -> Value {
    serde_json::from_str(&text(out)).expect("exactly one JSON object on stdout")
}

fn data_ok<'a>(exit: i32, envelope: &'a Value, context: &str) -> &'a Value {
    assert_eq!(exit, 0, "{context}: {envelope}");
    assert_eq!(envelope["ok"], true, "{context}: {envelope}");
    &envelope["data"]
}

/// Block until the fake provider logged its turn/start (bounded, no sleeps).
fn wait_for_log(root: &Path) {
    let log = root.join("submitted.log");
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if fs::read_to_string(&log).is_ok_and(|c| c.contains("turn")) {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("submitted.log never appeared: {}", log.display());
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

#[test]
fn successful_ask_streams_deltas_and_persists_completed_turn() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "success");
    let out = ask(root, &exe, &[]);
    let envelope = one_json(&out); // stdout must be exactly one JSON object
    assert_eq!(code(&out), 0, "{envelope}");
    assert_eq!(envelope["command"], "ask");
    assert_eq!(envelope["ok"], true);
    let data = &envelope["data"];
    assert_eq!(data["status"], "completed");
    assert!(data["answer"].as_str().unwrap().contains("Aurora"));
    assert!(!data["evidence"].as_array().unwrap().is_empty());
    assert!(data["session_id"].as_str().is_some());
    assert_eq!(data["provider_turn_id"], "turn_synthetic");
    // Deltas always go to stderr; stdout stays one clean envelope.
    assert!(!out.stderr.is_empty(), "deltas appeared on stderr");
}

#[test]
fn conversations_list_and_show_expose_history_and_unknown_session_is_not_found() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "success");
    let out = ask(root, &exe, &[]);
    let session = one_json(&out)["data"]["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (c, envelope) = {
        let out = brn_json(root, &["conversations", "list"]);
        (code(&out), one_json(&out))
    };
    let data = data_ok(c, &envelope, "conversations list");
    let sessions = data["sessions"].as_array().unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0]["id"].as_str().unwrap(), session);
    assert_eq!(sessions[0]["turns"], 1);
    assert_eq!(sessions[0]["has_thread"], true);

    let (c, envelope) = {
        let out = brn_json(root, &["conversations", "show", &session]);
        (code(&out), one_json(&out))
    };
    let data = data_ok(c, &envelope, "conversations show");
    assert_eq!(data["session_id"].as_str().unwrap(), session);
    let turns = data["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0]["question"], QUESTION);
    assert!(turns[0]["answer"].as_str().unwrap().contains("Aurora"));
    assert_eq!(turns[0]["status"], "completed");
    assert_eq!(turns[0]["profile"], "keyword");
    assert!(!turns[0]["evidence"].as_array().unwrap().is_empty());

    let unknown = Uuid::new_v4().to_string();
    let (c, envelope) = {
        let out = brn_json(root, &["conversations", "show", &unknown]);
        (code(&out), one_json(&out))
    };
    assert_eq!(c, 1);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "NOT_FOUND");
    assert!(envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains(&unknown));
}

#[test]
fn conversations_list_on_fresh_workspace_is_empty() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (c, envelope) = {
        let out = brn_json(root, &["conversations", "list"]);
        (code(&out), one_json(&out))
    };
    let data = data_ok(c, &envelope, "fresh conversations list");
    assert_eq!(data["sessions"], serde_json::json!([]));
}

#[test]
fn provider_failure_reports_operational_workflow_error() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "fail");
    let out = ask(root, &exe, &[]);
    let envelope = one_json(&out);
    assert_eq!(code(&out), 1, "{envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    assert!(!envelope["error"]["message"].as_str().unwrap().is_empty());
}

#[test]
fn sigint_reports_interrupted_with_exit_130() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "cancel");
    let child = Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["ask", QUESTION, "--codex"])
        .arg(&exe)
        .arg("--data-dir")
        .arg(root)
        .arg("--json")
        .env("CODEX_HOME", root.join("synthetic-home"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("brn ask spawns");
    wait_for_log(root);
    let status = Command::new("/bin/kill")
        .arg("-INT")
        .arg(child.id().to_string())
        .status()
        .expect("/bin/kill runs");
    assert!(status.success(), "SIGINT delivered");
    let out = child.wait_with_output().expect("brn ask waits");
    let envelope = one_json(&out);
    assert_eq!(code(&out), 130, "{envelope}");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "INTERRUPTED");
}

#[test]
fn deadline_times_out_with_exit_124_and_reaps_provider() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "stall");
    let op = Uuid::new_v4().to_string();
    let out = ask(root, &exe, &["--operation", &op, "--timeout-seconds", "1"]);
    let envelope = one_json(&out);
    assert_eq!(code(&out), 124, "{envelope}");
    assert_eq!(envelope["ok"], false, "{envelope}");
    assert_eq!(envelope["error"]["code"], "TIMEOUT");
    let message = envelope["error"]["message"].as_str().unwrap();
    assert!(message.contains(&op), "{message}");
    assert_provider_gone(&exe);
}

#[test]
fn ask_without_codex_fails_before_workspace_init_identically() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("synthetic-home")).unwrap();
    let run = || {
        let out = brn_json(root, &["ask", QUESTION]);
        (code(&out), one_json(&out))
    };
    // A missing required option is a usage error (exit 2, code USAGE).
    let (c, envelope) = run();
    assert_eq!(c, 2, "{envelope}");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "USAGE");
    let message = envelope["error"]["message"].as_str().unwrap().to_string();
    assert!(message.contains("--codex"), "{message}");
    // No workspace state may have been created for the refused invocation.
    assert!(
        !root.join("brn.sqlite3").exists(),
        "store never initialized"
    );
    let (c2, envelope2) = run();
    assert_eq!(c2, c);
    assert_eq!(envelope2["error"]["code"], "USAGE");
    assert_eq!(
        envelope2["error"]["message"].as_str().unwrap(),
        message,
        "repeated runs behave identically"
    );
}

#[test]
fn operation_resubmit_replays_saved_turn_and_conflicts_on_change() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "success");
    let op = Uuid::new_v4().to_string();
    let first = ask(root, &exe, &["--operation", &op]);
    let first_envelope = one_json(&first);
    let data = data_ok(code(&first), &first_envelope, "first ask");
    let answer = data["answer"].as_str().unwrap().to_string();
    let session = data["session_id"].as_str().unwrap().to_string();
    let turn = data["provider_turn_id"].as_str().unwrap().to_string();

    let second = ask(root, &exe, &["--operation", &op]);
    let second_envelope = one_json(&second);
    let data2 = data_ok(code(&second), &second_envelope, "resubmitted ask");
    assert_eq!(data2["answer"].as_str().unwrap(), answer);
    assert_eq!(data2["session_id"].as_str().unwrap(), session);
    assert_eq!(data2["provider_turn_id"].as_str().unwrap(), turn);

    let conflict = {
        let out = brn_json(
            root,
            &[
                "ask",
                "A completely different question?",
                "--codex",
                exe.to_str().unwrap(),
                "--operation",
                &op,
            ],
        );
        (code(&out), one_json(&out))
    };
    assert_eq!(conflict.0, 1, "{}", conflict.1);
    assert_eq!(conflict.1["ok"], false);
    assert_eq!(conflict.1["error"]["code"], "OPERATION_CONFLICT");
}

#[test]
fn ask_with_unknown_session_is_not_found() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "stall"); // never reached: the session check fires first
    let unknown = Uuid::new_v4().to_string();
    let out = ask(root, &exe, &["--session", &unknown]);
    let envelope = one_json(&out);
    assert_eq!(code(&out), 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "NOT_FOUND");
    assert!(envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains(&unknown));
}

#[test]
fn timeout_bounds_outside_valid_range_are_usage_errors() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let exe = fake(root, "stall"); // never reached: the parser rejects first
    for seconds in ["0", "3601"] {
        let out = ask(root, &exe, &["--timeout-seconds", seconds]);
        let envelope = one_json(&out);
        assert_eq!(code(&out), 2, "{seconds}: {envelope}");
        assert_eq!(envelope["error"]["code"], "USAGE");
        assert!(envelope["error"]["message"]
            .as_str()
            .unwrap()
            .contains("--timeout-seconds"));
    }
}
