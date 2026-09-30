//! Subprocess tests for `brn drafts create --title TITLE --text-file PATH
//! [--operation UUID]`: bounded UTF-8 file read under the 1 MiB draft limit,
//! exact byte preservation, empty text permitted, blank title rejected,
//! replay and conflict semantics, and usage errors that never initialize a
//! workspace. Workspaces are disposable temp dirs seeded through the CLI.
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::tempdir;
use uuid::Uuid;

/// 1 MiB draft-size limit (brn_store::MAX_DRAFT_BYTES).
const MAX_DRAFT_BYTES: usize = 1024 * 1024;

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

fn one_json(out: &Output) -> Value {
    serde_json::from_str(&text(out)).expect("exactly one JSON object on stdout")
}

fn run_json(dir: &Path, args: &[&str]) -> (i32, Value) {
    let mut all: Vec<&str> = args.to_vec();
    let dir_str = dir.to_str().unwrap();
    all.extend_from_slice(&["--data-dir", dir_str, "--json"]);
    let out = brn(&all);
    let envelope = one_json(&out);
    (code(&out), envelope)
}

fn data_ok<'a>(exit: i32, envelope: &'a Value, context: &str) -> &'a Value {
    assert_eq!(exit, 0, "{context}: {envelope}");
    assert_eq!(envelope["ok"], true, "{context}: {envelope}");
    assert_eq!(envelope["command"], "drafts.create", "{context}");
    &envelope["data"]
}

/// Exact bytes survive: Estonian text, CRLF line endings, a path containing
/// spaces; the caller-supplied operation id and the draft stamp are in the
/// envelope; `drafts show` reopens the same exact content.
#[test]
fn create_then_show_roundtrip_with_exact_bytes() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let contents = "Tere hommikust!\r\nÖö on selge, männi all on hämar.\r\nLõpp.\r\n";
    let file = root.join("minu mustandi tekst.txt");
    fs::write(&file, contents).unwrap();
    let op = Uuid::new_v4().to_string();

    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "Mustand öö Heimichi kohta",
            "--text-file",
            file.to_str().unwrap(),
            "--operation",
            &op,
        ],
    );
    let data = data_ok(c, &env, "drafts create");
    assert_eq!(data["title"], "Mustand öö Heimichi kohta");
    assert_eq!(data["operation_id"], op);
    assert_eq!(data["generation"], 0);
    let id = data["id"].as_str().unwrap().to_string();
    let base = data["base_revision"].as_str().unwrap().to_string();
    let sha = data["sha256_hex"].as_str().unwrap().to_string();
    assert_eq!(sha.len(), 64);
    assert!(id.parse::<Uuid>().is_ok());
    assert!(base.parse::<Uuid>().is_ok());

    // Reopen through the unchanged read-only surface: exact bytes return.
    let (c, env) = run_json(root, &["drafts", "show", &id]);
    assert_eq!(c, 0, "{env}");
    assert_eq!(env["command"], "drafts.show");
    assert_eq!(env["data"]["content"], contents, "exact bytes round-trip");
    assert_eq!(env["data"]["sha256_hex"], sha);
    assert_eq!(env["data"]["base_revision"], base);
    assert_eq!(env["data"]["generation"], 0);
}

/// Without --operation the envelope still carries a generated operation id.
#[test]
fn create_without_operation_generates_one() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("note.txt");
    fs::write(&file, "synthetic draft body\n").unwrap();
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "generated id",
            "--text-file",
            file.to_str().unwrap(),
        ],
    );
    let data = data_ok(c, &env, "generated operation");
    let op = data["operation_id"].as_str().unwrap();
    assert!(op.parse::<Uuid>().is_ok(), "operation_id is a UUID: {op}");
}

/// The existing draft rule permits empty text (only import requires nonempty).
#[test]
fn create_allows_empty_text() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("empty.txt");
    fs::write(&file, "").unwrap();
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "empty draft",
            "--text-file",
            file.to_str().unwrap(),
        ],
    );
    let data = data_ok(c, &env, "empty text draft");
    let id = data["id"].as_str().unwrap().to_string();
    let (c, env) = run_json(root, &["drafts", "show", &id]);
    assert_eq!(c, 0, "{env}");
    assert_eq!(env["data"]["content"], "");
}

/// Invalid UTF-8 input is rejected and creates no draft.
#[test]
fn create_rejects_invalid_utf8_without_creating_draft() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("binary.bin");
    fs::write(&file, [b'f', b'e', 0xFF, b'\n']).unwrap();
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "t",
            "--text-file",
            file.to_str().unwrap(),
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "WORKFLOW_ERROR");
    let (_, env) = run_json(root, &["drafts", "list"]);
    assert_eq!(env["data"]["drafts"].as_array().unwrap().len(), 0);
}

/// Exactly the limit succeeds; one byte over is rejected without a draft.
#[test]
fn create_rejects_oversize_file() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let boundary = root.join("boundary.txt");
    fs::write(&boundary, vec![b'a'; MAX_DRAFT_BYTES]).unwrap();
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "at the limit",
            "--text-file",
            boundary.to_str().unwrap(),
        ],
    );
    data_ok(c, &env, "exactly 1 MiB succeeds");

    let over = root.join("over.txt");
    fs::write(&over, vec![b'a'; MAX_DRAFT_BYTES + 1]).unwrap();
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "too big",
            "--text-file",
            over.to_str().unwrap(),
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["error"]["code"], "WORKFLOW_ERROR");

    let (_, env) = run_json(root, &["drafts", "list"]);
    let drafts = env["data"]["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 1, "only the boundary draft exists");
    assert_eq!(drafts[0]["title"], "at the limit");
}

