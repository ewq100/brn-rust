//! Subprocess acceptance tests for `brn comments add --draft DRAFT_ID`.
//! Workspaces are seeded through the CLI where possible and reopened through
//! separate CLI processes so the command's durable receipt and provenance are
//! exercised at its public boundary.
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
const ORIGINAL_SHA256: &str = "2420347a76e1d88c8f47ec61dfffeccd3a0947c031d124fc27b979e51f0f97c2";

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

fn data_ok<'a>(exit: i32, envelope: &'a Value, context: &str) -> &'a Value {
    assert_eq!(exit, 0, "{context}: {envelope}");
    assert_eq!(envelope["ok"], true, "{context}: {envelope}");
    &envelope["data"]
}

fn create_via_cli(root: &Path, text: &str) -> (String, String) {
    let file = root.join("draft.txt");
    fs::write(&file, text).unwrap();
    let (exit, envelope) = run_json(
        root,
        &[
            "drafts",
            "create",
            "--title",
            "Eesti kommentaar",
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

fn add_args(
    draft: &str,
    base: &str,
    generation: u64,
    text_file: &Path,
    quote_file: &Path,
    body_file: &Path,
    operation: Option<&str>,
) -> Vec<String> {
    let mut args = vec![
        "comments".to_string(),
        "add".to_string(),
        "--draft".to_string(),
        draft.to_string(),
        "--base-revision".to_string(),
        base.to_string(),
        "--expected-generation".to_string(),
        generation.to_string(),
        "--generation".to_string(),
        generation.to_string(),
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
    if let Some(operation) = operation {
        args.extend(["--operation".to_string(), operation.to_string()]);
    }
    args
}

fn run_owned_args(root: &Path, args: &[String]) -> (i32, Value) {
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    run_json(root, &borrowed)
}

#[test]
fn add_preserves_estonian_provenance_and_reopens_comments() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, TEXT);
    let text_file = root.join("submitted.txt");
    let quote_file = root.join("quote.txt");
    let body_file = root.join("body.txt");
    fs::write(&text_file, TEXT).unwrap();
    fs::write(&quote_file, QUOTE).unwrap();
    fs::write(&body_file, "Kontrolli seda lõiku.").unwrap();
    let operation = Uuid::new_v4().to_string();
    let args = add_args(
        &draft,
        &base,
        0,
        &text_file,
        &quote_file,
        &body_file,
        Some(&operation),
    );

    let (exit, added) = run_owned_args(root, &args);
    let data = data_ok(exit, &added, "comment add");
    assert_eq!(added["command"], "comments.add");
    assert_eq!(data["operation_id"], operation);
    assert_eq!(data["submitted_generation"], 0);
    let comment_id = data["comment_id"].as_str().unwrap();
    assert_ne!(data["draft"]["base_revision"], base);

    let (exit, listed) = run_json(root, &["comments", "list", "--draft", &draft]);
    assert_eq!(exit, 0, "{listed}");
    let comments = listed["data"]["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 1);
    let comment = &comments[0];
    assert_eq!(comment["id"], comment_id);
    assert_eq!(
        comment["original_revision_id"],
        data["draft"]["base_revision"]
    );
    assert_eq!(comment["original_sha256_hex"], ORIGINAL_SHA256);
    assert_eq!(comment["original_start"], START);
    assert_eq!(comment["original_end"], END);
    assert_eq!(comment["original_quote"], QUOTE);
    assert_eq!(comment["body"], "Kontrolli seda lõiku.");
    assert_eq!(
        comment["anchor"],
        serde_json::json!({"state": "anchored", "start": START, "end": END})
    );

    let original_revision = comment["original_revision_id"].as_str().unwrap();
    let (exit, revision) = run_json(root, &["revisions", "show", original_revision]);
    assert_eq!(exit, 0, "{revision}");
    assert_eq!(revision["data"]["content"], TEXT);
}

#[test]
fn add_rejects_stale_and_wrong_draft_state_without_extra_comments() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, TEXT);
    let (_other_draft, other_base) = create_via_cli(root, "Teine mustand\n");
    let text_file = root.join("submitted.txt");
    let quote_file = root.join("quote.txt");
    let body_file = root.join("body.txt");
    fs::write(&text_file, TEXT).unwrap();
    fs::write(&quote_file, QUOTE).unwrap();
    fs::write(&body_file, "Märkus.").unwrap();

    let edited_text_file = root.join("edited.txt");
    fs::write(
        &edited_text_file,
        "Pealkiri\nEesti jõgi voolab leebelt.\nLõpp.\n",
    )
    .unwrap();
    let unsaved_edit = add_args(
        &draft,
        &base,
        0,
        &edited_text_file,
        &quote_file,
        &body_file,
        None,
    );
    let (exit, unsaved) = run_owned_args(root, &unsaved_edit);
    assert_eq!(exit, 1, "{unsaved}");
    assert_eq!(unsaved["error"]["code"], "WORKFLOW_ERROR");

    let wrong_draft = add_args(
        &draft,
        &other_base,
        0,
        &text_file,
        &quote_file,
        &body_file,
        None,
    );
    let (exit, wrong) = run_owned_args(root, &wrong_draft);
    assert_eq!(exit, 1, "{wrong}");
    assert_eq!(wrong["error"]["code"], "WORKFLOW_ERROR");

    let first = add_args(&draft, &base, 0, &text_file, &quote_file, &body_file, None);
    let (exit, first_added) = run_owned_args(root, &first);
    let first_data = data_ok(exit, &first_added, "first comment").clone();
    let stale = add_args(&draft, &base, 0, &text_file, &quote_file, &body_file, None);
    let (exit, stale_result) = run_owned_args(root, &stale);
    assert_eq!(exit, 1, "{stale_result}");
    assert_eq!(stale_result["error"]["code"], "WORKFLOW_ERROR");

    let (exit, listed) = run_json(root, &["comments", "list", "--draft", &draft]);
    assert_eq!(exit, 0, "{listed}");
    let comments = listed["data"]["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["id"], first_data["comment_id"]);
}

#[test]
fn add_rejects_invalid_ranges_and_mismatched_quotes() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, TEXT);
    let text_file = root.join("submitted.txt");
    let quote_file = root.join("quote.txt");
    let body_file = root.join("body.txt");
    fs::write(&text_file, TEXT).unwrap();
    fs::write(&quote_file, QUOTE).unwrap();
    fs::write(&body_file, "Märkus.").unwrap();

    for (start, end, quote) in [
        (17, END, QUOTE),
        (START, 17, QUOTE),
        (END, START, QUOTE),
        (START, 100, QUOTE),
        (START, END, "jõgi voolab!"),
    ] {
        let mut args = add_args(&draft, &base, 0, &text_file, &quote_file, &body_file, None);
        let start_pos = args.iter().position(|arg| arg == "--start-byte").unwrap() + 1;
        let end_pos = args.iter().position(|arg| arg == "--end-byte").unwrap() + 1;
        args[start_pos] = start.to_string();
        args[end_pos] = end.to_string();
        fs::write(&quote_file, quote).unwrap();
        let (exit, envelope) = run_owned_args(root, &args);
        assert_eq!(exit, 1, "{envelope}");
        assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    }

    let (exit, listed) = run_json(root, &["comments", "list", "--draft", &draft]);
    assert_eq!(exit, 0, "{listed}");
    assert!(listed["data"]["comments"].as_array().unwrap().is_empty());
}

