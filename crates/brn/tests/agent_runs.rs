//! Process checks of `ask` outcomes offline: a test-only loopback stub stands
//! in for the Copilot endpoint (support/stub.rs). Completed, Failed, model
//! candidates applied, dismissed and gone Stale, interrupted runs (SIGINT,
//! SIGTERM, deadline) and a killed run left Working.
#![cfg(unix)]
mod support;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};
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
    let data = root.workspace("data");
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
    let data = root.workspace("data");
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
    let data = root.workspace("data");
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
    let data = root.workspace("data");
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
    let data = root.workspace("data");
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
    let data = root.workspace("data");
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
    let (session, token) = begin_edit(&root, &data, &a, 1);
    let session = id(&session);
    let buffer = [
        "update-buffer",
        "--session",
        &session,
        "--generation",
        "1",
        "--text",
        "owner edit",
        "--token-file",
        &token,
    ];
    ok(&root, &data, &buffer);
    ok(
        &root,
        &data,
        &[
            "save",
            "--session",
            &session,
            "--generation",
            "1",
            "--token-file",
            &token,
        ],
    );
    let stale = call(&root, &data, &["apply", "--operation", &candidate]);
    assert_eq!(stale["status"], "stale", "{stale}");
    assert_eq!(stale["exit"], 3);
    assert_eq!(stale["conflict"][0]["id"], a.as_str());
    assert_eq!(stale["conflict"][0]["version"], 2);
}

