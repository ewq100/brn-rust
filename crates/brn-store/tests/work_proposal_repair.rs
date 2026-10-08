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
fn draft() -> ProposalDraft {
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    ProposalDraft {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: Some(VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent.clone(),
        }),
        title: "Repair exact 日本語".into(),
        sources: vec![],
        changes: vec![
            NoteChange::Create {
                path: "created.md".into(),
                parent: parent.clone(),
                text: "\u{feff}Created λ\r\n".into(),
            },
            NoteChange::Replace {
                path: "replaced.md".into(),
                parent: parent.clone(),
                before: fingerprint("\u{feff}Old 日本語\r\n", 2),
                before_text: "\u{feff}Old 日本語\r\n".into(),
                text: "\u{feff}Replacement 🦀\r\n".into(),
            },
            NoteChange::Trash {
                path: "trashed.md".into(),
                parent,
                before: fingerprint("\u{feff}Retained λ\r\n", 3),
                before_text: "\u{feff}Retained λ\r\n".into(),
            },
        ],
        action_changes: Vec::new(),
    }
}
fn prepared(store: &mut WorkStore, draft: &ProposalDraft) -> ApplyJournal {
    let review = store.create_proposal(draft).unwrap();
    let review = store
        .add_proposal_comment(&CommentRequest {
            expected: review.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Temporary 🦀\r\n".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    let journal = store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        })
        .unwrap();
    let proofs: Vec<_> = journal
        .approved
        .draft
        .changes
        .iter()
        .enumerate()
        .map(|(index, change)| match change {
            NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } => {
                fingerprint(text, 100 + index as u64)
            }
            NoteChange::Trash { before, .. } => before.clone(),

            _ => unreachable!("Markdown-only fixture"),
        })
        .collect();
    store
        .record_proposal_prepared(journal.request.operation_id, &proofs)
        .unwrap()
}
fn observed(journal: &ApplyJournal, mask: u64) -> Vec<ApplyMemberProof> {
    journal
        .approved
        .draft
        .changes
        .iter()
        .zip(journal.prepared.as_ref().unwrap())
        .enumerate()
        .map(|(index, (change, new))| {
            let applied = mask & (1 << index) != 0;
            match change {
                NoteChange::Create { .. } => ApplyMemberProof {
                    destination: applied.then(|| new.clone()),
                    staging: (!applied).then(|| new.clone()),
                },
                NoteChange::Replace { before, .. } => ApplyMemberProof {
                    destination: Some(if applied { new } else { before }.clone()),
                    staging: Some(if applied { before } else { new }.clone()),
                },
                NoteChange::Trash { before, .. } => ApplyMemberProof {
                    destination: (!applied).then(|| before.clone()),
                    staging: applied.then(|| before.clone()),
                },

                _ => unreachable!("Markdown-only fixture"),
            }
        })
        .collect()
}

fn read(store: &WorkStore, journal: &ApplyJournal) -> ApplyJournal {
    store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap()
}
fn raw(dir: &std::path::Path) -> Connection {
    Connection::open(dir.join("brn.sqlite")).unwrap()
}

fn overwrite(conn: &Connection, journal: &ApplyJournal, rehash: bool) {
    let bytes = serde_json::to_vec(journal).unwrap();
    let digest: [u8; 32] = if rehash {
        Sha256::digest(&bytes).into()
    } else {
        [0; 32]
    };
    conn.execute(
        "UPDATE proposal_applies SET journal_json=?1,journal_sha256=?2 WHERE operation_id=?3",
        params![
            bytes,
            digest.as_slice(),
            journal.request.operation_id.to_string()
        ],
    )
    .unwrap();
}

#[test]
fn unknown_partial_unprepared_and_settled_sources_refuse_without_admission() {
    let (_dir, mut store) = fixture();
    let journal = prepared(&mut store, &draft());
    let proofs = observed(&journal, 0);
    let request = request(&journal, &proofs, RepairDirection::Finish);
    assert!(matches!(
        journal.repair_preview(&proofs[..2]),
        Err(Error::Invalid(_))
    ));
    for index in 0..3 {
        let mut changed = proofs.clone();
        changed[index].destination = Some(fingerprint("unrelated", 900));
        assert!(matches!(
            journal.repair_preview(&changed),
            Err(Error::StateChanged(_))
        ));
        assert!(matches!(
            store.begin_proposal_repair(&request, &changed),
            Err(Error::StateChanged(_))
        ));
        assert_eq!(read(&store, &journal), journal);
    }
    let mut unprepared = journal.clone();
    unprepared.prepared = None;
    assert!(matches!(
        unprepared.repair_preview(&proofs),
        Err(Error::StateChanged(_))
    ));
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed(&journal, 7)),
        )
        .unwrap();
    assert!(matches!(
        read(&store, &journal).repair_preview(&proofs),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.begin_proposal_repair(&request, &proofs),
        Err(Error::StateChanged(_))
    ));
    assert!(store.proposal_repair(request.id).unwrap().is_none());
}