#[test]
fn add_rejects_empty_and_oversize_bodies_before_workspace_access() {
    let parent = tempdir().unwrap();
    let root = parent.path().join("data");
    fs::create_dir(&root).unwrap();
    let fixtures = tempdir().unwrap();
    let text_file = fixtures.path().join("text.txt");
    let quote_file = fixtures.path().join("quote.txt");
    let empty_body = fixtures.path().join("empty-body.txt");
    let oversize_body = fixtures.path().join("oversize-body.txt");
    fs::write(&text_file, TEXT).unwrap();
    fs::write(&quote_file, QUOTE).unwrap();
    fs::write(&empty_body, "").unwrap();
    fs::write(&oversize_body, vec![b'x'; 64 * 1024 + 1]).unwrap();

    for body_file in [&empty_body, &oversize_body] {
        let args = add_args(
            &Uuid::new_v4().to_string(),
            &Uuid::new_v4().to_string(),
            0,
            &text_file,
            &quote_file,
            body_file,
            None,
        );
        let (exit, envelope) = run_owned_args(&root, &args);
        assert_eq!(exit, 1, "{envelope}");
        assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
        assert!(fs::read_dir(&root).unwrap().next().is_none());
    }
}

#[test]
fn add_replays_after_later_save_and_rejects_conflicting_operation_reuse() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, TEXT);
    let text_file = root.join("submitted.txt");
    let quote_file = root.join("quote.txt");
    let body_file = root.join("body.txt");
    fs::write(&text_file, TEXT).unwrap();
    fs::write(&quote_file, QUOTE).unwrap();
    fs::write(&body_file, "Märkus.").unwrap();
    let operation = Uuid::new_v4().to_string();
    let args = add_args(
        &draft,
        &base,
        0,
        &text_file,
        &quote_file,
        &body_file,
        Some(&operation),
    );
    let (exit, first) = run_owned_args(root, &args);
    let first_data = data_ok(exit, &first, "first comment").clone();
    let current_base = first_data["draft"]["base_revision"].as_str().unwrap();
    let later_file = root.join("later.txt");
    fs::write(&later_file, "Pealkiri\nEesti jõgi voolab kiirelt.\nLõpp.\n").unwrap();
    let (exit, later) = run_json(
        root,
        &[
            "drafts",
            "save",
            &draft,
            "--base-revision",
            current_base,
            "--expected-generation",
            "0",
            "--generation",
            "1",
            "--text-file",
            later_file.to_str().unwrap(),
        ],
    );
    assert_eq!(exit, 0, "{later}");

    let (exit, replay) = run_owned_args(root, &args);
    assert_eq!(exit, 0, "{replay}");
    assert_eq!(replay["data"], first_data);

    let conflicting_body = root.join("conflicting-body.txt");
    fs::write(&conflicting_body, "Teine märkus.").unwrap();
    let conflicting = add_args(
        &draft,
        &base,
        0,
        &text_file,
        &quote_file,
        &conflicting_body,
        Some(&operation),
    );
    let (exit, conflict) = run_owned_args(root, &conflicting);
    assert_eq!(exit, 1, "{conflict}");
    assert_eq!(conflict["error"]["code"], "OPERATION_CONFLICT");

    let (exit, shown) = run_json(root, &["drafts", "show", &draft]);
    assert_eq!(exit, 0, "{shown}");
    assert_eq!(shown["data"]["generation"], 1);
    assert_eq!(
        shown["data"]["content"],
        "Pealkiri\nEesti jõgi voolab kiirelt.\nLõpp.\n"
    );
}

#[test]
fn generated_operation_id_is_returned() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let (draft, base) = create_via_cli(root, TEXT);
    let text_file = root.join("submitted.txt");
    let quote_file = root.join("quote.txt");
    let body_file = root.join("body.txt");
    fs::write(&text_file, TEXT).unwrap();
    fs::write(&quote_file, QUOTE).unwrap();
    fs::write(&body_file, "Märkus.").unwrap();

    let (exit, envelope) = run_owned_args(
        root,
        &add_args(&draft, &base, 0, &text_file, &quote_file, &body_file, None),
    );
    let data = data_ok(exit, &envelope, "generated operation");
    assert!(data["operation_id"]
        .as_str()
        .unwrap()
        .parse::<Uuid>()
        .is_ok());
}
