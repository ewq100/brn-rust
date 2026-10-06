use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{proposal_apply::*, proposals::*},
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use uuid::Uuid;

fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: Sha256::digest(text.as_bytes()).into(),
    }
}
fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}
fn source_draft() -> ProposalDraft {
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    ProposalDraft {
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: Some(Uuid::new_v4()),
        session_id: Some(Uuid::new_v4()),
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent.clone(),
        }),
        title: "Exact original proposal 日本語".into(),
        changes: vec![
            NoteChange::Create {
                path: "notes/new.md".into(),
                parent: parent.clone(),
                text: "\u{feff}Created 🦀\r\n".into(),
            },
            NoteChange::Replace {
                path: "notes/replaced.md".into(),
                parent: parent.clone(),
                before: fingerprint("\u{feff}Original λ\r\n", 2),
                before_text: "\u{feff}Original λ\r\n".into(),
                text: "\u{feff}Replacement 日本語\r\n".into(),
            },
            NoteChange::Trash {
                path: "notes/trashed.md".into(),
                parent,
                before: fingerprint("\u{feff}Trashed λ\r\n", 3),
                before_text: "\u{feff}Trashed λ\r\n".into(),
            },
        ],
        sources: vec![SourceVersion {
            path: "external/source.md".into(),
            fingerprint: fingerprint("External evidence", 4),
        }],
        action_changes: Vec::new(),
    }
}
fn prepared(journal: &ApplyJournal) -> Vec<FileFingerprint> {
    journal
        .approved
        .draft
        .changes
        .iter()
        .enumerate()
        .map(|(index, change)| {
            if let Some(original) = journal
                .undo
                .as_ref()
                .and_then(|binding| binding.originals[index].as_ref())
            {
                return original.fingerprint.clone();
            }
            match change {
                NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } => {
                    fingerprint(text, 100 + index as u64)
                }
                NoteChange::Trash { before, .. } => before.clone(),

                _ => unreachable!("Markdown-only fixture"),
            }
        })
        .collect()
}
fn applied(journal: &ApplyJournal) -> Vec<ApplyMemberProof> {
    journal
        .approved
        .draft
        .changes
        .iter()
        .zip(journal.prepared.as_ref().unwrap())
        .map(|(change, new)| match change {
            NoteChange::Create { .. } => ApplyMemberProof {
                destination: Some(new.clone()),
                staging: None,
            },
            NoteChange::Replace { before, .. } => ApplyMemberProof {
                destination: Some(new.clone()),
                staging: Some(before.clone()),
            },
            NoteChange::Trash { before, .. } => ApplyMemberProof {
                destination: None,
                staging: Some(before.clone()),
            },

            _ => unreachable!("Markdown-only fixture"),
        })
        .collect()
}
fn apply_draft(store: &mut WorkStore, draft: &ProposalDraft) -> ApplyJournal {
    let draft = store.create_proposal(draft).unwrap();
    apply_review(store, &draft)
}
fn apply_review(store: &mut WorkStore, draft: &ProposalRecord) -> ApplyJournal {
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: draft.stamp(),
    };
    let journal = store.begin_proposal_apply(&request).unwrap();
    let journal = store
        .record_proposal_prepared(request.operation_id, &prepared(&journal))
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&journal)),
        )
        .unwrap();
    store.proposal_apply(request.operation_id).unwrap().unwrap()
}
fn source(store: &mut WorkStore) -> ApplyJournal {
    let draft = store.create_proposal(&source_draft()).unwrap();
    let draft = store
        .add_proposal_comment(&CommentRequest {
            expected: draft.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Temporary review".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    apply_review(store, &draft)
}

fn unchanged(journal: &ApplyJournal) -> Vec<ApplyMemberProof> {
    journal
        .approved
        .draft
        .changes
        .iter()
        .enumerate()
        .map(|(index, change)| ApplyMemberProof {
            destination: match change {
                NoteChange::Create { .. } => None,
                NoteChange::Replace { before, .. } | NoteChange::Trash { before, .. } => {
                    Some(before.clone())
                }

                _ => unreachable!("Markdown-only fixture"),
            },
            staging: journal
                .undo
                .as_ref()
                .and_then(|binding| binding.originals[index].as_ref())
                .map(|original| original.fingerprint.clone()),
        })
        .collect()
}

fn raw(dir: &std::path::Path) -> Connection {
    Connection::open(dir.join("brn.sqlite")).unwrap()
}

fn assert_absent(store: &WorkStore, operation: Uuid) {
    assert!(store.proposal(operation).unwrap().is_none());
    assert!(store.proposal_apply(operation).unwrap().is_none());
}
fn request(source: &ApplyJournal) -> UndoRequest {
    UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source.request.operation_id,
        trash_member: None,
    }
}

#[test]
fn preview_and_admission_bind_the_exact_ordered_inverse_without_copying_external_sources() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    let request = request(&source);
    let preview = store.preview_proposal_undo(&request).unwrap();
    assert!(store.proposal(request.operation_id).unwrap().is_none());
    assert_eq!(store.proposal_applies().unwrap(), vec![source.clone()]);
    assert_eq!(preview.draft.id, request.operation_id);
    assert_eq!(preview.draft.group_id, None);
    assert_eq!(preview.draft.session_id, source.approved.draft.session_id);
    assert_eq!(preview.draft.vault, source.approved.draft.vault);
    assert!(preview.draft.title.starts_with("Undo: "));
    assert!(preview.draft.title.len() <= source.approved.draft.title.len());
    assert!(preview.draft.sources.is_empty());
    assert_eq!(preview.binding.operation_id, request.target_operation_id);
    let before = source.prepared.as_ref().unwrap();
    assert!(
        matches!(&preview.draft.changes[0], NoteChange::Trash { path, before: proof, before_text, .. }
        if path == "notes/new.md" && proof == &before[0] && before_text == "\u{feff}Created 🦀\r\n")
    );
    assert!(
        matches!(&preview.draft.changes[1], NoteChange::Replace { path, before: proof, before_text, text, .. }
        if path == "notes/replaced.md" && proof == &before[1] && before_text == "\u{feff}Replacement 日本語\r\n" && text == "\u{feff}Original λ\r\n")
    );
    assert!(
        matches!(&preview.draft.changes[2], NoteChange::Create { path, text, .. }
        if path == "notes/trashed.md" && text == "\u{feff}Trashed λ\r\n")
    );
    assert_eq!(
        preview.binding.originals,
        vec![
            None,
            Some(UndoOriginal {
                member_id: source.members[1].id,
                fingerprint: fingerprint("\u{feff}Original λ\r\n", 2)
            }),
            Some(UndoOriginal {
                member_id: source.members[2].id,
                fingerprint: fingerprint("\u{feff}Trashed λ\r\n", 3)
            })
        ]
    );
    let journal = store.begin_proposal_undo(&request).unwrap();
    assert_eq!(journal.approved.draft, preview.draft);
    assert_eq!(journal.undo, Some(preview.binding));
    assert_eq!(journal.approved.version, 1);
    assert_eq!(journal.approved.state, ProposalState::Draft);
    assert!(journal.approved.comments.is_empty());
    assert_eq!(journal.request.operation_id, request.operation_id);
    assert_eq!(journal.request.expected.id, request.operation_id);
    assert_eq!(journal.members[1].id, source.members[1].id);
    assert_eq!(journal.members[1].staging, source.members[1].staging);
    assert_eq!(journal.members[2].id, source.members[2].id);
    assert_eq!(journal.members[2].staging, source.members[2].staging);
    assert_ne!(journal.members[0].id, source.members[0].id);
    let live = store.proposal(request.operation_id).unwrap().unwrap();
    assert_eq!(live.state, ProposalState::Applying);
    assert_eq!(live.version, 2);
    assert_eq!(live.draft, journal.approved.draft);
    assert_eq!(store.begin_proposal_undo(&request).unwrap(), journal);
    journal.validate().unwrap();
}

