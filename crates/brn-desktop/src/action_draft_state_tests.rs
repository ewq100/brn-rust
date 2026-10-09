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
