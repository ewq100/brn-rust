use super::*;
use brn_workflow::{
    ErrorKind,
    actions::{ActionRecord, ActionState},
    action_completion::CompleteActionRequest,
    proposals::{ProposalEdit, ProposalSource},
};

const TEXT: &str = "\u{feff}# Saved Eesti 日本語\r\nexact λ\r\n";

fn fields(state: &mut AiState, title: &str) {
    let form = state.draft.as_mut().unwrap();
    let mut fields = form.action.as_ref().unwrap().fields.clone();
    fields.values[0] = title.into();
    fields.values[1] = TEXT.into();
    form.edit_action_fields(fields);
}
fn source(worker: &AppWorker, path: &str) -> ProposalSource {
    let (_, AppEvent::ProposalSource(source)) = reply(
        worker,
        (Uuid::new_v4(), AppCommand::ProposalSource(path.into())),
    ) else { panic!("source") };
    *source
}
fn action(worker: &AppWorker, id: Uuid) -> ActionRecord {
    let (_, AppEvent::Action(action)) = reply(worker, (Uuid::new_v4(), AppCommand::Action(id)))
    else { panic!("Action") };
    *action
}
fn vaultless(fixture: &Fixture) -> AppWorker {
    let worker = AppWorker::start(fixture.0.path().join("data"), AppConfig {
        vault_root: None,
        credentials_dir: Some(fixture.0.path().join("credentials")),
        model_dir: None,
    }).unwrap();
    assert!(matches!(worker.recv_event_timeout(Duration::from_secs(10)).unwrap().1,
        AppEvent::Ready { vault_bound: false, model_installed: false }));
    worker
}
fn create_form(worker: &AppWorker, state: &mut AiState) -> ProposalRecord {
    let command = state.create_draft().unwrap();
    let (id, AppEvent::Proposal(record)) = reply(worker, command) else { panic!("review") };
    let followups = state.apply(id, AppEvent::Proposal(record.clone()));
    drain_reads(worker, state, followups);
    record
}
fn approve(worker: &AppWorker, record: &ProposalRecord) {
    assert!(matches!(reply(worker, (Uuid::new_v4(), AppCommand::ApproveProposal(ApprovalRequest {
        operation_id: Uuid::new_v4(), expected: record.stamp(),
    }))).1, AppEvent::ProposalApplied(_)));
}

