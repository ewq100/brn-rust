use super::*;
use brn_workflow::{
    ErrorKind,
    action_completion::CompleteActionRequest,
    actions::{ActionData, ActionRecord, ActionState},
    proposals::{ActionChange, NoteChange},
};

const ORIGINAL: &str = "\u{feff}Original õ🦀\r\n正文\nlast\r";
const APPROVED: &str = "\u{feff}Approved 日本語 λ\r\n";
const TRASH: &str = "\u{feff}Retained Trash õ🦀\r\n";
const CREATED: &str = "\u{feff}Created evidence λ\r\n";
const LATER: &str = "\u{feff}Later explicit user bytes Ελληνικά\r\n";

struct Mixed {
    source: ApprovalRequest,
    before: EditorRecord,
    trash: EditorRecord,
}

fn mixed(fixture: &Fixture) -> (AppWorker, Mixed) {
    fs::write(fixture.vault().join("replace.md"), ORIGINAL).unwrap();
    fs::write(fixture.vault().join("trash.md"), TRASH).unwrap();
    let worker = fixture.worker();
    let editor = |path: &str| {
        let (_, AppEvent::Editor(view)) = reply(
            &worker,
            (Uuid::new_v4(), AppCommand::OpenEditor(path.into())),
        ) else {
            panic!("open source note");
        };
        view.record
    };
    let before = editor("replace.md");
    let trash = editor("trash.md");
    let (_, AppEvent::Proposal(record)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(DraftRequest {
                intake: None,
                inbox_visual: None,
                inbox_knowledge: None,
                inbox_source: None,
                action_changes: Vec::new(),
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Exact native 日本語 Replace/Create/Trash\r\nλ".into(),
                changes: vec![
                    DraftNoteChange::Replace {
                        path: before.path.clone(),
                        expected: before.baseline.clone(),
                        text: APPROVED.into(),
                    },
                    DraftNoteChange::Create {
                        path: "created.md".into(),
                        text: CREATED.into(),
                    },
                    DraftNoteChange::Trash {
                        path: trash.path.clone(),
                        expected: trash.baseline.clone(),
                    },
                ],
                sources: vec![],
            }),
        ),
    ) else {
        panic!("mixed source draft");
    };
    let source = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    let (_, AppEvent::ProposalApplied(receipt)) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::ApproveProposal(source.clone())),
    ) else {
        panic!("mixed source Applied");
    };
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert_eq!(receipt.operation_id, source.operation_id);
    (
        worker,
        Mixed {
            source,
            before,
            trash,
        },
    )
}

fn files(fixture: &Fixture) -> Vec<(String, Vec<u8>, u64)> {
    let mut files = fs::read_dir(fixture.vault())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
                entry.metadata().unwrap().ino(),
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn preview(
    worker: &AppWorker,
    state: &mut AiState,
    source: Uuid,
    member: Option<usize>,
) -> crate::approval::UndoCapture {
    let command = state.preview_undo(source, member).unwrap();
    let (id, event) = reply(worker, command);
    assert!(matches!(event, AppEvent::ProposalUndoPreview(_)));
    assert!(state.apply(id, event).is_empty());
    state.undo_preview.as_ref().unwrap().clone()
}

fn assert_originals(fixture: &Fixture, original: &Mixed) {
    assert_eq!(
        fs::read(fixture.vault().join("replace.md")).unwrap(),
        ORIGINAL.as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault().join("trash.md")).unwrap(),
        TRASH.as_bytes()
    );
    assert!(!fixture.vault().join("created.md").exists());
    assert_eq!(
        fs::metadata(fixture.vault().join("replace.md"))
            .unwrap()
            .ino(),
        original.before.baseline.inode
    );
    assert_eq!(
        fs::metadata(fixture.vault().join("trash.md"))
            .unwrap()
            .ino(),
        original.trash.baseline.inode
    );
}

