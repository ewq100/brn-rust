//! Same-folder Create revision, exact acknowledgements and portable replay lineage.
use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{proposal_apply::*, proposals::*},
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn parent() -> VaultIdentity {
    VaultIdentity {
        device: 1,
        inode: 2,
    }
}
fn fingerprint(bytes: &[u8], inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: bytes.len() as u64,
        sha256: digest(bytes),
    }
}
fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}
fn draft() -> ProposalDraft {
    ProposalDraft {
        id: Uuid::new_v4(),
        group_id: Some(Uuid::new_v4()),
        session_id: Some(Uuid::new_v4()),
        intake: None,
        inbox_source: None,
        inbox_visual: None,
        inbox_knowledge: None,
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent(),
        }),
        title: "Retain exact review".into(),
        changes: vec![
            NoteChange::Create {
                path: "notes/new.md".into(),
                parent: parent(),
                text: "\u{feff}Owner 日本語 🦀\r\n".into(),
            },
            NoteChange::Create {
                path: "notes/second.md".into(),
                parent: parent(),
                text: "Second\r\n".into(),
            },
            NoteChange::Replace {
                path: "notes/history.md".into(),
                parent: parent(),
                before: fingerprint(b"before\r\n", 3),
                before_text: "before\r\n".into(),
                text: "after\r\n".into(),
            },
            NoteChange::CreateAsset {
                path: "notes/image.png".into(),
                parent: parent(),
                bytes: vec![0, 255, 1],
            },
        ],
        sources: vec![SourceVersion {
            path: "sources/evidence.md".into(),
            fingerprint: fingerprint(b"source", 4),
        }],
        action_changes: vec![],
    }
}
fn approval(record: &ProposalRecord) -> ApprovalRequest {
    ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    }
}
fn originals(path: &str) -> Vec<OriginalCreatePath> {
    vec![OriginalCreatePath {
        change_index: 0,
        path: path.into(),
    }]
}
fn stored_bytes(dir: &std::path::Path, id: Uuid) -> (Vec<u8>, Vec<u8>) {
    Connection::open(dir.join("brn.sqlite"))
        .unwrap()
        .query_row(
            "SELECT record_json,record_sha256 FROM proposals WHERE id=?1",
            [id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap()
}

#[test]
fn repeated_rename_preserves_complete_review_and_earliest_paths_with_exact_noop() {
    let (dir, mut store) = fixture();
    let initial = draft();
    let first = store.create_proposal(&initial).unwrap();
    let legacy_bytes = format!(
        "{{\"creation_sha256\":{},\"record\":{}}}",
        serde_json::to_string(&digest(&serde_json::to_vec(&initial).unwrap())).unwrap(),
        serde_json::to_string(&first).unwrap(),
    )
    .into_bytes();
    assert_eq!(stored_bytes(dir.path(), initial.id).0, legacy_bytes);
    let before = store
        .add_proposal_comment(&CommentRequest {
            expected: first.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Keep quote".into(),
                target: CommentTarget::Text(TextAnchor {
                    change_index: 0,
                    start: 3,
                    end: 8,
                    quote: "Owner".into(),
                }),
            },
        })
        .unwrap();
    let saved = stored_bytes(dir.path(), initial.id);
    assert_eq!(
        store
            .rename_proposal_create(before.stamp(), 0, "notes/new.md")
            .unwrap(),
        before
    );
    assert_eq!(saved, stored_bytes(dir.path(), initial.id));
    // Rename a later member first, then insert the earlier index before it.
    let before = store
        .rename_proposal_create(before.stamp(), 1, "notes/other.md")
        .unwrap();
    let renamed = store
        .rename_proposal_create(before.stamp(), 0, "notes/corrected.md")
        .unwrap();
    let mut exact = before.clone();
    let NoteChange::Create { path, .. } = &mut exact.draft.changes[0] else {
        unreachable!()
    };
    *path = "notes/corrected.md".into();
    exact.version += 1;
    exact.updated_at_ms = renamed.updated_at_ms;
    assert_eq!(renamed, exact);
    validate_create_rename_transition(&before, &renamed, 0, "notes/corrected.md").unwrap();
    assert!(store.begin_proposal_apply(&approval(&before)).is_err());
    let second = store
        .rename_proposal_create(renamed.stamp(), 1, "notes/final-other.md")
        .unwrap();
    let back = store
        .rename_proposal_create(second.stamp(), 0, "notes/new.md")
        .unwrap();
    assert_eq!(
        store.proposal_original_create_paths(initial.id).unwrap(),
        vec![
            OriginalCreatePath {
                change_index: 0,
                path: "notes/new.md".into()
            },
            OriginalCreatePath {
                change_index: 1,
                path: "notes/second.md".into()
            },
        ]
    );
    assert_eq!(store.create_proposal(&initial).unwrap(), back);
    let mut changed_initial = initial.clone();
    changed_initial.title.push('!');
    assert!(matches!(
        store.create_proposal(&changed_initial),
        Err(Error::OperationConflict(_))
    ));
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.create_proposal(&initial).unwrap(), back);
    assert_eq!(
        store
            .proposal_original_create_paths(initial.id)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn invalid_requests_and_acknowledgements_leave_exact_review_unchanged() {
    let (_dir, mut store) = fixture();
    let before = store.create_proposal(&draft()).unwrap();
    for (index, path) in [
        (0, "other/new.md"),
        (0, "notes/.hidden.md"),
        (0, "notes/../new.md"),
        (0, "notes/new.txt"),
        (0, "notes/second.md"),
        (2, "notes/retarget.md"),
        (3, "notes/retarget.md"),
        (99, "notes/new.md"),
    ] {
        assert!(
            store
                .rename_proposal_create(before.stamp(), index, path)
                .is_err(),
            "{index}: {path}"
        );
        assert_eq!(
            store.proposal(before.draft.id).unwrap(),
            Some(before.clone())
        );
    }
    let after = store
        .rename_proposal_create(before.stamp(), 0, "notes/revised.md")
        .unwrap();
    assert!(matches!(
        store.rename_proposal_create(before.stamp(), 0, "notes/revised.md"),
        Err(Error::StateChanged(_))
    ));
    for defect in 0..9 {
        let mut wrong = after.clone();
        match defect {
            0 => wrong.draft.id = Uuid::new_v4(),
            1 => wrong.draft.title.push('!'),
            2 => {
                if let NoteChange::Create { text, .. } = &mut wrong.draft.changes[0] {
                    text.push('!')
                }
            }
            3 => {
                if let NoteChange::Create { parent, .. } = &mut wrong.draft.changes[0] {
                    parent.inode += 1
                }
            }
            4 => wrong.draft.sources.clear(),
            5 => wrong.version += 1,
            6 => wrong.updated_at_ms = before.updated_at_ms - 1,
            7 => wrong.created_at_ms -= 1,
            _ => wrong.draft.changes.swap(1, 2),
        }
        assert!(
            validate_create_rename_transition(&before, &wrong, 0, "notes/revised.md").is_err(),
            "defect {defect}"
        );
    }
    let mut wrong_noop = before.clone();
    wrong_noop.version += 1;
    assert!(validate_create_rename_transition(&before, &wrong_noop, 0, "notes/new.md").is_err());
    let journal = store.begin_proposal_apply(&approval(&after)).unwrap();
    assert!(
        store
            .rename_proposal_create(
                ProposalStamp {
                    id: after.draft.id,
                    version: after.version + 1
                },
                0,
                "notes/revised.md"
            )
            .is_err()
    );
    assert_eq!(journal.original_create_paths, originals("notes/new.md"));
}

#[test]
fn oversized_original_lineage_refuses_atomically_and_empty_history_keeps_its_bytes() {
    let (dir, mut store) = fixture();
    let mut initial = draft();
    if let NoteChange::Create { path, .. } = &mut initial.changes[0] {
        *path = format!("notes/{}.md", '"'.to_string().repeat(33 * 1024));
    }
    let before = store.create_proposal(&initial).unwrap();
    let saved = stored_bytes(dir.path(), initial.id);
    let value: serde_json::Value = serde_json::from_slice(&saved.0).unwrap();
    assert!(value.get("original_create_paths").is_none());
    assert!(
        matches!(store.rename_proposal_create(before.stamp(), 0, "notes/short.md"), Err(Error::Invalid(message)) if message.contains("64 KiB"))
    );
    assert_eq!(saved, stored_bytes(dir.path(), initial.id));
    assert!(
        store
            .proposal_original_create_paths(initial.id)
            .unwrap()
            .is_empty()
    );
    let (_ordinary_dir, mut ordinary) = fixture();
    let before = ordinary.create_proposal(&draft()).unwrap();
    let journal = ordinary.begin_proposal_apply(&approval(&before)).unwrap();
    let bytes = serde_json::to_vec(&journal).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.get("original_create_paths").is_none());
    let decoded: ApplyJournal = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    assert_eq!(ordinary.restore_proposal_apply(&decoded).unwrap(), journal);
}

#[test]
fn malformed_lineage_refuses_journals_checked_reads_and_startup() {
    let (dir, mut store) = fixture();
    let before = store.create_proposal(&draft()).unwrap();
    let renamed = store
        .rename_proposal_create(before.stamp(), 0, "notes/revised.md")
        .unwrap();
    let pending = store.begin_proposal_apply(&approval(&renamed)).unwrap();
    let saved = stored_bytes(dir.path(), before.draft.id);
    let conn = Connection::open(dir.path().join("brn.sqlite")).unwrap();
    for invalid in [
        vec![OriginalCreatePath {
            change_index: 99,
            path: "notes/new.md".into(),
        }],
        vec![OriginalCreatePath {
            change_index: 2,
            path: "notes/history.md".into(),
        }],
        vec![OriginalCreatePath {
            change_index: 3,
            path: "notes/image.md".into(),
        }],
        originals("other/new.md"),
        originals("notes/.new.md"),
        originals("notes/new.png"),
        vec![originals("notes/new.md")[0].clone(); 2],
        vec![
            OriginalCreatePath {
                change_index: 1,
                path: "notes/second.md".into(),
            },
            originals("notes/new.md")[0].clone(),
        ],
        originals(&format!("notes/{}.md", "x".repeat(64 * 1024))),
        vec![originals("notes/new.md")[0].clone(); 65],
    ] {
        let mut journal = pending.clone();
        journal.original_create_paths = invalid.clone();
        assert!(journal.validate().is_err());
        let mut value: serde_json::Value = serde_json::from_slice(&saved.0).unwrap();
        value["original_create_paths"] = serde_json::to_value(invalid).unwrap();
        let bytes = serde_json::to_vec(&value).unwrap();
        conn.execute(
            "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
            params![
                before.draft.id.to_string(),
                bytes,
                digest(&bytes).as_slice()
            ],
        )
        .unwrap();
        assert!(
            store
                .proposal_original_create_paths(before.draft.id)
                .is_err()
        );
        assert!(WorkStore::open(dir.path()).is_err());
        conn.execute(
            "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
            params![before.draft.id.to_string(), saved.0, saved.1],
        )
        .unwrap();
    }
    let (journal_bytes, journal_hash): (Vec<u8>, Vec<u8>) = conn
        .query_row(
            "SELECT journal_json,journal_sha256 FROM proposal_applies WHERE operation_id=?1",
            [pending.request.operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let mut fork = pending.clone();
    fork.original_create_paths[0].path = "notes/different-original.md".into();
    fork.validate().unwrap();
    let bytes = serde_json::to_vec(&fork).unwrap();
    conn.execute(
        "UPDATE proposal_applies SET journal_json=?2,journal_sha256=?3 WHERE operation_id=?1",
        params![
            pending.request.operation_id.to_string(),
            bytes,
            digest(&bytes).as_slice()
        ],
    )
    .unwrap();
    assert!(store.proposal_apply(pending.request.operation_id).is_err());
    assert!(WorkStore::open(dir.path()).is_err());
    conn.execute(
        "UPDATE proposal_applies SET journal_json=?2,journal_sha256=?3 WHERE operation_id=?1",
        params![
            pending.request.operation_id.to_string(),
            journal_bytes,
            journal_hash
        ],
    )
    .unwrap();
    assert!(
        serde_json::from_str::<OriginalCreatePath>(
            r#"{"change_index":0,"path":"notes/new.md","extra":true}"#
        )
        .is_err()
    );
    let mut first = pending.clone();
    first.approved.version = 1;
    first.request.expected.version = 1;
    assert!(first.validate().is_err());
}

#[test]
fn mirror_and_old_database_restore_preserve_newer_lineage_without_original_journal() {
    let (_source_dir, mut source) = fixture();
    let initial = draft();
    let first = source.create_proposal(&initial).unwrap();
    let old_pending = source.begin_proposal_apply(&approval(&first)).unwrap();
    source
        .refuse_proposal_before_effects(old_pending.request.operation_id, None)
        .unwrap();
    let old_done = source
        .proposal_apply(old_pending.request.operation_id)
        .unwrap()
        .unwrap();
    let review = source.proposal(initial.id).unwrap().unwrap();
    let renamed = source
        .rename_proposal_create(review.stamp(), 0, "notes/revised.md")
        .unwrap();
    let request = approval(&renamed);
    source.begin_proposal_apply(&request).unwrap();
    source
        .refuse_proposal_before_effects(request.operation_id, None)
        .unwrap();
    let latest = source
        .proposal_apply(request.operation_id)
        .unwrap()
        .unwrap();
    let bytes = serde_json::to_vec(&latest).unwrap();
    let mirror: ApplyJournal = serde_json::from_slice(&bytes).unwrap();
    let (dir, mut recovered) = fixture();
    // A mirror alone is sufficient: no original admission or source journal.
    recovered.restore_proposal_apply(&mirror).unwrap();
    assert_eq!(
        recovered
            .proposal_original_create_paths(initial.id)
            .unwrap(),
        originals("notes/new.md")
    );
    let restored = recovered.create_proposal(&initial).unwrap();
    let original_live = source.proposal(initial.id).unwrap().unwrap();
    assert_eq!(restored.draft, original_live.draft);
    assert_eq!(restored.version, original_live.version);
    assert_eq!(restored.comments, original_live.comments);
    let live = recovered.proposal(initial.id).unwrap().unwrap();
    recovered.restore_proposal_apply(&old_done).unwrap();
    assert_eq!(recovered.proposal(initial.id).unwrap(), Some(live.clone()));
    assert_eq!(
        recovered
            .proposal_original_create_paths(initial.id)
            .unwrap(),
        latest.original_create_paths
    );
    // First prove changed-path normalization above, then separately exercise
    // a back-rename whose public paths equal the initial input again.
    let review = source.proposal(initial.id).unwrap().unwrap();
    let back = source
        .rename_proposal_create(review.stamp(), 0, "notes/new.md")
        .unwrap();
    let back_request = approval(&back);
    source.begin_proposal_apply(&back_request).unwrap();
    source
        .refuse_proposal_before_effects(back_request.operation_id, None)
        .unwrap();
    let back_latest = source
        .proposal_apply(back_request.operation_id)
        .unwrap()
        .unwrap();
    recovered.restore_proposal_apply(&back_latest).unwrap();
    let live = recovered.proposal(initial.id).unwrap().unwrap();
    // A same-version back-rename fork can have identical public records and a
    // matching creation hash while silently erasing the first-path evidence.
    let mut fork = back_latest.clone();
    fork.original_create_paths.clear();
    fork.request.operation_id = Uuid::new_v4();
    fork.receipt.as_mut().unwrap().operation_id = fork.request.operation_id;
    fork.validate().unwrap();
    assert!(recovered.restore_proposal_apply(&fork).is_err());
    assert_eq!(recovered.proposal(initial.id).unwrap(), Some(live.clone()));
    drop(recovered);
    let (mut recovered, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(recovered.create_proposal(&initial).unwrap(), live);
    // Restore into an actual old review with an unresolved older ledger, then
    // catch that ledger up to its settled NotApplied receipt after the rename.
    let (_old_dir, mut old_db) = fixture();
    old_db.restore_proposal_apply(&old_pending).unwrap();
    old_db.restore_proposal_apply(&latest).unwrap();
    let live = old_db.proposal(initial.id).unwrap().unwrap();
    old_db.restore_proposal_apply(&old_done).unwrap();
    assert_eq!(old_db.proposal(initial.id).unwrap(), Some(live));
    assert_eq!(
        old_db.proposal_original_create_paths(initial.id).unwrap(),
        latest.original_create_paths
    );
}

#[test]
fn failed_rename_write_rolls_back_record_and_first_path_together() {
    let (dir, mut store) = fixture();
    let before = store.create_proposal(&draft()).unwrap();
    let saved = stored_bytes(dir.path(), before.draft.id);
    let conn = Connection::open(dir.path().join("brn.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER refuse_rename BEFORE UPDATE ON proposals BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").unwrap();
    assert!(matches!(
        store.rename_proposal_create(before.stamp(), 0, "notes/revised.md"),
        Err(Error::Sql(_))
    ));
    assert_eq!(
        store.proposal(before.draft.id).unwrap(),
        Some(before.clone())
    );
    assert!(
        store
            .proposal_original_create_paths(before.draft.id)
            .unwrap()
            .is_empty()
    );
    assert_eq!(stored_bytes(dir.path(), before.draft.id), saved);
}
