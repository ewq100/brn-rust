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
        .starts_with("brn-threads-guidance-15:"));
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
