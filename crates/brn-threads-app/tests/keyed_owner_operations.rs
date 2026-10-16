//! Caller-keyed owner creation and Undo replay instead of duplicating.
use brn_threads_app::*;

fn open() -> (tempfile::TempDir, Workspace) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let app = Workspace::open(dir.path().join("data")).unwrap();
    (dir, app)
}

#[test]
fn keyed_note_and_thread_creation_replay_and_refuse_other_content() {
    let (_dir, mut app) = open();
    let note = app.new_note_with_key("n1", "Plan", "Body\n").unwrap();
    assert_eq!(app.new_note_with_key("n1", "Plan", "Body\n").unwrap(), note);
    assert_eq!(app.notes().unwrap().len(), 1);
    assert!(matches!(
        app.new_note_with_key("n1", "Plan", "Other"),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    assert_eq!(app.notes().unwrap().len(), 1);

    let thread = app.new_thread_with_key("t1", "Chat").unwrap();
    assert_eq!(app.new_thread_with_key("t1", "Chat").unwrap(), thread);
    assert_eq!(app.threads().unwrap().len(), 1);
    assert!(matches!(
        app.new_thread_with_key("t1", "Other chat"),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    assert_eq!(app.threads().unwrap().len(), 1);
}

#[test]
fn keyed_undo_retries_its_deferred_compensation_and_then_replays() {
    let (_dir, mut app) = open();
    let created = app.new_note_with_key("n1", "Plan", "Body").unwrap();
    let note = &created.writes[0].after;
    let session = app.store.begin_edit(&note.id, note.version).unwrap();
    let (deferred_op, deferred) = app.undo_with_key("u1", created.operation.as_str()).unwrap();
    assert!(
        matches!(deferred, ApplyOutcome::Deferred { .. }),
        "{deferred:?}"
    );
    app.store.discard(&session.id, session.generation).unwrap();
    let (applied_op, applied) = app.undo_with_key("u1", created.operation.as_str()).unwrap();
    assert_eq!(applied_op, deferred_op);
    let ApplyOutcome::Applied(receipt) = applied else {
        panic!("{applied:?}");
    };
    assert_eq!(receipt.operation, deferred_op);
    let (replayed_op, replayed) = app.undo_with_key("u1", created.operation.as_str()).unwrap();
    assert_eq!(replayed_op, deferred_op);
    assert_eq!(replayed, ApplyOutcome::Applied(receipt));
    // Candidates are unapplied operations only.
    assert!(
        !app.candidates()
            .unwrap()
            .iter()
            .any(|c| c.operation == deferred_op)
    );
}

#[test]
fn unkeyed_creation_and_undo_stay_distinct_per_call() {
    let (_dir, mut app) = open();
    let first = app.new_note("Plan", "Body").unwrap();
    let second = app.new_note("Plan", "Body").unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(app.notes().unwrap().len(), 2);
    let a = app.new_thread("Chat").unwrap();
    let b = app.new_thread("Chat").unwrap();
    assert_ne!(a.id, b.id);
    let created = app.new_note("Undo me", "Body").unwrap();
    let operation = app.history().unwrap().last().unwrap().operation.clone();
    assert_eq!(created.id, operation.creation_id(0));
    assert!(matches!(
        app.undo(operation.as_str()).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    // A replay would return the same receipt; a fresh compensation finds the note changed.
    assert!(matches!(
        app.undo(operation.as_str()).unwrap(),
        ApplyOutcome::Stale { .. }
    ));
}

fn converted(
    markdown: &str,
    source_bytes: &[u8],
    reference: &str,
) -> brn_threads_intake::ConversionResult {
    use sha2::{Digest, Sha256};
    brn_threads_intake::ConversionResult {
        intent: brn_threads_intake::ImportIntent::FullNote,
        markdown: markdown.into(),
        assets: vec![],
        source_sha256: Sha256::digest(source_bytes).into(),
        source_reference: reference.into(),
        converter: "synthetic".into(),
        sources: vec![],
        coverage: brn_threads_intake::Coverage {
            status: brn_threads_intake::CoverageStatus::Complete,
            checked_items: vec![],
            gaps: vec![],
        },
    }
}
#[test]
fn keyed_topics_create_converges() {
    let (_dir, mut app) = open();
    let (first, outcome) = app.create_topics_note_with_key("k1").unwrap();
    assert!(matches!(outcome, ApplyOutcome::Applied(_)), "{outcome:?}");
    let (again, outcome) = app.create_topics_note_with_key("k1").unwrap();
    assert!(matches!(outcome, ApplyOutcome::Applied(_)), "{outcome:?}");
    assert_eq!(again.id, first.id);
    assert_eq!(app.notes().unwrap().len(), 1);
    assert_eq!(app.topics().unwrap().unwrap().note, first.id);
}
#[test]
fn keyed_seed_chat_replays_first_findings() {
    let (_dir, mut app) = open();
    let check = app.knowledge_check().unwrap();
    let (chat, message) = app.seed_knowledge_review_with_key("k1", &check).unwrap();
    app.new_note("Later", "A later note changes the findings.")
        .unwrap();
    let later = app.knowledge_check().unwrap();
    assert_ne!(later.notes, check.notes);
    let (again, replayed) = app.seed_knowledge_review_with_key("k1", &later).unwrap();
    assert_eq!((again, replayed.clone()), (chat.clone(), message.clone()));
    assert_eq!(app.threads().unwrap().len(), 1);
    let receipt = app.store.receipt(&message).unwrap().unwrap();
    let RecordData::Message(text) = &receipt.writes[0].after.data else {
        panic!()
    };
    assert_eq!(text.role, "owner");
    assert!(text.text.contains("over 0 current notes"), "{}", text.text);
    assert_eq!(receipt.authority, "owner:owner");
}
#[test]
fn keyed_full_import_replay_does_not_bypass_an_archived_chat() {
    let (_dir, mut app) = open();
    let chat = app.new_thread("Chat").unwrap().id;
    app.archive_chat("archive", &chat).unwrap();
    let doc = converted("# Doc\n", b"# Doc\n", "/abs/doc.md");
    let refused = app.attach_full("cli-key:intake:k1", &chat, "Doc", &doc);
    assert!(
        matches!(&refused, Err(AppError::Invalid(m)) if m.starts_with("Choose an active chat")),
        "{refused:?}"
    );
    // The key is allocated but nothing committed: nothing to replay, so the
    // normal path runs and refuses the archived chat again.
    let replay = app
        .full_import_replay(
            "cli-key:intake:k1",
            "Doc",
            "/abs/doc.md",
            b"# Doc\n",
            Some(&chat),
        )
        .unwrap();
    assert!(replay.is_none());
    let again = app.attach_full("cli-key:intake:k1", &chat, "Doc", &doc);
    assert!(
        matches!(&again, Err(AppError::Invalid(m)) if m.starts_with("Choose an active chat")),
        "{again:?}"
    );
}
#[test]
fn keyed_full_import_replay_detects_a_changed_title() {
    let (_dir, mut app) = open();
    let doc = converted("# Doc\n", b"# Doc\n", "/abs/doc.md");
    let ApplyOutcome::Applied(first) = app.import_full("cli-key:intake:k1", "Doc", &doc).unwrap()
    else {
        panic!()
    };
    let replay = app
        .full_import_replay("cli-key:intake:k1", "Doc", "/abs/doc.md", b"# Doc\n", None)
        .unwrap();
    assert_eq!(replay, Some(ApplyOutcome::Applied(first.clone())));
    for (title, reference, scope) in [
        ("Renamed", "/abs/doc.md", None),
        ("Doc", "/abs/other.md", None),
        ("Doc", "/abs/doc.md", Some("some-chat")),
    ] {
        assert!(matches!(
            app.full_import_replay("cli-key:intake:k1", title, reference, b"# Doc\n", scope),
            Err(AppError::Core(Error::ImmutableRequest))
        ));
    }
    assert!(
        app.full_import_replay(
            "cli-key:intake:unused",
            "Doc",
            "/abs/doc.md",
            b"# Doc\n",
            None
        )
        .unwrap()
        .is_none()
    );
}
#[test]
fn keyed_full_import_replay_matches_html_source_bytes() {
    let (_dir, mut app) = open();
    let html = b"<html><body><h1>Doc</h1></body></html>";
    // Conversion output differs from the source bytes; the replay matches the bytes.
    let doc = converted("# Doc\n\nConverted wording.\n", html, "/abs/doc.html");
    let ApplyOutcome::Applied(first) = app.import_full("cli-key:intake:h1", "Doc", &doc).unwrap()
    else {
        panic!()
    };
    let replay = app
        .full_import_replay("cli-key:intake:h1", "Doc", "/abs/doc.html", html, None)
        .unwrap();
    assert_eq!(replay, Some(ApplyOutcome::Applied(first)));
    assert!(matches!(
        app.full_import_replay(
            "cli-key:intake:h1",
            "Doc",
            "/abs/doc.html",
            b"<html>changed</html>",
            None
        ),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    assert_eq!(app.notes().unwrap().len(), 1);
}
#[test]
fn keyed_full_import_replay_after_reimport_is_none() {
    let (_dir, mut app) = open();
    let doc = converted("# Doc\n", b"# Doc\n", "/abs/doc.md");
    assert!(matches!(
        app.import_full("cli-key:intake:a", "Doc", &doc).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    // The same bytes under another key only observe the earlier import.
    assert!(matches!(
        app.import_full("cli-key:intake:b", "Doc", &doc).unwrap(),
        ApplyOutcome::Applied(_)
    ));
    assert_eq!(app.notes().unwrap().len(), 1);
    let replay = app
        .full_import_replay("cli-key:intake:b", "Doc", "/abs/doc.md", b"# Doc\n", None)
        .unwrap();
    assert!(replay.is_none());
}
