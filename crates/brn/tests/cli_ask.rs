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

/// Write the fake provider executable for `mode`
/// (success|fail|stall|cancel|stall-init|server-fail).
fn fake(dir: &Path, mode: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let exe = dir.join(format!("fake-{mode}"));
    fs::write(
        &exe,
        r#"#!/usr/bin/env python3
import json,sys,os,sqlite3
if '--version' in sys.argv:
 print('codex-cli 0.155.0-alpha.16.4');sys.exit(0)
mode=os.path.basename(sys.argv[0])[len('fake-'):]
def send(x):print(json.dumps(x),flush=True)
for line in sys.stdin:
 q=json.loads(line);m=q.get('method');rid=q.get('id')
 if m=='initialize':
  if mode=='stall-init':
   with open('../init-stalled.log','w') as f:f.write('stalled\n')
   import time
   while True:time.sleep(1)
  send({'id':rid,'result':{'userAgent':'codex/0.155.0-alpha.16.4','codexHome':os.environ.get('CODEX_HOME','/tmp/fake'),'platformFamily':'unix','platformOs':'macos'}})
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
  if mode=='server-fail':send({'method':'turn/completed','params':{'threadId':tid,'turn':{'id':'turn_synthetic','status':'failed'}}});continue
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
        .import_file(
            &AtomicBool::new(false),
            Uuid::new_v4(),
            &file,
            SearchApproval::Approved,
        )
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

/// Block until `name` exists under `root` (bounded, no busy loop).
fn wait_for_marker(root: &Path, name: &str) {
    let log = root.join(name);
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if fs::read_to_string(&log).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("{name} never appeared: {}", log.display());
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

/// Spawn `brn ask` with the fake executable and extra flags, stdout/stderr piped.
fn spawn_ask(root: &Path, exe: &Path, extra: &[&str]) -> std::process::Child {
    Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(["ask", QUESTION, "--codex"])
        .arg(exe)
        .args(extra)
        .arg("--data-dir")
        .arg(root)
        .arg("--json")
        .env("CODEX_HOME", root.join("synthetic-home"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("brn ask spawns")
}

/// Wait for a spawned run to finish and capture its output.
fn wait_exit(child: std::process::Child) -> Output {
    child.wait_with_output().expect("brn ask waits")
}

/// The additive machine-readable error context of a failure envelope.
fn error_context(envelope: &Value) -> &Value {
    &envelope["error"]["context"]
}

/// Parse a context identifier string as a UUID.
fn uuid_of(value: &Value, what: &str) -> Uuid {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{what} is a string: {value}"))
        .parse()
        .unwrap_or_else(|_| panic!("{what} is a UUID: {value}"))
}

/// Count the submitted external turn requests in submitted.log.
fn submitted_count(root: &Path) -> usize {
    fs::read_to_string(root.join("submitted.log"))
        .unwrap()
        .lines()
        .filter(|l| l.contains("turn"))
        .count()
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
    let child = spawn_ask(root, &exe, &[]);
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
    // Honesty rule: this turn was durably recorded as interrupted, but the
    // record itself is ambiguous — the provider did confirm the interruption
    // here, yet a recorded interrupted turn alone (as after transport loss)
    // is never proof of cancellation, so provider_outcome reports "unknown".
    let context = error_context(&envelope);
    assert_eq!(context["recorded_status"], "interrupted", "{envelope}");
    assert_eq!(context["provider_outcome"], "unknown", "{envelope}");
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

#[test]
fn transport_loss_after_submission_reports_structured_context() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "fail");
    let out = ask(root, &exe, &[]);
    let envelope = one_json(&out);
    assert_eq!(code(&out), 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    let context = error_context(&envelope);
    let op = uuid_of(&context["operation_id"], "context operation_id");
    let session = uuid_of(&context["session_id"], "context session_id");
    assert_eq!(context["recorded_status"], "interrupted", "{envelope}");
    assert_eq!(context["provider_outcome"], "unknown", "{envelope}");
    // The reported identifiers must match the durable local record.
    let show = brn_json(root, &["conversations", "show", &session.to_string()]);
    let show_envelope = one_json(&show);
    let data = data_ok(code(&show), &show_envelope, "conversations show");
    let turns = data["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0]["operation_id"].as_str().unwrap(), op.to_string());
    assert_eq!(turns[0]["status"], "interrupted");
}

#[test]
fn deadline_after_submission_timeout_context() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "stall");
    let op = Uuid::new_v4().to_string();
    // Generous deadline, and only start judging after the turn was actually
    // submitted: the assertions must not depend on machine load racing the
    // deadline before submission.
    let child = spawn_ask(root, &exe, &["--operation", &op, "--timeout-seconds", "5"]);
    wait_for_log(root);
    let out = wait_exit(child);
    let envelope = one_json(&out);
    assert_eq!(code(&out), 124, "{envelope}");
    assert_eq!(envelope["error"]["code"], "TIMEOUT");
    let context = error_context(&envelope);
    assert_eq!(context["operation_id"].as_str().unwrap(), op);
    assert_eq!(context["recorded_status"], "interrupted", "{envelope}");
    assert_eq!(context["provider_outcome"], "unknown", "{envelope}");
}

#[test]
fn deadline_before_submission_timeout_context() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "stall-init");
    let out = ask(root, &exe, &["--timeout-seconds", "1"]);
    // Make the stall deterministic before the deadline fires.
    wait_for_marker(root, "init-stalled.log");
    let envelope = one_json(&out);
    assert_eq!(code(&out), 124, "{envelope}");
    assert_eq!(envelope["error"]["code"], "TIMEOUT");
    let context = error_context(&envelope);
    assert!(context["operation_id"].as_str().is_some(), "{envelope}");
    assert_eq!(context["session_id"], serde_json::json!(null), "{envelope}");
    assert_eq!(
        context["recorded_status"],
        serde_json::json!(null),
        "{envelope}"
    );
    assert_eq!(context["provider_outcome"], "unknown", "{envelope}");
    assert_provider_gone(&exe);
}

#[test]
fn sigint_before_submission_reports_unestablished_context() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "stall-init");
    let child = spawn_ask(root, &exe, &[]);
    wait_for_marker(root, "init-stalled.log");
    let status = Command::new("/bin/kill")
        .arg("-INT")
        .arg(child.id().to_string())
        .status()
        .expect("/bin/kill runs");
    assert!(status.success(), "SIGINT delivered");
    let out = child.wait_with_output().expect("brn ask waits");
    let envelope = one_json(&out);
    assert_eq!(code(&out), 130, "{envelope}");
    assert_eq!(envelope["error"]["code"], "INTERRUPTED");
    let context = error_context(&envelope);
    assert_eq!(context["session_id"], serde_json::json!(null), "{envelope}");
    assert_eq!(
        context["recorded_status"],
        serde_json::json!(null),
        "{envelope}"
    );
    assert_eq!(context["provider_outcome"], "unknown", "{envelope}");
}

