//! Subprocess acceptance tests for `brn drafts save DRAFT_ID`.
//! Workspaces are seeded through the shared workflow and are dropped before
//! each CLI subprocess takes exclusive ownership.
use brn_workflow::{CommentCapture, Config, Draft, EditTrace, Workspace, MAX_DRAFT_BYTES};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::{tempdir, TempDir};
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

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("stdout is UTF-8")
}

fn json(out: &Output) -> Value {
    serde_json::from_str(&stdout(out)).expect("exactly one JSON envelope")
}

fn run_json(root: &Path, args: &[&str]) -> (i32, Value) {
    let mut all = args.to_vec();
    all.extend_from_slice(&["--data-dir", root.to_str().unwrap(), "--json"]);
    let out = brn(&all);
    (code(&out), json(&out))
}

fn data_ok<'a>(exit: i32, envelope: &'a Value, context: &str) -> &'a Value {
    assert_eq!(exit, 0, "{context}: {envelope}");
    assert_eq!(envelope["ok"], true, "{context}: {envelope}");
    &envelope["data"]
}

fn create_via_cli(root: &Path, text: &str, title: &str) -> (String, String) {
    let file = root.join("create.txt");
    fs::write(&file, text).unwrap();
    let (exit, envelope) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            title,
            "--text-file",
            file.to_str().unwrap(),
        ],
    );
    let data = data_ok(exit, &envelope, "draft creation");
    (
        data["id"].as_str().unwrap().to_string(),
        data["base_revision"].as_str().unwrap().to_string(),
    )
}

fn save_args_raw(
    draft: &str,
    base: &str,
    expected_generation: &str,
    generation: &str,
    text_file: &Path,
    operation: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "drafts".to_string(),
        "save".to_string(),
        draft.to_string(),
        "--base-revision".to_string(),
        base.to_string(),
        "--expected-generation".to_string(),
        expected_generation.to_string(),
        "--generation".to_string(),
        generation.to_string(),
        "--text-file".to_string(),
        text_file.to_str().unwrap().to_string(),
    ];
    if let Some(operation) = operation {
        args.extend(["--operation".to_string(), operation.to_string()]);
    }
    args
}

fn save_args(
    draft: &str,
    base: &str,
    expected_generation: u64,
    generation: u64,
    text_file: &Path,
    operation: Option<&str>,
) -> Vec<String> {
    save_args_raw(
        draft,
        base,
        &expected_generation.to_string(),
        &generation.to_string(),
        text_file,
        operation,
    )
}

fn run_owned_args(root: &Path, args: &[String]) -> (i32, Value) {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    run_json(root, &borrowed)
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn save_create_show_reopen_preserves_exact_unicode_crlf_and_operation() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, "initial\n", "save roundtrip");
    let text = "Tere hommikust!\r\nÖö on selge, männi all on hämar.\r\nLõpp.\r\n";
    let file = root.join("save.txt");
    fs::write(&file, text).unwrap();
    let operation = Uuid::new_v4().to_string();
    let args = save_args(&draft, &base, 0, 1, &file, Some(&operation));

    let (exit, saved) = run_owned_args(root, &args);
    let data = data_ok(exit, &saved, "draft save");
    assert_eq!(saved["command"], "drafts.save");
    assert_eq!(data["id"], draft);
    assert_eq!(data["base_revision"], base);
    assert_eq!(data["generation"], 1);
    assert_eq!(data["operation_id"], operation);

    let (exit, shown) = run_json(root, &["drafts", "show", &draft]);
    assert_eq!(exit, 0, "{shown}");
    assert_eq!(shown["data"]["content"], text);
    assert_eq!(shown["data"]["base_revision"], base);
    assert_eq!(shown["data"]["generation"], 1);

    // A separate process reopens the same store and must return the same exact
    // bytes, including Unicode and CRLF line endings.
    let (exit, reopened) = run_json(root, &["drafts", "show", &draft]);
    assert_eq!(exit, 0, "{reopened}");
    assert_eq!(reopened["data"]["content"], text);
    assert_eq!(reopened["data"]["sha256_hex"], shown["data"]["sha256_hex"]);
}

