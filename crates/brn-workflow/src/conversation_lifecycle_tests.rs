//! Actual worker admission/drain/replay, without accounts or model inference.
use super::*;
use crate::conversations::*;
use brn_store::work::WorkStore;

fn reply(worker: &AppWorker, id: Uuid) -> AppEvent {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let (actual, event) = worker
            .recv_event_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
            .unwrap();
        if actual == id {
            return event;
        }
    }
}

fn seed(fixture: &Fixture) -> (WorkTurn, ConversationLifecycle) {
    let (mut store, _) = WorkStore::open(&fixture.base.path().join("data")).unwrap();
    let id = Uuid::new_v4();
    let (turn, _) = store
        .begin_turn_with_effort_and_budget(
            id,
            None,
            "Retained exact õ\r\n",
            "chatgpt",
            "gpt-5.5",
            Some("high"),
            Some(crate::WorkBudget {
                max_tool_rounds: 3,
                timeout_seconds: 180,
            }),
        )
        .unwrap();
    let turn = store
        .finish_turn(turn.id, WorkTurnStatus::Completed, "Saved λ\r\n", None)
        .unwrap();
    let lifecycle = store.conversation_lifecycle(turn.conversation_id).unwrap();
    (turn, lifecycle)
}

fn transition(worker: &AppWorker, request: &ConversationLifecycleRequest) -> AppEvent {
    worker
        .submit(
            request.operation_id,
            AppCommand::SetConversationLifecycle(request.clone()),
        )
        .unwrap();
    reply(worker, request.operation_id)
}

fn request(
    before: &ConversationLifecycle,
    target: ConversationState,
) -> ConversationLifecycleRequest {
    ConversationLifecycleRequest {
        operation_id: Uuid::new_v4(),
        expected: before.stamp,
        target,
    }
}

#[test]
fn archive_restart_restore_and_old_operation_replay_preserve_history_budget_and_timestamps() {
    let fixture = Fixture::new();
    let (saved, initial) = seed(&fixture);
    let mut worker = fixture.start(Hooks::default());
    let archive = request(&initial, ConversationState::Archived);
    let AppEvent::ConversationLifecycleChanged(archived) = transition(&worker, &archive) else {
        panic!("archive");
    };
    archived.validate().unwrap();
    assert_eq!(archived.current.state, ConversationState::Archived);
    let mut ask = fixture.request();
    ask.id = saved.id;
    ask.conversation = Some(saved.conversation_id);
    ask.question = saved.question.clone();
    worker.submit(ask.id, AppCommand::Ask(ask.clone())).unwrap();
    let replayed = terminal(&worker, ask.id);
    assert_eq!(
        serde_json::to_value(replayed).unwrap(),
        serde_json::to_value(&saved).unwrap()
    );
    ask.id = Uuid::new_v4();
    worker.submit(ask.id, AppCommand::Ask(ask.clone())).unwrap();
    let AppEvent::Chat(ChatEvent::Rejected { error, .. }) = reply(&worker, ask.id) else {
        panic!("archived new Ask must refuse");
    };
    assert_eq!(error.kind, ErrorKind::ContextStale);
    assert!(error.message.contains("Restore"));
    worker.shutdown().unwrap();

    let mut worker = fixture.start(Hooks::default());
    let restore = request(&archived.current, ConversationState::Active);
    let AppEvent::ConversationLifecycleChanged(restored) = transition(&worker, &restore) else {
        panic!("restore");
    };
    let AppEvent::ConversationLifecycleChanged(old_replay) = transition(&worker, &archive) else {
        panic!("old replay");
    };
    assert_eq!(old_replay.receipt, archived.receipt);
    assert_eq!(old_replay.current, restored.current);
    worker.shutdown().unwrap();
    let (store, _) = WorkStore::open(&fixture.base.path().join("data")).unwrap();
    assert_eq!(
        serde_json::to_value(store.turn(saved.id).unwrap().unwrap()).unwrap(),
        serde_json::to_value(&saved).unwrap()
    );
    assert_eq!(
        store.run_budget(saved.id).unwrap(),
        Some(crate::WorkBudget {
            max_tool_rounds: 3,
            timeout_seconds: 180
        })
    );
    assert_eq!(
        store.conversation_lifecycle(saved.conversation_id).unwrap(),
        restored.current
    );
    assert_eq!(
        std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
        b"current"
    );
}

