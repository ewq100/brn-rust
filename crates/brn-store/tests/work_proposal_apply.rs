use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        proposal_apply::*,
        proposals::{
            CommentRequest, CommentTarget, NoteChange, ProposalDraft, ProposalEdit, ProposalRecord,
            ProposalState, ReviewComment, SourceVersion,
        },
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn fingerprint(text: &str, inode: u64) -> FileFingerprint {
    FileFingerprint {
        device: 1,
        inode,
        len: text.len() as u64,
        sha256: digest(text.as_bytes()),
    }
}

fn draft() -> ProposalDraft {
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    ProposalDraft {
        id: Uuid::new_v4(),
        group_id: Some(Uuid::new_v4()),
        session_id: Some(Uuid::new_v4()),
        vault: VaultRecord {
            id: Uuid::new_v4(),
            root: "/synthetic/vault".into(),
            identity: parent.clone(),
        },
        title: "Approve the exact full proposal".into(),
        changes: vec![
            NoteChange::Create {
                path: "notes/new.md".into(),
                parent: parent.clone(),
                text: "\u{feff}新しい 🦀\r\n".into(),
            },
            NoteChange::Replace {
                path: "notes/existing.md".into(),
                parent: parent.clone(),
                before: fingerprint("\u{feff}старое\r\n", 2),
                before_text: "\u{feff}старое\r\n".into(),
                text: "\u{feff}новое\r\n".into(),
            },
            NoteChange::Trash {
                path: "notes/trash.md".into(),
                parent,
                before: fingerprint("archived 日本語\r\n", 3),
                before_text: "archived 日本語\r\n".into(),
            },
        ],
        sources: vec![SourceVersion {
            path: "sources/evidence.md".into(),
            fingerprint: fingerprint("source", 4),
        }],
    }
}

fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir().unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}

fn reviewed(store: &mut WorkStore) -> ProposalRecord {
    let record = store.create_proposal(&draft()).unwrap();
    store
        .add_proposal_comment(&CommentRequest {
            expected: record.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Keep this review until success 🦀\r\n".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap()
}

fn approval(record: &ProposalRecord) -> ApprovalRequest {
    ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    }
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
    }
}

fn prepared(journal: &ApplyJournal) -> Vec<FileFingerprint> {
    journal
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
        })
        .collect()
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
            },
            // Bounded unexpected staging is observation only, not ownership proof.
            staging: Some(fingerprint("unexpected occupant", 900 + index as u64)),
        })
        .collect()
}

fn raw(dir: &Path) -> Connection {
    Connection::open(dir.join("brn.sqlite")).unwrap()
}