#[test]
fn stamps_bind_observation_identity_and_prior_ids_but_ignore_comment_cleanup_and_original_receipts()
{
    let (_dir, mut store) = fixture();
    let journal = prepared(&mut store, &draft());
    let before = observed(&journal, 0);
    let preview = journal.repair_preview(&before).unwrap();
    let mut commentless = journal.clone();
    commentless.approved.comments.clear();
    assert_eq!(
        commentless.repair_preview(&before).unwrap().expected,
        preview.expected
    );
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Uncertain,
            Some(&observed(&journal, 1)),
        )
        .unwrap();
    let uncertain = read(&store, &journal);
    assert_eq!(
        uncertain.repair_preview(&before).unwrap().expected,
        preview.expected
    );
    let request = request(&uncertain, &before, RepairDirection::Finish);
    assert!(matches!(
        store.begin_proposal_repair(&request, &observed(&uncertain, 2)),
        Err(Error::StateChanged(_))
    ));
    let admitted = store.begin_proposal_repair(&request, &before).unwrap();
    assert_ne!(
        admitted.repair_preview(&before).unwrap().expected,
        preview.expected
    );
    let mut stale = request.clone();
    stale.id = Uuid::new_v4();
    assert!(matches!(
        store.begin_proposal_repair(&stale, &before),
        Err(Error::StateChanged(_))
    ));
    for changed in [
        RepairRequest {
            direction: RepairDirection::Restore,
            ..request.clone()
        },
        RepairRequest {
            operation_id: Uuid::new_v4(),
            ..request.clone()
        },
        RepairRequest {
            expected: [0; 32],
            ..request.clone()
        },
    ] {
        assert!(matches!(
            store.begin_proposal_repair(&changed, &[]),
            Err(Error::OperationConflict(_))
        ));
    }
    assert_eq!(
        store.begin_proposal_repair(&request, &[]).unwrap(),
        admitted
    );
}