#[test]
fn save_allows_unchanged_text_without_creating_a_checkpoint() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, "same", "unchanged save");
    let file = root.join("same.txt");
    fs::write(&file, "same").unwrap();

    let (exit, saved) = run_owned_args(root, &save_args(&draft, &base, 0, 1, &file, None));
    let data = data_ok(exit, &saved, "unchanged draft save");
    assert_eq!(data["base_revision"], base);
    assert_eq!(data["generation"], 1);
    assert!(data["operation_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .is_ok());

    let (exit, revisions) = run_json(root, &["revisions", "list", "--draft", &draft]);
    assert_eq!(exit, 0, "{revisions}");
    assert_eq!(revisions["data"]["revisions"].as_array().unwrap().len(), 1);
}

#[test]
fn save_rejects_expected_state_mismatch_without_mutating_the_draft() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, "original", "first draft");
    let (_, other_base) = create_via_cli(root, "other", "second draft");
    let mismatched_file = root.join("mismatched.txt");
    fs::write(&mismatched_file, "must not save").unwrap();

    let (exit, envelope) = run_owned_args(
        root,
        &save_args(&draft, &other_base, 0, 1, &mismatched_file, None),
    );
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");

    let first_file = root.join("first.txt");
    fs::write(&first_file, "first successful save").unwrap();
    let (exit, first) = run_owned_args(root, &save_args(&draft, &base, 0, 1, &first_file, None));
    data_ok(exit, &first, "first successful save");

    let stale_file = root.join("stale-generation.txt");
    fs::write(&stale_file, "stale generation").unwrap();
    let (exit, envelope) = run_owned_args(root, &save_args(&draft, &base, 0, 2, &stale_file, None));
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");

    let (exit, shown) = run_json(root, &["drafts", "show", &draft]);
    assert_eq!(exit, 0, "{shown}");
    assert_eq!(shown["data"]["content"], "first successful save");
    assert_eq!(shown["data"]["base_revision"], base);
    assert_eq!(shown["data"]["generation"], 1);
}

#[test]
fn save_rejects_invalid_input_before_opening_the_workspace() {
    let parent = tempdir().unwrap();
    let data = parent.path().join("data");
    fs::create_dir(&data).unwrap();
    let fixtures = tempdir().unwrap();
    let file = fixtures.path().join("invalid.bin");
    fs::write(&file, [b'a', 0xff, b'\n']).unwrap();
    let args = save_args(
        &Uuid::new_v4().to_string(),
        &Uuid::new_v4().to_string(),
        0,
        1,
        &file,
        Some(&Uuid::new_v4().to_string()),
    );

    let (exit, envelope) = run_owned_args(&data, &args);
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    assert!(fs::read_dir(&data).unwrap().next().is_none());
}

#[test]
fn save_rejects_text_over_the_shared_draft_limit_before_opening_workspace() {
    let parent = tempdir().unwrap();
    let data = parent.path().join("data");
    fs::create_dir(&data).unwrap();
    let fixtures = tempdir().unwrap();
    let file = fixtures.path().join("oversize.txt");
    fs::write(&file, vec![b'x'; MAX_DRAFT_BYTES + 1]).unwrap();
    let args = save_args(
        &Uuid::new_v4().to_string(),
        &Uuid::new_v4().to_string(),
        0,
        1,
        &file,
        None,
    );

    let (exit, envelope) = run_owned_args(&data, &args);
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    assert!(fs::read_dir(&data).unwrap().next().is_none());
}

#[test]
fn save_replays_the_original_receipt_after_a_later_edit_and_restart() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, "before", "replay save");
    let file = root.join("first.txt");
    fs::write(&file, "first").unwrap();
    let operation = Uuid::new_v4().to_string();
    let args = save_args(&draft, &base, 0, 1, &file, Some(&operation));

    let (exit, first) = run_owned_args(root, &args);
    let first_data = data_ok(exit, &first, "first save").clone();

    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let current = workspace
        .draft(Uuid::parse_str(&draft).unwrap())
        .unwrap()
        .unwrap();
    let later = workspace
        .save_draft(Uuid::new_v4(), current.id, current.stamp, 2, "later edit")
        .unwrap();
    drop(workspace);

    let (exit, replay) = run_owned_args(root, &args);
    assert_eq!(exit, 0, "{replay}");
    assert_eq!(replay["data"], first_data);

    let (exit, shown) = run_json(root, &["drafts", "show", &draft]);
    assert_eq!(exit, 0, "{shown}");
    assert_eq!(shown["data"]["content"], later.text);
    assert_eq!(shown["data"]["generation"], 2);
}