fn refused_then_revised(store: &mut WorkStore) -> (ApplyJournal, ProposalRecord) {
    let first = reviewed(store);
    let journal = store.begin_proposal_apply(&approval(&first)).unwrap();
    let journal = store
        .record_proposal_prepared(journal.request.operation_id, &prepared(&journal))
        .unwrap();
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&unchanged(&journal)),
        )
        .unwrap();
    let refused_journal = store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap();
    assert!(!refused_journal.approved.comments.is_empty());
    let refused = store.proposal(first.draft.id).unwrap().unwrap();
    let mut change = edit(&refused);
    change.texts[0] = Some("Revised exact review 日本語\r\n".into());
    let revised = store.edit_proposal(&change).unwrap();
    let revised = store
        .add_proposal_comment(&CommentRequest {
            expected: revised.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "New temporary comment 🦀\r\n".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    (refused_journal, revised)
}

#[test]
fn successful_reapproval_removes_all_journal_comments_preserving_old_receipts_and_proofs() {
    let (dir, mut store) = fixture();
    let (prior, revised) = refused_then_revised(&mut store);
    let other = reviewed(&mut store);
    let other_journal = store.begin_proposal_apply(&approval(&other)).unwrap();
    store
        .finish_proposal_apply(
            other_journal.request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&unchanged(&other_journal)),
        )
        .unwrap();
    let other_journal = store
        .proposal_apply(other_journal.request.operation_id)
        .unwrap()
        .unwrap();
    let current = store.begin_proposal_apply(&approval(&revised)).unwrap();
    let current = store
        .record_proposal_prepared(current.request.operation_id, &prepared(&current))
        .unwrap();
    let observations = applied(&current);
    let receipt = store
        .finish_proposal_apply(
            current.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observations),
        )
        .unwrap();
    let live = store.proposal(revised.draft.id).unwrap().unwrap();
    assert_eq!(live.stamp(), receipt.stamp);
    assert!(live.comments.is_empty());

    let mut expected_prior = prior.clone();
    expected_prior.approved.comments.clear();
    assert_eq!(
        store.proposal_apply(prior.request.operation_id).unwrap(),
        Some(expected_prior.clone())
    );
    let mut expected_current = current;
    expected_current.approved.comments.clear();
    expected_current.receipt = Some(receipt);
    expected_current.observations = Some(observations);
    assert_eq!(
        store
            .proposal_apply(expected_current.request.operation_id)
            .unwrap(),
        Some(expected_current.clone())
    );
    // Other proposals' refused review annotations remain available.
    assert_eq!(
        store
            .proposal_apply(other_journal.request.operation_id)
            .unwrap(),
        Some(other_journal)
    );
    assert_eq!(
        store.proposal(other.draft.id).unwrap().unwrap().comments,
        other.comments
    );
    assert_eq!(
        store
            .finish_proposal_apply(
                prior.request.operation_id,
                ApplyOutcome::NotApplied,
                prior.observations.as_deref()
            )
            .unwrap(),
        prior.receipt.clone().unwrap()
    );
    assert_eq!(
        store.begin_proposal_apply(&prior.request).unwrap(),
        expected_prior
    );
    assert_eq!(
        store.proposal(revised.draft.id).unwrap(),
        Some(live.clone())
    );
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.proposal_apply(prior.request.operation_id).unwrap(),
        Some(expected_prior)
    );
    assert_eq!(
        store
            .proposal_apply(expected_current.request.operation_id)
            .unwrap(),
        Some(expected_current)
    );
    assert_eq!(
        store
            .finish_proposal_apply(
                prior.request.operation_id,
                ApplyOutcome::NotApplied,
                prior.observations.as_deref()
            )
            .unwrap(),
        prior.receipt.clone().unwrap()
    );
    assert_eq!(store.proposal(revised.draft.id).unwrap(), Some(live));

    // A prior NotApplied journal alone permits its frozen review comments, but
    // storage must reject them once this proposal has actually been approved.
    let bad_prior = prior.clone();
    bad_prior.validate().unwrap();
    mutate_json(
        &raw(dir.path()),
        "proposal_applies",
        prior.request.operation_id,
        |value| {
            value["approved"]["comments"] = serde_json::to_value(&prior.approved.comments).unwrap();
        },
        true,
    );
    assert!(matches!(
        store.proposal_apply(prior.request.operation_id),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(store.proposal_applies(), Err(Error::Invalid(_))));
}

#[test]
fn failed_finalization_rolls_back_comment_cleanup_in_prior_and_current_journals() {
    let (dir, mut store) = fixture();
    let (prior, revised) = refused_then_revised(&mut store);
    let current = store.begin_proposal_apply(&approval(&revised)).unwrap();
    let current = store
        .record_proposal_prepared(current.request.operation_id, &prepared(&current))
        .unwrap();
    store
        .finish_proposal_apply(current.request.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    let uncertain = store
        .proposal_apply(current.request.operation_id)
        .unwrap()
        .unwrap();
    let live = store.proposal(revised.draft.id).unwrap().unwrap();
    assert_eq!(live.state, ProposalState::Uncertain);
    let conn = raw(dir.path());
    for (table, condition) in [
        (
            "proposal_applies",
            format!("OLD.operation_id='{}'", current.request.operation_id),
        ),
        (
            "proposal_applies",
            format!("OLD.operation_id='{}'", prior.request.operation_id),
        ),
        ("proposals", format!("OLD.id='{}'", revised.draft.id)),
    ] {
        conn.execute_batch(&format!("CREATE TRIGGER fail_cleanup BEFORE UPDATE ON {table} WHEN {condition} BEGIN SELECT RAISE(ABORT, 'synthetic'); END;")).unwrap();
        assert!(matches!(
            store.finish_proposal_apply(
                current.request.operation_id,
                ApplyOutcome::Applied,
                Some(&applied(&current))
            ),
            Err(Error::Sql(_))
        ));
        assert_eq!(
            store.proposal_apply(prior.request.operation_id).unwrap(),
            Some(prior.clone())
        );
        assert_eq!(
            store.proposal_apply(current.request.operation_id).unwrap(),
            Some(uncertain.clone())
        );
        assert_eq!(
            store.proposal(revised.draft.id).unwrap(),
            Some(live.clone())
        );
        conn.execute_batch("DROP TRIGGER fail_cleanup;").unwrap();
    }
    store
        .finish_proposal_apply(
            current.request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&current)),
        )
        .unwrap();
    assert!(
        store
            .proposal(revised.draft.id)
            .unwrap()
            .unwrap()
            .comments
            .is_empty()
    );
    assert!(
        store
            .proposal_apply(prior.request.operation_id)
            .unwrap()
            .unwrap()
            .approved
            .comments
            .is_empty()
    );
    assert!(
        store
            .proposal_apply(current.request.operation_id)
            .unwrap()
            .unwrap()
            .approved
            .comments
            .is_empty()
    );
}