#[test]
fn interruptions_and_new_captures_preserve_first_original_uncertainty_and_review_work() {
    let (_dir, mut store) = fixture();
    let journal = prepared(&mut store, &draft());
    let original_proofs = observed(&journal, 1);
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Uncertain,
            Some(&original_proofs),
        )
        .unwrap();
    let original = read(&store, &journal);
    let review = store.proposal(journal.approved.draft.id).unwrap().unwrap();
    let first = request(&original, &observed(&original, 2), RepairDirection::Finish);
    let first_journal = store
        .begin_proposal_repair(&first, &observed(&original, 2))
        .unwrap();
    assert!(matches!(
        store.finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Uncertain,
            Some(&observed(&journal, 2))
        ),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(read(&store, &journal), first_journal);
    let receipt = store.interrupt_proposal_repair(first.id).unwrap();
    assert_eq!(receipt.outcome, Some(ApplyOutcome::Uncertain));
    assert_eq!(store.interrupt_proposal_repair(first.id).unwrap(), receipt);
    let interrupted = read(&store, &journal);
    assert_eq!(interrupted.receipt, original.receipt);
    assert_eq!(interrupted.observations, original.observations);
    let second = request(
        &interrupted,
        &observed(&interrupted, 4),
        RepairDirection::Restore,
    );
    let second_journal = store
        .begin_proposal_repair(&second, &observed(&interrupted, 4))
        .unwrap();
    assert_eq!(
        second_journal.repair.as_ref().unwrap().attempts[0].request,
        first
    );
    assert_eq!(
        second_journal.repair.as_ref().unwrap().attempts[0].outcome,
        Some(ApplyOutcome::Uncertain)
    );
    assert_eq!(store.interrupt_proposal_repair(first.id).unwrap(), receipt);
    assert_eq!(read(&store, &journal), second_journal);
    assert_eq!(
        store.proposal(journal.approved.draft.id).unwrap(),
        Some(review)
    );
    assert!(matches!(
        store.refuse_proposal_before_effects(journal.request.operation_id, None),
        Err(Error::StateChanged(_))
    ));
}

#[test]
fn admission_interrupt_and_whole_receipt_failures_roll_back_history_and_comments() {
    let (dir, mut store) = fixture();
    let journal = prepared(&mut store, &draft());
    let proofs = observed(&journal, 1);
    let request = request(&journal, &proofs, RepairDirection::Finish);
    let review = store.proposal(journal.approved.draft.id).unwrap().unwrap();
    let conn = raw(dir.path());
    conn.execute_batch("CREATE TRIGGER fail_history BEFORE UPDATE ON proposal_applies BEGIN SELECT RAISE(ABORT,'synthetic history failure'); END").unwrap();
    assert!(matches!(
        store.begin_proposal_repair(&request, &proofs),
        Err(Error::Sql(_))
    ));
    assert_eq!(read(&store, &journal), journal);
    assert!(store.proposal_repair(request.id).unwrap().is_none());
    conn.execute_batch("DROP TRIGGER fail_history").unwrap();
    let admitted = store.begin_proposal_repair(&request, &proofs).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_history BEFORE UPDATE ON proposal_applies BEGIN SELECT RAISE(ABORT,'synthetic history failure'); END").unwrap();
    assert!(matches!(
        store.interrupt_proposal_repair(request.id),
        Err(Error::Sql(_))
    ));
    assert_eq!(read(&store, &journal), admitted);
    conn.execute_batch("DROP TRIGGER fail_history; CREATE TRIGGER fail_review BEFORE UPDATE ON proposals WHEN json_extract(NEW.record_json,'$.record.state')='applied' BEGIN SELECT RAISE(ABORT,'synthetic review failure'); END").unwrap();
    assert!(matches!(
        store.finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed(&journal, 7))
        ),
        Err(Error::Sql(_))
    ));
    assert_eq!(read(&store, &journal), admitted);
    assert_eq!(
        store.proposal(journal.approved.draft.id).unwrap(),
        Some(review)
    );
    conn.execute_batch("DROP TRIGGER fail_review").unwrap();
    let whole = store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed(&journal, 7)),
        )
        .unwrap();
    let repaired = store.proposal_repair(request.id).unwrap().unwrap();
    assert_eq!(repaired.outcome, Some(whole.outcome));
    assert!(read(&store, &journal).approved.comments.is_empty());
    assert!(
        store
            .proposal(journal.approved.draft.id)
            .unwrap()
            .unwrap()
            .comments
            .is_empty()
    );
}

