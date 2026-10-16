//! Process checks of bounded inspection: paged history, candidates and
//! records, revisions, revision and paged read, runs, and operation lookup.
mod support;
use serde_json::{json, Value};
use std::path::Path;
use support::*;

/// Every page of `command` with `options`, following `result.cursor`.
fn all_pages(root: &Root, data: &Path, command: &[&str], list: &str) -> Vec<Vec<Value>> {
    let mut pages = vec![];
    let mut cursor: Option<String> = None;
    loop {
        let mut argv: Vec<&str> = command.to_vec();
        if let Some(cursor) = &cursor {
            argv.extend(["--cursor", cursor.as_str()]);
        }
        let envelope = call(root, data, &argv);
        assert_eq!(envelope["status"], "ok", "{envelope}");
        let result = &envelope["result"];
        assert_eq!(envelope["page"]["cursor"], result["cursor"]);
        assert_eq!(envelope["page"]["shown"], result["shown"]);
        pages.push(result[list].as_array().unwrap().clone());
        match result["cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => return pages,
        }
    }
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
fn operations(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .map(|item| item["operation"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn history_pages_are_bounded_stable_and_filter_by_record() {
    let root = Root::new();
    let data = root.workspace("data");
    for i in 0..25 {
        note(&root, &data, &format!("N{i}"), "text");
    }
    let unbounded = call(&root, &data, &["history"]);
    assert_eq!(unbounded["page"], Value::Null);
    let default_mode = run(&root, false, &data, &["history"]);
    assert_eq!(default_mode.stdout, format!("{}\n", unbounded["result"]));
    let full: Vec<String> = operations(unbounded["result"].as_array().unwrap());
    // Writes between pages are seen later and nothing repeats.
    let first = call(&root, &data, &["history", "--limit", "10"]);
    assert_eq!(first["result"]["shown"], 10);
    assert_eq!(first["page"]["limit"], 10);
    assert_eq!(first["page"]["total"], full.len());
    let cursor = first["result"]["cursor"].as_str().unwrap().to_owned();
    let late = note(&root, &data, "late", "text");
    let mut seen = operations(first["result"]["receipts"].as_array().unwrap());
    let mut next = Some(cursor);
    while let Some(cursor) = next {
        let page = ok(
            &root,
            &data,
            &["history", "--limit", "10", "--cursor", &cursor],
        );
        seen.extend(operations(page["receipts"].as_array().unwrap()));
        next = page["cursor"].as_str().map(str::to_owned);
    }
    let after = operations(ok(&root, &data, &["history"]).as_array().unwrap());
    assert_eq!(seen, after);
    assert_eq!(after.len(), full.len() + 1);
    // Default mode prints the page result as one JSON line.
    let page = plain(&root, &data, &["history", "--limit", "3"]);
    assert_eq!(page["shown"], 3);
    // --record keeps the receipts that wrote that record.
    let (session, token) = begin_edit(&root, &data, &late, 1);
    let session = id(&session);
    let text = [
        "update-buffer",
        "--session",
        &session,
        "--generation",
        "1",
        "--token-file",
        &token,
    ];
    ok(&root, &data, &[&text[..], &["--text", "edited"]].concat());
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
    let pages = all_pages(
        &root,
        &data,
        &["history", "--record", &late, "--limit", "1"],
        "receipts",
    );
    assert_eq!(pages.len(), 2);
    for receipt in pages.iter().flatten() {
        assert!(receipt["writes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["id"] == late.as_str()));
        for field in [
            "actor",
            "reason",
            "authority",
            "committed_at",
            "undo_of",
            "writes_total",
        ] {
            assert!(receipt.get(field).is_some(), "{field}");
        }
    }
    let record = call(&root, &data, &["history", "--record", &late]);
    assert_eq!(record["page"]["total"], 2);
    // A receipt with more than 32 writes is cut to 32 with writes_total.
    let project = call(&root, &data, &["project-create", "--title", "P"]);
    let project = created(&project, "Project");
    for i in 0..33 {
        ok(
            &root,
            &data,
            &[
                "project-chat",
                "--project",
                &project,
                "--title",
                &format!("C{i}"),
            ],
        );
    }
    let preview = run(
        &root,
        false,
        &data,
        &["project-delete-preview", "--project", &project],
    );
    let preview_file = root.file("preview.json", preview.stdout.trim_end().as_bytes());
    let deleted = call(
        &root,
        &data,
        &[
            "project-delete",
            "--preview-file",
            &preview_file,
            "--actions",
            "{}",
        ],
    );
    assert_eq!(deleted["status"], "ok", "{deleted}");
    let last = ok(
        &root,
        &data,
        &["history", "--record", &project, "--limit", "100"],
    );
    let receipt = last["receipts"].as_array().unwrap().last().unwrap();
    assert_eq!(receipt["writes"].as_array().unwrap().len(), 32);
    assert!(receipt["writes_total"].as_u64().unwrap() > 32);
}

fn edit(root: &Root, data: &Path, note: &str, base: u64, text: &str) {
    let (session, token) = begin_edit(root, data, note, base);
    let session = id(&session);
    let file = root.file(&format!("edit-{base}.md"), text.as_bytes());
    ok(
        root,
        data,
        &[
            "update-buffer",
            "--session",
            &session,
            "--generation",
            "1",
            "--text-file",
            &file,
            "--token-file",
            &token,
        ],
    );
    let saved = call(
        root,
        data,
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
    assert_eq!(saved["outcome"], "Saved", "{saved}");
}
/// Reassembles one revision from byte pages of `limit`.
fn reassemble(root: &Root, data: &Path, note: &str, version: u64, limit: usize) -> String {
    let mut text = String::new();
    let mut cursor: Option<String> = None;
    loop {
        let version = version.to_string();
        let limit = limit.to_string();
        let mut argv = vec![
            "revision",
            "--note",
            note,
            "--version",
            &version,
            "--limit",
            &limit,
        ];
        if let Some(cursor) = &cursor {
            argv.extend(["--cursor", cursor.as_str()]);
        }
        let envelope = call(root, data, &argv);
        assert_eq!(envelope["status"], "ok", "{envelope}");
        text.push_str(envelope["result"]["markdown"].as_str().unwrap());
        match envelope["result"]["cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => return text,
        }
    }
}

#[test]
fn revisions_and_revision_read_page_through_a_large_note() {
    let root = Root::new();
    let data = root.workspace("data");
    let body = |tag: &str| {
        let line = format!("{tag} — “β” 👩‍💻 مرحبا 你好 $HOME `x`\n");
        line.repeat(200 * 1024 / line.len() + 1)
    };
    let texts = [body("v1"), body("v2"), body("v3"), body("v4")];
    assert!(texts[0].len() > 200 * 1024);
    let file = root.file("v1.md", texts[0].as_bytes());
    let created = ok(
        &root,
        &data,
        &["note-create", "--title", "Large", "--text-file", &file],
    );
    let note = id(&created);
    for (base, text) in texts.iter().enumerate().skip(1) {
        edit(&root, &data, &note, base as u64, text);
    }
    let pages = all_pages(
        &root,
        &data,
        &["revisions", "--note", &note, "--limit", "3"],
        "revisions",
    );
    assert_eq!(pages.len(), 2);
    let versions: Vec<u64> = pages
        .iter()
        .flatten()
        .map(|r| r["version"].as_u64().unwrap())
        .collect();
    assert_eq!(versions, [1, 2, 3, 4]);
    for (index, text) in texts.iter().enumerate() {
        assert_eq!(
            &reassemble(&root, &data, &note, index as u64 + 1, 65536),
            text
        );
    }
    // A limit that falls inside a character still advances without splitting it.
    let small = "é👩‍💻x“β”";
    let file = root.file("small.md", small.as_bytes());
    let small_note = id(&ok(
        &root,
        &data,
        &["note-create", "--title", "S", "--text-file", &file],
    ));
    assert_eq!(reassemble(&root, &data, &small_note, 1, 5), small);
    // Paged read of the current version reassembles it too.
    let mut text = String::new();
    let mut cursor: Option<String> = None;
    loop {
        let mut argv = vec!["read", "--id", note.as_str(), "--limit", "50000"];
        if let Some(cursor) = &cursor {
            argv.extend(["--cursor", cursor.as_str()]);
        }
        let envelope = call(&root, &data, &argv);
        assert_eq!(envelope["outcome"], "Read", "{envelope}");
        text.push_str(envelope["result"]["note"]["markdown"].as_str().unwrap());
        match envelope["result"]["cursor"].as_str() {
            Some(next) => cursor = Some(next.to_owned()),
            None => break,
        }
    }
    assert_eq!(text, texts[3]);
}

#[test]
fn records_page_by_kind_and_cursor() {
    let root = Root::new();
    let data = root.workspace("data");
    let mut ids = vec![];
    for i in 0..5 {
        ids.push(note(&root, &data, &format!("Note {i}"), "text"));
    }
    ok(&root, &data, &["thread-start", "--title", "Chat"]);
    ids.sort();
    let pages = all_pages(
        &root,
        &data,
        &["records", "--kind", "Note", "--limit", "2"],
        "records",
    );
    assert_eq!(pages.len(), 3);
    let listed: Vec<String> = pages.iter().flatten().map(id).collect();
    assert_eq!(listed, ids);
    for record in pages.iter().flatten() {
        assert_eq!(record["kind"], "Note");
        assert!(record["title"].as_str().unwrap().starts_with("Note "));
    }
    let every = all_pages(&root, &data, &["records", "--limit", "100"], "records");
    let unbounded = ok(&root, &data, &["records"]);
    assert_eq!(every.concat().len(), unbounded.as_array().unwrap().len());
}

#[test]
fn runs_page_validates_options_and_lists_nothing_on_a_fresh_workspace() {
    let root = Root::new();
    let data = root.workspace("data");
    let envelope = call(&root, &data, &["runs"]);
    assert_eq!(envelope["status"], "ok", "{envelope}");
    assert_eq!(
        envelope["result"],
        json!({"runs": [], "shown": 0, "cursor": null, "total": null})
    );
    assert_eq!(envelope["page"]["limit"], 20);
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let filtered = ok(
        &root,
        &data,
        &[
            "runs", "--thread", &chat, "--state", "working", "--cursor", "zzz",
        ],
    );
    assert_eq!(filtered["runs"], json!([]));
    let absent = root.join("absent");
    for bad in [["--state", "bogus"], ["--limit", "0"]] {
        let envelope = call(&root, &absent, &[&["runs"][..], &bad[..]].concat());
        assert_error(&envelope, "usage", "invalid_value");
        assert_eq!(
            envelope["error"]["details"]["option"],
            bad[0].trim_start_matches("--")
        );
    }
    assert!(!absent.exists());
}

#[test]
fn page_options_are_refused_before_opening() {
    let root = Root::new();
    let data = root.join("absent");
    let cases: &[(&[&str], &str, &str)] = &[
        (&["history", "--cursor", "abc"], "invalid_value", "cursor"),
        (&["history", "--limit", "0"], "invalid_value", "limit"),
        (&["history", "--limit", "101"], "invalid_value", "limit"),
        (&["candidates", "--limit", "-1"], "invalid_value", "limit"),
        (&["runs", "--state", "bogus"], "invalid_value", "state"),
        (&["records", "--kind", "Bogus"], "invalid_value", "kind"),
        (
            &["read", "--id", "x", "--cursor", "1.0"],
            "missing_option",
            "limit",
        ),
        (
            &["read", "--id", "x", "--limit", "10", "--cursor", "0.0"],
            "invalid_value",
            "cursor",
        ),
        (
            &[
                "revision",
                "--note",
                "x",
                "--version",
                "1",
                "--cursor",
                "-1",
            ],
            "invalid_value",
            "cursor",
        ),
        (
            &["revisions", "--note", "x", "--limit", "33"],
            "invalid_value",
            "limit",
        ),
    ];
    for (argv, code, option) in cases {
        let envelope = call(&root, &data, argv);
        assert_error(&envelope, "usage", code);
        assert_eq!(envelope["error"]["details"]["option"], *option, "{argv:?}");
        let default = run(&root, false, &data, argv);
        assert_eq!(default.code, 1, "{argv:?}");
        assert!(default.stdout.is_empty());
        assert!(
            default.stderr.contains(&format!("--{option}")),
            "{}",
            default.stderr
        );
    }
    assert!(!data.exists());
}

#[test]
fn read_pages_detect_a_change_between_pages() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "0123456789");
    let first = ok(&root, &data, &["read", "--id", &note, "--limit", "4"]);
    assert_eq!(first["note"]["markdown"], "0123");
    let cursor = first["cursor"].as_str().unwrap().to_owned();
    assert_eq!(cursor, "1.4");
    edit(&root, &data, &note, 1, "changed");
    let stale = call(
        &root,
        &data,
        &["read", "--id", &note, "--limit", "4", "--cursor", &cursor],
    );
    assert_eq!(stale["status"], "stale", "{stale}");
    assert_eq!(stale["exit"], 3);
    assert_eq!(stale["outcome"], "Stale");
    assert_eq!(
        stale["conflict"],
        json!([{"id": note, "kind": "Note", "version": 2, "archived": false, "open_edit": false}])
    );
    // Default mode exits 0 with the same result.
    let default = plain(
        &root,
        &data,
        &["read", "--id", &note, "--limit", "4", "--cursor", &cursor],
    );
    assert_eq!(default["outcome"], "Stale");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let thread = call(&root, &data, &["read", "--id", &chat, "--limit", "4"]);
    assert_error(&thread, "rejected", "invalid_request");
    assert_eq!(thread["error"]["details"]["reason"], "wrong_kind");
    let missing = call(
        &root,
        &data,
        &["read", "--id", "no-such-note", "--limit", "4"],
    );
    assert_error(&missing, "rejected", "not_found");
}

#[test]
fn page_refusals_after_opening_name_the_cause() {
    let root = Root::new();
    let data = root.workspace("data");
    let note = note(&root, &data, "N", "é");
    let unknown = call(&root, &data, &["history", "--record", "no-such-record"]);
    assert_error(&unknown, "rejected", "not_found");
    let inside = call(
        &root,
        &data,
        &[
            "revision",
            "--note",
            &note,
            "--version",
            "1",
            "--cursor",
            "1",
        ],
    );
    assert_error(&inside, "rejected", "invalid_request");
    let absent = call(&root, &data, &["revisions", "--note", "no-such-note"]);
    assert_error(&absent, "rejected", "not_found");
}

#[test]
fn identical_page_reads_return_identical_pages() {
    let root = Root::new();
    let data = root.workspace("data");
    for i in 0..3 {
        note(&root, &data, &format!("N{i}"), "text");
    }
    let before = ok(&root, &data, &["records"]);
    let argv = ["history", "--limit", "2"];
    let first = call(&root, &data, &argv);
    let second = call(&root, &data, &argv);
    assert_eq!(first, second);
    assert_eq!(first["retry"], "read");
    assert_eq!(ok(&root, &data, &["records"]), before);
}

/// `operation --key KEY --command COMMAND`: the envelope.
fn lookup(root: &Root, data: &Path, command: &str, key: &str) -> Value {
    call(
        root,
        data,
        &["operation", "--key", key, "--command", command],
    )
}
/// A keyed call, its replay and the lookup all name one operation.
fn keyed_once(root: &Root, data: &Path, argv: &[&str], command: &str, key: &str) -> Value {
    let argv = [argv, &["--key", key]].concat();
    let first = call(root, data, &argv);
    assert_eq!(first["status"], "ok", "{first}");
    let replay = call(root, data, &argv);
    assert_eq!(replay["operation"], first["operation"], "{argv:?}");
    let found = lookup(root, data, command, key);
    assert_eq!(found["status"], "ok", "{found}");
    assert_eq!(found["key"], key);
    assert_eq!(found["retry"], "read");
    assert_eq!(found["result"]["operation"], first["operation"], "{argv:?}");
    assert_eq!(found["result"]["state"], "applied");
    assert_eq!(found["result"]["writes"], first["changed"], "{argv:?}");
    first
}

#[test]
fn operation_by_key_finds_every_keyed_command() {
    let root = Root::new();
    let data = root.workspace("data");
    let made = keyed_once(
        &root,
        &data,
        &["note-create", "--title", "N", "--text", "x"],
        "note-create",
        "n1",
    );
    let chat = keyed_once(
        &root,
        &data,
        &["thread-start", "--title", "C"],
        "thread-start",
        "t1",
    );
    let chat = chat["changed"][0]["id"].as_str().unwrap().to_owned();
    keyed_once(
        &root,
        &data,
        &["queue", "--thread", &chat, "--text", "later"],
        "queue",
        "q1",
    );
    let source = root.file("doc.md", b"# Doc\n\nBody.\n");
    let attached = keyed_once(
        &root,
        &data,
        &[
            "intake", "--source", &source, "--intent", "chat", "--thread", &chat,
        ],
        "intake",
        "i1",
    );
    let attachment = created(&attached, "Note");
    keyed_once(
        &root,
        &data,
        &["keep-in-brn", "--note", &attachment, "--base", "1"],
        "keep-in-brn",
        "k1",
    );
    keyed_once(
        &root,
        &data,
        &["undo", "--operation", made["operation"].as_str().unwrap()],
        "undo",
        "u1",
    );
    keyed_once(
        &root,
        &data,
        &["archive", "--thread", &chat],
        "archive",
        "a1",
    );
    // resolve shares archive's scope.
    assert_eq!(
        lookup(&root, &data, "resolve", "a1")["result"]["operation"],
        lookup(&root, &data, "archive", "a1")["result"]["operation"]
    );
    keyed_once(
        &root,
        &data,
        &["unarchive", "--thread", &chat],
        "unarchive",
        "a2",
    );
    let project = keyed_once(
        &root,
        &data,
        &["project-create", "--title", "P"],
        "project-create",
        "p1",
    );
    // Project commands share one raw-key scope: any project command finds it.
    let via_chat = lookup(&root, &data, "project-chat", "p1");
    assert_eq!(via_chat["result"]["operation"], project["operation"]);
}

#[test]
fn candidates_and_operation_report_unapplied_candidates() {
    let root = Root::new();
    let data = root.workspace("data");
    let made = call(
        &root,
        &data,
        &["note-create", "--title", "N", "--text", "one"],
    );
    let note = made["changed"][0]["id"].as_str().unwrap().to_owned();
    let original = made["operation"].as_str().unwrap().to_owned();
    let (session, token) = begin_edit(&root, &data, &note, 1);
    let session = id(&session);
    let undo = ["undo", "--operation", original.as_str(), "--key", "u1"];
    let deferred = call(&root, &data, &undo);
    assert_eq!(deferred["status"], "deferred", "{deferred}");
    let compensation = deferred["operation"].as_str().unwrap().to_owned();
    let page = call(&root, &data, &["candidates", "--limit", "1"]);
    let listed = &page["result"]["candidates"][0];
    assert_eq!(listed["operation"], compensation.as_str());
    assert_eq!(listed["undo_of"], original.as_str());
    assert_eq!(listed["writes"][0]["id"], note.as_str());
    assert_eq!(listed["writes"][0]["version"], 1);
    assert_eq!(page["page"]["shown"], 1);
    let info = ok(&root, &data, &["operation", "--id", &compensation]);
    assert_eq!(info["state"], "prepared");
    assert_eq!(info["receipt"], Value::Null);
    assert_eq!(info["undo_of"], original.as_str());
    assert_eq!(
        lookup(&root, &data, "undo", "u1")["result"]["state"],
        "prepared"
    );
    ok(
        &root,
        &data,
        &[
            "discard",
            "--session",
            &session,
            "--generation",
            "0",
            "--token-file",
            &token,
        ],
    );
    let applied = call(&root, &data, &undo);
    assert_eq!(applied["outcome"], "Applied", "{applied}");
    assert_eq!(applied["operation"], compensation.as_str());
    let info = ok(&root, &data, &["operation", "--id", &compensation]);
    assert_eq!(info["state"], "applied");
    assert!(info["receipt"]["committed_at"].is_string());
    let empty = ok(&root, &data, &["candidates", "--limit", "10"]);
    assert_eq!(empty["candidates"], json!([]));
}

#[test]
fn operation_lookup_refuses_bad_forms_and_never_allocates() {
    let root = Root::new();
    let absent = root.join("absent");
    let cases: &[(&[&str], &str, &str)] = &[
        (
            &[
                "operation",
                "--id",
                "x",
                "--key",
                "k",
                "--command",
                "note-create",
            ],
            "conflicting_options",
            "",
        ),
        (&["operation", "--key", "k"], "missing_option", "command"),
        (
            &["operation", "--command", "note-create"],
            "missing_option",
            "key",
        ),
        (&["operation"], "missing_option", "id"),
        (
            &["operation", "--key", "k", "--command", "notes"],
            "invalid_value",
            "command",
        ),
        (
            &["operation", "--key", "bad key", "--command", "note-create"],
            "invalid_value",
            "key",
        ),
    ];
    for (argv, code, option) in cases {
        let envelope = call(&root, &absent, argv);
        assert_error(&envelope, "usage", code);
        if !option.is_empty() {
            assert_eq!(envelope["error"]["details"]["option"], *option, "{argv:?}");
        }
    }
    assert!(!absent.exists());
    let data = root.workspace("data");
    assert_error(
        &call(&root, &data, &["operation", "--id", "no-such-op"]),
        "rejected",
        "not_found",
    );
    let unused = lookup(&root, &data, "note-create", "fresh");
    assert_error(&unused, "rejected", "not_found");
    // Looking a key up did not use it: the keyed write still creates normally.
    let first = call(
        &root,
        &data,
        &[
            "note-create",
            "--title",
            "T",
            "--text",
            "x",
            "--key",
            "fresh",
        ],
    );
    assert_eq!(first["changed"][0]["created"], true, "{first}");
    assert_eq!(
        lookup(&root, &data, "note-create", "fresh")["result"]["operation"],
        first["operation"]
    );
}
