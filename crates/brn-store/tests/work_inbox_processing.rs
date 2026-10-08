use brn_store::{
    Error, WorkStore,
    work::{inbox::*, inbox_processing::*},
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;
fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn item(store: &mut WorkStore) -> InboxItem {
    store
        .capture_inbox(&InboxCapture {
            id: Uuid::new_v4(),
            kind: InboxKind::Text,
            title: "Synthetic original õ".into(),
            original_name: None,
            copy: InboxCopy {
                directory: "/synthetic/inbox".into(),
                directory_device: 1,
                directory_inode: 2,
                file_device: 1,
                file_inode: 3,
                byte_len: 4,
                sha256: digest(b"body"),
            },
        })
        .unwrap()
}
fn fixture() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn request(store: &mut WorkStore, count: usize) -> ProcessInboxRequest {
    ProcessInboxRequest {
        limits: None,

        id: Uuid::new_v4(),
        items: (0..count).map(|_| item(store)).collect(),
    }
}
fn converted() -> InboxProcessOutcome {
    InboxProcessOutcome::Converted {
        format: InboxConversionFormat::LiteralTextV1,
        byte_len: 17,
        sha256: digest(b"```text\nbody\n```\n"),
    }
}
#[test]
fn bounded_atomic_admission_exact_replay_and_per_member_immutable_outcomes() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let first = request(&mut store, 8);
    let second = request(&mut store, 8);
    let overflow = request(&mut store, 1);
    let initial = store.process_inbox(&first).unwrap();
    store.process_inbox(&second).unwrap();
    assert_eq!(store.process_inbox(&first).unwrap(), initial);
    assert!(matches!(
        store.process_inbox(&overflow),
        Err(Error::OperationConflict(_))
    ));
    assert!(store.inbox_processing(overflow.id).unwrap().is_none());
    let mut changed = first.clone();
    changed.items.swap(0, 1);
    assert!(matches!(
        store.process_inbox(&changed),
        Err(Error::OperationConflict(_))
    ));
    let mut stale = overflow.clone();
    stale.items[0].capture.title.push('x');
    assert!(matches!(
        store.process_inbox(&stale),
        Err(Error::OperationConflict(_))
    ));
    let mut duplicate = overflow.clone();
    duplicate.items.push(duplicate.items[0].clone());
    assert!(matches!(
        store.process_inbox(&duplicate),
        Err(Error::Invalid(_))
    ));
    assert!(
        store
            .finish_inbox_processing(first.id, 0, converted())
            .is_err()
    );
    store.start_inbox_processing(first.id, 0).unwrap();
    let done = store
        .finish_inbox_processing(first.id, 0, converted())
        .unwrap();
    assert_eq!(done.entries[0].outcome, converted());
    assert!(
        store
            .finish_inbox_processing(first.id, 0, InboxProcessOutcome::Cancelled)
            .is_err()
    );
    store.process_inbox(&overflow).unwrap();
    let cancelled = store.cancel_inbox_processing(first.id).unwrap();
    assert_eq!(cancelled.entries[0], done.entries[0]);
    assert!(
        cancelled.entries[1..]
            .iter()
            .all(|e| e.outcome == InboxProcessOutcome::Cancelled)
    );
    assert_eq!(store.process_inbox(&first).unwrap(), cancelled);
    assert_eq!(store.cancel_inbox_processing(first.id).unwrap(), cancelled);
}
#[test]
fn restart_interrupts_pending_members_preserves_completed_receipts_and_migrates_v12() {
    let data = fixture();
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let request = request(&mut store, 3);
    store.process_inbox(&request).unwrap();
    store.start_inbox_processing(request.id, 0).unwrap();
    let done = store
        .finish_inbox_processing(request.id, 0, converted())
        .unwrap();
    store.start_inbox_processing(request.id, 1).unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(data.path()).unwrap();
    let batch = store.inbox_processing(request.id).unwrap().unwrap();
    assert_eq!(batch.entries[0], done.entries[0]);
    assert!(
        batch.entries[1..]
            .iter()
            .all(|e| e.outcome == InboxProcessOutcome::Interrupted && e.finished_at_ms.is_some())
    );
    assert_eq!(store.process_inbox(&request).unwrap(), batch);
    assert!(store.next_inbox_processing().unwrap().is_none());
    let saved = request.items[0].clone();
    drop(store);
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    raw.execute_batch(
        "DROP TABLE ai_run_budgets; DROP TABLE intake_snapshots; DROP TABLE inbox_original_operations; DROP TABLE inbox_actions; DROP TABLE inbox_processing; PRAGMA user_version=12;",
    )
    .unwrap();
    drop(raw);
    let (store, _) = WorkStore::open(data.path()).unwrap();
    assert_eq!(store.inbox_item(saved.capture.id).unwrap(), Some(saved));
    assert!(store.inbox_processing(request.id).unwrap().is_none());
    let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
    assert_eq!(
        raw.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        17
    );
}
#[test]
fn readable_semantic_and_schema_damage_refuses_before_backup_or_recovery() {
    for mode in 0..4 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let request = request(&mut store, 1);
        let batch = store.process_inbox(&request).unwrap();
        drop(store);
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        match mode {
            0 => {
                let mut bad = batch.clone();
                bad.entries[0].outcome = converted();
                let bytes = serde_json::to_vec(&bad).unwrap();
                raw.execute(
                    "UPDATE inbox_processing SET record_json=?1,record_sha256=?2",
                    params![bytes, digest(&bytes).as_slice()],
                )
                .unwrap();
            }
            1 => {
                raw.execute("UPDATE inbox_processing SET pending=0", [])
                    .unwrap();
            }
            2 => {
                raw.execute_batch("DROP INDEX inbox_processing_queue;")
                    .unwrap();
            }
            _ => {
                raw.execute(
                    "UPDATE inbox_processing SET record_json=?1",
                    [vec![b'x'; 512 * 1024 + 1]],
                )
                .unwrap();
            }
        }
        drop(raw);
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        let bytes = std::fs::read(data.path().join("brn.sqlite")).unwrap();
        assert!(matches!(
            WorkStore::open(data.path()),
            Err(Error::Invalid(_))
        ));
        assert_eq!(
            std::fs::read(data.path().join("brn.sqlite")).unwrap(),
            bytes
        );
        assert_eq!(
            std::fs::read_dir(data.path().join("backups"))
                .unwrap()
                .count(),
            backups
        );
    }
}

