//! Subprocess acceptance tests for `brn comments reopen COMMENT_ID`.
//!
//! Each CLI invocation is a fresh process, so the lifecycle assertions also
//! cover the persisted receipt and current state after restart.
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

fn lifecycle_args(
    command: &str,
    fixture: &Fixture,
    expected_status_version: u64,
    operation: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "comments".to_string(),
        command.to_string(),
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
        assert_eq!(after[field], before[field], "reopen changed {field}");
    }
}

fn resolve_via_workflow(root: &Path, fixture: &Fixture, operation: Uuid) {
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let result = workspace
        .set_comment_status(CommentStatusChange {
            op: operation,
            draft_id: fixture.draft.parse().unwrap(),
            comment_id: fixture.comment.parse().unwrap(),
            expected_status_version: 2,
            status: CommentStatus::Resolved,
        })
        .unwrap();
    assert_eq!(result.comment.status, CommentStatus::Resolved);
    assert_eq!(result.comment.status_version, 3);
}

#[test]
fn reopen_resolves_then_reopens_and_preserves_evidence_and_draft_after_restart() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let fixture = create_and_add_via_cli(root);

    let (exit, resolved) = run_owned(root, &lifecycle_args("resolve", &fixture, 0, None));
    let resolved_data = data_ok(exit, &resolved, "comment resolve");
    assert_eq!(resolved_data["comment"]["status"], "resolved");
    assert_eq!(resolved_data["comment"]["status_version"], 1);

    let operation = Uuid::new_v4().to_string();
    let (exit, reopened) = run_owned(
        root,
        &lifecycle_args("reopen", &fixture, 1, Some(&operation)),
    );
    let reopened_data = data_ok(exit, &reopened, "comment reopen");
    assert_eq!(reopened["command"], "comments.reopen");
    assert_eq!(reopened_data["operation_id"], operation);
    assert_eq!(reopened_data["comment"]["status"], "open");
    assert_eq!(reopened_data["comment"]["status_version"], 2);
    assert_immutable_fields(&fixture.original_comment, &reopened_data["comment"]);

    let (exit, listed) = run_json(root, &["comments", "list", "--draft", &fixture.draft]);
    let listed_comment = &data_ok(exit, &listed, "comments list after reopen")["comments"][0];
    assert_eq!(listed_comment["status"], "open");
    assert_eq!(listed_comment["status_version"], 2);
    assert_immutable_fields(&fixture.original_comment, listed_comment);

    let (exit, shown) = run_json(root, &["drafts", "show", &fixture.draft]);
    assert_eq!(data_ok(exit, &shown, "draft after reopen")["content"], TEXT);

    let (exit, replayed) = run_owned(
        root,
        &lifecycle_args("reopen", &fixture, 1, Some(&operation)),
    );
    assert_eq!(exit, 0, "reopen replay after restart: {replayed}");
    assert_eq!(replayed["data"], *reopened_data);
}

#[test]
fn reopen_enforces_identity_status_version_and_conflicting_operation_payload() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let fixture = create_and_add_via_cli(root);

    let (_exit, _) = run_owned(root, &lifecycle_args("resolve", &fixture, 0, None));

    let wrong_draft = Fixture {
        draft: Uuid::new_v4().to_string(),
        comment: fixture.comment.clone(),
        original_comment: fixture.original_comment.clone(),
    };
    let (exit, wrong_draft_result) =
        run_owned(root, &lifecycle_args("reopen", &wrong_draft, 1, None));
    assert_eq!(exit, 1, "{wrong_draft_result}");
    assert_eq!(wrong_draft_result["error"]["code"], "WORKFLOW_ERROR");

    let wrong_comment = Fixture {
        draft: fixture.draft.clone(),
        comment: Uuid::new_v4().to_string(),
        original_comment: fixture.original_comment.clone(),
    };
    let (exit, wrong_comment_result) =
        run_owned(root, &lifecycle_args("reopen", &wrong_comment, 1, None));
    assert_eq!(exit, 1, "{wrong_comment_result}");
    assert_eq!(wrong_comment_result["error"]["code"], "WORKFLOW_ERROR");

    let (exit, stale) = run_owned(root, &lifecycle_args("reopen", &fixture, 0, None));
    assert_eq!(exit, 1, "{stale}");
    assert_eq!(stale["error"]["code"], "WORKFLOW_ERROR");

    let operation = Uuid::new_v4().to_string();
    let (exit, first) = run_owned(
        root,
        &lifecycle_args("reopen", &fixture, 1, Some(&operation)),
    );
    let first_data = data_ok(exit, &first, "first reopen").clone();

    let (exit, same_status) = run_owned(root, &lifecycle_args("reopen", &fixture, 2, None));
    let same_status_data = data_ok(exit, &same_status, "already-open reopen");
    assert_ne!(same_status_data["operation_id"], operation);
    assert!(same_status_data["operation_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .is_ok());
    assert_eq!(same_status_data["comment"]["status"], "open");
    assert_eq!(same_status_data["comment"]["status_version"], 2);
    assert_immutable_fields(&fixture.original_comment, &same_status_data["comment"]);

    resolve_via_workflow(root, &fixture, Uuid::new_v4());

    let (exit, replay) = run_owned(
        root,
        &lifecycle_args("reopen", &fixture, 1, Some(&operation)),
    );
    assert_eq!(exit, 0, "replay after later status change: {replay}");
    assert_eq!(replay["data"], first_data);

    let (exit, conflict) = run_owned(
        root,
        &lifecycle_args("reopen", &fixture, 2, Some(&operation)),
    );
    assert_eq!(exit, 1, "{conflict}");
    assert_eq!(conflict["error"]["code"], "OPERATION_CONFLICT");
}