#[test]
fn title_truncation_preserves_utf8_and_the_original_aggregate_budget() {
    let (_dir, mut store) = fixture();
    let mut draft = source_draft();
    draft.title = "🦀".repeat(128);
    let source = apply_draft(&mut store, &draft);
    let preview = store.preview_proposal_undo(&request(&source)).unwrap();
    assert!(preview.draft.title.starts_with("Undo: 🦀"));
    assert_eq!(preview.draft.title.len(), 510);
    assert!(preview.draft.title.len() <= draft.title.len());

    let mut full = source_draft();
    full.title = "X".into();
    full.sources.clear();
    let paths: Vec<_> = (0..8).map(|index| format!("full-{index}.md")).collect();
    let overhead = full.vault.as_ref().unwrap().root.to_str().unwrap().len()
        + full.title.len()
        + paths.iter().map(String::len).sum::<usize>();
    full.changes = paths
        .into_iter()
        .enumerate()
        .map(|(index, path)| NoteChange::Create {
            path,
            parent: full.vault.as_ref().unwrap().identity.clone(),
            text: "a"
                .repeat(brn_store::work::MAX_NOTE_BYTES - if index == 7 { overhead } else { 0 }),
        })
        .collect();
    let source = apply_draft(&mut store, &full);
    let request = request(&source);
    let preview = store.preview_proposal_undo(&request).unwrap();
    assert_eq!(preview.draft.title, "X");
    assert!(
        preview
            .draft
            .changes
            .iter()
            .all(|change| matches!(change, NoteChange::Trash { .. }))
    );
    assert_eq!(
        store.begin_proposal_undo(&request).unwrap().approved.draft,
        preview.draft
    );
}