#[test]
fn full_inverse_preview_cancel_and_stale_generation_never_admit_work() {
    let fixture = Fixture::new();
    let (mut worker, original) = mixed(&fixture);
    let mut state = ready();
    open(&worker, &mut state, original.source.expected.id);
    let initial_files = files(&fixture);
    let command = state
        .preview_undo(original.source.operation_id, None)
        .unwrap();
    let outer = command.0;
    let (id, AppEvent::ProposalUndoPreview(full)) = reply(&worker, command) else {
        panic!("full inverse");
    };
    assert_eq!(id, outer);
    assert!(
        state
            .apply(Uuid::new_v4(), AppEvent::ProposalUndoPreview(full.clone()))
            .is_empty()
    );
    assert!(state.apply(outer, AppEvent::Proposals(vec![])).is_empty());
    assert!(state.pending.contains_key(&outer));
    assert!(state.undo_preview.is_none());
    for wrong in [
        {
            let mut wrong = full.clone();
            wrong.draft.id = Uuid::new_v4();
            wrong
        },
        {
            let mut wrong = full.clone();
            wrong.binding.operation_id = Uuid::new_v4();
            wrong
        },
        {
            let mut wrong = full.clone();
            wrong.binding.trash_member = Some(2);
            wrong
        },
    ] {
        assert!(
            state
                .apply(outer, AppEvent::ProposalUndoPreview(wrong))
                .is_empty()
        );
        assert!(state.pending.contains_key(&outer));
        assert!(state.undo_preview.is_none());
    }
    assert!(
        state
            .apply(outer, AppEvent::ProposalUndoPreview(full.clone()))
            .is_empty()
    );
    let canceled = state.undo_preview.as_ref().unwrap().clone();
    assert_eq!(canceled.preview(), &full);
    assert_eq!(full.draft.changes.len(), 3);
    assert_eq!(full.binding.operation_id, original.source.operation_id);
    assert_eq!(full.draft.group_id, None);
    assert_eq!(full.draft.changes[0].text(), Some(ORIGINAL));
    assert!(
        matches!(&full.draft.changes[1], NoteChange::Trash { before_text, .. } if before_text == CREATED)
    );
    assert_eq!(full.draft.changes[2].text(), Some(TRASH));
    assert!(full.binding.originals[0].is_some());
    assert!(full.binding.originals[1].is_none());
    assert!(full.binding.originals[2].is_some());
    assert!(state.last_undo_request.is_none());

    let older = state
        .preview_undo(original.source.operation_id, None)
        .unwrap();
    let (older_id, older_event) = reply(&worker, older);
    let newer = state
        .preview_undo(original.source.operation_id, Some(2))
        .unwrap();
    let (newer_id, newer_event) = reply(&worker, newer);
    state.apply(newer_id, newer_event);
    let newest = state.undo_preview.as_ref().unwrap().clone();
    assert_eq!(newest.request().trash_member, Some(2));
    assert!(state.apply(older_id, older_event).is_empty());
    assert_eq!(state.undo_preview.as_ref(), Some(&newest));
    assert!(!state.pending.contains_key(&older_id));
    assert!(state.confirm_undo(&canceled).is_none());
    assert!(!state.application_busy());
    assert_eq!(files(&fixture), initial_files);
    for request in [canceled.request(), newest.request()] {
        let (_, AppEvent::ProposalApply(None)) = reply(
            &worker,
            (
                Uuid::new_v4(),
                AppCommand::ProposalApply(request.operation_id),
            ),
        ) else {
            panic!("preview/cancel must not admit an operation");
        };
    }
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    assert_eq!(files(&fixture), initial_files);
    let (_, AppEvent::Proposals(records)) =
        reply(&worker, (Uuid::new_v4(), AppCommand::Proposals(None)))
    else {
        panic!("restart proposals");
    };
    assert_eq!(records.len(), 1);
    worker.shutdown().unwrap();
}

