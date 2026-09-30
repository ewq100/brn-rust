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

/// One fresh empty data directory plus a fixture directory outside it.
struct FreshInput {
    /// Keeps the parent of `data` alive for the whole test.
    _parent: tempfile::TempDir,
    /// Fresh, empty data directory: must stay free of BRN artifacts when
    /// the invocation's input is invalid.
    data: std::path::PathBuf,
    /// Fixture directory, deliberately outside `data`.
    fixtures: tempfile::TempDir,
}

fn fresh_input() -> FreshInput {
    let parent = tempdir().unwrap();
    let data = parent.path().join("data");
    fs::create_dir(&data).unwrap();
    FreshInput {
        _parent: parent,
        data,
        fixtures: tempdir().unwrap(),
    }
}

/// A data directory handed to an invocation with invalid input must remain
/// completely empty: no database, owner-lock file, SQLite sidecars or any
/// other BRN initialization artifact may have appeared. Asserted directly
/// on the filesystem, before any workspace-opening command runs.
fn assert_workspace_never_initialized(data: &Path) {
    let entries: Vec<String> = fs::read_dir(data)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(entries.is_empty(), "workspace was initialized: {entries:?}");
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

/// Invalid UTF-8 input is rejected before the workspace is opened: the
/// data directory stays completely free of initialization artifacts.
#[test]
fn create_rejects_invalid_utf8_without_initializing_workspace() {
    let input = fresh_input();
    let file = input.fixtures.path().join("binary.bin");
    fs::write(&file, [b'f', b'e', 0xFF, b'\n']).unwrap();
    let (c, env) = run_json(
        &input.data,
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
    assert_workspace_never_initialized(&input.data);
}

/// Exactly the limit succeeds: the full 1 MiB body is stored intact.
#[test]
fn create_accepts_exactly_at_limit() {
    let input = fresh_input();
    let boundary = input.fixtures.path().join("boundary.txt");
    fs::write(&boundary, vec![b'a'; MAX_DRAFT_BYTES]).unwrap();
    let (c, env) = run_json(
        &input.data,
        &[
            "drafts",
            "create",
            "--title",
            "at the limit",
            "--text-file",
            boundary.to_str().unwrap(),
        ],
    );
    let data = data_ok(c, &env, "exactly 1 MiB succeeds");
    let id = data["id"].as_str().unwrap().to_string();
    let (c, env) = run_json(&input.data, &["drafts", "show", &id]);
    assert_eq!(c, 0, "{env}");
    assert_eq!(
        env["data"]["content"].as_str().unwrap().len(),
        MAX_DRAFT_BYTES,
        "the full body is stored"
    );
}

/// One byte over the limit is rejected before the workspace is opened.
#[test]
fn create_rejects_oversize_text_file_without_initializing_workspace() {
    let input = fresh_input();
    let over = input.fixtures.path().join("over.txt");
    fs::write(&over, vec![b'a'; MAX_DRAFT_BYTES + 1]).unwrap();
    let (c, env) = run_json(
        &input.data,
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
    assert_workspace_never_initialized(&input.data);
}

/// A missing text file is rejected before the workspace is opened.
#[test]
fn create_rejects_missing_text_file_without_initializing_workspace() {
    let input = fresh_input();
    let missing = input.fixtures.path().join("does-not-exist.txt");
    let (c, env) = run_json(
        &input.data,
        &[
            "drafts",
            "create",
            "--title",
            "t",
            "--text-file",
            missing.to_str().unwrap(),
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "WORKFLOW_ERROR");
    assert_workspace_never_initialized(&input.data);
}

/// A directory (non-file) text-file path is rejected before the workspace
/// is opened.
#[test]
fn create_rejects_directory_text_file_without_initializing_workspace() {
    let input = fresh_input();
    let nested = input.fixtures.path().join("nested");
    fs::create_dir(&nested).unwrap();
    let (c, env) = run_json(
        &input.data,
        &[
            "drafts",
            "create",
            "--title",
            "t",
            "--text-file",
            nested.to_str().unwrap(),
        ],
    );
    assert_eq!(c, 1, "{env}");
    assert_eq!(env["ok"], false);
    assert_eq!(env["error"]["code"], "WORKFLOW_ERROR");
    assert_workspace_never_initialized(&input.data);
}

/// A blank title follows the existing workflow rule and is rejected before
/// the workspace is opened.
#[test]
fn create_rejects_blank_title_without_initializing_workspace() {
    let input = fresh_input();
    let file = input.fixtures.path().join("note.txt");
    fs::write(&file, "body\n").unwrap();
    let (c, env) = run_json(
        &input.data,
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
    assert_eq!(env["error"]["code"], "WORKFLOW_ERROR");
    assert_eq!(env["error"]["message"], "draft title is required");
    assert_workspace_never_initialized(&input.data);
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
