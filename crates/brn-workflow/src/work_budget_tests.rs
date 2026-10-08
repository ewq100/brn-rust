//! Synthetic owned-worker budget witnesses; no account or inference transport.
use super::*;
use crate::WorkBudget;
use std::sync::atomic::AtomicUsize;
use std::time::Instant;

fn query(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (actual, event) = worker
            .recv_event_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if actual == id {
            return event;
        }
    }
}

fn finish(worker: &AppWorker, id: Uuid) -> (WorkTurn, Vec<ChatEvent>) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut observed = Vec::new();
    loop {
        let (_, event) = worker
            .recv_event_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if let AppEvent::Chat(chat) = event {
            if chat.id() != id {
                continue;
            }
            match &chat {
                ChatEvent::Rejected { error, .. } | ChatEvent::PersistenceFailed { error, .. } => {
                    panic!("unexpected budget result: {:?}", error.kind)
                }
                ChatEvent::Finished { turn, .. } => {
                    let turn = turn.clone();
                    observed.push(chat);
                    return (turn, observed);
                }
                _ => observed.push(chat),
            }
        }
    }
}

#[test]
fn timeout_preserves_observed_partial_and_frozen_progress_and_refuses_late_completion() {
    let fixture = Fixture::new();
    let budget = WorkBudget {
        max_tool_rounds: 3,
        timeout_seconds: 1,
    };
    let (cancelled, seen_cancel) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |request, _, reads, cancel, emit| {
        assert_eq!(request.budget, Some(budget));
        let cancelled = cancelled.clone();
        Box::pin(async move {
            emit(AiEvent::Text("Saved partial õ 🦀\r\n".into()));
            emit(AiEvent::BudgetProgress {
                model_turns: 2,
                tool_rounds: 1,
                max_tool_rounds: 3,
            });
            // Malformed hook observations cannot change the frozen UI limits.
            emit(AiEvent::BudgetProgress {
                model_turns: 2,
                tool_rounds: 1,
                max_tool_rounds: 8,
            });
            emit(AiEvent::BudgetProgress {
                model_turns: 5,
                tool_rounds: 4,
                max_tool_rounds: 3,
            });
            cancel.cancelled().await;
            cancelled.send(()).unwrap();
            drop(reads);
            emit(AiEvent::Text("late provisional tail".into()));
            AiAnswer {
                text: "late completed answer".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.budget = Some(budget);
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let (turn, events) = finish(&worker, request.id);
    seen_cancel.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Failed);
    assert_eq!(turn.error_code.as_deref(), Some("time_limit_reached"));
    assert_eq!(turn.answer, "Saved partial õ 🦀\r\n");
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, ChatEvent::BudgetStopping { .. }))
            .count(),
        1
    );
    let progress: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            ChatEvent::BudgetProgress {
                budget,
                model_turns,
                tool_rounds,
                ..
            } => Some((*budget, *model_turns, *tool_rounds)),
            _ => None,
        })
        .collect();
    assert_eq!(progress, vec![(budget, 0, 0), (budget, 2, 1)]);
    assert!(
        matches!(query(&worker,AppCommand::RunBudget(request.id)),AppEvent::RunBudget { budget: Some(recorded), .. } if recorded==budget)
    );
    worker.shutdown().unwrap();
    let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
    assert_eq!(
        serde_json::to_vec(&store.turn(request.id).unwrap().unwrap()).unwrap(),
        serde_json::to_vec(&turn).unwrap()
    );
    assert_eq!(store.run_budget(request.id).unwrap(), Some(budget));
}

#[test]
fn manual_stop_remains_interrupted_and_never_claims_time_exhaustion() {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, reads, cancel, emit| {
        let started = started.clone();
        Box::pin(async move {
            emit(AiEvent::Text("manual partial\r\n".into()));
            started.send(()).unwrap();
            cancel.cancelled().await;
            drop(reads);
            AiAnswer {
                text: "manual partial\r\n".into(),
                terminal: AiTerminal::Failed(AiError::new(AiErrorKind::Other)),
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.budget = Some(WorkBudget {
        max_tool_rounds: 2,
        timeout_seconds: 60,
    });
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelTurn(request.id))
        .unwrap();
    let (turn, events) = finish(&worker, request.id);
    assert_eq!(turn.status, WorkTurnStatus::Interrupted);
    assert_eq!(turn.error_code, None);
    assert_eq!(turn.answer, "manual partial\r\n");
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ChatEvent::BudgetStopping { .. }))
    );
    worker.shutdown().unwrap();
}

