//! Correlation, immutable completion and actual headless-worker witnesses.
use super::*;
use brn_workflow::{
    ErrorKind, WorkflowError,
    action_completion::{ActionCompletion, CompleteActionRequest},
    actions::{ActionRecord, ActionState},
    dashboard::{DashboardCounts, DashboardEntry, DashboardFilter, DashboardPage},
};

pub(crate) fn record(id: u128) -> ActionRecord {
    let proposal = crate::review::action_tests::fixture();
    let brn_workflow::proposals::ActionChange::Replace { before, .. } =
        &proposal.draft.action_changes[1]
    else {
        panic!("before")
    };
    let mut record = *before.clone();
    record.origin.id = Uuid::from_u128(id);
    record.validate().unwrap();
    record
}
pub(crate) fn page(record: ActionRecord) -> DashboardPage {
    DashboardPage {
        as_of: "2028-02-29".into(),
        counts: DashboardCounts {
            open: 1,
            waiting: 0,
            blocked: 0,
            completed: 0,
            overdue: 0,
            follow_up: 0,
        },
        entries: vec![DashboardEntry {
            dependencies: record
                .data
                .dependencies
                .iter()
                .map(|id| brn_workflow::dashboard::ActionDependencyObservation {
                    id: *id,
                    state: None,
                })
                .collect(),
            dependency_blocked: true,
            overdue: false,
            follow_up: false,
            action: record,
        }],
        next_before: None,
    }
}
fn loaded() -> AiState {
    let mut ai = AiState {
        ready: true,
        ..Default::default()
    };
    let query = ai.open_dashboard().unwrap();
    assert!(
        ai.apply(
            query.0,
            AppEvent::ActionDashboard(Box::new(page(record(200))))
        )
        .is_empty()
    );
    assert!(ai.select_dashboard_action(Uuid::from_u128(200)));
    ai
}
fn complete(request: &CompleteActionRequest) -> ActionCompletion {
    let mut after = *request.before.clone();
    after.data.state = ActionState::Completed;
    after.version += 1;
    after.updated_at_ms += 1;
    after.completed_at_ms = Some(after.updated_at_ms);
    after.waiting_since_ms = None;
    let completion = ActionCompletion {
        request: request.clone(),
        after,
    };
    completion.validate().unwrap();
    completion
}
fn captured(ai: &mut AiState) -> (Uuid, AppCommand, CompleteActionRequest) {
    let capture = ai.capture_action_completion().unwrap();
    let request = capture.request().clone();
    let (id, command) = ai.confirm_action_completion(&capture).unwrap();
    assert_eq!(id, request.operation_id);
    assert!(matches!(&command, AppCommand::CompleteAction(submitted) if submitted == &request));
    (id, command, request)
}

#[test]
fn changed_filter_closed_view_and_malformed_page_cannot_acknowledge_current_query() {
    let mut ai = loaded();
    let old = ai
        .refresh_dashboard(DashboardFilter::Active, false)
        .unwrap();
    let current = ai
        .refresh_dashboard(DashboardFilter::Completed, false)
        .unwrap();
    ai.apply(
        old.0,
        AppEvent::ActionDashboard(Box::new(page(record(200)))),
    );
    assert!(ai.dashboard.page.is_none());
    assert!(!ai.pending.contains_key(&old.0));
    ai.apply(
        current.0,
        AppEvent::ActionDashboard(Box::new(page(record(200)))),
    );
    assert!(ai.dashboard.page.is_none());
    assert!(ai.pending.contains_key(&current.0));
    ai.close_dashboard();
    ai.apply(
        current.0,
        AppEvent::Failed(WorkflowError::msg("old view failure")),
    );
    assert!(ai.dashboard.error.is_none());
    assert!(!ai.pending.contains_key(&current.0));
    assert!(ai.dashboard.selected.is_none());
}

#[test]
fn captured_full_before_is_immutable_and_confirmation_refuses_changed_selection_or_busy_state() {
    let mut ai = loaded();
    let capture = ai.capture_action_completion().unwrap();
    let frozen = capture.request().clone();
    let refresh = ai
        .refresh_dashboard(DashboardFilter::Active, false)
        .unwrap();
    let mut newer = record(200);
    newer.version += 1;
    newer.updated_at_ms += 1;
    newer.data.title = "Changed full record".into();
    ai.apply(refresh.0, AppEvent::ActionDashboard(Box::new(page(newer))));
    ai.select_dashboard_action(Uuid::from_u128(200));
    assert!(ai.confirm_action_completion(&capture).is_none());
    assert_eq!(capture.request(), &frozen);
    let current = ai.capture_action_completion().unwrap();
    ai.ready = false;
    assert!(ai.confirm_action_completion(&current).is_none());
    ai.ready = true;
    let admitted = ai.confirm_action_completion(&current).unwrap();
    assert!(ai.application_busy());
    assert!(ai.capture_action_completion().is_none());
    assert!(ai.confirm_action_completion(&current).is_none());
    assert!(ai.pending.contains_key(&admitted.0));
}

