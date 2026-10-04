use super::*;
mod undo {
    include!("undo_state_tests.rs");
}
mod repair {
    include!("repair_state_tests.rs");
}
mod link_preparation {
    include!("link_preparation_state_tests.rs");
}
mod creation {
    include!("draft_state_tests.rs");
}
use brn_workflow::{
    app::AppConfig,
    app_worker::AppWorker,
    proposal_apply::{ApplySummary, GroupApprovalResult},
    proposals::{CommentTarget, DraftNoteChange, DraftRequest},
};
use std::{
    collections::VecDeque,
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::Path,
};

struct Fixture(tempfile::TempDir);

impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        assert!(!owner.path().starts_with(env!("CARGO_MANIFEST_DIR")));
        fs::create_dir(owner.path().join("data")).unwrap();
        fs::create_dir(owner.path().join("vault")).unwrap();
        Self(owner)
    }
    fn vault(&self) -> PathBuf {
        self.0.path().join("vault")
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(
            self.0.path().join("data"),
            AppConfig {
                vault_root: Some(self.vault()),
                credentials_dir: Some(self.0.path().join("credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        loop {
            match worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1
            {
                AppEvent::Ready { vault_bound, .. } => {
                    assert!(vault_bound);
                    return worker;
                }
                AppEvent::Failed(error) => panic!("startup {:?}: {}", error.kind, error.message),
                _ => {}
            }
        }
    }
}

fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> (Uuid, AppEvent) {
    let (id, command) = command;
    worker.submit(id, command).unwrap();
    loop {
        let event = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if event.0 == id {
            return event;
        }
    }
}

fn record(worker: &AppWorker, id: Uuid) -> ProposalRecord {
    let (_, AppEvent::Proposal(record)) = reply(worker, (Uuid::new_v4(), AppCommand::Proposal(id)))
    else {
        panic!("full proposal record");
    };
    record
}

fn create(worker: &AppWorker, path: &str, group: Option<Uuid>) -> ProposalRecord {
    let (_, AppEvent::Proposal(record)) = reply(
        worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(DraftRequest {
                action_changes: Vec::new(),
                id: Uuid::new_v4(),
                group_id: group,
                session_id: None,
                title: format!("\u{feff}Full 日本語 🧭\r\n{path}"),
                changes: vec![DraftNoteChange::Create {
                    path: path.into(),
                    text: "\u{feff}# 正文 café 🦀\r\n第二 λ\r\n".into(),
                }],
                sources: vec![],
            }),
        ),
    ) else {
        panic!("create proposal");
    };
    let (_, AppEvent::Proposal(commented)) = reply(
        worker,
        (
            Uuid::new_v4(),
            AppCommand::AddProposalComment(CommentRequest {
                expected: record.stamp(),
                comment: ReviewComment {
                    id: Uuid::new_v4(),
                    text: "\u{feff}Temporary 日本語 λ\r\n".into(),
                    target: CommentTarget::Proposal,
                },
            }),
        ),
    ) else {
        panic!("comment proposal");
    };
    commented
}

fn drain_reads(worker: &AppWorker, state: &mut AiState, commands: Vec<(Uuid, AppCommand)>) {
    let mut commands = VecDeque::from(commands);
    while let Some(command) = commands.pop_front() {
        assert!(
            matches!(
                command.1,
                AppCommand::Proposal(_)
                    | AppCommand::Proposals(_)
                    | AppCommand::Activity(_)
                    | AppCommand::ProposalRecovery
                    | AppCommand::ProposalApply(_)
                    | AppCommand::Notes { .. }
                    | AppCommand::ScopedNotes { .. }
            ),
            "terminal followups must be read only; no retry or provider commands"
        );
        let (id, event) = reply(worker, command);
        if let AppEvent::Failed(error) = &event {
            panic!("read followup {:?}: {}", error.kind, error.message);
        }
        commands.extend(state.apply(id, event));
    }
}

fn open(worker: &AppWorker, state: &mut AiState, proposal: Uuid) {
    let command = state.open_review(proposal).unwrap();
    let (id, event) = reply(worker, command);
    let commands = state.apply(id, event);
    drain_reads(worker, state, commands);
}

fn load_proposals(worker: &AppWorker, state: &mut AiState) {
    let command = state.command(Pending::Proposals, AppCommand::Proposals(None));
    let (id, event) = reply(worker, command);
    assert!(state.apply(id, event).is_empty());
}

fn inspect(worker: &AppWorker, state: &mut AiState, operation: Uuid) -> ApplyJournal {
    let command = state.inspect_apply(operation).unwrap();
    assert!(matches!(command.1, AppCommand::ProposalApply(id) if id == operation));
    let (id, event) = reply(worker, command);
    let commands = state.apply(id, event);
    drain_reads(worker, state, commands);
    let journal = state.application_snapshot.as_ref().unwrap();
    assert_eq!(journal.request.operation_id, operation);
    journal.clone()
}

fn assert_critical(state: &AiState) {
    assert!(state.application_busy());
    assert!(!state.review_can_leave());
    assert!(!state.review_editable());
    assert!(!state.review_can_mutate());
    assert!(!state.can_ask());
    assert!(state.capture_approval(false).is_none());
}

#[test]
fn exact_approval_requires_current_ack_and_replay_preserves_later_manual_bytes() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let original = create(&worker, "approved.md", None);
    let mut state = ready();
    open(&worker, &mut state, original.draft.id);
    let old = state.capture_approval(false).unwrap();
    let edited = "\u{feff}Manual proposal edit 日本語 🧭\r\nλ\r\n";
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, edited.into(), Instant::now())
        .unwrap();
    assert!(state.confirm_approval(&old).is_none());
    let command = state.recover_review().unwrap();
    let (id, event) = reply(&worker, command);
    let commands = state.apply(id, event);
    drain_reads(&worker, &mut state, commands);
    assert!(state.confirm_approval(&old).is_none());
    let capture = state.capture_approval(false).unwrap();
    assert_eq!(capture.records()[0].draft.title, original.draft.title);
    assert_eq!(capture.records()[0].draft.changes[0].text(), Some(edited));
    assert_eq!(capture.records()[0].comments, original.comments);
    let command = state.confirm_approval(&capture).unwrap();
    let outer = command.0;
    let AppCommand::ApproveProposal(request) = &command.1 else {
        panic!("individual approval");
    };
    assert_eq!(request, &capture.requests()[0]);
    assert_eq!(request.expected, capture.records()[0].stamp());
    assert_critical(&state);
    let (_, AppEvent::ProposalApplied(applied)) = reply(&worker, command) else {
        panic!("Applied receipt");
    };
    assert_eq!(applied.outcome, ApplyOutcome::Applied);
    assert!(
        state
            .apply(Uuid::new_v4(), AppEvent::ProposalApplied(applied.clone()))
            .is_empty()
    );
    assert!(state.pending.contains_key(&outer));
    let mut wrong = applied.clone();
    wrong.operation_id = Uuid::new_v4();
    assert!(
        state
            .apply(outer, AppEvent::ProposalApplied(wrong))
            .is_empty()
    );
    assert!(state.pending.contains_key(&outer));
    let commands = state.apply(outer, AppEvent::ProposalApplied(applied.clone()));
    assert!(!state.pending.contains_key(&outer));
    assert_critical(&state); // The full post-application review has not acknowledged yet.
    let acknowledgement = commands
        .iter()
        .find_map(|(id, _)| {
            matches!(state.pending.get(id), Some(Pending::AppliedReview { .. })).then_some(*id)
        })
        .unwrap();
    let mut other = state.review.as_ref().unwrap().record.clone();
    other.draft.id = Uuid::new_v4();
    state.apply(acknowledgement, AppEvent::Proposal(other));
    assert!(state.pending.contains_key(&acknowledgement));
    assert_critical(&state);
    let mut misbound = state.review.as_ref().unwrap().record.clone();
    if let brn_workflow::proposals::NoteChange::Create { path, .. } = &mut misbound.draft.changes[0]
    {
        *path = "different.md".into();
    }
    state.apply(acknowledgement, AppEvent::Proposal(misbound));
    assert!(state.pending.contains_key(&acknowledgement));
    assert_critical(&state);
    state.apply(acknowledgement, AppEvent::Proposals(vec![]));
    assert!(state.pending.contains_key(&acknowledgement));
    assert_critical(&state);
    assert!(
        commands
            .iter()
            .any(|(id, command)| matches!(command, AppCommand::Proposal(_))
                && matches!(state.pending.get(id), Some(Pending::AppliedReview { .. })))
    );
    drain_reads(&worker, &mut state, commands);
    assert!(!state.application_busy());
    assert!(state.review_can_leave());
    let approved = &state.review.as_ref().unwrap().record;
    assert_eq!(approved.state, ProposalState::Applied);
    assert!(approved.comments.is_empty());
    assert_eq!(approved.draft.changes[0].text(), Some(edited));
    assert_eq!(
        fs::read(fixture.vault().join("approved.md")).unwrap(),
        edited.as_bytes()
    );
    assert_eq!(state.activity.as_ref().unwrap().entries.len(), 1);
    assert_eq!(
        state.activity.as_ref().unwrap().entries[0].operation_id,
        applied.operation_id
    );
    assert!(state.applies.is_empty());
    assert!(state.application_snapshot.is_none());
    let journal = inspect(&worker, &mut state, applied.operation_id);
    assert!(journal.approved.comments.is_empty());
    worker.shutdown().unwrap();

    let later = "\u{feff}Later explicit user bytes Ελληνικά\r\n";
    fs::write(fixture.vault().join("approved.md"), later).unwrap();
    let metadata = fs::metadata(fixture.vault().join("approved.md")).unwrap();
    let mut worker = fixture.worker();
    let (_, AppEvent::ProposalApplied(replayed)) =
        reply(&worker, (Uuid::new_v4(), capture.command()))
    else {
        panic!("durable replay");
    };
    assert_eq!(replayed, applied);
    assert_eq!(
        fs::read(fixture.vault().join("approved.md")).unwrap(),
        later.as_bytes()
    );
    assert_eq!(
        fs::metadata(fixture.vault().join("approved.md"))
            .unwrap()
            .ino(),
        metadata.ino()
    );
    assert_eq!(fs::read_dir(fixture.vault()).unwrap().count(), 1);
    let mut reopened = ready();
    open(&worker, &mut reopened, original.draft.id);
    assert!(reopened.review.as_ref().unwrap().record.comments.is_empty());
    let (_, AppEvent::Activity(history)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::Activity(ActivityRequest::default()),
        ),
    ) else {
        panic!("durable history");
    };
    assert_eq!(history.entries.len(), 1);
    assert_eq!(history.entries[0].operation_id, applied.operation_id);
    worker.shutdown().unwrap();
}

