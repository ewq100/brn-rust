//! Subprocess tests for the review surfaces: `drafts`, `comments`,
//! `revisions`. Workspaces are seeded in-process with `brn_workflow::Workspace`
//! in disposable temp dirs and dropped before the subprocess runs so it can
//! take the ownership lock.
use brn_workflow::{
    CommentCapture, CommentStatus, CommentStatusChange, Config, DraftWriteWithComments, EditTrace,
    TextEdit, Workspace,
};
use serde_json::Value;
use std::{
    path::Path,
    process::{Command, Output, Stdio},
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

fn one_json(out: &Output) -> Value {
    serde_json::from_str(&text(out)).expect("exactly one JSON object on stdout")
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn run_brn(root: &Path, args: &[&str]) -> Output {
    let mut all = args.to_vec();
    all.extend_from_slice(&["--data-dir", root.to_str().unwrap(), "--json"]);
    brn(&all)
}

#[test]
fn drafts_list_and_show_return_exact_content() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let contents = "héllo\r\nwörld ✓";
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let draft = workspace
        .create_draft(Uuid::new_v4(), "Spec draft", contents)
        .unwrap();
    drop(workspace);

    let out = run_brn(root, &["drafts", "list"]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "drafts.list");
    assert_eq!(envelope["ok"], true);
    let drafts = envelope["data"]["drafts"].as_array().unwrap();
    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0]["id"], draft.id.to_string());
    assert_eq!(drafts[0]["title"], "Spec draft");
    assert_eq!(drafts[0]["generation"], 0);
    assert_eq!(
        drafts[0]["base_revision"],
        draft.stamp.base_revision.to_string()
    );
    assert_eq!(drafts[0]["sha256_hex"], hex(&draft.sha256));
    assert!(drafts[0].get("content").is_none(), "list omits text");
    assert!(drafts[0].get("text").is_none(), "list omits text");

    let id = draft.id.to_string();
    let out = run_brn(root, &["drafts", "show", &id]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "drafts.show");
    assert_eq!(envelope["ok"], true);
    let data = &envelope["data"];
    assert_eq!(data["id"], id);
    assert_eq!(data["title"], "Spec draft");
    assert_eq!(data["generation"], 0);
    assert_eq!(data["sha256_hex"], hex(&draft.sha256));
    assert_eq!(data["content"], contents);
}

#[test]
fn drafts_show_unknown_draft_is_not_found() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    workspace
        .create_draft(Uuid::new_v4(), "present", "present\n")
        .unwrap();
    drop(workspace);
    let missing = Uuid::new_v4().to_string();
    let out = run_brn(root, &["drafts", "show", &missing]);
    assert_eq!(code(&out), 1);
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "drafts.show");
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "NOT_FOUND");
    assert!(envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
}

#[test]
fn comments_list_reports_exact_provenance_and_status() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let body = "alpha beta gamma";
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let draft = workspace
        .create_draft(Uuid::new_v4(), "reviewed", body)
        .unwrap();
    let first = workspace
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: draft.stamp,
            generation: draft.stamp.generation,
            text: body.to_string(),
            edits: EditTrace::Steps(vec![]),
            range: 6..10,
            quote: "beta".to_string(),
            body: "Check this claim".to_string(),
        })
        .unwrap();
    workspace
        .set_comment_status(CommentStatusChange {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            comment_id: first.comment_id,
            expected_status_version: 0,
            status: CommentStatus::Resolved,
        })
        .unwrap();
    let second = workspace
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: first.saved.draft.stamp,
            generation: first.saved.draft.stamp.generation,
            text: body.to_string(),
            edits: EditTrace::Steps(vec![]),
            range: 0..5,
            quote: "alpha".to_string(),
            body: "Open point".to_string(),
        })
        .unwrap();
    drop(workspace);

    let id = draft.id.to_string();
    let out = run_brn(root, &["comments", "list", "--draft", &id]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "comments.list");
    assert_eq!(envelope["ok"], true);
    let data = &envelope["data"];
    assert_eq!(data["draft"]["id"], id);
    assert_eq!(
        data["draft"]["generation"],
        first.saved.draft.stamp.generation
    );
    let comments = data["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 2, "{}", text(&out));

    // Insertion order: the resolved "beta" comment first, the open one second.
    assert_eq!(comments[0]["id"], first.comment_id.to_string());
    assert_eq!(comments[0]["original_quote"], "beta");
    assert_eq!(comments[0]["original_start"], 6);
    assert_eq!(comments[0]["original_end"], 10);
    assert_eq!(
        comments[0]["original_revision_id"],
        first.saved.draft.stamp.base_revision.to_string()
    );
    assert_eq!(
        comments[0]["original_sha256_hex"],
        hex(&first.saved.draft.sha256)
    );
    assert_eq!(comments[0]["status"], "resolved");
    assert_eq!(comments[0]["status_version"], 1);
    assert_eq!(comments[0]["anchor"]["state"], "anchored");
    assert_eq!(comments[0]["anchor"]["start"], 6);
    assert_eq!(comments[0]["anchor"]["end"], 10);
    assert!(comments[0]["anchor"].get("reason").is_none());

    assert_eq!(comments[1]["id"], second.comment_id.to_string());
    assert_eq!(comments[1]["status"], "open");
    assert_eq!(comments[1]["status_version"], 0);
    assert_eq!(comments[1]["original_quote"], "alpha");
    assert_eq!(comments[1]["anchor"]["state"], "anchored");
    assert_eq!(comments[1]["anchor"]["start"], 0);
    assert_eq!(comments[1]["anchor"]["end"], 5);
}