#[test]
fn rehashed_applied_journal_with_review_comments_is_rejected() {
    let (dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
    let journal = store
        .record_proposal_prepared(journal.request.operation_id, &prepared(&journal))
        .unwrap();
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&journal)),
        )
        .unwrap();
    let mut corrupted = store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap();
    corrupted.approved.comments = record.comments.clone();
    assert!(matches!(corrupted.validate(), Err(Error::Invalid(_))));
    mutate_json(
        &raw(dir.path()),
        "proposal_applies",
        journal.request.operation_id,
        |value| {
            value["approved"]["comments"] = serde_json::to_value(&record.comments).unwrap();
        },
        true,
    );
    assert!(matches!(
        store.proposal_apply(journal.request.operation_id),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn approval_freezes_exact_review_and_complete_application_survives_restart_replay() {
    let (dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let request = approval(&record);
    let journal = store.begin_proposal_apply(&request).unwrap();
    journal.validate().unwrap();
    assert_eq!(journal.approved, record);
    assert_eq!(journal.members.len(), 3);
    for (member, change) in journal.members.iter().zip(&record.draft.changes) {
        assert!(!member.id.is_nil());
        assert_eq!(
            member.staging,
            Path::new(change.path()).with_file_name(format!(".brn-{}.stage", member.id))
        );
    }
    let admitted = store.proposal(record.draft.id).unwrap().unwrap();
    assert_eq!(admitted.state, ProposalState::Applying);
    assert_eq!(admitted.version, record.version + 1);
    assert_eq!(admitted.comments, record.comments);
    assert_eq!(store.begin_proposal_apply(&request).unwrap(), journal);
    let prepared = store
        .record_proposal_prepared(request.operation_id, &prepared(&journal))
        .unwrap();
    assert_eq!(prepared.approved, record);
    assert_eq!(
        store
            .record_proposal_prepared(request.operation_id, prepared.prepared.as_ref().unwrap())
            .unwrap(),
        prepared
    );
    let observed = applied(&prepared);
    let receipt = store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&observed))
        .unwrap();
    assert_eq!(receipt.approved_version, record.version);
    assert_eq!(receipt.stamp.version, record.version + 2);
    assert_eq!(receipt.proposal_id, record.draft.id);
    let applied_record = store.proposal(record.draft.id).unwrap().unwrap();
    assert_eq!(applied_record.state, ProposalState::Applied);
    assert_eq!(applied_record.stamp(), receipt.stamp);
    assert!(applied_record.comments.is_empty());
    assert_eq!(applied_record.draft, record.draft);
    let settled = store.proposal_apply(request.operation_id).unwrap().unwrap();
    assert!(settled.approved.comments.is_empty());
    assert_eq!(settled.observations.as_ref(), Some(&observed));
    assert_eq!(store.proposal_applies().unwrap(), vec![settled.clone()]);
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(store.begin_proposal_apply(&request).unwrap(), settled);
    assert_eq!(
        store
            .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&observed))
            .unwrap(),
        receipt
    );
    assert_eq!(
        store.proposal(record.draft.id).unwrap(),
        Some(applied_record)
    );
    assert_eq!(
        store
            .record_proposal_prepared(request.operation_id, prepared.prepared.as_ref().unwrap())
            .unwrap()
            .receipt,
        Some(receipt)
    );
}

