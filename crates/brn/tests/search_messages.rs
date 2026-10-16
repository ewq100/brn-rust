//! The CLI `search` lists saved chat Messages with their chat, as their own
//! kind beside notes. Synthetic data; a missing auth file makes every
//! provider step fail offline.
use serde_json::{json, Value};
use std::{
    path::Path,
    process::{Command, Output},
};
fn command(data: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_brn"))
        .arg("--data-dir")
        .arg(data)
        .args(args)
        .output()
        .unwrap()
}
fn json_command(data: &Path, args: &[&str]) -> Value {
    let output = command(data, args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn search_lists_a_saved_chat_message_with_its_chat() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("replies");
    json_command(&data, &["init", "--purpose", "test"]);
    let missing = root.path().join("missing-auth.json");
    json_command(
        &data,
        &[
            "configure",
            "--provider",
            "Codex",
            "--model",
            "gpt-6.1-sol",
            "--effort",
            "medium",
            "--auth-file",
            missing.to_str().unwrap(),
        ],
    );
    let chat = json_command(&data, &["thread-start", "--title", "Replies"])["id"]
        .as_str()
        .unwrap()
        .to_owned();
    // Offline, the run fails, but the owner Message is saved.
    let asked = json_command(
        &data,
        &[
            "ask",
            "--thread",
            &chat,
            "--prompt",
            "Berth survey reply draft 1.4 m",
        ],
    );
    assert!(asked["terminal"]
        .as_str()
        .unwrap()
        .starts_with("Failed:ReconnectNeeded"));
    let page = json_command(&data, &["search", "--query", "berth"]);
    // Before: no `messages`; the Message was found only by browsing chats.
    assert_eq!(page["message_total"], json!(1), "{page}");
    let hit = &page["messages"][0];
    assert_eq!(
        (&hit["chat"], &hit["chat_title"], &hit["role"]),
        (&json!(chat), &json!("Replies"), &json!("owner"))
    );
    assert_eq!(hit["snippet"], "Berth survey reply draft 1.4 m");
    assert_eq!(page["message_cursor"], Value::Null);
    // Notes are counted apart, as before.
    assert_eq!(
        (&page["total"], &page["shown"], &page["hits"]),
        (&json!(0), &json!(0), &json!([]))
    );
    // In another chat's context, a general chat still shows.
    let other = json_command(&data, &["thread-start", "--title", "Other"])["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let page = json_command(&data, &["search", "--query", "berth", "--thread", &other]);
    assert_eq!(page["messages"][0]["chat"], json!(chat));
    assert_eq!(page["messages"][0]["archived"], json!(false));
    // Owner decision 3: an archived chat's Message stays, labelled. Before:
    // `message_total` 0 until Unarchive.
    json_command(
        &data,
        &["note-create", "--title", "Berth", "--text", "Berth depth."],
    );
    json_command(&data, &["archive", "--thread", &chat]);
    let page = json_command(&data, &["search", "--query", "berth"]);
    assert_eq!(page["message_total"], json!(1), "{page}");
    assert_eq!(
        (
            &page["messages"][0]["chat"],
            &page["messages"][0]["archived"]
        ),
        (&json!(chat), &json!(true))
    );
    // A general note hit carries the field too, false.
    assert_eq!(page["hits"][0]["archived"], json!(false), "{page}");
    json_command(&data, &["unarchive", "--thread", &chat]);
    let page = json_command(&data, &["search", "--query", "berth"]);
    assert_eq!(page["messages"][0]["archived"], json!(false));
}

/// One machine-mode call: the envelope.
fn envelope(data: &Path, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_brn"))
        .arg("--json")
        .arg("--data-dir")
        .arg(data)
        .args(args)
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}

/// PR #156 LOW 3: the envelope `page` covers both lanes. When the notes end
/// first, `page.cursor` is `result.message_cursor`, and following it to null
/// reaches every Message once.
#[test]
fn search_page_cursor_follows_the_message_lane() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("lanes");
    json_command(&data, &["init", "--purpose", "test"]);
    let missing = root.path().join("missing-auth.json");
    json_command(
        &data,
        &[
            "configure",
            "--provider",
            "Codex",
            "--model",
            "gpt-6.1-sol",
            "--effort",
            "medium",
            "--auth-file",
            missing.to_str().unwrap(),
        ],
    );
    json_command(
        &data,
        &["note-create", "--title", "Berth", "--text", "Berth depth."],
    );
    let chat = json_command(&data, &["thread-start", "--title", "Replies"])["id"]
        .as_str()
        .unwrap()
        .to_owned();
    for n in 1..=3 {
        // Offline, each run fails, but the owner Message is saved.
        json_command(
            &data,
            &[
                "ask",
                "--thread",
                &chat,
                "--prompt",
                &format!("Berth reply {n}"),
            ],
        );
    }
    let mut cursor: Option<String> = None;
    let mut messages = vec![];
    let mut notes = 0;
    for page in 1.. {
        assert!(page <= 5, "the cursor never ended");
        let mut args = vec!["search", "--query", "berth", "--limit", "1"];
        if let Some(cursor) = &cursor {
            args.extend(["--cursor", cursor.as_str()]);
        }
        let envelope = envelope(&data, &args);
        assert_eq!(envelope["status"], "ok", "{envelope}");
        let result = &envelope["result"];
        let shown =
            result["hits"].as_array().unwrap().len() + result["messages"].as_array().unwrap().len();
        assert_eq!(envelope["page"]["shown"], shown, "{envelope}");
        assert_eq!(envelope["page"]["total"], 4, "{envelope}");
        // `result` is unchanged: its `shown` still counts notes only.
        assert_eq!(result["shown"], result["hits"].as_array().unwrap().len());
        if page == 1 {
            assert_eq!(result["cursor"], Value::Null, "{envelope}");
            assert!(result["message_cursor"].is_string(), "{envelope}");
            assert_eq!(envelope["page"]["cursor"], result["message_cursor"]);
        }
        notes += result["hits"].as_array().unwrap().len();
        for message in result["messages"].as_array().unwrap() {
            messages.push(message["message"].as_str().unwrap().to_owned());
        }
        match envelope["page"]["cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => break,
        }
    }
    assert_eq!(notes, 1);
    let mut unique = messages.clone();
    unique.sort();
    unique.dedup();
    assert_eq!((messages.len(), unique.len()), (3, 3), "{messages:?}");
}
