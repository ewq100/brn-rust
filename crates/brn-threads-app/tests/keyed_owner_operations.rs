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
    let (deferred_op, deferred) = app
        .undo_with_key("u1", created.operation.as_str())
        .unwrap();
    assert!(matches!(deferred, ApplyOutcome::Deferred { .. }), "{deferred:?}");
    app.store.discard(&session.id, session.generation).unwrap();
    let (applied_op, applied) = app
        .undo_with_key("u1", created.operation.as_str())
        .unwrap();
    assert_eq!(applied_op, deferred_op);
    let ApplyOutcome::Applied(receipt) = applied else {
        panic!("{applied:?}");
    };
    assert_eq!(receipt.operation, deferred_op);
    let (replayed_op, replayed) = app
        .undo_with_key("u1", created.operation.as_str())
        .unwrap();
    assert_eq!(replayed_op, deferred_op);
    assert_eq!(replayed, ApplyOutcome::Applied(receipt));
    assert!(
        !app.candidates()
            .unwrap()
            .iter()
            .any(|c| c.operation == deferred_op && app.store.receipt(&c.operation).unwrap().is_none())
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
