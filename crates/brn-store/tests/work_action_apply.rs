use brn_store::{
    Error, WorkStore,
    files::{FileFingerprint, VaultIdentity, VaultRecord},
    work::{
        action_completion::CompleteActionRequest,
        actions::{ActionData, ActionRecord, ActionState},
        proposal_apply::{
            ApplyJournal, ApplyMemberProof, ApplyOutcome, ApprovalRequest, UndoBinding, UndoRequest,
        },
        proposals::{
            ActionChange, CommentRequest, CommentTarget, NoteChange, ProposalDraft, ProposalEdit,
            ProposalRecord, ProposalState, ReviewComment,
        },
    },
};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

fn hash(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
fn fixture() -> (tempfile::TempDir, WorkStore) {
    let dir = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let (store, _) = WorkStore::open(dir.path()).unwrap();
    (dir, store)
}
fn data(state: ActionState) -> ActionData {
    ActionData {
        title: "Küsi tähtaega λ\r\n".into(),
        description: "Exact English ja eesti\r\n".into(),
        state,
        owner: Some("Anna Õun".into()),
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: None,
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: None,
    }
}
fn draft(changes: Vec<ActionChange>) -> ProposalDraft {
    ProposalDraft {
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: None,
        title: "Exact Action application".into(),
        changes: vec![],
        sources: vec![],
        action_changes: changes,
    }
}
fn review(store: &mut WorkStore, draft: &ProposalDraft) -> ProposalRecord {
    let record = store.create_proposal(draft).unwrap();
    store
        .add_proposal_comment(&CommentRequest {
            expected: record.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Temporary exact review\r\n".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap()
}
fn admit(store: &mut WorkStore, record: &ProposalRecord) -> ApplyJournal {
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: record.stamp(),
        })
        .unwrap()
}
fn prepared(store: &mut WorkStore, journal: &ApplyJournal) -> ApplyJournal {
    let proofs = journal
        .approved
        .draft
        .changes
        .iter()
        .enumerate()
        .map(|(i, change)| match change {
            NoteChange::Create { text, .. } | NoteChange::Replace { text, .. } => FileFingerprint {
                device: 1,
                inode: i as u64 + 100,
                len: text.len() as u64,
                sha256: hash(text.as_bytes()),
            },
            NoteChange::Trash { before, .. } => before.clone(),

            _ => unreachable!("Markdown-only fixture"),
        })
        .collect::<Vec<_>>();
    store
        .record_proposal_prepared(journal.request.operation_id, &proofs)
        .unwrap()
}
fn observations(journal: &ApplyJournal) -> Vec<ApplyMemberProof> {
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
fn finish(store: &mut WorkStore, journal: &ApplyJournal) -> ApplyJournal {
    let journal = prepared(store, journal);
    store
        .finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observations(&journal)),
        )
        .unwrap();
    store
        .proposal_apply(journal.request.operation_id)
        .unwrap()
        .unwrap()
}
fn create(store: &mut WorkStore, state: ActionState) -> (ActionRecord, ApplyJournal) {
    let id = Uuid::new_v4();
    let record = review(
        store,
        &draft(vec![ActionChange::Create {
            id,
            data: data(state),
        }]),
    );
    let journal = admit(store, &record);
    let journal = finish(store, &journal);
    (store.action(id).unwrap().unwrap(), journal)
}
fn replace(
    store: &mut WorkStore,
    before: &ActionRecord,
    state: ActionState,
) -> (ActionRecord, ApplyJournal) {
    let mut candidate = before.data.clone();
    candidate.state = state;
    candidate.description.push('λ');
    let record = review(
        store,
        &draft(vec![ActionChange::Replace {
            before: Box::new(before.clone()),
            data: candidate,
        }]),
    );
    let journal = admit(store, &record);
    let journal = finish(store, &journal);
    (store.action(before.origin.id).unwrap().unwrap(), journal)
}
fn raw(dir: &Path) -> Connection {
    Connection::open(dir.join("brn.sqlite")).unwrap()
}
// Test-only external row replacement simulates concurrent work and recovery forks.
fn put(dir: &Path, record: &ActionRecord) {
    record.validate().unwrap();
    let bytes = serde_json::to_vec(record).unwrap();
    let origin = serde_json::to_vec(&record.origin).unwrap();
    let state = serde_json::to_value(record.data.state)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned();
    raw(dir).execute("INSERT OR REPLACE INTO actions(id,version,state,created_at_ms,creation_sha256,record_json,record_sha256) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![record.origin.id.to_string(), record.version as i64, state, record.origin.created_at_ms as i64, hash(&origin).as_slice(), bytes, hash(&bytes).as_slice()]).unwrap();
}
fn with_note(draft: &mut ProposalDraft, text: String) {
    let parent = VaultIdentity {
        device: 1,
        inode: 1,
    };
    draft.vault = Some(VaultRecord {
        id: Uuid::new_v4(),
        root: "/synthetic/vault".into(),
        identity: parent.clone(),
    });
    draft.changes.push(NoteChange::Create {
        path: "exact.md".into(),
        parent,
        text,
    });
}

#[test]
fn joined_action_only_and_mixed_application_is_exact_atomic_and_replayed() {
    for mixed in [false, true] {
        let (dir, mut store) = fixture();
        let mut draft = draft(vec![ActionChange::Create {
            id: Uuid::new_v4(),
            data: data(ActionState::Waiting),
        }]);
        if mixed {
            with_note(&mut draft, "\u{feff}Täpne\r\n".into());
        }
        let review = review(&mut store, &draft);
        let admitted = admit(&mut store, &review);
        assert_eq!(admitted.action_records.len(), 1);
        let after = &admitted.action_records[0];
        assert_eq!(after.origin.id, draft.action_changes[0].id());
        assert_eq!(after.origin.proposal, review.stamp());
        assert_eq!(after.origin.data, *draft.action_changes[0].data());
        assert_eq!(after.origin.created_at_ms, admitted.started_at_ms);
        assert_eq!(after.updated_at_ms, admitted.started_at_ms);
        assert_eq!(after.waiting_since_ms, Some(admitted.started_at_ms));
        assert_eq!(after.version, 1);
        assert_eq!(store.action(after.origin.id).unwrap(), None);
        let settled = finish(&mut store, &admitted);
        assert_eq!(store.action(after.origin.id).unwrap(), Some(after.clone()));
        assert!(settled.approved.comments.is_empty());
        let applied = store.proposal(review.draft.id).unwrap().unwrap();
        assert_eq!(applied.state, ProposalState::Applied);
        assert!(applied.comments.is_empty());
        drop(store);
        let (mut store, _) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(
            store.begin_proposal_apply(&admitted.request).unwrap(),
            settled
        );
        assert_eq!(
            store
                .finish_proposal_apply(
                    settled.request.operation_id,
                    ApplyOutcome::Applied,
                    Some(&observations(&settled))
                )
                .unwrap(),
            settled.receipt.unwrap()
        );
        assert_eq!(store.action(after.origin.id).unwrap(), Some(after.clone()));
        assert!(!dir.path().join("credentials").exists());
    }
}

#[test]
fn replace_preserves_origin_waiting_and_clamps_full_before_clock() {
    let (dir, mut store) = fixture();
    let (initial, _) = create(&mut store, ActionState::Waiting);
    let mut future = initial.clone();
    future.version = 7;
    future.updated_at_ms = i64::MAX as u64 - 100;
    put(dir.path(), &future);
    let (waiting, journal) = replace(&mut store, &future, ActionState::Waiting);
    assert_eq!(waiting.version, 8);
    assert_eq!(waiting.origin, initial.origin);
    assert_eq!(waiting.updated_at_ms, future.updated_at_ms);
    assert_eq!(waiting.waiting_since_ms, initial.waiting_since_ms);
    assert_eq!(journal.started_at_ms, future.updated_at_ms);
    let (open, _) = replace(&mut store, &waiting, ActionState::Open);
    assert_eq!(open.waiting_since_ms, None);
    let (waiting_again, _) = replace(&mut store, &open, ActionState::Waiting);
    assert_eq!(
        waiting_again.waiting_since_ms,
        Some(waiting_again.updated_at_ms)
    );
    let mut overflow = waiting_again.clone();
    overflow.version = i64::MAX as u64;
    put(dir.path(), &overflow);
    let record = review(
        &mut store,
        &draft(vec![ActionChange::Replace {
            before: Box::new(overflow),
            data: data(ActionState::Open),
        }]),
    );
    assert!(
        store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: record.stamp()
            })
            .is_err()
    );
    assert_eq!(store.proposal(record.draft.id).unwrap(), Some(record));
}

