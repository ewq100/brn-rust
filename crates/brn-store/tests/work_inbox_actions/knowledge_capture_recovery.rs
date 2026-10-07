use super::*;
use brn_store::work::proposal_apply::ApplyJournal;
use brn_store::work::proposals::{CommentRequest, CommentTarget, ReviewComment};

fn archive(
    outcome: Option<ApplyOutcome>,
    supersession: bool,
) -> (tempfile::TempDir, WorkStore, InboxActionJob, ApplyJournal) {
    let dir = fixture();
    let (mut store, _) = WorkStore::open(dir.path()).unwrap();
    let mut captured = capture();
    captured.purpose = InboxAnalysisPurpose::KnowledgeAndActions;
    let job = store
        .reserve_inbox_action(&captured, "Exact original question õ\r\n")
        .unwrap();
    let draft = if supersession {
        supersession_draft(&captured)
    } else {
        knowledge_draft(&captured)
    };
    let review = store.create_proposal(&draft).unwrap();
    let review = store
        .add_proposal_comment(&CommentRequest {
            expected: review.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "Exact temporary review observation".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: review.stamp(),
    };
    store.begin_proposal_apply(&request).unwrap();
    match outcome {
        Some(ApplyOutcome::Applied) => {
            let prepared: Vec<_> = draft
                .changes
                .iter()
                .enumerate()
                .map(|(i, change)| {
                    let text = change.text().unwrap();
                    FileFingerprint {
                        device: 1,
                        inode: 100 + i as u64,
                        len: text.len() as u64,
                        sha256: digest(text.as_bytes()),
                    }
                })
                .collect();
            store
                .record_proposal_prepared(request.operation_id, &prepared)
                .unwrap();
            let observed: Vec<_> = draft
                .changes
                .iter()
                .zip(&prepared)
                .map(|(change, proof)| ApplyMemberProof {
                    destination: Some(proof.clone()),
                    staging: match change {
                        NoteChange::Replace { before, .. } => Some(before.clone()),
                        _ => None,
                    },
                })
                .collect();
            store
                .finish_proposal_apply(request.operation_id, ApplyOutcome::Applied, Some(&observed))
                .unwrap();
        }
        Some(ApplyOutcome::NotApplied) => {
            store
                .refuse_proposal_before_effects(request.operation_id, None)
                .unwrap();
        }
        Some(ApplyOutcome::Uncertain) => {
            store
                .finish_proposal_apply(request.operation_id, ApplyOutcome::Uncertain, None)
                .unwrap();
        }
        None => {}
    }
    let journal = store.proposal_apply(request.operation_id).unwrap().unwrap();
    (dir, store, job, journal)
}

fn assert_empty_import(dir: &std::path::Path) {
    let raw = Connection::open(dir.join("brn.sqlite")).unwrap();
    for table in [
        "inbox_actions",
        "proposals",
        "proposal_applies",
        "messages",
        "conversations",
    ] {
        let count: i64 = raw
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "partial import in {table}");
    }
}