#[test]
fn save_conflicting_operation_reuse_is_rejected_and_state_stays_frozen() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, "before", "conflict save");
    let operation = Uuid::new_v4().to_string();
    let first_file = root.join("first.txt");
    fs::write(&first_file, "first").unwrap();
    let first_args = save_args(&draft, &base, 0, 1, &first_file, Some(&operation));
    let (exit, first) = run_owned_args(root, &first_args);
    data_ok(exit, &first, "first save");

    let conflicting_file = root.join("conflicting.txt");
    fs::write(&conflicting_file, "conflicting").unwrap();
    let (exit, envelope) = run_owned_args(
        root,
        &save_args(&draft, &base, 0, 1, &conflicting_file, Some(&operation)),
    );
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "OPERATION_CONFLICT");

    let (exit, shown) = run_json(root, &["drafts", "show", &draft]);
    assert_eq!(exit, 0, "{shown}");
    assert_eq!(shown["data"]["content"], "first");
    assert_eq!(shown["data"]["generation"], 1);
}

fn seed_alpha_beta_comments(root: &Path) -> Draft {
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let draft = workspace
        .create_draft(Uuid::new_v4(), "commented save", "alpha beta gamma")
        .unwrap();
    let alpha = workspace
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: draft.stamp,
            generation: draft.stamp.generation,
            text: draft.text.clone(),
            edits: EditTrace::Steps(vec![]),
            range: 0..5,
            quote: "alpha".into(),
            body: "alpha evidence".into(),
        })
        .unwrap();
    let beta = workspace
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: alpha.saved.draft.stamp,
            generation: alpha.saved.draft.stamp.generation,
            text: alpha.saved.draft.text.clone(),
            edits: EditTrace::Steps(vec![]),
            range: 6..10,
            quote: "beta".into(),
            body: "beta evidence".into(),
        })
        .unwrap();
    drop(workspace);
    beta.saved.draft
}

#[test]
fn save_maps_comments_and_preserves_original_evidence() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let current = seed_alpha_beta_comments(root);
    let file = root.join("mapped.txt");
    fs::write(&file, "X alpha beta gamma").unwrap();
    let (exit, saved) = run_owned_args(
        root,
        &save_args(
            &current.id.to_string(),
            &current.stamp.base_revision.to_string(),
            current.stamp.generation,
            current.stamp.generation + 1,
            &file,
            None,
        ),
    );
    data_ok(exit, &saved, "mapped comment save");

    let (exit, comments) = run_json(
        root,
        &["comments", "list", "--draft", &current.id.to_string()],
    );
    assert_eq!(exit, 0, "{comments}");
    let rows = comments["data"]["comments"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["original_quote"], "alpha");
    assert_eq!(
        rows[0]["anchor"],
        serde_json::json!({"state": "anchored", "start": 2, "end": 7})
    );
    assert_eq!(rows[1]["original_quote"], "beta");
    assert_eq!(
        rows[1]["anchor"],
        serde_json::json!({"state": "anchored", "start": 8, "end": 12})
    );
    assert_eq!(rows[0]["original_start"], 0);
    assert_eq!(rows[0]["original_end"], 5);
    assert_eq!(rows[1]["original_start"], 6);
    assert_eq!(rows[1]["original_end"], 10);
    assert_eq!(
        rows[0]["original_sha256_hex"],
        rows[1]["original_sha256_hex"]
    );

    for row in rows {
        let revision = row["original_revision_id"].as_str().unwrap();
        let (exit, shown) = run_json(root, &["revisions", "show", revision]);
        assert_eq!(exit, 0, "{shown}");
        assert_eq!(shown["data"]["content"], "alpha beta gamma");
    }
}