#[test]
fn missing_and_reversed_action_history_import_preserve_latest_work() {
    let (_, mut source) = fixture();
    let (first, created) = create(&mut source, ActionState::Waiting);
    let (second, replaced) = replace(&mut source, &first, ActionState::Blocked);
    let (third, latest) = replace(&mut source, &second, ActionState::Open);
    for order in [
        [&created, &replaced, &latest],
        [&latest, &created, &replaced],
        [&replaced, &latest, &created],
    ] {
        let (dir, mut restored) = fixture();
        for journal in order {
            restored.restore_proposal_apply(journal).unwrap();
        }
        assert_eq!(
            restored.action(first.origin.id).unwrap(),
            Some(third.clone())
        );
        drop(restored);
        let (restored, _) = WorkStore::open(dir.path()).unwrap();
        assert_eq!(
            restored.action(first.origin.id).unwrap(),
            Some(third.clone())
        );
    }
}

#[test]
fn admission_and_settlement_compare_full_action_records_before_any_commit() {
    let (dir, mut store) = fixture();
    let (first, _) = create(&mut store, ActionState::Open);
    let existing = review(
        &mut store,
        &draft(vec![ActionChange::Create {
            id: first.origin.id,
            data: data(ActionState::Open),
        }]),
    );
    assert!(matches!(
        store.begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: existing.stamp()
        }),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(store.proposal(existing.draft.id).unwrap(), Some(existing));
    let mut missing = first.clone();
    missing.origin.id = Uuid::new_v4();
    let missing = review(
        &mut store,
        &draft(vec![ActionChange::Replace {
            before: Box::new(missing),
            data: data(ActionState::Blocked),
        }]),
    );
    assert!(
        store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: missing.stamp()
            })
            .is_err()
    );
    let review = review(
        &mut store,
        &draft(vec![
            ActionChange::Create {
                id: Uuid::new_v4(),
                data: data(ActionState::Waiting),
            },
            ActionChange::Replace {
                before: Box::new(first.clone()),
                data: data(ActionState::Blocked),
            },
        ]),
    );
    let journal = admit(&mut store, &review);
    let journal = prepared(&mut store, &journal);
    let applying = store.proposal(review.draft.id).unwrap().unwrap();
    let mut competing = first.clone();
    competing.version = 2;
    competing.data.description.push_str("other exact data");
    put(dir.path(), &competing);
    assert!(matches!(
        store.finish_proposal_apply(
            journal.request.operation_id,
            ApplyOutcome::Applied,
            Some(&observations(&journal))
        ),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(
        store.proposal_apply(journal.request.operation_id).unwrap(),
        Some(journal.clone())
    );
    assert_eq!(store.proposal(review.draft.id).unwrap(), Some(applying));
    assert_eq!(store.action(first.origin.id).unwrap(), Some(competing));
    assert_eq!(
        store.action(journal.action_records[0].origin.id).unwrap(),
        None
    );
}

#[test]
fn nonapplied_and_uncertain_receipts_never_mutate_actions() {
    for outcome in [ApplyOutcome::NotApplied, ApplyOutcome::Uncertain] {
        let (_, mut store) = fixture();
        let (before, _) = create(&mut store, ActionState::Open);
        let record = review(
            &mut store,
            &draft(vec![
                ActionChange::Create {
                    id: Uuid::new_v4(),
                    data: data(ActionState::Open),
                },
                ActionChange::Replace {
                    before: Box::new(before.clone()),
                    data: data(ActionState::Blocked),
                },
            ]),
        );
        let journal = admit(&mut store, &record);
        store
            .finish_proposal_apply(journal.request.operation_id, outcome, Some(&[]))
            .unwrap();
        assert_eq!(store.action(before.origin.id).unwrap(), Some(before));
        assert_eq!(
            store.action(journal.action_records[0].origin.id).unwrap(),
            None
        );
        assert!(
            !store
                .proposal(record.draft.id)
                .unwrap()
                .unwrap()
                .comments
                .is_empty()
        );
        if outcome == ApplyOutcome::Uncertain {
            let settled = finish(&mut store, &journal);
            assert_eq!(settled.receipt.unwrap().outcome, ApplyOutcome::Applied);
        }
    }
}

#[test]
fn action_writes_receipt_review_and_prior_comment_cleanup_roll_back_together() {
    for failed_table in ["proposal_applies", "proposals"] {
        let (dir, mut store) = fixture();
        let (before, _) = create(&mut store, ActionState::Waiting);
        let draft = draft(vec![
            ActionChange::Create {
                id: Uuid::new_v4(),
                data: data(ActionState::Open),
            },
            ActionChange::Replace {
                before: Box::new(before.clone()),
                data: data(ActionState::Blocked),
            },
        ]);
        let record = review(&mut store, &draft);
        let prior = admit(&mut store, &record);
        store
            .refuse_proposal_before_effects(prior.request.operation_id, None)
            .unwrap();
        let refused = store.proposal(draft.id).unwrap().unwrap();
        let next = store
            .edit_proposal(&ProposalEdit {
                expected: refused.stamp(),
                title: "Revised".into(),
                texts: vec![],
                action_data: refused
                    .draft
                    .action_changes
                    .iter()
                    .map(|a| a.data().clone())
                    .collect(),
            })
            .unwrap();
        let next = store
            .add_proposal_comment(&CommentRequest {
                expected: next.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "New review".into(),
                    target: CommentTarget::Proposal,
                },
            })
            .unwrap();
        let journal = admit(&mut store, &next);
        let journal = prepared(&mut store, &journal);
        let applying = store.proposal(draft.id).unwrap().unwrap();
        let prior = store
            .proposal_apply(prior.request.operation_id)
            .unwrap()
            .unwrap();
        raw(dir.path()).execute_batch(&format!("CREATE TRIGGER fail_settlement BEFORE UPDATE ON {failed_table} BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;")).unwrap();
        assert!(
            store
                .finish_proposal_apply(
                    journal.request.operation_id,
                    ApplyOutcome::Applied,
                    Some(&[])
                )
                .is_err()
        );
        assert_eq!(
            store.action(before.origin.id).unwrap(),
            Some(before.clone())
        );
        assert_eq!(
            store.action(journal.action_records[0].origin.id).unwrap(),
            None
        );
        assert_eq!(
            store.proposal_apply(journal.request.operation_id).unwrap(),
            Some(journal.clone())
        );
        assert_eq!(
            store.proposal_apply(prior.request.operation_id).unwrap(),
            Some(prior.clone())
        );
        assert_eq!(store.proposal(draft.id).unwrap(), Some(applying));
        raw(dir.path())
            .execute_batch("DROP TRIGGER fail_settlement;")
            .unwrap();
        let final_journal = finish(&mut store, &journal);
        assert_eq!(
            store.action(before.origin.id).unwrap(),
            Some(journal.action_records[1].clone())
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
        assert!(final_journal.approved.comments.is_empty());
    }
}

#[test]
fn earlier_replace_snapshots_restore_real_baseline_and_preserve_newer_review_edits() {
    let (_, mut source) = fixture();
    let (before, _) = create(&mut source, ActionState::Waiting);
    let mut candidate = before.data.clone();
    candidate.state = ActionState::Blocked;
    let record = review(
        &mut source,
        &draft(vec![
            ActionChange::Create {
                id: Uuid::new_v4(),
                data: data(ActionState::Open),
            },
            ActionChange::Replace {
                before: Box::new(before.clone()),
                data: candidate,
            },
        ]),
    );
    let pending = admit(&mut source, &record);
    let (_, mut target) = fixture();
    target.restore_proposal_apply(&pending).unwrap();
    assert_eq!(
        target.action(before.origin.id).unwrap(),
        Some(before.clone())
    );
    assert_eq!(
        target.action(pending.action_records[0].origin.id).unwrap(),
        None
    );
    source
        .refuse_proposal_before_effects(pending.request.operation_id, None)
        .unwrap();
    let terminal = source
        .proposal_apply(pending.request.operation_id)
        .unwrap()
        .unwrap();
    let (_, mut target) = fixture();
    target.restore_proposal_apply(&terminal).unwrap();
    let record = target.proposal(record.draft.id).unwrap().unwrap();
    let mut data = record
        .draft
        .action_changes
        .iter()
        .map(|a| a.data().clone())
        .collect::<Vec<_>>();
    data[0].description.push_str("User's later unapproved text");
    let edited = target
        .edit_proposal(&ProposalEdit {
            expected: record.stamp(),
            title: "Later title".into(),
            texts: vec![],
            action_data: data,
        })
        .unwrap();
    target.restore_proposal_apply(&terminal).unwrap();
    assert_eq!(target.proposal(edited.draft.id).unwrap(), Some(edited));
    assert_eq!(target.action(before.origin.id).unwrap(), Some(before));
}

#[test]
fn recovery_forks_terminal_lineage_and_late_failure_roll_back_every_import() {
    let (_, mut source) = fixture();
    let (first, created) = create(&mut source, ActionState::Open);
    let (second, _) = replace(&mut source, &first, ActionState::Waiting);
    let new_id = Uuid::new_v4();
    let record = review(
        &mut source,
        &draft(vec![
            ActionChange::Create {
                id: new_id,
                data: data(ActionState::Open),
            },
            ActionChange::Replace {
                before: Box::new(second.clone()),
                data: data(ActionState::Blocked),
            },
        ]),
    );
    let journal = admit(&mut source, &record);
    let latest = finish(&mut source, &journal);
    for conflicting in [0, 1, 2, 3] {
        let (dir, mut target) = fixture();
        let mut live = latest.action_records[1].clone();
        match conflicting {
            0 => live.data.description.push_str("same-version fork"),
            1 => {
                live.origin.proposal.id = Uuid::new_v4();
            }
            2 => {
                live.version = second.version;
                live.data.state = ActionState::Completed;
                live.waiting_since_ms = None;
                live.completed_at_ms = Some(live.updated_at_ms);
            }
            3 => {
                live = second.clone();
                live.data
                    .description
                    .push_str("fork at the retained before revision");
            }
            _ => unreachable!(),
        }
        put(dir.path(), &live);
        assert!(matches!(
            target.restore_proposal_apply(&latest),
            Err(Error::OperationConflict(_))
        ));
        assert_eq!(target.action(first.origin.id).unwrap(), Some(live));
        assert_eq!(target.action(new_id).unwrap(), None);
        assert_eq!(target.proposal(latest.approved.draft.id).unwrap(), None);
        assert_eq!(
            target.proposal_apply(latest.request.operation_id).unwrap(),
            None
        );
    }
    let (dir, mut target) = fixture();
    let mut completed = latest.action_records[1].clone();
    completed.version = 9;
    completed.data.state = ActionState::Completed;
    completed.waiting_since_ms = None;
    completed.completed_at_ms = Some(completed.updated_at_ms);
    put(dir.path(), &completed);
    target.restore_proposal_apply(&latest).unwrap();
    target.restore_proposal_apply(&created).unwrap();
    assert_eq!(target.action(first.origin.id).unwrap(), Some(completed));

    // Completion wins even when its revision is below both known endpoints of
    // the incoming unfinished application, rather than merely tying before.
    let (later_before, _) = replace(&mut source, &latest.action_records[1], ActionState::Waiting);
    assert_eq!(later_before.version, 4);
    let record = review(
        &mut source,
        &draft(vec![ActionChange::Replace {
            before: Box::new(later_before),
            data: data(ActionState::Open),
        }]),
    );
    let later = admit(&mut source, &record);
    let later = finish(&mut source, &later);
    let (dir, mut target) = fixture();
    let mut earlier_completion = second;
    earlier_completion.data.state = ActionState::Completed;
    earlier_completion.waiting_since_ms = None;
    earlier_completion.completed_at_ms = Some(earlier_completion.updated_at_ms);
    put(dir.path(), &earlier_completion);
    assert!(matches!(
        target.restore_proposal_apply(&later),
        Err(Error::OperationConflict(_))
    ));
    assert_eq!(
        target.action(first.origin.id).unwrap(),
        Some(earlier_completion)
    );
    assert_eq!(
        target.proposal_apply(later.request.operation_id).unwrap(),
        None
    );
}

#[test]
fn exact_snapshot_bindings_order_and_action_undo_are_strict() {
    let (_, mut store) = fixture();
    let record = review(
        &mut store,
        &draft(vec![
            ActionChange::Create {
                id: Uuid::new_v4(),
                data: data(ActionState::Open),
            },
            ActionChange::Create {
                id: Uuid::new_v4(),
                data: data(ActionState::Waiting),
            },
        ]),
    );
    let journal = admit(&mut store, &record);
    for mutation in 0..6 {
        let mut bad = journal.clone();
        match mutation {
            0 => bad.action_records.clear(),
            1 => bad.action_records.swap(0, 1),
            2 => bad.action_records[0].origin.proposal.version += 1,
            3 => bad.action_records[0].updated_at_ms += 1,
            4 => bad.action_records[0].data.description.push('x'),
            5 => bad.action_records[0].origin.id = Uuid::new_v4(),
            _ => unreachable!(),
        }
        assert!(bad.validate().is_err());
        assert!(store.restore_proposal_apply(&bad).is_err());
        assert_eq!(
            store.proposal_apply(journal.request.operation_id).unwrap(),
            Some(journal.clone())
        );
    }
    let settled = finish(&mut store, &journal);
    for mixed in [false, true] {
        let target = if mixed {
            let mut draft = draft(vec![ActionChange::Create {
                id: Uuid::new_v4(),
                data: data(ActionState::Open),
            }]);
            with_note(&mut draft, "mixed".into());
            let record = review(&mut store, &draft);
            let journal = admit(&mut store, &record);
            finish(&mut store, &journal)
        } else {
            settled.clone()
        };
        let request = UndoRequest {
            operation_id: Uuid::new_v4(),
            target_operation_id: target.request.operation_id,
            trash_member: None,
        };
        assert!(store.preview_proposal_undo(&request).is_err());
        assert!(store.begin_proposal_undo(&request).is_err());
        assert_eq!(store.proposal(request.operation_id).unwrap(), None);
        assert_eq!(store.proposal_apply(request.operation_id).unwrap(), None);
        let mut unsupported_journal = target;
        unsupported_journal.request.operation_id = unsupported_journal.approved.draft.id;
        unsupported_journal.receipt.as_mut().unwrap().operation_id =
            unsupported_journal.request.operation_id;
        unsupported_journal.undo = Some(UndoBinding {
            operation_id: Uuid::new_v4(),
            trash_member: None,
            originals: vec![None; unsupported_journal.members.len()],
        });
        assert!(unsupported_journal.validate().is_err());
    }
}

#[test]
fn physical_older_backup_and_fresh_database_reconstruct_retained_action_receipts() {
    let (dir, mut store) = fixture();
    let (first, created) = create(&mut store, ActionState::Waiting);
    drop(store);
    let (mut store, report) = WorkStore::open(dir.path()).unwrap();
    let older_backup = report.backup;
    let (second, replacement) = replace(&mut store, &first, ActionState::Blocked);
    drop(store);
    std::fs::write(
        dir.path().join("brn.sqlite"),
        b"synthetic physical corruption",
    )
    .unwrap();
    let (mut restored, report) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(report.restored_from, Some(older_backup));
    assert_eq!(
        restored.action(first.origin.id).unwrap(),
        Some(first.clone())
    );
    restored.restore_proposal_apply(&replacement).unwrap();
    restored.restore_proposal_apply(&created).unwrap();
    assert_eq!(
        restored.action(first.origin.id).unwrap(),
        Some(second.clone())
    );
    assert_eq!(
        restored
            .proposal_apply(replacement.request.operation_id)
            .unwrap(),
        Some(replacement.clone())
    );
    let (fresh_dir, mut fresh) = fixture();
    fresh.restore_proposal_apply(&replacement).unwrap();
    assert_eq!(fresh.action(first.origin.id).unwrap(), Some(second.clone()));
    drop(fresh);
    let (fresh, _) = WorkStore::open(fresh_dir.path()).unwrap();
    assert_eq!(fresh.action(first.origin.id).unwrap(), Some(second));
}

#[test]
fn recovery_sql_failure_rolls_back_actions_review_and_imported_journal() {
    let (_, mut source) = fixture();
    let (after, journal) = create(&mut source, ActionState::Waiting);
    let (dir, mut target) = fixture();
    raw(dir.path()).execute_batch("CREATE TRIGGER fail_import BEFORE INSERT ON proposal_applies BEGIN SELECT RAISE(ABORT,'synthetic import failure'); END;").unwrap();
    assert!(target.restore_proposal_apply(&journal).is_err());
    assert_eq!(target.action(after.origin.id).unwrap(), None);
    assert_eq!(target.proposal(journal.approved.draft.id).unwrap(), None);
    assert_eq!(
        target.proposal_apply(journal.request.operation_id).unwrap(),
        None
    );
    raw(dir.path())
        .execute_batch("DROP TRIGGER fail_import;")
        .unwrap();
    target.restore_proposal_apply(&journal).unwrap();
    assert_eq!(target.action(after.origin.id).unwrap(), Some(after));
}

#[test]
fn maximum_escaped_action_and_note_mixtures_fit_existing_whole_journal_limit() {
    const BUDGET: usize = 8 * 1024 * 1024;
    const JOURNAL_BOUND: usize = 48 * 1024 * 1024 + 1024 * 1024;
    for action_count in [3, 20, 21] {
        let (_, mut store) = fixture();
        let mut maximum = draft(vec![]);
        let mut full_data = data(ActionState::Waiting);
        full_data.title = "\u{1}".repeat(512);
        full_data.owner = Some("\u{1}".repeat(512));
        full_data.description = "\u{1}".repeat(64 * 1024);
        maximum.action_changes = (0..action_count)
            .map(|_| ActionChange::Create {
                id: Uuid::new_v4(),
                data: full_data.clone(),
            })
            .collect();
        let mut action_bytes = maximum
            .action_changes
            .iter()
            .map(|c| serde_json::to_vec(c).unwrap().len())
            .sum::<usize>();
        if action_count == 21 {
            let excess = (action_bytes + maximum.title.len()).saturating_sub(BUDGET);
            let last = maximum.action_changes.last_mut().unwrap().data_mut();
            last.description
                .truncate(last.description.len() - excess.div_ceil(6));
            action_bytes = maximum
                .action_changes
                .iter()
                .map(|c| serde_json::to_vec(c).unwrap().len())
                .sum();
            assert!(BUDGET - action_bytes - maximum.title.len() < 6);
        }
        assert!(action_bytes + maximum.title.len() <= BUDGET);
        if action_count != 21 {
            let parent = VaultIdentity {
                device: 1,
                inode: 1,
            };
            maximum.vault = Some(VaultRecord {
                id: Uuid::new_v4(),
                root: "/synthetic/vault".into(),
                identity: parent.clone(),
            });
            let note_count = 8;
            let path_bytes = (0..note_count)
                .map(|i| format!("note{i}.md").len())
                .sum::<usize>();
            let mut remaining =
                BUDGET - action_bytes - maximum.title.len() - "/synthetic/vault".len() - path_bytes;
            for i in 0..note_count {
                let length = remaining.min(brn_store::MAX_NOTE_BYTES);
                remaining -= length;
                maximum.changes.push(NoteChange::Create {
                    path: format!("note{i}.md"),
                    parent: parent.clone(),
                    text: "\u{1}".repeat(length),
                });
            }
            assert_eq!(remaining, 0);
        }
        let review = store.create_proposal(&maximum).unwrap();
        if !maximum.changes.is_empty() {
            let mut over = maximum.clone();
            over.id = Uuid::new_v4();
            if let NoteChange::Create { text, .. } = over.changes.last_mut().unwrap() {
                text.push('x');
            }
            assert!(store.create_proposal(&over).is_err());
        }
        let journal = admit(&mut store, &review);
        assert!(serde_json::to_vec(&journal.action_records).unwrap().len() > 256 * 1024);
        let bytes = serde_json::to_vec(&journal).unwrap();
        assert!(
            bytes.len() < JOURNAL_BOUND,
            "{action_count}: {}",
            bytes.len()
        );
        assert!(bytes.len() < 64 * 1024 * 1024);
        let settled = finish(&mut store, &journal);
        settled.validate().unwrap();
        assert_eq!(
            store
                .action_list(&Default::default())
                .unwrap()
                .entries
                .len(),
            action_count
        );
    }
}

#[test]
fn same_revision_action_race_refuses_but_terminal_replay_preserves_later_completion() {
    let (dir, mut store) = fixture();
    let (first, created) = create(&mut store, ActionState::Open);
    let (second, replacement) = replace(&mut store, &first, ActionState::Waiting);
    let record = review(
        &mut store,
        &draft(vec![ActionChange::Replace {
            before: Box::new(second.clone()),
            data: data(ActionState::Blocked),
        }]),
    );
    let pending = admit(&mut store, &record);
    let pending = prepared(&mut store, &pending);
    let mut same_revision = second.clone();
    same_revision
        .data
        .description
        .push_str("parallel edit at retained revision");
    put(dir.path(), &same_revision);
    assert!(matches!(
        store.finish_proposal_apply(
            pending.request.operation_id,
            ApplyOutcome::Applied,
            Some(&[])
        ),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(store.action(first.origin.id).unwrap(), Some(same_revision));
    assert_eq!(
        store.proposal_apply(pending.request.operation_id).unwrap(),
        Some(pending)
    );
    let mut completed = second;
    completed.version += 1;
    completed.data.state = ActionState::Completed;
    completed.waiting_since_ms = None;
    completed.completed_at_ms = Some(completed.updated_at_ms);
    put(dir.path(), &completed);
    for terminal in [&created, &replacement] {
        assert_eq!(
            store.begin_proposal_apply(&terminal.request).unwrap(),
            *terminal
        );
        assert_eq!(
            store
                .finish_proposal_apply(
                    terminal.request.operation_id,
                    ApplyOutcome::Applied,
                    Some(&observations(terminal))
                )
                .unwrap(),
            *terminal.receipt.as_ref().unwrap()
        );
        store.restore_proposal_apply(terminal).unwrap();
        assert_eq!(
            store.action(first.origin.id).unwrap(),
            Some(completed.clone())
        );
    }
}

fn undo_request(source: &ApplyJournal) -> UndoRequest {
    UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: source.request.operation_id,
        trash_member: None,
    }
}

#[test]
fn action_replacement_undo_previews_complete_data_and_compensates_at_a_new_revision() {
    let (dir, mut store) = fixture();
    let (first, _) = create(&mut store, ActionState::Waiting);
    let (second, _) = create(&mut store, ActionState::Blocked);
    let mut first_candidate = first.data.clone();
    first_candidate.state = ActionState::Open;
    first_candidate.title = "Incorrect approved title".into();
    first_candidate.owner = None;
    let mut second_candidate = second.data.clone();
    second_candidate.description = "Incorrect approved description\r\n".into();
    second_candidate.due_on = None;
    let record = review(
        &mut store,
        &draft(vec![
            ActionChange::Replace {
                before: Box::new(first.clone()),
                data: first_candidate,
            },
            ActionChange::Replace {
                before: Box::new(second.clone()),
                data: second_candidate,
            },
        ]),
    );
    let admitted = admit(&mut store, &record);
    let source = finish(&mut store, &admitted);
    let request = undo_request(&source);
    let source_review = store.proposal(record.draft.id).unwrap();
    let preview = store.preview_proposal_undo(&request).unwrap();
    assert!(preview.draft.changes.is_empty());
    assert!(preview.binding.originals.is_empty());
    assert_eq!(preview.binding.operation_id, source.request.operation_id);
    assert_eq!(
        preview.draft.action_changes,
        vec![
            ActionChange::Replace {
                before: Box::new(source.action_records[0].clone()),
                data: first.data.clone(),
            },
            ActionChange::Replace {
                before: Box::new(source.action_records[1].clone()),
                data: second.data.clone(),
            },
        ]
    );
    assert_eq!(store.proposal(request.operation_id).unwrap(), None);
    assert_eq!(store.proposal_apply(request.operation_id).unwrap(), None);
    assert_eq!(store.proposal(record.draft.id).unwrap(), source_review);
    for installed in &source.action_records {
        assert_eq!(
            store.action(installed.origin.id).unwrap(),
            Some(installed.clone())
        );
    }
    let admitted = store.begin_proposal_undo(&request).unwrap();
    assert_eq!(admitted.approved.draft, preview.draft);
    assert_eq!(admitted.undo, Some(preview.binding));
    for installed in &source.action_records {
        assert_eq!(
            store.action(installed.origin.id).unwrap(),
            Some(installed.clone())
        );
    }
    let terminal = finish(&mut store, &admitted);
    for (index, original) in [&first, &second].into_iter().enumerate() {
        let restored = &terminal.action_records[index];
        assert_eq!(restored.origin, original.origin);
        assert_eq!(restored.data, original.data);
        assert_eq!(restored.version, source.action_records[index].version + 1);
        assert!(restored.updated_at_ms >= source.action_records[index].updated_at_ms);
        assert_eq!(restored.completed_at_ms, None);
        assert_eq!(
            store.action(original.origin.id).unwrap(),
            Some(restored.clone())
        );
    }
    assert_eq!(
        terminal.action_records[0].waiting_since_ms,
        Some(terminal.started_at_ms)
    );
    assert_eq!(terminal.action_records[1].waiting_since_ms, None);
    drop(store);
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        store.proposal_apply(request.operation_id).unwrap(),
        Some(terminal.clone())
    );
    let (later, _) = replace(
        &mut store,
        &terminal.action_records[0],
        ActionState::Blocked,
    );
    assert_eq!(store.begin_proposal_undo(&request).unwrap(), terminal);
    assert_eq!(
        store
            .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
            .unwrap(),
        *terminal.receipt.as_ref().unwrap()
    );
    assert_eq!(store.action(first.origin.id).unwrap(), Some(later.clone()));
    let completed = store
        .complete_action_with(
            &CompleteActionRequest {
                sent_source: None,
                operation_id: Uuid::new_v4(),
                before: Box::new(later),
            },
            terminal.started_at_ms + 1,
            |_| Ok(()),
        )
        .unwrap();
    assert_eq!(store.begin_proposal_undo(&request).unwrap(), terminal);
    assert_eq!(
        store
            .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[]))
            .unwrap(),
        *terminal.receipt.as_ref().unwrap()
    );
    assert_eq!(
        store.action(first.origin.id).unwrap(),
        Some(completed.after.clone())
    );
    assert_eq!(
        store.preview_proposal_undo(&request).unwrap().draft,
        terminal.approved.draft
    );
    for changed in [
        UndoRequest {
            target_operation_id: Uuid::new_v4(),
            ..request.clone()
        },
        UndoRequest {
            trash_member: Some(0),
            ..request.clone()
        },
    ] {
        assert!(matches!(
            store.preview_proposal_undo(&changed),
            Err(Error::OperationConflict(_))
        ));
        assert!(matches!(
            store.begin_proposal_undo(&changed),
            Err(Error::OperationConflict(_))
        ));
    }
    assert_eq!(
        store.action(first.origin.id).unwrap(),
        Some(completed.after)
    );
}

