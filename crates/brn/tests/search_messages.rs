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
    json_command(&data, &["init"]);
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
}
