//! Process checks of contract 2 discovery: the envelope shape, per-command
//! help, global options, pre-open validation and missing workspaces.
mod support;
use serde_json::{json, Value};
use support::*;

fn contract_2(envelope: &Value) {
    assert_eq!(envelope["contract"], 2, "{envelope}");
    let object = envelope.as_object().unwrap();
    assert!(object.contains_key("page"), "{envelope}");
    assert!(object.contains_key("stop"), "{envelope}");
    assert_eq!(object.len(), 15, "{envelope}");
}

#[test]
fn every_envelope_has_contract_2_page_and_stop() {
    let root = Root::new();
    let data = root.join("data");
    let created = call(
        &root,
        &data,
        &["note-create", "--title", "T", "--text", "x"],
    );
    assert_eq!(created["status"], "ok");
    contract_2(&created);
    assert_eq!(created["page"], Value::Null);
    assert_eq!(created["stop"], Value::Null);
    let usage = run_argv(&root, &["--json", "notes"], None).envelope();
    assert_error(&usage, "usage", "missing_data_dir");
    contract_2(&usage);
    let rejected = call(&root, &data, &["read", "--id", "missing"]);
    assert_error(&rejected, "rejected", "not_found");
    contract_2(&rejected);
    let help = run_argv(&root, &["--json", "--help"], None);
    assert_eq!(help.code, 0);
    let description: Value = serde_json::from_str(&help.stdout).unwrap();
    assert_eq!(description["contract"], json!(2));
}

#[test]
fn enum_values_are_refused_before_the_workspace_opens() {
    let root = Root::new();
    let data = root.join("absent");
    let cases: &[(&[&str], &str, &str)] = &[
        (
            &[
                "project-settings",
                "--project",
                "p",
                "--base",
                "1",
                "--title",
                "T",
                "--instructions",
                "I",
                "--knowledge",
                "maybe",
            ],
            "knowledge",
            "--knowledge takes yes|no",
        ),
        (
            &["knowledge-check", "--seed-chat", "no"],
            "seed-chat",
            "--seed-chat takes yes",
        ),
        (
            &["connections", "--note", "n", "--sources", "no"],
            "sources",
            "--sources takes yes",
        ),
        (
            &["connections", "--note", "n", "--hops", "x"],
            "hops",
            "--hops must be an integer",
        ),
        (
            &["search", "--query", "q", "--limit", "many"],
            "limit",
            "--limit must be an integer",
        ),
        (
            &["intake", "--source", "/abs/doc.md", "--intent", "summary"],
            "intent",
            "Choose useful, full, or chat with --thread",
        ),
        (
            &[
                "configure",
                "--provider",
                "Bad",
                "--model",
                "m",
                "--effort",
                "low",
            ],
            "provider",
            "Provider must be Codex or Copilot",
        ),
        (
            &[
                "configure",
                "--provider",
                "Codex",
                "--model",
                "m",
                "--effort",
                "extreme",
            ],
            "effort",
            "Effort must be low, medium or high",
        ),
        (
            &["begin-edit", "--note", "n", "--base", "two"],
            "base",
            "--base must be an integer",
        ),
        (
            &["save", "--session", "s", "--generation", "-1"],
            "generation",
            "--generation must be an integer",
        ),
        (
            &["ask", "--thread", "t", "--prompt", "p", "--budget", "lots"],
            "budget",
            "Invalid budget",
        ),
    ];
    for (argv, option, message) in cases {
        let envelope = call(&root, &data, argv);
        assert_error(&envelope, "usage", "invalid_value");
        assert_eq!(envelope["error"]["details"]["option"], *option, "{argv:?}");
        let default = run(&root, false, &data, argv);
        assert_eq!(default.code, 1, "{argv:?}");
        assert_eq!(default.stderr, format!("{message}\n"), "{argv:?}");
        assert!(!data.exists(), "{argv:?} created the data directory");
    }
    // Case-insensitive providers still pass the pre-open check.
    let configure = [
        "configure",
        "--provider",
        "COPILOT",
        "--model",
        "m",
        "--effort",
        "low",
    ];
    assert_ne!(
        call(&root, &data, &configure)["error"]["code"],
        "invalid_value"
    );
}

#[test]
fn machine_mode_reads_on_missing_or_empty_dirs_are_missing_workspace() {
    let root = Root::new();
    let missing = root.join("missing");
    let reads: &[&[&str]] = &[
        &["notes"],
        &["revisions", "--note", "n"],
        &["operation", "--id", "x"],
        &["runs"],
    ];
    for argv in reads {
        let envelope = call(&root, &missing, argv);
        assert_error(&envelope, "rejected", "missing_workspace");
        assert_eq!(envelope["exit"], 6);
        assert!(!missing.exists(), "{argv:?} created the directory");
    }
    let empty = root.join("empty");
    std::fs::create_dir(&empty).unwrap();
    assert_error(
        &call(&root, &empty, &["notes"]),
        "rejected",
        "missing_workspace",
    );
    assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    // A foreign non-empty folder stays unsafe_directory (guard).
    let foreign = root.join("foreign");
    std::fs::create_dir(&foreign).unwrap();
    std::fs::write(foreign.join("notes.txt"), "not BRN").unwrap();
    assert_error(
        &call(&root, &foreign, &["notes"]),
        "rejected",
        "unsafe_directory",
    );
    // After init the identical read is ok; writes still create a workspace.
    ok(&root, &missing, &["init"]);
    assert_eq!(call(&root, &missing, &["notes"])["status"], "ok");
    let fresh = root.join("fresh");
    let created = call(
        &root,
        &fresh,
        &["note-create", "--title", "T", "--text", "x"],
    );
    assert_eq!(created["status"], "ok");
    // Default mode keeps initialising on a read (guard).
    let plain_dir = root.join("plain");
    let default = run(&root, false, &plain_dir, &["notes"]);
    assert_eq!((default.code, default.stdout.as_str()), (0, "[]\n"));
    assert!(plain_dir.exists());
}