#[test]
fn admitted_approval_refuses_review_mutations_and_stale_or_conflicting_requests() {
    let (_dir, mut store) = fixture();
    let first = store.create_proposal(&draft()).unwrap();
    let stale_request = approval(&first);
    let mut changed = edit(&first);
    changed.title = "Manual newer review".into();
    let current = store.edit_proposal(&changed).unwrap();
    assert!(matches!(
        store.begin_proposal_apply(&stale_request),
        Err(Error::StateChanged(_))
    ));
    assert!(store.proposal_applies().unwrap().is_empty());
    let request = approval(&current);
    let journal = store.begin_proposal_apply(&request).unwrap();
    let admitted = store.proposal(current.draft.id).unwrap().unwrap();
    assert!(matches!(
        store.edit_proposal(&edit(&admitted)),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.rewrite_proposal(&edit(&current)),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.reject_proposal(admitted.stamp()),
        Err(Error::StateChanged(_))
    ));
    assert!(matches!(
        store.add_proposal_comment(&CommentRequest {
            expected: admitted.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "late comment".into(),
                target: CommentTarget::Proposal,
            }
        }),
        Err(Error::StateChanged(_))
    ));
    let mut conflict = request.clone();
    conflict.expected.version += 1;
    assert!(matches!(
        store.begin_proposal_apply(&conflict),
        Err(Error::OperationConflict(_))
    ));
    conflict.expected = store.create_proposal(&draft()).unwrap().stamp();
    assert!(matches!(
        store.begin_proposal_apply(&conflict),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(store.begin_proposal_apply(&request).unwrap(), journal);
    assert_eq!(store.proposal(current.draft.id).unwrap(), Some(admitted));
}

#[test]
fn only_one_unresolved_application_is_admitted_even_after_uncertainty_and_restart() {
    let (dir, mut store) = fixture();
    let first = reviewed(&mut store);
    let second = store.create_proposal(&draft()).unwrap();
    let request = approval(&first);
    let other = approval(&second);
    let journal = store.begin_proposal_apply(&request).unwrap();
    assert!(matches!(
        store.begin_proposal_apply(&other),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(
        store.proposal(second.draft.id).unwrap(),
        Some(second.clone())
    );
    let uncertain = store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    assert_eq!(uncertain.stamp.version, first.version + 2);
    assert!(matches!(
        store.begin_proposal_apply(&other),
        Err(Error::StateChanged(_))
    ));
    let conn = raw(dir.path());
    // The unique partial index is an independent admission backstop.
    assert!(conn.execute("INSERT INTO proposal_applies SELECT ?1,proposal_id,outcome,request_sha256,journal_json,journal_sha256 FROM proposal_applies WHERE operation_id=?2", params![Uuid::new_v4().to_string(), request.operation_id.to_string()]).is_err());
    drop(conn);
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert!(matches!(
        store.begin_proposal_apply(&other),
        Err(Error::StateChanged(_))
    ));
    let settled = store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&unchanged(&journal)),
        )
        .unwrap();
    assert_eq!(settled.stamp.version, first.version + 3);
    assert_eq!(
        store.proposal(first.draft.id).unwrap().unwrap().state,
        ProposalState::Draft
    );
    store.begin_proposal_apply(&other).unwrap();
}

#[test]
fn invalid_or_partial_preparations_are_atomic_and_complete_prepared_set_is_immutable() {
    let (_dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
    let expected = prepared(&journal);
    for defect in 0..7 {
        let mut bad = expected.clone();
        match defect {
            0 => {
                bad.pop();
            }
            1 => bad[0].sha256 = [0; 32],
            2 => bad[0].len += 1,
            3 => bad[1].inode = bad[0].inode,
            4 => bad[0].inode = 2,
            5 => bad[0].inode = 4,
            _ => bad[2].inode += 1,
        }
        assert!(
            matches!(
                store.record_proposal_prepared(journal.request.operation_id, &bad),
                Err(Error::Invalid(_))
            ),
            "defect {defect}"
        );
        assert_eq!(
            store.proposal_apply(journal.request.operation_id).unwrap(),
            Some(journal.clone())
        );
    }
    let recorded = store
        .record_proposal_prepared(journal.request.operation_id, &expected)
        .unwrap();
    let mut changed = expected.clone();
    changed[0].inode += 100;
    assert!(matches!(
        store.record_proposal_prepared(journal.request.operation_id, &changed),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(
        store
            .record_proposal_prepared(journal.request.operation_id, &expected)
            .unwrap(),
        recorded
    );
}

#[test]
fn applied_requires_all_exact_installed_and_retained_original_proofs() {
    let (_dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
    assert!(matches!(
        store.finish_proposal_apply(journal.request.operation_id, ApplyOutcome::Applied, None),
        Err(Error::Invalid(_))
    ));
    let mut with_prepared = journal.clone();
    with_prepared.prepared = Some(prepared(&journal));
    let expected = applied(&with_prepared);
    assert!(matches!(
        store.finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&expected)
        ),
        Err(Error::Invalid(_))
    ));
    let prepared = store
        .record_proposal_prepared(
            journal.request.operation_id,
            with_prepared.prepared.as_ref().unwrap(),
        )
        .unwrap();
    for defect in 0..7 {
        let mut bad = expected.clone();
        match defect {
            0 => {
                bad.pop();
            }
            1 => bad[0].destination = None,
            2 => bad[0].staging = Some(fingerprint("extra", 300)),
            3 => bad[1].staging = None,
            4 => bad[1].destination.as_mut().unwrap().inode += 1,
            5 => bad[2].destination = Some(prepared.prepared.as_ref().unwrap()[2].clone()),
            _ => bad[2].staging.as_mut().unwrap().sha256 = [0; 32],
        }
        assert!(
            matches!(
                store.finish_proposal_apply(
                    journal.request.operation_id,
                    ApplyOutcome::Applied,
                    Some(&bad)
                ),
                Err(Error::Invalid(_))
            ),
            "defect {defect}"
        );
        assert_eq!(
            store.proposal_apply(journal.request.operation_id).unwrap(),
            Some(prepared.clone())
        );
        assert_eq!(
            store.proposal(record.draft.id).unwrap().unwrap().state,
            ProposalState::Applying
        );
    }
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&expected),
        )
        .unwrap();
    let mut wrong = expected;
    wrong[0].destination.as_mut().unwrap().inode += 1;
    assert!(matches!(
        store.finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&wrong)
        ),
        Err(Error::OperationConflict(_))
    ));
    assert!(matches!(
        store.finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&unchanged(&journal))
        ),
        Err(Error::OperationConflict(_))
    ));
}

