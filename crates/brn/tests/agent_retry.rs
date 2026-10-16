//! Process checks of the round-2 retry identities: keyed `topics-create`,
//! keyed `knowledge-check --seed-chat yes`, keyed intake that replays before
//! conversion, and key reuse on a different original.
mod support;
use serde_json::{json, Value};
use support::*;

fn created(envelope: &Value, kind: &str) -> String {
    envelope["changed"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["kind"] == kind && c["created"] == true)
        .unwrap_or_else(|| panic!("no created {kind}: {envelope}"))["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn keyed_topics_create_retries_without_a_second_note() {
    let root = Root::new();
    let data = root.workspace("data");
    let first = call(&root, &data, &["topics-create", "--key", "t1"]);
    assert_eq!(first["status"], "ok", "{first}");
    assert_eq!(first["retry"], "converge");
    let note = first["result"]["note"]["id"].clone();
    let again = call(&root, &data, &["topics-create", "--key", "t1"]);
    assert_eq!(again["status"], "ok", "{again}");
    assert_eq!(again["result"]["note"]["id"], note);
    assert_eq!(ok(&root, &data, &["notes"]).as_array().unwrap().len(), 1);
    assert_eq!(ok(&root, &data, &["topics"])["note"], note);
    // Without a key a retry is a second Topics note.
    let unkeyed = call(&root, &data, &["topics-create"]);
    assert_eq!(unkeyed["retry"], "duplicate");
    assert_eq!(ok(&root, &data, &["notes"]).as_array().unwrap().len(), 2);
}

#[test]
fn keyed_seed_chat_replays_and_ignores_later_findings() {
    let root = Root::new();
    let data = root.workspace("data");
    let argv = ["knowledge-check", "--seed-chat", "yes", "--key", "s1"];
    let first = call(&root, &data, &argv);
    assert_eq!(first["status"], "ok", "{first}");
    assert_eq!(first["retry"], "replay");
    let message = first["operation"].as_str().unwrap().to_owned();
    let chat = first["result"]["review_chat"].clone();
    note(&root, &data, "Later", "A later note changes the findings.");
    let again = call(&root, &data, &argv);
    assert_eq!(again["operation"], message.as_str());
    assert_eq!(again["result"]["review_chat"], chat);
    assert_ne!(again["result"]["notes"], first["result"]["notes"]);
    assert_eq!(ok(&root, &data, &["threads"]).as_array().unwrap().len(), 1);
    let found = ok(
        &root,
        &data,
        &["operation", "--key", "s1", "--command", "knowledge-check"],
    );
    assert_eq!(found["operation"], message.as_str());
    assert_eq!(found["related"].as_array().unwrap().len(), 1);
    let chat_op = found["related"][0].as_str().unwrap();
    let chat_info = ok(&root, &data, &["operation", "--id", chat_op]);
    assert_eq!(chat_info["writes"][0]["id"], chat);
    // Unkeyed seeding still makes another chat.
    let unkeyed = call(&root, &data, &["knowledge-check", "--seed-chat", "yes"]);
    assert_eq!(unkeyed["retry"], "duplicate");
    assert_eq!(ok(&root, &data, &["threads"]).as_array().unwrap().len(), 2);
}

#[test]
fn knowledge_check_key_requires_seed_chat() {
    let root = Root::new();
    let data = root.join("absent");
    let envelope = call(&root, &data, &["knowledge-check", "--key", "k1"]);
    assert_error(&envelope, "usage", "conflicting_options");
    assert_eq!(
        envelope["error"]["details"]["options"],
        json!(["key", "seed-chat"])
    );
    let default = run(&root, false, &data, &["knowledge-check", "--key", "k1"]);
    assert_eq!(default.code, 1);
    assert_eq!(default.stderr, "--key needs --seed-chat yes\n");
    assert!(!data.exists());
}

#[cfg(unix)]
#[test]
fn keyed_intake_retry_replays_before_conversion_through_a_symlinked_folder() {
    let root = Root::new();
    let data = root.workspace("data");
    std::fs::create_dir(root.join("real")).unwrap();
    std::os::unix::fs::symlink(root.join("real"), root.join("link")).unwrap();
    std::fs::write(root.join("real/doc.md"), "# Doc\n\nBody “β”.\n").unwrap();
    let source = root.join("link/doc.md");
    let source = source.to_str().unwrap();
    let argv = [
        "intake", "--source", source, "--intent", "full", "--title", "Doc", "--key", "i1",
    ];
    let first = run(&root, true, &data, &argv);
    let envelope = first.envelope();
    assert_eq!(envelope["outcome"], "Applied", "{envelope}");
    assert!(!first.stderr.is_empty(), "the first import converts");
    let retry = run(&root, true, &data, &argv);
    let replayed = retry.envelope();
    assert_eq!(replayed["operation"], envelope["operation"]);
    assert_eq!(replayed["retry"], "replay");
    assert_eq!(retry.stderr, "", "the replay printed conversion progress");
    let renamed = [
        "intake", "--source", source, "--intent", "full", "--title", "Other", "--key", "i1",
    ];
    let conflict = run(&root, true, &data, &renamed);
    assert_eq!(conflict.stderr, "", "the refusal converted first");
    let refused = conflict.envelope();
    assert_error(&refused, "rejected", "key_conflict");
    assert_eq!(refused["error"]["details"]["key"], "i1");
    assert_eq!(ok(&root, &data, &["notes"]).as_array().unwrap().len(), 1);
    // A chat attachment under a key replays the same way.
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let attach = [
        "intake", "--source", source, "--intent", "chat", "--thread", &chat, "--key", "c1",
    ];
    let first = call(&root, &data, &attach);
    assert_eq!(first["outcome"], "Applied", "{first}");
    let again = run(&root, true, &data, &attach);
    assert_eq!(again.envelope()["operation"], first["operation"]);
    assert_eq!(again.stderr, "");
}

#[test]
fn keyed_undo_with_a_different_original_is_key_conflict() {
    let root = Root::new();
    let data = root.workspace("data");
    let a = call(
        &root,
        &data,
        &["note-create", "--title", "A", "--text", "a"],
    );
    let b = call(
        &root,
        &data,
        &["note-create", "--title", "B", "--text", "b"],
    );
    let undo_a = [
        "undo",
        "--operation",
        a["operation"].as_str().unwrap(),
        "--key",
        "u1",
    ];
    assert_eq!(call(&root, &data, &undo_a)["outcome"], "Applied");
    let before = ok(&root, &data, &["history"]);
    let undo_b = [
        "undo",
        "--operation",
        b["operation"].as_str().unwrap(),
        "--key",
        "u1",
    ];
    let refused = call(&root, &data, &undo_b);
    assert_error(&refused, "rejected", "key_conflict");
    assert_eq!(refused["exit"], 6);
    assert_eq!(refused["error"]["details"]["key"], "u1");
    assert_eq!(ok(&root, &data, &["history"]), before);
}

#[test]
fn keyed_keep_in_brn_with_a_different_note_is_key_conflict() {
    let root = Root::new();
    let data = root.workspace("data");
    let chat = id(&ok(&root, &data, &["thread-start", "--title", "Chat"]));
    let attach = |name: &str| {
        let source = root.file(name, format!("# {name}\n").as_bytes());
        let envelope = call(
            &root,
            &data,
            &[
                "intake", "--source", &source, "--intent", "chat", "--thread", &chat,
            ],
        );
        created(&envelope, "Note")
    };
    let (first, second) = (attach("one.md"), attach("two.md"));
    let keep = |note: &str| {
        call(
            &root,
            &data,
            &["keep-in-brn", "--note", note, "--base", "1", "--key", "k1"],
        )
    };
    assert_eq!(keep(&first)["outcome"], "Applied");
    let before = ok(&root, &data, &["history"]);
    let refused = keep(&second);
    assert_error(&refused, "rejected", "key_conflict");
    assert_eq!(refused["error"]["details"]["key"], "k1");
    assert_eq!(ok(&root, &data, &["history"]), before);
}

/// PR #156 LOW 1: a keyed full intake retried under another spelling of the
/// same file (a symlinked parent; on macOS also `/tmp` for `/private/tmp`)
/// replays: the binding compares canonical locators. Same operation, no
/// conversion, no `key_conflict`, one note.
#[cfg(unix)]
#[test]
fn keyed_full_intake_retry_with_another_spelling_replays() {
    let root = Root::new();
    let data = root.workspace("data");
    std::fs::create_dir(root.join("real")).unwrap();
    std::os::unix::fs::symlink(root.join("real"), root.join("link")).unwrap();
    std::fs::write(root.join("real/doc.md"), "# Doc\n\nSpelled twice.\n").unwrap();
    let real = root.join("real/doc.md");
    let link = root.join("link/doc.md");
    let intake = |source: &str, key: &str| {
        run(
            &root,
            true,
            &data,
            &[
                "intake", "--source", source, "--intent", "full", "--title", "Doc", "--key", key,
            ],
        )
    };
    let first = intake(real.to_str().unwrap(), "s1");
    let applied = first.envelope();
    assert_eq!(applied["outcome"], "Applied", "{applied}");
    let retry = intake(link.to_str().unwrap(), "s1");
    let replayed = retry.envelope();
    assert_eq!(replayed["status"], "ok", "{replayed}");
    assert_eq!(replayed["operation"], applied["operation"]);
    assert_eq!(replayed["retry"], "replay");
    assert_eq!(retry.stderr, "", "the retry converted again");
    assert_eq!(ok(&root, &data, &["notes"]).as_array().unwrap().len(), 1);
    // `/tmp` and `/private/tmp` name the same file on macOS.
    let private = std::path::Path::new("/private/tmp");
    let aliased = std::fs::canonicalize("/tmp").is_ok_and(|tmp| tmp == private);
    if !aliased {
        eprintln!("skipped: /tmp is not /private/tmp here");
        return;
    }
    let scratch = tempfile::Builder::new()
        .prefix("brn-r3-locator-")
        .tempdir_in(private)
        .unwrap();
    let file = scratch.path().join("tmp-doc.md");
    std::fs::write(&file, "# Tmp doc\n\nSpelled via /tmp.\n").unwrap();
    let private_spelling = file.to_str().unwrap().to_owned();
    let tmp_spelling = private_spelling.replacen("/private/tmp/", "/tmp/", 1);
    let first = intake(&private_spelling, "s2");
    let applied = first.envelope();
    assert_eq!(applied["outcome"], "Applied", "{applied}");
    let retry = intake(&tmp_spelling, "s2");
    let replayed = retry.envelope();
    assert_eq!(replayed["operation"], applied["operation"], "{replayed}");
    assert_eq!(replayed["retry"], "replay");
    assert_eq!(retry.stderr, "");
    assert_eq!(ok(&root, &data, &["notes"]).as_array().unwrap().len(), 2);
}
