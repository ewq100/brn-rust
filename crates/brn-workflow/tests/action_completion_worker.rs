//! Exact identified user completion through the shared headless application.
#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    action_completion::{ActionCompletion, CompleteActionRequest},
    actions::{ActionData, ActionListRequest, ActionPriority, ActionRecord, ActionState},
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    proposal_apply::ApprovalRequest,
    proposals::{ActionChange, DraftRequest},
};
use std::{fs, path::PathBuf, time::Duration};
use uuid::Uuid;

struct Fixture {
    _owner: tempfile::TempDir,
    data: PathBuf,
    credentials: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = owner.path().join("data");
        fs::create_dir(&data).unwrap();
        Self {
            data,
            credentials: owner.path().join("credentials"),
            _owner: owner,
        }
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(
            self.data.clone(),
            AppConfig {
                vault_root: None,
                credentials_dir: Some(self.credentials.clone()),
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
                AppEvent::Ready {
                    vault_bound: false,
                    model_installed: false,
                } => break,
                AppEvent::Restored { .. } => {}
                AppEvent::Failed(error) => panic!("worker startup failed: {error}"),
                _ => panic!("unexpected startup reply"),
            }
        }
        worker
    }
}
fn query(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = match &command {
        AppCommand::CompleteAction(request) => request.operation_id,
        _ => Uuid::new_v4(),
    };
    worker.submit(id, command).unwrap();
    let (reply, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(reply, id);
    event
}
fn action(worker: &AppWorker, id: Uuid) -> ActionRecord {
    match query(worker, AppCommand::Action(id)) {
        AppEvent::Action(record) => *record,
        AppEvent::Failed(error) => panic!("Action: {error}"),
        _ => panic!("Action reply"),
    }
}
fn approved(worker: &AppWorker, follows_up: Option<Uuid>) -> ActionRecord {
    let id = Uuid::new_v4();
    let input = DraftRequest {
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Approve explicit follow-up λ".into(),
        changes: vec![],
        sources: vec![],
        action_changes: vec![ActionChange::Create {
            id,
            data: ActionData {
                title: "Exact Waiting õ".into(),
                description: "\u{feff}Quoted 日本語\r\n\t\u{0001}".into(),
                state: ActionState::Waiting,
                owner: Some("Zoë".into()),
                related_person: None,
                related_project: None,
                sources: vec![],
                thread: None,
                due_on: Some("2028-02-29".into()),
                follow_up_on: Some("2028-03-01".into()),
                dependencies: vec![],
                parent: None,
                follows_up,
                priority: Some(ActionPriority::High),
            },
        }],
    };
    let AppEvent::Proposal(review) = query(worker, AppCommand::CreateProposal(input)) else {
        panic!("Action review")
    };
    assert!(
        matches!(query(worker, AppCommand::Action(id)), AppEvent::Failed(error) if error.kind == ErrorKind::NotFound)
    );
    assert!(matches!(
        query(
            worker,
            AppCommand::ApproveProposal(ApprovalRequest {
                operation_id: Uuid::new_v4(),
                expected: review.stamp(),
            })
        ),
        AppEvent::ProposalApplied(_)
    ));
    action(worker, id)
}
fn complete(worker: &AppWorker, request: &CompleteActionRequest) -> ActionCompletion {
    match query(worker, AppCommand::CompleteAction(request.clone())) {
        AppEvent::ActionCompleted(receipt) => *receipt,
        AppEvent::Failed(error) => panic!("Complete: {error}"),
        _ => panic!("Complete receipt"),
    }
}

#[test]
fn exact_direct_completion_and_new_approved_follow_up_survive_restart_without_provider() {
    let f = Fixture::new();
    let mut worker = f.worker();
    let before = approved(&worker, None);
    let request = CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(before.clone()),
    };
    let receipt = complete(&worker, &request);
    receipt.validate().unwrap();
    let mut expected = before.clone();
    expected.version += 1;
    expected.data.state = ActionState::Completed;
    expected.waiting_since_ms = None;
    expected.updated_at_ms = receipt.after.updated_at_ms;
    expected.completed_at_ms = Some(expected.updated_at_ms);
    assert_eq!(receipt.after, expected);
    assert_eq!(receipt.request, request);
    assert_eq!(action(&worker, before.origin.id), expected);
    assert_eq!(complete(&worker, &request), receipt);
    let follow_up = approved(&worker, Some(before.origin.id));
    assert_ne!(follow_up.origin.id, before.origin.id);
    assert_eq!(follow_up.data.follows_up, Some(before.origin.id));
    assert_eq!(action(&worker, before.origin.id), expected);
    assert!(
        matches!(query(&worker, AppCommand::Actions(ActionListRequest {
        state: Some(ActionState::Completed), ..Default::default()
    })), AppEvent::Actions(page) if page.entries == vec![expected.clone()])
    );
    worker.shutdown().unwrap();
    let mut worker = f.worker();
    assert_eq!(complete(&worker, &request), receipt);
    assert_eq!(action(&worker, follow_up.origin.id), follow_up);
    assert!(matches!(
        query(&worker, AppCommand::Selection),
        AppEvent::Selection(None)
    ));
    assert!(
        matches!(query(&worker, AppCommand::Conversations), AppEvent::Conversations(items) if items.is_empty())
    );
    worker.shutdown().unwrap();
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}

