use brn_store::{
    Error, MAX_NOTE_BYTES, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{WorkTurnStatus, proposals::*},
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: digest(text.as_bytes()),
    }
}

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn draft() -> ProposalDraft {
    ProposalDraft {
        id: Uuid::new_v4(),
        group_id: Some(Uuid::new_v4()),
        session_id: Some(Uuid::new_v4()),
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent(),
        }),
        title: "Exact review work".into(),
        changes: vec![
            NoteChange::Create {
                path: "notes/new.md".into(),
                parent: parent(),
                text: "\u{feff}---\r\ncustom: 🦀\r\n---\r\n日本語 русский\r\n".into(),
            },
            NoteChange::Replace {
                path: "notes/old.md".into(),
                parent: parent(),
                before: fingerprint("\u{feff}старое\r\n", 2),
                before_text: "\u{feff}старое\r\n".into(),
                text: "\u{feff}новое\r\n".into(),
            },
            NoteChange::Trash {
                path: "notes/trash.md".into(),
                parent: parent(),
                before: fingerprint("source\r\n", 3),
                before_text: "source\r\n".into(),
            },
        ],
        sources: vec![SourceVersion {
            path: "notes/source.md".into(),
            fingerprint: fingerprint("source", 99),
        }],
        action_changes: Vec::new(),
    }
}

fn parent() -> VaultIdentity {
    VaultIdentity {
        device: 1,
        inode: 1,
    }
}

fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}

fn edit(record: &ProposalRecord) -> ProposalEdit {
    ProposalEdit {
        expected: record.stamp(),
        title: record.draft.title.clone(),
        texts: record
            .draft
            .changes
            .iter()
            .map(|change| change.text().map(str::to_owned))
            .collect(),
        action_data: Vec::new(),
    }
}

fn comment(record: &ProposalRecord, target: CommentTarget) -> CommentRequest {
    CommentRequest {
        expected: record.stamp(),
        comment: ReviewComment {
            id: Uuid::new_v4(),
            text: "Review comment 🦀\r\n".into(),
            target,
        },
    }
}

fn raw(dir: &Path) -> Connection {
    Connection::open(dir.join("brn.sqlite")).unwrap()
}

#[test]
fn exact_review_bytes_restart_group_listing_and_creation_replay() {
    let (dir, mut store) = fixture();
    let original = draft();
    let first = store.create_proposal(&original).unwrap();
    assert_eq!(first.draft, original);
    assert_eq!(first.version, 1);
    assert_eq!(first.state, ProposalState::Draft);
    assert!(first.created_at_ms > 0);
    assert_eq!(first.updated_at_ms, first.created_at_ms);
    assert_eq!(store.create_proposal(&original).unwrap(), first);

    let mut change = edit(&first);
    change.texts[0] = Some("\u{feff}---\r\ncustom: 🦀\r\n---\r\n日本語の変更\r\n".into());
    let edited = store.edit_proposal(&change).unwrap();
    assert_eq!(edited.version, 2);
    assert_eq!(edited.draft.changes[1], original.changes[1]);
    assert_eq!(edited.draft.sources, original.sources);
    assert_eq!(store.create_proposal(&original).unwrap(), edited);
    // The UUID belongs to the original creation, not the latest edited draft.
    assert!(matches!(
        store.create_proposal(&edited.draft),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(store.edit_proposal(&edit(&edited)).unwrap(), edited);

    let mut sibling = draft();
    sibling.group_id = original.group_id;
    let sibling = store.create_proposal(&sibling).unwrap();
    let unrelated = store.create_proposal(&draft()).unwrap();
    let mut ungrouped = draft();
    ungrouped.group_id = None;
    let ungrouped = store.create_proposal(&ungrouped).unwrap();
    assert_eq!(
        store.proposals(original.group_id).unwrap(),
        vec![edited.clone(), sibling.clone()]
    );
    assert!(store.proposals(Some(Uuid::new_v4())).unwrap().is_empty());
    assert_eq!(
        store.proposals(None).unwrap(),
        vec![edited.clone(), sibling, unrelated, ungrouped]
    );
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.proposal(original.id).unwrap(), Some(edited.clone()));
    assert_eq!(store.create_proposal(&original).unwrap(), edited);
    // A proposal never created the synthetic vault or any target file.
    assert_eq!(
        std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| {
                let name = entry.unwrap().file_name();
                name.to_str()
                    .filter(|name| name.ends_with(".md"))
                    .map(str::to_owned)
            })
            .count(),
        0
    );
}