#[test]
fn whole_undo_exact_receipt_guards_and_restart_replay_preserve_later_bytes_and_inodes() {
    let fixture = Fixture::new();
    let (mut worker, original) = mixed(&fixture);
    let mut state = ready();
    open(&worker, &mut state, original.source.expected.id);
    let capture = preview(&worker, &mut state, original.source.operation_id, None);
    let command = state.confirm_undo(&capture).unwrap();
    let outer = command.0;
    assert!(
        matches!(&command.1, AppCommand::UndoProposal(request) if request == capture.request())
    );
    assert_eq!(state.last_undo_request.as_ref(), Some(capture.request()));
    assert_critical(&state);
    let (_, AppEvent::ProposalApplied(receipt)) = reply(&worker, command) else {
        panic!("whole Undo receipt");
    };
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    assert!(
        state
            .apply(Uuid::new_v4(), AppEvent::ProposalApplied(receipt.clone()))
            .is_empty()
    );
    assert!(state.apply(outer, AppEvent::Proposals(vec![])).is_empty());
    let mut wrong = receipt.clone();
    wrong.operation_id = original.source.operation_id;
    assert!(
        state
            .apply(outer, AppEvent::ProposalApplied(wrong))
            .is_empty()
    );
    let mut wrong = receipt.clone();
    wrong.proposal_id = original.source.expected.id;
    assert!(
        state
            .apply(outer, AppEvent::ProposalApplied(wrong))
            .is_empty()
    );
    assert!(state.pending.contains_key(&outer));
    assert_critical(&state);
    let followups = state.apply(outer, AppEvent::ProposalApplied(receipt.clone()));
    assert!(!state.pending.contains_key(&outer));
    assert_critical(&state); // Matching post-result review acknowledgement is still required.
    drain_reads(&worker, &mut state, followups);
    assert!(!state.application_busy());
    assert_eq!(state.approval_receipts, vec![receipt.clone()]);
    assert!(state.operation_error.is_none());
    assert_originals(&fixture, &original);
    let journal = inspect(&worker, &mut state, capture.request().operation_id);
    assert_eq!(journal.receipt.as_ref(), Some(&receipt));
    assert_eq!(journal.undo.as_ref(), Some(&capture.preview().binding));
    assert_eq!(state.activity.as_ref().unwrap().entries.len(), 2);
    assert_eq!(
        state.activity.as_ref().unwrap().entries[0]
            .undo
            .as_ref()
            .unwrap()
            .operation_id,
        original.source.operation_id
    );
    worker.shutdown().unwrap();

    fs::write(fixture.vault().join("replace.md"), LATER).unwrap();
    fs::write(
        fixture.vault().join("created.md"),
        "Later new occupant 日本語\r\n",
    )
    .unwrap();
    let later_files = files(&fixture);
    let mut worker = fixture.worker();
    let mut reopened = ready();
    open(&worker, &mut reopened, original.source.expected.id);
    let (id, event) = reply(&worker, (Uuid::new_v4(), capture.command()));
    let AppEvent::ProposalApplied(replayed) = event else {
        panic!("historical Undo replay");
    };
    assert_eq!(replayed, receipt);
    assert!(!reopened.pending.contains_key(&id));
    assert_eq!(files(&fixture), later_files);
    let journal = inspect(&worker, &mut reopened, capture.request().operation_id);
    assert_eq!(journal.receipt, Some(receipt));
    assert_eq!(journal.approved.draft, capture.preview().draft);
    worker.shutdown().unwrap();
}

#[test]
fn changed_target_after_preview_refuses_without_overwrite_and_retains_identified_request() {
    let fixture = Fixture::new();
    let (mut worker, original) = mixed(&fixture);
    let mut state = ready();
    open(&worker, &mut state, original.source.expected.id);
    let capture = preview(&worker, &mut state, original.source.operation_id, None);
    fs::write(fixture.vault().join("replace.md"), LATER).unwrap();
    let changed = files(&fixture);
    let command = state.confirm_undo(&capture).unwrap();
    let outer = command.0;
    assert_critical(&state);
    let (_, AppEvent::Failed(error)) = reply(&worker, command) else {
        panic!("stale target refusal");
    };
    assert_eq!(error.kind, ErrorKind::ContextStale);
    let followups = state.apply(outer, AppEvent::Failed(error));
    drain_reads(&worker, &mut state, followups);
    assert!(!state.application_busy());
    assert!(state.operation_error.is_some());
    assert_eq!(state.last_undo_request.as_ref(), Some(capture.request()));
    assert!(state.approval_receipts.is_empty());
    assert_eq!(files(&fixture), changed);
    assert!(state.application_snapshot.is_none());
    assert!(state.applies.is_empty());
    assert_eq!(state.activity.as_ref().unwrap().entries.len(), 1);
    worker.shutdown().unwrap();
}

