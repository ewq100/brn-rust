//! Subprocess acceptance tests for `brn drafts checkpoint DRAFT_ID`.
//! Workspaces use the shared workflow for setup and are always dropped before
//! the CLI subprocess takes the exclusive ownership lock.
use brn_workflow::{CommentCapture, Config, EditTrace, Workspace};
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

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn create_via_cli(root: &Path, text: &str) -> (String, String) {
    let file = root.join("create.txt");
    fs::write(&file, text).unwrap();
    let (exit, envelope) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "checkpoint test",
            "--text-file",
            file.to_str().unwrap(),
        ],
    );
    assert_eq!(exit, 0, "{envelope}");
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["command"], "drafts.create");
    let data = &envelope["data"];
    (
        data["id"].as_str().unwrap().to_string(),
        data["base_revision"].as_str().unwrap().to_string(),
    )
}

fn checkpoint_args_raw(
    draft: &str,
    base: &str,
    expected_generation: &str,
    generation: &str,
    text_file: &Path,
    operation: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "drafts".to_string(),
        "checkpoint".to_string(),
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

fn checkpoint_args(
    draft: &str,
    base: &str,
    expected_generation: u64,
    generation: u64,
    text_file: &Path,
    operation: Option<&str>,
) -> Vec<String> {
    checkpoint_args_raw(
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

struct FreshCheckpointInput {
    _data_parent: TempDir,
    data: std::path::PathBuf,
    _fixtures: TempDir,
    text_file: std::path::PathBuf,
}

fn fresh_checkpoint_input() -> FreshCheckpointInput {
    let data_parent = tempdir().unwrap();
    let data = data_parent.path().join("data");
    fs::create_dir(&data).unwrap();
    let fixtures = tempdir().unwrap();
    let text_file = fixtures.path().join("checkpoint.txt");
    fs::write(&text_file, "checkpoint\n").unwrap();
    FreshCheckpointInput {
        _data_parent: data_parent,
        data,
        _fixtures: fixtures,
        text_file,
    }
}

fn assert_out_of_range_generation_rejected(flag: &str, value: &str, input: &FreshCheckpointInput) {
    let draft = Uuid::new_v4().to_string();
    let base = Uuid::new_v4().to_string();
    let expected_generation = if flag == "expected-generation" {
        value
    } else {
        "0"
    };
    let generation = if flag == "generation" { value } else { "1" };
    let args = checkpoint_args_raw(
        &draft,
        &base,
        expected_generation,
        generation,
        &input.text_file,
        Some(&Uuid::new_v4().to_string()),
    );

    let (exit, envelope) = run_owned_args(&input.data, &args);
    let entries: Vec<_> = fs::read_dir(&input.data)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.is_empty(),
        "{flag}={value} opened the workspace: exit={exit}, envelope={envelope}, entries={entries:?}"
    );
    assert_eq!(exit, 2, "{envelope}");
    assert_eq!(envelope["error"]["code"], "USAGE");
}

#[test]
fn expected_generation_i64_max_plus_one_is_rejected_before_workspace_access() {
    let input = fresh_checkpoint_input();
    assert_out_of_range_generation_rejected("expected-generation", "9223372036854775808", &input);
}

#[test]
fn expected_generation_u64_max_is_rejected_before_workspace_access() {
    let input = fresh_checkpoint_input();
    assert_out_of_range_generation_rejected("expected-generation", "18446744073709551615", &input);
}

#[test]
fn generation_i64_max_plus_one_is_rejected_before_workspace_access() {
    let input = fresh_checkpoint_input();
    assert_out_of_range_generation_rejected("generation", "9223372036854775808", &input);
}

#[test]
fn generation_u64_max_is_rejected_before_workspace_access() {
    let input = fresh_checkpoint_input();
    assert_out_of_range_generation_rejected("generation", "18446744073709551615", &input);
}

#[test]
fn checkpoint_creates_immutable_revision_and_reopens_exact_content() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let text = "Tere\r\nÖö ✓\r\n";
    let (draft, root_revision) = create_via_cli(root, text);
    let file = root.join("checkpoint.txt");
    fs::write(&file, text).unwrap();
    let operation = Uuid::new_v4().to_string();
    let args = checkpoint_args(&draft, &root_revision, 0, 1, &file, Some(&operation));

    let (exit, envelope) = run_owned_args(root, &args);
    assert_eq!(exit, 0, "{envelope}");
    assert_eq!(envelope["command"], "drafts.checkpoint");
    assert_eq!(envelope["ok"], true);
    let data = &envelope["data"];
    assert_eq!(data["operation_id"], operation);
    assert_eq!(data["id"], draft);
    assert_eq!(data["base_revision"], data["checkpoint_id"]);
    assert_ne!(data["checkpoint_id"], root_revision);
    assert_eq!(data["generation"], 1);

    let (exit, reopened) = run_json(root, &["drafts", "show", &draft]);
    assert_eq!(exit, 0, "{reopened}");
    assert_eq!(reopened["data"]["content"], text);
    assert_eq!(reopened["data"]["sha256_hex"], data["sha256_hex"]);
    assert_eq!(reopened["data"]["base_revision"], data["checkpoint_id"]);
    assert_eq!(reopened["data"]["generation"], 1);

    let (exit, revisions) = run_json(root, &["revisions", "list", "--draft", &draft]);
    assert_eq!(exit, 0, "{revisions}");
    let rows = revisions["data"]["revisions"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["id"], data["checkpoint_id"]);
    assert_eq!(rows[1]["parent_id"], root_revision);
    assert_eq!(rows[1]["kind"], "checkpoint");

    let checkpoint_id = data["checkpoint_id"].as_str().unwrap();
    let (exit, revision) = run_json(root, &["revisions", "show", checkpoint_id]);
    assert_eq!(exit, 0, "{revision}");
    assert_eq!(revision["data"]["content"], text);
}

#[test]
fn checkpoint_preserves_the_existing_unchanged_text_generation_rule() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, root_revision) = create_via_cli(root, "same");
    let file = root.join("same.txt");
    fs::write(&file, "same").unwrap();
    let args = checkpoint_args(&draft, &root_revision, 0, 0, &file, None);

    let (exit, envelope) = run_owned_args(root, &args);
    assert_eq!(exit, 0, "{envelope}");
    assert_eq!(envelope["data"]["generation"], 0);
    assert!(envelope["data"]["operation_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .is_ok());
    assert_ne!(envelope["data"]["checkpoint_id"], root_revision);
}

#[test]
fn checkpoint_rejects_stale_base_and_generation_without_extra_revision() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, root_revision) = create_via_cli(root, "first");
    let first_file = root.join("first-checkpoint.txt");
    fs::write(&first_file, "first checkpoint").unwrap();
    let first_args = checkpoint_args(&draft, &root_revision, 0, 1, &first_file, None);
    let (exit, first) = run_owned_args(root, &first_args);
    assert_eq!(exit, 0, "{first}");
    let current_base = first["data"]["checkpoint_id"].as_str().unwrap().to_string();

    let stale_base_file = root.join("stale-base.txt");
    fs::write(&stale_base_file, "stale base").unwrap();
    let stale_base = checkpoint_args(&draft, &root_revision, 1, 2, &stale_base_file, None);
    let (exit, envelope) = run_owned_args(root, &stale_base);
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");

    let stale_generation_file = root.join("stale-generation.txt");
    fs::write(&stale_generation_file, "stale generation").unwrap();
    let stale_generation =
        checkpoint_args(&draft, &current_base, 0, 2, &stale_generation_file, None);
    let (exit, envelope) = run_owned_args(root, &stale_generation);
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");

    let (exit, revisions) = run_json(root, &["revisions", "list", "--draft", &draft]);
    assert_eq!(exit, 0, "{revisions}");
    assert_eq!(revisions["data"]["revisions"].as_array().unwrap().len(), 2);
}