#[test]
fn comments_list_marks_deleted_and_ambiguous_anchors() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let draft = workspace
        .create_draft(Uuid::new_v4(), "edited", "alpha beta gamma")
        .unwrap();
    let first = workspace
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: draft.stamp,
            generation: draft.stamp.generation,
            text: "alpha beta gamma".to_string(),
            edits: EditTrace::Steps(vec![]),
            range: 6..10,
            quote: "beta".to_string(),
            body: "gone soon".to_string(),
        })
        .unwrap();
    // Remove "beta" entirely: the anchor becomes Deleted.
    let second = workspace
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: first.saved.draft.stamp,
            generation: 1,
            text: "alpha  gamma".to_string(),
            edits: EditTrace::Steps(vec![TextEdit {
                start: 6,
                end: 10,
                replacement: String::new(),
            }]),
            checkpoint: false,
        })
        .unwrap();
    let gamma = workspace
        .create_draft_comment(CommentCapture {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: second.draft.stamp,
            generation: second.draft.stamp.generation,
            text: second.draft.text.clone(),
            edits: EditTrace::Steps(vec![]),
            range: 7..12,
            quote: "gamma".to_string(),
            body: "touched soon".to_string(),
        })
        .unwrap();
    // Overlap the "gamma" quote with a partial replacement: Touched ambiguity.
    workspace
        .write_draft_with_comments(DraftWriteWithComments {
            op: Uuid::new_v4(),
            draft_id: draft.id,
            expected: gamma.saved.draft.stamp,
            generation: 2,
            text: "alpha  Xamma".to_string(),
            edits: EditTrace::Steps(vec![TextEdit {
                start: 7,
                end: 8,
                replacement: "X".to_string(),
            }]),
            checkpoint: false,
        })
        .unwrap();
    drop(workspace);

    let id = draft.id.to_string();
    let out = run_brn(root, &["comments", "list", "--draft", &id]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    let comments = envelope["data"]["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 2);
    assert_eq!(comments[0]["original_quote"], "beta");
    assert_eq!(comments[0]["anchor"]["state"], "deleted");
    assert!(comments[0]["anchor"].get("start").is_none());
    assert!(comments[0]["anchor"].get("reason").is_none());
    assert_eq!(comments[1]["original_quote"], "gamma");
    assert_eq!(comments[1]["anchor"]["state"], "ambiguous");
    assert_eq!(comments[1]["anchor"]["reason"], "touched");
    assert!(comments[1]["anchor"].get("start").is_none());
}

#[test]
fn revisions_list_and_show_report_lineage_and_exact_content() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let draft = workspace
        .create_draft(Uuid::new_v4(), "history", "line")
        .unwrap();
    let first = draft.stamp.base_revision;
    let checked = workspace
        .checkpoint_draft(Uuid::new_v4(), draft.id, draft.stamp, 1, "line\n")
        .unwrap();
    let second = checked.stamp.base_revision;
    drop(workspace);

    let id = draft.id.to_string();
    let out = run_brn(root, &["revisions", "list", "--draft", &id]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "revisions.list");
    assert_eq!(envelope["ok"], true);
    let revisions = envelope["data"]["revisions"].as_array().unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0]["id"], first.to_string());
    assert_eq!(revisions[0]["draft_id"], id);
    assert_eq!(revisions[0]["kind"], "checkpoint");
    assert!(revisions[0]["parent_id"].is_null());
    assert!(revisions[0]["origin_turn"].is_null());
    assert!(revisions[0].get("content").is_none(), "list omits text");
    assert_eq!(revisions[1]["id"], second.to_string());
    assert_eq!(revisions[1]["parent_id"], first.to_string());
    assert_eq!(revisions[1]["kind"], "checkpoint");

    let out = run_brn(root, &["revisions", "show", &second.to_string()]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "revisions.show");
    let data = &envelope["data"];
    assert_eq!(data["id"], second.to_string());
    assert_eq!(data["draft_id"], id);
    assert_eq!(data["parent_id"], first.to_string());
    assert_eq!(data["kind"], "checkpoint");
    assert_eq!(data["content"], "line\n");

    let missing = Uuid::new_v4().to_string();
    let out = run_brn(root, &["revisions", "show", &missing]);
    assert_eq!(code(&out), 1);
    let envelope = one_json(&out);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "NOT_FOUND");
}