#[test]
fn original_trash_member_two_restores_only_trash_and_keeps_unrelated_later_edit() {
    let fixture = Fixture::new();
    let (mut worker, original) = mixed(&fixture);
    let mut state = ready();
    open(&worker, &mut state, original.source.expected.id);
    fs::write(fixture.vault().join("replace.md"), LATER).unwrap();
    let before_replace = fs::metadata(fixture.vault().join("replace.md"))
        .unwrap()
        .ino();
    let before_create = fs::metadata(fixture.vault().join("created.md"))
        .unwrap()
        .ino();
    let capture = preview(&worker, &mut state, original.source.operation_id, Some(2));
    assert_eq!(capture.request().trash_member, Some(2));
    assert_eq!(capture.preview().binding.trash_member, Some(2));
    assert_eq!(capture.preview().draft.changes.len(), 1);
    assert!(
        matches!(&capture.preview().draft.changes[0], NoteChange::Create { path, text, .. } if path == "trash.md" && text == TRASH)
    );
    assert_eq!(
        capture.preview().binding.originals[0]
            .as_ref()
            .unwrap()
            .fingerprint,
        original.trash.baseline
    );
    let command = state.confirm_undo(&capture).unwrap();
    let (id, AppEvent::ProposalApplied(receipt)) = reply(&worker, command) else {
        panic!("single Trash restore");
    };
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    let followups = state.apply(id, AppEvent::ProposalApplied(receipt.clone()));
    drain_reads(&worker, &mut state, followups);
    assert_eq!(
        fs::read(fixture.vault().join("replace.md")).unwrap(),
        LATER.as_bytes()
    );
    assert_eq!(
        fs::metadata(fixture.vault().join("replace.md"))
            .unwrap()
            .ino(),
        before_replace
    );
    assert_eq!(
        fs::read(fixture.vault().join("created.md")).unwrap(),
        CREATED.as_bytes()
    );
    assert_eq!(
        fs::metadata(fixture.vault().join("created.md"))
            .unwrap()
            .ino(),
        before_create
    );
    assert_eq!(
        fs::read(fixture.vault().join("trash.md")).unwrap(),
        TRASH.as_bytes()
    );
    assert_eq!(
        fs::metadata(fixture.vault().join("trash.md"))
            .unwrap()
            .ino(),
        original.trash.baseline.inode
    );
    let activity = &state.activity.as_ref().unwrap().entries[0];
    assert_eq!(activity.undo.as_ref().unwrap().trash_member, Some(2));
    assert_eq!(activity.changes.len(), 1);
    assert_eq!(activity.changes[0].path, "trash.md");
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    let later_files = files(&fixture);
    let (_, AppEvent::ProposalApplied(replayed)) =
        reply(&worker, (Uuid::new_v4(), capture.command()))
    else {
        panic!("scoped historical replay");
    };
    assert_eq!(replayed, receipt);
    assert_eq!(files(&fixture), later_files);
    worker.shutdown().unwrap();
}

fn action(worker: &AppWorker, id: Uuid) -> ActionRecord {
    let (_, AppEvent::Action(record)) = reply(worker, (Uuid::new_v4(), AppCommand::Action(id)))
    else {
        panic!("complete Action record");
    };
    *record
}

fn action_worker(fixture: &Fixture) -> AppWorker {
    let worker = AppWorker::start(
        fixture.0.path().join("data"),
        AppConfig {
            vault_root: None,
            credentials_dir: Some(fixture.0.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready {
            vault_bound: false,
            model_installed: false
        }
    ));
    worker
}

fn action_replacement(
    fixture: &Fixture,
) -> (
    AppWorker,
    ApprovalRequest,
    Vec<ActionRecord>,
    Vec<ActionRecord>,
) {
    let worker = action_worker(fixture);
    let data = ActionData {
        title: "\u{feff}Previous 日本語 λ\r\n".into(),
        description: "Full previous details 🧭\r\n".into(),
        state: ActionState::Waiting,
        owner: Some("Owner õ".into()),
        related_person: None,
        related_project: None,
        sources: vec![],
        thread: None,
        due_on: Some("2028-02-29".into()),
        follow_up_on: Some("2028-03-01".into()),
        dependencies: vec![],
        parent: None,
        follows_up: None,
        priority: Some(brn_workflow::actions::ActionPriority::High),
    };
    let ids = [Uuid::new_v4(), Uuid::new_v4()];
    let draft = |title: &str, action_changes| DraftRequest {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: title.into(),
        changes: vec![],
        action_changes,
        sources: vec![],
        intake: None,
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
    };
    let (_, AppEvent::Proposal(created)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(draft(
                "Original Actions",
                ids.iter()
                    .map(|id| ActionChange::Create {
                        id: *id,
                        data: data.clone(),
                    })
                    .collect(),
            )),
        ),
    ) else {
        panic!("Action creation review")
    };
    let (_, AppEvent::ProposalApplied(created_receipt)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::ApproveProposal(ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: created.stamp(),
            }),
        ),
    ) else {
        panic!("original Actions Applied")
    };
    assert_eq!(created_receipt.outcome, ApplyOutcome::Applied);
    let originals: Vec<_> = ids.iter().map(|id| action(&worker, *id)).collect();
    let (_, AppEvent::Proposal(replaced)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(draft(
                "Approved incorrect details",
                originals
                    .iter()
                    .enumerate()
                    .map(|(index, before)| ActionChange::Replace {
                        before: Box::new(before.clone()),
                        data: ActionData {
                            title: format!("Approved changed title {index} Ελληνικά\r\n"),
                            description: "Complete changed description\r\n".into(),
                            state: if index == 0 {
                                ActionState::Open
                            } else {
                                ActionState::Waiting
                            },
                            owner: None,
                            priority: Some(brn_workflow::actions::ActionPriority::Low),
                            ..before.data.clone()
                        },
                    })
                    .collect(),
            )),
        ),
    ) else {
        panic!("Action replacement review")
    };
    let source = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: replaced.stamp(),
    };
    let (_, AppEvent::ProposalApplied(receipt)) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::ApproveProposal(source.clone())),
    ) else {
        panic!("Action replacement Applied");
    };
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    let installed = ids.iter().map(|id| action(&worker, *id)).collect();
    (worker, source, originals, installed)
}