#[test]
fn refusal_preserves_comments_and_replay_does_not_clobber_later_draft_edits() {
    let (_dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let request = approval(&record);
    let journal = store.begin_proposal_apply(&request).unwrap();
    let observations = unchanged(&journal);
    assert!(matches!(
        store.finish_proposal_apply(request.operation_id, ApplyOutcome::NotApplied, None),
        Err(Error::Invalid(_))
    ));
    let mut occupied = observations.clone();
    occupied[0].destination = Some(fingerprint("occupant", 500));
    assert!(matches!(
        store.finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&occupied)
        ),
        Err(Error::Invalid(_))
    ));
    let mut replaced = observations.clone();
    replaced[1].destination.as_mut().unwrap().inode += 1;
    assert!(matches!(
        store.finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&replaced)
        ),
        Err(Error::Invalid(_))
    ));
    let receipt = store
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&observations),
        )
        .unwrap();
    let refused = store.proposal(record.draft.id).unwrap().unwrap();
    assert_eq!(refused.comments, record.comments);
    assert_eq!(refused.state, ProposalState::Draft);
    assert_eq!(refused.version, record.version + 2);
    assert!(matches!(
        store.record_proposal_prepared(request.operation_id, &prepared(&journal)),
        Err(Error::StateChanged(_))
    ));
    let mut user_edit = edit(&refused);
    user_edit.texts[0] = Some("Later exact user bytes\r\n日本語".into());
    let edited = store.edit_proposal(&user_edit).unwrap();
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
    let replayed = store.begin_proposal_apply(&request).unwrap();
    assert_eq!(replayed.approved, record);
    assert_eq!(replayed.receipt, Some(receipt));
    assert_eq!(
        store.proposal(record.draft.id).unwrap(),
        Some(edited.clone())
    );
    assert!(matches!(
        store.begin_proposal_apply(&ApprovalRequest {
            operation_id: request.operation_id,
            expected: edited.stamp()
        }),
        Err(Error::OperationConflict(_))
    ));
    store.begin_proposal_apply(&approval(&edited)).unwrap();
    assert_eq!(
        store
            .finish_proposal_apply(
                request.operation_id,
                ApplyOutcome::NotApplied,
                Some(&observations)
            )
            .unwrap()
            .approved_version,
        record.version
    );
}

#[test]
fn uncertain_work_can_record_missing_complete_preparations_and_explicitly_settle() {
    let (dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
    let mut partial = unchanged(&journal);
    partial.pop();
    assert!(matches!(
        store.finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Uncertain,
            Some(&partial)
        ),
        Err(Error::Invalid(_))
    ));
    let uncertain = store
        .finish_proposal_apply(journal.request.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    assert_eq!(
        store
            .finish_proposal_apply(journal.request.operation_id, ApplyOutcome::Uncertain, None)
            .unwrap(),
        uncertain
    );
    assert!(matches!(
        store.finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Uncertain,
            Some(&unchanged(&journal))
        ),
        Err(Error::OperationConflict(_))
    ));
    let current = store.proposal(record.draft.id).unwrap().unwrap();
    assert_eq!(current.state, ProposalState::Uncertain);
    assert_eq!(current.comments, record.comments);
    assert!(matches!(
        store.edit_proposal(&edit(&current)),
        Err(Error::StateChanged(_))
    ));
    let prepared = store
        .record_proposal_prepared(journal.request.operation_id, &prepared(&journal))
        .unwrap();
    assert_eq!(prepared.receipt, Some(uncertain.clone()));
    assert_eq!(store.proposal(record.draft.id).unwrap(), Some(current));
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.proposal_apply(journal.request.operation_id).unwrap(),
        Some(prepared.clone())
    );
    let receipt = store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&prepared)),
        )
        .unwrap();
    assert_eq!(receipt.stamp.version, record.version + 3);
    assert_eq!(
        store.proposal(record.draft.id).unwrap().unwrap().state,
        ProposalState::Applied
    );
}