#[test]
fn checkpoint_rejects_a_base_revision_from_another_draft() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (first_draft, first_base) = create_via_cli(root, "first");
    let second_file = root.join("second.txt");
    fs::write(&second_file, "second").unwrap();
    let (exit, second_created) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "second",
            "--text-file",
            second_file.to_str().unwrap(),
        ],
    );
    assert_eq!(exit, 0, "{second_created}");
    let second_draft = second_created["data"]["id"].as_str().unwrap();
    let second_base = second_created["data"]["base_revision"].as_str().unwrap();
    let changed_file = root.join("wrong-draft.txt");
    fs::write(&changed_file, "changed").unwrap();
    let args = checkpoint_args(&first_draft, second_base, 0, 1, &changed_file, None);

    let (exit, envelope) = run_owned_args(root, &args);
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    let (exit, shown) = run_json(root, &["drafts", "show", &first_draft]);
    assert_eq!(exit, 0, "{shown}");
    assert_eq!(shown["data"]["base_revision"], first_base);
    assert_eq!(shown["data"]["generation"], 0);
    let (exit, second_shown) = run_json(root, &["drafts", "show", second_draft]);
    assert_eq!(exit, 0, "{second_shown}");
    assert_eq!(second_shown["data"]["content"], "second");
}

#[test]
fn checkpoint_replay_after_later_edit_and_restart_returns_original_receipt() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, root_revision) = create_via_cli(root, "before");
    let file = root.join("replay.txt");
    fs::write(&file, "checkpoint").unwrap();
    let operation = Uuid::new_v4().to_string();
    let args = checkpoint_args(&draft, &root_revision, 0, 1, &file, Some(&operation));
    let (exit, first) = run_owned_args(root, &args);
    assert_eq!(exit, 0, "{first}");
    let first_data = first["data"].clone();

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
    assert_eq!(shown["data"]["base_revision"], first_data["checkpoint_id"]);
}

