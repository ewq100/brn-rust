//! Subprocess acceptance tests for `brn comments resolve COMMENT_ID`.
//!
//! Draft and comment fixtures are built through the public CLI. The only
//! direct workflow setup is the later reopen needed to prove that replay does
//! not precheck the current comment state.
use brn_workflow::{CommentStatus, CommentStatusChange, Config, Workspace};
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::tempdir;
use uuid::Uuid;

const TEXT: &str = "Pealkiri\nEesti jõgi voolab vaikselt.\nLõpp.\n";
const QUOTE: &str = "jõgi voolab";
const START: usize = 15;
const END: usize = 27;

struct Fixture {
    draft: String,
    comment: String,
    original_comment: Value,
}

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

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).expect("exactly one JSON envelope")
}

fn run_json(root: &Path, args: &[&str]) -> (i32, Value) {
    let mut all = args.to_vec();
    all.extend_from_slice(&["--data-dir", root.to_str().unwrap(), "--json"]);
    let out = brn(&all);
    (code(&out), json(&out))
}

fn run_owned(root: &Path, args: &[String]) -> (i32, Value) {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    run_json(root, &borrowed)
}

fn data_ok<'a>(exit: i32, envelope: &'a Value, context: &str) -> &'a Value {
    assert_eq!(exit, 0, "{context}: {envelope}");
    assert_eq!(envelope["ok"], true, "{context}: {envelope}");
    &envelope["data"]
}

fn create_and_add_via_cli(root: &Path) -> Fixture {
    let text_file = root.join("draft.txt");
    let quote_file = root.join("quote.txt");
    let body_file = root.join("body.txt");
    fs::write(&text_file, TEXT).unwrap();
    fs::write(&quote_file, QUOTE).unwrap();
    fs::write(&body_file, "Kontrolli seda lõiku.").unwrap();

    let (exit, created) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "Eesti kommentaar",
            "--text-file",
            text_file.to_str().unwrap(),
        ],
    );
    let created_data = data_ok(exit, &created, "draft creation");
    let draft = created_data["id"].as_str().unwrap().to_string();
    let base = created_data["base_revision"].as_str().unwrap().to_string();

    let add_args = vec![
        "comments".to_string(),
        "add".to_string(),
        "--draft".to_string(),
        draft.clone(),
        "--base-revision".to_string(),
        base,
        "--expected-generation".to_string(),
        "0".to_string(),
        "--generation".to_string(),
        "0".to_string(),
        "--text-file".to_string(),
        text_file.to_str().unwrap().to_string(),
        "--start-byte".to_string(),
        START.to_string(),
        "--end-byte".to_string(),
        END.to_string(),
        "--quote-file".to_string(),
        quote_file.to_str().unwrap().to_string(),
        "--body-file".to_string(),
        body_file.to_str().unwrap().to_string(),
    ];
    let (exit, added) = run_owned(root, &add_args);
    let added_data = data_ok(exit, &added, "comment creation");
    let comment = added_data["comment_id"].as_str().unwrap().to_string();

    Fixture {
        draft,
        comment,
        original_comment: added_data["comments"][0].clone(),
    }
}

fn resolve_args(
    fixture: &Fixture,
    expected_status_version: u64,
    operation: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "comments".to_string(),
        "resolve".to_string(),
        fixture.comment.clone(),
        "--draft".to_string(),
        fixture.draft.clone(),
        "--expected-status-version".to_string(),
        expected_status_version.to_string(),
    ];
    if let Some(operation) = operation {
        args.extend(["--operation".to_string(), operation.to_string()]);
    }
    args
}

fn assert_immutable_fields(before: &Value, after: &Value) {
    for field in [
        "id",
        "original_revision_id",
        "original_sha256_hex",
        "original_start",
        "original_end",
        "original_quote",
        "body",
    ] {
        assert_eq!(after[field], before[field], "resolve changed {field}");
    }
}