/// B1 phase 1: the CLI's own receipts carry `--actor`; receipts a Run wrote
/// (through brn-ai's own connections) carry none, even under `ask --actor`.
#[test]
fn stub_run_receipts_carry_no_actor_and_the_cli_apply_carries_it() {
    let root = root();
    let data = root.workspace("data");
    ok(&root, &data, &["delegation", "--choice", "review-first"]);
    let a = note(&root, &data, "Alpha", "alpha one");
    let stub = Stub::start(vec![
        read_note(&a),
        prepare("a", &a, "Alpha", "alpha from the model"),
        answer("Proposed."),
    ]);
    let chat = configured(&root, &data, &stub);
    let asked = call(
        &root,
        &data,
        &[
            "--actor", "a1", "ask", "--thread", &chat, "--prompt", "Hello",
        ],
    );
    assert_eq!(asked["outcome"], "Completed", "{asked}");
    assert_eq!(asked["authority"]["actor"], "a1");
    let candidate = ok(&root, &data, &["candidates", "--limit", "1"])["candidates"][0]["operation"]
        .as_str()
        .unwrap()
        .to_owned();
    let before = ok(&root, &data, &["history", "--limit", "100"])["receipts"]
        .as_array()
        .unwrap()
        .clone();
    let applied = call(
        &root,
        &data,
        &["apply", "--operation", &candidate, "--actor", "a1"],
    );
    assert_eq!(applied["outcome"], "Applied", "{applied}");
    let history = ok(&root, &data, &["history", "--limit", "100"])["receipts"]
        .as_array()
        .unwrap()
        .clone();
    let receipt = history
        .iter()
        .find(|r| r["operation"] == candidate.as_str())
        .unwrap();
    assert_eq!(receipt["actor"], "a1", "{receipt}");
    let info = ok(&root, &data, &["operation", "--id", &candidate]);
    assert_eq!(info["actor"], "a1");
    // Every receipt from before the apply (setup and the Run's own) has none.
    assert!(!before.is_empty());
    for earlier in &before {
        assert_eq!(earlier["actor"], Value::Null, "{earlier}");
    }
    let run_receipts: Vec<_> = before
        .iter()
        .filter(|r| r["authority"].as_str().is_some_and(|a| a.contains("run")))
        .collect();
    assert!(!run_receipts.is_empty(), "{before:?}");
}
#[test]
fn runs_page_filters_by_thread_and_state() {
    let root = root();
    let data = root.workspace("data");
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

/// SIGINT stops a held run. `spawn` isolates the child, so this passes from
/// any parent shell.
#[test]
fn stub_sigint_interrupts_a_held_stream() {
    let root = root();
    let data = root.workspace("data");
    let (child, _stub, _) = held_ask(&root, &data, &[]);
    signal(&child, libc::SIGINT);
    let run = wait_output(child, Duration::from_secs(30));
    assert_eq!(run.code, 130);
    let envelope = run.envelope();
    assert_eq!(envelope["outcome"], "Interrupted");
    assert_eq!(envelope["stop"], "sigint");
    assert_eq!(runs(&root, &data, &["--state", "interrupted"]).len(), 1);
    // Default mode: exit 0 with "terminal":"Interrupted" (the live-journeys contract).
    let other = root.workspace("default");
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
    let data = root.workspace("data");
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
    let data = root.workspace("data");
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
    let data = root.workspace("data");
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
    // On the owner's instruction, pause stops it: an outside stop reads
    // back as Interrupted, never Cancelled.
    ok(&root, &data, &["pause"]);
    let all = runs(&root, &data, &["--thread", &chat]);
    assert_eq!(all[0]["state"], "interrupted");
    assert_eq!(all[0]["guards_writes"], false);
    assert!(runs(&root, &data, &["--state", "working"]).is_empty());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn scoped_selected_skill_loads_exact_files_and_run_details_hide_prior_scope_after_move() {
    let root = root();
    let data = root.workspace("data");
    let mut script = (0..15)
        .map(|n| {
            tool(
                &format!("ref{n}"),
                "read_skill",
                json!({"id":"installed:run-review","path":format!("references/ref{n}.md")}),
            )
        })
        .collect::<Vec<_>>();
    script.push(tool("builtin", "read_skill", json!({"id":"intake"})));
    script.push(answer("Checked the selected instructions."));
    script.push(answer("General context only."));
    let stub = Stub::start(script);
    let chat = configured(&root, &data, &stub);
    let receipt = ok(
        &root,
        &data,
        &["project-create", "--title", "Private skill scope"],
    );
    let project = receipt["writes"][0]["after"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    ok(
        &root,
        &data,
        &["chat-move", "--thread", &chat, "--project", &project],
    );
    let source = root.join("run-package");
    std::fs::create_dir_all(source.join("references")).unwrap();
    let mut text="---\nname: run-review\ndescription: Synthetic scoped review.\n---\nSCOPED_SELECTED_CANARY\n".to_owned();
    for n in 0..15 {
        std::fs::write(
            source.join(format!("references/ref{n}.md")),
            format!("Exact synthetic reference {n}"),
        )
        .unwrap();
        text.push_str(&format!("[Reference {n}](references/ref{n}.md)\n"));
    }
    std::fs::write(source.join("SKILL.md"), text).unwrap();
    let info = ok(
        &root,
        &data,
        &["skill-install", "--source", source.to_str().unwrap()],
    );
    let digest = info["digest"].as_str().unwrap();
    let selection = call(
        &root,
        &data,
        &[
            "skill-select",
            "--id",
            "installed:run-review",
            "--project",
            &project,
        ],
    );
    assert_error(&selection, "rejected", "invalid_request");
    assert!(stub.requests().is_empty());
    ok(
        &root,
        &data,
        &[
            "skill-enable",
            "--id",
            "installed:run-review",
            "--digest",
            digest,
            "--project",
            &project,
        ],
    );
    ok(
        &root,
        &data,
        &[
            "skill-select",
            "--id",
            "installed:run-review",
            "--project",
            &project,
        ],
    );
    // Explicit owner inventory is global; the separately requested policy is
    // exact scope and cannot disclose P1 enablement/selection in another scope.
    let p2_receipt = ok(&root, &data, &["project-create", "--title", "Other scope"]);
    let p2 = p2_receipt["writes"][0]["after"]["id"].as_str().unwrap();
    for scope in [None, Some(p2)] {
        let args = scope.map_or(vec!["skills"], |id| vec!["skills", "--project", id]);
        let inventory = ok(&root, &data, &args);
        assert!(inventory["skills"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == "installed:run-review"));
        assert!(!inventory["policy"]
            .to_string()
            .contains("installed:run-review"));
        assert!(!inventory["policy"].to_string().contains(digest));
        let args = scope.map_or(vec!["skill-inspect", "--name", "run-review"], |id| {
            vec!["skill-inspect", "--name", "run-review", "--project", id]
        });
        let inspected = ok(&root, &data, &args);
        assert!(inspected["files"]["SKILL.md"]
            .as_str()
            .unwrap()
            .contains("SCOPED_SELECTED_CANARY"));
        assert_eq!(inspected["policy"], json!([]));
    }
    let result = call(
        &root,
        &data,
        &[
            "ask",
            "--thread",
            &chat,
            "--prompt",
            "Use the selected review instructions",
            "--budget",
            "24",
        ],
    );
    assert_eq!(result["outcome"], "Completed", "{result}");
    assert_eq!(stub.requests().len(), 17);
    let requests = stub.requests();
    let first = &requests[0];
    assert!(first["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "user" && m.to_string().contains("SCOPED_SELECTED_CANARY")));
    assert!(!first["messages"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["role"] == "system" && m.to_string().contains("SCOPED_SELECTED_CANARY")));
    let listed = runs(&root, &data, &["--thread", &chat]);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["loaded_skill_details_total"], 17);
    assert_eq!(listed[0]["loaded_skill_details_omitted"], 1);
    assert_eq!(
        listed[0]["loaded_skill_details"].as_array().unwrap().len(),
        16
    );
    assert!(listed[0]["guide_identity"]
        .as_str()
        .unwrap()
        .starts_with("brn-threads-guidance-17:"));
    assert!(listed[0]["loaded_skill_details"]
        .as_array()
        .unwrap()
        .iter()
        .all(|d| d["project"] == project && d["package_digest"] == digest));
    ok(
        &root,
        &data,
        &["skill-remove", "--name", "run-review", "--digest", digest],
    );
    assert_eq!(
        runs(&root, &data, &["--thread", &chat])[0]["loaded_skill_details"],
        listed[0]["loaded_skill_details"],
        "historical permitted identity does not need local bytes"
    );
    ok(
        &root,
        &data,
        &["chat-move", "--thread", &chat, "--project", "none"],
    );
    let moved = runs(&root, &data, &["--thread", &chat]);
    assert_eq!(moved[0]["loaded_skill_details"], json!([]));
    assert_eq!(moved[0]["loaded_skill_details_total"], 0);
    assert_eq!(moved[0]["loaded_skill_details_omitted"], 0);
    let serialized = moved[0].to_string();
    assert!(!serialized.contains("run-review"));
    assert!(!serialized.contains("references/ref"));
    assert!(!serialized.contains(digest));
    let general = ask(&root, &data, &chat);
    assert_eq!(general["outcome"], "Completed");
    let last = stub.requests().pop().unwrap();
    assert!(!last.to_string().contains("SCOPED_SELECTED_CANARY"));
    assert!(!last.to_string().contains("installed:run-review"));
}

const NATIVE_FIXTURE_OUTPUT: &str = "BRN_NATIVE_SUGGESTION_OUTPUT";
const NATIVE_FIXTURE_REVISION: &str = "BRN_NATIVE_SUGGESTION_SOURCE_REVISION";
const NATIVE_FIXTURE_ENDPOINT: &str = "BRN_NATIVE_SUGGESTION_ENDPOINT";
const NATIVE_FIXTURE_REPORT: &str = "native-suggestion-child-report.json";
const NATIVE_FIXTURE_SEED: &str = "native-suggestion-seed.json";
const NATIVE_FIXTURE_MANIFEST: &str = "manifest.json";
const ORIGINAL_WRITING: &str = "Rannatuli põleb vaikselt. Teine lause jääb alles.\n";
const REVISED_WRITING: &str = "Rannatuli põleb selgelt. Teine lause jääb alles.\n";
const UNRELATED_WRITING: &str = "Teine sünteetiline märkus jääb puutumata.\n";

fn fixture_sha256(bytes: &[u8]) -> String {
    brn_intake::hex(&brn_intake::digest(bytes))
}

fn fixture_source_paths() -> (PathBuf, PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repository = manifest.join("../..").canonicalize().unwrap();
    (
        repository,
        manifest.join("tests/agent_runs.rs"),
        manifest.join("tests/support/stub.rs"),
    )
}

fn fixture_source_revision() -> String {
    let supplied = std::env::var(NATIVE_FIXTURE_REVISION).unwrap_or_else(|_| {
        panic!("set {NATIVE_FIXTURE_REVISION} to the committed source revision")
    });
    assert_eq!(
        supplied.len(),
        40,
        "source revision must be a full commit ID"
    );
    assert!(supplied.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let (repository, harness, stub) = fixture_source_paths();
    let output = Command::new("git")
        .args(["-C", repository.to_str().unwrap(), "rev-parse", "HEAD"])
        .output()
        .expect("read the fixture checkout revision");
    assert!(output.status.success(), "git rev-parse failed");
    let head = String::from_utf8(output.stdout).unwrap().trim().to_owned();
    assert_eq!(
        head, supplied,
        "supplied source pin must match checkout HEAD"
    );

    // Bind the two compiled fixture files to the checked-in source snapshot.
    assert_eq!(
        fixture_sha256(include_bytes!("agent_runs.rs")),
        fixture_sha256(&fs::read(&harness).unwrap()),
        "agent_runs.rs changed after this test binary was built"
    );
    assert_eq!(
        fixture_sha256(include_bytes!("support/stub.rs")),
        fixture_sha256(&fs::read(&stub).unwrap()),
        "support/stub.rs changed after this test binary was built"
    );
    for path in [&harness, &stub] {
        let relative = path.strip_prefix(&repository).unwrap();
        let changed = Command::new("git")
            .args([
                "-C",
                repository.to_str().unwrap(),
                "diff",
                "--quiet",
                "HEAD",
                "--",
            ])
            .arg(relative)
            .status()
            .expect("check fixture source against its pinned commit");
        assert!(
            changed.success(),
            "{} differs from pinned HEAD",
            relative.display()
        );
    }
    supplied
}

fn reserve_fixture_output(root: &Root) -> PathBuf {
    let path =
        PathBuf::from(std::env::var_os(NATIVE_FIXTURE_OUTPUT).unwrap_or_else(|| {
            panic!("set {NATIVE_FIXTURE_OUTPUT} to a fresh absolute output root")
        }));
    assert!(path.is_absolute(), "fixture output path must be absolute");
    assert!(
        path.file_name().is_some(),
        "fixture output root needs a final component"
    );
    assert!(
        !path.starts_with(&root.path),
        "durable fixture output must be outside the disposable Root"
    );
    let parent = path.parent().unwrap();
    assert!(parent.is_dir(), "fixture output parent must already exist");
    assert_eq!(
        fs::canonicalize(parent).unwrap(),
        parent,
        "fixture output parent must use its canonical absolute path"
    );
    fs::create_dir(&path).expect("fixture output must be a new, exclusively reserved directory");
    fs::canonicalize(&path).unwrap()
}

fn isolated_fixture_child(root: &Root, output: &Path, endpoint: &str) -> std::process::Child {
    let current = std::env::current_exe().unwrap();
    let mut command = Command::new(current);
    command.args([
        "--exact",
        "native_suggestion_fixture_child",
        "--ignored",
        "--nocapture",
    ]);
    command.env_clear();
    for name in [
        "PATH",
        "DYLD_LIBRARY_PATH",
        "DYLD_FALLBACK_LIBRARY_PATH",
        "LD_LIBRARY_PATH",
    ] {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    command
        .env("HOME", root.join("home"))
        .env("CODEX_HOME", root.join("home"))
        .env("TMPDIR", root.join("tmp"))
        .env("TMP", root.join("tmp"))
        .env("TEMP", root.join("tmp"))
        .env(NATIVE_FIXTURE_OUTPUT, output)
        .env(
            NATIVE_FIXTURE_REVISION,
            std::env::var(NATIVE_FIXTURE_REVISION).unwrap(),
        )
        .env(NATIVE_FIXTURE_ENDPOINT, endpoint)
        .envs(root.env.iter().map(|(key, value)| (key, value)))
        .current_dir(&root.path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start isolated fixture child")
}

fn seed_native_fixture(data: &Path) -> Value {
    use brn_threads_app::{
        RecordData, Workspace, WorkspacePurpose, WritingCommentInput, WritingReadRequirements,
    };

    let mut app =
        Workspace::create_without_default_credentials(data, WorkspacePurpose::Test).unwrap();
    let settings = app.settings().unwrap();
    assert!(settings.auth_file.is_none() && settings.credentials_dir.is_none());
    assert!(settings.mcp.is_none() && settings.helper.is_none());
    let settings_history = app
        .store
        .receipts()
        .unwrap()
        .into_iter()
        .flat_map(|receipt| receipt.writes)
        .filter_map(|write| match write.after.data {
            RecordData::Settings(settings) => Some(settings),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!settings_history.is_empty());
    assert!(settings_history
        .iter()
        .all(|s| s.auth_file.is_none() && s.credentials_dir.is_none()));

    let note = app
        .new_note("Synthetic writing suggestion", ORIGINAL_WRITING)
        .unwrap();
    let unrelated = app
        .new_note("Unrelated synthetic note", UNRELATED_WRITING)
        .unwrap();
    let context = app
        .ensure_writing_context("native-suggestion-context", &note.id, None)
        .unwrap();
    let thread = context.conversation.unwrap();
    let first_quote = "Rannatuli";
    let first = app
        .add_writing_comment(
            "native-suggestion-captured",
            &WritingCommentInput {
                note: note.id.clone(),
                version: note.version,
                thread: thread.clone(),
                range: (0, first_quote.len()),
                quote: first_quote.into(),
                body: "Clarify the first sentence.".into(),
            },
        )
        .unwrap();
    let second_quote = "Teine lause";
    let second_start = ORIGINAL_WRITING.find(second_quote).unwrap();
    let second = app
        .add_writing_comment(
            "native-suggestion-excluded",
            &WritingCommentInput {
                note: note.id.clone(),
                version: note.version,
                thread: thread.clone(),
                range: (second_start, second_start + second_quote.len()),
                quote: second_quote.into(),
                body: "Preserve this sentence.".into(),
            },
        )
        .unwrap();
    let snapshot = app
        .snapshot_writing_request_with_requirements(
            "native-suggestion-request",
            &note.id,
            note.version,
            &thread,
            std::slice::from_ref(&first.id),
            "",
            &WritingReadRequirements::default(),
        )
        .unwrap();
    let request = serde_json::to_value(snapshot.change_request(REVISED_WRITING).unwrap()).unwrap();
    json!({
        "data_directory": data,
        "note": note,
        "unrelated_note": unrelated,
        "thread": thread,
        "request_message": snapshot.request_message,
        "captured_comment": first,
        "excluded_comment": second,
        "request": request
    })
}

fn writing_model_replies(seed: &Value) -> Vec<Reply> {
    let note = &seed["note"];
    let comment = &seed["captured_comment"];
    vec![
        tool(
            "read-note",
            "read_note",
            json!({
                "id": note["id"], "version": note["version"], "start": 0,
                "max_bytes": ORIGINAL_WRITING.len(), "section": false
            }),
        ),
        tool(
            "prepare-suggestion",
            "prepare_changes",
            json!({
                "request_key": "native-suggestion-prepare",
                "request": seed["request"],
                "writing": {
                    "quotations": [], "requirements_reported": [],
                    "proposed_addressed_comment_ids": [comment["id"]]
                }
            }),
        ),
        answer("Prepared the synthetic revision for owner review."),
    ]
}

/// Parent owns the loopback listener and publishes the manifest only after it
/// validates the child report and the exact number of completed HTTP exchanges.
#[test]
#[ignore = "manual synthetic model-suggestion fixture for Mac UI review"]
fn native_suggestion_fixture_parent() {
    let root = root();
    let source_revision = fixture_source_revision();
    let output = reserve_fixture_output(&root);
    let data = output.join("data");
    let mut seed = seed_native_fixture(&data);
    let server = Stub::start(writing_model_replies(&seed));
    let endpoint = format!("http://127.0.0.1:{}", server.port);
    let credentials = server.credentials_in(&output);
    seed["credentials_directory"] = json!(credentials);
    seed["endpoint"] = json!(endpoint);
    let seed_path = output.join(NATIVE_FIXTURE_SEED);
    let mut seed_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&seed_path)
        .unwrap();
    use std::io::Write;
    serde_json::to_writer_pretty(&mut seed_file, &seed).unwrap();
    seed_file.write_all(b"\n").unwrap();
    let child = isolated_fixture_child(&root, &output, &endpoint);
    let result = wait_output(child, Duration::from_secs(180));
    assert_eq!(result.code, 0, "fixture child failed: {}", result.stderr);
    assert_eq!(result.signal, None);
    assert_eq!(
        server.requests().len(),
        3,
        "unexpected provider transport count"
    );
    for request in server.requests() {
        assert_eq!(
            request["model"], "synthetic",
            "provider request escaped the fixture model"
        );
        assert!(
            request["messages"].is_array(),
            "unexpected provider request body"
        );
    }
    let report_path = output.join(NATIVE_FIXTURE_REPORT);
    let report: Value = serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    assert_eq!(report["source_revision"], source_revision);
    assert_eq!(report["endpoint"], endpoint);
    assert_eq!(report["provider_requests"], 3);
    assert_eq!(report["completed"], true);
    assert_eq!(report["candidate_operation"].as_str().is_some(), true);
    assert_eq!(report["cli_readback"], true);

    let (_, harness, stub_source) = fixture_source_paths();
    let runner = std::env::current_exe().unwrap();
    let cli = PathBuf::from(env!("CARGO_BIN_EXE_brn"));
    let manifest = json!({
        "format": "BRN native model-suggestion fixture v1",
        "source_identity": {
            "repository_revision": source_revision,
            "revision_validation": "invocation pin matched git HEAD; fixture source bytes matched compiled includes",
            "harness_path": harness,
            "harness_sha256": fixture_sha256(include_bytes!("agent_runs.rs")),
            "stub_path": stub_source,
            "stub_sha256": fixture_sha256(include_bytes!("support/stub.rs"))
        },
        "binaries": {
            "fixture_test_harness": runner,
            "fixture_test_harness_sha256": fixture_sha256(&fs::read(&runner).unwrap()),
            "ordinary_cli": cli,
            "ordinary_cli_sha256": fixture_sha256(&fs::read(&cli).unwrap())
        },
        "reproduction_command": format!(
            "BRN_NATIVE_SUGGESTION_OUTPUT=<fresh absolute directory> BRN_NATIVE_SUGGESTION_SOURCE_REVISION={source_revision} rustup run 1.98.1 cargo test -p brn --test agent_runs --locked --offline --jobs 1 native_suggestion_fixture_parent -- --ignored --exact --nocapture"
        ),
        "data_directory": output.join("data"),
        "seed": seed,
        "runtime_validation": report,
        "provider": "Copilot",
        "model": "synthetic",
        "synthetic_settings": {
            "provider": "Copilot", "model": "synthetic", "effort": "low",
            "credentials_directory": seed["credentials_directory"],
            "auth_file": null, "helper": null, "mcp": null
        },
        "transport": {
            "endpoint": endpoint,
            "requests": 3,
            "credentials": "synthetic cached session token only",
            "proxy": "closed port with loopback-only bypass"
        },
        "review_state": {
            "suggestion_is_model_reported": true,
            "selected_comment_ids": [],
            "captured_comments_pending": report["captured_comments_pending"],
            "candidate_unapplied": report["candidate_unapplied"],
            "no_apply_receipt": report["no_apply_receipt"],
            "no_working_run": report["no_working_run"],
            "unrelated_note_unchanged": report["unrelated_note_unchanged"]
        },
        "native_observed": false
    });
    let manifest_path = output.join(NATIVE_FIXTURE_MANIFEST);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&manifest_path)
        .expect("success manifest must be new");
    serde_json::to_writer_pretty(&mut file, &manifest).unwrap();
    file.write_all(b"\n").unwrap();
    fs::remove_file(output.join(NATIVE_FIXTURE_REPORT)).unwrap();
    println!("Synthetic writing fixture: {}", output.display());
}

/// Ignored child selected only by `native_suggestion_fixture_parent`.
#[ignore = "only run under the isolated fixture parent"]
#[tokio::test]
async fn native_suggestion_fixture_child() {
    use brn_ai::threads::{run_prepared, OwnerRequest, TraceLevel};
    use brn_threads_app::{
        Record, RecordData, RunState, ThreadRunStart, Workspace, WRITING_EVIDENCE_ROLE,
    };
    use tokio_util::sync::CancellationToken;

    let source_revision = fixture_source_revision();
    let output = PathBuf::from(std::env::var_os(NATIVE_FIXTURE_OUTPUT).unwrap());
    let endpoint = std::env::var(NATIVE_FIXTURE_ENDPOINT).unwrap();
    let address = endpoint.strip_prefix("http://127.0.0.1:").unwrap();
    assert!(
        address.parse::<u16>().is_ok(),
        "endpoint must be literal loopback HTTP"
    );
    let seed: Value =
        serde_json::from_slice(&fs::read(output.join(NATIVE_FIXTURE_SEED)).unwrap()).unwrap();
    let data = PathBuf::from(seed["data_directory"].as_str().unwrap());
    let credentials = PathBuf::from(seed["credentials_directory"].as_str().unwrap());
    assert_eq!(seed["endpoint"], endpoint);
    let credential_cache: Value =
        serde_json::from_slice(&fs::read(credentials.join("copilot.json")).unwrap()).unwrap();
    assert_eq!(credential_cache["token"], "SYNTHETIC");
    assert_eq!(credential_cache["expires_at"], 4102444800i64);
    assert_eq!(credential_cache["endpoints"]["api"], endpoint);
    let mut app = Workspace::open_without_default_credentials(&data).unwrap();
    let initial = app.settings().unwrap();
    assert!(initial.auth_file.is_none() && initial.credentials_dir.is_none());
    assert!(initial.mcp.is_none() && initial.helper.is_none());
    let initial_settings = app
        .store
        .receipts()
        .unwrap()
        .into_iter()
        .flat_map(|receipt| receipt.writes)
        .filter_map(|write| match write.after.data {
            RecordData::Settings(settings) => Some(settings),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!initial_settings.is_empty());
    assert!(initial_settings
        .iter()
        .all(|settings| settings.auth_file.is_none() && settings.credentials_dir.is_none()));
    assert_eq!(initial.provider, "Codex");

    let mut configured = initial;
    configured.provider = "Copilot".into();
    configured.model = "synthetic".into();
    configured.effort = "low".into();
    configured.credentials_dir = Some(credentials.to_string_lossy().into_owned());
    app.configure(configured).unwrap();

    assert!(credentials.join("copilot.json").is_file());
    let settings = app.settings().unwrap();
    assert_eq!(settings.provider, "Copilot");
    assert_eq!(settings.model, "synthetic");
    assert_eq!(settings.effort, "low");
    assert_eq!(
        settings.credentials_dir.as_deref(),
        Some(credentials.to_str().unwrap())
    );
    assert!(settings.auth_file.is_none() && settings.helper.is_none() && settings.mcp.is_none());

    let request_message = seed["request_message"].as_str().unwrap().to_owned();
    let thread = seed["thread"].as_str().unwrap().to_owned();
    let prepared = app
        .prepare_thread_run(ThreadRunStart {
            thread: thread.clone(),
            prompt: "Rewrite the saved note using its captured feedback.".into(),
            request_message: None,
            queue_claim: None,
            owner_request: Some(OwnerRequest::RewriteNote {
                request_message: request_message.clone(),
            }),
            sources: vec![],
            max_tool_rounds: Some(4),
            trace: TraceLevel::Full,
            offered_copilot_models: None,
        })
        .unwrap();
    let result = run_prepared(
        prepared.prepared,
        CancellationToken::new(),
        std::sync::Arc::new(|_| {}),
    )
    .await
    .unwrap();
    assert!(matches!(
        result.answer.terminal,
        brn_ai::AiTerminal::Completed
    ));
    assert_eq!(
        result.answer.text,
        "Prepared the synthetic revision for owner review."
    );
    let trace = result.trace.as_ref().expect("full runtime trace");
    let tools = trace
        .calls
        .iter()
        .map(|call| (call.tool.as_str(), call.outcome.as_str()))
        .collect::<Vec<_>>();
    assert_eq!(
        tools,
        [("read_note", "success"), ("prepare_changes", "success")]
    );
    let read = &trace.calls[0].result;
    assert_eq!(read["outcome"], "Read");
    assert_eq!(read["note"]["complete"], true);
    assert_eq!(read["note"]["markdown"], ORIGINAL_WRITING);
    assert_eq!(read["note"]["total_bytes"], ORIGINAL_WRITING.len());
    assert_eq!(trace.calls[1].result["operation"].as_str().is_some(), true);
    let candidate_id = trace.calls[1].result["operation"]
        .as_str()
        .unwrap()
        .to_owned();
    let run_id = result.run_id.clone();
    drop(app);

    let mut reopened = Workspace::open_without_default_credentials(&data).unwrap();
    let prepared_candidates = reopened.store.candidates().unwrap();
    assert_eq!(prepared_candidates.len(), 1);
    let candidate = prepared_candidates
        .iter()
        .find(|candidate| candidate.operation.as_str() == candidate_id)
        .unwrap();
    let evidence_message = candidate
        .request
        .writes
        .iter()
        .find_map(|write| match &write.data {
            RecordData::Message(message) if message.role == WRITING_EVIDENCE_ROLE => Some(message),
            _ => None,
        })
        .expect("runtime-created host evidence");
    let evidence: Value = serde_json::from_str(&evidence_message.text).unwrap();
    assert_eq!(evidence["request_message"], request_message);
    assert_eq!(evidence["thread"], thread);
    let note = &seed["note"];
    let captured = &seed["captured_comment"];
    let unrelated = &seed["unrelated_note"];
    assert_eq!(
        evidence["proposed_addressed_comment_ids"],
        json!([captured["id"]])
    );
    let read_evidence = evidence["reads"]
        .as_array()
        .unwrap()
        .iter()
        .find(|read| read["id"] == note["id"])
        .unwrap();
    assert_eq!(read_evidence["version"], 1);
    assert_eq!(read_evidence["complete"], true);
    assert_eq!(
        read_evidence["covered_retained_bytes"],
        ORIGINAL_WRITING.len()
    );
    assert_eq!(
        read_evidence["total_retained_bytes"],
        ORIGINAL_WRITING.len()
    );
    let review = reopened
        .writing_candidate_review(&candidate_id, Some(&thread))
        .unwrap();
    assert_eq!(
        review.proposed_addressed_comment_ids,
        vec![captured["id"].as_str().unwrap().to_owned()]
    );
    assert_eq!(
        review.model_explanation,
        "Prepared the synthetic revision for owner review."
    );
    assert_eq!(
        candidate
            .request
            .writes
            .iter()
            .find_map(|write| match &write.data {
                RecordData::Note(value) if write.id == note["id"] => Some(value.markdown.as_str()),
                _ => None,
            }),
        Some(REVISED_WRITING)
    );
    assert_eq!(
        reopened
            .store
            .record(note["id"].as_str().unwrap())
            .unwrap()
            .unwrap()
            .version,
        1
    );
    let original_note: Record = serde_json::from_value(note.clone()).unwrap();
    assert_eq!(
        reopened
            .store
            .record(note["id"].as_str().unwrap())
            .unwrap()
            .unwrap(),
        original_note
    );
    assert_eq!(
        reopened
            .store
            .record(unrelated["id"].as_str().unwrap())
            .unwrap()
            .unwrap(),
        serde_json::from_value(unrelated.clone()).unwrap()
    );
    let comments = reopened
        .writing_comments(note["id"].as_str().unwrap())
        .unwrap();
    assert_eq!(comments.len(), 2);
    assert!(comments.iter().all(|comment| !comment.addressed));
    let no_apply_receipt = reopened
        .store
        .receipt(&reopened.store.operation(&candidate_id).unwrap())
        .unwrap()
        .is_none();
    assert!(no_apply_receipt);
    let run_records = reopened.records().unwrap();
    let no_working_run = run_records.iter().all(
        |record| !matches!(&record.data, RecordData::Run(run) if run.state == RunState::Working),
    );
    assert!(no_working_run);
    assert!(run_records.iter().any(|record| record.id == run_id
        && matches!(&record.data, RecordData::Run(run) if run.state == RunState::Completed)));
    let unrelated_note_unchanged = reopened
        .store
        .record(unrelated["id"].as_str().unwrap())
        .unwrap()
        .unwrap()
        == serde_json::from_value(unrelated.clone()).unwrap();
    assert!(unrelated_note_unchanged);
    drop(reopened);

    let mut cli_root = Root::new();
    cli_root.env = proxy_env();
    let listed = ok(&cli_root, &data, &["candidates", "--limit", "10"]);
    assert!(listed["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .any(|candidate| candidate["operation"] == candidate_id));
    let note_read = ok(
        &cli_root,
        &data,
        &["read", "--id", note["id"].as_str().unwrap()],
    );
    assert_eq!(note_read["version"], 1);
    assert_eq!(note_read["data"]["Note"]["markdown"], ORIGINAL_WRITING);
    let run_readback = ok(&cli_root, &data, &["runs", "--thread", &thread]);
    assert!(run_readback["runs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|run| run["id"] == run_id && run["state"] == "completed"));

    let current = std::env::current_exe().unwrap();
    let cli = PathBuf::from(env!("CARGO_BIN_EXE_brn"));
    let report = json!({
        "source_revision": source_revision,
        "endpoint": endpoint,
        "provider_requests": 3,
        "completed": true,
        "cli_readback": true,
        "data_directory": data,
        "note": note,
        "unrelated_note": unrelated,
        "thread": thread,
        "request_message": request_message,
        "captured_comment": captured,
        "excluded_comment": seed["excluded_comment"],
        "candidate_operation": candidate_id,
        "run_id": run_id,
        "original_markdown": ORIGINAL_WRITING,
        "original_markdown_utf8_bytes": ORIGINAL_WRITING.len(),
        "original_markdown_sha256": fixture_sha256(ORIGINAL_WRITING.as_bytes()),
        "revised_markdown": REVISED_WRITING,
        "revised_markdown_utf8_bytes": REVISED_WRITING.len(),
        "revised_markdown_sha256": fixture_sha256(REVISED_WRITING.as_bytes()),
        "unrelated_markdown": UNRELATED_WRITING,
        "unrelated_markdown_utf8_bytes": UNRELATED_WRITING.len(),
        "unrelated_markdown_sha256": fixture_sha256(UNRELATED_WRITING.as_bytes()),
        "candidate_unapplied": true,
        "captured_comments_pending": true,
        "no_apply_receipt": no_apply_receipt,
        "no_working_run": no_working_run,
        "unrelated_note_unchanged": unrelated_note_unchanged,
        "runtime_trace_tools": tools.iter().map(|(tool, outcome)| json!({"tool":tool,"outcome":outcome})).collect::<Vec<_>>(),
        "ordinary_cli_binary": cli,
        "ordinary_cli_binary_sha256": fixture_sha256(&fs::read(&cli).unwrap()),
        "fixture_test_binary": current,
        "fixture_test_binary_sha256": fixture_sha256(&fs::read(&current).unwrap())
    });
    let report_path = output.join(NATIVE_FIXTURE_REPORT);
    let mut report_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(report_path)
        .unwrap();
    use std::io::Write;
    serde_json::to_writer_pretty(&mut report_file, &report).unwrap();
    report_file.write_all(b"\n").unwrap();
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn restored_selected_package_without_local_bytes_is_unavailable_before_provider_contact() {
    let root = root();
    let data = root.workspace("original");
    let stub = Stub::start(vec![answer("MUST_NOT_DISPATCH")]);
    let chat = configured(&root, &data, &stub);
    let note = id(&ok(
        &root,
        &data,
        &[
            "note-create",
            "--title",
            "Retained work",
            "--text",
            "CANONICAL_SKILL_RECOVERY_POSITIVE",
        ],
    ));
    let source = root.join("restore-package");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("SKILL.md"),"---\nname: restore-review\ndescription: Synthetic restored selection.\n---\nLOCAL_INSTRUCTION_BYTES_CANARY\n").unwrap();
    let package = ok(
        &root,
        &data,
        &["skill-install", "--source", source.to_str().unwrap()],
    );
    let digest = package["digest"].as_str().unwrap();
    ok(
        &root,
        &data,
        &[
            "skill-enable",
            "--id",
            "installed:restore-review",
            "--digest",
            digest,
        ],
    );
    ok(
        &root,
        &data,
        &["skill-select", "--id", "installed:restore-review"],
    );
    let canonical = ok(&root, &data, &["settings"]);
    let backup = root.join("skill-backup.sqlite3");
    ok(
        &root,
        &data,
        &["backup", "--destination", backup.to_str().unwrap()],
    );
    let restored = root.join("restored");
    ok(
        &root,
        &restored,
        &["restore", "--source", backup.to_str().unwrap()],
    );
    assert!(!restored.join("skills").exists());
    assert_eq!(ok(&root, &restored, &["settings"]), canonical);
    let before = ok(&root, &restored, &["read", "--id", &note]);
    let result = ask(&root, &restored, &chat);
    assert_error(&result, "failed", "unavailable");
    assert_eq!(result["error"]["message"],"The selected skill is unavailable. Reinstall its exact approved package or change the scoped selection before continuing.");
    assert!(
        stub.requests().is_empty(),
        "no auth, summary or chat dispatch"
    );
    assert_eq!(ok(&root, &restored, &["settings"]), canonical);
    assert_eq!(ok(&root, &restored, &["read", "--id", &note]), before);
    assert!(runs(&root, &restored, &[]).is_empty());
}

#[test]
fn every_packaged_builtin_can_be_explicitly_selected_and_is_actually_delivered() {
    let root = root();
    let data = root.workspace("builtins");
    let ids = [
        "intake",
        "maintain-notes",
        "resolve-conflict",
        "prepare-reply",
    ];
    let texts = [
        include_str!("../../../agent/skills/intake/SKILL.md"),
        include_str!("../../../agent/skills/maintain-notes/SKILL.md"),
        include_str!("../../../agent/skills/resolve-conflict/SKILL.md"),
        include_str!("../../../agent/skills/prepare-reply/SKILL.md"),
    ];
    let stub = Stub::start(
        ids.iter()
            .map(|_| answer("Builtin guidance considered."))
            .collect(),
    );
    let chat = configured(&root, &data, &stub);
    for (n, id) in ids.iter().enumerate() {
        ok(&root, &data, &["skill-select", "--id", id]);
        let result = ask(&root, &data, &chat);
        assert_eq!(result["status"], "ok", "{result}");
        assert_eq!(stub.requests().len(), n + 1);
        assert!(
            stub.requests()[n]["messages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|m| m["role"] == "system"
                    && m["content"]
                        .as_array()
                        .is_some_and(|parts| parts.iter().any(|part| part["type"] == "text"
                            && part["text"]
                                .as_str()
                                .is_some_and(|text| text.contains(texts[n]))))),
            "{id}: exact selected body absent from actual system text blocks"
        );
        let saved = runs(&root, &data, &["--thread", &chat]);
        assert!(saved.iter().any(|r| r["loaded_skill_details"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["id"] == *id && d["origin"] == "builtin" && d["file"] == "SKILL.md")));
    }
}
