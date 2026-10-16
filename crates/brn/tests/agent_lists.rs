//! Process checks of the round-2b list pages: `notes`, `threads`,
//! `needs-you`, `projects` and `recovery` page to exactly their unpaged
//! output, and `messages --thread` pages one chat in creation order.
#![cfg(unix)]
mod support;
use serde_json::{json, Value};
use std::path::Path;
use support::stub::{answer, proxy_env, Stub};
use support::*;

/// Every page of `command` (which carries `--limit`), following `cursor`.
/// Each page envelope's `page` mirrors the result.
fn all_pages(root: &Root, data: &Path, command: &[&str], list: &str) -> Vec<Value> {
    let mut items = vec![];
    let mut cursor: Option<String> = None;
    loop {
        let mut argv: Vec<&str> = command.to_vec();
        if let Some(cursor) = &cursor {
            argv.extend(["--cursor", cursor.as_str()]);
        }
        let envelope = call(root, data, &argv);
        assert_eq!(envelope["status"], "ok", "{argv:?}: {envelope}");
        let result = &envelope["result"];
        assert_eq!(envelope["page"]["cursor"], result["cursor"], "{envelope}");
        assert_eq!(envelope["page"]["shown"], result["shown"], "{envelope}");
        assert_eq!(envelope["page"]["total"], Value::Null);
        assert_eq!(result["total"], Value::Null);
        let page = result[list].as_array().unwrap();
        assert_eq!(result["shown"], page.len());
        items.extend(page.iter().cloned());
        match result["cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => return items,
        }
    }
}
fn project(root: &Root, data: &Path, title: &str) -> String {
    let receipt = ok(root, data, &["project-create", "--title", title]);
    receipt["writes"][0]["after"]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Notes (one chat-only, one trashed with its project), chats (one archived),
/// open sessions and projects (one trashed) in one workspace.
fn workspace(root: &Root, data: &Path) {
    let mut notes = vec![];
    for i in 0..4 {
        notes.push(note(root, data, &format!("N{i}"), "text"));
    }
    let mut chats = vec![];
    for i in 0..4 {
        chats.push(id(&ok(
            root,
            data,
            &["thread-start", "--title", &format!("C{i}")],
        )));
    }
    // A chat-only attachment is not a general note.
    let source = root.file("attachment.md", b"# Attachment\n\nChat only.\n");
    let attached = call(
        root,
        data,
        &[
            "intake", "--source", &source, "--intent", "chat", "--thread", &chats[0],
        ],
    );
    assert_eq!(attached["outcome"], "Applied", "{attached}");
    ok(root, data, &["archive", "--thread", &chats[3]]);
    for note in &notes[..3] {
        ok(root, data, &["begin-edit", "--note", note, "--base", "1"]);
    }
    for title in ["P0", "P1"] {
        project(root, data, title);
    }
    // A trashed project with a document: the document leaves the notes list.
    let trashed = project(root, data, "Trashed");
    ok(
        root,
        data,
        &[
            "project-document",
            "--project",
            &trashed,
            "--title",
            "D",
            "--text",
            "doc",
        ],
    );
    let preview = ok(
        root,
        data,
        &["project-delete-preview", "--project", &trashed],
    );
    let preview_file = root.file("preview.json", preview.to_string().as_bytes());
    let deleted = call(
        root,
        data,
        &[
            "project-delete",
            "--preview-file",
            &preview_file,
            "--actions",
            "{}",
        ],
    );
    assert_eq!(deleted["status"], "ok", "{deleted}");
}

#[test]
fn remaining_lists_page_to_their_unpaged_output() {
    let root = Root::new();
    let data = root.workspace("data");
    workspace(&root, &data);
    for (command, list, extra, expected) in [
        ("notes", "notes", &[][..], Some(4)),
        ("threads", "threads", &[][..], Some(3)),
        // Attention is not reachable offline from the CLI; the core test
        // `note_thread_needs_you_and_project_pages_match_the_app_filters` covers it.
        ("needs-you", "threads", &[][..], None),
        ("recovery", "sessions", &[][..], Some(3)),
        ("projects", "projects", &[][..], Some(2)),
        ("projects", "projects", &["--trash", "yes"][..], Some(3)),
        ("projects", "projects", &["--trash", "no"][..], Some(2)),
    ] {
        let unpaged = ok(&root, &data, &[&[command][..], extra].concat());
        let paged = all_pages(
            &root,
            &data,
            &[&[command, "--limit", "2"][..], extra].concat(),
            list,
        );
        assert_eq!(json!(paged), unpaged, "{command} {extra:?}");
        if let Some(expected) = expected {
            assert_eq!(paged.len(), expected, "{command} {extra:?}");
        }
    }
    // The chat-only attachment and the trashed document are not notes.
    let notes = ok(&root, &data, &["notes"]);
    assert!(notes
        .as_array()
        .unwrap()
        .iter()
        .all(|n| n["data"]["Note"]["chat"].is_null() && n["archived"] == false));
}

#[test]
fn lists_without_page_options_are_unchanged() {
    let root = Root::new();
    let data = root.workspace("data");
    workspace(&root, &data);
    for command in ["notes", "threads", "needs-you", "recovery", "projects"] {
        let envelope = call(&root, &data, &[command]);
        assert_eq!(envelope["page"], Value::Null, "{command}");
        let default = run(&root, false, &data, &[command]);
        assert_eq!(default.code, 0);
        assert!(
            default.stdout.starts_with('['),
            "{command}: {}",
            default.stdout
        );
        assert_eq!(
            default.stdout,
            format!("{}\n", envelope["result"]),
            "{command}"
        );
    }
    let trash = run(&root, false, &data, &["projects", "--trash", "yes"]);
    assert_eq!(
        trash.stdout,
        format!("{}\n", ok(&root, &data, &["projects", "--trash", "yes"]))
    );
    // A page option selects the page object in default mode too.
    let page = plain(&root, &data, &["notes", "--limit", "1"]);
    assert_eq!(page["shown"], 1);
    assert!(page["cursor"].is_string());
    let help = run_argv(&root, &["--json", "--help"], None);
    let description: Value = serde_json::from_str(&help.stdout).unwrap();
    let entry = |name: &str| {
        description["commands"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()
            .clone()
    };
    for (name, form) in [
        ("notes", "notes --limit N [--cursor ID]"),
        ("threads", "threads --limit N [--cursor ID]"),
        ("needs-you", "needs-you --limit N [--cursor ID]"),
        ("recovery", "recovery --limit N [--cursor ID]"),
        ("projects", "projects [--trash yes] --limit N [--cursor ID]"),
    ] {
        let entry = entry(name);
        assert_eq!(entry["bounded"], false, "{name}");
        assert_eq!(entry["bounded_form"], form, "{name}");
        assert_eq!(
            entry["page"],
            json!({"limit": {"min": 1, "max": 100, "default": 20}, "cursor": "id"}),
            "{name}"
        );
    }
    let messages = entry("messages");
    assert_eq!(messages["bounded"], true);
    assert_eq!(messages["required"], json!(["thread"]));
    assert_eq!(messages["retry"]["default"], "read");
    assert_eq!(messages["page"]["cursor"], "rowid");
}

#[test]
fn list_page_options_are_refused_before_opening() {
    let root = Root::new();
    let data = root.join("absent");
    for command in ["notes", "threads", "needs-you", "recovery", "projects"] {
        for (option, value) in [("limit", "0"), ("limit", "101"), ("cursor", "bad id!")] {
            let flag = format!("--{option}");
            let argv = [command, &flag, value];
            let envelope = call(&root, &data, &argv);
            assert_error(&envelope, "usage", "invalid_value");
            assert_eq!(envelope["error"]["details"]["option"], option, "{argv:?}");
            let default = run(&root, false, &data, &argv);
            assert_eq!(default.code, 1, "{argv:?}");
        }
    }
    for (option, value) in [("limit", "0"), ("limit", "101"), ("cursor", "x1")] {
        let flag = format!("--{option}");
        let argv = ["messages", "--thread", "t", &flag, value];
        let envelope = call(&root, &data, &argv);
        assert_error(&envelope, "usage", "invalid_value");
        assert_eq!(envelope["error"]["details"]["option"], option);
    }
    let missing = call(&root, &data, &["messages"]);
    assert_error(&missing, "usage", "missing_option");
    assert_eq!(missing["error"]["details"]["option"], "thread");
    assert!(!data.exists());
}

fn roles(messages: &[Value]) -> Vec<String> {
    messages
        .iter()
        .map(|m| m["data"]["Message"]["role"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn messages_page_a_chat_in_creation_order() {
    let mut root = Root::new();
    root.env = proxy_env();
    let data = root.workspace("data");
    let stub = Stub::start(vec![answer("First answer."), answer("Second answer.")]);
    let creds = stub.credentials(&root);
    ok(
        &root,
        &data,
        &[
            "configure",
            "--provider",
            "Copilot",
            "--model",
            "synthetic",
            "--effort",
            "low",
            "--credentials-dir",
            creds.to_str().unwrap(),
        ],
    );
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let other = id(&ok(&root, &data, &["thread-start", "--title", "Other"]));
    for prompt in ["One", "Two"] {
        let asked = call(
            &root,
            &data,
            &["ask", "--thread", &chat, "--prompt", prompt],
        );
        assert_eq!(asked["outcome"], "Completed", "{asked}");
    }
    let messages = all_pages(
        &root,
        &data,
        &["messages", "--thread", &chat, "--limit", "3"],
        "messages",
    );
    assert_eq!(
        roles(&messages),
        ["owner", "assistant", "owner", "assistant"]
    );
    let ids: Vec<&str> = messages.iter().map(|m| m["id"].as_str().unwrap()).collect();
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "a page repeated a message");
    assert_eq!(messages[0]["data"]["Message"]["text"], "One");
    assert!(messages
        .iter()
        .all(|m| m["data"]["Message"]["thread"] == chat.as_str()));
    let page = ok(&root, &data, &["messages", "--thread", &chat]);
    assert_eq!(page["thread"], chat.as_str());
    assert_eq!(page["chat_archived"], false);
    assert_eq!(page["shown"], 4);
    assert_eq!(page["cursor"], Value::Null);
    let empty = ok(&root, &data, &["messages", "--thread", &other]);
    assert_eq!(empty["messages"], json!([]));
    // An archived chat is still listed: the owner's view.
    ok(&root, &data, &["archive", "--thread", &chat]);
    let archived = ok(&root, &data, &["messages", "--thread", &chat]);
    assert_eq!(archived["chat_archived"], true);
    assert_eq!(json!(messages), archived["messages"]);
    let unknown = call(&root, &data, &["messages", "--thread", "no-such-chat"]);
    assert_error(&unknown, "rejected", "not_found");
    let a_note = note(&root, &data, "N", "text");
    let wrong = call(&root, &data, &["messages", "--thread", &a_note]);
    assert_error(&wrong, "rejected", "invalid_request");
    assert_eq!(
        wrong["error"]["details"],
        json!({"option": "thread", "reason": "wrong_kind"})
    );
    let missing = call(
        &root,
        &root.join("absent"),
        &["messages", "--thread", &chat],
    );
    assert_error(&missing, "rejected", "missing_workspace");
    assert!(!root.join("absent").exists());
}

#[test]
fn seeded_review_chat_lists_its_message() {
    let root = Root::new();
    let data = root.workspace("data");
    note(&root, &data, "N", "text");
    let seeded = ok(&root, &data, &["knowledge-check", "--seed-chat", "yes"]);
    let chat = seeded["review_chat"].as_str().unwrap();
    let page = ok(
        &root,
        &data,
        &["messages", "--thread", chat, "--limit", "1"],
    );
    assert_eq!(page["shown"], 1);
    assert_eq!(page["cursor"], Value::Null);
    assert_eq!(roles(page["messages"].as_array().unwrap()), ["owner"]);
}