#[test]
fn revisions_diff_covers_changes_noop_and_cross_draft_errors() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let a = workspace.create_draft(Uuid::new_v4(), "a", "line").unwrap();
    let a_first = a.stamp.base_revision;
    let a_checked = workspace
        .checkpoint_draft(Uuid::new_v4(), a.id, a.stamp, 1, "line\n")
        .unwrap();
    let a_second = a_checked.stamp.base_revision;
    let b = workspace
        .create_draft(Uuid::new_v4(), "b", "other")
        .unwrap();
    drop(workspace);

    // Real change between the two checkpoints of draft a.
    let (a_id, from, to) = (a.id.to_string(), a_first.to_string(), a_second.to_string());
    let out = run_brn(
        root,
        &[
            "revisions",
            "diff",
            "--draft",
            &a_id,
            "--from",
            &from,
            "--to",
            &to,
        ],
    );
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["command"], "revisions.diff");
    assert_eq!(envelope["ok"], true);
    let data = &envelope["data"];
    assert_eq!(data["draft_id"], a_id);
    assert_eq!(data["from"], from);
    assert_eq!(data["to"], to);
    let diff = data["diff"].as_str().unwrap();
    assert!(!diff.is_empty());
    assert!(diff.contains("-line"), "{diff}");
    assert!(diff.contains("+line"), "{diff}");

    // Same revision on both sides: the workflow's no-changes message.
    let out = run_brn(
        root,
        &[
            "revisions",
            "diff",
            "--draft",
            &a_id,
            "--from",
            &to,
            "--to",
            &to,
        ],
    );
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert!(envelope["data"]["diff"]
        .as_str()
        .unwrap()
        .contains("No changes"));

    // Revision of another draft: workflow error flows through classification.
    let b_root = b.stamp.base_revision.to_string();
    let out = run_brn(
        root,
        &[
            "revisions",
            "diff",
            "--draft",
            &a_id,
            "--from",
            &from,
            "--to",
            &b_root,
        ],
    );
    assert_eq!(code(&out), 1, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "WORKFLOW_ERROR");
    assert!(envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains("draft"));

    // Unknown --from revision.
    let missing = Uuid::new_v4().to_string();
    let out = run_brn(
        root,
        &[
            "revisions",
            "diff",
            "--draft",
            &a_id,
            "--from",
            &missing,
            "--to",
            &to,
        ],
    );
    assert_eq!(code(&out), 1);
    let envelope = one_json(&out);
    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["error"]["code"], "NOT_FOUND");
    assert!(envelope["error"]["message"]
        .as_str()
        .unwrap()
        .contains(&missing));
}

#[test]
fn empty_lists_are_ok() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let draft = workspace
        .create_draft(Uuid::new_v4(), "quiet", "just text\n")
        .unwrap();
    drop(workspace);

    let out = run_brn(
        root,
        &["comments", "list", "--draft", &draft.id.to_string()],
    );
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["comments"], serde_json::json!([]));
    assert_eq!(envelope["data"]["draft"]["id"], draft.id.to_string());

    // A workspace that has never seen a draft lists an empty array.
    let fresh = tempdir().unwrap();
    let out = run_brn(fresh.path(), &["drafts", "list"]);
    assert_eq!(code(&out), 0, "{}", text(&out));
    let envelope = one_json(&out);
    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"]["drafts"], serde_json::json!([]));
}

#[test]
fn text_mode_prints_to_stdout_with_empty_stderr() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let mut workspace = Workspace::open(root, Config::default()).unwrap();
    let draft = workspace
        .create_draft(Uuid::new_v4(), "title", "line")
        .unwrap();
    let checked = workspace
        .checkpoint_draft(Uuid::new_v4(), draft.id, draft.stamp, 1, "line\n")
        .unwrap();
    drop(workspace);

    let out = brn(&["drafts", "list", "--data-dir", root.to_str().unwrap()]);
    assert_eq!(code(&out), 0);
    assert!(!text(&out).is_empty());
    assert!(out.stderr.is_empty());

    let diff_args = [
        "revisions".to_string(),
        "diff".to_string(),
        "--draft".to_string(),
        draft.id.to_string(),
        "--from".to_string(),
        draft.stamp.base_revision.to_string(),
        "--to".to_string(),
        checked.stamp.base_revision.to_string(),
        "--data-dir".to_string(),
        root.to_str().unwrap().to_string(),
    ];
    let refs: Vec<&str> = diff_args.iter().map(String::as_str).collect();
    let out = brn(&refs);
    assert_eq!(code(&out), 0);
    let stdout = text(&out);
    assert!(stdout.contains("-line"), "{stdout}");
    assert!(stdout.contains("+line"), "{stdout}");
    assert!(out.stderr.is_empty());
}