#[test]
fn inverse_review_and_journal_admission_roll_back_together() {
    let (dir, mut store) = fixture();
    let source = source(&mut store);
    let request = request(&source);
    let conn = raw(dir.path());
    for table in ["proposals", "proposal_applies"] {
        conn.execute_batch(&format!(
            "CREATE TRIGGER refuse_undo BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;"
        )).unwrap();
        assert!(matches!(
            store.begin_proposal_undo(&request),
            Err(Error::Sql(_))
        ));
        assert_absent(&store, request.operation_id);
        assert_eq!(store.proposal_applies().unwrap(), vec![source.clone()]);
        conn.execute_batch("DROP TRIGGER refuse_undo").unwrap();
    }
    assert_eq!(
        store
            .begin_proposal_undo(&request)
            .unwrap()
            .request
            .operation_id,
        request.operation_id
    );
}

#[test]
fn malformed_missing_unsettled_sources_and_uuid_collisions_do_not_admit_work() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    for bad in [
        UndoRequest {
            operation_id: Uuid::nil(),
            target_operation_id: source.request.operation_id,
            trash_member: None,
        },
        UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: Uuid::nil(),
            trash_member: None,
        },
        UndoRequest {
            operation_id: source.request.operation_id,
            target_operation_id: source.request.operation_id,
            trash_member: None,
        },
    ] {
        assert!(matches!(
            store.preview_proposal_undo(&bad),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.begin_proposal_undo(&bad),
            Err(Error::Invalid(_))
        ));
    }
    let missing = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: Uuid::new_v4(),
        trash_member: None,
    };
    assert!(matches!(
        store.begin_proposal_undo(&missing),
        Err(Error::NotFound(_))
    ));
    assert_absent(&store, missing.operation_id);
    let normal_uuid = UndoRequest {
        operation_id: source.request.operation_id,
        target_operation_id: Uuid::new_v4(),
        trash_member: None,
    };
    assert!(matches!(
        store.begin_proposal_undo(&normal_uuid),
        Err(Error::OperationConflict(_))
    ));

    let unrelated = store.create_proposal(&source_draft()).unwrap();
    let collision = UndoRequest {
        operation_id: unrelated.draft.id,
        target_operation_id: source.request.operation_id,
        trash_member: None,
    };
    assert!(matches!(
        store.begin_proposal_undo(&collision),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(
        store.proposal(unrelated.draft.id).unwrap(),
        Some(unrelated.clone())
    );
    let pending = store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: unrelated.stamp(),
        })
        .unwrap();
    let nonapplied = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: pending.request.operation_id,
        trash_member: None,
    };
    assert!(matches!(
        store.begin_proposal_undo(&nonapplied),
        Err(Error::StateChanged(_))
    ));
    assert_absent(&store, nonapplied.operation_id);
    store
        .finish_proposal_apply(pending.request.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    assert!(matches!(
        store.begin_proposal_undo(&nonapplied),
        Err(Error::StateChanged(_))
    ));
    store
        .finish_proposal_apply(
            pending.request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&unchanged(&pending)),
        )
        .unwrap();
    assert!(matches!(
        store.begin_proposal_undo(&nonapplied),
        Err(Error::StateChanged(_))
    ));
    assert_absent(&store, nonapplied.operation_id);

    let admitted = request(&source);
    store.begin_proposal_undo(&admitted).unwrap();
    let another_target = UndoRequest {
        operation_id: admitted.operation_id,
        target_operation_id: Uuid::new_v4(),
        trash_member: None,
    };
    assert!(matches!(
        store.preview_proposal_undo(&another_target),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.begin_proposal_undo(&another_target),
        Err(Error::OperationConflict(_))
    ));
}

