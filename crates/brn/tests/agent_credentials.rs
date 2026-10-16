//! Process checks of owner decision B2: a workspace the CLI creates has no
//! credential source, and machine-mode provider work on it is refused
//! before any commit or provider contact.
mod support;
use serde_json::{json, Value};
use support::*;

fn history_total(root: &Root, data: &std::path::Path) -> Value {
    ok(root, data, &["history", "--limit", "1"])["total"].clone()
}

#[test]
fn cli_created_workspace_has_no_credential_source() {
    let root = Root::new();
    // A synthetic sign-in file where the desktop would derive one.
    root.file("home/auth.json", b"{}");
    std::fs::create_dir(root.join("home/.codex")).unwrap();
    root.file("home/.codex/auth.json", b"{}");
    let data = root.join("data");
    let settings = ok(&root, &data, &["settings"]);
    assert_eq!(settings["auth_file"], Value::Null, "{settings}");
    assert_eq!(settings["credentials_dir"], Value::Null);
    let init = plain(&root, &root.join("other"), &["init"]);
    assert_eq!(init["settings"]["auth_file"], Value::Null);
}

#[test]
fn ask_without_credentials_is_refused_before_any_commit() {
    let root = Root::new();
    let data = root.join("data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let before = history_total(&root, &data);
    let refused = call(
        &root,
        &data,
        &["ask", "--thread", &chat, "--prompt", "Hello"],
    );
    assert_error(&refused, "rejected", "credentials_not_configured");
    assert_eq!(refused["exit"], 6);
    assert_eq!(refused["error"]["details"], json!({"provider": "Codex"}));
    assert_eq!(history_total(&root, &data), before);
    assert_eq!(ok(&root, &data, &["runs"])["runs"], json!([]));
}

#[test]
fn provider_intake_without_credentials_is_refused_before_conversion() {
    let root = Root::new();
    let data = root.join("data");
    ok(&root, &data, &["init"]);
    let before = history_total(&root, &data);
    let html = root.file("doc.html", b"<html><body><h1>Doc</h1></body></html>");
    let markdown = root.file("doc.md", b"# Doc\n\nBody.\n");
    for argv in [
        vec!["intake", "--source", html.as_str(), "--intent", "full"],
        vec![
            "intake",
            "--source",
            markdown.as_str(),
            "--intent",
            "useful",
        ],
    ] {
        let run = run(&root, true, &data, &argv);
        assert_eq!(run.stderr, "", "{argv:?} printed progress");
        assert_error(&run.envelope(), "rejected", "credentials_not_configured");
    }
    assert_eq!(history_total(&root, &data), before);
    assert_eq!(ok(&root, &data, &["threads"]), json!([]));
    // Markdown needs no provider: a full import still works.
    let full = call(
        &root,
        &data,
        &["intake", "--source", &markdown, "--intent", "full"],
    );
    assert_eq!(full["outcome"], "Applied", "{full}");
}

#[test]
fn refused_ask_stays_refused_and_records_nothing() {
    let root = Root::new();
    let data = root.join("data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let argv = ["ask", "--thread", chat.as_str(), "--prompt", "Hello"];
    let first = call(&root, &data, &argv);
    let second = call(&root, &data, &argv);
    for envelope in [&first, &second] {
        assert_error(envelope, "rejected", "credentials_not_configured");
        assert_eq!(envelope["retry"], "refuse");
    }
    assert_eq!(ok(&root, &data, &["runs"])["runs"], json!([]));
}