#[test]
fn wrong_receipt_stays_pending_and_terminal_result_cannot_regress_on_late_failure() {
    let mut ai = loaded();
    let (id, _, request) = captured(&mut ai);
    let mut wrong = complete(&request);
    wrong.request.operation_id = Uuid::new_v4();
    ai.apply(id, AppEvent::ActionCompleted(Box::new(wrong)));
    assert!(ai.pending.contains_key(&id));
    assert!(ai.dashboard.attempts[0].receipt.is_none());
    let mut wrong = complete(&request);
    wrong.after.data.description.push_str(" uncaptured");
    ai.apply(id, AppEvent::ActionCompleted(Box::new(wrong)));
    assert!(ai.pending.contains_key(&id));
    let exact = complete(&request);
    let commands = ai.apply(id, AppEvent::ActionCompleted(Box::new(exact.clone())));
    assert_eq!(commands.len(), 1);
    assert!(matches!(&commands[0].1, AppCommand::ActionDashboard(_)));
    assert!(!ai.pending.contains_key(&id));
    assert_eq!(ai.dashboard.attempts[0].receipt.as_ref(), Some(&exact));
    ai.apply(
        id,
        AppEvent::Failed(WorkflowError {
            kind: ErrorKind::SaveUncertain,
            message: "late".into(),
        }),
    );
    assert!(ai.dashboard.attempts[0].error.is_none());
    assert_eq!(ai.dashboard.attempts[0].receipt.as_ref(), Some(&exact));
    assert!(ai.retry_action_completion(id).is_none());
}

#[test]
fn errors_and_exact_retry_survive_navigation_without_replacing_new_selection_or_old_attempt() {
    let mut ai = loaded();
    let (id, _, request) = captured(&mut ai);
    ai.close_dashboard();
    ai.apply(
        id,
        AppEvent::Failed(WorkflowError {
            kind: ErrorKind::SaveUncertain,
            message: "keep exact request".into(),
        }),
    );
    assert_eq!(ai.dashboard.attempts[0].request, request);
    assert_eq!(
        ai.dashboard.attempts[0].error.as_ref().unwrap().kind,
        ErrorKind::SaveUncertain
    );
    let query = ai.open_dashboard().unwrap();
    ai.apply(
        query.0,
        AppEvent::ActionDashboard(Box::new(page(record(201)))),
    );
    ai.select_dashboard_action(Uuid::from_u128(201));
    let second = ai.capture_action_completion().unwrap();
    let (second_id, _) = ai.confirm_action_completion(&second).unwrap();
    assert_eq!(ai.dashboard.attempts.len(), 2);
    assert!(ai.retry_action_completion(id).is_none());
    ai.apply(
        second_id,
        AppEvent::Failed(WorkflowError {
            kind: ErrorKind::ContextStale,
            message: "second refused".into(),
        }),
    );
    let (retry_id, command) = ai.retry_action_completion(id).unwrap();
    assert_eq!(retry_id, id);
    assert!(matches!(command, AppCommand::CompleteAction(ref exact) if exact == &request));
    ai.apply(
        retry_id,
        AppEvent::ActionCompleted(Box::new(complete(&request))),
    );
    // Refresh clears the unrelated selection instead of installing the old Action.
    assert!(ai.dashboard.selected.is_none());
    assert_eq!(ai.dashboard.attempts[1].request, *second.request());
    assert_eq!(
        ai.dashboard.attempts[1].error.as_ref().unwrap().kind,
        ErrorKind::ContextStale
    );
    ai.close_dashboard();
    assert_eq!(ai.retry_action_completion(second_id).unwrap().0, second_id);
}

#[test]
fn pagination_carries_returned_date_and_cursor_and_refuses_substituted_continuation_date() {
    let mut ai = loaded();
    let query = ai
        .refresh_dashboard(DashboardFilter::Active, false)
        .unwrap();
    let mut initial = page(record(200));
    initial.entries.clear();
    for id in (200..225).rev() {
        let mut entry = page(record(id)).entries.remove(0);
        entry.action.origin.created_at_ms = 7;
        initial.entries.push(entry);
    }
    initial.counts.open = 26;
    initial.next_before = Some(brn_workflow::actions::ActionCursor {
        created_at_ms: 7,
        id: Uuid::from_u128(200),
    });
    ai.apply(query.0, AppEvent::ActionDashboard(Box::new(initial)));
    let older = ai.refresh_dashboard(DashboardFilter::Active, true).unwrap();
    let AppCommand::ActionDashboard(request) = &older.1 else {
        panic!("query")
    };
    assert_eq!(request.as_of.as_deref(), Some("2028-02-29"));
    assert_eq!(request.before.unwrap().id, Uuid::from_u128(200));
    let mut older_page = page(record(199));
    older_page.counts.open = 26;
    older_page.as_of = "2028-03-01".into();
    ai.apply(
        older.0,
        AppEvent::ActionDashboard(Box::new(older_page.clone())),
    );
    assert!(ai.pending.contains_key(&older.0));
    older_page.as_of = "2028-02-29".into();
    ai.apply(older.0, AppEvent::ActionDashboard(Box::new(older_page)));
    assert!(!ai.pending.contains_key(&older.0));
    assert_eq!(
        ai.dashboard.page.as_ref().unwrap().entries[0]
            .action
            .origin
            .id,
        Uuid::from_u128(199)
    );
}

