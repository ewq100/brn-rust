//! Offline journeys through real worker responses, including restart and approval.
use super::*;
use brn_workflow::inbox::CaptureBinaryInboxRequest;

fn drain(worker: &AppWorker, state: &mut AiState) {
    let start = std::time::Instant::now();
    while !state.pending.is_empty() {
        assert!(
            start.elapsed() < Duration::from_secs(20),
            "guided journey stalled"
        );
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        for (id, command) in state.apply(id, event) {
            assert!(
                !matches!(
                    command,
                    AppCommand::AnalyzeInboxActions(_)
                        | AppCommand::ApproveProposal(_)
                        | AppCommand::ApproveProposalGroup(_)
                ),
                "reading must not investigate or approve"
            );
            worker.submit(id, command).unwrap();
        }
    }
}
fn send(worker: &AppWorker, state: &mut AiState, command: (Uuid, AppCommand)) {
    worker.submit(command.0, command.1).unwrap();
    drain(worker, state);
}
fn email_request() -> CaptureBinaryInboxRequest {
    CaptureBinaryInboxRequest {
        id: Uuid::new_v4(),
        title: "Harbor email".into(),
        original_name: Some("plural.eml".into()),
        bytes: include_bytes!(
            "../../../experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml"
        )
        .to_vec(),
    }
}

#[test]
fn guided_import_read_offline_source_approval_and_restart_preserve_evidence() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let command = state.open_inbox().unwrap();
    send(&worker, &mut state, command);
    state.draft = crate::draft::DraftForm::new(None);
    state.draft.as_mut().unwrap().text = "Unfinished owner work".into();
    let input = state.draft.as_ref().unwrap().id;
    let request = email_request();
    let command = state.import_binary_inbox(request.clone()).unwrap();
    send(&worker, &mut state, command);
    let preview = state
        .inbox_queue
        .preview
        .clone()
        .expect("import must open readable extraction");
    assert_eq!(preview.original.capture.id, request.id);
    let extraction = preview.extraction.as_ref().unwrap();
    assert_eq!(
        (
            extraction.sources.len(),
            extraction.assets.len(),
            extraction.occurrences.len(),
            extraction.gaps.len()
        ),
        (8, 1, 3, 4)
    );
    assert!(
        state
            .inbox_queue
            .page
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.item.capture.id == request.id)
    );
    let source = InboxSourceRequest {
        candidate: preview.request.clone(),
        proposal_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        path: "harbor-source.md".into(),
        title: "Harbor original".into(),
    };
    // A missing selection cannot turn the same gesture into an implicit model/account route.
    assert!(state.begin_guided_source(source.clone(), true).is_none());
    assert!(state.proposals.is_empty());
    let command = state.begin_guided_source(source.clone(), false).unwrap();
    send(&worker, &mut state, command);
    assert!(!state.guided_source_busy());
    assert_eq!(
        state.guided_source_record().unwrap().draft.id,
        source.proposal_id
    );
    assert_eq!(state.guided_related_proposals().len(), 1);
    assert_eq!(state.draft.as_ref().unwrap().id, input);
    assert_eq!(state.draft.as_ref().unwrap().text, "Unfinished owner work");
    fixture.unchanged();
    assert!(
        state.open_review(source.proposal_id).is_none(),
        "unfinished form must still guard navigation"
    );
    // A separate clean presentation can review the retained proposal without discarding that form.
    let mut approver = super::state();
    let command = approver.open_review(source.proposal_id).unwrap();
    send(&worker, &mut approver, command);
    let exact = approver.capture_approval(false).unwrap();
    let command = approver.confirm_approval(&exact).unwrap();
    // This single approval is an explicit test gesture; read-only followups remain constrained.
    send(&worker, &mut approver, command);
    assert!(fixture.0.path().join("vault/harbor-source.md").is_file());
    let source_bytes = fs::read(fixture.0.path().join("vault/harbor-source.md")).unwrap();
    let original_path = preview
        .original
        .capture
        .copy
        .directory
        .join(preview.original.capture.copy_name());
    assert_eq!(fs::read(&original_path).unwrap(), request.bytes);
    worker.shutdown().unwrap();
    let mut worker = fixture.worker();
    let mut reopened = super::state();
    // The fixture consumes Ready before creating AiState; mirror the real
    // startup's proposal inventory request before opening retained reading.
    let command = reopened.command(Pending::Proposals, AppCommand::Proposals(None));
    send(&worker, &mut reopened, command);
    let command = reopened.open_inbox().unwrap();
    send(&worker, &mut reopened, command);
    let command = reopened.select_guided_inbox(request.id).unwrap();
    send(&worker, &mut reopened, command);
    assert_eq!(reopened.inbox_queue.guided.snapshots.len(), 1);
    assert_eq!(
        reopened.inbox_queue.preview.as_ref().unwrap().extraction,
        preview.extraction
    );
    assert!(
        reopened.inbox_queue.batch.is_none(),
        "reopen must not reconvert"
    );
    assert_eq!(
        fs::read(fixture.0.path().join("vault/harbor-source.md")).unwrap(),
        source_bytes
    );
    assert_eq!(
        reopened.guided_source_record().unwrap().state,
        ProposalState::Applied
    );
    reopened.selection = Some(brn_workflow::Selection {
        provider: brn_workflow::Provider::Chatgpt,
        model: "gpt-6-luna".into(),
    });
    reopened.effort = Some(brn_workflow::ReasoningEffort::Medium);
    let command = reopened
        .begin_guided_investigation(source.proposal_id)
        .unwrap();
    let (id, event) = reply(&worker, command);
    reopened.generation += 1;
    assert!(
        reopened.apply(id, event).is_empty(),
        "changed generation must also fence approved-Source investigation"
    );
    let command = reopened
        .begin_guided_investigation(source.proposal_id)
        .unwrap();
    let (id, event) = reply(&worker, command);
    let AppEvent::InboxIntakeBinding(ref held_binding) = event else {
        panic!("retained collection binding");
    };
    let duplicate = AppEvent::InboxIntakeBinding(held_binding.clone());
    let submitted = reopened.apply(id, event);
    assert_eq!(submitted.len(), 1);
    let AppCommand::AnalyzeInboxActions(investigation) = &submitted[0].1 else {
        panic!("one explicit investigation");
    };
    let binding = investigation.intake.as_ref().unwrap();
    assert_eq!(binding.source_proposal.id, source.proposal_id);
    assert_eq!(binding.assets.len(), 1);
    assert_eq!(binding.occurrences.len(), 3);
    assert_eq!(investigation.selection, reopened.selection.clone().unwrap());
    assert_eq!(investigation.effort, reopened.effort.unwrap());
    assert!(
        reopened.apply(id, duplicate).is_empty(),
        "a duplicate binding reply must not submit another investigation"
    );
    // Intercept the provider command: this state test makes no live/model call.
    assert_eq!(
        fs::read(fixture.0.path().join("vault/harbor-source.md")).unwrap(),
        source_bytes
    );
    worker.shutdown().unwrap();
}

