use super::*;
use brn_workflow::{
    app::AppConfig, app_worker::AppWorker, findings::*, knowledge::IdentityOutcome,
};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    time::Duration,
};

const NOTE: &str = "11111111-1111-4111-8111-111111111111";
const TEXT: &str = "\u{feff}---\r\nbrn_id: 11111111-1111-4111-8111-111111111111\r\n---\r\n# 日本語 λ\r\n[õ][source]\r\n\r\n[source]: missing.md\r\n";
struct Fixture(tempfile::TempDir);
impl Fixture {
    fn new() -> Self {
        let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        fs::set_permissions(owner.path(), fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            fs::metadata(owner.path()).unwrap().permissions().mode() & 0o777,
            0o700
        );
        fs::create_dir(owner.path().join("data")).unwrap();
        fs::create_dir(owner.path().join("vault")).unwrap();
        fs::create_dir(owner.path().join("vault/archive")).unwrap();
        fs::write(owner.path().join("vault/current.md"), TEXT).unwrap();
        fs::write(owner.path().join("vault/archive/duplicate.md"), TEXT).unwrap();
        Self(owner)
    }
    fn worker(&self) -> AppWorker {
        let worker = AppWorker::start(
            self.0.path().join("data"),
            AppConfig {
                vault_root: Some(self.0.path().join("vault")),
                credentials_dir: Some(self.0.path().join("credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        assert!(matches!(
            worker
                .recv_event_timeout(Duration::from_secs(10))
                .unwrap()
                .1,
            AppEvent::Ready { .. }
        ));
        worker
    }
    fn unchanged(&self, worker: &AppWorker) {
        assert_eq!(
            fs::read(self.0.path().join("vault/current.md")).unwrap(),
            TEXT.as_bytes()
        );
        assert_eq!(
            fs::read(self.0.path().join("vault/archive/duplicate.md")).unwrap(),
            TEXT.as_bytes()
        );
        let (_, AppEvent::Proposals(records)) =
            reply(worker, (Uuid::new_v4(), AppCommand::Proposals(None)))
        else {
            panic!("proposal list")
        };
        assert!(records.is_empty());
        assert_eq!(
            fs::read_dir(self.0.path().join("credentials"))
                .unwrap()
                .count(),
            0
        );
    }
}
fn reply(worker: &AppWorker, command: (Uuid, AppCommand)) -> (Uuid, AppEvent) {
    worker.submit(command.0, command.1).unwrap();
    loop {
        let event = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if event.0 == command.0 {
            return event;
        }
    }
}
fn ready(worker: &AppWorker) -> AiState {
    let (_, AppEvent::NoteLinks(links)) = reply(
        worker,
        (Uuid::new_v4(), AppCommand::NoteLinks("current.md".into())),
    ) else {
        panic!("links")
    };
    assert_eq!(links.source_outcome, Some(IdentityOutcome::Ambiguous));
    AiState {
        ready: true,
        vault_bound: true,
        links: Some(*links),
        evidence: Some(EvidenceDocument {
            path: "current.md".into(),
            scope: KnowledgeScope::Current,
            note: None,
        }),
        ..Default::default()
    }
}
fn settle(worker: &AppWorker, state: &mut AiState, command: (Uuid, AppCommand)) {
    let (id, event) = reply(worker, command);
    let mut commands = std::collections::VecDeque::from(state.apply(id, event));
    while let Some(command) = commands.pop_front() {
        let (id, event) = reply(worker, command);
        commands.extend(state.apply(id, event));
    }
}
fn retained(worker: &AppWorker) -> FindingRecord {
    let (_, AppEvent::NoteLinks(links)) = reply(
        worker,
        (Uuid::new_v4(), AppCommand::NoteLinks("current.md".into())),
    ) else {
        panic!("links")
    };
    let link = &links.links[0];
    let request = CaptureFindingRequest {
        id: Uuid::new_v4(),
        origin: FindingOrigin::UnresolvedLink {
            path: "current.md".into(),
            source_sha256: links.source.sha256,
            destination: link.destination.clone(),
            start_byte: link.evidence[0].start_byte,
        },
    };
    let (_, AppEvent::Finding(record)) = reply(
        worker,
        (Uuid::new_v4(), AppCommand::CaptureFinding(request)),
    ) else {
        panic!("retained finding")
    };
    *record
}
fn open(worker: &AppWorker, state: &mut AiState) {
    let command = state.open_findings().expect("open queue read");
    settle(worker, state, command);
}
fn select(worker: &AppWorker, state: &mut AiState, id: Uuid) {
    let command = state.select_finding(id).expect("detail read");
    settle(worker, state, command);
}

#[test]
fn native_state_capture_two_origins_restart_inspect_drift_and_close_without_knowledge_changes() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = ready(&worker);
    let command = state
        .capture_link_finding(0)
        .expect("explicit saved-link capture");
    settle(&worker, &mut state, command);
    let captured = state.finding_queue.capture_result.clone().unwrap();
    assert_eq!(captured.draft.evidence.len(), 2);
    assert!(!state.finding_queue.visible);
    open(&worker, &mut state);
    assert_eq!(state.finding_queue.page.as_ref().unwrap().open_count, 1);
    let command = state
        .capture_identity_finding()
        .expect("explicit ambiguous source capture");
    settle(&worker, &mut state, command);
    assert_eq!(state.finding_queue.page.as_ref().unwrap().open_count, 2);
    assert!(
        matches!(state.finding_queue.capture_result.as_ref().unwrap().draft.request.origin,FindingOrigin::IdentityAmbiguity {note_id} if note_id.to_string()==NOTE)
    );
    select(&worker, &mut state, captured.draft.request.id);
    let command = state
        .inspect_selected_finding()
        .expect("fresh separate inspection");
    settle(&worker, &mut state, command);
    assert!(
        state
            .finding_queue
            .inspection
            .as_ref()
            .unwrap()
            .evidence
            .iter()
            .all(|proof| proof.outcome == FindingEvidenceOutcome::Unchanged)
    );
    fixture.unchanged(&worker);
    worker.shutdown().unwrap();
    drop(worker);
    worker = fixture.worker();
    let mut state = ready(&worker);
    open(&worker, &mut state);
    select(&worker, &mut state, captured.draft.request.id);
    let old_inode = fs::metadata(fixture.0.path().join("vault/current.md"))
        .unwrap()
        .ino();
    fs::write(fixture.0.path().join("vault/replacement.md"), TEXT).unwrap();
    fs::rename(
        fixture.0.path().join("vault/replacement.md"),
        fixture.0.path().join("vault/current.md"),
    )
    .unwrap();
    assert_ne!(
        old_inode,
        fs::metadata(fixture.0.path().join("vault/current.md"))
            .unwrap()
            .ino()
    );
    let command = state.inspect_selected_finding().unwrap();
    settle(&worker, &mut state, command);
    let inspected = state.finding_queue.inspection.as_ref().unwrap();
    assert_eq!(inspected.record, captured);
    assert!(
        inspected
            .evidence
            .iter()
            .all(|proof| proof.outcome == FindingEvidenceOutcome::Changed)
    );
    let command = state
        .close_selected_finding(FindingState::Resolved)
        .expect("exact direct queue closure");
    settle(&worker, &mut state, command);
    assert_eq!(
        state.finding_queue.selected.as_ref().unwrap().state,
        FindingState::Resolved
    );
    assert_eq!(
        state.finding_queue.close_result.as_ref().unwrap().version,
        2
    );
    assert_eq!(state.finding_queue.page.as_ref().unwrap().open_count, 1);
    assert!(
        state
            .close_selected_finding(FindingState::Dismissed)
            .is_none()
    );
    fixture.unchanged(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn queue_and_detail_wrong_stale_closed_filter_replies_cannot_change_current_data_or_errors() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let record = retained(&worker);
    let mut state = ready(&worker);
    let page = state.open_findings().expect("page request");
    let page_id = page.0;
    let (_, AppEvent::Findings(event)) = reply(&worker, page) else {
        panic!("page")
    };
    state.apply(Uuid::new_v4(), AppEvent::Findings(event.clone()));
    assert!(state.finding_queue.page.is_none());
    let filtered = state
        .refresh_findings(Some(FindingState::Dismissed), None)
        .unwrap();
    settle(&worker, &mut state, filtered);
    state.finding_queue.error = Some("Current filter notice".into());
    state.apply(page_id, AppEvent::Findings(event));
    assert!(
        state
            .finding_queue
            .page
            .as_ref()
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(
        state.finding_queue.error.as_deref(),
        Some("Current filter notice")
    );
    let current = state.refresh_findings(None, None).unwrap();
    let current_id = current.0;
    let (_, event) = reply(&worker, current);
    state.finding_queue.error = Some("Current closed-panel notice".into());
    state.close_findings();
    state.apply(current_id, event);
    assert!(!state.finding_queue.visible);
    assert_eq!(
        state.finding_queue.error.as_deref(),
        Some("Current closed-panel notice")
    );
    open(&worker, &mut state);
    let selected = state.select_finding(record.draft.request.id).unwrap();
    let selected_id = selected.0;
    let (_, AppEvent::Finding(reply_record)) = reply(&worker, selected) else {
        panic!("detail")
    };
    let new = state.select_finding(record.draft.request.id).unwrap();
    settle(&worker, &mut state, new);
    state.finding_queue.error = Some("New selection notice".into());
    state.apply(selected_id, AppEvent::Finding(reply_record));
    state.apply(
        selected_id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("old failure")),
    );
    assert_eq!(state.finding_queue.selected.as_ref().unwrap(), &record);
    assert_eq!(
        state.finding_queue.error.as_deref(),
        Some("New selection notice")
    );
    worker.shutdown().unwrap();
}

#[test]
fn capture_exact_retry_pending_guard_and_navigation_retain_global_acknowledged_outcome() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = ready(&worker);
    state.ready = false;
    assert!(state.capture_link_finding(0).is_none());
    assert!(state.capture_identity_finding().is_none());
    state.ready = true;
    state.evidence.as_mut().unwrap().path = "another.md".into();
    assert!(state.capture_link_finding(0).is_none());
    assert!(state.capture_identity_finding().is_none());
    state.evidence.as_mut().unwrap().path = "current.md".into();
    let saved_outcome = state.links.as_ref().unwrap().links[0].outcome;
    state.links.as_mut().unwrap().links[0].outcome =
        brn_workflow::knowledge::NoteLinkOutcome::Resolved;
    assert!(state.capture_link_finding(0).is_none());
    state.links.as_mut().unwrap().links[0].outcome = saved_outcome;
    let first = state.capture_link_finding(0).expect("capture request");
    let first_id = first.0;
    let AppCommand::CaptureFinding(request) = first.1 else {
        panic!("capture command")
    };
    assert!(state.finding_capture_pending());
    assert!(state.capture_identity_finding().is_none());
    state.close_findings();
    state.apply(
        first_id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg(
            "synthetic refused admission",
        )),
    );
    assert_eq!(
        state.finding_queue.capture_error.as_deref(),
        Some("synthetic refused admission")
    );
    let retry = state
        .capture_link_finding(0)
        .expect("explicit same capture retry");
    assert!(matches!(&retry.1,AppCommand::CaptureFinding(replayed) if replayed==&request));
    settle(&worker, &mut state, retry);
    let result = state.finding_queue.capture_result.clone().unwrap();
    assert_eq!(result.draft.request, request);
    assert!(!state.finding_queue.visible);
    assert!(!state.finding_capture_pending());
    open(&worker, &mut state);
    state.close_findings();
    assert_eq!(state.finding_queue.capture_result.as_ref(), Some(&result));
    let (_, AppEvent::Findings(page)) = reply(
        &worker,
        (
            Uuid::new_v4(),
            AppCommand::Findings(FindingListRequest::default()),
        ),
    ) else {
        panic!("queue")
    };
    assert_eq!(page.entries.len(), 1);
    fixture.unchanged(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn close_full_binding_guard_errors_retry_and_stale_navigation_preserve_selected_and_receipt() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let record = retained(&worker);
    let mut state = ready(&worker);
    open(&worker, &mut state);
    select(&worker, &mut state, record.draft.request.id);
    let first = state
        .close_selected_finding(FindingState::Resolved)
        .expect("closure");
    let id = first.0;
    assert!(state.finding_close_pending());
    assert!(
        state
            .close_selected_finding(FindingState::Dismissed)
            .is_none()
    );
    let mut wrong = record.clone();
    wrong.version = 2;
    wrong.state = FindingState::Resolved;
    wrong.draft.summary.push_str("changed immutable proof");
    state.apply(id, AppEvent::Finding(Box::new(wrong)));
    assert!(state.finding_close_pending());
    state.apply(
        id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg(
            "synthetic closure failure",
        )),
    );
    assert_eq!(state.finding_queue.selected.as_ref(), Some(&record));
    assert_eq!(
        state.finding_queue.close_error.as_deref(),
        Some("synthetic closure failure")
    );
    let retry = state
        .close_selected_finding(FindingState::Resolved)
        .unwrap();
    assert!(
        matches!((&first.1,&retry.1),(AppCommand::CloseFinding(a),AppCommand::CloseFinding(b)) if a==b)
    );
    let retry_id = retry.0;
    let (_, AppEvent::Finding(closed)) = reply(&worker, retry) else {
        panic!("closure receipt")
    };
    state.close_findings();
    let commands = state.apply(retry_id, AppEvent::Finding(closed.clone()));
    assert!(commands.is_empty());
    assert_eq!(
        state.finding_queue.close_result.as_ref(),
        Some(closed.as_ref())
    );
    assert_eq!(state.finding_queue.selected.as_ref(), Some(&record));
    open(&worker, &mut state);
    let all = state.refresh_findings(None, None).unwrap();
    settle(&worker, &mut state, all);
    select(&worker, &mut state, record.draft.request.id);
    assert_eq!(state.finding_queue.selected.as_ref(), Some(closed.as_ref()));
    assert!(!state.finding_close_pending());
    fixture.unchanged(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn admitted_capture_and_closure_drain_on_shutdown_and_closed_replay_needs_no_current_vault() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = ready(&worker);
    let (id, command) = state.capture_link_finding(0).expect("admitted capture");
    let AppCommand::CaptureFinding(request) = &command else {
        panic!("capture")
    };
    let request = request.clone();
    worker.submit(id, command).unwrap();
    worker.shutdown().unwrap();
    loop {
        let (event_id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if event_id == id {
            state.apply(id, event);
            break;
        }
    }
    let record = state.finding_queue.capture_result.clone().unwrap();
    drop(worker);
    worker = fixture.worker();
    open(&worker, &mut state);
    select(&worker, &mut state, record.draft.request.id);
    let (id, command) = state
        .close_selected_finding(FindingState::Dismissed)
        .unwrap();
    worker.submit(id, command).unwrap();
    state.close_findings();
    worker.shutdown().unwrap();
    loop {
        let (event_id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if event_id == id {
            state.apply(id, event);
            break;
        }
    }
    let terminal = state.finding_queue.close_result.clone().unwrap();
    drop(worker);
    fs::rename(
        fixture.0.path().join("vault"),
        fixture.0.path().join("retained-vault"),
    )
    .unwrap();
    worker = fixture.worker();
    state.vault_bound = false;
    open(&worker, &mut state);
    let all = state.refresh_findings(None, None).unwrap();
    settle(&worker, &mut state, all);
    select(&worker, &mut state, record.draft.request.id);
    assert_eq!(state.finding_queue.selected.as_ref(), Some(&terminal));
    let command = state.inspect_selected_finding().unwrap();
    settle(&worker, &mut state, command);
    assert!(
        state
            .finding_queue
            .inspection
            .as_ref()
            .unwrap()
            .evidence
            .iter()
            .all(|proof| proof.outcome == FindingEvidenceOutcome::Unavailable)
    );
    let (_, AppEvent::Finding(replayed)) = reply(
        &worker,
        (Uuid::new_v4(), AppCommand::CaptureFinding(request)),
    ) else {
        panic!("closed replay")
    };
    assert_eq!(*replayed, terminal);
    assert!(state.capture_link_finding(0).is_none());
    worker.shutdown().unwrap();
}

#[test]
fn exact_lost_outcome_retries_without_saved_navigation_or_vault_retain_historical_receipts() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = ready(&worker);
    assert!(state.finding_capture_retry_request().is_none());
    assert!(state.retry_finding_capture().is_none());
    assert!(state.finding_close_retry_request().is_none());
    let capture = state.capture_link_finding(0).unwrap();
    let capture_id = capture.0;
    let AppCommand::CaptureFinding(request) = &capture.1 else {
        panic!("capture")
    };
    let request = request.clone();
    let (_, AppEvent::Finding(captured)) = reply(&worker, capture) else {
        panic!("capture receipt")
    };
    // Admission succeeded, but the response was lost before state acknowledgment.
    state.apply(
        capture_id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("lost capture receipt")),
    );
    fixture.unchanged(&worker);
    state.close_findings();
    state.links = None;
    state.evidence = None;
    state.vault_bound = false;
    worker.shutdown().unwrap();
    drop(worker);
    fs::rename(
        fixture.0.path().join("vault"),
        fixture.0.path().join("retained-vault"),
    )
    .unwrap();
    worker = fixture.worker();
    state.ready = false;
    assert!(state.finding_capture_retry_request().is_none());
    assert!(state.retry_finding_capture().is_none());
    state.ready = true;
    assert_eq!(state.finding_capture_retry_request(), Some(&request));
    let retry = state
        .retry_finding_capture()
        .expect("explicit historical capture retry");
    assert!(matches!(&retry.1, AppCommand::CaptureFinding(value) if value==&request));
    assert!(state.retry_finding_capture().is_none());
    assert!(state.finding_capture_retry_request().is_none());
    settle(&worker, &mut state, retry);
    assert_eq!(
        state.finding_queue.capture_result.as_ref(),
        Some(captured.as_ref())
    );
    assert!(state.finding_queue.capture_error.is_none());
    assert!(!state.finding_queue.visible);
    open(&worker, &mut state);
    assert_eq!(state.finding_queue.page.as_ref().unwrap().entries.len(), 1);
    select(&worker, &mut state, request.id);
    // Exact operational closure also works without a currently bound vault.
    let closure = state
        .close_selected_finding(FindingState::Resolved)
        .unwrap();
    let closure_id = closure.0;
    let AppCommand::CloseFinding(close_request) = &closure.1 else {
        panic!("closure")
    };
    let close_request = close_request.clone();
    let (_, AppEvent::Finding(closed)) = reply(&worker, closure) else {
        panic!("closure receipt")
    };
    state.apply(
        closure_id,
        AppEvent::Failed(brn_workflow::WorkflowError::msg("lost closure receipt")),
    );
    state.close_findings();
    state.ready = false;
    assert!(state.finding_close_retry_request().is_none());
    assert!(state.retry_finding_close().is_none());
    state.ready = true;
    assert_eq!(state.finding_close_retry_request(), Some(&close_request));
    let retry = state
        .retry_finding_close()
        .expect("exact historical closure retry");
    assert!(matches!(&retry.1, AppCommand::CloseFinding(value) if value==&close_request));
    assert!(state.retry_finding_close().is_none());
    assert!(state.finding_close_retry_request().is_none());
    settle(&worker, &mut state, retry);
    assert_eq!(
        state.finding_queue.close_result.as_ref(),
        Some(closed.as_ref())
    );
    assert_eq!(
        state.finding_queue.selected.as_ref(),
        Some(captured.as_ref())
    );
    assert!(!state.finding_queue.visible);
    assert!(state.finding_queue.close_error.is_none());
    open(&worker, &mut state);
    let all = state.refresh_findings(None, None).unwrap();
    settle(&worker, &mut state, all);
    select(&worker, &mut state, request.id);
    assert_eq!(state.finding_queue.selected.as_ref(), Some(closed.as_ref()));
    let inspection = state.inspect_selected_finding().unwrap();
    settle(&worker, &mut state, inspection);
    assert!(
        state
            .finding_queue
            .inspection
            .as_ref()
            .unwrap()
            .evidence
            .iter()
            .all(|proof| proof.outcome == FindingEvidenceOutcome::Unavailable)
    );
    fs::rename(
        fixture.0.path().join("retained-vault"),
        fixture.0.path().join("vault"),
    )
    .unwrap();
    fixture.unchanged(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn current_payload_validation_rejects_substituted_inspection_and_terminal_regression() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let record = retained(&worker);
    let mut state = ready(&worker);
    let page = state.open_findings().unwrap();
    let page_id = page.0;
    let (_, AppEvent::Findings(valid_page)) = reply(&worker, page) else {
        panic!("page")
    };
    let mut duplicate = (*valid_page).clone();
    duplicate.entries.push(record.clone());
    state.apply(page_id, AppEvent::Findings(Box::new(duplicate)));
    assert!(state.finding_queue.page.is_none());
    assert!(state.findings_loading());
    state.apply(page_id, AppEvent::Findings(valid_page));
    select(&worker, &mut state, record.draft.request.id);
    let inspection = state.inspect_selected_finding().unwrap();
    let inspection_id = inspection.0;
    let (_, AppEvent::FindingInspection(valid)) = reply(&worker, inspection) else {
        panic!("inspection")
    };
    let mut wrong = (*valid).clone();
    wrong.evidence[0].index = 1;
    state.apply(inspection_id, AppEvent::FindingInspection(Box::new(wrong)));
    assert!(state.finding_queue.inspection.is_none());
    assert!(state.finding_loading());
    let mut wrong = (*valid).clone();
    wrong.evidence[0].outcome = FindingEvidenceOutcome::Changed;
    wrong.evidence[0].observed.as_mut().unwrap().path = "archive/duplicate.md".into();
    state.apply(inspection_id, AppEvent::FindingInspection(Box::new(wrong)));
    assert!(state.finding_queue.inspection.is_none());
    state.apply(inspection_id, AppEvent::FindingInspection(valid));
    assert!(!state.finding_loading());
    let stale = state.inspect_selected_finding().unwrap();
    let stale_id = stale.0;
    let (_, stale_reply) = reply(&worker, stale);
    let closure = state
        .close_selected_finding(FindingState::Resolved)
        .unwrap();
    settle(&worker, &mut state, closure);
    let terminal = state.finding_queue.selected.clone().unwrap();
    state.finding_queue.error = Some("Latest terminal notice".into());
    state.apply(stale_id, stale_reply);
    assert_eq!(state.finding_queue.selected.as_ref(), Some(&terminal));
    assert!(state.finding_queue.inspection.is_none());
    assert_eq!(
        state.finding_queue.error.as_deref(),
        Some("Latest terminal notice")
    );
    let detail = state.select_finding(record.draft.request.id).unwrap();
    let detail_id = detail.0;
    let (_, valid_detail) = reply(&worker, detail);
    state.apply(detail_id, AppEvent::Finding(Box::new(record)));
    assert_eq!(state.finding_queue.selected.as_ref(), Some(&terminal));
    assert!(state.finding_loading());
    let mut wrong = terminal.clone();
    wrong.state = FindingState::Dismissed;
    state.apply(detail_id, AppEvent::Finding(Box::new(wrong)));
    assert_eq!(state.finding_queue.selected.as_ref(), Some(&terminal));
    state.apply(detail_id, valid_detail);
    assert!(!state.finding_loading());
    fixture.unchanged(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn receipt_details_outside_filtered_page_preserve_full_immutable_capture_binding() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = ready(&worker);
    let command = state.capture_link_finding(0).unwrap();
    settle(&worker, &mut state, command);
    let receipt = state.finding_queue.capture_result.clone().unwrap();
    open(&worker, &mut state);
    let filter = state
        .refresh_findings(Some(FindingState::Dismissed), None)
        .unwrap();
    settle(&worker, &mut state, filter);
    assert!(
        state
            .finding_queue
            .page
            .as_ref()
            .unwrap()
            .entries
            .is_empty()
    );
    let command = state
        .select_finding(receipt.draft.request.id)
        .expect("receipt detail outside page");
    let id = command.0;
    let (_, valid) = reply(&worker, command);
    let mut wrong = receipt.clone();
    wrong.draft.title.push_str(" substituted immutable content");
    state.apply(id, AppEvent::Finding(Box::new(wrong)));
    assert!(state.finding_queue.selected.is_none());
    assert!(state.finding_loading());
    state.apply(id, valid);
    assert_eq!(state.finding_queue.selected.as_ref(), Some(&receipt));
    let command = state
        .close_selected_finding(FindingState::Resolved)
        .unwrap();
    settle(&worker, &mut state, command);
    let closed = state.finding_queue.close_result.clone().unwrap();
    let filter = state
        .refresh_findings(Some(FindingState::Open), None)
        .unwrap();
    settle(&worker, &mut state, filter);
    assert!(
        state
            .finding_queue
            .page
            .as_ref()
            .unwrap()
            .entries
            .is_empty()
    );
    let command = state.select_finding(receipt.draft.request.id).unwrap();
    let id = command.0;
    let (_, valid) = reply(&worker, command);
    state.apply(id, AppEvent::Finding(Box::new(receipt)));
    assert!(state.finding_queue.selected.is_none());
    state.apply(id, valid);
    assert_eq!(state.finding_queue.selected.as_ref(), Some(&closed));
    fixture.unchanged(&worker);
    worker.shutdown().unwrap();
}