#[test]
fn unresolved_application_blocks_admission_without_leaking_an_inverse_review() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    let unrelated = store.create_proposal(&source_draft()).unwrap();
    let blocker = store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: unrelated.stamp(),
        })
        .unwrap();
    let request = request(&source);
    store.preview_proposal_undo(&request).unwrap();
    assert!(matches!(
        store.begin_proposal_undo(&request),
        Err(Error::StateChanged(_))
    ));
    assert_absent(&store, request.operation_id);
    store
        .refuse_proposal_before_effects(blocker.request.operation_id, None)
        .unwrap();
    store.begin_proposal_undo(&request).unwrap();
}

#[test]
fn retained_original_preparations_and_strict_refusal_proofs_are_exact_and_atomic() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    let request = request(&source);
    let journal = store.begin_proposal_undo(&request).unwrap();
    let correct = prepared(&journal);
    assert!(matches!(
        store.record_proposal_prepared(request.operation_id, &correct[..2]),
        Err(Error::Invalid(_))
    ));
    let mut wrong = correct.clone();
    wrong[1].inode += 999;
    assert!(matches!(
        store.record_proposal_prepared(request.operation_id, &wrong),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        store.proposal_apply(request.operation_id).unwrap(),
        Some(journal.clone())
    );
    let journal = store
        .record_proposal_prepared(request.operation_id, &correct)
        .unwrap();
    let mut observations = unchanged(&journal);
    observations[1].staging = None;
    assert!(matches!(
        store.finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&observations)
        ),
        Err(Error::Invalid(_))
    ));
    observations = unchanged(&journal);
    observations[2].staging.as_mut().unwrap().inode += 999;
    assert!(matches!(
        store.finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&observations)
        ),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        store.proposal_apply(request.operation_id).unwrap(),
        Some(journal.clone())
    );
    let observations = unchanged(&journal);
    let receipt = store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&observations),
        )
        .unwrap();
    assert_eq!(receipt.stamp.version, 3);
    assert_eq!(
        store.begin_proposal_undo(&request).unwrap().receipt,
        Some(receipt.clone())
    );
    assert_eq!(
        store
            .finish_proposal_apply(
                request.operation_id,
                ApplyOutcome::NotApplied,
                Some(&observations)
            )
            .unwrap(),
        receipt
    );

    // Editing and ordinarily approving a refused inverse never borrows originals.
    let review = store.proposal(request.operation_id).unwrap().unwrap();
    let mut edit = ProposalEdit {
        expected: review.stamp(),
        title: review.draft.title.clone(),
        texts: review
            .draft
            .changes
            .iter()
            .map(|change| change.text().map(str::to_owned))
            .collect(),
        action_data: Vec::new(),
    };
    edit.texts[1] = Some("Owner's newer inverse 日本語\r\n".into());
    let review = store.edit_proposal(&edit).unwrap();
    assert_eq!(
        store.begin_proposal_undo(&request).unwrap().receipt,
        Some(receipt)
    );
    assert_eq!(
        store.proposal(request.operation_id).unwrap(),
        Some(review.clone())
    );
    let ordinary = store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        })
        .unwrap();
    assert_eq!(ordinary.undo, None);
    assert_ne!(ordinary.members[1].id, source.members[1].id);
    assert_ne!(ordinary.members[2].id, source.members[2].id);
}

#[test]
fn no_effects_certificate_is_distinct_from_unknown_interruption() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    let request = request(&source);
    let journal = store.begin_proposal_undo(&request).unwrap();
    let external = vec![
        ApplyMemberProof {
            destination: Some(fingerprint("external", 900)),
            staging: None
        };
        journal.members.len()
    ];
    assert!(matches!(
        store.finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&external)
        ),
        Err(Error::Invalid(_))
    ));
    let receipt = store
        .refuse_proposal_before_effects(request.operation_id, Some(&external))
        .unwrap();
    let refused = store.begin_proposal_undo(&request).unwrap();
    assert!(refused.no_effects);
    assert_eq!(refused.observations, Some(external));
    assert_eq!(refused.receipt, Some(receipt));
    let later = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source.request.operation_id,
        trash_member: None,
    };
    store.begin_proposal_undo(&later).unwrap();
    store
        .finish_proposal_apply(later.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    assert!(matches!(
        store.refuse_proposal_before_effects(later.operation_id, None),
        Err(Error::StateChanged(_))
    ));
}

