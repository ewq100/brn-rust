//! Subprocess tests for the brn CLI foundation: parsing, envelopes, status
//! and document surfaces. Workspaces are seeded in-process and always
//! disposable temp dirs; the in-process workspace is dropped before the
//! subprocess runs so it can take the ownership lock.
use brn_workflow::{Config, SearchApproval, Workspace};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    process::{Command, Output},
};
use tempfile::tempdir;
use uuid::Uuid;

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

/// Create a workspace by importing `contents` as a .md file; returns
/// (source_id, version_id, file path). The workspace is dropped (lock released).
fn seed(dir: &Path, name: &str, contents: &str) -> (Uuid, Uuid, PathBuf) {
    let file = dir.join(name);
    fs::write(&file, contents).unwrap();
    let mut workspace = Workspace::open(dir, Config::default()).unwrap();
    let result = workspace
        .import_file(Uuid::new_v4(), &file, SearchApproval::Draft)
        .unwrap();
    drop(workspace);
    (result.source_id, result.version_id, file)
}

#[test]
fn help_and_version_work_without_data_dir() {
    for args in [["--help"], ["help"]] {
        let out = brn(&args);
        assert_eq!(code(&out), 0, "{args:?}");
        let stdout = text(&out);
        assert!(stdout.contains("Usage: brn"), "{args:?}");
        assert!(stdout.contains("documents set-search-approval"), "{args:?}");
        assert!(stdout.contains("revisions diff"), "{args:?}");
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
    let dir = tempdir().unwrap();
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
            vec!["documents", "show", "not-a-uuid"],
            Some("documents.show"),
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
fn status_json_on_fresh_empty_dir() {
    let dir = tempdir().unwrap();
    let out = brn(&[
        "status",
        "--data-dir",
        dir.path().to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    assert!(out.stderr.is_empty());
    let envelope = one_json(&out);
    assert_eq!(envelope["schema_version"], 1);
    assert_eq!(envelope["command"], "status");
    assert_eq!(envelope["ok"], true);
    let data = &envelope["data"];
    assert_eq!(data["recovered_operations"], 0);
    assert_eq!(data["active_index"]["present"], false);
    assert!(data["active_index"]["fingerprint"].is_null());
    assert!(data["active_index"]["error"].is_null());
    assert_eq!(data["capabilities"]["native_retrieval"], false);
    assert_eq!(data["capabilities"]["codex_configured"], false);
    let expected_dir = dir.path().canonicalize().unwrap();
    assert_eq!(data["data_dir"], expected_dir.to_str().unwrap());
    assert_eq!(data["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn status_text_mode_prints_labeled_lines() {
    let dir = tempdir().unwrap();
    let out = brn(&["status", "--data-dir", dir.path().to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let stdout = text(&out);
    assert!(stdout.contains("recovered_operations: 0"), "{stdout}");
    assert!(stdout.contains("active_index: absent"), "{stdout}");
}

#[test]
fn documents_list_json_matches_seed_order_and_fields() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (first_id, first_version, _) = seed(root, "alpha.md", "# Alpha\n\nFirst synthetic note.\n");
    let (second_id, second_version, _) = seed(root, "beta.md", "Approved beta content.\r\n");
    // Re-open to approve the second import; approval does not change list order.
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    workspace
        .set_approval(
            Uuid::new_v4(),
            second_id,
            second_version,
            SearchApproval::Approved,
        )
        .unwrap();
    drop(workspace);

    let out = brn(&[
        "documents",
        "list",
        "--data-dir",
        root.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "documents.list");
    assert_eq!(envelope["ok"], true);
    let docs = envelope["data"]["documents"].as_array().unwrap();
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0]["source_id"], first_id.to_string());
    assert_eq!(docs[0]["version_id"], first_version.to_string());
    assert_eq!(docs[0]["title"], "alpha.md");
    assert_eq!(docs[0]["approval"], "draft");
    assert_eq!(docs[1]["source_id"], second_id.to_string());
    assert_eq!(docs[1]["approval"], "approved");
    for doc in docs {
        let hash = doc["sha256_hex"].as_str().unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    }

    let out = brn(&["documents", "list", "--data-dir", root.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    let stdout = text(&out);
    assert!(
        stdout.starts_with(&format!("{first_id} {first_version} draft alpha.md\n")),
        "{stdout}"
    );
}

#[test]
fn documents_show_returns_exact_content_bytes() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let contents = "héllo\r\nwörld";
    let (source_id, _, _) = seed(root, "exact.md", contents);
    let id = source_id.to_string();
    let out = brn(&[
        "documents",
        "show",
        &id,
        "--data-dir",
        root.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "documents.show");
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["content"], contents);
    assert_eq!(envelope["data"]["title"], "exact.md");
    assert_eq!(envelope["data"]["approval"], "draft");
}

#[test]
fn documents_show_unknown_source_is_not_found() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    seed(root, "present.md", "present\n");
    let missing = Uuid::new_v4().to_string();
    let out = brn(&[
        "documents",
        "show",
        &missing,
        "--data-dir",
        root.to_str().unwrap(),
        "--json",
    ]);
    assert_eq!(code(&out), 1);
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "documents.show");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "NOT_FOUND");
}

#[test]
fn stub_commands_report_not_implemented() {
    let dir = tempdir().unwrap();
    let root = dir.path().to_str().unwrap().to_string();
    let draft_id = Uuid::new_v4().to_string();
    let revision_id = Uuid::new_v4().to_string();
    let cases: Vec<Vec<&str>> = vec![
        vec!["search", "x", "--data-dir", &root],
        vec!["--data-dir", &root, "import", "some-file.md"],
        vec!["index", "build", "--data-dir", &root],
        vec!["ask", "why", "--data-dir", &root],
        vec!["conversations", "list", "--data-dir", &root],
        vec!["drafts", "list", "--data-dir", &root],
        vec![
            "comments",
            "list",
            "--draft",
            &draft_id,
            "--data-dir",
            &root,
        ],
        vec!["revisions", "show", &revision_id, "--data-dir", &root],
    ];
    for args in cases {
        let mut json_args = args.clone();
        json_args.push("--json");
        let out = brn(&json_args);
        assert_eq!(code(&out), 1, "{args:?}");
        let envelope = one_json(&out);
        assert_eq!(envelope["ok"], false, "{args:?}");
        assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR", "{args:?}");
        assert!(
            envelope["error"]["message"]
                .as_str()
                .unwrap()
                .contains("not implemented"),
            "{args:?}"
        );
    }
    let out = brn(&["search", "x", "--data-dir", &root]);
    assert_eq!(code(&out), 1);
    assert!(text(&out).is_empty());
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("not implemented"));
}
