//! Shared refusal contract until ordinary recovery receipts support another OS.
#![cfg(not(target_os = "macos"))]

use brn_workflow::{
    ErrorKind,
    actions::{ActionData, ActionListRequest, ActionState},
    app::{App, AppConfig},
    proposal_apply::ApprovalRequest,
    proposals::{ActionChange, DraftRequest},
};
use std::fs;
use uuid::Uuid;

#[test]
fn source_free_review_is_retained_but_unsupported_approval_has_no_effects() {
    let parent = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data_dir = parent.path().join("data");
    fs::create_dir(&data_dir).unwrap();
    let credentials = parent.path().join("credentials");
    let mut app = App::open(
        &data_dir,
        AppConfig {
            vault_root: None,
            credentials_dir: Some(credentials.clone()),
            model_dir: None,
        },
    )
    .unwrap();
    let id = Uuid::new_v4();
    let request = DraftRequest {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Retained exact review".into(),
        changes: Vec::new(),
        sources: Vec::new(),
        action_changes: vec![ActionChange::Create {
            id,
            data: ActionData {
                title: "Exact λ\r\n".into(),
                description: "Retain source-free review; no operational effect.".into(),
                state: ActionState::Waiting,
                owner: None,
                related_person: None,
                related_project: None,
                sources: Vec::new(),
                thread: None,
                due_on: None,
                follow_up_on: None,
                dependencies: Vec::new(),
                parent: None,
                follows_up: None,
                priority: None,
            },
        }],
    };
    let record = app.create_proposal(&request).unwrap();
    assert_eq!(record.draft.action_changes, request.action_changes);
    assert!(record.draft.vault.is_none());
    assert!(
        app.actions(&ActionListRequest::default())
            .unwrap()
            .entries
            .is_empty()
    );
    let approval = ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: record.stamp(),
    };
    assert_eq!(
        app.approve_proposal(&approval).unwrap_err().kind,
        ErrorKind::ToolRejected
    );
    assert_eq!(app.proposal(request.id).unwrap(), record);
    assert_eq!(app.create_proposal(&request).unwrap(), record);
    assert!(app.work_store().proposal_apply_ids().unwrap().is_empty());
    assert!(app.work_store().action(id).unwrap().is_none());
    assert!(
        app.actions(&ActionListRequest::default())
            .unwrap()
            .entries
            .is_empty()
    );
    assert!(app.vault_root().is_none());
    assert_eq!(fs::read_dir(credentials).unwrap().count(), 0);
    assert!(!data_dir.join("index.sqlite").exists());
}