#[test]
fn applied_inverse_and_exact_replay_survive_restart_and_unavailable_source() {
    let (dir, mut store) = fixture();
    let source = source(&mut store);
    let request = request(&source);
    let journal = store.begin_proposal_undo(&request).unwrap();
    let journal = store
        .record_proposal_prepared(request.operation_id, &prepared(&journal))
        .unwrap();
    let observations = applied(&journal);
    let receipt = store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&observations),
        )
        .unwrap();
    let terminal = store.begin_proposal_undo(&request).unwrap();
    assert_eq!(terminal.receipt, Some(receipt));
    terminal.validate().unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    // Replay is bound to the admitted inverse; it does not rederive from source.
    let conn = raw(dir.path());
    conn.execute(
        "DELETE FROM proposal_applies WHERE operation_id=?1",
        [source.request.operation_id.to_string()],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM proposals WHERE id=?1",
        [source.approved.draft.id.to_string()],
    )
    .unwrap();
    assert_eq!(store.begin_proposal_undo(&request).unwrap(), terminal);
    assert_eq!(
        store.preview_proposal_undo(&request).unwrap().draft,
        terminal.approved.draft
    );
    let fresh = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: request.target_operation_id,
        trash_member: None,
    };
    assert!(matches!(
        store.begin_proposal_undo(&fresh),
        Err(Error::NotFound(_))
    ));
    assert_absent(&store, fresh.operation_id);
}

#[test]
fn fresh_database_restore_is_source_independent_and_merges_prepared_uncertain_applied_forward() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    let request = request(&source);
    let pending = store.begin_proposal_undo(&request).unwrap();
    let ready = store
        .record_proposal_prepared(request.operation_id, &prepared(&pending))
        .unwrap();
    store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    let uncertain = store.proposal_apply(request.operation_id).unwrap().unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&ready)),
        )
        .unwrap();
    let terminal = store.proposal_apply(request.operation_id).unwrap().unwrap();
    let (fresh_dir, mut fresh) = fixture();
    for snapshot in [&pending, &ready, &uncertain, &terminal] {
        assert_eq!(fresh.restore_proposal_apply(snapshot).unwrap(), *snapshot);
        assert!(
            fresh
                .proposal_apply(source.request.operation_id)
                .unwrap()
                .is_none()
        );
    }
    assert_eq!(fresh.restore_proposal_apply(&pending).unwrap(), terminal);
    assert_eq!(fresh.begin_proposal_undo(&request).unwrap(), terminal);
    drop(fresh);
    let (mut fresh, _) = WorkStore::open(fresh_dir.path()).unwrap();
    assert_eq!(fresh.begin_proposal_undo(&request).unwrap(), terminal);
    assert_eq!(
        fresh.proposal(request.operation_id).unwrap().unwrap().state,
        ProposalState::Applied
    );
}