#[test]
fn real_worker_approved_action_completion_dashboard_refresh_and_restart_need_no_vault_or_account() {
    use brn_workflow::{
        app::AppConfig,
        app_worker::AppWorker,
        proposal_apply::ApprovalRequest,
        proposals::{ActionChange, DraftRequest},
    };
    use std::{fs, time::Duration};
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let credentials = root.path().join("credentials");
    fs::create_dir(&data).unwrap();
    let start = || {
        let worker = AppWorker::start(
            data.clone(),
            AppConfig {
                vault_root: None,
                credentials_dir: Some(credentials.clone()),
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
    };
    let reply = |worker: &AppWorker, command: (Uuid, AppCommand)| {
        worker.submit(command.0, command.1).unwrap();
        let event = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(event.0, command.0);
        event
    };
    let mut worker = start();
    let id = Uuid::new_v4();
    let mut action_data = record(200).data;
    action_data.state = ActionState::Waiting;
    action_data.related_person = None;
    action_data.related_project = None;
    action_data.sources.clear();
    action_data.thread = None;
    action_data.dependencies.clear();
    action_data.parent = None;
    action_data.follows_up = None;
    action_data.due_on = Some("0001-01-01".into());
    action_data.follow_up_on = Some("0001-01-01".into());
    let create = AppCommand::CreateProposal(DraftRequest {
        inbox_visual: None,
        inbox_knowledge: None,
        inbox_source: None,
        id: Uuid::new_v4(),
        group_id: None,
        session_id: None,
        title: "Explicit synthetic Action review".into(),
        changes: vec![],
        sources: vec![],
        action_changes: vec![ActionChange::Create {
            id,
            data: action_data.clone(),
        }],
    });
    let (_, AppEvent::Proposal(review)) = reply(&worker, (Uuid::new_v4(), create)) else {
        panic!("review")
    };
    assert!(
        matches!(reply(&worker, (Uuid::new_v4(), AppCommand::Action(id))).1, AppEvent::Failed(error) if error.kind == ErrorKind::NotFound)
    );
    assert!(matches!(
        reply(
            &worker,
            (
                Uuid::new_v4(),
                AppCommand::ApproveProposal(ApprovalRequest {
                    operation_id: Uuid::new_v4(),
                    expected: review.stamp(),
                })
            )
        )
        .1,
        AppEvent::ProposalApplied(_)
    ));
    let mut ai = AiState {
        ready: true,
        ..Default::default()
    };
    let (query, event) = reply(&worker, ai.open_dashboard().unwrap());
    ai.apply(query, event);
    assert_eq!(ai.dashboard.page.as_ref().unwrap().counts.waiting, 1);
    assert_eq!(ai.dashboard.page.as_ref().unwrap().counts.overdue, 1);
    assert!(ai.select_dashboard_action(id));
    let capture = ai.capture_action_completion().unwrap();
    assert_eq!(capture.request().before.data, action_data);
    let command = ai.confirm_action_completion(&capture).unwrap();
    worker.submit(command.0, command.1).unwrap();
    // The user-admitted mutation drains even if native shutdown starts first.
    worker.shutdown().unwrap();
    let (operation, event) = worker
        .try_event()
        .expect("drained completion acknowledgement");
    assert_eq!(operation, capture.request().operation_id);
    let refresh = ai.apply(operation, event);
    assert!(ai.dashboard.attempts[0].receipt.is_some());
    assert_eq!(refresh.len(), 1);
    let retained = ai.dashboard.attempts[0].receipt.clone().unwrap();
    let mut worker = start();
    let (query, event) = reply(&worker, refresh.into_iter().next().unwrap());
    ai.apply(query, event);
    let counts = &ai.dashboard.page.as_ref().unwrap().counts;
    assert_eq!(
        (
            counts.waiting,
            counts.completed,
            counts.overdue,
            counts.follow_up
        ),
        (0, 1, 0, 0)
    );
    let (query, event) = reply(
        &worker,
        ai.refresh_dashboard(DashboardFilter::Completed, false)
            .unwrap(),
    );
    ai.apply(query, event);
    assert!(ai.select_dashboard_action(id));
    assert_eq!(
        ai.dashboard.selected.as_ref().unwrap().action,
        retained.after
    );
    assert!(ai.capture_action_completion().is_none());
    let replay = reply(
        &worker,
        (
            retained.request.operation_id,
            AppCommand::CompleteAction(retained.request.clone()),
        ),
    )
    .1;
    assert!(matches!(replay, AppEvent::ActionCompleted(receipt) if *receipt == retained));
    worker.shutdown().unwrap();
    assert_eq!(fs::read_dir(credentials).unwrap().count(), 0);
    assert!(!data.join("index.sqlite").exists());
}
