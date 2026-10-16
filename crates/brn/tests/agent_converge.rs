//! Process checks of owner decision B4: a no-op archive, resolve, unarchive,
//! chat-move, project-link or project-unlink returns `Unchanged` and records
//! nothing; a keyed retry of an earlier change still replays its receipt.
mod support;
use serde_json::{json, Value};
use std::path::Path;
use support::*;

fn total(root: &Root, data: &Path) -> Value {
    ok(root, data, &["history", "--limit", "1"])["total"].clone()
}
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
fn version(root: &Root, data: &Path, id: &str) -> Value {
    ok(root, data, &["read", "--id", id])["version"].clone()
}
/// A no-op converging call: ok, exit 0, `Unchanged`, no operation, nothing written.
fn assert_unchanged(root: &Root, data: &Path, argv: &[&str], record: &str) {
    let before = (total(root, data), version(root, data, record));
    let envelope = call(root, data, argv);
    assert_eq!(envelope["status"], "ok", "{argv:?}: {envelope}");
    assert_eq!(envelope["exit"], 0);
    assert_eq!(envelope["outcome"], "Unchanged", "{argv:?}: {envelope}");
    assert_eq!(envelope["operation"], Value::Null);
    assert_eq!(envelope["changed"], json!([]));
    assert_eq!(
        envelope["result"],
        json!({"outcome": "Unchanged", "record": record, "version": before.1})
    );
    assert_eq!((total(root, data), version(root, data, record)), before);
}

#[test]
fn repeated_archive_and_resolve_are_unchanged_without_a_receipt() {
    let root = Root::new();
    let data = root.join("data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let first = call(&root, &data, &["archive", "--thread", &chat]);
    assert_eq!(first["outcome"], "Applied", "{first}");
    assert_unchanged(&root, &data, &["archive", "--thread", &chat], &chat);
    assert_unchanged(&root, &data, &["resolve", "--thread", &chat], &chat);
    assert_unchanged(
        &root,
        &data,
        &["archive", "--thread", &chat, "--key", "a1"],
        &chat,
    );
}

#[test]
fn repeated_unarchive_move_link_unlink_are_unchanged() {
    let root = Root::new();
    let data = root.join("data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    assert_unchanged(&root, &data, &["unarchive", "--thread", &chat], &chat);
    let project = call(&root, &data, &["project-create", "--title", "P"]);
    let project = created(&project, "Project");
    let note = note(&root, &data, "Doc", "text");
    let link = [
        "project-link",
        "--project",
        project.as_str(),
        "--note",
        note.as_str(),
    ];
    assert_eq!(call(&root, &data, &link)["outcome"], "Applied");
    assert_unchanged(&root, &data, &link, &project);
    let unlink = [
        "project-unlink",
        "--project",
        project.as_str(),
        "--note",
        note.as_str(),
    ];
    assert_eq!(call(&root, &data, &unlink)["outcome"], "Applied");
    assert_unchanged(&root, &data, &unlink, &project);
    let into = [
        "chat-move",
        "--thread",
        chat.as_str(),
        "--project",
        project.as_str(),
    ];
    assert_eq!(call(&root, &data, &into)["outcome"], "Applied");
    assert_unchanged(&root, &data, &into, &chat);
    let out = ["chat-move", "--thread", chat.as_str(), "--project", "none"];
    assert_eq!(call(&root, &data, &out)["outcome"], "Applied");
    assert_unchanged(&root, &data, &out, &chat);
}

/// A no-op-looking archive still records a change when it stops a Working run.
#[cfg(unix)]
#[test]
fn archive_with_a_working_run_records_a_change() {
    use support::stub::{proxy_env, Reply, Stub};
    let mut root = Root::new();
    root.env = proxy_env();
    let data = root.join("data");
    let stub = Stub::start(vec![Reply::Hold]);
    let creds = stub.credentials(&root);
    let configure = [
        "configure",
        "--provider",
        "Copilot",
        "--model",
        "synthetic",
        "--effort",
        "low",
        "--credentials-dir",
        creds.to_str().unwrap(),
    ];
    ok(&root, &data, &configure);
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let argv = argv(
        true,
        &data,
        &["ask", "--thread", &chat, "--prompt", "Hello"],
    );
    let child = spawn(&root, &argv);
    stub.wait_for(1);
    signal(&child, libc::SIGKILL);
    wait_output(child, std::time::Duration::from_secs(30));
    let archived = call(&root, &data, &["archive", "--thread", &chat]);
    assert_eq!(archived["outcome"], "Applied", "{archived}");
    assert!(archived["changed"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["kind"] == "Run"));
    let runs = ok(&root, &data, &["runs", "--thread", &chat]);
    // An outside stop reads back as Interrupted, never Cancelled.
    assert_eq!(runs["runs"][0]["state"], "interrupted");
    assert_unchanged(&root, &data, &["archive", "--thread", &chat], &chat);
}

#[test]
fn keyed_archive_retry_after_a_change_replays_the_original_receipt() {
    let root = Root::new();
    let data = root.join("data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let argv = ["archive", "--thread", chat.as_str(), "--key", "a1"];
    let first = call(&root, &data, &argv);
    assert_eq!(first["outcome"], "Applied");
    let before = total(&root, &data);
    let retry = call(&root, &data, &argv);
    assert_eq!(retry["outcome"], "Applied", "{retry}");
    assert_eq!(retry["operation"], first["operation"]);
    assert_eq!(retry["result"], first["result"]);
    assert_eq!(retry["retry"], "replay");
    assert_eq!(total(&root, &data), before);
}

#[test]
fn default_mode_unchanged_prints_the_unchanged_object() {
    let root = Root::new();
    let data = root.join("data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let version = version(&root, &data, &chat);
    let printed = run(&root, false, &data, &["unarchive", "--thread", &chat]);
    assert_eq!(printed.code, 0);
    assert_eq!(
        printed.stdout,
        format!(
            "{}\n",
            json!({"outcome": "Unchanged", "record": chat, "version": version})
        )
    );
}