#[test]
fn resolve_open_comment_returns_operation_and_reopens_listing_unchanged() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let fixture = create_and_add_via_cli(root);

    let (exit, resolved) = run_owned(root, &resolve_args(&fixture, 0, None));
    let data = data_ok(exit, &resolved, "comment resolve");
    assert_eq!(resolved["command"], "comments.resolve");
    assert!(data["operation_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .is_ok());
    assert_eq!(data["comment"]["id"], fixture.comment);
    assert_eq!(data["comment"]["status"], "resolved");
    assert_eq!(data["comment"]["status_version"], 1);
    assert_immutable_fields(&fixture.original_comment, &data["comment"]);

    let (exit, listed) = run_json(root, &["comments", "list", "--draft", &fixture.draft]);
    let list_data = data_ok(exit, &listed, "comments list after restart");
    let listed_comment = &list_data["comments"][0];
    assert_eq!(listed_comment["status"], "resolved");
    assert_eq!(listed_comment["status_version"], 1);
    assert_immutable_fields(&fixture.original_comment, listed_comment);

    let (exit, shown) = run_json(root, &["drafts", "show", &fixture.draft]);
    let shown_data = data_ok(exit, &shown, "draft after resolve");
    assert_eq!(shown_data["content"], TEXT);
}

#[test]
fn resolve_rejects_stale_status_and_wrong_draft_or_comment_identity() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let fixture = create_and_add_via_cli(root);

    let wrong_draft = Fixture {
        draft: Uuid::new_v4().to_string(),
        comment: fixture.comment.clone(),
        original_comment: fixture.original_comment.clone(),
    };
    let (exit, wrong_draft_result) = run_owned(root, &resolve_args(&wrong_draft, 0, None));
    assert_eq!(exit, 1, "{wrong_draft_result}");
    assert_eq!(wrong_draft_result["error"]["code"], "WORKFLOW_ERROR");

    let wrong_comment = Fixture {
        draft: fixture.draft.clone(),
        comment: Uuid::new_v4().to_string(),
        original_comment: fixture.original_comment.clone(),
    };
    let (exit, wrong_comment_result) = run_owned(root, &resolve_args(&wrong_comment, 0, None));
    assert_eq!(exit, 1, "{wrong_comment_result}");
    assert_eq!(wrong_comment_result["error"]["code"], "WORKFLOW_ERROR");

    let (exit, resolved) = run_owned(root, &resolve_args(&fixture, 0, None));
    data_ok(exit, &resolved, "valid resolve");
    let (exit, stale) = run_owned(root, &resolve_args(&fixture, 0, None));
    assert_eq!(exit, 1, "{stale}");
    assert_eq!(stale["error"]["code"], "WORKFLOW_ERROR");

    let (exit, listed) = run_json(root, &["comments", "list", "--draft", &fixture.draft]);
    let comment = &data_ok(exit, &listed, "comments after rejected resolves")["comments"][0];
    assert_eq!(comment["status"], "resolved");
    assert_eq!(comment["status_version"], 1);
}

#[test]
fn resolve_replays_same_operation_after_later_reopen() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let fixture = create_and_add_via_cli(root);
    let operation = Uuid::new_v4().to_string();
    let args = resolve_args(&fixture, 0, Some(&operation));

    let (exit, first) = run_owned(root, &args);
    let first_data = data_ok(exit, &first, "first resolve").clone();

    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let reopened = workspace
        .set_comment_status(CommentStatusChange {
            op: Uuid::new_v4(),
            draft_id: fixture.draft.parse().unwrap(),
            comment_id: fixture.comment.parse().unwrap(),
            expected_status_version: 1,
            status: CommentStatus::Open,
        })
        .unwrap();
    assert_eq!(reopened.comment.status, CommentStatus::Open);
    assert_eq!(reopened.comment.status_version, 2);
    drop(workspace);

    let (exit, replay) = run_owned(root, &args);
    assert_eq!(exit, 0, "{replay}");
    assert_eq!(replay["data"], first_data);

    let (exit, listed) = run_json(root, &["comments", "list", "--draft", &fixture.draft]);
    let comment = &data_ok(exit, &listed, "comments after replay")["comments"][0];
    assert_eq!(comment["status"], "open");
    assert_eq!(comment["status_version"], 2);
}

#[test]
fn resolve_rejects_conflicting_operation_reuse() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let fixture = create_and_add_via_cli(root);
    let operation = Uuid::new_v4().to_string();
    let (exit, first) = run_owned(root, &resolve_args(&fixture, 0, Some(&operation)));
    data_ok(exit, &first, "first resolve");

    let (exit, conflict) = run_owned(root, &resolve_args(&fixture, 1, Some(&operation)));
    assert_eq!(exit, 1, "{conflict}");
    assert_eq!(
        conflict["error"]["code"], "OPERATION_CONFLICT",
        "{conflict}"
    );

    let (exit, listed) = run_json(root, &["comments", "list", "--draft", &fixture.draft]);
    let comment = &data_ok(exit, &listed, "comments after conflict")["comments"][0];
    assert_eq!(comment["status"], "resolved");
    assert_eq!(comment["status_version"], 1);
}

#[test]
fn resolve_same_status_is_a_no_op_with_a_new_operation_receipt() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let fixture = create_and_add_via_cli(root);

    let first_operation = Uuid::new_v4().to_string();
    let (exit, first) = run_owned(root, &resolve_args(&fixture, 0, Some(&first_operation)));
    data_ok(exit, &first, "first resolve");

    let (exit, same_status) = run_owned(root, &resolve_args(&fixture, 1, None));
    let data = data_ok(exit, &same_status, "same-status resolve");
    assert_ne!(data["operation_id"], first_operation);
    assert_eq!(data["comment"]["status"], "resolved");
    assert_eq!(data["comment"]["status_version"], 1);
    assert_immutable_fields(&fixture.original_comment, &data["comment"]);
}