/// Missing and non-regular-file inputs are rejected (no draft).
#[test]
fn create_rejects_missing_and_non_file_text_file() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let missing = root.join("does-not-exist.txt");
    for path in [missing.to_str().unwrap(), root.to_str().unwrap()] {
        let (c, env) = run_json(
            root,
            &["drafts", "create", "--title", "t", "--text-file", path],
        );
        assert_eq!(c, 1, "{path}: {env}");
        assert_eq!(env["ok"], false, "{path}");
        assert_eq!(env["error"]["code"], "WORKFLOW_ERROR", "{path}");
    }
    let (_, env) = run_json(root, &["drafts", "list"]);
    assert_eq!(env["data"]["drafts"].as_array().unwrap().len(), 0);
}

/// A blank title follows the existing workflow rule (rejected; no draft).
#[test]
fn create_rejects_blank_title_without_creating_draft() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("note.txt");
    fs::write(&file, "body\n").unwrap();
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "   ",
            "--text-file",
            file.to_str().unwrap(),
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["ok"], false);
    let (_, env) = run_json(root, &["drafts", "list"]);
    assert_eq!(env["data"]["drafts"].as_array().unwrap().len(), 0);
}

/// Invalid independent arguments are usage errors (exit 2) and never
/// initialize the workspace: no brn.sqlite3 appears.
#[test]
fn invalid_independent_arguments_do_not_initialize_workspace() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("note.txt");
    fs::write(&file, "body\n").unwrap();
    for args in [
        vec!["drafts", "create", "--text-file", file.to_str().unwrap()], // missing --title
        vec!["drafts", "create", "--title", "t"],                        // missing --text-file
        vec!["drafts", "create", "--title", "t", "--text-file"],         // missing value
        vec![
            "drafts",
            "create",
            "--title",
            "t",
            "--text-file",
            file.to_str().unwrap(),
            "--operation",
            "not-a-uuid",
        ],
        vec![
            "drafts",
            "create",
            "--title",
            "t",
            "--text-file",
            file.to_str().unwrap(),
            "extra",
        ],
    ] {
        let out = brn(&[
            args.as_slice(),
            &["--data-dir", dir.path().to_str().unwrap(), "--json"],
        ]
        .concat());
        assert_eq!(code(&out), 2, "{args:?}: {}", text(&out));
        let env = one_json(&out);
        assert_eq!(env["ok"], false, "{args:?}");
        assert_eq!(env["error"]["code"], "USAGE", "{args:?}");
    }
    assert!(
        !dir.path().join("brn.sqlite3").exists(),
        "no workspace was initialized"
    );
}

/// The same operation id with the same title and text replays the recorded
/// draft after process restart; exactly one draft exists.
#[test]
fn replay_same_operation_after_restart_returns_same_draft() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("same.txt");
    fs::write(&file, "identical bytes\n").unwrap();
    let op = Uuid::new_v4().to_string();
    let args = [
        "drafts",
        "create",
        "--title",
        "replay me",
        "--text-file",
        file.to_str().unwrap(),
        "--operation",
        &op,
    ];
    let (c, first) = run_json(root, &args);
    let data = data_ok(c, &first, "first create");
    let id = data["id"].as_str().unwrap().to_string();
    let base = data["base_revision"].as_str().unwrap().to_string();

    // A separate process is a restart: the recorded result replays unchanged.
    let (c, replay) = run_json(root, &args);
    let data = data_ok(c, &replay, "replayed create");
    assert_eq!(data["id"], id, "same draft id replays");
    assert_eq!(data["base_revision"], base);
    assert_eq!(data["operation_id"], op);

    let (_, env) = run_json(root, &["drafts", "list"]);
    let drafts = env["data"]["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 1, "replay created no second draft");
    assert_eq!(drafts[0]["id"], id);
}

/// The same operation id with a different payload is rejected as a typed
/// OPERATION_CONFLICT and leaves the recorded draft intact.
#[test]
fn conflicting_payload_reuse_is_operation_conflict() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("same.txt");
    fs::write(&file, "identical bytes\n").unwrap();
    let op = Uuid::new_v4().to_string();
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "original",
            "--text-file",
            file.to_str().unwrap(),
            "--operation",
            &op,
        ],
    );
    let data = data_ok(c, &env, "original create");
    let id = data["id"].as_str().unwrap().to_string();

    // Different title, same operation id.
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "different",
            "--text-file",
            file.to_str().unwrap(),
            "--operation",
            &op,
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["error"]["code"], "OPERATION_CONFLICT");

    // Same title, different text, same operation id.
    let other = root.join("other.txt");
    fs::write(&other, "different bytes\n").unwrap();
    let (c, env) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "original",
            "--text-file",
            other.to_str().unwrap(),
            "--operation",
            &op,
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["error"]["code"], "OPERATION_CONFLICT");

    let (_, env) = run_json(root, &["drafts", "list"]);
    let drafts = env["data"]["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 1, "conflicting reuse created no draft");
    assert_eq!(drafts[0]["id"], id);
}
