//! Subprocess parsing, envelopes and retired-command refusal; synthetic data only.
use serde_json::Value;
use std::{
    fs,
    process::{Command, Output, Stdio},
};
mod support;

fn brn(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brn"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("brn binary runs")
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

#[test]
fn help_and_version_work_without_data_dir() {
    for args in [["--help"], ["help"]] {
        let out = brn(&args);
        assert_eq!(code(&out), 0, "{args:?}");
        let stdout = text(&out);
        assert!(stdout.contains("Usage: brn"), "{args:?}");
        assert!(stdout.contains("edit save"), "{args:?}");
        assert!(stdout.contains("conversations show"), "{args:?}");
        assert!(
            serde_json::from_str::<Value>(&stdout).is_err(),
            "help stays textual"
        );
    }
    let out = brn(&["--version"]);
    assert_eq!(code(&out), 0);
    assert!(text(&out).starts_with("brn "));
    let out = brn(&["status", "--help"]);
    assert_eq!(code(&out), 0);
    assert!(text(&out).contains("Usage: brn"));
}

#[test]
fn usage_errors_exit_two_with_json_envelope() {
    let dir = support::data_dir();
    let good = dir.path().to_str().unwrap().to_string();
    let cases: Vec<(Vec<&str>, Option<&str>)> = vec![
        (vec!["bogus"], None),
        (
            vec!["status", "--data-dir", &good, "--bogus", "x"],
            Some("status"),
        ),
        (vec!["--json", "status", "--data-dir"], Some("status")),
        (
            vec!["status", "--data-dir", &good, "--data-dir", &good],
            Some("status"),
        ),
        (
            vec!["conversations", "show", "not-a-uuid"],
            Some("conversations.show"),
        ),
        (vec!["status"], Some("status")),
        (vec!["status", "--data-dir", "relative/dir"], Some("status")),
        (
            vec!["status", "--data-dir", "/nonexistent/brn-probe"],
            Some("status"),
        ),
        (vec!["status", "--codex", "relative/codex"], Some("status")),
    ];
    for (mut args, expected_command) in cases {
        args.push("--json");
        let out = brn(&args);
        assert_eq!(code(&out), 2, "{args:?} -> {}", text(&out));
        let envelope = one_json(&out);
        assert_eq!(envelope["schema_version"], 1, "{args:?}");
        assert_eq!(envelope["ok"], false, "{args:?}");
        assert_eq!(envelope["error"]["code"], "USAGE", "{args:?}");
        match expected_command {
            Some(name) => assert_eq!(envelope["command"], name, "{args:?}"),
            None => assert!(envelope["command"].is_null(), "{args:?}"),
        }
        assert!(out.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn non_json_usage_errors_go_to_stderr() {
    let out = brn(&["bogus"]);
    assert_eq!(code(&out), 2);
    assert!(text(&out).is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.starts_with("error: unknown command: bogus"),
        "{stderr}"
    );
}

#[test]
fn help_wins_before_required_arguments_at_every_level() {
    let reference = text(&brn(&["--help"]));
    assert!(reference.contains("brn inbox source --file REQUEST_JSON"));
    let cases: &[&[&str]] = &[
        &["--help"],
        &["status", "--help"],
        &["search", "--help"],
        &["edit", "save", "--help"],
        &["notes", "show", "--help"],
        &["ai", "connect", "--help"],
        &["models", "download", "--help"],
        &["ask", "--help"],
        &["conversations", "show", "--help"],
        &["inbox", "source", "--help"],
        // Representative --json case: help remains textual, exit 0.
        &["ask", "--help", "--json"],
    ];
    for args in cases {
        let out = brn(args);
        assert_eq!(code(&out), 0, "{args:?}");
        assert_eq!(text(&out), reference, "{args:?}");
        assert!(out.stderr.is_empty(), "{args:?}");
        assert!(
            serde_json::from_str::<Value>(&text(&out)).is_err(),
            "help stays textual: {args:?}"
        );
    }
}

/// A --help request must not open, create or lock a workspace: no files may
/// appear in the data directory.
#[test]
fn help_touches_no_files_in_data_dir() {
    let dir = support::data_dir();
    let root = dir.path().to_str().unwrap().to_string();
    let out = brn(&["ask", "--help", "--data-dir", &root]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "help created files: {:?}",
        entries.iter().map(|e| e.as_ref().unwrap().path())
    );
}

#[test]
fn retired_commands_and_flags_refuse_before_creating_any_files() {
    for args in [
        vec!["--legacy", "status"],
        vec!["status", "--legacy"],
        vec!["import", "synthetic.md"],
        vec!["documents", "list"],
        vec!["index", "build"],
        vec!["drafts", "list"],
        vec!["comments", "list"],
        vec!["revisions", "list"],
        vec!["notes", "open", "plan.md"],
        vec!["notes", "show", "00000000-0000-0000-0000-000000000001"],
        vec!["notes", "recovery", "list"],
        vec!["notes", "save", "id"],
    ] {
        let dir = support::data_dir();
        let mut input = args;
        input.extend(["--json", "--data-dir", dir.path().to_str().unwrap()]);
        let out = brn(&input);
        assert_eq!(code(&out), 2, "{input:?}: {}", text(&out));
        assert_eq!(one_json(&out)["error"]["code"], "USAGE");
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
