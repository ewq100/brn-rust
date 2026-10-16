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

#[test]
fn every_command_has_text_and_json_help_matching_the_description() {
    let root = Root::new();
    let help = run_argv(&root, &["--json", "--help"], None);
    let description: Value = serde_json::from_str(&help.stdout).unwrap();
    assert!(description["global_options"].is_array());
    for entry in description["commands"].as_array().unwrap() {
        let name = entry["name"].as_str().unwrap();
        let machine = run_argv(&root, &["--json", name, "--help"], None);
        assert_eq!(machine.code, 0, "{name}");
        assert_eq!(machine.stdout.matches('\n').count(), 1, "{name}");
        let value: Value = serde_json::from_str(&machine.stdout).unwrap();
        assert_eq!(value, json!({"contract": 2, "command": entry}), "{name}");
        let text = run_argv(&root, &[name, "--help"], None);
        assert_eq!(text.code, 0, "{name}");
        let summary = entry["summary"].as_str().unwrap();
        assert!(
            text.stdout
                .starts_with(&format!("brn {name} — {summary}\n")),
            "{name}"
        );
        assert!(text.stdout.contains(&format!(
            "Retry: {}",
            entry["retry"]["default"].as_str().unwrap()
        )));
        assert!(
            text.stdout.contains(entry["example"].as_str().unwrap()),
            "{name}"
        );
        for required in entry["required"].as_array().unwrap() {
            assert!(
                text.stdout
                    .contains(&format!("--{}", required.as_str().unwrap())),
                "{name}"
            );
        }
        // With a data dir and other options, --help still opens nothing.
        let data = root.join(&format!("help-{name}"));
        let with_dir = run_argv(
            &root,
            &[
                "--json",
                "--data-dir",
                data.to_str().unwrap(),
                name,
                "--help",
            ],
            None,
        );
        assert_eq!(with_dir.stdout, machine.stdout, "{name}");
        assert!(!data.exists(), "{name}");
    }
    let intake = run_argv(&root, &["intake", "--help"], None).stdout;
    assert!(intake.contains("replays when bytes, path, scope and title match"));
    assert!(intake.contains("Owner decision when --thread names a Project"));
    let save = run_argv(&root, &["save", "--help"], None).stdout;
    assert!(save.contains("begin-edit --note N --base <conflict[0].version>"));
}

#[test]
fn data_dir_and_timeout_are_accepted_after_the_command() {
    let root = Root::new();
    let data = root.join("data");
    let d = data.to_str().unwrap();
    let created = run_argv(
        &root,
        &[
            "--json",
            "note-create",
            "--title",
            "T",
            "--data-dir",
            d,
            "--text",
            "x",
            "--timeout",
            "30",
        ],
        None,
    )
    .envelope();
    assert_eq!(created["status"], "ok", "{created}");
    let notes = run_argv(&root, &["notes", "--data-dir", d], None);
    assert_eq!(notes.code, 0);
    assert_eq!(
        serde_json::from_str::<Value>(&notes.stdout)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // A value is never read as an option: this title is "--data-dir".
    let titled = run_argv(
        &root,
        &[
            "--json",
            "--data-dir",
            d,
            "note-create",
            "--title",
            "--data-dir",
            "--text",
            "y",
        ],
        None,
    )
    .envelope();
    assert_eq!(titled["status"], "ok", "{titled}");
    let id = titled["changed"][0]["id"].as_str().unwrap();
    let read = ok(&root, &data, &["read", "--id", id]);
    assert_eq!(read["data"]["Note"]["title"], "--data-dir");
}

#[test]
fn relative_backup_and_restore_paths_are_refused_before_opening() {
    let root = Root::new();
    let data = root.join("absent");
    for (argv, option) in [
        (
            &["backup", "--destination", "backup.sqlite3"][..],
            "destination",
        ),
        (&["export", "--destination", "export"][..], "destination"),
        (&["restore", "--source", "backup.sqlite3"][..], "source"),
        (
            &["upgrade", "--backup-directory", "backups"][..],
            "backup-directory",
        ),
        (
            &["model-install", "--destination", "model"][..],
            "destination",
        ),
    ] {
        let envelope = call(&root, &data, argv);
        assert_error(&envelope, "usage", "invalid_value");
        assert_eq!(envelope["error"]["details"]["option"], option);
        let default = run(&root, false, &data, argv);
        assert_eq!(default.code, 1);
        assert_eq!(
            default.stderr,
            format!("--{option} must be an absolute path\n")
        );
    }
    assert!(!data.exists());
}

#[test]
fn help_after_an_unknown_command_is_unknown_command() {
    let root = Root::new();
    let envelope = run_argv(&root, &["--json", "frobnicate", "--help"], None).envelope();
    assert_error(&envelope, "usage", "unknown_command");
    assert_eq!(envelope["error"]["details"]["command"], "frobnicate");
    let data = root.join("absent");
    let with_dir = run_argv(
        &root,
        &[
            "--json",
            "--data-dir",
            data.to_str().unwrap(),
            "frobnicate",
            "--help",
        ],
        None,
    )
    .envelope();
    assert_error(&with_dir, "usage", "unknown_command");
    assert!(!data.exists());
}

#[test]
fn repeated_global_options_are_repeated_option() {
    let root = Root::new();
    let data = root.join("absent");
    let d = data.to_str().unwrap();
    for (argv, option) in [
        (
            vec!["--json", "--data-dir", d, "notes", "--data-dir", d],
            "data-dir",
        ),
        (
            vec![
                "--json",
                "--data-dir",
                d,
                "--timeout",
                "5",
                "notes",
                "--timeout",
                "5",
            ],
            "timeout",
        ),
        (
            vec!["--json", "notes", "--data-dir", d, "--data-dir", d],
            "data-dir",
        ),
    ] {
        let envelope = run_argv(&root, &argv, None).envelope();
        assert_error(&envelope, "usage", "repeated_option");
        assert_eq!(envelope["error"]["details"]["option"], option, "{argv:?}");
    }
    assert!(!data.exists());
}

#[test]
fn bare_and_unknown_invocations_keep_their_exits_and_help_text() {
    let root = Root::new();
    for argv in [&[][..], &["--unknown"][..], &["notes"][..]] {
        let run = run_argv(&root, argv, None);
        assert_eq!(run.code, 1, "{argv:?}");
        assert_eq!(
            run.stderr,
            "Supply --data-dir ABSOLUTE_DIRECTORY; run --help\n"
        );
    }
    let help = run_argv(&root, &["--help"], None);
    assert_eq!(help.code, 0);
    assert!(help
        .stdout
        .starts_with("BRN Threads\n\nUsage: brn --data-dir ABSOLUTE_DIRECTORY COMMAND"));
    assert!(help.stdout.contains("Agents and scripts:"));
}
