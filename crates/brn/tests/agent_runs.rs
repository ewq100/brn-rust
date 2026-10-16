//! Process checks of `ask` outcomes offline: a test-only loopback stub stands
//! in for the Copilot endpoint (support/stub.rs). Completed, Failed, model
//! candidates applied, dismissed and gone Stale, interrupted runs (SIGINT,
//! SIGTERM, deadline) and a killed run left Working.
#![cfg(unix)]
mod support;
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use support::stub::{answer, proxy_env, tool, Reply, Stub};
use support::*;

/// A root whose children see only the stub (proxies on a closed port).
fn root() -> Root {
    let mut root = Root::new();
    root.env = proxy_env();
    root
}
/// Configures Copilot with the stub's synthetic credentials and starts a chat.
fn configured(root: &Root, data: &Path, stub: &Stub) -> String {
    let creds = stub.credentials(root);
    let creds = creds.to_str().unwrap();
    let settings = [
        "configure",
        "--provider",
        "Copilot",
        "--model",
        "synthetic",
        "--effort",
        "low",
        "--credentials-dir",
        creds,
    ];
    ok(root, data, &settings);
    id(&ok(root, data, &["thread-start", "--title", "Chat"]))
}
fn ask(root: &Root, data: &Path, chat: &str) -> Value {
    call(root, data, &["ask", "--thread", chat, "--prompt", "Hello"])
}
fn read_note(note: &str) -> Reply {
    tool(
        "r",
        "read_note",
        json!({"id": note, "version": 1, "start": 0, "max_bytes": 4096, "section": false}),
    )
}
fn prepare(key: &str, note: &str, title: &str, markdown: &str) -> Reply {
    tool(
        key,
        "prepare_changes",
        json!({"request_key": key, "request": {
            "reason": "Synthetic model proposal",
            "writes": [{"id": note, "expected_version": 1, "archived": false,
                "data": {"Note": {"title": title, "markdown": markdown, "protected": false,
                    "confirmed": false, "superseded_by": null, "import": null}}}],
            "inputs": [{"record": note, "version": 1}]}}),
    )
}
fn runs(root: &Root, data: &Path, filter: &[&str]) -> Vec<Value> {
    let argv = [&["runs"][..], filter].concat();
    ok(root, data, &argv)["runs"].as_array().unwrap().clone()
}

#[test]
fn stub_ask_completes() {
    let root = root();
    let data = root.join("data");
    let stub = Stub::start(vec![answer("Synthetic answer.")]);
    let chat = configured(&root, &data, &stub);
    let envelope = ask(&root, &data, &chat);
    assert_eq!(envelope["status"], "ok", "{envelope}");
    assert_eq!(envelope["outcome"], "Completed");
    assert_eq!(envelope["result"]["answer"], "Synthetic answer.");
    assert_eq!(stub.requests().len(), 1);
    let listed = runs(&root, &data, &["--thread", &chat]);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["state"], "completed");
    assert_eq!(listed[0]["guards_writes"], false);
}

/// Owner decision B2: the CLI-created workspace reaches a provider only after `configure`.
#[test]
fn configure_then_ask_reaches_the_provider() {
    let root = root();
    let data = root.join("data");
    let stub = Stub::start(vec![answer("After configure.")]);
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let refused = ask(&root, &data, &chat);
    assert_error(&refused, "rejected", "credentials_not_configured");
    assert!(stub.requests().is_empty());
    configured(&root, &data, &stub);
    assert_eq!(ask(&root, &data, &chat)["outcome"], "Completed");
    assert_eq!(stub.requests().len(), 1);
}

#[test]
fn stub_ask_on_an_unknown_chat_never_contacts_the_stub() {
    let root = root();
    let data = root.join("data");
    let stub = Stub::start(vec![answer("never")]);
    configured(&root, &data, &stub);
    let refused = ask(&root, &data, "no-such-chat");
    assert_error(&refused, "rejected", "invalid_request");
    assert_eq!(refused["error"]["details"]["kind"], "tool_rejected");
    assert!(stub.requests().is_empty());
}

#[test]
fn stub_provider_error_ends_failed() {
    let root = root();
    let data = root.join("data");
    let stub = Stub::start(vec![Reply::Status(500)]);
    let chat = configured(&root, &data, &stub);
    let envelope = ask(&root, &data, &chat);
    assert_eq!(envelope["status"], "failed", "{envelope}");
    assert_eq!(envelope["exit"], 7);
    assert_eq!(envelope["outcome"], "Failed");
    assert_eq!(runs(&root, &data, &["--state", "failed"]).len(), 1);
}