#[test]
fn binding_mutation_and_rehashed_malformed_storage_are_refused() {
    let (dir, mut store) = fixture();
    let source = source(&mut store);
    let request = request(&source);
    let journal = store.begin_proposal_undo(&request).unwrap();
    let mut other_source = journal.clone();
    other_source.undo.as_mut().unwrap().operation_id = Uuid::new_v4();
    other_source.validate().unwrap();
    assert!(matches!(
        store.restore_proposal_apply(&other_source),
        Err(Error::OperationConflict(_))
    ));
    let mut other_identity = journal.clone();
    other_identity.undo.as_mut().unwrap().originals[1]
        .as_mut()
        .unwrap()
        .fingerprint
        .inode += 900;
    other_identity.validate().unwrap();
    assert!(matches!(
        store.restore_proposal_apply(&other_identity),
        Err(Error::OperationConflict(_))
    ));
    let mut invalid = journal.clone();
    invalid.undo.as_mut().unwrap().originals[1]
        .as_mut()
        .unwrap()
        .member_id = Uuid::new_v4();
    assert!(matches!(invalid.validate(), Err(Error::Invalid(_))));
    let mut duplicate = journal.clone();
    duplicate.undo.as_mut().unwrap().originals[2] =
        duplicate.undo.as_ref().unwrap().originals[1].clone();
    assert!(matches!(duplicate.validate(), Err(Error::Invalid(_))));
    assert_eq!(
        store.proposal_apply(request.operation_id).unwrap(),
        Some(journal)
    );
    let bytes = serde_json::to_vec(&invalid).unwrap();
    raw(dir.path())
        .execute(
            "UPDATE proposal_applies SET journal_json=?1,journal_sha256=?2 WHERE operation_id=?3",
            params![
                bytes,
                Sha256::digest(&bytes).to_vec(),
                request.operation_id.to_string()
            ],
        )
        .unwrap();
    assert!(matches!(
        store.proposal_apply(request.operation_id),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn absent_undo_preserves_legacy_json_bytes_hash_and_replay_without_a_migration() {
    let (dir, mut store) = fixture();
    let source = source(&mut store);
    assert_eq!(source.undo, None);
    let conn = raw(dir.path());
    let (bytes, hash): (Vec<u8>, Vec<u8>) = conn
        .query_row(
            "SELECT journal_json,journal_sha256 FROM proposal_applies WHERE operation_id=?1",
            [source.request.operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value.get("undo").is_none());
    let decoded: ApplyJournal = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    assert_eq!(Sha256::digest(&bytes).to_vec(), hash);
    assert_eq!(store.begin_proposal_apply(&source.request).unwrap(), source);
    drop(store);
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.proposal_apply(source.request.operation_id).unwrap(),
        Some(source)
    );
    let current: Vec<u8> = conn
        .query_row(
            "SELECT journal_json FROM proposal_applies WHERE operation_id=?1",
            [decoded.request.operation_id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(current, bytes);
}

#[test]
fn scoped_trash_restore_binds_only_the_identified_member_and_preserves_mixed_source() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    let source_review = store.proposal(source.approved.draft.id).unwrap().unwrap();
    let request = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source.request.operation_id,
        trash_member: Some(2),
    };
    let preview = store.preview_proposal_undo(&request).unwrap();
    assert_eq!(preview.binding.trash_member, Some(2));
    assert_eq!(preview.draft.changes.len(), 1);
    assert!(
        matches!(&preview.draft.changes[0], NoteChange::Create { path, parent, text }
        if path == "notes/trashed.md" && parent == &source.approved.draft.vault.as_ref().unwrap().identity
        && text == "\u{feff}Trashed λ\r\n")
    );
    assert_eq!(
        preview.binding.originals,
        vec![Some(UndoOriginal {
            member_id: source.members[2].id,
            fingerprint: fingerprint("\u{feff}Trashed λ\r\n", 3),
        })]
    );
    let journal = store.begin_proposal_undo(&request).unwrap();
    assert_eq!(journal.members, vec![source.members[2].clone()]);
    let journal = store
        .record_proposal_prepared(request.operation_id, &prepared(&journal))
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&journal)),
        )
        .unwrap();
    assert_eq!(
        store.proposal_apply(source.request.operation_id).unwrap(),
        Some(source)
    );
    assert_eq!(
        store.proposal(source_review.draft.id).unwrap(),
        Some(source_review)
    );
}

#[test]
fn scoped_trash_wrong_kind_index_or_replay_scope_cannot_admit_other_work() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    for index in [0, 1, 3, 64, usize::MAX] {
        let bad = UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: source.request.operation_id,
            trash_member: Some(index),
        };
        assert!(matches!(
            store.preview_proposal_undo(&bad),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.begin_proposal_undo(&bad),
            Err(Error::Invalid(_))
        ));
        assert_absent(&store, bad.operation_id);
    }
    let whole = request(&source);
    let journal = store.begin_proposal_undo(&whole).unwrap();
    let scoped_same_id = UndoRequest {
        trash_member: Some(2),
        ..whole.clone()
    };
    assert!(matches!(
        store.begin_proposal_undo(&scoped_same_id),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.preview_proposal_undo(&scoped_same_id),
        Err(Error::OperationConflict(_))
    ));
    store
        .refuse_proposal_before_effects(journal.request.operation_id, None)
        .unwrap();
    let scoped = UndoRequest {
        operation_id: Uuid::new_v4(),
        ..scoped_same_id
    };
    store.begin_proposal_undo(&scoped).unwrap();
    let whole_same_id = UndoRequest {
        trash_member: None,
        ..scoped.clone()
    };
    assert!(matches!(
        store.begin_proposal_undo(&whole_same_id),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.preview_proposal_undo(&whole_same_id),
        Err(Error::OperationConflict(_))
    ));
    let different_member = UndoRequest {
        trash_member: Some(1),
        ..scoped
    };
    assert!(matches!(
        store.begin_proposal_undo(&different_member),
        Err(Error::OperationConflict(_))
    ));
}