#[test]
fn unicode_anchor_invalidation_never_searches_and_explicit_reattachment_is_exact() {
    let (dir, mut store) = fixture();
    let mut original = draft();
    original.changes[0] = NoteChange::Create {
        path: "notes/new.md".into(),
        parent: parent(),
        text: "α🦀日本語🦀\r\n".into(),
    };
    let first = store.create_proposal(&original).unwrap();
    let anchor = TextAnchor {
        change_index: 0,
        start: "α".len(),
        end: "α🦀".len(),
        quote: "🦀".into(),
    };
    let anchored_request = comment(&first, CommentTarget::Text(anchor.clone()));
    let anchored = store.add_proposal_comment(&anchored_request).unwrap();
    let whole_request = comment(&anchored, CommentTarget::Proposal);
    let both = store.add_proposal_comment(&whole_request).unwrap();
    assert_eq!(both.version, 3);
    let mut change = edit(&both);
    // The same quote still exists at the same offset. Any changed target bytes
    // invalidate the old attachment rather than silently accepting it.
    change.texts[0] = Some("α🦀日本語🦀!\r\n".into());
    let changed = store.rewrite_proposal(&change).unwrap();
    assert_eq!(
        changed.comments[0].target,
        CommentTarget::Unresolved(anchor.clone())
    );
    assert_eq!(changed.comments[1], whole_request.comment);
    let mut next = edit(&changed);
    next.texts[0] = Some("short".into());
    let shortened = store.edit_proposal(&next).unwrap();
    assert_eq!(
        shortened.comments[0].target,
        CommentTarget::Unresolved(anchor)
    );
    assert_eq!(
        store.proposal(original.id).unwrap(),
        Some(shortened.clone())
    );

    let mut submitted_uncertain = anchored_request.clone();
    submitted_uncertain.expected = shortened.stamp();
    submitted_uncertain.comment.target = shortened.comments[0].target.clone();
    assert!(matches!(
        store.update_proposal_comment(&submitted_uncertain),
        Err(Error::Invalid(_))
    ));
    let mut restored_edit = edit(&shortened);
    restored_edit.texts[0] = Some("β日本語🦀\r\n".into());
    let restored = store.edit_proposal(&restored_edit).unwrap();
    assert!(matches!(
        restored.comments[0].target,
        CommentTarget::Unresolved(_)
    ));
    let mut attached = anchored_request;
    attached.expected = restored.stamp();
    attached.comment.target = CommentTarget::Text(TextAnchor {
        change_index: 0,
        start: "β日本語".len(),
        end: "β日本語🦀".len(),
        quote: "🦀".into(),
    });
    let reattached = store.update_proposal_comment(&attached).unwrap();
    assert_eq!(reattached.version, restored.version + 1);
    assert_eq!(reattached.comments[0], attached.comment);
    attached.expected = reattached.stamp();
    assert_eq!(
        store.update_proposal_comment(&attached).unwrap(),
        reattached
    );
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.proposal(original.id).unwrap(), Some(reattached));
}