#[test]
fn captured_group_excludes_late_arrival_and_stops_after_second_preflight_refusal() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let group = Uuid::new_v4();
    let current = create(&worker, "one.md", Some(group));
    create(&worker, "two.md", Some(group));
    let mut state = ready();
    open(&worker, &mut state, current.draft.id);
    load_proposals(&worker, &mut state);
    let capture = state.capture_approval(true).unwrap();
    assert_eq!(capture.records().len(), 2);
    let first = &capture.records()[0];
    let second = &capture.records()[1];
    let late = create(&worker, "late.md", Some(group));
    load_proposals(&worker, &mut state);
    let occupant = "\u{feff}Unexpected user occupant 🦀\r\n";
    fs::write(
        fixture.vault().join(second.draft.changes[0].path()),
        occupant,
    )
    .unwrap();
    let command = state.confirm_approval(&capture).unwrap();
    let outer = command.0;
    let AppCommand::ApproveProposalGroup(request) = &command.1 else {
        panic!("group approval");
    };
    assert_eq!(request.approvals, capture.requests());
    assert!(
        !request
            .approvals
            .iter()
            .any(|request| request.expected.id == late.draft.id)
    );
    assert_critical(&state);
    let (_, AppEvent::ProposalGroupApplied(result)) = reply(&worker, command) else {
        panic!("group result");
    };
    assert_eq!(result.receipts.len(), 1);
    assert_eq!(
        result.receipts[0].operation_id,
        capture.requests()[0].operation_id
    );
    assert_eq!(result.receipts[0].outcome, ApplyOutcome::Applied);
    assert_eq!(
        result.stopped.as_ref().unwrap().operation_id,
        capture.requests()[1].operation_id
    );
    let wrong = GroupApprovalResult {
        receipts: vec![ApplyReceipt {
            operation_id: Uuid::new_v4(),
            ..result.receipts[0].clone()
        }],
        stopped: result.stopped.clone(),
    };
    assert!(
        state
            .apply(outer, AppEvent::ProposalGroupApplied(wrong))
            .is_empty()
    );
    assert!(state.pending.contains_key(&outer));
    let commands = state.apply(outer, AppEvent::ProposalGroupApplied(result));
    assert_critical(&state);
    drain_reads(&worker, &mut state, commands);
    assert_eq!(
        fs::read(fixture.vault().join(first.draft.changes[0].path())).unwrap(),
        first.draft.changes[0].text().unwrap().as_bytes()
    );
    assert_eq!(
        fs::read(fixture.vault().join(second.draft.changes[0].path())).unwrap(),
        occupant.as_bytes()
    );
    assert!(!fixture.vault().join("late.md").exists());
    assert_eq!(record(&worker, second.draft.id), *second);
    assert_eq!(record(&worker, late.draft.id), late);
    assert!(record(&worker, first.draft.id).comments.is_empty());
    assert_eq!(state.approval_receipts.len(), 1);
    assert!(state.approval_error.is_some());
    assert!(state.applies.is_empty()); // No unresolved application remains after refusal.
    assert_eq!(
        inspect(&worker, &mut state, capture.requests()[0].operation_id)
            .receipt
            .as_ref()
            .unwrap()
            .outcome,
        ApplyOutcome::Applied
    );
    let (_, AppEvent::ProposalApply(second_journal)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::ProposalApply(capture.requests()[1].operation_id),
        ),
    ) else {
        panic!("explicit second journal lookup");
    };
    assert!(second_journal.is_none()); // Occupied destination is refused before a second intent.
    assert_eq!(state.activity.as_ref().unwrap().entries.len(), 1);
    worker.shutdown().unwrap();
}