#[test]
fn scoped_trash_journal_recovers_without_source_and_refuses_changed_scope() {
    let (_dir, mut store) = fixture();
    let source = source(&mut store);
    let request = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source.request.operation_id,
        trash_member: Some(2),
    };
    let journal = store.begin_proposal_undo(&request).unwrap();
    let journal = store
        .record_proposal_prepared(request.operation_id, &prepared(&journal))
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&journal)),
        )
        .unwrap();
    let terminal = store.proposal_apply(request.operation_id).unwrap().unwrap();
    let (_fresh_dir, mut fresh) = fixture();
    assert_eq!(fresh.restore_proposal_apply(&terminal).unwrap(), terminal);
    assert_eq!(fresh.begin_proposal_undo(&request).unwrap(), terminal);
    assert!(
        fresh
            .proposal_apply(source.request.operation_id)
            .unwrap()
            .is_none()
    );
    let mut wrong_scope = terminal.clone();
    wrong_scope.undo.as_mut().unwrap().trash_member = Some(3);
    wrong_scope.validate().unwrap();
    assert!(matches!(
        fresh.restore_proposal_apply(&wrong_scope),
        Err(Error::OperationConflict(_))
    ));
    let mut invalid = terminal.clone();
    invalid.undo.as_mut().unwrap().trash_member = Some(MAX_PROPOSAL_CHANGES);
    assert!(matches!(invalid.validate(), Err(Error::Invalid(_))));
    invalid = terminal.clone();
    invalid.undo.as_mut().unwrap().trash_member = None;
    invalid.validate().unwrap();
    assert!(matches!(
        fresh.restore_proposal_apply(&invalid),
        Err(Error::OperationConflict(_))
    ));
    let mut malformed_whole = store
        .preview_proposal_undo(&UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: source.request.operation_id,
            trash_member: None,
        })
        .unwrap();
    malformed_whole.binding.trash_member = Some(2);
    let whole = store
        .begin_proposal_undo(&UndoRequest {
            operation_id: malformed_whole.draft.id,
            target_operation_id: source.request.operation_id,
            trash_member: None,
        })
        .unwrap();
    let mut invalid = whole;
    invalid.undo = Some(malformed_whole.binding);
    assert!(matches!(invalid.validate(), Err(Error::Invalid(_))));
}

#[test]
fn corrupted_source_and_receipt_failure_preserve_inverse_work_atomically() {
    let (dir, mut store) = fixture();
    let source = source(&mut store);
    let request = request(&source);
    let conn = raw(dir.path());
    conn.execute(
        "UPDATE proposal_applies SET journal_sha256=zeroblob(32) WHERE operation_id=?1",
        [source.request.operation_id.to_string()],
    )
    .unwrap();
    assert!(matches!(
        store.begin_proposal_undo(&request),
        Err(Error::Invalid(_))
    ));
    assert_absent(&store, request.operation_id);
    let bytes = serde_json::to_vec(&source).unwrap();
    conn.execute(
        "UPDATE proposal_applies SET journal_sha256=?1 WHERE operation_id=?2",
        params![
            Sha256::digest(bytes).to_vec(),
            source.request.operation_id.to_string()
        ],
    )
    .unwrap();
    let journal = store.begin_proposal_undo(&request).unwrap();
    let journal = store
        .record_proposal_prepared(request.operation_id, &prepared(&journal))
        .unwrap();
    let review = store.proposal(request.operation_id).unwrap().unwrap();
    conn.execute_batch("CREATE TRIGGER fail_receipt BEFORE UPDATE ON proposal_applies WHEN NEW.outcome='applied' BEGIN SELECT RAISE(ABORT,'synthetic receipt failure'); END").unwrap();
    assert!(matches!(
        store.finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&journal))
        ),
        Err(Error::Sql(_))
    ));
    assert_eq!(
        store.proposal_apply(request.operation_id).unwrap(),
        Some(journal.clone())
    );
    assert_eq!(store.proposal(request.operation_id).unwrap(), Some(review));
    conn.execute_batch("DROP TRIGGER fail_receipt").unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&journal)),
        )
        .unwrap();
}