#[test]
fn action_compensation_requires_every_exact_live_record_at_admission_and_settlement() {
    for mutation in 0..5 {
        let (dir, mut store) = fixture();
        let (first, _) = create(&mut store, ActionState::Waiting);
        let (second, _) = create(&mut store, ActionState::Open);
        let record = review(
            &mut store,
            &draft(vec![
                ActionChange::Replace {
                    before: Box::new(first.clone()),
                    data: data(ActionState::Blocked),
                },
                ActionChange::Replace {
                    before: Box::new(second.clone()),
                    data: data(ActionState::Waiting),
                },
            ]),
        );
        let admitted = admit(&mut store, &record);
        let source = finish(&mut store, &admitted);
        let request = undo_request(&source);
        let preview = store.preview_proposal_undo(&request).unwrap();
        let mut changed = source.action_records[1].clone();
        match mutation {
            0 => {
                raw(dir.path())
                    .execute(
                        "DELETE FROM actions WHERE id=?1",
                        [second.origin.id.to_string()],
                    )
                    .unwrap();
            }
            1 => {
                put(dir.path(), &second);
            }
            2 => {
                changed.version += 1;
                put(dir.path(), &changed);
            }
            3 => {
                changed.data.description.push_str("equal-version fork");
                put(dir.path(), &changed);
            }
            4 => {
                store
                    .complete_action_with(
                        &CompleteActionRequest {
                            sent_source: None,
                            operation_id: Uuid::new_v4(),
                            before: Box::new(changed),
                        },
                        0,
                        |_| Ok(()),
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        // Historical review remains available; confirmation freshly compares every full record.
        assert_eq!(store.preview_proposal_undo(&request).unwrap(), preview);
        let raced = store.action(second.origin.id).unwrap();
        assert!(matches!(
            store.begin_proposal_undo(&request),
            Err(Error::StateChanged(_))
        ));
        assert_eq!(store.proposal(request.operation_id).unwrap(), None);
        assert_eq!(store.proposal_apply(request.operation_id).unwrap(), None);
        assert_eq!(
            store.action(first.origin.id).unwrap(),
            Some(source.action_records[0].clone())
        );
        assert_eq!(store.action(second.origin.id).unwrap(), raced);
    }
    let (dir, mut store) = fixture();
    let (first, _) = create(&mut store, ActionState::Open);
    let (_, source) = replace(&mut store, &first, ActionState::Waiting);
    let request = undo_request(&source);
    let admitted = store.begin_proposal_undo(&request).unwrap();
    let admitted = prepared(&mut store, &admitted);
    let mut changed = source.action_records[0].clone();
    changed
        .data
        .description
        .push_str("changed after compensation admission");
    put(dir.path(), &changed);
    assert!(matches!(
        store.finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&[])),
        Err(Error::StateChanged(_))
    ));
    assert_eq!(
        store.proposal_apply(request.operation_id).unwrap(),
        Some(admitted)
    );
    assert_eq!(store.action(first.origin.id).unwrap(), Some(changed));
}

#[test]
fn action_compensation_clamps_future_clocks_and_keeps_unchanged_waiting_start() {
    let (dir, mut store) = fixture();
    let (mut first, _) = create(&mut store, ActionState::Waiting);
    first.version += 1;
    first.updated_at_ms += 1_000_000_000;
    put(dir.path(), &first);
    let (installed, source) = replace(&mut store, &first, ActionState::Waiting);
    let request = undo_request(&source);
    let admitted = store.begin_proposal_undo(&request).unwrap();
    assert!(admitted.started_at_ms >= installed.updated_at_ms);
    assert_eq!(admitted.approved.created_at_ms, admitted.started_at_ms);
    assert_eq!(admitted.approved.updated_at_ms, admitted.started_at_ms);
    assert_eq!(
        admitted.action_records[0].waiting_since_ms,
        installed.waiting_since_ms
    );
    let terminal = finish(&mut store, &admitted);
    assert_eq!(terminal.action_records[0].data, first.data);
    assert_eq!(
        terminal.action_records[0].waiting_since_ms,
        first.waiting_since_ms
    );
    assert_eq!(terminal.action_records[0].version, installed.version + 1);
}

#[test]
fn action_compensation_admission_is_atomic_and_no_effect_refusal_leaves_actions_unchanged() {
    let (dir, mut store) = fixture();
    let (first, _) = create(&mut store, ActionState::Open);
    let (installed, source) = replace(&mut store, &first, ActionState::Blocked);
    let request = undo_request(&source);
    raw(dir.path()).execute_batch("CREATE TRIGGER fail_compensation BEFORE INSERT ON proposal_applies BEGIN SELECT RAISE(ABORT, 'synthetic admission failure'); END;").unwrap();
    assert!(store.begin_proposal_undo(&request).is_err());
    assert_eq!(store.proposal(request.operation_id).unwrap(), None);
    assert_eq!(store.proposal_apply(request.operation_id).unwrap(), None);
    assert_eq!(
        store.action(first.origin.id).unwrap(),
        Some(installed.clone())
    );
    raw(dir.path())
        .execute_batch("DROP TRIGGER fail_compensation;")
        .unwrap();
    let admitted = store.begin_proposal_undo(&request).unwrap();
    let refused = store
        .refuse_proposal_before_effects(request.operation_id, None)
        .unwrap();
    assert_eq!(refused.outcome, ApplyOutcome::NotApplied);
    assert_eq!(store.action(first.origin.id).unwrap(), Some(installed));
    assert_eq!(
        store.proposal(request.operation_id).unwrap().unwrap().state,
        ProposalState::Draft
    );
    assert!(
        store
            .proposal_apply(request.operation_id)
            .unwrap()
            .unwrap()
            .no_effects
    );
    assert_eq!(
        store.begin_proposal_undo(&request).unwrap().receipt,
        Some(refused)
    );
    assert_eq!(
        store.preview_proposal_undo(&request).unwrap().draft,
        admitted.approved.draft
    );
}

#[test]
fn action_compensation_recovery_is_complete_without_the_source_journal() {
    let (_, mut source_store) = fixture();
    let (first, _) = create(&mut source_store, ActionState::Waiting);
    let (_, source) = replace(&mut source_store, &first, ActionState::Blocked);
    let request = undo_request(&source);
    let admitted = source_store.begin_proposal_undo(&request).unwrap();
    let terminal = finish(&mut source_store, &admitted);
    let (dir, mut restored) = fixture();
    restored.restore_proposal_apply(&terminal).unwrap();
    assert_eq!(
        restored
            .proposal_apply(source.request.operation_id)
            .unwrap(),
        None
    );
    assert_eq!(
        restored.action(first.origin.id).unwrap(),
        Some(terminal.action_records[0].clone())
    );
    assert_eq!(restored.begin_proposal_undo(&request).unwrap(), terminal);
    drop(restored);
    let (restored, _) = WorkStore::open(dir.path()).unwrap();
    assert_eq!(
        restored.proposal_apply(request.operation_id).unwrap(),
        Some(terminal.clone())
    );
    assert_eq!(
        restored.action(first.origin.id).unwrap(),
        Some(terminal.action_records[0].clone())
    );
    for mutation in 0..4 {
        let mut bad = terminal.clone();
        match mutation {
            0 => bad.undo.as_mut().unwrap().trash_member = Some(0),
            1 => bad.action_records[0].version -= 1,
            2 => bad.action_records[0].origin.proposal.id = Uuid::new_v4(),
            3 => bad.action_records[0].data.description.push('x'),
            _ => unreachable!(),
        }
        assert!(bad.validate().is_err());
    }
}

#[test]
fn action_undo_refuses_scoped_and_mixed_replacements_without_admission() {
    let (_, mut store) = fixture();
    let (first, _) = create(&mut store, ActionState::Open);
    let (mut installed, replacement) = replace(&mut store, &first, ActionState::Blocked);
    let mut scoped = undo_request(&replacement);
    scoped.trash_member = Some(0);
    assert!(store.preview_proposal_undo(&scoped).is_err());
    assert!(store.begin_proposal_undo(&scoped).is_err());
    assert_eq!(store.proposal(scoped.operation_id).unwrap(), None);
    assert_eq!(store.proposal_apply(scoped.operation_id).unwrap(), None);
    for mixed_file in [false, true] {
        let mut draft = draft(vec![ActionChange::Replace {
            before: Box::new(installed.clone()),
            data: data(ActionState::Waiting),
        }]);
        if mixed_file {
            with_note(
                &mut draft,
                "Mixed file replacement compensation remains unsupported".into(),
            );
        } else {
            draft.action_changes.push(ActionChange::Create {
                id: Uuid::new_v4(),
                data: data(ActionState::Open),
            });
        }
        let record = review(&mut store, &draft);
        let admitted = admit(&mut store, &record);
        let source = finish(&mut store, &admitted);
        let request = undo_request(&source);
        assert!(store.preview_proposal_undo(&request).is_err());
        assert!(store.begin_proposal_undo(&request).is_err());
        assert_eq!(store.proposal(request.operation_id).unwrap(), None);
        assert_eq!(store.proposal_apply(request.operation_id).unwrap(), None);
        installed = source.action_records[0].clone();
    }
}

#[test]
fn oversized_action_compensation_refuses_the_whole_inverse_without_truncation() {
    let (dir, mut store) = fixture();
    let mut full_data = data(ActionState::Open);
    full_data.description = "x".repeat(64 * 1024);
    let initial = review(
        &mut store,
        &draft(
            (0..43)
                .map(|_| ActionChange::Create {
                    id: Uuid::new_v4(),
                    data: full_data.clone(),
                })
                .collect(),
        ),
    );
    let admitted = admit(&mut store, &initial);
    let created = finish(&mut store, &admitted);
    let mut replacements = draft(
        created
            .action_records
            .iter()
            .map(|record| {
                let mut before = record.clone();
                // Crossing a decimal revision boundary adds bytes to every inverse baseline.
                before.version = 9;
                put(dir.path(), &before);
                ActionChange::Replace {
                    before: Box::new(before),
                    data: full_data.clone(),
                }
            })
            .collect(),
    );
    let encoded_size = replacements.title.len()
        + replacements
            .action_changes
            .iter()
            .map(|change| serde_json::to_vec(change).unwrap().len())
            .sum::<usize>();
    let mut excess = encoded_size
        .checked_sub(brn_store::work::proposals::MAX_PROPOSAL_BYTES)
        .unwrap();
    assert!(excess > 0);
    for change in replacements.action_changes.iter_mut().rev() {
        let description = &mut change.data_mut().description;
        let trim = description.len().min(excess);
        description.truncate(description.len() - trim);
        excess -= trim;
        if excess == 0 {
            break;
        }
    }
    assert_eq!(excess, 0);
    assert_eq!(
        replacements.title.len()
            + replacements
                .action_changes
                .iter()
                .map(|change| serde_json::to_vec(change).unwrap().len())
                .sum::<usize>(),
        brn_store::work::proposals::MAX_PROPOSAL_BYTES
    );
    let record = store.create_proposal(&replacements).unwrap();
    let admitted = admit(&mut store, &record);
    let source = finish(&mut store, &admitted);
    let request = undo_request(&source);
    assert!(store.preview_proposal_undo(&request).is_err());
    assert!(store.begin_proposal_undo(&request).is_err());
    assert_eq!(store.proposal(request.operation_id).unwrap(), None);
    assert_eq!(store.proposal_apply(request.operation_id).unwrap(), None);
    for installed in source.action_records {
        assert_eq!(installed.version, 10);
        assert_eq!(store.action(installed.origin.id).unwrap(), Some(installed));
    }
}
