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
    // creation semantics: B2 covers every workspace the CLI creates.
    let data = root.join("data");
    ok(&root, &data, &["init"]);
    let settings = ok(&root, &data, &["settings"]);
    assert_eq!(settings["auth_file"], Value::Null, "{settings}");
    assert_eq!(settings["credentials_dir"], Value::Null);
    let init = plain(&root, &root.join("other"), &["init"]);
    assert_eq!(init["settings"]["auth_file"], Value::Null);
    // A test workspace (B1 phase 1) gets no credential source either.
    let test = root.workspace("test");
    let settings = ok(&root, &test, &["settings"]);
    assert_eq!(settings["auth_file"], Value::Null, "{settings}");
    assert_eq!(settings["credentials_dir"], Value::Null);
}

#[test]
fn ask_without_credentials_is_refused_before_any_commit() {
    let root = Root::new();
    // creation semantics: B2 covers every workspace the CLI creates.
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
    #[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
    let root = {
        let mut root = root;
        for name in ["BRN_XBERG_HELPER", "BRN_XBERG_HELPER_BRN"] {
            root.env.push((
                name.into(),
                std::env::var(name)
                    .expect("declared helper artifact required for native qualification"),
            ));
        }
        root
    };
    // creation semantics: B2 covers every workspace the CLI creates.
    let data = root.join("data");
    ok(&root, &data, &["init"]);
    ok(
        &root,
        &data,
        &["document-conversion", "--converter", "xberg", "--ai", "on"],
    );
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
    ok(
        &root,
        &data,
        &["document-conversion", "--converter", "xberg", "--ai", "off"],
    );
    // AI Off needs no conversion provider; native helpers are admitted on
    // macOS and Linux x86_64.
    #[cfg(not(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))))]
    let before_off = (
        ok(&root, &data, &["records"]),
        ok(&root, &data, &["history"]),
    );
    let full = call(
        &root,
        &data,
        &["intake", "--source", &markdown, "--intent", "full"],
    );
    #[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
    assert_eq!(full["outcome"], "Applied", "{full}");
    #[cfg(not(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))))]
    {
        assert_error(&full, "failed", "unavailable");
        assert_eq!(ok(&root, &data, &["records"]), before_off.0);
        assert_eq!(ok(&root, &data, &["history"]), before_off.1);
        assert_eq!(ok(&root, &data, &["runs"])["runs"], json!([]));
    }
}

#[cfg(not(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64"))))]
#[test]
fn off_conversion_is_unavailable_without_effects_for_both_converters() {
    let root = Root::new();
    let data = root.workspace("data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Existing"]));
    let source = root.file("doc.md", b"# Doc\n\nBody.\n");
    for converter in ["xberg", "xberg-brn"] {
        ok(
            &root,
            &data,
            &[
                "document-conversion",
                "--converter",
                converter,
                "--ai",
                "off",
            ],
        );
        let records = ok(&root, &data, &["records"]);
        let history = ok(&root, &data, &["history"]);
        let threads = ok(&root, &data, &["threads"]);
        for intent in ["full", "chat"] {
            let mut argv = vec!["intake", "--source", source.as_str(), "--intent", intent];
            if intent == "chat" {
                argv.extend(["--thread", chat.as_str()]);
            }
            let refused = call(&root, &data, &argv);
            assert_error(&refused, "failed", "unavailable");
            assert_eq!(ok(&root, &data, &["records"]), records);
            assert_eq!(ok(&root, &data, &["history"]), history);
            assert_eq!(ok(&root, &data, &["threads"]), threads);
            assert_eq!(ok(&root, &data, &["runs"])["runs"], json!([]));
        }
    }
}

#[test]
fn refused_ask_stays_refused_and_records_nothing() {
    let root = Root::new();
    // creation semantics: B2 covers every workspace the CLI creates.
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