#[test]
fn guided_saved_versions_require_choice_and_late_import_preserves_navigation() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let command = state.open_inbox().unwrap();
    send(&worker, &mut state, command);
    let request = email_request();
    let command = state.import_binary_inbox(request.clone()).unwrap();
    send(&worker, &mut state, command);
    let command = state.read_guided_original().unwrap();
    send(&worker, &mut state, command);
    let command = state.select_guided_inbox(request.id).unwrap();
    send(&worker, &mut state, command);
    assert_eq!(state.inbox_queue.guided.snapshots.len(), 2);
    assert!(
        state.inbox_queue.preview.is_none(),
        "no implicit latest version"
    );
    let version = state.inbox_queue.guided.snapshots[0].id;
    assert!(state.select_saved_inbox_extraction(version));
    let held = state.inbox_queue.preview.clone();
    let mut duplicate = request.clone();
    duplicate.id = Uuid::new_v4();
    let command = state.import_binary_inbox(duplicate.clone()).unwrap();
    // A navigation gesture after admission invalidates the capture's presentation intent.
    worker.submit(command.0, command.1).unwrap();
    let selected = state.select_guided_inbox(request.id).unwrap();
    send(&worker, &mut state, selected);
    assert_eq!(state.inbox_queue.guided.selected, Some(request.id));
    assert_eq!(
        state
            .inbox_queue
            .capture_result
            .as_ref()
            .unwrap()
            .capture
            .id,
        duplicate.id
    );
    assert_eq!(
        state.inbox_queue.preview, held,
        "returning preserves the exact owner-chosen version"
    );
    assert!(state.select_saved_inbox_extraction(version));
    assert_eq!(state.inbox_queue.preview, held);
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[test]
fn guided_discovery_rejects_misbound_reply_and_source_failure_can_retry() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let command = state.open_inbox().unwrap();
    send(&worker, &mut state, command);
    let request = email_request();
    let command = state.import_binary_inbox(request.clone()).unwrap();
    send(&worker, &mut state, command);
    let selected = state.select_guided_inbox(request.id).unwrap();
    let (read_id, read) = reply(&worker, selected);
    let mut discovery = state.apply(read_id, read);
    assert_eq!(discovery.len(), 1);
    let (id, event) = reply(&worker, discovery.pop().unwrap());
    let AppEvent::InboxRetainedExtractions { item_id, snapshots } = event else {
        panic!("saved extraction response")
    };
    state.apply(
        id,
        AppEvent::InboxRetainedExtractions {
            item_id: Uuid::new_v4(),
            snapshots: snapshots.clone(),
        },
    );
    assert!(state.pending.contains_key(&id));
    assert!(state.inbox_queue.preview.is_none());
    state.apply(
        id,
        AppEvent::InboxRetainedExtractions { item_id, snapshots },
    );
    assert!(!state.pending.contains_key(&id));
    let preview = state.inbox_queue.preview.as_ref().unwrap();
    let source = InboxSourceRequest {
        candidate: preview.request.clone(),
        proposal_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        path: "retained.md".into(),
        title: "Retained Source".into(),
    };
    let command = state.begin_guided_source(source.clone(), false).unwrap();
    assert!(state.guided_source_busy());
    state.apply(
        command.0,
        AppEvent::Failed(brn_workflow::WorkflowError::msg(
            "synthetic preparation failure",
        )),
    );
    assert!(!state.guided_source_busy());
    assert!(state.inbox_queue.source_error.is_some());
    let command = state.begin_guided_source(source.clone(), false).unwrap();
    send(&worker, &mut state, command);
    assert_eq!(
        state.guided_source_record().unwrap().draft.id,
        source.proposal_id
    );
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[test]
fn guided_investigation_requires_frozen_explicit_selection_and_submits_once() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    let mut state = state();
    let command = state.open_inbox().unwrap();
    send(&worker, &mut state, command);
    let command = state.import_binary_inbox(email_request()).unwrap();
    send(&worker, &mut state, command);
    let source = InboxSourceRequest {
        candidate: state.inbox_queue.preview.as_ref().unwrap().request.clone(),
        proposal_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        path: "review-source.md".into(),
        title: "Review Source".into(),
    };
    let command = state.begin_guided_source(source.clone(), false).unwrap();
    send(&worker, &mut state, command);
    assert!(
        state
            .begin_guided_investigation(source.proposal_id)
            .is_none()
    );
    state.selection = Some(brn_workflow::Selection {
        provider: brn_workflow::Provider::Chatgpt,
        model: "gpt-6-luna".into(),
    });
    state.effort = Some(brn_workflow::ReasoningEffort::Medium);
    let command = state
        .begin_guided_investigation(source.proposal_id)
        .unwrap();
    let (id, event) = reply(&worker, command);
    state.generation += 1;
    assert!(
        state.apply(id, event).is_empty(),
        "changed selection generation cancels automatic submission"
    );
    assert!(state.active.is_none());
    let command = state
        .begin_guided_investigation(source.proposal_id)
        .unwrap();
    let (id, event) = reply(&worker, command);
    let submitted = state.apply(id, event);
    assert_eq!(submitted.len(), 1);
    let AppCommand::AnalyzeInboxActions(request) = &submitted[0].1 else {
        panic!("one explicit investigation request")
    };
    assert_eq!(
        request.intake.as_ref().unwrap().source_proposal.id,
        source.proposal_id
    );
    assert_eq!(request.selection, state.selection.clone().unwrap());
    assert_eq!(request.effort, state.effort.unwrap());
    assert!(
        state
            .begin_guided_investigation(source.proposal_id)
            .is_none()
    );
    // Deliberately do not submit the intercepted provider command. No account or model is contacted.
    fixture.unchanged();
    worker.shutdown().unwrap();
}