#[test]
fn terminal_results_and_historical_attempt_replay_survive_restart_and_later_draft_edits() {
    let (dir, mut store) = fixture();
    let journal = prepared(&mut store, &draft());
    let first = request(&journal, &observed(&journal, 1), RepairDirection::Finish);
    let first_journal = store
        .begin_proposal_repair(&first, &observed(&journal, 1))
        .unwrap();
    let second = request(
        &first_journal,
        &observed(&journal, 2),
        RepairDirection::Restore,
    );
    let second_journal = store
        .begin_proposal_repair(&second, &observed(&journal, 2))
        .unwrap();
    assert_eq!(
        second_journal.repair.as_ref().unwrap().attempts[0].outcome,
        Some(ApplyOutcome::Uncertain)
    );
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&observed(&journal, 0)),
        )
        .unwrap();
    let terminal = read(&store, &journal);
    let review = store.proposal(journal.approved.draft.id).unwrap().unwrap();
    assert!(!review.comments.is_empty());
    let mut texts: Vec<_> = review
        .draft
        .changes
        .iter()
        .map(|change| change.text().map(str::to_owned))
        .collect();
    texts[0] = Some("Newer owner draft 日本語\r\n".into());
    let newer = store
        .edit_proposal(&ProposalEdit {
            expected: review.stamp(),
            title: review.draft.title.clone(),
            texts,
            action_data: Vec::new(),
        })
        .unwrap();
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.begin_proposal_repair(&first, &[]).unwrap(), terminal);
    assert_eq!(store.begin_proposal_repair(&second, &[]).unwrap(), terminal);
    assert_eq!(
        store.proposal_repair(first.id).unwrap().unwrap().outcome,
        Some(ApplyOutcome::Uncertain)
    );
    assert_eq!(
        store.proposal_repair(second.id).unwrap().unwrap().outcome,
        Some(ApplyOutcome::NotApplied)
    );
    assert_eq!(
        store.proposal(journal.approved.draft.id).unwrap(),
        Some(newer)
    );
}

