use brn_store::{DraftStamp, MAX_DRAFT_BYTES, OperationStatus, RevisionKind, Store};
use uuid::Uuid;

fn open() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = Store::open(dir.path()).unwrap();
    (dir, store)
}

#[test]
fn draft_working_copy_and_checkpoints_survive_reopen() {
    let (dir, mut store) = open();
    let initial = "α\r\nlast\n";
    let draft = store
        .create_draft(Uuid::new_v4(), " Notes ", initial)
        .unwrap();
    assert_eq!(draft.text, initial);
    assert_eq!(draft.stamp.generation, 0);
    let root = store
        .draft_revision(draft.stamp.base_revision)
        .unwrap()
        .unwrap();
    assert_eq!(
        (root.kind, root.parent_id, root.text.as_str()),
        (RevisionKind::Checkpoint, None, initial)
    );
    let saved = store
        .save_draft(Uuid::new_v4(), draft.id, draft.stamp, 1, "α\r\nlast")
        .unwrap();
    let checked = store
        .checkpoint_draft(Uuid::new_v4(), draft.id, saved.stamp, 1, "α\r\nlast")
        .unwrap();
    assert_eq!(checked.stamp.generation, 1);
    assert_ne!(checked.stamp.base_revision, root.id);
    drop(store);
    let (store, _) = Store::open(dir.path()).unwrap();
    assert_eq!(store.draft(draft.id).unwrap().unwrap(), checked);
    assert_eq!(store.drafts().unwrap(), vec![checked]);
    let revisions = store.draft_revisions(draft.id).unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0], root);
    assert_eq!(revisions[1].parent_id, Some(root.id));
    assert_eq!(revisions[1].text, "α\r\nlast");
}

#[test]
fn stale_stamp_is_rejected_without_changes() {
    let (_dir, mut store) = open();
    let d = store.create_draft(Uuid::new_v4(), "title", "one").unwrap();
    let saved = store
        .save_draft(Uuid::new_v4(), d.id, d.stamp, 1, "two")
        .unwrap();
    let op = Uuid::new_v4();
    assert!(store.save_draft(op, d.id, d.stamp, 2, "stale").is_err());
    assert!(store.operation(op).unwrap().is_none());
    assert_eq!(store.draft(d.id).unwrap().unwrap(), saved);
    assert_eq!(store.draft_revisions(d.id).unwrap().len(), 1);
}

#[test]
fn retry_returns_original_result_after_newer_save() {
    let (_dir, mut store) = open();
    let d = store.create_draft(Uuid::new_v4(), "title", "").unwrap();
    let op = Uuid::new_v4();
    let first = store.save_draft(op, d.id, d.stamp, 1, "first").unwrap();
    let second = store
        .save_draft(Uuid::new_v4(), d.id, first.stamp, 2, "second")
        .unwrap();
    assert_eq!(
        store.save_draft(op, d.id, d.stamp, 1, "first").unwrap(),
        first
    );
    assert_eq!(store.draft(d.id).unwrap().unwrap(), second);
}

#[test]
fn conflicting_operation_reuse_is_rejected() {
    let (_dir, mut store) = open();
    let op = Uuid::new_v4();
    let d = store.create_draft(op, "title", "a").unwrap();
    assert!(store.create_draft(op, "title", "b").is_err());
    assert!(store.save_draft(op, d.id, d.stamp, 1, "b").is_err());
    assert_eq!(store.draft(d.id).unwrap().unwrap(), d);
}

#[test]
fn candidate_preserves_working_copy_and_requires_completed_turn() {
    let (_dir, mut store) = open();
    let d = store.create_draft(Uuid::new_v4(), "title", "own").unwrap();
    let session = store
        .create_session(
            Uuid::new_v4(),
            "codex",
            "local",
            None,
            Some("thread"),
            b"{}",
        )
        .unwrap();
    let turn = Uuid::new_v4();
    store
        .prepare_turn(turn, session, "question", "grounded", "[]")
        .unwrap();
    assert!(
        store
            .candidate_from_turn(Uuid::new_v4(), d.id, d.stamp.base_revision, turn)
            .is_err()
    );
    store
        .complete_turn(turn, OperationStatus::Completed, "answer\n", None)
        .unwrap();
    let op = Uuid::new_v4();
    let candidate = store
        .candidate_from_turn(op, d.id, d.stamp.base_revision, turn)
        .unwrap();
    assert_eq!(candidate.kind, RevisionKind::Candidate);
    assert_eq!(candidate.origin_turn, Some(turn));
    assert_eq!(candidate.text, "answer\n");
    assert_eq!(candidate.parent_id, Some(d.stamp.base_revision));
    assert_eq!(
        store
            .candidate_from_turn(op, d.id, d.stamp.base_revision, turn)
            .unwrap(),
        candidate
    );
    assert_eq!(store.draft(d.id).unwrap().unwrap(), d);
}

#[test]
fn boundary_validation_and_wrong_draft_parent_are_atomic() {
    let (_dir, mut store) = open();
    assert!(store.create_draft(Uuid::new_v4(), "  ", "x").is_err());
    let d = store.create_draft(Uuid::new_v4(), "title", "").unwrap();
    let other = store.create_draft(Uuid::new_v4(), "other", "x").unwrap();
    let op = Uuid::new_v4();
    assert!(
        store
            .save_draft(op, d.id, d.stamp, 1, &"x".repeat(MAX_DRAFT_BYTES + 1))
            .is_err()
    );
    assert!(store.operation(op).unwrap().is_none());
    assert!(
        store
            .save_draft(Uuid::new_v4(), d.id, d.stamp, 0, "x")
            .is_err()
    );
    let overflow_op = Uuid::new_v4();
    assert!(
        store
            .save_draft(overflow_op, d.id, d.stamp, u64::MAX, "x")
            .is_err()
    );
    assert!(store.operation(overflow_op).unwrap().is_none());
    assert!(
        store
            .save_draft(
                Uuid::new_v4(),
                d.id,
                DraftStamp {
                    base_revision: other.stamp.base_revision,
                    generation: 0
                },
                1,
                "x"
            )
            .is_err()
    );
    assert_eq!(store.draft(d.id).unwrap().unwrap(), d);
    assert_eq!(store.draft_revisions(d.id).unwrap().len(), 1);
}

#[test]
fn candidate_wrong_draft_parent_and_failed_turn_leave_no_result() {
    let (_dir, mut store) = open();
    let d = store.create_draft(Uuid::new_v4(), "a", "a").unwrap();
    let other = store.create_draft(Uuid::new_v4(), "b", "b").unwrap();
    let session = store
        .create_session(
            Uuid::new_v4(),
            "codex",
            "local",
            None,
            Some("thread"),
            b"{}",
        )
        .unwrap();
    let turn = Uuid::new_v4();
    store
        .prepare_turn(turn, session, "question", "grounded", "[]")
        .unwrap();
    store
        .complete_turn(turn, OperationStatus::Failed, "failed answer", None)
        .unwrap();
    let bad_parent = Uuid::new_v4();
    assert!(
        store
            .candidate_from_turn(bad_parent, d.id, other.stamp.base_revision, turn)
            .is_err()
    );
    assert!(store.operation(bad_parent).unwrap().is_none());
    let failed_turn = Uuid::new_v4();
    assert!(
        store
            .candidate_from_turn(failed_turn, d.id, d.stamp.base_revision, turn)
            .is_err()
    );
    assert!(store.operation(failed_turn).unwrap().is_none());
    assert_eq!(store.draft_revisions(d.id).unwrap().len(), 1);
}