#[test]
fn applying_and_settled_transactions_roll_back_both_review_and_journal() {
    let (dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let request = approval(&record);
    let conn = raw(dir.path());
    conn.execute_batch("CREATE TRIGGER fail_admission BEFORE INSERT ON proposal_applies BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").unwrap();
    assert!(matches!(
        store.begin_proposal_apply(&request),
        Err(Error::Sql(_))
    ));
    assert_eq!(
        store.proposal(record.draft.id).unwrap(),
        Some(record.clone())
    );
    assert!(store.proposal_applies().unwrap().is_empty());
    conn.execute_batch("DROP TRIGGER fail_admission;").unwrap();
    let journal = store.begin_proposal_apply(&request).unwrap();
    let prepared = store
        .record_proposal_prepared(request.operation_id, &prepared(&journal))
        .unwrap();
    let before = store.proposal(record.draft.id).unwrap().unwrap();
    for table in ["proposals", "proposal_applies"] {
        conn.execute_batch(&format!("CREATE TRIGGER fail_receipt BEFORE UPDATE ON {table} BEGIN SELECT RAISE(ABORT, 'synthetic'); END;")).unwrap();
        assert!(matches!(
            store.finish_proposal_apply(
                request.operation_id,
                ApplyOutcome::Applied,
                Some(&applied(&prepared))
            ),
            Err(Error::Sql(_))
        ));
        assert_eq!(
            store.proposal(record.draft.id).unwrap(),
            Some(before.clone())
        );
        assert_eq!(
            store.proposal_apply(request.operation_id).unwrap(),
            Some(prepared.clone())
        );
        conn.execute_batch("DROP TRIGGER fail_receipt;").unwrap();
    }
    store
        .finish_proposal_apply(request.operation_id, ApplyOutcome::Uncertain, None)
        .unwrap();
    let uncertain_journal = store.proposal_apply(request.operation_id).unwrap().unwrap();
    let uncertain_record = store.proposal(record.draft.id).unwrap().unwrap();
    conn.execute_batch("CREATE TRIGGER fail_receipt BEFORE UPDATE ON proposals BEGIN SELECT RAISE(ABORT, 'synthetic'); END;").unwrap();
    assert!(matches!(
        store.finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&prepared))
        ),
        Err(Error::Sql(_))
    ));
    assert_eq!(
        store.proposal(record.draft.id).unwrap(),
        Some(uncertain_record)
    );
    assert_eq!(
        store.proposal_apply(request.operation_id).unwrap(),
        Some(uncertain_journal)
    );
}

fn mutate_json(
    conn: &Connection,
    table: &str,
    key: Uuid,
    mutate: impl FnOnce(&mut serde_json::Value),
    rehash: bool,
) {
    let (column, key_column, digest_column) = if table == "proposals" {
        ("record_json", "id", "record_sha256")
    } else {
        ("journal_json", "operation_id", "journal_sha256")
    };
    let bytes: Vec<u8> = conn
        .query_row(
            &format!("SELECT {column} FROM {table} WHERE {key_column}=?1"),
            [key.to_string()],
            |row| row.get(0),
        )
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    mutate(&mut value);
    let bytes = serde_json::to_vec(&value).unwrap();
    let sql = if rehash {
        format!("UPDATE {table} SET {column}=?2,{digest_column}=?3 WHERE {key_column}=?1")
    } else {
        format!("UPDATE {table} SET {column}=?2 WHERE {key_column}=?1")
    };
    if rehash {
        conn.execute(
            &sql,
            params![key.to_string(), bytes, digest(&bytes).as_slice()],
        )
        .unwrap();
    } else {
        conn.execute(&sql, params![key.to_string(), bytes]).unwrap();
    }
}

