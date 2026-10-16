//! Process checks of `invalid_request` `details.reason`: the code stays
//! `invalid_request`; the reason names the kind of refusal.
mod support;
use serde_json::Value;
use std::path::Path;
use support::*;

fn created(envelope: &Value, kind: &str) -> String {
    envelope["changed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == kind && c["created"] == true)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn project(root: &Root, data: &Path) -> String {
    created(
        &call(root, data, &["project-create", "--title", "P"]),
        "Project",
    )
}
fn assert_reason(envelope: &Value, reason: &str) {
    assert_error(envelope, "rejected", "invalid_request");
    assert_eq!(envelope["error"]["details"]["reason"], reason, "{envelope}");
}

#[test]
fn stale_project_settings_reason_is_version_mismatch() {
    let root = Root::new();
    let data = root.workspace("data");
    let project = project(&root, &data);
    let argv = [
        "project-settings",
        "--project",
        project.as_str(),
        "--base",
        "99",
        "--title",
        "T",
        "--instructions",
        "I",
        "--knowledge",
        "yes",
    ];
    let first = call(&root, &data, &argv);
    assert_reason(&first, "version_mismatch");
    // An identical retry meets the same refusal.
    assert_reason(&call(&root, &data, &argv), "version_mismatch");
}

#[test]
fn trashed_project_chat_reason_is_inactive_target() {
    let root = Root::new();
    let data = root.workspace("data");
    let project = project(&root, &data);
    let chat = call(
        &root,
        &data,
        &["project-chat", "--project", &project, "--title", "C"],
    );
    let chat = created(&chat, "Thread");
    let preview = run(
        &root,
        false,
        &data,
        &["project-delete-preview", "--project", &project],
    );
    let file = root.file("preview.json", preview.stdout.trim_end().as_bytes());
    let deleted = call(
        &root,
        &data,
        &["project-delete", "--preview-file", &file, "--actions", "{}"],
    );
    assert_eq!(deleted["status"], "ok", "{deleted}");
    let queued = call(
        &root,
        &data,
        &["queue", "--thread", &chat, "--text", "later"],
    );
    assert_reason(&queued, "inactive_target");
}

#[test]
fn incomplete_project_delete_actions_reason_is_project_boundary() {
    let root = Root::new();
    let data = root.workspace("data");
    let project = project(&root, &data);
    let preview = run(
        &root,
        false,
        &data,
        &["project-delete-preview", "--project", &project],
    );
    let file = root.file("preview.json", preview.stdout.trim_end().as_bytes());
    let refused = call(
        &root,
        &data,
        &[
            "project-delete",
            "--preview-file",
            &file,
            "--actions",
            r#"{"no-such-action":"Keep"}"#,
        ],
    );
    assert_reason(&refused, "project_boundary");
}

#[test]
fn oversized_project_title_reason_is_size_limit() {
    let root = Root::new();
    let data = root.workspace("data");
    let title = root.file("title.txt", &vec![b't'; 1025]);
    let refused = call(&root, &data, &["project-create", "--title-file", &title]);
    assert_reason(&refused, "size_limit");
}

#[test]
fn search_thread_naming_a_note_reason_is_wrong_kind() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "text");
    let refused = call(
        &root,
        &data,
        &["search", "--query", "text", "--thread", &note],
    );
    assert_reason(&refused, "wrong_kind");
    assert_eq!(refused["error"]["details"]["option"], "thread");
    let paged = call(
        &root,
        &data,
        &[
            "read",
            "--id",
            &id(&ok(&root, &data, &["thread-start", "--title", "C"])),
            "--limit",
            "10",
        ],
    );
    assert_reason(&paged, "wrong_kind");
}