#[test]
fn guided_twenty_sixth_import_and_back_navigation_keep_exact_reading() {
    let fixture = Fixture::new();
    let mut worker = fixture.worker();
    for index in 0..25 {
        let mut request = request(InboxKind::Text, "Older retained original");
        request.title = format!("Older {index}");
        let (_, event) = reply(&worker, (request.id, AppCommand::CaptureInbox(request)));
        assert!(matches!(event, AppEvent::InboxCaptured(_)));
    }
    std::thread::sleep(Duration::from_millis(2));
    let mut state = state();
    let command = state.open_inbox().unwrap();
    send(&worker, &mut state, command);
    let request = email_request();
    let command = state.import_binary_inbox(request.clone()).unwrap();
    send(&worker, &mut state, command);
    let held = state.inbox_queue.preview.clone().unwrap();
    assert_eq!(state.inbox_queue.page.as_ref().unwrap().total_count, 26);
    assert!(
        !state
            .inbox_queue
            .page
            .as_ref()
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.item.capture.id == request.id)
    );
    assert_eq!(state.inbox_queue.guided.selected, Some(request.id));
    state.close_inbox();
    let command = state.open_inbox().unwrap();
    send(&worker, &mut state, command);
    assert_eq!(
        state.inbox_queue.selected.as_ref().unwrap().item,
        held.original
    );
    assert_eq!(state.inbox_queue.preview, Some(held));
    assert!(!state.processing_pending());
    fixture.unchanged();
    worker.shutdown().unwrap();
}
