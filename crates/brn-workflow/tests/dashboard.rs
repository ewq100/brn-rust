use brn_store::{
    WorkStore,
    work::{
        proposal_apply::{ApplyOutcome, ApprovalRequest},
        proposals::{ActionChange, ProposalDraft},
    },
};
use brn_workflow::{
    ErrorKind,
    actions::{ActionData, ActionState},
    app::{App, AppConfig},
    app_worker::{AppCommand, AppEvent, AppWorker},
    dashboard::{DashboardFilter, DashboardRequest},
};
use std::{fs, time::Duration};
use uuid::Uuid;

fn draft(state: ActionState) -> ProposalDraft {
    ProposalDraft {
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        vault: None,
        title: "Synthetic Dashboard work".into(),
        changes: vec![],
        sources: vec![],
        action_changes: vec![ActionChange::Create {
            id: Uuid::new_v4(),
            data: ActionData {
                title: "Exact Tähtaeg 🦀\r\n".into(),
                description: "Quoted 日本語\r\n".into(),
                state,
                owner: None,
                related_person: None,
                related_project: None,
                sources: vec![],
                thread: None,
                due_on: Some("2028-02-28".into()),
                follow_up_on: Some("2028-02-29".into()),
                dependencies: vec![],
                parent: None,
                follows_up: None,
                priority: None,
            },
        }],
    }
}
fn config(credentials: std::path::PathBuf) -> AppConfig {
    AppConfig {
        vault_root: None,
        credentials_dir: Some(credentials),
        model_dir: None,
    }
}
fn explicit() -> DashboardRequest {
    DashboardRequest {
        as_of: Some("2028-02-29".into()),
        filter: DashboardFilter::Active,
        limit: 1,
        before: None,
    }
}

#[test]
fn headless_worker_and_direct_application_share_exact_global_snapshot_without_vault_or_provider() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    fs::create_dir(&data).unwrap();
    let credentials = root.path().join("credentials");
    let (mut store, _) = WorkStore::open(&data).unwrap();
    for state in [
        ActionState::Open,
        ActionState::Waiting,
        ActionState::Blocked,
    ] {
        let review = store.create_proposal(&draft(state)).unwrap();
        let operation_id = Uuid::new_v4();
        store
            .begin_proposal_apply(&ApprovalRequest {
                operation_id,
                expected: review.stamp(),
            })
            .unwrap();
        store.record_proposal_prepared(operation_id, &[]).unwrap();
        store
            .finish_proposal_apply(operation_id, ApplyOutcome::Applied, Some(&[]))
            .unwrap();
    }
    drop(store);
    let app = App::open(&data, config(credentials.clone())).unwrap();
    let input = explicit();
    let expected = app.action_dashboard(&input).unwrap();
    input.validate_page(&expected).unwrap();
    assert_eq!(
        (
            expected.counts.open,
            expected.counts.waiting,
            expected.counts.blocked,
            expected.counts.overdue,
            expected.counts.follow_up
        ),
        (1, 1, 1, 3, 3)
    );
    assert_eq!(expected.entries.len(), 1);
    assert!(expected.next_before.is_some());
    drop(app);
    let mut worker = AppWorker::start(data.clone(), config(credentials.clone())).unwrap();
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
            _ => panic!("unexpected startup"),
        }
    }
    let op = Uuid::new_v4();
    worker
        .submit(op, AppCommand::ActionDashboard(input.clone()))
        .unwrap();
    let (id, reply) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(id, op);
    let AppEvent::ActionDashboard(page) = reply else {
        panic!("dashboard reply")
    };
    assert_eq!(*page, expected);
    let mut later = input.clone();
    later.before = page.next_before;
    worker
        .submit(op, AppCommand::ActionDashboard(later.clone()))
        .unwrap();
    let (_, AppEvent::ActionDashboard(next)) =
        worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
    else {
        panic!("next page")
    };
    later.validate_page(&next).unwrap();
    assert_eq!(next.counts, page.counts);
    assert_ne!(
        next.entries[0].action.origin.id,
        page.entries[0].action.origin.id
    );
    worker.shutdown().unwrap();
    let app = App::open(&data, config(credentials.clone())).unwrap();
    assert_eq!(app.action_dashboard(&input).unwrap(), expected);
    assert!(app.selection().unwrap().is_none());
    assert!(app.work_store().conversations().unwrap().is_empty());
    assert_eq!(fs::read_dir(credentials).unwrap().count(), 0);
    assert!(!data.join("index.sqlite").exists());
}

#[test]
fn local_default_date_is_resolved_once_and_continuation_requires_explicit_returned_date() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    fs::create_dir(&data).unwrap();
    let app = App::open(&data, config(root.path().join("credentials"))).unwrap();
    let before = chrono::Local::now().date_naive().to_string();
    let input = DashboardRequest::default();
    assert_eq!(input.filter, DashboardFilter::Active);
    let page = app.action_dashboard(&input).unwrap();
    let after = chrono::Local::now().date_naive().to_string();
    assert!(page.as_of == before || page.as_of == after);
    input.validate_page(&page).unwrap();
    let mut later = input;
    later.before = Some(brn_workflow::actions::ActionCursor {
        created_at_ms: 1,
        id: Uuid::new_v4(),
    });
    assert_eq!(later.validate().unwrap_err().kind, ErrorKind::ToolRejected);
    later.as_of = Some(page.as_of);
    assert!(later.validate().is_ok());
    for day in ["2028-2-29", "2027-02-29", "0000-01-01"] {
        later.as_of = Some(day.into());
        assert_eq!(later.validate().unwrap_err().kind, ErrorKind::ToolRejected);
    }
}

#[test]
fn unsettled_authoritative_application_fences_all_dashboard_counts_and_entries() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    fs::create_dir(&data).unwrap();
    let (mut store, _) = WorkStore::open(&data).unwrap();
    let review = store.create_proposal(&draft(ActionState::Open)).unwrap();
    store
        .begin_proposal_apply(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        })
        .unwrap();
    drop(store);
    let app = App::open(&data, config(root.path().join("credentials"))).unwrap();
    assert_eq!(
        app.action_dashboard(&explicit()).unwrap_err().kind,
        ErrorKind::SaveUncertain
    );
}