#[test]
fn stale_rewrite_after_edit_comment_and_rejection_preserves_all_review_work() {
    let (_dir, mut store) = fixture();
    let original = draft();
    let first = store.create_proposal(&original).unwrap();
    let mut stale = edit(&first);
    stale.texts[0] = Some("late AI rewrite".into());
    let mut user_edit = edit(&first);
    user_edit.title = "User title".into();
    let edited = store.edit_proposal(&user_edit).unwrap();
    assert!(matches!(
        store.rewrite_proposal(&stale),
        Err(Error::StateChanged(_))
    ));
    stale.expected = edited.stamp();
    let with_comment = store
        .add_proposal_comment(&comment(&edited, CommentTarget::Proposal))
        .unwrap();
    assert!(matches!(
        store.rewrite_proposal(&stale),
        Err(Error::StateChanged(_))
    ));
    stale.expected = with_comment.stamp();
    let rejected = store.reject_proposal(with_comment.stamp()).unwrap();
    assert_eq!(rejected.state, ProposalState::Rejected);
    assert_eq!(rejected.comments, with_comment.comments);
    assert_eq!(rejected.version, with_comment.version + 1);
    assert!(matches!(
        store.rewrite_proposal(&stale),
        Err(Error::StateChanged(_))
    ));
    // Even a fresh rejected stamp cannot reopen or mutate the review.
    assert!(matches!(
        store.edit_proposal(&edit(&rejected)),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.add_proposal_comment(&comment(&rejected, CommentTarget::Proposal)),
        Err(Error::StateChanged(_))
    ));
    let update = CommentRequest {
        expected: rejected.stamp(),
        comment: rejected.comments[0].clone(),
    };
    assert!(matches!(
        store.update_proposal_comment(&update),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.remove_proposal_comment(rejected.stamp(), rejected.comments[0].id),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.reject_proposal(rejected.stamp()),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(store.proposal(original.id).unwrap(), Some(rejected.clone()));
    assert_eq!(store.create_proposal(&original).unwrap(), rejected);
}

#[test]
fn comment_crud_requires_exact_version_valid_utf8_quotes_and_unique_ids() {
    let (_dir, mut store) = fixture();
    let mut original = draft();
    original.changes[0] = NoteChange::Create {
        path: "note.md".into(),
        parent: parent(),
        text: "α🦀word".into(),
    };
    let first = store.create_proposal(&original).unwrap();
    for target in [
        CommentTarget::Text(TextAnchor {
            change_index: 0,
            start: 1,
            end: 2,
            quote: "x".into(),
        }),
        CommentTarget::Text(TextAnchor {
            change_index: 0,
            start: 2,
            end: 6,
            quote: "wrong".into(),
        }),
        CommentTarget::Text(TextAnchor {
            change_index: 0,
            start: 2,
            end: 2,
            quote: "".into(),
        }),
        CommentTarget::Text(TextAnchor {
            change_index: 3,
            start: 0,
            end: 1,
            quote: "x".into(),
        }),
        CommentTarget::Text(TextAnchor {
            change_index: 2,
            start: 0,
            end: 1,
            quote: "s".into(),
        }),
        CommentTarget::Unresolved(TextAnchor {
            change_index: 0,
            start: 2,
            end: 6,
            quote: "🦀".into(),
        }),
    ] {
        assert!(matches!(
            store.add_proposal_comment(&comment(&first, target)),
            Err(Error::Invalid(_))
        ));
        assert_eq!(store.proposal(original.id).unwrap(), Some(first.clone()));
    }
    let request = comment(&first, CommentTarget::Proposal);
    let added = store.add_proposal_comment(&request).unwrap();
    assert!(matches!(
        store.remove_proposal_comment(first.stamp(), request.comment.id),
        Err(Error::StateChanged(_))
    ));
    let mut duplicate = request.clone();
    duplicate.expected = added.stamp();
    assert!(matches!(
        store.add_proposal_comment(&duplicate),
        Err(Error::OperationConflict(_))
    ));
    duplicate.comment.text = "Updated exact bytes\r\n".into();
    let updated = store.update_proposal_comment(&duplicate).unwrap();
    assert_eq!(updated.version, 3);
    assert_eq!(updated.comments[0].text, "Updated exact bytes\r\n");
    assert!(matches!(
        store.update_proposal_comment(&duplicate),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.remove_proposal_comment(updated.stamp(), Uuid::new_v4()),
        Err(Error::NotFound(_))
    ));
    let removed = store
        .remove_proposal_comment(updated.stamp(), request.comment.id)
        .unwrap();
    assert_eq!(removed.version, 4);
    assert!(removed.comments.is_empty());
}

fn assert_invalid_new(store: &mut WorkStore, invalid: &ProposalDraft) {
    let before = store.proposals(None).unwrap();
    assert!(matches!(
        store.create_proposal(invalid),
        Err(Error::Invalid(_))
    ));
    assert_eq!(store.proposals(None).unwrap(), before);
}

#[test]
fn invalid_paths_ids_fingerprints_types_and_duplicates_fail_atomically() {
    let (_dir, mut store) = fixture();
    for bad_path in [
        "",
        "/note.md",
        "../note.md",
        "nested/../note.md",
        "nested//note.md",
        "./note.md",
        "note.txt",
        "a\\note.md",
        "a\0.md",
        ".hidden.md",
        "notes/.brn-stage.md",
        ".brn/note.md",
    ] {
        let mut invalid = draft();
        invalid.changes[0] = NoteChange::Create {
            path: bad_path.into(),
            parent: parent(),
            text: "".into(),
        };
        assert_invalid_new(&mut store, &invalid);
    }
    for field in 0..4 {
        let mut invalid = draft();
        match field {
            0 => invalid.id = Uuid::nil(),
            1 => invalid.group_id = Some(Uuid::nil()),
            2 => invalid.session_id = Some(Uuid::nil()),
            _ => invalid.vault.as_mut().unwrap().id = Uuid::nil(),
        }
        assert_invalid_new(&mut store, &invalid);
    }
    for title in ["".into(), "x".repeat(513)] {
        let mut invalid = draft();
        invalid.title = title;
        assert_invalid_new(&mut store, &invalid);
    }
    let mut invalid = draft();
    invalid.vault.as_mut().unwrap().root = "relative/vault".into();
    assert_invalid_new(&mut store, &invalid);
    let mut invalid = draft();
    invalid.changes.clear();
    assert_invalid_new(&mut store, &invalid);
    let mut invalid = draft();
    invalid.changes[0] = NoteChange::Create {
        path: "NOTES/OLD.MD".into(),
        parent: parent(),
        text: "".into(),
    };
    assert_invalid_new(&mut store, &invalid);
    let mut invalid = draft();
    if let NoteChange::Trash { before, .. } = &mut invalid.changes[2] {
        before.inode = 2;
    }
    assert_invalid_new(&mut store, &invalid);
    let mut invalid = draft();
    if let NoteChange::Replace { before, .. } = &mut invalid.changes[1] {
        before.len += 1;
    }
    assert_invalid_new(&mut store, &invalid);
    let mut invalid = draft();
    if let NoteChange::Replace { before, .. } = &mut invalid.changes[1] {
        before.sha256 = [0; 32];
    }
    assert_invalid_new(&mut store, &invalid);
    let mut invalid = draft();
    invalid.sources[0].fingerprint.len = MAX_NOTE_BYTES as u64 + 1;
    assert_invalid_new(&mut store, &invalid);
    let mut invalid = draft();
    invalid.sources[0].path = "../source.md".into();
    assert_invalid_new(&mut store, &invalid);

    let valid = store.create_proposal(&draft()).unwrap();
    for texts in [
        vec![],
        vec![None, Some("a".into()), None],
        vec![
            Some("a".into()),
            Some("b".into()),
            Some("trash text".into()),
        ],
    ] {
        let invalid = ProposalEdit {
            expected: valid.stamp(),
            title: valid.draft.title.clone(),
            texts,
            action_data: Vec::new(),
        };
        assert!(matches!(
            store.edit_proposal(&invalid),
            Err(Error::Invalid(_))
        ));
        assert_eq!(store.proposal(valid.draft.id).unwrap(), Some(valid.clone()));
    }
}

#[test]
fn note_aggregate_change_source_and_comment_limits_are_enforced_without_partial_mutation() {
    let (_dir, mut store) = fixture();
    let mut maximum_note = draft();
    maximum_note.changes[0] = NoteChange::Create {
        path: "note.md".into(),
        parent: parent(),
        text: "x".repeat(MAX_NOTE_BYTES),
    };
    let accepted = store.create_proposal(&maximum_note).unwrap();
    let mut too_big = edit(&accepted);
    too_big.texts[0].as_mut().unwrap().push('x');
    assert!(matches!(
        store.edit_proposal(&too_big),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        store.proposal(accepted.draft.id).unwrap(),
        Some(accepted.clone())
    );
    let mut quote_too_large = comment(
        &accepted,
        CommentTarget::Text(TextAnchor {
            change_index: 0,
            start: 0,
            end: MAX_COMMENT_BYTES + 1,
            quote: "x".repeat(MAX_COMMENT_BYTES + 1),
        }),
    );
    assert!(matches!(
        store.add_proposal_comment(&quote_too_large),
        Err(Error::Invalid(_))
    ));
    quote_too_large.comment.target = CommentTarget::Proposal;
    quote_too_large.comment.id = Uuid::nil();
    assert!(matches!(
        store.add_proposal_comment(&quote_too_large),
        Err(Error::Invalid(_))
    ));

    let mut bulk = draft();
    bulk.changes = (0..7)
        .map(|index| NoteChange::Create {
            path: format!("{index}.md"),
            parent: parent(),
            text: "x".repeat(MAX_NOTE_BYTES),
        })
        .collect();
    let bulk_record = store.create_proposal(&bulk).unwrap();
    bulk.id = Uuid::new_v4();
    bulk.changes.push(NoteChange::Create {
        path: "7.md".into(),
        parent: parent(),
        text: "x".repeat(MAX_NOTE_BYTES),
    });
    assert_invalid_new(&mut store, &bulk);
    let mut invalid_edit = edit(&bulk_record);
    invalid_edit.title = "x".repeat(513);
    assert!(store.edit_proposal(&invalid_edit).is_err());
    assert_eq!(
        store.proposal(bulk_record.draft.id).unwrap(),
        Some(bulk_record)
    );

    let mut near_limit = draft();
    near_limit.changes = (0..8)
        .map(|index| NoteChange::Create {
            path: format!("near/{index}.md"),
            parent: parent(),
            text: "x".repeat(MAX_NOTE_BYTES - if index == 7 { 1024 } else { 0 }),
        })
        .collect();
    let near_limit = store.create_proposal(&near_limit).unwrap();
    let mut over_aggregate = comment(&near_limit, CommentTarget::Proposal);
    over_aggregate.comment.text = "x".repeat(MAX_COMMENT_BYTES);
    assert!(matches!(
        store.add_proposal_comment(&over_aggregate),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        store.proposal(near_limit.draft.id).unwrap(),
        Some(near_limit)
    );

    let mut many = draft();
    many.changes = (0..MAX_PROPOSAL_CHANGES)
        .map(|index| NoteChange::Create {
            path: format!("{index}.md"),
            parent: parent(),
            text: "".into(),
        })
        .collect();
    many.sources = (0..MAX_PROPOSAL_CHANGES)
        .map(|index| SourceVersion {
            path: format!("sources/{index}.md"),
            fingerprint: fingerprint("", index as u64),
        })
        .collect();
    let mut record = store.create_proposal(&many).unwrap();
    let mut invalid = many.clone();
    invalid.id = Uuid::new_v4();
    invalid.changes.push(NoteChange::Create {
        path: "overflow.md".into(),
        parent: parent(),
        text: "".into(),
    });
    assert_invalid_new(&mut store, &invalid);
    let mut invalid = many;
    invalid.id = Uuid::new_v4();
    invalid.sources.push(invalid.sources[0].clone());
    assert_invalid_new(&mut store, &invalid);
    for _ in 0..MAX_PROPOSAL_COMMENTS {
        let mut request = comment(&record, CommentTarget::Proposal);
        request.comment.text = "x".repeat(MAX_COMMENT_BYTES);
        record = store.add_proposal_comment(&request).unwrap();
    }
    assert!(matches!(
        store.add_proposal_comment(&comment(&record, CommentTarget::Proposal)),
        Err(Error::Invalid(_))
    ));
    let mut oversized = CommentRequest {
        expected: record.stamp(),
        comment: record.comments[0].clone(),
    };
    oversized.comment.text.push('x');
    assert!(matches!(
        store.update_proposal_comment(&oversized),
        Err(Error::Invalid(_))
    ));
    assert_eq!(store.proposal(record.draft.id).unwrap(), Some(record));
}

#[test]
fn v3_additive_migration_preserves_settings_history_editor_recovery_and_backups() {
    let (dir, mut store) = fixture();
    store.set_setting("vault", "/synthetic/vault").unwrap();
    let turn_id = Uuid::new_v4();
    let mut chat = store.chat_connection().unwrap();
    let turn = chat
        .begin_turn(turn_id, None, "Question 🦀", "copilot", "gpt-5.5")
        .unwrap();
    chat.finish_turn(turn_id, WorkTurnStatus::Completed, "Answer\r\n", None)
        .unwrap();
    let opened = store
        .open_editor("note.md", &fingerprint("base\r\n", 20), "base\r\n")
        .unwrap();
    let recovered = store
        .recover_editor(&brn_store::work::EditRequest {
            path: opened.path.clone(),
            expected: opened.stamp,
            generation: 1,
            text: "unsaved 日本語\r\n".into(),
        })
        .unwrap();
    store
        .put_unsaved_edit("legacy-recovery.md", [1; 32], "protected")
        .unwrap();
    drop(chat);
    drop(store);
    let conn = raw(dir.path());
    conn.execute_batch("DROP TABLE inbox_processing; DROP TABLE inbox_items; DROP TABLE action_completions; DROP TABLE actions; DROP TABLE findings; ALTER TABLE messages DROP COLUMN started_at_ms; ALTER TABLE messages DROP COLUMN finished_at_ms; ALTER TABLE conversations DROP COLUMN last_activity_at_ms; ALTER TABLE messages DROP COLUMN effort; DROP TABLE proposal_rewrites; DROP TABLE proposal_applies; DROP TABLE proposals; PRAGMA user_version=3;")
        .unwrap();
    drop(conn);
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    assert!(report.backup.is_file());
    assert_eq!(
        store.setting("vault").unwrap().as_deref(),
        Some("/synthetic/vault")
    );
    assert_eq!(store.editor("note.md").unwrap(), Some(recovered));
    assert_eq!(
        store
            .unsaved_edit("legacy-recovery.md")
            .unwrap()
            .unwrap()
            .text,
        "protected"
    );
    let chat = store.chat_connection().unwrap();
    let historical = chat.turn(turn_id).unwrap().unwrap();
    assert_eq!(historical.conversation_id, turn.conversation_id);
    assert_eq!(historical.answer, "Answer\r\n");
    assert_eq!(historical.status, WorkTurnStatus::Completed);
    assert!(store.proposals(None).unwrap().is_empty());
    let proposal = store.create_proposal(&draft()).unwrap();
    assert_eq!(store.proposal(proposal.draft.id).unwrap(), Some(proposal));
    assert_eq!(
        raw(dir.path())
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        13
    );
}

#[test]
fn proposal_review_work_survives_operational_backup_restore() {
    let (dir, mut store) = fixture();
    let original = draft();
    let first = store.create_proposal(&original).unwrap();
    let reviewed = store
        .add_proposal_comment(&comment(&first, CommentTarget::Proposal))
        .unwrap();
    let rejected = store.reject_proposal(reviewed.stamp()).unwrap();
    drop(store);
    // A successful open makes the supported startup snapshot, including review
    // work. Corrupt only this disposable fixture's active operational database.
    drop(WorkStore::open(dir.path()).unwrap());
    std::fs::write(dir.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    assert!(report.restored_from.is_some());
    assert!(report.corrupt_moved_to.is_some());
    assert_eq!(store.proposal(original.id).unwrap(), Some(rejected.clone()));
    assert_eq!(store.create_proposal(&original).unwrap(), rejected);
}

fn change_stored_json(
    conn: &Connection,
    id: Uuid,
    change: impl FnOnce(&mut serde_json::Value),
    rehash: bool,
) {
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT record_json FROM proposals WHERE id=?1",
            [id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    change(&mut json);
    let changed = serde_json::to_vec(&json).unwrap();
    if rehash {
        conn.execute(
            "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
            params![id.to_string(), changed, digest(&changed).as_slice()],
        )
        .unwrap();
    } else {
        conn.execute(
            "UPDATE proposals SET record_json=?2 WHERE id=?1",
            params![id.to_string(), changed],
        )
        .unwrap();
    }
}

#[test]
fn tampered_hash_and_rehashed_invalid_structure_are_refused_on_all_read_paths() {
    let (dir, mut store) = fixture();
    let first = store.create_proposal(&draft()).unwrap();
    let conn = raw(dir.path());
    let saved: (Vec<u8>, Vec<u8>) = conn
        .query_row(
            "SELECT record_json,record_sha256 FROM proposals WHERE id=?1",
            [first.draft.id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    change_stored_json(
        &conn,
        first.draft.id,
        |value| value["record"]["draft"]["title"] = "tampered".into(),
        false,
    );
    assert!(matches!(
        store.proposal(first.draft.id),
        Err(Error::Invalid(_))
    ));
    conn.execute(
        "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
        params![first.draft.id.to_string(), saved.0, saved.1],
    )
    .unwrap();
    for defect in 0..8 {
        change_stored_json(
            &conn,
            first.draft.id,
            |value| match defect {
                0 => value["record"]["version"] = 0.into(),
                1 => value["record"]["draft"]["id"] = Uuid::new_v4().to_string().into(),
                2 => value["record"]["draft"]["group_id"] = Uuid::new_v4().to_string().into(),
                3 => value["record"]["draft"]["changes"][0]["path"] = "../escape.md".into(),
                4 => value["record"]["draft"]["title"] = "x".repeat(513).into(),
                5 => value["record"]["updated_at_ms"] = 0.into(),
                6 => value["record"]["state"] = "rejected".into(),
                _ => value["record"]["unknown_field"] = true.into(),
            },
            true,
        );
        assert!(
            matches!(store.proposal(first.draft.id), Err(Error::Invalid(_))),
            "defect {defect}"
        );
        assert!(matches!(store.proposals(None), Err(Error::Invalid(_))));
        assert!(matches!(
            store.proposals(first.draft.group_id),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.edit_proposal(&edit(&first)),
            Err(Error::Invalid(_))
        ));
        conn.execute(
            "UPDATE proposals SET record_json=?2,record_sha256=?3 WHERE id=?1",
            params![first.draft.id.to_string(), saved.0, saved.1],
        )
        .unwrap();
    }
    conn.execute(
        "UPDATE proposals SET creation_sha256=?2 WHERE id=?1",
        params![first.draft.id.to_string(), [0_u8; 32].as_slice()],
    )
    .unwrap();
    assert!(matches!(
        store.proposal(first.draft.id),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn sqlite_failure_rolls_back_creation_edits_comments_and_rejection() {
    let (dir, mut store) = fixture();
    let conn = raw(dir.path());
    conn.execute_batch("CREATE TRIGGER fail_create BEFORE INSERT ON proposals BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").unwrap();
    let original = draft();
    assert!(matches!(
        store.create_proposal(&original),
        Err(Error::Sql(_))
    ));
    assert_eq!(store.proposal(original.id).unwrap(), None);
    conn.execute_batch("DROP TRIGGER fail_create;").unwrap();
    let record = store.create_proposal(&original).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_change BEFORE UPDATE ON proposals BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").unwrap();
    let mut changed = edit(&record);
    changed.title = "new title".into();
    assert!(matches!(store.edit_proposal(&changed), Err(Error::Sql(_))));
    assert!(matches!(
        store.add_proposal_comment(&comment(&record, CommentTarget::Proposal)),
        Err(Error::Sql(_))
    ));
    assert!(matches!(
        store.reject_proposal(record.stamp()),
        Err(Error::Sql(_))
    ));
    assert_eq!(store.proposal(original.id).unwrap(), Some(record.clone()));
    conn.execute_batch("DROP TRIGGER fail_change;").unwrap();
    assert_eq!(
        store.edit_proposal(&changed).unwrap().version,
        record.version + 1
    );
}

#[test]
fn review_version_overflow_is_refused_without_wrapping() {
    let (dir, mut store) = fixture();
    let first = store.create_proposal(&draft()).unwrap();
    let conn = raw(dir.path());
    change_stored_json(
        &conn,
        first.draft.id,
        |value| value["record"]["version"] = u64::MAX.into(),
        true,
    );
    let maximum = store.proposal(first.draft.id).unwrap().unwrap();
    let mut changed = edit(&maximum);
    changed.title = "changed".into();
    assert!(matches!(
        store.edit_proposal(&changed),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        store.add_proposal_comment(&comment(&maximum, CommentTarget::Proposal)),
        Err(Error::Invalid(_))
    ));
    assert_eq!(store.proposal(first.draft.id).unwrap(), Some(maximum));
}
