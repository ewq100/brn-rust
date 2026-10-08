use super::*;
use crate::WorkBudget;

#[test]
fn inbox_timeout_cancels_shared_proposals_retains_draft_and_waits_for_read_and_proposal_leases() {
    let fixture = Fixture::new();
    let (sent, leases) = mpsc::channel();
    let (noticed, cancelled) = mpsc::channel();
    let hook: ProposalAnswerHook = Arc::new(move |ask, _, reads, proposals, cancel, emit| {
        assert_eq!(
            ask.budget,
            Some(WorkBudget {
                max_tool_rounds: 4,
                timeout_seconds: 1
            })
        );
        let sent = sent.clone();
        let noticed = noticed.clone();
        Box::pin(async move {
            emit(AiEvent::Text("Tentative interpretation õ\r\n".into()));
            sent.send((reads, proposals)).unwrap();
            cancel.cancelled().await;
            noticed.send(()).unwrap();
            AiAnswer {
                text: "late completed interpretation".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let source = capture_source(
        &worker,
        "Original exact evidence for a tentative Action\r\n",
    );
    let mut request = request(&source);
    let budget = WorkBudget {
        max_tool_rounds: 4,
        timeout_seconds: 1,
    };
    request.budget = Some(budget);
    worker
        .submit(
            request.id,
            AppCommand::AnalyzeInboxActions(Box::new(request.clone())),
        )
        .unwrap();
    let (reads, proposals) = leases.recv_timeout(Duration::from_secs(10)).unwrap();
    let receipt = proposals.propose_actions(args()).unwrap();
    assert!(receipt["stamp"]["id"].as_str().is_some());
    cancelled.recv_timeout(Duration::from_secs(10)).unwrap();
    let mut saw_stopping = false;
    let mut wait = OperationWait::new(request.id, "budget stopping");
    while !saw_stopping {
        let (_, event) = wait.event(&worker);
        match event {
            AppEvent::Chat(ChatEvent::BudgetStopping { id, .. }) if id == request.id => {
                saw_stopping = true
            }
            AppEvent::Chat(ChatEvent::Finished { id, .. }) if id == request.id => {
                panic!("finished before retained leases drained")
            }
            _ => {}
        }
    }
    let mut changed = args();
    changed.title.push_str(" after cancellation");
    assert_eq!(
        proposals.propose_actions(changed).unwrap_err().kind,
        AiErrorKind::ToolRejected
    );
    assert!(
        matches!(reply(&worker,AppCommand::Turn(request.id)),AppEvent::Turn(Some(turn)) if turn.status==WorkTurnStatus::Running)
    );
    let retained = analysis(&worker, request.id);
    assert_eq!(retained.budget, Some(budget));
    assert_eq!(retained.proposals.len(), 1);
    assert_eq!(retained.proposals[0].state, ProposalState::Draft);
    no_actions(&worker);
    drop(proposals);
    // The separate read lease still fences finalization after proposal release.
    assert!(
        matches!(reply(&worker,AppCommand::Turn(request.id)),AppEvent::Turn(Some(turn)) if turn.status==WorkTurnStatus::Running)
    );
    drop(reads);
    let turn = finish(&worker, &request).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Failed);
    assert_eq!(turn.error_code.as_deref(), Some("time_limit_reached"));
    assert_eq!(turn.answer, "Tentative interpretation õ\r\n");
    assert_eq!(analysis(&worker, request.id).proposals.len(), 1);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
    let mut worker = fixture.start(Hooks::default());
    let persisted = analysis(&worker, request.id);
    assert_eq!(persisted.budget, Some(budget));
    assert_eq!(persisted.proposals.len(), 1);
    assert_eq!(persisted.turn.unwrap().answer, turn.answer);
    let mut replay = request.clone();
    replay.budget = None;
    assert_eq!(json!(analyze(&worker, &replay).unwrap()), json!(turn));
    replay.budget = Some(WorkBudget::default());
    assert_eq!(
        analyze(&worker, &replay).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    no_actions(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn historical_unbudgeted_unfinished_inbox_reservation_is_visible_but_cannot_restart_inference() {
    let fixture = Fixture::new();
    let mut worker = fixture.start(Hooks::default());
    let source = capture_source(&worker, "Historical exact evidence\r\n");
    let request = request(&source);
    worker.shutdown().unwrap();
    let mut app =
        crate::app::App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
    let (ask, capture) = app.prepare_inbox_action_request(&request).unwrap();
    let job = app
        .work_store_mut()
        .reserve_inbox_action(&capture, &ask.question)
        .unwrap();
    let canonical = serde_json::to_vec(&job).unwrap();
    assert!(app.work_store().turn(request.id).unwrap().is_none());
    assert_eq!(app.work_store().run_budget(request.id).unwrap(), None);
    drop(app);
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let hook: ProposalAnswerHook = Arc::new(move |_, _, _, _, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        panic!("historical reservation restarted inference")
    });
    let mut worker = fixture.start(Hooks {
        proposal_answer: Some(hook),
        ..Hooks::default()
    });
    let inspected = analysis(&worker, request.id);
    assert_eq!(inspected.budget, None);
    assert!(inspected.turn.is_none());
    assert_eq!(serde_json::to_vec(&inspected.job).unwrap(), canonical);
    assert_eq!(
        analyze(&worker, &request).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    let mut explicit = request.clone();
    explicit.budget = Some(WorkBudget::default());
    assert_eq!(
        analyze(&worker, &explicit).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(matches!(
        reply(&worker, AppCommand::RunBudget(request.id)),
        AppEvent::RunBudget { budget: None, .. }
    ));
    assert!(analysis(&worker, request.id).turn.is_none());
    no_actions(&worker);
    retained_original(&worker, &source);
    no_credentials(&fixture);
    worker.shutdown().unwrap();
}