/// Restore only this owned child's captured inode and owner, including on panic.
struct UnwritableChild {
    path: PathBuf,
    identity: (u64, u64, u32),
    permissions: fs::Permissions,
}
impl UnwritableChild {
    fn new(fixture: &Fixture, path: &Path) -> Self {
        assert_eq!(path.parent(), Some(fixture.vault().as_path()));
        let metadata = fs::symlink_metadata(path).unwrap();
        assert!(metadata.is_dir());
        assert_eq!(
            metadata.uid(),
            fs::metadata(fixture.0.path()).unwrap().uid()
        );
        let guard = Self {
            path: path.into(),
            identity: (metadata.dev(), metadata.ino(), metadata.uid()),
            permissions: metadata.permissions(),
        };
        fs::set_permissions(path, fs::Permissions::from_mode(0o500)).unwrap();
        guard
    }
}
impl Drop for UnwritableChild {
    fn drop(&mut self) {
        if let Ok(metadata) = fs::symlink_metadata(&self.path)
            && metadata.is_dir()
            && (metadata.dev(), metadata.ino(), metadata.uid()) == self.identity
        {
            let _ = fs::set_permissions(&self.path, self.permissions.clone());
        }
    }
}

#[test]
fn failed_preparation_records_not_applied_and_explicit_reconcile_preserves_original_occupant() {
    let fixture = Fixture::new();
    let child = fixture.vault().join("owned-child");
    fs::create_dir(&child).unwrap();
    let original = "\u{feff}Original user occupant 日本語\r\n";
    fs::write(child.join("existing.md"), original).unwrap();
    let mut worker = fixture.worker();
    let (_, AppEvent::Editor(view)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::OpenEditor("owned-child/existing.md".into()),
        ),
    ) else {
        panic!("trusted original");
    };
    let (_, AppEvent::Proposal(created)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::CreateProposal(DraftRequest {
                action_changes: Vec::new(),
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Replace retained original λ".into(),
                changes: vec![DraftNoteChange::Replace {
                    path: "owned-child/existing.md".into(),
                    expected: view.observed.unwrap(),
                    text: "\u{feff}Proposed replacement 🧭\r\n".into(),
                }],
                sources: vec![],
            }),
        ),
    ) else {
        panic!("Replace draft");
    };
    let mut state = ready();
    open(&worker, &mut state, created.draft.id);
    let capture = state.capture_approval(false).unwrap();
    let permissions = UnwritableChild::new(&fixture, &child);
    let command = state.confirm_approval(&capture).unwrap();
    let (outer, event) = reply(&worker, command);
    assert!(
        matches!(event, AppEvent::Failed(_)),
        "staging in a non-writable owned child must fail"
    );
    drop(permissions);
    let commands = state.apply(outer, event);
    assert!(
        state
            .notice
            .contains("an error does not establish no file effects")
    );
    assert_eq!(state.approval_requests, capture.requests());
    assert_critical(&state);
    drain_reads(&worker, &mut state, commands);
    assert!(state.applies.is_empty());
    let journal = inspect(&worker, &mut state, capture.requests()[0].operation_id);
    assert_eq!(journal.request, capture.requests()[0]);
    assert_eq!(
        journal.receipt.as_ref().unwrap().outcome,
        ApplyOutcome::NotApplied
    );
    assert!(journal.no_effects);
    assert_eq!(
        state.review.as_ref().unwrap().record.state,
        ProposalState::Draft
    );
    assert!(state.activity.as_ref().unwrap().entries.is_empty());
    assert_eq!(
        fs::read(child.join("existing.md")).unwrap(),
        original.as_bytes()
    );
    assert!(state.reconcile_apply(Uuid::new_v4()).is_none());
    assert!(state.reconcile_apply(Uuid::nil()).is_none());
    let command = state.reconcile_apply(journal.request.operation_id).unwrap();
    assert!(
        matches!(command.1, AppCommand::ReconcileProposal(id) if id == journal.request.operation_id)
    );
    assert_critical(&state);
    let (id, event) = reply(&worker, command);
    let AppEvent::ProposalApplied(receipt) = &event else {
        panic!("explicit recorded reconciliation");
    };
    assert_eq!(receipt, journal.receipt.as_ref().unwrap());
    let commands = state.apply(id, event);
    drain_reads(&worker, &mut state, commands);
    assert!(state.applies.is_empty());
    assert_eq!(
        inspect(&worker, &mut state, journal.request.operation_id),
        journal
    );
    assert_eq!(
        fs::read(child.join("existing.md")).unwrap(),
        original.as_bytes()
    );
    assert_eq!(fs::read_dir(&child).unwrap().count(), 1);
    worker.shutdown().unwrap();
}