#[test]
fn exact_capture_imports_each_outcome_and_supersession_without_fabricating_chat() {
    for supersession in [false, true] {
        for outcome in [
            None,
            Some(ApplyOutcome::Applied),
            Some(ApplyOutcome::NotApplied),
            Some(ApplyOutcome::Uncertain),
        ] {
            let (_source_dir, _source, job, journal) = archive(outcome, supersession);
            journal
                .approved
                .draft
                .inbox_knowledge
                .as_ref()
                .unwrap()
                .validate_capture(&job)
                .unwrap();
            let original_job_bytes = serde_json::to_vec(&job).unwrap();
            let original_journal_bytes = serde_json::to_vec(&journal).unwrap();
            let dir = fixture();
            let (mut target, _) = WorkStore::open(dir.path()).unwrap();
            assert_eq!(
                target
                    .restore_proposal_apply_with_capture(&journal, Some(&job))
                    .unwrap(),
                journal
            );
            assert_eq!(
                target.inbox_action(job.capture.id).unwrap(),
                Some(job.clone())
            );
            assert_eq!(
                target.proposal_apply(journal.request.operation_id).unwrap(),
                Some(journal.clone())
            );
            assert!(target.turn(job.capture.id).unwrap().is_none());
            assert!(target.conversations().unwrap().is_empty());
            assert!(matches!(
                target.begin_inbox_action_turn(&job),
                Err(Error::StateChanged(_))
            ));
            let mut chat = target.chat_connection().unwrap();
            assert!(matches!(
                chat.begin_inbox_action_turn(&job),
                Err(Error::StateChanged(_))
            ));
            drop(chat);
            assert_eq!(
                target.restore_proposal_apply(&journal).unwrap(),
                journal,
                "None uses the checked retained reservation"
            );
            assert_eq!(
                target
                    .restore_proposal_apply_with_capture(&journal, Some(&job))
                    .unwrap(),
                journal
            );
            let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
            let row: (Vec<u8>, Vec<u8>, i64) = raw
                .query_row(
                    "SELECT record_json,record_sha256,created_at_ms FROM inbox_actions WHERE id=?1",
                    [job.capture.id.to_string()],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .unwrap();
            assert_eq!(
                row,
                (
                    original_job_bytes.clone(),
                    digest(&original_job_bytes).to_vec(),
                    job.created_at_ms as i64
                )
            );
            assert_eq!(
                raw.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                16
            );
            drop(raw);
            drop(target);
            let (mut target, _) = WorkStore::open(dir.path()).unwrap();
            assert_eq!(
                serde_json::to_vec(&target.inbox_action(job.capture.id).unwrap().unwrap()).unwrap(),
                original_job_bytes
            );
            assert_eq!(
                serde_json::to_vec(
                    &target
                        .restore_proposal_apply_with_capture(&journal, Some(&job))
                        .unwrap()
                )
                .unwrap(),
                original_journal_bytes
            );
            assert!(target.turn(job.capture.id).unwrap().is_none());
            assert!(target.conversations().unwrap().is_empty());
        }
    }
}

#[test]
fn capture_payload_and_every_quote_are_checked_before_import() {
    let (_source_dir, _source, job, journal) = archive(Some(ApplyOutcome::Applied), false);
    let binding = journal.approved.draft.inbox_knowledge.as_ref().unwrap();
    for mode in 0..13 {
        let mut bad = job.clone();
        match mode {
            0 => bad.capture.id = Uuid::new_v4(),
            1 => bad.capture.purpose = InboxAnalysisPurpose::Actions,
            2 => bad.capture.source.as_mut().unwrap().path = "Sources/another.md".into(),
            3 => bad.capture.source.as_mut().unwrap().fingerprint.inode += 1,
            4 => bad.capture.source.as_mut().unwrap().fingerprint.sha256[0] ^= 1,
            5 => bad.capture.source_text.push('x'),
            6 => {
                bad.capture.source_text = bad.capture.source_text.replace("body", "Body");
                rebind_text(&mut bad.capture);
            }
            7 => bad.question.clear(),
            8 => bad.created_at_ms = i64::MAX as u64 + 1,
            9 => bad.capture.provider = "invented".into(),
            10 => bad.capture.model = "bad model".into(),
            11 => bad.capture.effort = "invented".into(),
            _ => bad.capture.conversation = Some(Uuid::nil()),
        }
        assert!(
            binding.validate_capture(&bad).is_err(),
            "pure payload mode {mode}"
        );
        let dir = fixture();
        let (mut target, _) = WorkStore::open(dir.path()).unwrap();
        assert!(
            target
                .restore_proposal_apply_with_capture(&journal, Some(&bad))
                .is_err(),
            "import mode {mode}"
        );
        assert_empty_import(dir.path());
    }
    let mut two = binding.clone();
    let mut second = two.citations[0].clone();
    second.start_byte = job.capture.source_text.find("text\n").unwrap();
    second.end_byte = second.start_byte + 4;
    second.quote = "text".into();
    two.citations.push(second);
    two.validate_capture(&job).unwrap();
    two.citations[1].quote = "xxxx".into();
    two.validate().unwrap();
    assert!(
        two.validate_capture(&job).is_err(),
        "the second exact quote is also checked"
    );
    let mut range = binding.clone();
    range.citations[0].start_byte += 1;
    range.citations[0].end_byte += 1;
    range.validate().unwrap();
    assert!(range.validate_capture(&job).is_err());
    let (_foreign_dir, _foreign_store, foreign, _) = archive(Some(ApplyOutcome::Applied), false);
    let dir = fixture();
    let (mut target, _) = WorkStore::open(dir.path()).unwrap();
    assert!(
        target
            .restore_proposal_apply_with_capture(&journal, Some(&foreign))
            .is_err()
    );
    let mut plain = journal.clone();
    plain.approved.draft.inbox_knowledge = None;
    plain.creation_sha256 = digest(&serde_json::to_vec(&plain.approved.draft).unwrap());
    plain.validate().unwrap();
    assert!(
        target
            .restore_proposal_apply_with_capture(&plain, Some(&job))
            .is_err()
    );
    assert_empty_import(dir.path());
}

#[test]
fn retained_reservation_rejects_valid_question_time_and_selection_forks() {
    let (_source_dir, _source, job, journal) = archive(Some(ApplyOutcome::Applied), false);
    let dir = fixture();
    let (mut target, _) = WorkStore::open(dir.path()).unwrap();
    target
        .restore_proposal_apply_with_capture(&journal, Some(&job))
        .unwrap();
    for mode in 0..6 {
        let mut bad = job.clone();
        match mode {
            0 => bad.question.push('x'),
            1 => bad.created_at_ms += 1,
            2 => bad.capture.provider = "copilot".into(),
            3 => bad.capture.model = "another-valid-model".into(),
            4 => bad.capture.effort = "high".into(),
            _ => bad.capture.conversation = Some(Uuid::new_v4()),
        }
        // On an empty database these domains cannot independently prove their
        // original historical values; exact retained equality supplies that proof.
        journal
            .approved
            .draft
            .inbox_knowledge
            .as_ref()
            .unwrap()
            .validate_capture(&bad)
            .unwrap();
        assert!(
            matches!(
                target.restore_proposal_apply_with_capture(&journal, Some(&bad)),
                Err(Error::OperationConflict(_))
            ),
            "fork {mode}"
        );
        assert_eq!(
            target.inbox_action(job.capture.id).unwrap(),
            Some(job.clone())
        );
        assert_eq!(
            target.proposal_apply(journal.request.operation_id).unwrap(),
            Some(journal.clone())
        );
    }
}

#[test]
fn chat_and_rewrite_uuid_collisions_and_late_sql_failure_roll_back_import() {
    let (_source_dir, _source, job, journal) = archive(Some(ApplyOutcome::Applied), false);
    for rewrite in [false, true] {
        let dir = fixture();
        let (mut target, _) = WorkStore::open(dir.path()).unwrap();
        if rewrite {
            let expected = review(&mut target);
            target
                .begin_proposal_rewrite(&RewriteSpec {
                    id: job.capture.id,
                    expected,
                    provider: job.capture.provider.clone(),
                    model: job.capture.model.clone(),
                    effort: job.capture.effort.clone(),
                })
                .unwrap();
        } else {
            target
                .begin_turn_with_effort(
                    job.capture.id,
                    None,
                    &job.question,
                    &job.capture.provider,
                    &job.capture.model,
                    Some(&job.capture.effort),
                )
                .unwrap();
        }
        assert!(matches!(
            target.restore_proposal_apply_with_capture(&journal, Some(&job)),
            Err(Error::OperationConflict(_))
        ));
        assert!(target.inbox_action(job.capture.id).unwrap().is_none());
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
    }
    for table in ["proposals", "proposal_applies"] {
        let dir = fixture();
        let (mut target, _) = WorkStore::open(dir.path()).unwrap();
        let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
        raw.execute_batch(&format!("CREATE TRIGGER fail_import BEFORE INSERT ON {table} BEGIN SELECT RAISE(FAIL, 'synthetic write refusal'); END;")).unwrap();
        assert!(
            target
                .restore_proposal_apply_with_capture(&journal, Some(&job))
                .is_err()
        );
        assert_empty_import(dir.path());
    }
}

#[test]
fn issued_history_fences_only_missing_turn_and_checks_all_journals() {
    let (_source_dir, mut source, job, journal) = archive(Some(ApplyOutcome::NotApplied), false);
    assert!(matches!(
        source.begin_inbox_action_turn(&job),
        Err(Error::StateChanged(_))
    ));
    let mut fresh_capture = job.capture.clone();
    fresh_capture.id = Uuid::new_v4();
    let unissued = source
        .reserve_inbox_action(&fresh_capture, "Unissued exact question")
        .unwrap();
    let turn = source.begin_inbox_action_turn(&unissued).unwrap();
    let completed = source
        .finish_turn(
            turn.id,
            WorkTurnStatus::Completed,
            "Synthetic retained answer",
            None,
        )
        .unwrap();
    let draft = knowledge_draft(&unissued.capture);
    let record = source.create_proposal(&draft).unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    source.begin_proposal_apply(&request).unwrap();
    source
        .refuse_proposal_before_effects(request.operation_id, None)
        .unwrap();
    assert_eq!(
        serde_json::to_value(source.begin_inbox_action_turn(&unissued).unwrap()).unwrap(),
        serde_json::to_value(&completed).unwrap()
    );
    let mut chat = source.chat_connection().unwrap();
    assert_eq!(
        serde_json::to_value(chat.begin_inbox_action_turn(&unissued).unwrap()).unwrap(),
        serde_json::to_value(&completed).unwrap()
    );
    drop(chat);
    assert_eq!(source.turns(turn.conversation_id).unwrap().len(), 1);

    let dir = fixture();
    let (mut target, _) = WorkStore::open(dir.path()).unwrap();
    target
        .restore_proposal_apply_with_capture(&journal, Some(&job))
        .unwrap();
    let pending = target
        .reserve_inbox_action(&fresh_capture, "Separate unissued question")
        .unwrap();
    let mut damaged = journal.clone();
    damaged.receipt.as_mut().unwrap().approved_version += 1;
    let bytes = serde_json::to_vec(&damaged).unwrap();
    let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
    raw.execute(
        "UPDATE proposal_applies SET journal_json=?1,journal_sha256=?2 WHERE operation_id=?3",
        params![
            bytes,
            digest(&bytes).as_slice(),
            journal.request.operation_id.to_string()
        ],
    )
    .unwrap();
    assert!(
        target.begin_inbox_action_turn(&pending).is_err(),
        "an unrelated corrupt issued journal is not hidden by the fence scan"
    );
    assert!(target.turn(pending.capture.id).unwrap().is_none());
    assert!(target.conversations().unwrap().is_empty());
}

#[test]
fn fresh_knowledge_restore_without_capture_refuses_before_any_commit() {
    let (_source_dir, _source, _job, journal) = archive(Some(ApplyOutcome::Applied), false);
    let dir = fixture();
    let (mut target, _) = WorkStore::open(dir.path()).unwrap();
    assert!(
        target.restore_proposal_apply(&journal).is_err(),
        "a valid approval alone cannot restore its missing semantic capture"
    );
    assert_empty_import(dir.path());
    drop(target);
    WorkStore::open(dir.path()).unwrap();
}

#[test]
fn newer_review_and_annotation_cleanup_win_over_old_capture_backed_receipts() {
    let (_source_dir, mut source, job, old) = archive(Some(ApplyOutcome::NotApplied), false);
    assert!(!old.approved.comments.is_empty());
    let current = source.proposal(old.approved.draft.id).unwrap().unwrap();
    let text = current.draft.changes[0]
        .text()
        .unwrap()
        .replace("Interpreted summary", "Later owner interpretation");
    let reviewed = source
        .edit_proposal(&ProposalEdit {
            expected: current.stamp(),
            title: "Later owner review".into(),
            texts: vec![Some(text.clone())],
            action_data: vec![],
        })
        .unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: reviewed.stamp(),
    };
    source.begin_proposal_apply(&request).unwrap();
    let installed = FileFingerprint {
        device: 1,
        inode: 103,
        len: text.len() as u64,
        sha256: digest(text.as_bytes()),
    };
    source
        .record_proposal_prepared(request.operation_id, std::slice::from_ref(&installed))
        .unwrap();
    source
        .finish_proposal_apply(
            request.operation_id,
            ApplyOutcome::Applied,
            Some(&[ApplyMemberProof {
                destination: Some(installed),
                staging: None,
            }]),
        )
        .unwrap();
    let latest = source
        .proposal_apply(request.operation_id)
        .unwrap()
        .unwrap();
    let dir = fixture();
    let (mut target, _) = WorkStore::open(dir.path()).unwrap();
    target
        .restore_proposal_apply_with_capture(&old, Some(&job))
        .unwrap();
    target
        .restore_proposal_apply_with_capture(&latest, Some(&job))
        .unwrap();
    let live = target.proposal(old.approved.draft.id).unwrap().unwrap();
    assert_eq!(live.draft.changes[0].text(), Some(text.as_str()));
    assert!(live.comments.is_empty());
    let cleaned = target
        .proposal_apply(old.request.operation_id)
        .unwrap()
        .unwrap();
    assert!(cleaned.approved.comments.is_empty());
    assert_eq!(
        target
            .restore_proposal_apply_with_capture(&old, Some(&job))
            .unwrap(),
        cleaned
    );
    assert_eq!(target.proposal(live.draft.id).unwrap(), Some(live.clone()));
    assert_eq!(
        target.inbox_action(job.capture.id).unwrap(),
        Some(job.clone())
    );
    drop(target);
    let (mut target, _) = WorkStore::open(dir.path()).unwrap();
    target
        .restore_proposal_apply_with_capture(&latest, Some(&job))
        .unwrap();
    assert_eq!(target.proposal(live.draft.id).unwrap(), Some(live));
    assert!(target.turn(job.capture.id).unwrap().is_none());
}

