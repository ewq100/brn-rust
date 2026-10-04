use super::*;
use brn_workflow::{
    app::App,
    editor::FileFingerprint,
    proposal_apply::ApplyMemberPhase,
    proposals::SourceVersion,
    vault::{VaultPath, read_note},
};

const FIRST: &str = "\u{feff}First approved õ🦀\r\n正文\r";
const SECOND: &str = "\u{feff}Second approved λ🧭\r\n";

// Only public operational admission and task-owned staging files prepare a
// partial synthetic operation. Production code has no Store/file shortcut.
fn partial() -> (Fixture, ApplyJournal) {
    let fixture = Fixture::new();
    let mut app = App::open(
        &fixture.0.path().join("data"),
        AppConfig {
            vault_root: Some(fixture.vault()),
            credentials_dir: Some(fixture.0.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    fs::write(fixture.vault().join("source.md"), "Synthetic source õ\r\n").unwrap();
    let source = app.open_editor("source.md").unwrap().record;
    let draft = app
        .create_proposal(&DraftRequest {
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            title: "\u{feff}Partial full 日本語\r\n".into(),
            changes: vec![
                DraftNoteChange::Create {
                    path: "first.md".into(),
                    text: FIRST.into(),
                },
                DraftNoteChange::Create {
                    path: "second.md".into(),
                    text: SECOND.into(),
                },
            ],
            sources: vec![SourceVersion {
                path: source.path,
                fingerprint: source.baseline,
            }],
        })
        .unwrap();
    let draft = app
        .add_proposal_comment(&CommentRequest {
            expected: draft.stamp(),
            comment: ReviewComment {
                id: Uuid::new_v4(),
                text: "\u{feff}Retained temporary review 日本語\r\n".into(),
                target: CommentTarget::Proposal,
            },
        })
        .unwrap();
    let request = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: draft.stamp(),
    };
    let journal = app.work_store_mut().begin_proposal_apply(&request).unwrap();
    let mut proofs = vec![];
    for (member, text) in journal.members.iter().zip([FIRST, SECOND]) {
        let stage = fixture.vault().join(&member.staging);
        fs::write(&stage, text).unwrap();
        // The public exact reader obtains fixture hash bytes through a valid
        // contained Markdown path; no new dependency or private seam is needed.
        fs::write(fixture.vault().join("fixture-hash.md"), text).unwrap();
        let hash = read_note(
            &fixture.vault(),
            &VaultPath::parse("fixture-hash.md").unwrap(),
        )
        .unwrap()
        .sha256;
        fs::remove_file(fixture.vault().join("fixture-hash.md")).unwrap();
        let meta = stage.metadata().unwrap();
        proofs.push(FileFingerprint {
            device: meta.dev(),
            inode: meta.ino(),
            len: meta.len(),
            sha256: hash,
        });
    }
    let journal = app
        .work_store_mut()
        .record_proposal_prepared(request.operation_id, &proofs)
        .unwrap();
    fs::rename(
        fixture.vault().join(&journal.members[0].staging),
        fixture.vault().join("first.md"),
    )
    .unwrap();
    drop(app);
    (fixture, journal)
}

fn preview(
    worker: &AppWorker,
    state: &mut AiState,
    journal: &ApplyJournal,
    direction: RepairDirection,
) -> crate::approval::RepairCapture {
    let command = state
        .preview_repair(journal.request.operation_id, direction)
        .unwrap();
    let (id, AppEvent::ProposalRepairPreview(body)) = reply(worker, command) else {
        panic!("full observed repair preview");
    };
    assert_eq!(body.approved, journal.approved.draft);
    assert_eq!(
        body.phases,
        [ApplyMemberPhase::Applied, ApplyMemberPhase::Before]
    );
    state.apply(
        id,
        AppEvent::ProposalUndoPreview(brn_workflow::proposal_apply::UndoPreview {
            draft: body.approved.clone(),
            binding: brn_workflow::proposal_apply::UndoBinding {
                operation_id: body.operation_id,
                trash_member: None,
                originals: vec![None; body.phases.len()],
            },
        }),
    );
    assert!(state.pending.contains_key(&id));
    let mut wrong = body.clone();
    wrong.operation_id = Uuid::new_v4();
    state.apply(id, AppEvent::ProposalRepairPreview(wrong));
    assert!(state.pending.contains_key(&id));
    state.apply(id, AppEvent::ProposalRepairPreview(body));
    let capture = state.repair_preview.as_ref().unwrap().clone();
    assert!(state.confirm_repair(&capture).is_none()); // Full retained comments not loaded yet.
    inspect(worker, state, journal.request.operation_id);
    assert_eq!(
        state
            .application_snapshot
            .as_ref()
            .unwrap()
            .approved
            .comments,
        journal.approved.comments
    );
    capture
}

#[test]
fn finish_and_restore_freeze_exact_attempt_and_record_actual_outcome_through_restart() {
    for direction in [RepairDirection::Finish, RepairDirection::Restore] {
        let (fixture, journal) = partial();
        let mut worker = fixture.worker();
        let mut state = ready();
        open(&worker, &mut state, journal.approved.draft.id);
        let captured = preview(&worker, &mut state, &journal, direction);
        assert_eq!(
            fs::read(fixture.vault().join("first.md")).unwrap(),
            FIRST.as_bytes()
        );
        assert!(!fixture.vault().join("second.md").exists());
        let command = state.confirm_repair(&captured).unwrap();
        let outer = command.0;
        assert_eq!(state.last_repair_request.as_ref(), Some(captured.request()));
        assert_critical(&state);
        let (_, AppEvent::ProposalRepaired(receipt)) = reply(&worker, command) else {
            panic!("recorded repair receipt");
        };
        let mut wrong = receipt.clone();
        wrong.id = Uuid::new_v4();
        state.apply(outer, AppEvent::ProposalRepaired(wrong));
        state.apply(outer, AppEvent::ProposalRecovery(vec![]));
        assert!(state.pending.contains_key(&outer));
        assert_critical(&state);
        let mut wrong = receipt.clone();
        wrong.direction = match direction {
            RepairDirection::Finish => RepairDirection::Restore,
            RepairDirection::Restore => RepairDirection::Finish,
        };
        state.apply(outer, AppEvent::ProposalRepaired(wrong));
        assert!(state.pending.contains_key(&outer));
        let reads = state.apply(outer, AppEvent::ProposalRepaired(receipt.clone()));
        assert_critical(&state); // Current full review must acknowledge its new state.
        drain_reads(&worker, &mut state, reads);
        assert!(!state.application_busy());
        assert_eq!(state.repair_receipt, Some(receipt.clone()));
        assert!(state.applies.is_empty());
        match direction {
            RepairDirection::Finish => {
                assert_eq!(receipt.outcome, Some(ApplyOutcome::Applied));
                assert_eq!(
                    fs::read(fixture.vault().join("first.md")).unwrap(),
                    FIRST.as_bytes()
                );
                assert_eq!(
                    fs::read(fixture.vault().join("second.md")).unwrap(),
                    SECOND.as_bytes()
                );
                assert!(
                    state
                        .application_snapshot
                        .as_ref()
                        .unwrap()
                        .approved
                        .comments
                        .is_empty()
                );
            }
            RepairDirection::Restore => {
                assert_eq!(receipt.outcome, Some(ApplyOutcome::NotApplied));
                assert!(!fixture.vault().join("first.md").exists());
                assert!(!fixture.vault().join("second.md").exists());
                assert_eq!(
                    state
                        .application_snapshot
                        .as_ref()
                        .unwrap()
                        .approved
                        .comments,
                    journal.approved.comments
                );
            }
        }
        worker.shutdown().unwrap();
        fs::write(fixture.vault().join("first.md"), "Later owner bytes 🧭\r\n").unwrap();
        let mut worker = fixture.worker();
        let (_, AppEvent::ProposalRepaired(replayed)) =
            reply(&worker, (Uuid::new_v4(), captured.command()))
        else {
            panic!("exact historical repair replay");
        };
        assert_eq!(replayed, receipt);
        assert_eq!(
            fs::read(fixture.vault().join("first.md")).unwrap(),
            "Later owner bytes 🧭\r\n".as_bytes()
        );
        worker.shutdown().unwrap();
    }
}

#[test]
fn late_previews_and_failures_cannot_change_newer_direction_or_release_its_lookup() {
    let (fixture, journal) = partial();
    let mut worker = fixture.worker();
    let mut state = ready();
    let older = state
        .preview_repair(journal.request.operation_id, RepairDirection::Finish)
        .unwrap();
    let (old_id, AppEvent::ProposalRepairPreview(old)) = reply(&worker, older) else {
        panic!("old full preview")
    };
    let old_failure = state
        .preview_repair(journal.request.operation_id, RepairDirection::Finish)
        .unwrap()
        .0;
    let newer = state
        .preview_repair(journal.request.operation_id, RepairDirection::Restore)
        .unwrap();
    let (new_id, AppEvent::ProposalRepairPreview(new)) = reply(&worker, newer) else {
        panic!("new full preview")
    };
    state.apply(new_id, AppEvent::ProposalRecovery(vec![]));
    assert!(state.pending.contains_key(&new_id));
    state.apply(new_id, AppEvent::ProposalRepairPreview(new.clone()));
    let captured = state.repair_preview.clone().unwrap();
    state.apply(old_id, AppEvent::ProposalRepairPreview(old));
    state.apply(
        old_failure,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("Old preview error")),
    );
    assert_eq!(state.repair_preview, Some(captured.clone()));
    assert_eq!(captured.request().direction, RepairDirection::Restore);
    assert_eq!(captured.request().expected, new.expected);
    assert!(state.operation_error.is_none());
    assert_eq!(
        fs::read(fixture.vault().join("first.md")).unwrap(),
        FIRST.as_bytes()
    );
    assert!(!fixture.vault().join("second.md").exists());
    worker.shutdown().unwrap();
}

#[test]
fn changed_phase_after_capture_refuses_before_writes_and_preserves_inspectable_request() {
    let (fixture, journal) = partial();
    let mut worker = fixture.worker();
    let mut state = ready();
    let captured = preview(&worker, &mut state, &journal, RepairDirection::Finish);
    // An external complete phase changes the exact observation/hash without
    // introducing unknown bytes. The earlier capture must refuse, not finish.
    fs::rename(
        fixture.vault().join(&journal.members[1].staging),
        fixture.vault().join("second.md"),
    )
    .unwrap();
    let before = [
        fingerprint(&fixture.vault().join("first.md")),
        fingerprint(&fixture.vault().join("second.md")),
    ];
    let command = state.confirm_repair(&captured).unwrap();
    let (id, AppEvent::Failed(error)) = reply(&worker, command) else {
        panic!("stale observation must refuse")
    };
    assert_eq!(error.kind, brn_workflow::ErrorKind::ContextStale);
    let reads = state.apply(id, AppEvent::Failed(error));
    let mut reads = VecDeque::from(reads);
    while let Some(command) = reads.pop_front() {
        assert!(matches!(
            command.1,
            AppCommand::Proposals(_)
                | AppCommand::Proposal(_)
                | AppCommand::Activity(_)
                | AppCommand::ProposalRecovery
                | AppCommand::ProposalApply(_)
                | AppCommand::Notes { .. }
                | AppCommand::ScopedNotes { .. }
        ));
        let (id, event) = reply(&worker, command);
        if let AppEvent::Failed(error) = &event {
            assert_eq!(error.kind, brn_workflow::ErrorKind::SaveUncertain);
        }
        reads.extend(state.apply(id, event));
    }
    assert_eq!(state.last_repair_request.as_ref(), Some(captured.request()));
    assert!(state.operation_error.is_some());
    assert!(state.repair_receipt.is_none());
    assert_eq!(
        [
            fingerprint(&fixture.vault().join("first.md")),
            fingerprint(&fixture.vault().join("second.md"))
        ],
        before
    );
    assert!(
        state
            .application_snapshot
            .as_ref()
            .unwrap()
            .repair
            .is_none()
    );
    worker.shutdown().unwrap();
}

fn fingerprint(path: &Path) -> (u64, u64, Vec<u8>) {
    let meta = path.metadata().unwrap();
    (meta.dev(), meta.ino(), fs::read(path).unwrap())
}