#[test]
fn owner_replacement_review_approval_restart_and_existing_undo_preserve_action_history() {
    use brn_workflow::{
        proposal_apply::{ApplyOutcome, UndoRequest},
        proposals::ActionChange,
    };
    let fixture = Fixture::new();
    let mut worker = vaultless(&fixture);
    let mut state = ready();
    state.vault_bound = false;
    assert!(state.begin_action_draft(None));
    fields(&mut state, "Original waiting work");
    let mut input = state
        .draft
        .as_ref()
        .unwrap()
        .action
        .as_ref()
        .unwrap()
        .fields
        .clone();
    input.state = ActionState::Waiting;
    state.draft.as_mut().unwrap().edit_action_fields(input);
    let created = create_form(&worker, &mut state);
    let action_id = created.draft.action_changes[0].id();
    approve(&worker, &created);
    let before = action(&worker, action_id);
    assert!(state.begin_action_replace_draft(before.clone()));
    fields(&mut state, "Updated exact owner work");
    let mut input = state
        .draft
        .as_ref()
        .unwrap()
        .action
        .as_ref()
        .unwrap()
        .fields
        .clone();
    input.state = ActionState::Blocked;
    input.values[2] = "Owner õ".into();
    input.values[7] = "2028-03-01".into();
    input.values[8] = "2028-03-02".into();
    state
        .draft
        .as_mut()
        .unwrap()
        .edit_action_fields(input.clone());
    let rejected = create_form(&worker, &mut state);
    assert!(
        matches!(&rejected.draft.action_changes[0], ActionChange::Replace { before: bound, data } if bound.as_ref() == &before && data == &input.data().unwrap())
    );
    assert_eq!(action(&worker, action_id), before);
    assert!(matches!(
        reply(
            &worker,
            (Uuid::new_v4(), AppCommand::RejectProposal(rejected.stamp()))
        )
        .1,
        AppEvent::Proposal(_)
    ));
    assert_eq!(action(&worker, action_id), before);
    assert!(state.separate_draft());
    let reviewed = create_form(&worker, &mut state);
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: reviewed.stamp(),
    };
    assert!(
        matches!(reply(&worker, (Uuid::new_v4(), AppCommand::ApproveProposal(approval.clone()))).1, AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    let after = action(&worker, action_id);
    assert_eq!(after.origin, before.origin);
    assert_eq!(after.version, before.version + 1);
    assert_eq!(after.data, input.data().unwrap());
    assert!(after.waiting_since_ms.is_none());
    worker.shutdown().unwrap();
    worker = vaultless(&fixture);
    assert_eq!(action(&worker, action_id), after);
    assert!(matches!(
        reply(
            &worker,
            (
                Uuid::new_v4(),
                AppCommand::ApproveProposal(approval.clone())
            )
        )
        .1,
        AppEvent::ProposalApplied(_)
    ));
    assert_eq!(action(&worker, action_id), after);
    let undo = UndoRequest {
        operation_id: Uuid::new_v4(),
        target_operation_id: approval.operation_id,
        trash_member: None,
    };
    assert!(
        matches!(reply(&worker, (Uuid::new_v4(), AppCommand::UndoProposal(undo))).1, AppEvent::ProposalApplied(receipt) if receipt.outcome == ApplyOutcome::Applied)
    );
    let restored = action(&worker, action_id);
    assert_eq!(restored.origin, before.origin);
    assert_eq!(restored.data, before.data);
    assert_eq!(restored.version, after.version + 1);
    // A retained original baseline never silently rebases after approval/Undo.
    assert!(state.begin_action_replace_draft(before.clone()));
    fields(&mut state, "Stale owner input retained");
    let command = state.create_draft().unwrap();
    let (id, event) = reply(&worker, command);
    assert!(matches!(&event, AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale));
    state.apply(id, event);
    assert_eq!(
        state
            .draft
            .as_ref()
            .unwrap()
            .action
            .as_ref()
            .unwrap()
            .before
            .as_deref(),
        Some(&before)
    );
    assert_eq!(
        state
            .draft
            .as_ref()
            .unwrap()
            .action
            .as_ref()
            .unwrap()
            .fields
            .values[0],
        "Stale owner input retained"
    );
    assert_eq!(action(&worker, action_id), restored);
    worker.shutdown().unwrap();
}

#[test]
fn replacement_retains_historical_references_and_new_role_needs_explicit_source_capture() {
    let fixture = Fixture::new();
    let note_id = Uuid::new_v4();
    let text = format!("---\nbrn_id: {note_id}\n---\n# Exact referenced note\r\n");
    fs::write(fixture.vault().join("person.md"), &text).unwrap();
    let mut worker = fixture.worker();
    let capture = source(&worker, "person.md");
    let mut state = ready();
    assert!(state.begin_action_draft(None));
    fields(&mut state, "Referenced original");
    let form = state.draft.as_mut().unwrap();
    let mut input = form.action.as_ref().unwrap().fields.clone();
    input.values[3] = note_id.to_string();
    form.edit_action_fields(input);
    form.edit_action_source_path("person.md".into());
    assert!(form.retain_action_source(capture.clone()));
    let created = create_form(&worker, &mut state);
    approve(&worker, &created);
    let before = action(&worker, created.draft.action_changes[0].id());
    assert!(state.begin_action_replace_draft(before.clone()));
    fields(&mut state, "Historical reference retained");
    assert!(
        state
            .draft
            .as_ref()
            .unwrap()
            .action
            .as_ref()
            .unwrap()
            .sources
            .is_empty()
    );
    let reviewed = create_form(&worker, &mut state);
    approve(&worker, &reviewed);
    let before = action(&worker, before.origin.id);
    assert_eq!(before.data.related_person, Some(note_id));
    assert!(state.begin_action_replace_draft(before.clone()));
    let form = state.draft.as_mut().unwrap();
    let mut input = form.action.as_ref().unwrap().fields.clone();
    input.values[4] = note_id.to_string();
    form.edit_action_fields(input);
    let command = state.create_draft().unwrap();
    let (id, event) = reply(&worker, command);
    assert!(matches!(&event, AppEvent::Failed(_)));
    state.apply(id, event);
    assert_eq!(action(&worker, before.origin.id), before);
    assert!(state.separate_draft());
    let form = state.draft.as_mut().unwrap();
    form.edit_action_source_path("person.md".into());
    assert!(form.retain_action_source(capture));
    let reviewed = create_form(&worker, &mut state);
    approve(&worker, &reviewed);
    assert_eq!(
        action(&worker, before.origin.id).data.related_project,
        Some(note_id)
    );
    assert_eq!(
        fs::read(fixture.vault().join("person.md")).unwrap(),
        text.as_bytes()
    );
    worker.shutdown().unwrap();
}

#[test]
fn owner_replacement_stale_approval_cannot_reopen_completed_work_or_erase_input() {
    let fixture = Fixture::new();
    let mut worker = vaultless(&fixture);
    let mut state = ready();
    state.vault_bound = false;
    assert!(state.begin_action_draft(None));
    fields(&mut state, "Original real work");
    let created = create_form(&worker, &mut state);
    approve(&worker, &created);
    let before = action(&worker, created.draft.action_changes[0].id());
    assert!(state.begin_action_replace_draft(before.clone()));
    fields(&mut state, "Owner replacement input");
    // Direct construction also respects an unresolved retained form.
    assert!(!state.begin_action_replace_draft(before.clone()));
    assert_eq!(state.draft.as_ref().unwrap().action.as_ref().unwrap().fields.values[0], "Owner replacement input");
    let reviewed = create_form(&worker, &mut state);
    let completion = CompleteActionRequest { operation_id: Uuid::new_v4(), before: Box::new(before.clone()), sent_source: None };
    assert!(matches!(reply(&worker, (completion.operation_id, AppCommand::CompleteAction(completion))).1, AppEvent::ActionCompleted(_)));
    let completed = action(&worker, before.origin.id);
    let approval = ApprovalRequest { operation_id: Uuid::new_v4(), expected: reviewed.stamp() };
    assert!(matches!(reply(&worker, (Uuid::new_v4(), AppCommand::ApproveProposal(approval))).1, AppEvent::Failed(_)));
    assert_eq!(action(&worker, before.origin.id), completed);
    assert!(!state.begin_action_replace_draft(completed));
    assert_eq!(state.draft.as_ref().unwrap().action.as_ref().unwrap().before.as_deref(), Some(&before));
    assert_eq!(state.draft.as_ref().unwrap().action.as_ref().unwrap().fields.values[0], "Owner replacement input");
    worker.shutdown().unwrap();
}

#[test]
fn vaultless_form_requires_exact_approval_and_completed_followup_is_new_restartable_work() {
    let fixture = Fixture::new();
    let mut worker = vaultless(&fixture);
    let mut state = ready();
    state.vault_bound = false;
    assert!(state.begin_action_draft(None));
    fields(&mut state, "\u{feff}Initial λ 日本語\r\n");
    let id = state.draft.as_ref().unwrap().action.as_ref().unwrap().id;
    let review = create_form(&worker, &mut state);
    let submitted = state.draft.as_ref().unwrap().submitted.as_ref().unwrap().request.clone();
    assert!(matches!(reply(&worker, (Uuid::new_v4(), AppCommand::Action(id))).1,
        AppEvent::Failed(error) if error.kind == ErrorKind::NotFound));
    assert!(state.draft.as_ref().unwrap().can_leave());
    approve(&worker, &review);
    let original = action(&worker, id);
    assert_eq!(original.data.title.as_bytes(), "\u{feff}Initial λ 日本語\r\n".as_bytes());
    assert_eq!(original.data.description.as_bytes(), TEXT.as_bytes());
    let completion = CompleteActionRequest { operation_id: Uuid::new_v4(), before: Box::new(original), sent_source: None };
    assert!(matches!(reply(&worker, (completion.operation_id, AppCommand::CompleteAction(completion))).1,
        AppEvent::ActionCompleted(_)));
    let completed = action(&worker, id);
    assert_eq!(completed.data.state, ActionState::Completed);
    assert!(state.begin_action_draft(Some(id)));
    assert!(!state.draft.as_ref().unwrap().can_leave());
    fields(&mut state, "Separate follow-up õ\r\n");
    let next = state.draft.as_ref().unwrap().action.as_ref().unwrap().id;
    assert_ne!(next, id);
    let followup = create_form(&worker, &mut state);
    assert!(matches!(reply(&worker, (Uuid::new_v4(), AppCommand::Action(next))).1,
        AppEvent::Failed(error) if error.kind == ErrorKind::NotFound));
    assert_eq!(action(&worker, id), completed);
    approve(&worker, &followup);
    let related = action(&worker, next);
    assert_eq!(related.data.state, ActionState::Open);
    assert_eq!(related.data.follows_up, Some(id));
    assert_eq!(related.origin.proposal, followup.stamp());
    assert_eq!(action(&worker, id), completed);
    worker.shutdown().unwrap();
    let mut worker = vaultless(&fixture);
    assert_eq!(action(&worker, id), completed);
    assert_eq!(action(&worker, next), related);
    assert!(matches!(reply(&worker, (Uuid::new_v4(), AppCommand::CreateProposal(submitted))).1,
        AppEvent::Proposal(record) if record.draft.id == review.draft.id));
    assert_eq!(action(&worker, id), completed);
    assert_eq!(fs::read_dir(fixture.0.path().join("credentials")).unwrap().count(), 0);
    assert!(!fixture.0.path().join("data/index.sqlite").exists());
    worker.shutdown().unwrap();
}

#[test]
fn multiple_source_proofs_reject_forged_stale_and_late_replies_and_replace_only_explicit_path() {
    let fixture = Fixture::new();
    fs::write(fixture.vault().join("a.md"), TEXT).unwrap();
    fs::write(fixture.vault().join("b.md"), "# B\r\n").unwrap();
    let mut worker = fixture.worker();
    let a = source(&worker, "a.md");
    let b = source(&worker, "b.md");
    let mut state = ready();
    assert!(state.begin_action_draft(None));
    fields(&mut state, "Captured work");
    state.draft.as_mut().unwrap().edit_action_source_path("a.md".into());
    let old = state.draft_source().unwrap();
    let current = state.draft_source().unwrap();
    assert!(!state.draft.as_ref().unwrap().can_leave());
    assert!(state.create_draft().is_none());
    state.apply(old.0, AppEvent::ProposalSource(Box::new(a.clone())));
    assert!(state.draft.as_ref().unwrap().action.as_ref().unwrap().sources.is_empty());
    let mut forged = a.clone();
    forged.text = forged.text.replace('λ', "μ"); // same UTF-8 length, different hash
    state.apply(current.0, AppEvent::ProposalSource(Box::new(forged)));
    assert!(state.pending.contains_key(&current.0));
    assert_eq!(state.draft.as_ref().unwrap().source_operation, Some(current.0));
    state.apply(current.0, AppEvent::ProposalSource(Box::new(b.clone())));
    assert!(state.pending.contains_key(&current.0));
    state.apply(current.0, AppEvent::ProposalSource(Box::new(a.clone())));
    let first_gen = state.draft.as_ref().unwrap().generation;
    state.apply(old.0, AppEvent::Failed(brn_workflow::WorkflowError::msg("late old failure")));
    assert!(state.draft.as_ref().unwrap().source_error.is_none());
    state.draft.as_mut().unwrap().edit_action_source_path("b.md".into());
    let (id, event) = reply(&worker, state.draft_source().unwrap());
    state.apply(id, event);
    assert!(state.draft.as_ref().unwrap().generation > first_gen);
    assert_eq!(state.draft.as_ref().unwrap().action.as_ref().unwrap().sources, vec![a.clone(), b.clone()]);
    fs::write(fixture.vault().join("a.md"), "# Explicit changed source λ\r\n").unwrap();
    state.draft.as_mut().unwrap().edit_action_source_path("a.md".into());
    let (id, event) = reply(&worker, state.draft_source().unwrap());
    state.apply(id, event);
    let latest = source(&worker, "a.md");
    assert_ne!(latest, a);
    assert_eq!(state.draft.as_ref().unwrap().action.as_ref().unwrap().sources, vec![latest.clone(), b.clone()]);
    // A path change invalidates an in-flight capture without discarding retained proofs.
    let late = state.draft_source().unwrap();
    state.draft.as_mut().unwrap().edit_action_source_path("b.md".into());
    state.apply(late.0, AppEvent::ProposalSource(Box::new(latest.clone())));
    assert_eq!(state.draft.as_ref().unwrap().action.as_ref().unwrap().sources, vec![latest, b]);
    assert!(state.separate_draft());
    assert!(state.draft.as_ref().unwrap().source_operation.is_none());
    assert_eq!(state.draft.as_ref().unwrap().action.as_ref().unwrap().sources.len(), 2);
    worker.shutdown().unwrap();
}

#[test]
fn changed_source_refusal_keeps_whole_form_and_creation_ack_binds_actions_without_overwriting_typing() {
    let fixture = Fixture::new();
    fs::write(fixture.vault().join("a.md"), TEXT).unwrap();
    let mut worker = fixture.worker();
    let mut state = ready();
    assert!(state.begin_action_draft(None));
    fields(&mut state, "Initial exact title");
    state.draft.as_mut().unwrap().edit_action_source_path("a.md".into());
    let (id, event) = reply(&worker, state.draft_source().unwrap());
    state.apply(id, event);
    let captured = state.draft.as_ref().unwrap().action.as_ref().unwrap().sources.clone();
    let action_id = state.draft.as_ref().unwrap().action.as_ref().unwrap().id;
    fs::write(fixture.vault().join("a.md"), "# Changed\n").unwrap();
    let command = state.create_draft().unwrap();
    let (id, event) = reply(&worker, command);
    assert!(matches!(&event, AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale));
    let followups = state.apply(id, event);
    drain_reads(&worker, &mut state, followups);
    let form = state.draft.as_ref().unwrap();
    assert_eq!(form.action.as_ref().unwrap().sources, captured);
    assert_eq!(form.action.as_ref().unwrap().fields.values[0], "Initial exact title");
    assert!(form.error.is_some());
    assert!(matches!(reply(&worker, (Uuid::new_v4(), AppCommand::Action(action_id))).1,
        AppEvent::Failed(error) if error.kind == ErrorKind::NotFound));
    // New exact capture changes the payload, so retain it in a separate proposal UUID.
    assert!(state.separate_draft());
    let (id, event) = reply(&worker, state.draft_source().unwrap());
    state.apply(id, event);
    let command = state.create_draft().unwrap();
    let (outer, AppEvent::Proposal(created)) = reply(&worker, command) else { panic!("review") };
    let mut wrong = created.clone();
    wrong.draft.action_changes.clear();
    state.apply(outer, AppEvent::Proposal(wrong));
    assert!(state.pending.contains_key(&outer));
    let mut changed = created.draft.clone();
    changed.title = "Later reviewed proposal".into();
    changed.action_changes[0].data_mut().description = "Later reviewed Action data".into();
    let (_, AppEvent::Proposal(reviewed)) = reply(&worker, (Uuid::new_v4(), AppCommand::EditProposal(ProposalEdit {
        expected: created.stamp(), title: changed.title, texts: vec![],
        action_data: changed.action_changes.iter().map(|change| change.data().clone()).collect(),
    }))) else { panic!("edited review") };
    fields(&mut state, "Later local typing λ\r\n");
    let followups = state.apply(outer, AppEvent::Proposal(reviewed.clone()));
    drain_reads(&worker, &mut state, followups);
    let form = state.draft.as_ref().unwrap();
    assert_eq!(form.action.as_ref().unwrap().fields.values[0], "Later local typing λ\r\n");
    assert_eq!(form.result.as_ref().unwrap().1, reviewed);
    assert!(!form.can_leave());
    state.apply(outer, AppEvent::Failed(brn_workflow::WorkflowError::msg("late")));
    assert!(state.draft.as_ref().unwrap().error.is_none());
    worker.shutdown().unwrap();
}