#[test]
fn near_core_metadata_limit_undo_and_its_inverse_complete_and_restore() {
    let (_dir, mut store) = fixture();
    let mut draft = source_draft();
    draft.title = "Bounded long paths".into();
    draft.sources.clear();
    let parent_path = std::iter::repeat_n("p".repeat(200), 16)
        .chain(std::iter::once("q".repeat(234)))
        .collect::<Vec<_>>()
        .join("/");
    draft.changes = (0..MAX_PROPOSAL_CHANGES)
        .map(|index| NoteChange::Trash {
            path: format!("{parent_path}/{index}.md"),
            parent: draft.vault.as_ref().unwrap().identity.clone(),
            before: fingerprint("a", index as u64 + 2),
            before_text: "a".into(),
        })
        .collect();
    let source = apply_draft(&mut store, &draft);
    let request = request(&source);
    let journal = store.begin_proposal_undo(&request).unwrap();
    let journal = store
        .record_proposal_prepared(request.operation_id, &prepared(&journal))
        .unwrap();
    store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&journal)),
        )
        .unwrap();
    let terminal = store.proposal_apply(request.operation_id).unwrap().unwrap();
    assert_eq!(terminal.members.len(), MAX_PROPOSAL_CHANGES);
    let (_fresh_dir, mut fresh) = fixture();
    assert_eq!(fresh.restore_proposal_apply(&terminal).unwrap(), terminal);
    assert_eq!(fresh.begin_proposal_undo(&request).unwrap(), terminal);
    assert!(
        fresh
            .proposal_apply(source.request.operation_id)
            .unwrap()
            .is_none()
    );

    let inverse_request = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: terminal.request.operation_id,
        trash_member: None,
    };
    let inverse = fresh.begin_proposal_undo(&inverse_request).unwrap();
    assert_eq!(inverse.members.len(), MAX_PROPOSAL_CHANGES);
    let inverse = fresh
        .record_proposal_prepared(inverse_request.operation_id, &prepared(&inverse))
        .unwrap();
    fresh
        .finish_proposal_apply(
            inverse_request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&inverse)),
        )
        .unwrap();
    fresh
        .proposal_apply(inverse_request.operation_id)
        .unwrap()
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn near_limit_replace_proof_widths_and_repeated_undo_cycles_complete_after_restore() {
    let (_dir, mut store) = fixture();
    let mut draft = source_draft();
    draft.sources.clear();
    let parent_path = std::iter::repeat_n("p".repeat(200), 16)
        .chain(std::iter::once("q".repeat(220)))
        .collect::<Vec<_>>()
        .join("/");
    draft.changes = (0..MAX_PROPOSAL_CHANGES)
        .map(|index| NoteChange::Replace {
            path: format!("{parent_path}/{index}.md"),
            parent: draft.vault.as_ref().unwrap().identity.clone(),
            before: fingerprint("b", u64::MAX - index as u64),
            before_text: "b".into(),
            text: "a".into(),
        })
        .collect();
    let mut source = apply_draft(&mut store, &draft);
    for cycle in 0..4 {
        let (_fresh_dir, mut fresh) = fixture();
        assert_eq!(fresh.restore_proposal_apply(&source).unwrap(), source);
        let request = request(&source);
        let journal = fresh.begin_proposal_undo(&request).unwrap();
        let journal = fresh
            .record_proposal_prepared(request.operation_id, &prepared(&journal))
            .unwrap();
        let observations = applied(&journal);
        let receipt = fresh
            .finish_proposal_apply(
                request.operation_id,
                ApplyOutcome::Applied,
                Some(&observations),
            )
            .unwrap();
        assert_eq!(receipt.outcome, ApplyOutcome::Applied);
        assert_eq!(journal.members.len(), MAX_PROPOSAL_CHANGES);
        assert!(journal.approved.draft.changes.iter().all(|change| matches!(change,
            NoteChange::Replace { before_text, text, .. }
            if (before_text.as_str(), text.as_str()) == if cycle % 2 == 0 { ("a", "b") } else { ("b", "a") }
        )));
        let terminal = fresh.proposal_apply(request.operation_id).unwrap().unwrap();
        assert_eq!(fresh.begin_proposal_undo(&request).unwrap(), terminal);
        assert_eq!(fresh.restore_proposal_apply(&source).unwrap(), source);
        terminal.validate().unwrap();
        source = terminal;
    }
}