#[test]
fn stub_model_candidate_review_apply_and_dismiss() {
    let root = root();
    let data = root.join("data");
    ok(&root, &data, &["delegation", "--choice", "review-first"]);
    let a = note(&root, &data, "Alpha", "alpha one");
    let b = note(&root, &data, "Beta", "beta one");
    let stub = Stub::start(vec![
        read_note(&a),
        read_note(&b),
        prepare("a", &a, "Alpha", "alpha two"),
        prepare("b", &b, "Beta", "beta two"),
        answer("Proposed two changes."),
    ]);
    let chat = configured(&root, &data, &stub);
    let envelope = ask(&root, &data, &chat);
    assert_eq!(envelope["outcome"], "Completed", "{envelope}");
    let page = call(&root, &data, &["candidates", "--limit", "1"]);
    assert_eq!(page["result"]["shown"], 1, "{page}");
    assert!(page["result"]["cursor"].is_string());
    let all = ok(&root, &data, &["candidates", "--limit", "10"])["candidates"].clone();
    let operation_for = |note: &str| {
        all.as_array()
            .unwrap()
            .iter()
            .find(|c| c["writes"][0]["id"] == note)
            .unwrap()["operation"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    let (first, second) = (operation_for(&a), operation_for(&b));
    let info = ok(&root, &data, &["operation", "--id", &first]);
    assert_eq!(info["state"], "prepared");
    // The note the model read, and the chat context the run added.
    let inputs = info["inputs"].as_array().unwrap();
    assert!(inputs.contains(&json!({"record": a, "version": 1, "expectation": "Current"})));
    assert!(inputs
        .iter()
        .any(|i| i["record"] == chat.as_str() && i["expectation"] == "ChatContext"));
    assert_eq!(info["inputs_total"], inputs.len());
    let applied = call(&root, &data, &["apply", "--operation", &first]);
    assert_eq!(applied["outcome"], "Applied", "{applied}");
    assert_eq!(applied["operation"], first.as_str());
    let again = call(&root, &data, &["apply", "--operation", &first]);
    assert_eq!(again["operation"], first.as_str());
    assert_eq!(again["retry"], "replay");
    let read = ok(&root, &data, &["read", "--id", &a]);
    assert_eq!(read["data"]["Note"]["markdown"], "alpha two");
    let dismissed = call(&root, &data, &["dismiss", "--operation", &second]);
    assert_eq!(dismissed["outcome"], "Dismissed", "{dismissed}");
    assert_eq!(ok(&root, &data, &["read", "--id", &b])["version"], 1);
    assert_eq!(
        ok(&root, &data, &["candidates", "--limit", "10"])["candidates"],
        json!([])
    );
}

#[test]
fn stub_candidate_goes_stale_after_owner_edit() {
    let root = root();
    let data = root.join("data");
    ok(&root, &data, &["delegation", "--choice", "review-first"]);
    let a = note(&root, &data, "Alpha", "alpha one");
    let stub = Stub::start(vec![
        read_note(&a),
        prepare("a", &a, "Alpha", "alpha from the model"),
        answer("Proposed."),
    ]);
    let chat = configured(&root, &data, &stub);
    assert_eq!(ask(&root, &data, &chat)["outcome"], "Completed");
    let candidate = ok(&root, &data, &["candidates", "--limit", "1"])["candidates"][0]["operation"]
        .as_str()
        .unwrap()
        .to_owned();
    let session = id(&ok(
        &root,
        &data,
        &["begin-edit", "--note", &a, "--base", "1"],
    ));
    let buffer = [
        "update-buffer",
        "--session",
        &session,
        "--generation",
        "1",
        "--text",
        "owner edit",
    ];
    ok(&root, &data, &buffer);
    ok(
        &root,
        &data,
        &["save", "--session", &session, "--generation", "1"],
    );
    let stale = call(&root, &data, &["apply", "--operation", &candidate]);
    assert_eq!(stale["status"], "stale", "{stale}");
    assert_eq!(stale["exit"], 3);
    assert_eq!(stale["conflict"][0]["id"], a.as_str());
    assert_eq!(stale["conflict"][0]["version"], 2);
}

#[test]
fn runs_page_filters_by_thread_and_state() {
    let root = root();
    let data = root.join("data");
    let stub = Stub::start(vec![answer("one"), answer("two"), Reply::Status(500)]);
    let first = configured(&root, &data, &stub);
    let second = id(&ok(&root, &data, &["thread-start", "--title", "Second"]));
    assert_eq!(ask(&root, &data, &first)["outcome"], "Completed");
    assert_eq!(ask(&root, &data, &second)["outcome"], "Completed");
    assert_eq!(ask(&root, &data, &second)["outcome"], "Failed");
    assert_eq!(runs(&root, &data, &["--thread", &first]).len(), 1);
    assert_eq!(runs(&root, &data, &["--thread", &second]).len(), 2);
    assert_eq!(runs(&root, &data, &["--state", "completed"]).len(), 2);
    let failed = runs(&root, &data, &["--thread", &second, "--state", "failed"]);
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0]["thread"], second.as_str());
    let page = ok(&root, &data, &["runs", "--limit", "2"]);
    assert_eq!(page["shown"], 2);
    let cursor = page["cursor"].as_str().unwrap().to_owned();
    let rest = ok(&root, &data, &["runs", "--limit", "2", "--cursor", &cursor]);
    assert_eq!(rest["shown"], 1);
    assert_eq!(rest["cursor"], Value::Null);
}

