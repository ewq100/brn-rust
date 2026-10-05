//! Native completion recovery is required before direct completion can mutate work.
#![cfg(not(target_os = "macos"))]
use brn_store::{
    WorkStore,
    work::{proposal_apply::ApplyOutcome, proposals::ProposalDraft},
};
use brn_workflow::{
    ErrorKind,
    action_completion::CompleteActionRequest,
    actions::{ActionData, ActionState},
    app::{App, AppConfig},
    proposal_apply::ApprovalRequest,
    proposals::ActionChange,
};
use std::fs;
use uuid::Uuid;

#[test]
fn unsupported_direct_completion_preserves_checked_action_and_has_no_recovery_effects() {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    fs::create_dir(&data).unwrap();
    let credentials = owner.path().join("credentials");
    // Seed only through qualified public Store approval APIs, before App owns work.
    let (mut store, _) = WorkStore::open(&data).unwrap();
    let id = Uuid::new_v4();
    let review = store
        .create_proposal(&ProposalDraft {
            inbox_source: None,
            id: Uuid::new_v4(),
            group_id: None,
            session_id: None,
            vault: None,
            title: "Synthetic approved Waiting Action".into(),
            changes: vec![],
            sources: vec![],
            action_changes: vec![ActionChange::Create {
                id,
                data: ActionData {
                    title: "Exact λ\r\n".into(),
                    description: "Retain completion baseline 日本語".into(),
                    state: ActionState::Waiting,
                    owner: None,
                    related_person: None,
                    related_project: None,
                    sources: vec![],
                    thread: None,
                    due_on: None,
                    follow_up_on: None,
                    dependencies: vec![],
                    parent: None,
                    follows_up: None,
                    priority: None,
                },
            }],
        })
        .unwrap();
    let approval = Uuid::new_v4();
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: approval,
            expected: review.stamp(),
        })
        .unwrap();
    store.record_proposal_prepared(approval, &[]).unwrap();
    store
        .finish_proposal_apply(approval, ApplyOutcome::Applied, Some(&[]))
        .unwrap();
    let before = store.action(id).unwrap().unwrap();
    let proposal = store.proposal(review.draft.id).unwrap().unwrap();
    let journal = store.proposal_apply(approval).unwrap().unwrap();
    drop(store);
    let mut app = App::open(
        &data,
        AppConfig {
            vault_root: None,
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
        },
    )
    .unwrap();
    let request = CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(before.clone()),
    };
    request.validate().unwrap();
    let names = || {
        let mut names: Vec<_> = fs::read_dir(&data)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        names.sort();
        names
    };
    let original_names = names();
    for _ in 0..2 {
        assert_eq!(
            app.complete_action(&request).unwrap_err().kind,
            ErrorKind::ToolRejected
        );
        assert_eq!(app.work_store().action(id).unwrap(), Some(before.clone()));
        assert_eq!(
            app.work_store().action_completion_for(&request).unwrap(),
            None
        );
        assert_eq!(
            app.work_store().proposal(review.draft.id).unwrap(),
            Some(proposal.clone())
        );
        assert_eq!(
            app.work_store().proposal_apply(approval).unwrap(),
            Some(journal.clone())
        );
        assert_eq!(names(), original_names);
    }
    assert!(
        original_names
            .iter()
            .all(|name| !name.to_string_lossy().starts_with(".brn-complete-"))
    );
    assert!(app.vault_root().is_none());
    assert!(!data.join("index.sqlite").exists());
    assert_eq!(fs::read_dir(credentials).unwrap().count(), 0);
}