#[test]
fn late_local_text_before_applied_review_ack_remains_an_explicit_conflict() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let created = create(&worker, "late-local.md", None);
    let mut state = ready();
    open(&worker, &mut state, created.draft.id);
    let capture = state.capture_approval(false).unwrap();
    let command = state.confirm_approval(&capture).unwrap();
    let (id, event) = reply(&worker, command);
    let commands = state.apply(id, event);
    assert_critical(&state);
    let later = "\u{feff}Late local typing Ελληνικά 🦀\r\n";
    state
        .review
        .as_mut()
        .unwrap()
        .edit_text(0, later.into(), Instant::now())
        .unwrap();
    drain_reads(&worker, &mut state, commands);
    let review = state.review.as_ref().unwrap();
    assert_eq!(review.text(0), Some(later));
    assert_eq!(review.record, created);
    assert_eq!(
        review.observed.as_ref().unwrap().state,
        ProposalState::Applied
    );
    assert!(review.observed.as_ref().unwrap().comments.is_empty());
    assert!(!state.review_can_leave() && !state.review_can_mutate());
    assert!(state.capture_approval(false).is_none());
    assert_eq!(
        fs::read(fixture.vault().join("late-local.md")).unwrap(),
        created.draft.changes[0].text().unwrap().as_bytes()
    );
    worker.shutdown().unwrap();
}