#[test]
fn global_repair_uuid_collision_is_refused_on_admission_recovery_and_checked_lookup() {
    let (dir, mut store) = fixture();
    let first = prepared(&mut store, &draft());
    let first_request = request(&first, &observed(&first, 1), RepairDirection::Finish);
    store
        .begin_proposal_repair(&first_request, &observed(&first, 1))
        .unwrap();
    store
        .finish_proposal_apply(
            first.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed(&first, 7)),
        )
        .unwrap();
    let second = prepared(&mut store, &draft());
    let mut second_request = request(&second, &observed(&second, 2), RepairDirection::Restore);
    second_request.id = first_request.id;
    assert!(matches!(
        store.begin_proposal_repair(&second_request, &[]),
        Err(Error::OperationConflict(_))
    ));
    second_request.id = Uuid::new_v4();
    let mut incoming = store
        .begin_proposal_repair(&second_request, &observed(&second, 2))
        .unwrap();
    let before = incoming.clone();
    incoming.repair.as_mut().unwrap().attempts[0].request.id = first_request.id;
    incoming.validate().unwrap();
    assert!(matches!(
        store.restore_proposal_apply(&incoming),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(read(&store, &second), before);
    let bytes = serde_json::to_vec(&incoming).unwrap();
    raw(dir.path())
        .execute(
            "UPDATE proposal_applies SET journal_json=?1,journal_sha256=?2 WHERE operation_id=?3",
            params![
                bytes,
                Sha256::digest(&bytes).to_vec(),
                second.request.operation_id.to_string()
            ],
        )
        .unwrap();
    assert!(matches!(
        store.proposal_repair(first_request.id),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn recovery_merges_only_compatible_forward_history_and_settled_endpoints_cover_old_metadata() {
    let (_dir, mut store) = fixture();
    let journal = prepared(&mut store, &draft());
    let a = request(&journal, &observed(&journal, 1), RepairDirection::Finish);
    let first = store
        .begin_proposal_repair(&a, &observed(&journal, 1))
        .unwrap();
    store.interrupt_proposal_repair(a.id).unwrap();
    let interrupted = read(&store, &journal);
    let b = request(
        &interrupted,
        &observed(&journal, 2),
        RepairDirection::Restore,
    );
    let second = store
        .begin_proposal_repair(&b, &observed(&journal, 2))
        .unwrap();
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed(&journal, 7)),
        )
        .unwrap();
    let terminal = read(&store, &journal);
    let (_fresh_dir, mut fresh) = fixture();
    for snapshot in [&journal, &first, &interrupted, &second, &terminal] {
        assert_eq!(fresh.restore_proposal_apply(snapshot).unwrap(), *snapshot);
    }
    assert!(terminal.repair_history_covers(&second));
    assert!(second.repair_history_covers(&first));
    assert!(!first.repair_history_covers(&second));
    assert_eq!(fresh.restore_proposal_apply(&first).unwrap(), terminal);
    assert_eq!(fresh.restore_proposal_apply(&journal).unwrap(), terminal);
    let mut fork = second.clone();
    fork.repair.as_mut().unwrap().attempts[0].request.direction = RepairDirection::Restore;
    fork.validate().unwrap();
    assert!(!terminal.repair_history_covers(&fork));
    assert!(matches!(
        fresh.restore_proposal_apply(&fork),
        Err(Error::OperationConflict(_))
    ));
    let mut changed_capture = first.clone();
    changed_capture.repair.as_mut().unwrap().observations = observed(&journal, 4);
    // Its request cannot be changed under the same ID to adopt new observations.
    assert!(changed_capture.validate().is_err());
    let (_other_dir, mut other) = fixture();
    let unrelated = prepared(&mut other, &draft());
    assert!(!terminal.repair_history_covers(&unrelated));
}
fn request(
    journal: &ApplyJournal,
    proofs: &[ApplyMemberProof],
    direction: RepairDirection,
) -> RepairRequest {
    RepairRequest {
        id: Uuid::new_v4(),
        operation_id: journal.request.operation_id,
        expected: journal.repair_preview(proofs).unwrap().expected,
        direction,
    }
}

#[test]
fn all_mixed_phases_are_exact_and_both_directions_admit_without_changing_review() {
    for mask in 0..8 {
        for direction in [RepairDirection::Finish, RepairDirection::Restore] {
            let (_dir, mut store) = fixture();
            let journal = prepared(&mut store, &draft());
            let proofs = observed(&journal, mask);
            let preview = journal.repair_preview(&proofs).unwrap();
            assert_eq!(preview.approved, journal.approved.draft);
            assert_eq!(
                preview.phases,
                (0..3)
                    .map(|index| if mask & (1 << index) == 0 {
                        ApplyMemberPhase::Before
                    } else {
                        ApplyMemberPhase::Applied
                    })
                    .collect::<Vec<_>>()
            );
            let review = store.proposal(journal.approved.draft.id).unwrap().unwrap();
            let request = request(&journal, &proofs, direction);
            let admitted = store.begin_proposal_repair(&request, &proofs).unwrap();
            assert_eq!(
                store.proposal(journal.approved.draft.id).unwrap(),
                Some(review)
            );
            assert_eq!(admitted.observations, journal.observations);
            assert_eq!(admitted.receipt, journal.receipt);
            assert_eq!(admitted.repair.as_ref().unwrap().observations, proofs);
            assert_eq!(
                store.proposal_repair(request.id).unwrap().unwrap().outcome,
                None
            );
            assert_eq!(
                store.begin_proposal_repair(&request, &[]).unwrap(),
                admitted
            );
        }
    }
}

#[test]
fn terminal_repair_receipts_require_exact_complete_endpoint_pairs() {
    for missing in [true, false] {
        let (_dir, mut store) = fixture();
        let source = prepared(&mut store, &draft());
        let proofs = observed(&source, 0);
        let repair = request(&source, &proofs, RepairDirection::Restore);
        let admitted = store.begin_proposal_repair(&repair, &proofs).unwrap();
        let review = store.proposal(source.approved.draft.id).unwrap();
        let mut unknown = proofs.clone();
        if missing {
            unknown[0].staging = None;
        } else {
            unknown[1].staging.as_mut().unwrap().inode += 1000;
        }
        assert!(
            store
                .finish_proposal_apply(
                    source.request.operation_id,
                    ApplyOutcome::NotApplied,
                    Some(&unknown)
                )
                .is_err()
        );
        assert_eq!(read(&store, &source), admitted);
        assert_eq!(store.proposal(source.approved.draft.id).unwrap(), review);
        assert_eq!(
            store.proposal_repair(repair.id).unwrap().unwrap().outcome,
            None
        );
        store
            .finish_proposal_apply(
                source.request.operation_id,
                ApplyOutcome::NotApplied,
                Some(&proofs),
            )
            .unwrap();
        assert_eq!(
            store.proposal_repair(repair.id).unwrap().unwrap().outcome,
            Some(ApplyOutcome::NotApplied)
        );
    }
}

#[test]
fn basic_requests_and_rehashed_structural_corruption_are_refused() {
    let (dir, mut store) = fixture();
    let journal = prepared(&mut store, &draft());
    let proofs = observed(&journal, 0);
    let valid = request(&journal, &proofs, RepairDirection::Finish);
    for request in [
        RepairRequest {
            id: Uuid::nil(),
            ..valid.clone()
        },
        RepairRequest {
            operation_id: Uuid::nil(),
            ..valid.clone()
        },
        RepairRequest {
            id: valid.operation_id,
            ..valid.clone()
        },
    ] {
        assert!(matches!(
            store.begin_proposal_repair(&request, &proofs),
            Err(Error::Invalid(_))
        ));
        assert_eq!(read(&store, &journal), journal);
    }
    let absent = RepairRequest {
        operation_id: Uuid::new_v4(),
        ..valid.clone()
    };
    assert!(matches!(
        store.begin_proposal_repair(&absent, &proofs),
        Err(Error::NotFound(_))
    ));
    assert!(store.proposal_repair(absent.id).unwrap().is_none());
    assert!(matches!(
        store.interrupt_proposal_repair(absent.id),
        Err(Error::NotFound(_))
    ));

    let admitted = store.begin_proposal_repair(&valid, &proofs).unwrap();
    let mut invalid = vec![];
    let mut changed = admitted.clone();
    changed.repair.as_mut().unwrap().attempts[0]
        .request
        .expected = [0; 32];
    invalid.push(changed);
    let mut changed = admitted.clone();
    changed.repair.as_mut().unwrap().attempts[0].request.id = Uuid::nil();
    invalid.push(changed);
    let mut changed = admitted.clone();
    changed.repair.as_mut().unwrap().attempts[0]
        .request
        .operation_id = Uuid::new_v4();
    invalid.push(changed);
    let mut changed = admitted.clone();
    changed.repair.as_mut().unwrap().attempts[0].started_at_ms = journal.started_at_ms - 1;
    invalid.push(changed);
    let mut changed = admitted.clone();
    changed.repair.as_mut().unwrap().attempts[0].outcome = Some(ApplyOutcome::Applied);
    invalid.push(changed);
    let mut changed = admitted.clone();
    changed.repair.as_mut().unwrap().observations[0].staging = None;
    invalid.push(changed);
    let mut changed = admitted.clone();
    changed.repair.as_mut().unwrap().attempts.clear();
    invalid.push(changed);
    let conn = raw(dir.path());
    for changed in invalid {
        assert!(matches!(changed.validate(), Err(Error::Invalid(_))));
        overwrite(&conn, &changed, true);
        assert!(matches!(
            store.proposal_apply(journal.request.operation_id),
            Err(Error::Invalid(_))
        ));
        assert!(matches!(
            store.proposal_repair(valid.id),
            Err(Error::Invalid(_))
        ));
    }
    overwrite(&conn, &admitted, false);
    assert!(matches!(
        store.proposal_apply(journal.request.operation_id),
        Err(Error::Invalid(_))
    ));
    overwrite(&conn, &admitted, true);
    assert_eq!(read(&store, &journal), admitted);

    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&proofs),
        )
        .unwrap();
    let mut certified = read(&store, &journal);
    certified.no_effects = true;
    assert!(matches!(certified.validate(), Err(Error::Invalid(_))));
}