#[test]
fn completion_refuses_changed_immutable_current_review_or_wrong_state_version() {
    for defect in 0..5 {
        let (dir, mut store) = fixture();
        let record = reviewed(&mut store);
        let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
        let conn = raw(dir.path());
        mutate_json(
            &conn,
            "proposals",
            record.draft.id,
            |value| match defect {
                0 => value["record"]["draft"]["title"] = "unexpected changed review".into(),
                1 => value["record"]["comments"][0]["text"] = "unexpected changed comment".into(),
                2 => value["record"]["version"] = (record.version + 2).into(),
                3 => value["record"]["state"] = "draft".into(),
                _ => value["record"]["created_at_ms"] = 0.into(),
            },
            true,
        );
        assert!(
            matches!(
                store.finish_proposal_apply(
                    journal.request.operation_id,
                    ApplyOutcome::NotApplied,
                    Some(&unchanged(&journal))
                ),
                Err(Error::StateChanged(_))
            ),
            "defect {defect}"
        );
        assert!(matches!(
            store.record_proposal_prepared(journal.request.operation_id, &prepared(&journal)),
            Err(Error::StateChanged(_))
        ));
        assert_eq!(
            store
                .proposal_apply(journal.request.operation_id)
                .unwrap()
                .unwrap()
                .receipt,
            None
        );
    }
}

#[test]
fn hashed_journal_row_bindings_domain_and_edited_creation_lineage_are_checked() {
    for defect in 0..12 {
        let (dir, mut store) = fixture();
        let record = reviewed(&mut store);
        let mut newer = edit(&record);
        newer.texts[0] = Some("Manually changed approved snapshot 🦀\r\n".into());
        let record = store.edit_proposal(&newer).unwrap();
        let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
        let other = store.create_proposal(&draft()).unwrap();
        let conn = raw(dir.path());
        match defect {
            0 => mutate_json(
                &conn,
                "proposal_applies",
                journal.request.operation_id,
                |value| value["started_at_ms"] = 0.into(),
                false,
            ),
            1 => {
                conn.execute(
                    "UPDATE proposal_applies SET proposal_id=?2 WHERE operation_id=?1",
                    params![
                        journal.request.operation_id.to_string(),
                        other.draft.id.to_string()
                    ],
                )
                .unwrap();
            }
            2 => {
                conn.execute(
                    "UPDATE proposal_applies SET outcome='not_applied' WHERE operation_id=?1",
                    [journal.request.operation_id.to_string()],
                )
                .unwrap();
            }
            3 => {
                conn.execute(
                    "UPDATE proposal_applies SET request_sha256=zeroblob(32) WHERE operation_id=?1",
                    [journal.request.operation_id.to_string()],
                )
                .unwrap();
            }
            _ => mutate_json(
                &conn,
                "proposal_applies",
                journal.request.operation_id,
                |value| match defect {
                    4 => value["members"][0]["staging"] = "other/.brn-unbound.stage".into(),
                    5 => value["members"][0]["id"] = Uuid::nil().to_string().into(),
                    6 => value["members"] = serde_json::json!([]),
                    7 => value["approved"]["state"] = "applying".into(),
                    8 => value["request"]["expected"]["version"] = (record.version + 1).into(),
                    9 => value["creation_sha256"] = serde_json::to_value([0_u8; 32]).unwrap(),
                    10 => value["started_at_ms"] = 0.into(),
                    _ => value["unrecognized_field"] = true.into(),
                },
                true,
            ),
        }
        assert!(
            matches!(
                store.proposal_apply(journal.request.operation_id),
                Err(Error::Invalid(_))
            ),
            "defect {defect}"
        );
        assert!(matches!(store.proposal_applies(), Err(Error::Invalid(_))));
        assert!(matches!(
            store.begin_proposal_apply(&journal.request),
            Err(Error::Invalid(_))
        ));
    }
}

#[test]
fn pure_mirror_validation_checks_member_receipt_proof_and_metadata_bounds() {
    let (_dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
    for defect in 0..6 {
        let mut invalid = journal.clone();
        match defect {
            0 => invalid.request.operation_id = Uuid::nil(),
            1 => invalid.members[1].id = invalid.members[0].id,
            2 => invalid.prepared = Some(vec![]),
            3 => invalid.observations = Some(unchanged(&journal)),
            4 => {
                invalid.approved.draft.changes[0] = NoteChange::Create {
                    path: "../note.md".into(),
                    parent: VaultIdentity {
                        device: 1,
                        inode: 1,
                    },
                    text: "".into(),
                }
            }
            _ => invalid.started_at_ms = 0,
        }
        assert!(matches!(invalid.validate(), Err(Error::Invalid(_))));
    }
    let prepared = store
        .record_proposal_prepared(journal.request.operation_id, &prepared(&journal))
        .unwrap();
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&prepared)),
        )
        .unwrap();
    let terminal = store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap();
    for defect in 0..5 {
        let mut invalid = terminal.clone();
        match defect {
            0 => invalid.receipt.as_mut().unwrap().approved_version += 1,
            1 => invalid.receipt.as_mut().unwrap().stamp.version += 2,
            2 => invalid.receipt.as_mut().unwrap().proposal_id = Uuid::new_v4(),
            3 => invalid.observations.as_mut().unwrap()[1].staging = None,
            _ => {
                invalid.observations.as_mut().unwrap()[0]
                    .destination
                    .as_mut()
                    .unwrap()
                    .len = brn_store::MAX_NOTE_BYTES as u64 + 1
            }
        }
        assert!(matches!(invalid.validate(), Err(Error::Invalid(_))));
    }
    let mut large_metadata = journal;
    if let NoteChange::Create { path, .. } = &mut large_metadata.approved.draft.changes[0] {
        *path = format!("{}/note.md", "x".repeat(300_000));
    }
    large_metadata.members[0].staging = Path::new(large_metadata.approved.draft.changes[0].path())
        .with_file_name(format!(".brn-{}.stage", large_metadata.members[0].id));
    assert!(matches!(large_metadata.validate(), Err(Error::Invalid(_))));
}