#[test]
fn save_keeps_deleted_and_touched_comment_states_and_original_evidence() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let current = seed_alpha_beta_comments(root);
    let deleted_file = root.join("deleted.txt");
    fs::write(&deleted_file, "alpha  gamma").unwrap();
    let (exit, deleted) = run_owned_args(
        root,
        &save_args(
            &current.id.to_string(),
            &current.stamp.base_revision.to_string(),
            current.stamp.generation,
            current.stamp.generation + 1,
            &deleted_file,
            None,
        ),
    );
    let deleted_data = data_ok(exit, &deleted, "deleted comment save");

    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let after_delete = workspace.draft(current.id).unwrap().unwrap();
    let gamma = workspace
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: current.id,
            expected: after_delete.stamp,
            generation: after_delete.stamp.generation,
            text: after_delete.text.clone(),
            edits: EditTrace::Steps(vec![]),
            range: 7..12,
            quote: "gamma".into(),
            body: "gamma evidence".into(),
        })
        .unwrap();
    drop(workspace);

    let touched_file = root.join("touched.txt");
    fs::write(&touched_file, "alpha  Xamma").unwrap();
    let (exit, touched) = run_owned_args(
        root,
        &save_args(
            &current.id.to_string(),
            &gamma.saved.draft.stamp.base_revision.to_string(),
            gamma.saved.draft.stamp.generation,
            gamma.saved.draft.stamp.generation + 1,
            &touched_file,
            None,
        ),
    );
    let touched_data = data_ok(exit, &touched, "touched comment save");
    assert_eq!(touched_data["generation"], 2);
    assert_eq!(deleted_data["generation"], 1);

    let (exit, comments) = run_json(
        root,
        &["comments", "list", "--draft", &current.id.to_string()],
    );
    assert_eq!(exit, 0, "{comments}");
    let rows = comments["data"]["comments"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["original_quote"], "alpha");
    assert_eq!(rows[0]["anchor"]["state"], "anchored");
    assert_eq!(rows[1]["original_quote"], "beta");
    assert_eq!(rows[1]["anchor"], serde_json::json!({"state": "deleted"}));
    assert_eq!(rows[2]["original_quote"], "gamma");
    assert_eq!(
        rows[2]["anchor"],
        serde_json::json!({"state": "ambiguous", "reason": "touched"})
    );
    assert_eq!(
        rows[0]["original_sha256_hex"],
        rows[1]["original_sha256_hex"]
    );
    assert_eq!(rows[2]["original_quote"], "gamma");

    let beta_revision = rows[1]["original_revision_id"].as_str().unwrap();
    let (exit, beta_original) = run_json(root, &["revisions", "show", beta_revision]);
    assert_eq!(exit, 0, "{beta_original}");
    assert_eq!(beta_original["data"]["content"], "alpha beta gamma");
    let gamma_revision = rows[2]["original_revision_id"].as_str().unwrap();
    let (exit, gamma_original) = run_json(root, &["revisions", "show", gamma_revision]);
    assert_eq!(exit, 0, "{gamma_original}");
    assert_eq!(gamma_original["data"]["content"], "alpha  gamma");
    assert_eq!(
        rows[2]["original_sha256_hex"],
        hex(&gamma.saved.draft.sha256)
    );
}

struct FreshSaveInput {
    _data_parent: TempDir,
    data: std::path::PathBuf,
    _fixtures: TempDir,
}

fn fresh_save_input() -> FreshSaveInput {
    let data_parent = tempdir().unwrap();
    let data = data_parent.path().join("data");
    fs::create_dir(&data).unwrap();
    FreshSaveInput {
        _data_parent: data_parent,
        data,
        _fixtures: tempdir().unwrap(),
    }
}

#[test]
fn save_rejects_generation_out_of_range_before_workspace_access() {
    let input = fresh_save_input();
    let file = input._fixtures.path().join("save.txt");
    fs::write(&file, "save").unwrap();
    let args = save_args_raw(
        &Uuid::new_v4().to_string(),
        &Uuid::new_v4().to_string(),
        "0",
        "18446744073709551615",
        &file,
        None,
    );
    let (exit, envelope) = run_owned_args(&input.data, &args);
    assert_eq!(exit, 2, "{envelope}");
    assert_eq!(envelope["error"]["code"], "USAGE");
    assert!(fs::read_dir(&input.data).unwrap().next().is_none());
}
