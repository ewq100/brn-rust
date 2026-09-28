use brn_store::{Approval, BeginOperation, OperationStatus, Store};
use uuid::Uuid;

#[test]
fn imports_keep_stable_source_and_immutable_version_history() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let first_op = Uuid::new_v4();
    let first = store
        .import_text(first_op, "file:a.md", "A", b"one", Approval::Approved)
        .unwrap();
    assert!(first.changed);
    assert_eq!(
        store
            .import_text(first_op, "file:a.md", "A", b"one", Approval::Approved)
            .unwrap(),
        first
    );
    assert!(
        store
            .import_text(first_op, "file:a.md", "A", b"two", Approval::Approved)
            .is_err()
    );
    assert!(
        store
            .import_text(first_op, "file:a.md", "A", b"one", Approval::Draft)
            .is_err()
    );
    let same = store
        .import_text(Uuid::new_v4(), "file:a.md", "A", b"one", Approval::Draft)
        .unwrap();
    assert_eq!(same.source_id, first.source_id);
    assert_eq!(same.version_id, first.version_id);
    assert!(!same.changed);
    let changed = store
        .import_text(Uuid::new_v4(), "file:a.md", "A2", b"two", Approval::Draft)
        .unwrap();
    assert_eq!(changed.source_id, first.source_id);
    assert_ne!(changed.version_id, first.version_id);
    let reverted = store
        .import_text(Uuid::new_v4(), "file:a.md", "A2", b"one", Approval::Draft)
        .unwrap();
    assert_ne!(reverted.version_id, first.version_id);
    assert_eq!(store.versions(first.source_id).unwrap().len(), 3);
    let doc = &store.documents().unwrap()[0];
    assert_eq!(doc.version_id, reverted.version_id);
    assert_eq!(doc.title, "A2");
    assert_eq!(doc.approval, Approval::Draft);
    assert!(
        store
            .set_approval(
                Uuid::new_v4(),
                first.source_id,
                first.version_id,
                Approval::Withdrawn
            )
            .is_err()
    );
    let approval_op = Uuid::new_v4();
    store
        .set_approval(
            approval_op,
            first.source_id,
            reverted.version_id,
            Approval::Withdrawn,
        )
        .unwrap();
    store
        .set_approval(
            approval_op,
            first.source_id,
            reverted.version_id,
            Approval::Withdrawn,
        )
        .unwrap();
    assert!(
        store
            .set_approval(
                approval_op,
                first.source_id,
                reverted.version_id,
                Approval::Draft
            )
            .is_err()
    );
    assert_eq!(store.documents().unwrap()[0].approval, Approval::Withdrawn);
    drop(store);
    let (store, _) = Store::open(dir.path()).unwrap();
    assert_eq!(
        store.documents().unwrap()[0].version_id,
        reverted.version_id
    );
}

#[test]
fn thread_binding_and_chat_projection_survive_restart_without_replay() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let session = store
        .create_session(
            Uuid::new_v4(),
            "codex",
            "store-a",
            Some("acct"),
            None,
            b"{}",
        )
        .unwrap();
    let attach_op = Uuid::new_v4();
    store
        .attach_thread(attach_op, session, "provider-thread")
        .unwrap();
    store
        .attach_thread(attach_op, session, "provider-thread")
        .unwrap();
    assert!(
        store
            .attach_thread(Uuid::new_v4(), session, "different")
            .is_err()
    );
    let op = Uuid::new_v4();
    assert_eq!(
        store
            .prepare_turn(op, session, "Question?", "hybrid", "[]")
            .unwrap(),
        BeginOperation::New
    );
    assert_eq!(
        store
            .prepare_turn(op, session, "Question?", "hybrid", "[]")
            .unwrap(),
        BeginOperation::Existing
    );
    assert!(
        store
            .prepare_turn(op, session, "Changed?", "hybrid", "[]")
            .is_err()
    );
    assert_eq!(store.messages(session).unwrap().len(), 1);
    store.record_turn_started(op, "provider-turn").unwrap();
    store.record_turn_started(op, "provider-turn").unwrap();
    assert!(store.record_turn_started(op, "other-turn").is_err());
    drop(store);
    let (mut store, report) = Store::open(dir.path()).unwrap();
    assert_eq!(report.interrupted_operations, 1);
    assert_eq!(
        store
            .session(session)
            .unwrap()
            .unwrap()
            .thread_id
            .as_deref(),
        Some("provider-thread")
    );
    let turn = &store.turns(session).unwrap()[0];
    assert_eq!(turn.status, OperationStatus::Interrupted);
    assert_eq!(turn.provider_turn_id.as_deref(), Some("provider-turn"));
    assert_eq!(
        store
            .prepare_turn(op, session, "Question?", "hybrid", "[]")
            .unwrap(),
        BeginOperation::Existing
    );
    assert!(
        store
            .complete_turn(op, OperationStatus::Completed, "late answer", None)
            .is_err()
    );
    assert_eq!(store.messages(session).unwrap().len(), 1);
    let second = Uuid::new_v4();
    store
        .prepare_turn(second, session, "Again?", "keyword", "[1]")
        .unwrap();
    store
        .complete_turn(
            second,
            OperationStatus::Completed,
            "Answer",
            Some("{\"tokens\":1}"),
        )
        .unwrap();
    store
        .complete_turn(
            second,
            OperationStatus::Completed,
            "Answer",
            Some("{\"tokens\":1}"),
        )
        .unwrap();
    assert!(
        store
            .complete_turn(second, OperationStatus::Completed, "changed", None)
            .is_err()
    );
    assert_eq!(store.messages(session).unwrap().len(), 3);
    assert_eq!(
        store.turns(session).unwrap()[1].answer.as_deref(),
        Some("Answer")
    );
    assert!(
        store
            .finish_operation(second, OperationStatus::Failed, b"bypass")
            .is_err()
    );
    drop(store);
    let (store, report) = Store::open(dir.path()).unwrap();
    assert_eq!(report.interrupted_operations, 0);
    assert_eq!(store.turns(session).unwrap().len(), 2);
    assert_eq!(store.messages(session).unwrap().len(), 3);
}

#[test]
fn prepare_requires_attached_thread_and_one_active_turn() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, _) = Store::open(dir.path()).unwrap();
    let session = store
        .create_session(Uuid::new_v4(), "codex", "store-a", None, None, b"{}")
        .unwrap();
    let first = Uuid::new_v4();
    assert!(
        store
            .prepare_turn(first, session, "first", "keyword", "[]")
            .is_err()
    );
    assert!(store.operation(first).unwrap().is_none());
    assert!(store.messages(session).unwrap().is_empty());
    store
        .attach_thread(Uuid::new_v4(), session, "thread-1")
        .unwrap();
    assert_eq!(
        store
            .prepare_turn(first, session, "first", "keyword", "[]")
            .unwrap(),
        BeginOperation::New
    );
    assert_eq!(
        store
            .prepare_turn(first, session, "first", "keyword", "[]")
            .unwrap(),
        BeginOperation::Existing
    );
    let second = Uuid::new_v4();
    assert!(
        store
            .prepare_turn(second, session, "second", "keyword", "[]")
            .is_err()
    );
    assert!(store.operation(second).unwrap().is_none());
    assert_eq!(store.messages(session).unwrap().len(), 1);
    store
        .complete_turn(first, OperationStatus::Failed, "provider error", None)
        .unwrap();
    assert_eq!(
        store
            .prepare_turn(second, session, "second", "keyword", "[]")
            .unwrap(),
        BeginOperation::New
    );
    assert_eq!(store.messages(session).unwrap().len(), 2);
}