/// Starts `ask` against a held stream and returns once the stub has the request.
fn held_ask(root: &Root, data: &Path, extra: &[&str]) -> (std::process::Child, Stub, String) {
    let stub = Stub::start(vec![Reply::Hold]);
    let chat = configured(root, data, &stub);
    let mut argv = vec!["--json", "--data-dir", data.to_str().unwrap()];
    argv.extend_from_slice(extra);
    argv.extend(["ask", "--thread", &chat, "--prompt", "Hello"]);
    let child = spawn(root, &argv);
    stub.wait_for(1);
    (child, stub, chat)
}

/// SIGINT stops a held run. (This sandbox does not deliver SIGINT to children
/// of non-shell parents; CI runs it.)
#[test]
fn stub_sigint_interrupts_a_held_stream() {
    let root = root();
    let data = root.join("data");
    let (child, _stub, _) = held_ask(&root, &data, &[]);
    signal(&child, libc::SIGINT);
    let run = wait_output(child, Duration::from_secs(30));
    assert_eq!(run.code, 130);
    let envelope = run.envelope();
    assert_eq!(envelope["outcome"], "Interrupted");
    assert_eq!(envelope["stop"], "sigint");
    assert_eq!(runs(&root, &data, &["--state", "interrupted"]).len(), 1);
    // Default mode: exit 0 with "terminal":"Interrupted" (the live-journeys contract).
    let other = root.join("default");
    let stub = Stub::start(vec![Reply::Hold]);
    let chat = configured(&root, &other, &stub);
    let argv = argv(
        false,
        &other,
        &["ask", "--thread", &chat, "--prompt", "Hello"],
    );
    let child = spawn(&root, &argv);
    stub.wait_for(1);
    signal(&child, libc::SIGINT);
    let run = wait_output(child, Duration::from_secs(30));
    assert_eq!(run.code, 0);
    assert!(
        run.stdout.contains(r#""terminal":"Interrupted""#),
        "{}",
        run.stdout
    );
}

#[test]
fn stub_sigterm_interrupts_a_held_stream() {
    let root = root();
    let data = root.join("data");
    let (child, _stub, chat) = held_ask(&root, &data, &[]);
    signal(&child, libc::SIGTERM);
    let run = wait_output(child, Duration::from_secs(30));
    assert_eq!(run.code, 143, "{} {}", run.stdout, run.stderr);
    let envelope = run.envelope();
    assert_eq!(envelope["status"], "terminated");
    assert_eq!(envelope["outcome"], "Interrupted");
    assert_eq!(envelope["stop"], "sigterm");
    let stopped = runs(&root, &data, &["--thread", &chat, "--state", "interrupted"]);
    assert_eq!(stopped.len(), 1);
    assert_eq!(stopped[0]["guards_writes"], false);
}

#[test]
fn stub_deadline_interrupts_a_held_stream() {
    let root = root();
    let data = root.join("data");
    let (child, _stub, _) = held_ask(&root, &data, &["--timeout", "2"]);
    let run = wait_output(child, Duration::from_secs(30));
    assert_eq!(run.code, 7, "{} {}", run.stdout, run.stderr);
    let envelope = run.envelope();
    assert_eq!(envelope["status"], "failed");
    assert_eq!(envelope["outcome"], "Interrupted");
    assert_eq!(envelope["stop"], "deadline");
    assert!(envelope["result"]["run"].is_string());
}

#[test]
fn stub_killed_ask_leaves_a_working_run() {
    let root = root();
    let data = root.join("data");
    let topics = id(&ok(
        &root,
        &data,
        &["note-create", "--title", "Topics", "--text", "t"],
    ));
    let (child, _stub, chat) = held_ask(&root, &data, &[]);
    signal(&child, libc::SIGKILL);
    let killed = wait_output(child, Duration::from_secs(30));
    assert_eq!(killed.signal, Some(libc::SIGKILL));
    let working = runs(&root, &data, &["--state", "working"]);
    assert_eq!(working.len(), 1, "{working:?}");
    assert_eq!(working[0]["thread"], chat.as_str());
    assert_eq!(working[0]["guards_writes"], true);
    let deferred = call(&root, &data, &["topics-choose", "--note", &topics]);
    assert_eq!(deferred["status"], "deferred", "{deferred}");
    assert_eq!(deferred["conflict"][0]["kind"], "Run");
    // On the owner's instruction, pause stops it; never assume Cancelled.
    ok(&root, &data, &["pause"]);
    let all = runs(&root, &data, &["--thread", &chat]);
    assert_ne!(all[0]["state"], "working");
    assert_eq!(all[0]["guards_writes"], false);
    assert!(runs(&root, &data, &["--state", "working"]).is_empty());
}