#[test]
fn rehashed_markdown_receipt_cannot_fork_format_length_or_exact_original_hash() {
    for mode in 0..3 {
        let data = fixture();
        let (mut store, _) = WorkStore::open(data.path()).unwrap();
        let mut captured = item(&mut store).capture;
        captured.id = Uuid::new_v4();
        captured.kind = InboxKind::Markdown;
        let item = store.capture_inbox(&captured).unwrap();
        let request = ProcessInboxRequest {
            limits: None,

            id: Uuid::new_v4(),
            items: vec![item],
        };
        store.process_inbox(&request).unwrap();
        store.start_inbox_processing(request.id, 0).unwrap();
        let mut batch = store
            .finish_inbox_processing(
                request.id,
                0,
                InboxProcessOutcome::Converted {
                    format: InboxConversionFormat::VerbatimMarkdownV1,
                    byte_len: 4,
                    sha256: digest(b"body"),
                },
            )
            .unwrap();
        drop(store);
        let InboxProcessOutcome::Converted {
            format,
            byte_len,
            sha256,
        } = &mut batch.entries[0].outcome
        else {
            unreachable!()
        };
        match mode {
            0 => *format = InboxConversionFormat::LiteralTextV1,
            1 => *byte_len += 1,
            _ => sha256[0] ^= 1,
        }
        let bytes = serde_json::to_vec(&batch).unwrap();
        let raw = Connection::open(data.path().join("brn.sqlite")).unwrap();
        raw.execute(
            "UPDATE inbox_processing SET record_json=?1,record_sha256=?2 WHERE id=?3",
            params![bytes, digest(&bytes).as_slice(), request.id.to_string()],
        )
        .unwrap();
        drop(raw);
        let backups = std::fs::read_dir(data.path().join("backups"))
            .unwrap()
            .count();
        assert!(matches!(
            WorkStore::open(data.path()),
            Err(Error::Invalid(_))
        ));
        assert_eq!(
            std::fs::read_dir(data.path().join("backups"))
                .unwrap()
                .count(),
            backups
        );
    }
}