#[test]
fn recovery_rejects_capture_forks_short_terminal_histories_and_rolls_back_failed_imports() {
    let (_dir, mut source) = fixture();
    let journal = prepared(&mut source, &draft());
    let a = request(&journal, &observed(&journal, 1), RepairDirection::Finish);
    let first = source
        .begin_proposal_repair(&a, &observed(&journal, 1))
        .unwrap();
    let b = request(&first, &observed(&journal, 2), RepairDirection::Restore);
    let second = source
        .begin_proposal_repair(&b, &observed(&journal, 2))
        .unwrap();

    let (dir, mut target) = fixture();
    let conn = raw(dir.path());
    conn.execute_batch("CREATE TRIGGER fail_import BEFORE INSERT ON proposal_applies BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(matches!(
        target.restore_proposal_apply(&first),
        Err(Error::Sql(_))
    ));
    assert!(
        target
            .proposal(journal.approved.draft.id)
            .unwrap()
            .is_none()
    );
    assert!(
        target
            .proposal_apply(journal.request.operation_id)
            .unwrap()
            .is_none()
    );
    conn.execute_batch("DROP TRIGGER fail_import;").unwrap();
    target.restore_proposal_apply(&first).unwrap();
    let mut fork = first.clone();
    fork.repair.as_mut().unwrap().observations = observed(&journal, 4);
    fork.repair.as_mut().unwrap().attempts[0].request.expected = journal
        .repair_preview(&observed(&journal, 4))
        .unwrap()
        .expected;
    fork.validate().unwrap();
    assert!(!first.repair_history_covers(&fork));
    assert!(matches!(
        target.restore_proposal_apply(&fork),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(read(&target, &journal), first);

    conn.execute_batch("CREATE TRIGGER fail_forward BEFORE UPDATE ON proposal_applies BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(matches!(
        target.restore_proposal_apply(&second),
        Err(Error::Sql(_))
    ));
    assert_eq!(read(&target, &journal), first);
    conn.execute_batch("DROP TRIGGER fail_forward;").unwrap();
    target.restore_proposal_apply(&second).unwrap();

    let (_old_dir, mut old) = fixture();
    old.restore_proposal_apply(&first).unwrap();
    old.finish_proposal_apply(
        journal.request.operation_id,
        ApplyOutcome::Applied,
        Some(&observed(&journal, 7)),
    )
    .unwrap();
    let short_terminal = read(&old, &journal);
    assert!(!short_terminal.repair_history_covers(&second));
    assert!(matches!(
        target.restore_proposal_apply(&short_terminal),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(read(&target, &journal), second);
}

#[test]
fn omitted_repair_preserves_legacy_json_hash_and_repair_preserves_undo_bindings() {
    let (dir, mut store) = fixture();
    let journal = prepared(&mut store, &draft());
    let conn = raw(dir.path());
    let (bytes, digest): (Vec<u8>, Vec<u8>) = conn
        .query_row(
            "SELECT journal_json,journal_sha256 FROM proposal_applies WHERE operation_id=?1",
            [journal.request.operation_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(value.get("repair").is_none());
    let legacy: ApplyJournal = serde_json::from_slice(&bytes).unwrap();
    assert!(legacy.repair.is_none());
    assert_eq!(serde_json::to_vec(&legacy).unwrap(), bytes);
    assert_eq!(Sha256::digest(&bytes).as_slice(), digest);
    drop(conn);
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(read(&store, &journal), journal);

    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed(&journal, 7)),
        )
        .unwrap();
    let undo = store
        .begin_proposal_undo(&UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: journal.request.operation_id,
            trash_member: None,
        })
        .unwrap();
    let proofs: Vec<_> = undo
        .approved
        .draft
        .changes
        .iter()
        .zip(&undo.undo.as_ref().unwrap().originals)
        .map(|(change, original)| match change {
            NoteChange::Trash { before, .. } => before.clone(),
            _ => original.as_ref().unwrap().fingerprint.clone(),
        })
        .collect();
    let undo = store
        .record_proposal_prepared(undo.request.operation_id, &proofs)
        .unwrap();
    let request = request(&undo, &observed(&undo, 2), RepairDirection::Restore);
    let admitted = store
        .begin_proposal_repair(&request, &observed(&undo, 2))
        .unwrap();
    assert_eq!(admitted.undo, undo.undo);
    store
        .finish_proposal_apply(
            undo.request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&observed(&undo, 0)),
        )
        .unwrap();
    let terminal = read(&store, &undo);
    assert_eq!(terminal.undo, undo.undo);
    assert_eq!(
        terminal.repair.as_ref().unwrap().attempts[0].outcome,
        Some(ApplyOutcome::NotApplied)
    );
    let (_fresh_dir, mut fresh) = fixture();
    assert_eq!(fresh.restore_proposal_apply(&terminal).unwrap(), terminal);
    assert!(
        fresh
            .proposal_apply(journal.request.operation_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fresh.begin_proposal_repair(&request, &[]).unwrap(),
        terminal
    );
}

#[test]
fn maximum_repair_history_near_core_limit_still_completes_and_recovers() {
    let (dir, mut store) = fixture();
    let mut draft = draft();
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
    let mut journal = prepared(&mut store, &draft);
    let review = store.proposal(draft.id).unwrap().unwrap();
    for index in 0..64 {
        let proofs = observed(&journal, if index % 2 == 0 { 0 } else { u64::MAX });
        let request = request(
            &journal,
            &proofs,
            if index % 2 == 0 {
                RepairDirection::Finish
            } else {
                RepairDirection::Restore
            },
        );
        journal = store.begin_proposal_repair(&request, &proofs).unwrap();
        assert_eq!(journal.repair.as_ref().unwrap().attempts.len(), index + 1);
    }
    assert_eq!(store.proposal(draft.id).unwrap(), Some(review));
    let excess = request(&journal, &observed(&journal, 0), RepairDirection::Finish);
    assert!(matches!(
        store.begin_proposal_repair(&excess, &observed(&journal, 0)),
        Err(Error::Invalid(_))
    ));
    assert_eq!(read(&store, &journal), journal);
    assert!(store.proposal_repair(excess.id).unwrap().is_none());
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observed(&journal, u64::MAX)),
        )
        .unwrap();
    let terminal = read(&store, &journal);
    let binding = terminal.repair.as_ref().unwrap();
    assert!(
        binding.attempts[..63]
            .iter()
            .all(|attempt| attempt.outcome == Some(ApplyOutcome::Uncertain))
    );
    assert_eq!(binding.attempts[63].outcome, Some(ApplyOutcome::Applied));
    assert!(terminal.approved.comments.is_empty());
    let size: i64 = raw(dir.path())
        .query_row(
            "SELECT length(journal_json) FROM proposal_applies WHERE operation_id=?1",
            [journal.request.operation_id.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(size < 64 * 1024 * 1024);
    let (_fresh_dir, mut fresh) = fixture();
    assert_eq!(fresh.restore_proposal_apply(&terminal).unwrap(), terminal);
    assert_eq!(
        fresh
            .proposal_repair(binding.attempts[63].request.id)
            .unwrap()
            .unwrap()
            .outcome,
        Some(ApplyOutcome::Applied)
    );
}