#[test]
fn corrupt_retained_capture_refuses_even_an_equal_supplied_job() {
    let (_source_dir, _source, job, journal) = archive(Some(ApplyOutcome::Applied), false);
    for structural in [false, true] {
        let dir = fixture();
        let (mut target, _) = WorkStore::open(dir.path()).unwrap();
        target
            .restore_proposal_apply_with_capture(&journal, Some(&job))
            .unwrap();
        let raw = Connection::open(dir.path().join("brn.sqlite")).unwrap();
        let mut bad = job.clone();
        bad.question.clear();
        let bytes = serde_json::to_vec(&bad).unwrap();
        let hash = if structural { digest(&bytes) } else { [0; 32] };
        raw.execute(
            "UPDATE inbox_actions SET record_json=?1,record_sha256=?2 WHERE id=?3",
            params![bytes, hash.as_slice(), job.capture.id.to_string()],
        )
        .unwrap();
        let before: Vec<u8> = raw
            .query_row(
                "SELECT journal_json FROM proposal_applies WHERE operation_id=?1",
                [journal.request.operation_id.to_string()],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            target
                .restore_proposal_apply_with_capture(&journal, Some(&job))
                .is_err()
        );
        assert_eq!(
            raw.query_row(
                "SELECT journal_json FROM proposal_applies WHERE operation_id=?1",
                [journal.request.operation_id.to_string()],
                |r| r.get::<_, Vec<u8>>(0)
            )
            .unwrap(),
            before
        );
        assert_eq!(
            raw.query_row(
                "SELECT record_json FROM inbox_actions WHERE id=?1",
                [job.capture.id.to_string()],
                |r| r.get::<_, Vec<u8>>(0)
            )
            .unwrap(),
            bytes
        );
    }
}