#[test]
fn action_only_full_capture_stale_preview_receipt_guards_and_compensation_preserve_history() {
    let fixture = Fixture::new();
    let (mut worker, source, originals, installed) = action_replacement(&fixture);
    let mut state = ready();
    state.vault_bound = false;
    open(&worker, &mut state, source.expected.id);
    let old_command = state.preview_undo(source.operation_id, None).unwrap();
    let (old_id, old_event) = reply(&worker, old_command);
    let command = state.preview_undo(source.operation_id, None).unwrap();
    let (outer, AppEvent::ProposalUndoPreview(full)) = reply(&worker, command) else {
        panic!("Action inverse preview")
    };
    assert!(state.apply(old_id, old_event).is_empty());
    assert!(state.undo_preview.is_none());
    let mut wrong = full.clone();
    wrong.binding.operation_id = Uuid::new_v4();
    state.apply(outer, AppEvent::ProposalUndoPreview(wrong));
    assert!(state.pending.contains_key(&outer));
    assert!(state.undo_preview.is_none());
    state.apply(outer, AppEvent::ProposalUndoPreview(full.clone()));
    let capture = state.undo_preview.as_ref().unwrap().clone();
    assert_eq!(capture.preview(), &full);
    assert!(full.draft.changes.is_empty() && full.binding.originals.is_empty());
    assert_eq!(full.draft.action_changes.len(), 2);
    for ((change, original), exact) in full
        .draft
        .action_changes
        .iter()
        .zip(&originals)
        .zip(&installed)
    {
        assert!(
            matches!(change, ActionChange::Replace { before, data } if before.as_ref() == exact && data == &original.data)
        );
        assert_eq!(action(&worker, exact.origin.id), *exact); // Preview has no effects.
    }
    assert!(state.last_undo_request.is_none());
    let stale = capture.clone();
    let newest = preview(&worker, &mut state, source.operation_id, None);
    assert!(state.confirm_undo(&stale).is_none());
    let command = state.confirm_undo(&newest).unwrap();
    let outer = command.0;
    assert_critical(&state);
    let (_, AppEvent::ProposalApplied(receipt)) = reply(&worker, command) else {
        panic!("Action compensation receipt")
    };
    assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    state.apply(Uuid::new_v4(), AppEvent::ProposalApplied(receipt.clone()));
    for mismatch in 0..4 {
        let mut wrong = receipt.clone();
        match mismatch {
            0 => wrong.operation_id = source.operation_id,
            1 => wrong.proposal_id = source.expected.id,
            2 => wrong.approved_version += 1,
            _ => wrong.stamp.version = 2,
        }
        state.apply(outer, AppEvent::ProposalApplied(wrong));
        assert!(state.pending.contains_key(&outer));
        assert_critical(&state);
    }
    let commands = state.apply(outer, AppEvent::ProposalApplied(receipt.clone()));
    drain_reads(&worker, &mut state, commands);
    assert!(!state.application_busy());
    assert_eq!(state.approval_receipts, vec![receipt.clone()]);
    let restored: Vec<_> = originals
        .iter()
        .map(|original| action(&worker, original.origin.id))
        .collect();
    for ((restored, original), installed) in restored.iter().zip(&originals).zip(&installed) {
        assert_eq!(restored.data, original.data);
        assert_eq!(restored.origin, original.origin);
        assert_eq!(restored.version, installed.version + 1);
        assert!(restored.updated_at_ms >= installed.updated_at_ms);
        assert_eq!(restored.completed_at_ms, original.completed_at_ms);
    }
    assert_eq!(
        restored[0].waiting_since_ms,
        Some(restored[0].updated_at_ms)
    );
    assert_eq!(restored[1].waiting_since_ms, installed[1].waiting_since_ms);
    let journal = inspect(&worker, &mut state, newest.request().operation_id);
    assert_eq!(journal.approved.draft, newest.preview().draft);
    assert_eq!(journal.action_records, restored);
    assert_eq!(journal.receipt, Some(receipt.clone()));
    worker.shutdown().unwrap();
    let mut worker = action_worker(&fixture);
    for expected in &restored {
        assert_eq!(action(&worker, expected.origin.id), *expected);
    }
    let (_, AppEvent::ProposalApplied(replay)) = reply(&worker, (Uuid::new_v4(), newest.command()))
    else {
        panic!("compensation replay")
    };
    assert_eq!(replay, receipt);
    for expected in &restored {
        assert_eq!(action(&worker, expected.origin.id), *expected);
    }
    worker.shutdown().unwrap();
}