#[test]
fn immediate_stop_on_initial_budget_progress_already_owns_the_cancellation_token() {
    let fixture = Fixture::new();
    let hook: AnswerHook = Arc::new(move |_, _, reads, cancel, _| {
        Box::pin(async move {
            cancel.cancelled().await;
            drop(reads);
            AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.budget = Some(WorkBudget {
        max_tool_rounds: 2,
        timeout_seconds: 60,
    });
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let (_, event) = worker
            .recv_event_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if matches!(event,AppEvent::Chat(ChatEvent::BudgetProgress { id,model_turns: 0,tool_rounds: 0,.. }) if id==request.id)
        {
            break;
        }
    }
    let stop = Uuid::new_v4();
    worker
        .submit(stop, AppCommand::CancelTurn(request.id))
        .unwrap();
    let mut acknowledged = false;
    let mut finished = None;
    while !acknowledged || finished.is_none() {
        let (id, event) = worker
            .recv_event_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        match event {
            AppEvent::TurnCancelRequested { accepted, .. } if id == stop => {
                assert!(accepted);
                acknowledged = true;
            }
            AppEvent::Chat(ChatEvent::Finished { id, turn, .. }) if id == request.id => {
                finished = Some(turn)
            }
            AppEvent::Chat(ChatEvent::BudgetStopping { .. }) => {
                panic!("manual Stop became a timeout")
            }
            _ => {}
        }
    }
    assert_eq!(finished.unwrap().status, WorkTurnStatus::Interrupted);
    worker.shutdown().unwrap();
}

#[test]
fn recorded_budget_survives_restart_omitted_replay_and_changed_choice_refuses_without_inference() {
    let fixture = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let budget = WorkBudget {
        max_tool_rounds: 16,
        timeout_seconds: 180,
    };
    let hook: AnswerHook = Arc::new(move |request, _, reads, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        assert_eq!(request.budget, Some(budget));
        Box::pin(async move {
            drop(reads);
            AiAnswer {
                text: "frozen answer".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook.clone()),
        ..Hooks::default()
    });
    let mut request = fixture.request();
    request.budget = Some(budget);
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let (turn, _) = finish(&worker, request.id);
    worker.shutdown().unwrap();
    std::fs::remove_file(fixture.base.path().join("vault/a.md")).unwrap();
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    let mut replay = request.clone();
    replay.budget = None;
    worker
        .submit(replay.id, AppCommand::Ask(replay.clone()))
        .unwrap();
    let (again, events) = finish(&worker, replay.id);
    assert_eq!(
        serde_json::to_vec(&again).unwrap(),
        serde_json::to_vec(&turn).unwrap()
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ChatEvent::BudgetProgress { .. }))
    );
    assert!(
        matches!(query(&worker,AppCommand::RunBudget(request.id)),AppEvent::RunBudget { budget: Some(recorded),.. } if recorded==budget)
    );
    replay.budget = Some(WorkBudget::default());
    worker.submit(replay.id, AppCommand::Ask(replay)).unwrap();
    assert!(
        matches!(event(&worker).1,AppEvent::Chat(ChatEvent::Rejected { error,.. }) if error.kind==ErrorKind::OperationConflict)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    worker.shutdown().unwrap();
}

#[test]
fn invalid_budget_refuses_before_turn_or_metadata_or_model_work() {
    let fixture = Fixture::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let called = calls.clone();
    let hook: AnswerHook = Arc::new(move |_, _, _, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        panic!("invalid budget reached model hook")
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    for budget in [
        WorkBudget {
            max_tool_rounds: 0,
            timeout_seconds: 300,
        },
        WorkBudget {
            max_tool_rounds: 8,
            timeout_seconds: 3601,
        },
    ] {
        let mut request = fixture.request();
        request.budget = Some(budget);
        worker
            .submit(request.id, AppCommand::Ask(request.clone()))
            .unwrap();
        assert!(matches!(
            event(&worker).1,
            AppEvent::Chat(ChatEvent::Rejected { .. })
        ));
        assert!(matches!(
            query(&worker, AppCommand::Turn(request.id)),
            AppEvent::Turn(None)
        ));
        assert!(matches!(
            query(&worker, AppCommand::RunBudget(request.id)),
            AppEvent::RunBudget { budget: None, .. }
        ));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    worker.shutdown().unwrap();
}