#[test]
fn archive_refuses_active_and_cancelled_draining_work_but_exact_replay_remains_available() {
    let fixture = Fixture::new();
    let (saved, initial) = seed(&fixture);
    let (release, gate) = mpsc::channel();
    let gate = Arc::new(Mutex::new(Some(gate)));
    let (model_done, done) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, reads, _, emit| {
        let gate = gate.lock().unwrap().take().unwrap();
        let model_done = model_done.clone();
        Box::pin(async move {
            tokio::task::spawn_blocking(move || {
                gate.recv().unwrap();
                assert_eq!(reads.read_note("a.md").unwrap().text, "current");
            });
            emit(AiEvent::Text("confirmed answer".into()));
            model_done.send(()).unwrap();
            AiAnswer {
                text: "confirmed answer".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        answer: Some(hook),
        ..Hooks::default()
    });
    // Retain a historical archive/restore pair before another session is busy.
    let archive = request(&initial, ConversationState::Archived);
    let AppEvent::ConversationLifecycleChanged(archived) = transition(&worker, &archive) else {
        panic!("archive");
    };
    let restore = request(&archived.current, ConversationState::Active);
    let AppEvent::ConversationLifecycleChanged(restored) = transition(&worker, &restore) else {
        panic!("restore");
    };
    let mut ask = fixture.request();
    ask.conversation = Some(saved.conversation_id);
    worker.submit(ask.id, AppCommand::Ask(ask.clone())).unwrap();
    done.recv_timeout(Duration::from_secs(10)).unwrap();
    let fresh = request(&restored.current, ConversationState::Archived);
    let AppEvent::Failed(error) = transition(&worker, &fresh) else {
        panic!("busy archive");
    };
    assert_eq!(error.kind, ErrorKind::ToolsBusy);
    let AppEvent::ConversationLifecycleChanged(replayed) = transition(&worker, &archive) else {
        panic!("busy exact replay");
    };
    assert_eq!(replayed.current, restored.current);
    let cancel = Uuid::new_v4();
    worker
        .submit(cancel, AppCommand::CancelTurn(ask.id))
        .unwrap();
    assert!(matches!(
        reply(&worker, cancel),
        AppEvent::TurnCancelRequested { accepted: true, .. }
    ));
    let AppEvent::Failed(error) = transition(&worker, &fresh) else {
        panic!("draining archive");
    };
    assert_eq!(error.kind, ErrorKind::ToolsBusy);
    release.send(()).unwrap();
    terminal(&worker, ask.id);
    let AppEvent::ConversationLifecycleChanged(result) = transition(&worker, &fresh) else {
        panic!("settled archive");
    };
    assert_eq!(result.current.state, ConversationState::Archived);
    worker.shutdown().unwrap();
}

#[test]
fn already_admitted_lifecycle_change_settles_ahead_of_shutdown() {
    let fixture = Fixture::new();
    let (saved, initial) = seed(&fixture);
    let mut worker = fixture.start(Hooks::default());
    let archive = request(&initial, ConversationState::Archived);
    worker
        .submit(
            archive.operation_id,
            AppCommand::SetConversationLifecycle(archive.clone()),
        )
        .unwrap();
    worker.shutdown().unwrap();
    let AppEvent::ConversationLifecycleChanged(result) = reply(&worker, archive.operation_id)
    else {
        panic!("admitted archive acknowledgement");
    };
    assert_eq!(result.current.state, ConversationState::Archived);
    let (store, _) = WorkStore::open(&fixture.base.path().join("data")).unwrap();
    assert_eq!(
        store.conversation_lifecycle(saved.conversation_id).unwrap(),
        result.current
    );
}
