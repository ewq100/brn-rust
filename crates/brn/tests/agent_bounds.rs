//! Process checks of bounded execution: SIGTERM, the global `--timeout`
//! deadline (soft and hard stops) and SQLite lock contention (`busy`).
#![cfg(unix)]
mod support;
use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use support::*;

/// A `python3` child holding `BEGIN IMMEDIATE` on the workspace database
/// (no DML) until `release`, which rolls back. Fails the test without python3.
struct HeldLock {
    child: Child,
}
impl HeldLock {
    fn hold(data: &Path) -> Self {
        let script = "import sqlite3, sys\n\
            c = sqlite3.connect(sys.argv[1], isolation_level=None, timeout=0)\n\
            c.execute('BEGIN IMMEDIATE')\n\
            print('locked', flush=True)\n\
            sys.stdin.readline()\n\
            c.execute('ROLLBACK')\n";
        let mut child = Command::new("python3")
            .args(["-c", script])
            .arg(data.join("threads.sqlite3"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 is required for the lock-contention tests");
        let mut line = String::new();
        BufReader::new(child.stdout.as_mut().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert_eq!(line, "locked\n");
        Self { child }
    }
    fn release(mut self) {
        self.child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
        assert!(self.child.wait().unwrap().success());
    }
}

/// Writes 256 KiB to the child's stdin and keeps the pipe open.
fn fill_stdin(child: &mut Child) -> std::process::ChildStdin {
    let mut input = child.stdin.take().unwrap();
    input.write_all(&vec![b'a'; 256 * 1024]).unwrap();
    input
}

fn receipt_total(root: &Root, data: &Path) -> Value {
    ok(root, data, &["history", "--limit", "100"])["total"].clone()
}

#[test]
fn timeout_with_time_to_spare_changes_nothing() {
    let root = Root::new();
    let data = root.join("data");
    let argv = [
        "--json",
        "--data-dir",
        data.to_str().unwrap(),
        "--timeout",
        "60",
        "note-create",
        "--title",
        "T",
        "--text",
        "x",
    ];
    let envelope = run_argv(&root, &argv, None).envelope();
    assert_eq!(envelope["status"], "ok", "{envelope}");
    assert_eq!(envelope["stop"], Value::Null);
    assert_eq!(envelope["changed"][0]["created"], true);
}

#[test]
fn timeout_values_are_refused_before_opening() {
    let root = Root::new();
    let data = root.join("absent");
    let d = data.to_str().unwrap();
    for (values, code) in [
        (vec!["0"], "invalid_value"),
        (vec!["86401"], "invalid_value"),
        (vec!["abc"], "invalid_value"),
        (vec!["-5"], "invalid_value"),
        (vec!["10", "10"], "repeated_option"),
    ] {
        let mut argv = vec!["--json", "--data-dir", d];
        for value in &values {
            argv.extend(["--timeout", value]);
        }
        argv.push("notes");
        let envelope = run_argv(&root, &argv, None).envelope();
        assert_error(&envelope, "usage", code);
        assert_eq!(envelope["error"]["details"]["option"], "timeout");
        let default = run_argv(&root, &argv[1..], None);
        assert_eq!(default.code, 1);
        assert!(default.stderr.contains("--timeout") || default.stderr.contains("Repeated"));
    }
    assert!(!data.exists());
}

#[test]
fn sigterm_during_stdin_wait_is_terminated_and_creates_nothing() {
    let root = Root::new();
    let data = root.join("absent");
    let argv = argv(
        true,
        &data,
        &["note-create", "--title", "T", "--text-file", "-"],
    );
    let mut child = spawn(&root, &argv);
    // More than a pipe holds: once written, the child is reading and its handler is installed.
    let input = fill_stdin(&mut child);
    signal(&child, libc::SIGTERM);
    let run = wait_output(child, Duration::from_secs(10));
    drop(input);
    assert_eq!(run.code, 143, "{} {}", run.stdout, run.stderr);
    let envelope = run.envelope();
    assert_error(&envelope, "terminated", "terminated");
    assert_eq!(envelope["stop"], "sigterm");
    assert_eq!(envelope["retry"], "duplicate");
    assert!(!data.exists());
}

#[test]
fn default_mode_sigterm_keeps_default_disposition() {
    let root = Root::new();
    let data = root.join("absent");
    let argv = argv(
        false,
        &data,
        &["note-create", "--title", "T", "--text-file", "-"],
    );
    let mut child = spawn(&root, &argv);
    let input = fill_stdin(&mut child);
    signal(&child, libc::SIGTERM);
    let run = wait_output(child, Duration::from_secs(10));
    drop(input);
    assert_eq!(run.signal, Some(libc::SIGTERM));
    assert!(run.stdout.is_empty());
}

#[test]
fn timeout_during_fifo_wait_is_a_soft_deadline() {
    let root = Root::new();
    let data = root.join("absent");
    let fifo = root.join("input.fifo");
    let path = std::ffi::CString::new(fifo.to_str().unwrap()).unwrap();
    // SAFETY: creates a FIFO inside the test root.
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    let argv = [
        "--json",
        "--data-dir",
        data.to_str().unwrap(),
        "--timeout",
        "1",
        "note-create",
        "--title",
        "T",
        "--text-file",
        fifo.to_str().unwrap(),
    ];
    let started = Instant::now();
    let run = run_argv(&root, &argv, None);
    // About 1 s. `hard: false` below proves the soft path; the upper bound
    // only catches a hang, generous enough for a loaded parallel run.
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_secs(1) && elapsed < Duration::from_secs(20),
        "{elapsed:?}"
    );
    assert_eq!(run.code, 7);
    let envelope = run.envelope();
    assert_error(&envelope, "failed", "timeout");
    assert_eq!(envelope["error"]["details"]["deadline_seconds"], 1);
    assert_eq!(envelope["error"]["details"]["hard"], false);
    assert_eq!(envelope["stop"], "deadline");
    let default = run_argv(&root, &argv[1..], None);
    assert_eq!(default.code, 1);
    assert_eq!(
        default.stderr,
        "Stopped at the --timeout deadline (1 s); nothing was committed\n"
    );
    assert!(!data.exists());
}

#[test]
fn held_write_lock_reports_busy_and_creates_nothing() {
    let root = Root::new();
    let data = root.join("data");
    ok(&root, &data, &["init"]);
    let before = receipt_total(&root, &data);
    let lock = HeldLock::hold(&data);
    let started = Instant::now();
    let envelope = call(
        &root,
        &data,
        &["note-create", "--title", "T", "--text", "x"],
    );
    assert!(
        started.elapsed() >= Duration::from_secs(4),
        "{:?}",
        started.elapsed()
    );
    assert_error(&envelope, "failed", "busy");
    assert_eq!(envelope["exit"], 7);
    let default = run(
        &root,
        false,
        &data,
        &["note-create", "--title", "T", "--text", "x"],
    );
    assert_eq!(default.code, 1);
    assert!(default.stderr.starts_with("SQLite: "), "{}", default.stderr);
    lock.release();
    assert_eq!(receipt_total(&root, &data), before);
}

#[test]
fn held_lock_past_deadline_hard_stops_with_one_line() {
    let root = Root::new();
    let data = root.join("data");
    ok(&root, &data, &["init"]);
    let before = receipt_total(&root, &data);
    let lock = HeldLock::hold(&data);
    let argv = [
        "--json",
        "--data-dir",
        data.to_str().unwrap(),
        "--timeout",
        "1",
        "note-create",
        "--title",
        "T",
        "--text",
        "x",
    ];
    let started = Instant::now();
    let run = run_argv(&root, &argv, None);
    // Deadline 1 s plus the 3 s grace, measured from before spawn. `hard: true`
    // below proves the hard path; the upper bound only catches a hang.
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(3900) && elapsed < Duration::from_secs(30),
        "{elapsed:?}"
    );
    assert_eq!(run.code, 7);
    let envelope = run.envelope();
    assert_error(&envelope, "failed", "timeout");
    assert_eq!(envelope["error"]["details"]["hard"], true);
    assert_eq!(envelope["stop"], "deadline");
    lock.release();
    assert_eq!(receipt_total(&root, &data), before);
}

#[test]
fn busy_then_keyed_retry_applies_once() {
    let root = Root::new();
    let data = root.join("data");
    ok(&root, &data, &["init"]);
    let argv = ["note-create", "--title", "T", "--text", "x", "--key", "b1"];
    let lock = HeldLock::hold(&data);
    let busy = call(&root, &data, &argv);
    assert_error(&busy, "failed", "busy");
    lock.release();
    let first = call(&root, &data, &argv);
    assert_eq!(first["status"], "ok", "{first}");
    let replay = call(&root, &data, &argv);
    assert_eq!(replay["operation"], first["operation"]);
    let notes = ok(&root, &data, &["notes"]);
    assert_eq!(notes.as_array().unwrap().len(), 1);
}

/// SIGINT envelopes name their reason. The child is isolated (its own
/// process group, SIGINT unblocked), so this passes from any parent shell.
#[test]
fn sigint_envelopes_report_stop_sigint() {
    let root = Root::new();
    let data = root.join("absent");
    let argv = argv(
        true,
        &data,
        &["note-create", "--title", "T", "--text-file", "-"],
    );
    let mut child = spawn(&root, &argv);
    let input = fill_stdin(&mut child);
    signal(&child, libc::SIGINT);
    let run = wait_output(child, Duration::from_secs(10));
    drop(input);
    assert_eq!(run.code, 130);
    let envelope = run.envelope();
    assert_error(&envelope, "interrupted", "interrupted");
    assert_eq!(envelope["stop"], "sigint");
    assert!(!data.exists());
}

/// Starts a keyed-less `note-create` that reads its text from stdin while
/// `lock` holds the database, sends SIGINT once the child is past input
/// (inside the 5 s busy wait), then `then` (SIGTERM, or nothing for a
/// deadline). A fast 130 means SIGINT arrived before the busy wait: retry
/// with a longer delay, up to three attempts.
fn sigint_during_busy_wait(timeout: Option<&str>, then: Option<i32>) -> (Root, Run, Value, Value) {
    let mut delays = [300u64, 600, 1200].into_iter().enumerate();
    loop {
        let (attempt, delay) = delays
            .next()
            .expect("SIGINT always arrived before the busy wait");
        let root = Root::new();
        let data = root.join("data");
        ok(&root, &data, &["init"]);
        let before = receipt_total(&root, &data);
        let lock = HeldLock::hold(&data);
        let mut argv = vec!["--json", "--data-dir", data.to_str().unwrap()];
        if let Some(seconds) = timeout {
            argv.extend(["--timeout", seconds]);
        }
        argv.extend(["note-create", "--title", "T", "--text-file", "-"]);
        let mut child = spawn(&root, &argv);
        drop(fill_stdin(&mut child));
        std::thread::sleep(Duration::from_millis(delay));
        signal(&child, libc::SIGINT);
        if let Some(next) = then {
            // Signals sent back to back can be handled in either order; the
            // pause makes SIGINT the first reason.
            std::thread::sleep(Duration::from_millis(200));
            signal(&child, next);
        }
        let run = wait_output(child, Duration::from_secs(30));
        lock.release();
        let after = receipt_total(&root, &data);
        if run.code == 130 {
            eprintln!(
                "attempt {}: SIGINT before the busy wait; retrying",
                attempt + 1
            );
            continue;
        }
        eprintln!("attempt {} reached the busy wait", attempt + 1);
        return (root, run, before, after);
    }
}

#[test]
fn sigint_then_sigterm_still_hard_stops() {
    let (_root, run, before, after) = sigint_during_busy_wait(None, Some(libc::SIGTERM));
    assert_eq!(run.code, 143, "{} {}", run.stdout, run.stderr);
    let envelope = run.envelope();
    assert_error(&envelope, "terminated", "terminated");
    assert_eq!(envelope["error"]["details"]["hard"], true);
    // `stop` keeps the first reason.
    assert_eq!(envelope["stop"], "sigint");
    assert_eq!(after, before);
}

#[test]
fn sigint_then_deadline_still_hard_stops() {
    let (_root, run, before, after) = sigint_during_busy_wait(Some("1"), None);
    assert_eq!(run.code, 7, "{} {}", run.stdout, run.stderr);
    let envelope = run.envelope();
    assert_error(&envelope, "failed", "timeout");
    assert_eq!(envelope["error"]["details"]["hard"], true);
    assert_eq!(envelope["error"]["details"]["deadline_seconds"], 1);
    assert_eq!(envelope["stop"], "sigint");
    assert_eq!(after, before);
}

#[test]
fn begin_edit_under_a_held_lock_is_busy() {
    let root = Root::new();
    let data = root.join("data");
    let note = note(&root, &data, "N", "text");
    let begin = ["begin-edit", "--note", note.as_str(), "--base", "1"];
    ok(&root, &data, &begin);
    let lock = HeldLock::hold(&data);
    let busy = call(&root, &data, &begin);
    assert_error(&busy, "failed", "busy");
    assert_eq!(busy["exit"], 7);
    let default = run(&root, false, &data, &begin);
    assert_eq!(default.code, 1);
    assert!(default.stderr.starts_with("SQLite: "), "{}", default.stderr);
    lock.release();
    // Without the lock, the open session is reported as before.
    let open = call(&root, &data, &begin);
    assert_error(&open, "rejected", "edit_session_open");
    assert_eq!(
        open["error"]["details"],
        serde_json::json!({"note": note, "base_version": 1})
    );
}