#[test]
fn stale_full_baseline_operation_reuse_invalid_input_and_correlation_refuse() {
    let f = Fixture::new();
    let mut worker = f.worker();
    let before = approved(&worker, None);
    let request = CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(before.clone()),
    };
    let error = worker
        .submit(Uuid::new_v4(), AppCommand::CompleteAction(request.clone()))
        .unwrap_err();
    assert_eq!(error.kind, ErrorKind::OperationConflict);
    let mut stale = request.clone();
    stale.before.version += 1;
    stale.before.data.description.push_str("unreviewed drift");
    assert!(
        matches!(query(&worker, AppCommand::CompleteAction(stale.clone())), AppEvent::Failed(error) if error.kind == ErrorKind::ContextStale)
    );
    assert_eq!(action(&worker, before.origin.id), before);
    let receipt = complete(&worker, &request);
    assert!(
        matches!(query(&worker, AppCommand::CompleteAction(stale)), AppEvent::Failed(error) if error.kind == ErrorKind::OperationConflict)
    );
    let again = CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(receipt.after.clone()),
    };
    assert!(
        matches!(query(&worker, AppCommand::CompleteAction(again)), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    let nil = CompleteActionRequest {
        operation_id: Uuid::nil(),
        before: request.before.clone(),
    };
    assert!(
        matches!(query(&worker, AppCommand::CompleteAction(nil)), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert_eq!(complete(&worker, &request), receipt);
    assert_eq!(action(&worker, before.origin.id), receipt.after);
    worker.shutdown().unwrap();
}

#[test]
fn completion_evidence_reconstructs_a_fresh_database_before_worker_ready() {
    let f = Fixture::new();
    let mut worker = f.worker();
    let before = approved(&worker, None);
    let request = CompleteActionRequest {
        operation_id: Uuid::new_v4(),
        before: Box::new(before),
    };
    let receipt = complete(&worker, &request);
    worker.shutdown().unwrap();
    fs::remove_file(f.data.join("brn.sqlite")).unwrap();
    fs::remove_dir_all(f.data.join("backups")).unwrap();
    for suffix in ["-wal", "-shm"] {
        let path = f.data.join(format!("brn.sqlite{suffix}"));
        if path.exists() {
            fs::remove_file(path).unwrap();
        }
    }
    let mut worker = f.worker();
    assert_eq!(action(&worker, receipt.after.origin.id), receipt.after);
    assert_eq!(complete(&worker, &request), receipt);
    worker.shutdown().unwrap();
    assert_eq!(fs::read_dir(&f.credentials).unwrap().count(), 0);
}