#[test]
fn oversized_encoded_journal_is_refused_before_json_loading() {
    let (dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
    raw(dir.path())
        .execute(
            "UPDATE proposal_applies SET journal_json=zeroblob(54000000) WHERE operation_id=?1",
            [journal.request.operation_id.to_string()],
        )
        .unwrap();
    assert!(
        matches!(store.proposal_apply(journal.request.operation_id), Err(Error::Invalid(message)) if message.contains("encoded size limit"))
    );
}

#[test]
fn applied_proposal_cannot_retain_comments_in_a_rehashed_record() {
    let (dir, mut store) = fixture();
    let record = reviewed(&mut store);
    let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
    let prepared = store
        .record_proposal_prepared(journal.request.operation_id, &prepared(&journal))
        .unwrap();
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&applied(&prepared)),
        )
        .unwrap();
    mutate_json(
        &raw(dir.path()),
        "proposals",
        record.draft.id,
        |value| {
            value["record"]["comments"] = serde_json::to_value(&record.comments).unwrap();
        },
        true,
    );
    assert!(matches!(
        store.proposal(record.draft.id),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        store.proposal_apply(journal.request.operation_id),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn admission_reserves_checked_versions_for_uncertainty_and_reconciliation() {
    let (dir, mut store) = fixture();
    let record = reviewed(&mut store);
    mutate_json(
        &raw(dir.path()),
        "proposals",
        record.draft.id,
        |value| {
            value["record"]["version"] = (u64::MAX - 2).into();
        },
        true,
    );
    let maximum = store.proposal(record.draft.id).unwrap().unwrap();
    assert!(matches!(
        store.begin_proposal_apply(&approval(&maximum)),
        Err(Error::Invalid(_))
    ));
    assert!(store.proposal_applies().unwrap().is_empty());
    assert_eq!(store.proposal(record.draft.id).unwrap(), Some(maximum));
}

#[test]
fn additive_v4_migration_and_backups_preserve_review_and_uncertain_application() {
    let (dir, mut store) = fixture();
    let record = reviewed(&mut store);
    store.set_setting("selection", "synthetic").unwrap();
    drop(store);
    let conn = raw(dir.path());
    conn.execute_batch("DROP TABLE proposal_applies; PRAGMA user_version=4;")
        .unwrap();
    drop(conn);
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    assert!(report.backup.is_file());
    assert_eq!(
        store.setting("selection").unwrap().as_deref(),
        Some("synthetic")
    );
    assert_eq!(
        store.proposal(record.draft.id).unwrap(),
        Some(record.clone())
    );
    assert!(store.proposal_applies().unwrap().is_empty());
    let journal = store.begin_proposal_apply(&approval(&record)).unwrap();
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Uncertain,
            Some(&unchanged(&journal)),
        )
        .unwrap();
    let uncertain = store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap();
    let uncertain_review = store.proposal(record.draft.id).unwrap().unwrap();
    drop(store);
    drop(WorkStore::open(dir.path()).unwrap()); // supported startup backup
    std::fs::write(dir.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    assert!(report.restored_from.is_some());
    assert_eq!(
        store.proposal_apply(journal.request.operation_id).unwrap(),
        Some(uncertain)
    );
    assert_eq!(
        store.proposal(record.draft.id).unwrap(),
        Some(uncertain_review)
    );
    let receipt = store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::NotApplied,
            Some(&unchanged(&journal)),
        )
        .unwrap();
    assert_eq!(receipt.stamp.version, record.version + 3);
    assert_eq!(
        raw(dir.path())
            .query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        5
    );
}