#[test]
fn action_completed_after_preview_refuses_compensation_and_retains_identified_request() {
    let fixture = Fixture::new();
    let (mut worker, source, _, installed) = action_replacement(&fixture);
    let mut state = ready();
    state.vault_bound = false;
    open(&worker, &mut state, source.expected.id);
    let capture = preview(&worker, &mut state, source.operation_id, None);
    let completion = CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(installed[0].clone()),
        sent_source: None,
    };
    assert!(matches!(
        reply(
            &worker,
            (
                completion.operation_id,
                AppCommand::CompleteAction(completion)
            )
        )
        .1,
        AppEvent::ActionCompleted(_)
    ));
    let completed = action(&worker, installed[0].origin.id);
    let command = state.confirm_undo(&capture).unwrap();
    let (outer, AppEvent::Failed(error)) = reply(&worker, command) else {
        panic!("completed Action refusal")
    };
    assert_eq!(error.kind, ErrorKind::ContextStale);
    let commands = state.apply(outer, AppEvent::Failed(error));
    drain_reads(&worker, &mut state, commands);
    assert!(!state.application_busy());
    assert!(state.operation_error.is_some());
    assert_eq!(state.last_undo_request.as_ref(), Some(capture.request()));
    assert!(state.approval_receipts.is_empty());
    assert_eq!(action(&worker, completed.origin.id), completed);
    assert_eq!(action(&worker, installed[1].origin.id), installed[1]);
    worker.shutdown().unwrap();
}

#[test]
fn vaultless_state_cannot_confirm_file_inverse_or_request_scoped_trash_or_repair() {
    let fixture = Fixture::new();
    let (mut worker, original) = mixed(&fixture);
    let mut state = ready();
    open(&worker, &mut state, original.source.expected.id);
    let capture = preview(&worker, &mut state, original.source.operation_id, None);
    state.vault_bound = false;
    assert!(state.confirm_undo(&capture).is_none());
    assert!(
        state
            .preview_undo(original.source.operation_id, Some(2))
            .is_none()
    );
    assert!(
        state
            .preview_repair(original.source.operation_id, RepairDirection::Finish)
            .is_none()
    );
    assert!(
        state
            .preview_repair(original.source.operation_id, RepairDirection::Restore)
            .is_none()
    );
    // Even an exact file preview delivered to a whole-operation request cannot
    // turn vaultless Action eligibility into permission to confirm file work.
    let capture = preview(&worker, &mut state, original.source.operation_id, None);
    assert!(state.confirm_undo(&capture).is_none());
    let command = state
        .preview_undo(original.source.operation_id, None)
        .unwrap();
    let outer = command.0;
    let (_, AppEvent::ProposalUndoPreview(mut forged)) = reply(&worker, command) else {
        panic!("full file preview")
    };
    forged.binding.trash_member = Some(2);
    state.apply(outer, AppEvent::ProposalUndoPreview(forged));
    assert!(state.undo_preview.is_none());
    assert!(state.pending.contains_key(&outer));
    assert!(state.last_undo_request.is_none());
    assert!(!state.application_busy());
    worker.shutdown().unwrap();
}