fn activity(worker: &AppWorker, command: (Uuid, AppCommand), limit: usize) -> (Uuid, ActivityPage) {
    let (id, AppCommand::Activity(mut request)) = command else {
        panic!("activity request");
    };
    request.limit = limit;
    let (_, AppEvent::Activity(page)) = reply(worker, (id, AppCommand::Activity(request))) else {
        panic!("actual activity page");
    };
    (id, page)
}

#[test]
fn actual_paginated_history_ignores_old_generations_and_deduplicates_more_rows() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    for index in 0..3 {
        let draft = create(&worker, &format!("page-{index}.md"), None);
        let (_, AppEvent::ProposalApplied(receipt)) = reply(
            &worker,
            (
                Uuid::new_v4(),
                AppCommand::ApproveProposal(ApprovalRequest {
                    operation_id: Uuid::new_v4(),
                    expected: draft.stamp(),
                }),
            ),
        ) else {
            panic!("history fixture approval");
        };
        assert_eq!(receipt.outcome, ApplyOutcome::Applied);
    }
    let mut state = ready();
    let old_failure = state.refresh_activity().unwrap().0;
    let old_first = state.refresh_activity().unwrap();
    let (old_first_id, old_page) = activity(&worker, old_first, 1);
    let current = state.refresh_activity().unwrap();
    let (current_id, first) = activity(&worker, current, 1);
    state.apply(current_id, AppEvent::Activity(first.clone()));
    state.apply(old_first_id, AppEvent::Activity(old_page));
    state.apply(
        old_failure,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("Old first page failure")),
    );
    assert_eq!(state.activity, Some(first.clone()));
    assert!(state.activity_error.is_none());
    let more = state.more_activity().unwrap();
    let (more_id, mut remainder) = activity(&worker, more, 20);
    assert_eq!(remainder.entries.len(), 2);
    remainder.entries.insert(0, first.entries[0].clone());
    state.apply(more_id, AppEvent::Activity(remainder));
    assert_eq!(state.activity.as_ref().unwrap().entries.len(), 3);
    assert!(state.more_activity().is_none());
    let current_page = state.activity.clone();
    state.apply(more_id, AppEvent::Activity(first.clone()));
    assert_eq!(state.activity, current_page);

    let old = state.refresh_activity().unwrap();
    let (old_id, old_first) = activity(&worker, old, 1);
    state.apply(old_id, AppEvent::Activity(old_first));
    let old_more = state.more_activity().unwrap();
    let (old_more_id, old_more_page) = activity(&worker, old_more, 20);
    let current = state.refresh_activity().unwrap();
    let (id, refreshed) = activity(&worker, current, 20);
    state.apply(id, AppEvent::Activity(refreshed.clone()));
    state.apply(old_more_id, AppEvent::Activity(old_more_page));
    assert_eq!(state.activity, Some(refreshed));

    assert!(state.application_snapshot.is_none());
    let operations: Vec<_> = state
        .activity
        .as_ref()
        .unwrap()
        .entries
        .iter()
        .map(|entry| entry.operation_id)
        .collect();
    let old_failure = state.refresh_applies().unwrap().0;
    let old = state.refresh_applies().unwrap().0;
    let current = state.refresh_applies().unwrap();
    let (id, AppEvent::ProposalRecovery(summaries)) = reply(&worker, current) else {
        panic!("bounded actual recovery summaries");
    };
    assert!(summaries.is_empty()); // Completed history never retains full journals in the recovery list.
    state.apply(id, AppEvent::ProposalRecovery(summaries));
    let (_, AppEvent::ProposalApply(Some(historical))) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::ProposalApply(operations[0])),
    ) else {
        panic!("explicit historical journal");
    };
    // A stale uncertainty summary cannot resurrect recovery after a newer read.
    let stale = ApplySummary {
        request: historical.request.clone(),
        title: historical.approved.draft.title.clone(),
        outcome: Some(ApplyOutcome::Uncertain),
        started_at_ms: historical.started_at_ms,
    };
    state.apply(old, AppEvent::ProposalRecovery(vec![stale]));
    state.apply(
        old_failure,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("Old journal failure")),
    );
    assert!(state.applies.is_empty());
    assert!(state.applies_error.is_none());

    let old_failure = state.inspect_apply(operations[0]).unwrap().0;
    let older = state.inspect_apply(operations[0]).unwrap();
    let (older_id, AppEvent::ProposalApply(older_body)) = reply(&worker, older) else {
        panic!("old selected snapshot");
    };
    let newest = state.inspect_apply(operations[1]).unwrap();
    let (newest_id, AppEvent::ProposalApply(Some(newest_body))) = reply(&worker, newest) else {
        panic!("new selected snapshot");
    };
    assert!(state.application_snapshot.is_none());
    state.apply(
        Uuid::new_v4(),
        AppEvent::ProposalApply(Some(newest_body.clone())),
    );
    assert!(state.application_snapshot.is_none());
    state.apply(newest_id, AppEvent::ProposalRecovery(vec![]));
    assert!(state.pending.contains_key(&newest_id));
    state.apply(newest_id, AppEvent::ProposalApply(older_body.clone()));
    assert!(state.application_snapshot.is_none());
    assert!(state.pending.contains_key(&newest_id));
    state.apply(
        newest_id,
        AppEvent::ProposalApply(Some(newest_body.clone())),
    );
    state.apply(older_id, AppEvent::ProposalApply(older_body));
    state.apply(
        old_failure,
        AppEvent::Failed(brn_workflow::WorkflowError::msg(
            "Old selected snapshot failure",
        )),
    );
    assert_eq!(state.application_snapshot, Some(*newest_body));
    assert!(state.snapshot_error.is_none());
    assert!(state.applies.is_empty());
    worker.shutdown().unwrap();
}