#[test]
fn server_confirmed_failed_outcome_is_reported_as_confirmed() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "server-fail");
    let out = ask(root, &exe, &[]);
    let envelope = one_json(&out);
    assert_eq!(code(&out), 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    let context = error_context(&envelope);
    assert_eq!(context["recorded_status"], "failed", "{envelope}");
    assert_eq!(context["provider_outcome"], "failed", "{envelope}");
}

#[test]
fn connect_failure_reports_unestablished_context() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let out = ask(root, Path::new("/nonexistent/absolute/codex"), &[]);
    let envelope = one_json(&out);
    assert_eq!(code(&out), 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    let context = error_context(&envelope);
    assert!(context["operation_id"].as_str().is_some(), "{envelope}");
    assert_eq!(context["session_id"], serde_json::json!(null), "{envelope}");
    assert_eq!(
        context["recorded_status"],
        serde_json::json!(null),
        "{envelope}"
    );
    assert_eq!(context["provider_outcome"], "unknown", "{envelope}");
}

#[test]
fn generated_and_supplied_ids_appear_in_context() {
    // One workspace per run: the fake provider hardcodes a single synthetic
    // thread id per workspace, so a second stall run in the same workspace
    // would die on the fake's own consistency assert.
    fn stall_context() -> (tempfile::TempDir, Value) {
        let dir = tempdir().unwrap();
        let root = dir.path();
        seed(root);
        let exe = fake(root, "stall");
        let out = ask(root, &exe, &["--timeout-seconds", "2"]);
        let envelope = one_json(&out);
        assert_eq!(code(&out), 124, "{envelope}");
        (dir, error_context(&envelope).clone())
    }
    let (_d1, first) = stall_context();
    let op1 = uuid_of(&first["operation_id"], "first operation_id");
    let (_d2, second) = stall_context();
    let op2 = uuid_of(&second["operation_id"], "second operation_id");
    assert_ne!(op1, op2, "generated operation ids differ between runs");
    // A caller-supplied id is echoed verbatim in the context.
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let exe = fake(root, "stall");
    let supplied = Uuid::new_v4().to_string();
    let out = ask(
        root,
        &exe,
        &["--operation", &supplied, "--timeout-seconds", "2"],
    );
    let envelope = one_json(&out);
    assert_eq!(code(&out), 124, "{envelope}");
    assert_eq!(
        error_context(&envelope)["operation_id"].as_str().unwrap(),
        supplied
    );
}

#[test]
fn same_operation_retry_returns_recorded_without_resubmission() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root);
    let op = Uuid::new_v4().to_string();
    let stalled = fake(root, "stall");
    // Only judge after the turn was actually submitted: a too-tight deadline
    // under load could fire before submission, which would honestly report
    // recorded_status null instead.
    let first = spawn_ask(
        root,
        &stalled,
        &["--operation", &op, "--timeout-seconds", "5"],
    );
    wait_for_log(root);
    let first = wait_exit(first);
    let envelope = one_json(&first);
    assert_eq!(code(&first), 124, "{envelope}");
    let context = error_context(&envelope);
    assert_eq!(context["recorded_status"], "interrupted", "{envelope}");
    assert_eq!(context["provider_outcome"], "unknown", "{envelope}");

    // Same operation id: the recorded state is returned without a new
    // external submission, even though the provider now works.
    let working = fake(root, "success");
    let retry = ask(root, &working, &["--operation", &op]);
    let envelope = one_json(&retry);
    assert_eq!(code(&retry), 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    assert!(envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains("turn ended as interrupted"));
    let context = error_context(&envelope);
    assert_eq!(context["recorded_status"], "interrupted", "{envelope}");
    assert_eq!(context["provider_outcome"], "unknown", "{envelope}");
    assert_eq!(submitted_count(root), 1, "no second external submission");
}