#[test]
fn checkpoint_conflicting_operation_reuse_is_rejected_and_state_stays_frozen() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, root_revision) = create_via_cli(root, "before");
    let operation = Uuid::new_v4().to_string();
    let first_file = root.join("first.txt");
    fs::write(&first_file, "first").unwrap();
    let first_args = checkpoint_args(&draft, &root_revision, 0, 1, &first_file, Some(&operation));
    let (exit, first) = run_owned_args(root, &first_args);
    assert_eq!(exit, 0, "{first}");

    let conflicting_file = root.join("conflicting.txt");
    fs::write(&conflicting_file, "conflicting").unwrap();
    let conflicting = checkpoint_args(
        &draft,
        &root_revision,
        0,
        1,
        &conflicting_file,
        Some(&operation),
    );
    let (exit, envelope) = run_owned_args(root, &conflicting);
    assert_eq!(exit, 1, "{envelope}");
    assert_eq!(envelope["error"]["code"], "OPERATION_CONFLICT");

    let (exit, shown) = run_json(root, &["drafts", "show", &draft]);
    assert_eq!(exit, 0, "{shown}");
    assert_eq!(shown["data"]["content"], "first");
    assert_eq!(
        shown["data"]["base_revision"],
        first["data"]["checkpoint_id"]
    );
}

#[test]
fn checkpoint_preserves_existing_comment_provenance_and_anchor() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let draft = workspace
        .create_draft(Uuid::new_v4(), "commented", "alpha beta")
        .unwrap();
    let comment = workspace
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: draft.stamp,
            generation: 0,
            text: draft.text.clone(),
            edits: EditTrace::Steps(vec![]),
            range: 6..10,
            quote: "beta".into(),
            body: "keep this provenance".into(),
        })
        .unwrap();
    let commented = comment.saved.draft;
    drop(workspace);

    let file = root.join("comment-checkpoint.txt");
    fs::write(&file, commented.text.as_bytes()).unwrap();
    let args = checkpoint_args(
        &draft.id.to_string(),
        &commented.stamp.base_revision.to_string(),
        commented.stamp.generation,
        commented.stamp.generation + 1,
        &file,
        None,
    );
    let (exit, checkpoint) = run_owned_args(root, &args);
    assert_eq!(exit, 0, "{checkpoint}");
    assert_eq!(checkpoint["data"]["generation"], 1);

    let (exit, comments) = run_json(
        root,
        &["comments", "list", "--draft", &draft.id.to_string()],
    );
    assert_eq!(exit, 0, "{comments}");
    let row = &comments["data"]["comments"][0];
    assert_eq!(
        row["original_revision_id"],
        commented.stamp.base_revision.to_string()
    );
    assert_eq!(row["original_sha256_hex"], hex(&commented.sha256));
    assert_eq!(row["original_quote"], "beta");
    assert_eq!(row["anchor"]["state"], "anchored");
    assert_eq!(row["anchor"]["start"], 6);
    assert_eq!(row["anchor"]["end"], 10);
}
