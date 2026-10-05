use crate::{
    ErrorKind, Result, WorkflowError,
    app::{AppConfig, model_history},
    app_worker::{self, AppCommand, AppEvent, AppWorker},
    chat_worker::{AccountCommand, AccountEvent, AccountReply, AskRequest, ChatEvent, Hooks},
    models::{ModelInstallReport, ModelInstallRequest},
};
use brn_ai::{
    AiAnswer, AiError, AiErrorKind, AiEvent, AiTerminal, HistoryPair, LoginPrompt, Provider,
    ReadTools, Selection,
};
use brn_store::work::{WorkTurn, WorkTurnStatus};
use std::{
    future::Future,
    path::Path,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[cfg(target_os = "macos")]
#[path = "proposal_rewrite_tests.rs"]
mod rewrite;

type AnswerFuture = Pin<Box<dyn Future<Output = AiAnswer> + Send>>;
pub(crate) type AnswerHook = Arc<
    dyn Fn(
            AskRequest,
            Vec<HistoryPair>,
            Arc<dyn ReadTools>,
            CancellationToken,
            Arc<dyn Fn(AiEvent) + Send + Sync>,
        ) -> AnswerFuture
        + Send
        + Sync,
>;
pub(crate) type ProposalAnswerHook = Arc<
    dyn Fn(
            AskRequest,
            Vec<HistoryPair>,
            Arc<dyn ReadTools>,
            Arc<dyn brn_ai::ActionProposalTools>,
            CancellationToken,
            Arc<dyn Fn(AiEvent) + Send + Sync>,
        ) -> AnswerFuture
        + Send
        + Sync,
>;
type AccountFuture = Pin<Box<dyn Future<Output = AccountReply> + Send>>;
pub(crate) type AccountHook = Arc<
    dyn Fn(
            AccountCommand,
            CancellationToken,
            Arc<dyn Fn(LoginPrompt) + Send + Sync>,
        ) -> AccountFuture
        + Send
        + Sync,
>;
pub(crate) type InstallHook = Arc<
    dyn Fn(ModelInstallRequest, &AtomicBool, &dyn Fn(u64, u64)) -> Result<ModelInstallReport>
        + Send
        + Sync,
>;
pub(crate) type LoadHook =
    Arc<dyn Fn(&Path, &AtomicBool) -> Result<crate::library::SharedEmbedder> + Send + Sync>;

struct Fixture {
    base: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        std::fs::create_dir(base.path().join("data")).unwrap();
        std::fs::create_dir(base.path().join("vault")).unwrap();
        std::fs::write(base.path().join("vault/a.md"), b"current").unwrap();
        Self { base }
    }
    fn config(&self) -> AppConfig {
        AppConfig {
            vault_root: Some(self.base.path().join("vault")),
            credentials_dir: Some(self.base.path().join("credentials")),
            model_dir: None,
        }
    }
    fn start(&self, hooks: Hooks) -> AppWorker {
        self.start_with_startup_timeout(hooks, Duration::from_secs(10))
    }
    fn start_with_startup_timeout(&self, hooks: Hooks, timeout: Duration) -> AppWorker {
        let worker = app_worker::start_test(
            self.base.path().join("data"),
            self.config(),
            hooks,
            None,
            None,
        )
        .unwrap();
        assert!(matches!(
            worker.recv_event_timeout(timeout).unwrap().1,
            AppEvent::Ready { .. }
        ));
        worker
    }
    fn request(&self) -> AskRequest {
        AskRequest {
            id: Uuid::new_v4(),
            conversation: None,
            question: "explicit question".into(),
            selection: Selection {
                provider: Provider::Chatgpt,
                model: "gpt-5.5".into(),
            },
            effort: Some(crate::ReasoningEffort::High),
            generation: 51,
        }
    }
}
fn event(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}
fn terminal(worker: &AppWorker, id: Uuid) -> WorkTurn {
    loop {
        let (actual, event) = event(worker);
        if let AppEvent::Chat(ChatEvent::Finished {
            id: turn,
            generation,
            turn: result,
        }) = event
        {
            assert_eq!(actual, id);
            assert_eq!(turn, id);
            assert_eq!(generation, 51);
            return result;
        }
    }
}

#[test]
fn session_timestamps_project_through_owned_chat_reads_replay_and_restart() {
    let fixture = Fixture::new();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let called = calls.clone();
    let hook: AnswerHook = Arc::new(move |_, _, _, _, _| {
        called.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            AiAnswer {
                text: "Exact synthetic answer õ\r\n".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let completed = terminal(&worker, request.id);
    assert!(completed.started_at_ms.is_some());
    assert!(completed.finished_at_ms >= completed.started_at_ms);
    let snapshot = serde_json::to_value(&completed).unwrap();
    let read = |worker: &AppWorker, command| {
        let id = Uuid::new_v4();
        worker.submit(id, command).unwrap();
        loop {
            let (returned, event) = event(worker);
            if returned == id {
                return event;
            }
        }
    };
    let AppEvent::Conversations(conversations) = read(&worker, AppCommand::Conversations) else {
        panic!("conversations");
    };
    assert_eq!(conversations.len(), 1);
    assert!(conversations[0].created_at_ms <= completed.started_at_ms.unwrap());
    assert!(conversations[0].last_activity_at_ms >= completed.finished_at_ms);
    let sessions = serde_json::to_value(conversations).unwrap();
    let AppEvent::Turns(turns) = read(&worker, AppCommand::Turns(completed.conversation_id)) else {
        panic!("turns");
    };
    assert_eq!(serde_json::to_value(&turns[0]).unwrap(), snapshot);
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert_eq!(
        serde_json::to_value(terminal(&worker, request.id)).unwrap(),
        snapshot
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "replay must not call the answer hook"
    );
    worker.shutdown().unwrap();
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: None,
        account: None,
    });
    let AppEvent::Conversations(conversations) = read(&worker, AppCommand::Conversations) else {
        panic!("restart sessions");
    };
    assert_eq!(serde_json::to_value(conversations).unwrap(), sessions);
    let AppEvent::Turn(Some(turn)) = read(&worker, AppCommand::Turn(completed.id)) else {
        panic!("restart turn");
    };
    assert_eq!(serde_json::to_value(turn).unwrap(), snapshot);
    worker.shutdown().unwrap();
    assert_eq!(
        std::fs::read(fixture.base.path().join("vault/a.md")).unwrap(),
        b"current"
    );
}

#[test]
fn missing_effort_is_refused_before_vault_selection_or_provider_admission() {
    let fixture = Fixture::new();
    let (called, calls) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, _, _, _| {
        called.send(()).unwrap();
        Box::pin(async {
            AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let mut request = fixture.request();
    request.effort = None;
    request.selection = Selection {
        provider: Provider::Copilot,
        model: "never-discovered".into(),
    };
    std::fs::rename(
        fixture.base.path().join("vault"),
        fixture.base.path().join("parked-vault"),
    )
    .unwrap();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Chat(ChatEvent::Rejected { error, .. }))
        if id == request.id && error.kind == ErrorKind::SelectionRequired)
    );
    worker.shutdown().unwrap();
    assert!(calls.try_recv().is_err());
    let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
    assert!(store.turn(request.id).unwrap().is_none());
    assert!(store.conversations().unwrap().is_empty());
}

#[test]
fn ask_effort_is_frozen_during_setting_changes_and_exact_replay() {
    let fixture = Fixture::new();
    let (started, captured) = mpsc::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    let released = Arc::new(Mutex::new(Some(released)));
    let hook: AnswerHook = Arc::new(move |request, _, tools, _, _| {
        let started = started.clone();
        let released = released.lock().unwrap().take().unwrap();
        Box::pin(async move {
            started.send(request.effort).unwrap();
            let _ = released.await;
            drop(tools);
            AiAnswer {
                text: "exact answer 🧭\r\n".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let captured_effort = captured.recv_timeout(Duration::from_secs(10)).unwrap();
    let setting = Uuid::new_v4();
    worker
        .submit(
            setting,
            AppCommand::SelectEffort(crate::ReasoningEffort::Low),
        )
        .unwrap();
    let saved = event(&worker);
    release.send(()).unwrap();
    let turn = terminal(&worker, request.id);
    assert_eq!(captured_effort, Some(crate::ReasoningEffort::High));
    assert!(matches!(saved, (id, AppEvent::EffortSaved) if id == setting));
    assert_eq!(turn.effort.as_deref(), Some("high"));
    let mut conflict = request.clone();
    conflict.effort = Some(crate::ReasoningEffort::Low);
    worker
        .submit(request.id, AppCommand::Ask(conflict))
        .unwrap();
    assert!(
        matches!(event(&worker).1, AppEvent::Chat(ChatEvent::Rejected { error, .. })
        if error.kind == ErrorKind::OperationConflict)
    );
    std::fs::rename(
        fixture.base.path().join("vault"),
        fixture.base.path().join("parked-vault"),
    )
    .unwrap();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    let replay = terminal(&worker, request.id);
    assert_eq!(replay.effort.as_deref(), Some("high"));
    assert_eq!(replay.answer, turn.answer);
    worker.shutdown().unwrap();
    let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
    assert_eq!(
        store.turn(request.id).unwrap().unwrap().effort.as_deref(),
        Some("high")
    );
    assert_eq!(store.setting("ai.effort").unwrap().as_deref(), Some("low"));
}

#[test]
fn stop_before_and_after_partial_is_durable_and_recovery_ack_does_not_wait_for_model() {
    for partial in ["", "retained partial"] {
        let fixture = Fixture::new();
        let (started, ready) = mpsc::channel();
        let answer: AnswerHook = Arc::new(move |_, _, tools, cancel, emit| {
            let started = started.clone();
            Box::pin(async move {
                emit(AiEvent::Text(partial.into()));
                started.send(()).unwrap();
                cancel.cancelled().await;
                drop(tools);
                AiAnswer {
                    text: partial.into(),
                    terminal: AiTerminal::Failed(AiError::new(AiErrorKind::Other)),
                }
            })
        });
        let mut worker = fixture.start(Hooks {
            rewrite: None,
            proposal_answer: None,
            answer: Some(answer),
            account: None,
        });
        let request = fixture.request();
        worker
            .submit(request.id, AppCommand::Ask(request.clone()))
            .unwrap();
        ready.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(matches!(
            event(&worker).1,
            AppEvent::Chat(ChatEvent::Text { generation: 51, .. })
        ));
        let recovery = Uuid::new_v4();
        worker
            .submit(
                recovery,
                AppCommand::RecoverEdit {
                    path: "a.md".into(),
                    base_sha256: [3; 32],
                    text: "owned recovery".into(),
                },
            )
            .unwrap();
        assert!(matches!(event(&worker), (id, AppEvent::EditRecovered) if id == recovery));
        let wrong = Uuid::new_v4();
        worker
            .submit(wrong, AppCommand::CancelTurn(Uuid::new_v4()))
            .unwrap();
        assert!(
            matches!(event(&worker), (id, AppEvent::TurnCancelRequested { accepted: false, .. }) if id == wrong)
        );
        let stop = Uuid::new_v4();
        worker
            .submit(stop, AppCommand::CancelTurn(request.id))
            .unwrap();
        let turn = terminal(&worker, request.id);
        assert_eq!(turn.status, WorkTurnStatus::Interrupted);
        assert_eq!(turn.answer, partial);
        assert_eq!(turn.provider, "chatgpt");
        assert_eq!(turn.model, "gpt-5.5");
        worker.shutdown().unwrap();
        let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
        assert_eq!(store.turn(request.id).unwrap().unwrap().answer, partial);
        assert_eq!(
            store.unsaved_edit("a.md").unwrap().unwrap().text,
            "owned recovery"
        );
    }
}

#[test]
fn cancellation_does_not_mask_credential_storage_or_stream_failure() {
    for kind in [
        AiErrorKind::Storage,
        AiErrorKind::UnsafeCredentials,
        AiErrorKind::Network,
    ] {
        let fixture = Fixture::new();
        let (started, ready) = mpsc::channel();
        let hook: AnswerHook = Arc::new(move |_, _, tools, cancel, emit| {
            let started = started.clone();
            Box::pin(async move {
                emit(AiEvent::Text("partial".into()));
                started.send(()).unwrap();
                cancel.cancelled().await;
                drop(tools);
                AiAnswer {
                    text: "partial".into(),
                    terminal: AiTerminal::Failed(AiError::new(kind)),
                }
            })
        });
        let mut worker = fixture.start(Hooks {
            rewrite: None,
            proposal_answer: None,
            answer: Some(hook),
            account: None,
        });
        let request = fixture.request();
        worker
            .submit(request.id, AppCommand::Ask(request.clone()))
            .unwrap();
        ready.recv_timeout(Duration::from_secs(10)).unwrap();
        worker
            .submit(Uuid::new_v4(), AppCommand::CancelTurn(request.id))
            .unwrap();
        let turn = terminal(&worker, request.id);
        assert_eq!(turn.status, WorkTurnStatus::Failed);
        assert_eq!(turn.answer, "partial");
        assert_eq!(
            turn.error_code.as_deref(),
            Some(match kind {
                AiErrorKind::Storage => "storage",
                AiErrorKind::UnsafeCredentials => "unsafe_credentials",
                _ => "network",
            })
        );
        worker.shutdown().unwrap();
    }
}

#[test]
fn status_and_target_disconnect_work_while_stream_pending_and_other_provider_stays_connected() {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, tools, cancel, _| {
        let started = started.clone();
        Box::pin(async move {
            started.send(()).unwrap();
            cancel.cancelled().await;
            drop(tools);
            AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let credentials = fixture.base.path().join("credentials");
    // Synthetic target/other-provider caches, no authentication or HTTP.
    std::fs::write(credentials.join("github-token"), b"SYNTHETIC").unwrap();
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let status = Uuid::new_v4();
    worker
        .submit(
            status,
            AppCommand::Account {
                id: status,
                command: AccountCommand::Status(Provider::Chatgpt),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Status(_), .. })) if id == status)
    );
    let disconnect = Uuid::new_v4();
    worker
        .submit(
            disconnect,
            AppCommand::Account {
                id: disconnect,
                command: AccountCommand::Disconnect(Provider::Chatgpt),
            },
        )
        .unwrap();
    let turn = terminal(&worker, request.id);
    assert_eq!(turn.status, WorkTurnStatus::Interrupted);
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Disconnected, .. })) if id == disconnect)
    );
    assert_eq!(
        std::fs::read(credentials.join("github-token")).unwrap(),
        b"SYNTHETIC"
    );
    worker.shutdown().unwrap();
}

#[test]
fn pending_login_cancel_is_not_disconnect_and_other_provider_discovery_is_recorded_before_selection()
 {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let hook: AccountHook = Arc::new(move |command, cancel, login| {
        let started = started.clone();
        Box::pin(async move {
            match command {
                AccountCommand::Connect(Provider::Chatgpt) => {
                    login(LoginPrompt {
                        verification_uri: "https://synthetic.invalid".into(),
                        user_code: "SYNTHETIC".into(),
                    });
                    started.send(()).unwrap();
                    cancel.cancelled().await;
                    AccountReply::Failed(AiError::new(AiErrorKind::Other))
                }
                AccountCommand::Models(Provider::Copilot) => {
                    AccountReply::Models(vec![brn_ai::ModelOption {
                        id: "explicit-discovery".into(),
                        live_qualified: false,
                    }])
                }
                _ => AccountReply::Status(brn_ai::AccountStatus {
                    provider: Provider::Copilot,
                    connected: false,
                    name: None,
                }),
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: None,
        account: Some(hook),
    });
    let connect = Uuid::new_v4();
    worker
        .submit(
            connect,
            AppCommand::Account {
                id: connect,
                command: AccountCommand::Connect(Provider::Chatgpt),
            },
        )
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Login { .. })) if id == connect)
    );
    let models = Uuid::new_v4();
    worker
        .submit(
            models,
            AppCommand::Account {
                id: models,
                command: AccountCommand::Models(Provider::Copilot),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Models(_), .. })) if id == models)
    );
    let select = Uuid::new_v4();
    worker
        .submit(
            select,
            AppCommand::Select(Selection {
                provider: Provider::Copilot,
                model: "explicit-discovery".into(),
            }),
        )
        .unwrap();
    assert!(matches!(event(&worker), (id, AppEvent::SelectionSaved) if id == select));
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelAccount(connect))
        .unwrap();
    loop {
        if matches!(event(&worker).1, AppEvent::Account(AccountEvent::Finished { id, reply: AccountReply::Cancelled, .. }) if id == connect)
        {
            break;
        }
    }
    worker.shutdown().unwrap();
}

#[test]
fn history_last_twenty_earlier_terminal_pairs_includes_partials_and_excludes_running() {
    let turns = (1..=22)
        .map(|i| WorkTurn {
            id: Uuid::new_v4(),
            conversation_id: Uuid::nil(),
            question: format!("q{i}"),
            answer: if i == 7 {
                String::new()
            } else {
                format!("a{i}")
            },
            provider: "copilot".into(),
            model: "snapshot".into(),
            effort: None,
            started_at_ms: None,
            finished_at_ms: None,
            status: if i == 22 {
                WorkTurnStatus::Running
            } else if i % 2 == 0 {
                WorkTurnStatus::Failed
            } else {
                WorkTurnStatus::Interrupted
            },
            error_code: Some("network".into()),
        })
        .collect::<Vec<_>>();
    let history = model_history(&turns);
    assert_eq!(history.len(), 20);
    assert_eq!(history[0].question, "q2");
    assert_eq!(history[19].question, "q21");
    assert_eq!(history[5].question, "q7");
    assert!(history[5].answer.is_empty());
    let wire = serde_json::to_string(&history).unwrap();
    assert!(!wire.contains("provider"));
    assert!(!wire.contains("snapshot"));
    assert!(!wire.contains("network"));
}

#[test]
fn mismatched_generation_and_second_active_request_are_refused_without_another_running_pair() {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, tools, cancel, _| {
        let started = started.clone();
        Box::pin(async move {
            started.send(()).unwrap();
            cancel.cancelled().await;
            drop(tools);
            AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert!(
        matches!(event(&worker).1, AppEvent::Chat(ChatEvent::AlreadyRunning { id, generation: 51, turn })
        if id == request.id && turn.status == WorkTurnStatus::Running)
    );
    assert!(
        ready.try_recv().is_err(),
        "Running replay must not repeat the model"
    );
    let mut changed = request.clone();
    changed.generation += 1;
    worker.submit(changed.id, AppCommand::Ask(changed)).unwrap();
    assert!(
        matches!(event(&worker).1, AppEvent::Chat(ChatEvent::Rejected { error, .. }) if error.kind == ErrorKind::OperationConflict)
    );
    let second = fixture.request();
    worker.submit(second.id, AppCommand::Ask(second)).unwrap();
    assert!(
        matches!(event(&worker).1, AppEvent::Chat(ChatEvent::Rejected { error, .. }) if error.kind == ErrorKind::ToolsBusy)
    );
    worker.shutdown().unwrap();
    let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
    assert_eq!(store.conversations().unwrap().len(), 1);
    assert_eq!(
        store.turn(request.id).unwrap().unwrap().status,
        WorkTurnStatus::Interrupted
    );
}

#[test]
fn model_progress_is_correlated_download_precedes_idle_activation_and_control_does_not_wait() {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let answer: AnswerHook = Arc::new(move |_, _, tools, cancel, _| {
        let started = started.clone();
        Box::pin(async move {
            started.send(()).unwrap();
            cancel.cancelled().await;
            drop(tools);
            AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let (held, hold) = mpsc::channel();
    let hold = Arc::new(Mutex::new(hold));
    let install: InstallHook = Arc::new(move |request, _, progress| {
        progress(1, 2);
        hold.lock().unwrap().recv().unwrap();
        progress(2, 2);
        Ok(ModelInstallReport {
            directory: request.target().to_owned(),
            downloaded_bytes: 2,
        })
    });
    let (activated, activation) = mpsc::channel();
    let activate: LoadHook = Arc::new(move |_, _| {
        activated.send(()).unwrap();
        Ok(crate::library::SharedEmbedder::new(Box::new(
            SyntheticEmbedder(Arc::new(Mutex::new(Vec::new()))),
        )))
    });
    let mut worker = app_worker::start_test(
        fixture.base.path().join("data"),
        fixture.config(),
        Hooks {
            rewrite: None,
            proposal_answer: None,
            answer: Some(answer),
            account: None,
        },
        Some(install),
        Some(activate),
    )
    .unwrap();
    assert!(matches!(event(&worker).1, AppEvent::Ready { .. }));
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let model = Uuid::new_v4();
    worker
        .submit(
            model,
            AppCommand::DownloadModel {
                consent: true,
                target: fixture.base.path().join("model"),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::ModelDownload { received: 1, total: 2 }) if id == model)
    );
    held.send(()).unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::ModelDownload { received: 2, total: 2 }) if id == model)
    );
    assert!(matches!(event(&worker), (id, AppEvent::ModelDownloaded(_)) if id == model));
    assert!(activation.try_recv().is_err());
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelTurn(request.id))
        .unwrap();
    assert_eq!(
        terminal(&worker, request.id).status,
        WorkTurnStatus::Interrupted
    );
    assert!(matches!(event(&worker), (id, AppEvent::ModelInstalled) if id == model));
    activation.recv_timeout(Duration::from_secs(10)).unwrap();
    worker.shutdown().unwrap();
}

#[test]
fn installer_shutdown_cancels_and_joins_before_owner_release() {
    let fixture = Fixture::new();
    let (held, hold) = mpsc::channel();
    let hold = Arc::new(Mutex::new(hold));
    let (exited, exit) = mpsc::channel();
    let install: InstallHook = Arc::new(move |_, cancel, progress| {
        progress(1, 2);
        hold.lock().unwrap().recv().unwrap();
        assert!(cancel.load(Ordering::Acquire));
        exited.send(()).unwrap();
        Err(WorkflowError::cancelled())
    });
    let mut worker = app_worker::start_test(
        fixture.base.path().join("data"),
        fixture.config(),
        Hooks::default(),
        Some(install),
        None,
    )
    .unwrap();
    assert!(matches!(event(&worker).1, AppEvent::Ready { .. }));
    let model = Uuid::new_v4();
    worker
        .submit(
            model,
            AppCommand::DownloadModel {
                consent: true,
                target: fixture.base.path().join("model"),
            },
        )
        .unwrap();
    assert!(matches!(event(&worker).1, AppEvent::ModelDownload { .. }));
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelModelDownload(model))
        .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::ModelCancelRequested { accepted: true, .. }
    ));
    let data = fixture.base.path().join("data");
    let drain = std::thread::spawn(move || worker.shutdown());
    assert!(brn_store::WorkStore::open(&data).is_err());
    assert!(exit.try_recv().is_err());
    held.send(()).unwrap();
    drain.join().unwrap().unwrap();
    exit.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(brn_store::WorkStore::open(&data).is_ok());
}

#[test]
fn cancelling_downloaded_model_waiting_on_active_turn_wakes_the_application_lane() {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let answer: AnswerHook = Arc::new(move |_, _, tools, cancel, _| {
        let started = started.clone();
        Box::pin(async move {
            started.send(()).unwrap();
            cancel.cancelled().await;
            drop(tools);
            AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let install: InstallHook = Arc::new(move |request, _, _| {
        Ok(ModelInstallReport {
            directory: request.target().to_owned(),
            downloaded_bytes: 2,
        })
    });
    let mut worker = app_worker::start_test(
        fixture.base.path().join("data"),
        fixture.config(),
        Hooks {
            rewrite: None,
            proposal_answer: None,
            answer: Some(answer),
            account: None,
        },
        Some(install),
        None,
    )
    .unwrap();
    assert!(matches!(event(&worker).1, AppEvent::Ready { .. }));
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let model = Uuid::new_v4();
    worker
        .submit(
            model,
            AppCommand::DownloadModel {
                consent: true,
                target: fixture.base.path().join("model"),
            },
        )
        .unwrap();
    assert!(matches!(event(&worker), (id, AppEvent::ModelDownloaded(_)) if id == model));
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelModelDownload(model))
        .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::ModelCancelRequested { accepted: true, .. }
    ));
    assert!(
        matches!(event(&worker), (id, AppEvent::Failed(error)) if id == model && error.kind == ErrorKind::Cancelled)
    );
    worker.shutdown().unwrap();
}

#[test]
fn stream_failure_after_partial_commits_before_finished_and_restart_replay_does_not_submit() {
    let fixture = Fixture::new();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let wait = Arc::new(Mutex::new(Some(wait)));
    let hook: AnswerHook = Arc::new(move |_, _, tools, _, emit| {
        let wait = wait.lock().unwrap().take().expect("one turn only");
        Box::pin(async move {
            emit(AiEvent::Text("failure partial".into()));
            wait.await.unwrap();
            drop(tools);
            AiAnswer {
                text: "failure partial".into(),
                terminal: AiTerminal::Failed(AiError::new(AiErrorKind::Network)),
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::Chat(ChatEvent::Text { .. })
    ));
    release.send(()).unwrap();
    let turn = terminal(&worker, request.id);
    assert_eq!(turn.status, WorkTurnStatus::Failed);
    assert_eq!(turn.answer, "failure partial");
    let conn = rusqlite::Connection::open(fixture.base.path().join("data/brn.sqlite")).unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM messages WHERE turn_id = ?1 LIMIT 1",
            [request.id.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        status, "failed",
        "Finished is only emitted after durable commit"
    );
    drop(conn);
    worker.shutdown().unwrap();
    let mut worker = fixture.start(Hooks::default());
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert_eq!(terminal(&worker, request.id).answer, "failure partial");
    worker.shutdown().unwrap();
}

#[test]
fn persistence_failure_retains_in_memory_partial_and_shutdown_returns_the_finalization_error() {
    let fixture = Fixture::new();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let wait = Arc::new(Mutex::new(Some(wait)));
    let hook: AnswerHook = Arc::new(move |_, _, tools, _, emit| {
        let wait = wait.lock().unwrap().take().unwrap();
        Box::pin(async move {
            emit(AiEvent::Text("not committed".into()));
            wait.await.unwrap();
            drop(tools);
            AiAnswer {
                text: "not committed".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::Chat(ChatEvent::Text { .. })
    ));
    let conn = rusqlite::Connection::open(fixture.base.path().join("data/brn.sqlite")).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_finish BEFORE UPDATE ON messages BEGIN SELECT RAISE(ABORT, 'synthetic-write-failure'); END;").unwrap();
    release.send(()).unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Chat(ChatEvent::PersistenceFailed { generation: 51, partial, error, .. }))
        if id == request.id && partial == "not committed" && error.message.contains("only in memory"))
    );
    assert!(worker.shutdown().is_err());
    assert!(worker.shutdown().is_err());
    conn.execute_batch("DROP TRIGGER reject_finish").unwrap();
    drop(conn);
    let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
    let turn = store.turn(request.id).unwrap().unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Interrupted);
    assert!(
        turn.answer.is_empty(),
        "restart cannot reconstruct uncommitted stream text"
    );
}

#[test]
fn drop_joins_retained_blocking_tool_reads_and_keeps_the_owner_until_the_last_read_drains() {
    let fixture = Fixture::new();
    let (release, gate) = mpsc::channel();
    let gate = Arc::new(Mutex::new(Some(gate)));
    let (started, ready) = mpsc::channel();
    let (cancelled, cancellation) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, tools, cancel, _| {
        let (gate, started, cancelled) = (
            gate.lock().unwrap().take().unwrap(),
            started.clone(),
            cancelled.clone(),
        );
        Box::pin(async move {
            // Matches a Rig blocking read whose awaiting future was dropped on Stop.
            let _job = tokio::task::spawn_blocking(move || {
                started.send(()).unwrap();
                gate.recv().unwrap();
                assert_eq!(tools.read_note("a.md").unwrap().text, "current");
            });
            cancel.cancelled().await;
            cancelled.send(()).unwrap();
            AiAnswer {
                text: "partial before tool".into(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let (dropped, ended) = mpsc::channel();
    let drain = std::thread::spawn(move || {
        drop(worker);
        dropped.send(()).unwrap();
    });
    cancellation.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(brn_store::WorkStore::open(&fixture.base.path().join("data")).is_err());
    assert!(ended.try_recv().is_err());
    release.send(()).unwrap();
    drain.join().unwrap();
    ended.recv_timeout(Duration::from_secs(10)).unwrap();
    let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
    let turn = store.turn(request.id).unwrap().unwrap();
    assert_eq!(turn.status, WorkTurnStatus::Interrupted);
    assert_eq!(turn.answer, "partial before tool");
}

#[test]
fn disconnect_cancels_only_target_login_and_joins_it_before_cache_removal() {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let hook: AccountHook = Arc::new(move |command, cancel, _| {
        let started = started.clone();
        Box::pin(async move {
            if let AccountCommand::Connect(provider) = command {
                started.send(provider).unwrap();
                cancel.cancelled().await;
                AccountReply::Failed(AiError::new(AiErrorKind::Other))
            } else {
                AccountReply::Status(brn_ai::AccountStatus {
                    provider: Provider::Copilot,
                    connected: false,
                    name: None,
                })
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: None,
        account: Some(hook),
    });
    let credentials = fixture.base.path().join("credentials");
    std::fs::write(credentials.join("chatgpt.json"), b"synthetic target cache").unwrap();
    std::fs::write(credentials.join("github-token"), b"synthetic other cache").unwrap();
    let first = Uuid::new_v4();
    let other = Uuid::new_v4();
    for (id, provider) in [(first, Provider::Chatgpt), (other, Provider::Copilot)] {
        worker
            .submit(
                id,
                AppCommand::Account {
                    id,
                    command: AccountCommand::Connect(provider),
                },
            )
            .unwrap();
        assert_eq!(
            ready.recv_timeout(Duration::from_secs(10)).unwrap(),
            provider
        );
    }
    let disconnect = Uuid::new_v4();
    worker
        .submit(
            disconnect,
            AppCommand::Account {
                id: disconnect,
                command: AccountCommand::Disconnect(Provider::Chatgpt),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Cancelled, .. })) if id == first)
    );
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Disconnected, .. })) if id == disconnect)
    );
    assert!(!credentials.join("chatgpt.json").exists());
    assert!(credentials.join("github-token").exists());
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelAccount(other))
        .unwrap();
    loop {
        if matches!(event(&worker).1, AppEvent::Account(AccountEvent::Finished { id, reply: AccountReply::Cancelled, .. }) if id == other)
        {
            break;
        }
    }
    worker.shutdown().unwrap();
}

#[test]
fn application_activation_is_off_caller_and_account_events_and_disconnect_bypass_its_blocking_load()
{
    let fixture = Fixture::new();
    let (release, gate) = mpsc::channel();
    let gate = Arc::new(Mutex::new(gate));
    let (loading, loaded) = mpsc::channel();
    let activate: LoadHook = Arc::new(move |_, cancel| {
        assert_eq!(std::thread::current().name(), Some("brn-app"));
        loading.send(()).unwrap();
        gate.lock().unwrap().recv().unwrap();
        if cancel.load(Ordering::Acquire) {
            Err(WorkflowError::cancelled())
        } else {
            Ok(crate::library::SharedEmbedder::new(Box::new(
                SyntheticEmbedder(Arc::new(Mutex::new(Vec::new()))),
            )))
        }
    });
    let install: InstallHook = Arc::new(|request, _, _| {
        Ok(ModelInstallReport {
            directory: request.target().to_owned(),
            downloaded_bytes: 2,
        })
    });
    let account: AccountHook = Arc::new(|command, cancel, login| {
        Box::pin(async move {
            if matches!(command, AccountCommand::Connect(_)) {
                login(LoginPrompt {
                    verification_uri: "https://synthetic.invalid".into(),
                    user_code: "SYNTHETIC".into(),
                });
                cancel.cancelled().await;
                AccountReply::Failed(AiError::new(AiErrorKind::Other))
            } else {
                AccountReply::Status(brn_ai::AccountStatus {
                    provider: Provider::Copilot,
                    connected: false,
                    name: None,
                })
            }
        })
    });
    let mut worker = app_worker::start_test(
        fixture.base.path().join("data"),
        fixture.config(),
        Hooks {
            rewrite: None,
            proposal_answer: None,
            answer: None,
            account: Some(account),
        },
        Some(install),
        Some(activate),
    )
    .unwrap();
    assert!(matches!(event(&worker).1, AppEvent::Ready { .. }));
    let model = Uuid::new_v4();
    worker
        .submit(
            model,
            AppCommand::DownloadModel {
                consent: true,
                target: fixture.base.path().join("model"),
            },
        )
        .unwrap();
    assert!(matches!(event(&worker), (id, AppEvent::ModelDownloaded(_)) if id == model));
    loaded.recv_timeout(Duration::from_secs(10)).unwrap();
    let connect = Uuid::new_v4();
    worker
        .submit(
            connect,
            AppCommand::Account {
                id: connect,
                command: AccountCommand::Connect(Provider::Chatgpt),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Login { .. })) if id == connect)
    );
    let status = Uuid::new_v4();
    worker
        .submit(
            status,
            AppCommand::Account {
                id: status,
                command: AccountCommand::Status(Provider::Copilot),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Status(_), .. })) if id == status)
    );
    let disconnect = Uuid::new_v4();
    worker
        .submit(
            disconnect,
            AppCommand::Account {
                id: disconnect,
                command: AccountCommand::Disconnect(Provider::Chatgpt),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Cancelled, .. })) if id == connect)
    );
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Disconnected, .. })) if id == disconnect)
    );
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelModelDownload(model))
        .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::ModelCancelRequested { accepted: true, .. }
    ));
    release.send(()).unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Failed(error)) if id == model && error.kind == ErrorKind::Cancelled)
    );
    worker.shutdown().unwrap();
}

struct SyntheticEmbedder(Arc<Mutex<Vec<usize>>>);
impl crate::library::Embedder for SyntheticEmbedder {
    fn identity(&self) -> &str {
        "worker-synthetic-model"
    }
    fn dimension(&self) -> usize {
        2
    }
    fn embed(&mut self, texts: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        self.0.lock().unwrap().push(texts.len());
        Ok(texts.iter().map(|_| vec![1.0, 0.5]).collect())
    }
}

#[test]
fn one_model_adapter_is_shared_and_indexing_uses_bounded_batches_between_commands() {
    let fixture = Fixture::new();
    for i in 1..35 {
        std::fs::write(
            fixture.base.path().join(format!("vault/n{i}.md")),
            b"apple synthetic",
        )
        .unwrap();
    }
    let calls = Arc::new(Mutex::new(Vec::new()));
    let shared_calls = calls.clone();
    let activate: LoadHook = Arc::new(move |_, _| {
        Ok(crate::library::SharedEmbedder::new(Box::new(
            SyntheticEmbedder(shared_calls.clone()),
        )))
    });
    let install: InstallHook = Arc::new(|request, _, _| {
        Ok(ModelInstallReport {
            directory: request.target().to_owned(),
            downloaded_bytes: 0,
        })
    });
    let mut worker = app_worker::start_test(
        fixture.base.path().join("data"),
        fixture.config(),
        Hooks::default(),
        Some(install),
        Some(activate),
    )
    .unwrap();
    assert!(matches!(event(&worker).1, AppEvent::Ready { .. }));
    let model = Uuid::new_v4();
    worker
        .submit(
            model,
            AppCommand::DownloadModel {
                consent: true,
                target: fixture.base.path().join("model"),
            },
        )
        .unwrap();
    assert!(matches!(event(&worker).1, AppEvent::ModelDownloaded(_)));
    assert!(matches!(event(&worker).1, AppEvent::ModelInstalled));
    let mut batches = Vec::new();
    loop {
        match event(&worker) {
            (id, AppEvent::Indexing { embedded, total }) => {
                assert_eq!(id, model);
                batches.push(embedded);
                if embedded == total {
                    break;
                }
            }
            (_, AppEvent::Failed(error)) => panic!("{error}"),
            _ => panic!("unexpected indexing event"),
        }
    }
    assert_eq!(batches, vec![16, 32, 35]);
    assert_eq!(*calls.lock().unwrap(), vec![16, 16, 3]);
    use rusqlite::OptionalExtension;
    let conn = rusqlite::Connection::open(fixture.base.path().join("data/brn.sqlite")).unwrap();
    let saved: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'model.directory'",
            [],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    assert_eq!(
        saved,
        Some(
            fixture
                .base
                .path()
                .join("model")
                .to_str()
                .unwrap()
                .to_owned()
        )
    );
    drop(conn);
    let search = Uuid::new_v4();
    worker
        .submit(
            search,
            AppCommand::Search {
                query: "apple".into(),
                mode: crate::library::SearchMode::Semantic,
                limit: 10,
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Search(result)) if id == search && !result.keyword_only && !result.hits.is_empty())
    );
    assert_eq!(*calls.lock().unwrap(), vec![16, 16, 3, 1]);
    worker.shutdown().unwrap();
}

fn synthetic_loader() -> LoadHook {
    Arc::new(|_, _| {
        Ok(crate::library::SharedEmbedder::new(Box::new(
            SyntheticEmbedder(Arc::new(Mutex::new(Vec::new()))),
        )))
    })
}

fn assert_idle_without_failure(worker: &AppWorker) {
    app_worker::wait_test_idle(worker);
    while let Some((_, reply)) = worker.try_event() {
        if let AppEvent::Failed(error) = reply {
            panic!("unexpected idle failure: {error}");
        }
    }
}

fn bind_and_verify_semantic_index(worker: &AppWorker, fixture: &Fixture, refresh: bool) {
    let bind = Uuid::new_v4();
    worker
        .submit(
            bind,
            if refresh {
                AppCommand::Refresh
            } else {
                AppCommand::BindVault(fixture.base.path().join("vault"))
            },
        )
        .unwrap();
    let (id, reply) = event(worker);
    assert_eq!(id, bind);
    assert!(matches!(
        (refresh, reply),
        (false, AppEvent::VaultBound) | (true, AppEvent::Refreshed(_))
    ));
    assert!(
        matches!(event(worker), (id, AppEvent::Indexing { embedded: 1, total: 1 }) if id == bind)
    );
    let search = Uuid::new_v4();
    worker
        .submit(
            search,
            AppCommand::Search {
                query: "current".into(),
                mode: crate::library::SearchMode::Semantic,
                limit: 10,
            },
        )
        .unwrap();
    assert!(
        matches!(event(worker), (id, AppEvent::Search(result)) if id == search && !result.keyword_only && result.hits.len() == 1)
    );
}

#[test]
fn installed_model_startup_without_available_vault_has_no_indexing_failure() {
    for missing_bound in [true, false] {
        let fixture = Fixture::new();
        let vault = fixture.base.path().join("vault");
        if missing_bound {
            let mut app =
                crate::app::App::open(&fixture.base.path().join("data"), fixture.config()).unwrap();
            app.work_store_mut()
                .begin_turn(Uuid::new_v4(), None, "history", "chatgpt", "gpt-5.5")
                .unwrap();
            drop(app);
            std::fs::rename(&vault, fixture.base.path().join("parked-vault")).unwrap();
        }
        let mut config = fixture.config();
        config.vault_root = None;
        config.model_dir = Some(fixture.base.path().join("synthetic-model"));
        let mut worker = app_worker::start_test(
            fixture.base.path().join("data"),
            config,
            Hooks::default(),
            None,
            Some(synthetic_loader()),
        )
        .unwrap();
        assert!(
            matches!(event(&worker).1, AppEvent::Ready { vault_bound, model_installed: true } if vault_bound == missing_bound)
        );
        assert_idle_without_failure(&worker);
        let history = Uuid::new_v4();
        worker.submit(history, AppCommand::Conversations).unwrap();
        assert!(
            matches!(event(&worker), (id, AppEvent::Conversations(items)) if id == history && items.len() == usize::from(missing_bound))
        );
        let status = Uuid::new_v4();
        worker.submit(status, AppCommand::Status).unwrap();
        assert!(
            matches!(event(&worker), (id, AppEvent::Status(reply)) if id == status && reply.model_installed)
        );
        if missing_bound {
            std::fs::rename(fixture.base.path().join("parked-vault"), &vault).unwrap();
        }
        bind_and_verify_semantic_index(&worker, &fixture, missing_bound);
        worker.shutdown().unwrap();
    }
}

struct FailingEmbedder;

impl crate::library::Embedder for FailingEmbedder {
    fn identity(&self) -> &str {
        "worker-failing-model"
    }
    fn dimension(&self) -> usize {
        2
    }
    fn embed(&mut self, _: &[&str]) -> brn_retrieval::Result<Vec<Vec<f32>>> {
        Err(brn_retrieval::Error::ModelMismatch)
    }
}

#[test]
fn available_vault_indexing_errors_remain_correlated_failures() {
    let fixture = Fixture::new();
    let mut config = fixture.config();
    config.model_dir = Some(fixture.base.path().join("synthetic-model"));
    let mut worker = app_worker::start_test(
        fixture.base.path().join("data"),
        config,
        Hooks::default(),
        None,
        Some(Arc::new(|_, _| {
            Ok(crate::library::SharedEmbedder::new(Box::new(
                FailingEmbedder,
            )))
        })),
    )
    .unwrap();
    let (startup, reply) = event(&worker);
    assert!(matches!(
        reply,
        AppEvent::Ready {
            vault_bound: true,
            model_installed: true
        }
    ));
    assert!(
        matches!(event(&worker), (id, AppEvent::Failed(error)) if id == startup && error.kind == ErrorKind::IndexStale)
    );
    assert_idle_without_failure(&worker);
    worker.shutdown().unwrap();
}

#[test]
fn fresh_model_activation_without_vault_installs_without_later_failure() {
    let fixture = Fixture::new();
    let mut config = fixture.config();
    config.vault_root = None;
    let install: InstallHook = Arc::new(|request, _, _| {
        Ok(ModelInstallReport {
            directory: request.target().to_owned(),
            downloaded_bytes: 0,
        })
    });
    let mut worker = app_worker::start_test(
        fixture.base.path().join("data"),
        config,
        Hooks::default(),
        Some(install),
        Some(synthetic_loader()),
    )
    .unwrap();
    assert!(matches!(event(&worker).1, AppEvent::Ready { .. }));
    let model = Uuid::new_v4();
    worker
        .submit(
            model,
            AppCommand::DownloadModel {
                consent: true,
                target: fixture.base.path().join("model"),
            },
        )
        .unwrap();
    assert!(matches!(event(&worker), (id, AppEvent::ModelDownloaded(_)) if id == model));
    assert!(matches!(event(&worker), (id, AppEvent::ModelInstalled) if id == model));
    assert_idle_without_failure(&worker);
    let status = Uuid::new_v4();
    worker.submit(status, AppCommand::Status).unwrap();
    assert!(
        matches!(event(&worker), (id, AppEvent::Status(reply)) if id == status && reply.model_installed && reply.model_download.is_none())
    );
    bind_and_verify_semantic_index(&worker, &fixture, false);
    worker.shutdown().unwrap();
}

#[test]
fn shutdown_reports_credential_finalization_failure_instead_of_cancelled_success() {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let account: AccountHook = Arc::new(move |_, cancel, _| {
        let started = started.clone();
        Box::pin(async move {
            started.send(()).unwrap();
            cancel.cancelled().await;
            AccountReply::Failed(AiError::new(AiErrorKind::Storage))
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: None,
        account: Some(account),
    });
    let connect = Uuid::new_v4();
    worker
        .submit(
            connect,
            AppCommand::Account {
                id: connect,
                command: AccountCommand::Connect(Provider::Chatgpt),
            },
        )
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(worker.shutdown().is_err());
    assert!(
        matches!(event(&worker), (id, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Failed(error), .. }))
                    if id == connect && error.kind == AiErrorKind::Storage)
    );
}

#[test]
fn confirmed_completion_wins_over_stop_while_a_retained_tool_lease_drains() {
    let fixture = Fixture::new();
    let (release, gate) = mpsc::channel();
    let gate = Arc::new(Mutex::new(Some(gate)));
    let (finished_model, finished) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, tools, _, emit| {
        let (gate, finished_model) = (gate.lock().unwrap().take().unwrap(), finished_model.clone());
        Box::pin(async move {
            let _job = tokio::task::spawn_blocking(move || {
                gate.recv().unwrap();
                assert_eq!(tools.read_note("a.md").unwrap().text, "current");
            });
            emit(AiEvent::Text("confirmed complete".into()));
            finished_model.send(()).unwrap();
            AiAnswer {
                text: "confirmed complete".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    finished.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::Chat(ChatEvent::Text { .. })
    ));
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelTurn(request.id))
        .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::TurnCancelRequested { accepted: true, .. }
    ));
    release.send(()).unwrap();
    let turn = terminal(&worker, request.id);
    assert_eq!(turn.status, WorkTurnStatus::Completed);
    assert_eq!(turn.answer, "confirmed complete");
    worker.shutdown().unwrap();
}

#[test]
fn scoped_source_reads_are_forwarded_and_owned_until_the_completed_turn_drains() {
    use brn_ai::ReadScope;
    const SOURCE: &str = "\u{feff}---\r\nbrn_kind: source\r\n---\r\nneedle algne allikas\r\n";
    let fixture = Fixture::new();
    let source = fixture.base.path().join("vault/source.md");
    std::fs::write(&source, SOURCE).unwrap();
    let (release, gate) = mpsc::channel();
    let gate = Arc::new(Mutex::new(Some(gate)));
    let (finished_model, finished) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, tools, _, emit| {
        let gate = gate.lock().unwrap().take().unwrap();
        let finished_model = finished_model.clone();
        Box::pin(async move {
            assert_eq!(
                tools
                    .list_notes_scoped(None, None, ReadScope::Source)
                    .unwrap()
                    .notes[0]
                    .path,
                "source.md"
            );
            assert_eq!(
                tools
                    .search_notes_scoped("needle", 1, ReadScope::Source)
                    .unwrap()
                    .hits[0]
                    .path,
                "source.md"
            );
            assert_eq!(
                tools.read_note("source.md").unwrap_err().kind,
                AiErrorKind::ToolRejected
            );
            let _job = tokio::task::spawn_blocking(move || {
                gate.recv().unwrap();
                assert_eq!(
                    tools
                        .read_note_scoped("source.md", ReadScope::Source)
                        .unwrap()
                        .text,
                    SOURCE
                );
            });
            emit(AiEvent::Text("scoped source observed".into()));
            finished_model.send(()).unwrap();
            AiAnswer {
                text: "scoped source observed".into(),
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    finished.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::Chat(ChatEvent::Text { .. })
    ));
    worker
        .submit(Uuid::new_v4(), AppCommand::CancelTurn(request.id))
        .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::TurnCancelRequested { accepted: true, .. }
    ));
    assert!(
        worker
            .recv_event_timeout(Duration::from_millis(50))
            .is_err()
    );
    release.send(()).unwrap();
    let turn = terminal(&worker, request.id);
    assert_eq!(turn.status, WorkTurnStatus::Completed);
    assert_eq!(turn.answer, "scoped source observed");
    worker.shutdown().unwrap();
    assert_eq!(std::fs::read_to_string(source).unwrap(), SOURCE);
}

#[test]
fn shutdown_fences_queued_asks_and_reports_each_request_without_another_submission() {
    let fixture = Fixture::new();
    let (started, ready) = mpsc::channel();
    let hook: AnswerHook = Arc::new(move |_, _, tools, cancel, _| {
        let started = started.clone();
        Box::pin(async move {
            started.send(()).unwrap();
            cancel.cancelled().await;
            drop(tools);
            AiAnswer {
                text: String::new(),
                terminal: AiTerminal::Interrupted,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    let queued = (0..20).map(|_| fixture.request()).collect::<Vec<_>>();
    for request in &queued {
        worker
            .submit(request.id, AppCommand::Ask(request.clone()))
            .unwrap();
    }
    worker.shutdown().unwrap();
    let mut rejected = std::collections::HashSet::new();
    while let Some((id, event)) = worker.try_event() {
        if let AppEvent::Chat(ChatEvent::Rejected {
            id: inner,
            generation,
            error,
        }) = event
        {
            assert_eq!(id, inner);
            assert_eq!(generation, 51);
            assert!(matches!(
                error.kind,
                ErrorKind::Cancelled | ErrorKind::ToolsBusy
            ));
            rejected.insert(id);
        }
    }
    assert!(queued.iter().all(|request| rejected.contains(&request.id)));
    assert!(
        ready.try_recv().is_err(),
        "shutdown must not start another model"
    );
    let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
    assert_eq!(store.conversations().unwrap().len(), 1);
}

#[test]
fn account_uuid_conflicts_are_typed_and_never_start_another_operation() {
    let fixture = Fixture::new();
    let mut worker = fixture.start(Hooks::default());
    let id = Uuid::new_v4();
    assert!(
        worker
            .submit(
                Uuid::new_v4(),
                AppCommand::Account {
                    id,
                    command: AccountCommand::Status(Provider::Chatgpt)
                }
            )
            .is_err()
    );
    worker
        .submit(
            id,
            AppCommand::Account {
                id,
                command: AccountCommand::Status(Provider::Chatgpt),
            },
        )
        .unwrap();
    assert!(matches!(
        event(&worker).1,
        AppEvent::Account(AccountEvent::Finished {
            reply: AccountReply::Status(_),
            ..
        })
    ));
    worker
        .submit(
            id,
            AppCommand::Account {
                id,
                command: AccountCommand::Connect(Provider::Copilot),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (actual, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Rejected(error), .. }))
        if actual == id && error.kind == ErrorKind::OperationConflict)
    );
    worker.shutdown().unwrap();
}

#[test]
fn invalid_discovery_has_a_terminal_account_error_and_never_persists_a_substitute_list() {
    let fixture = Fixture::new();
    let hook: AccountHook = Arc::new(|_, _, _| {
        Box::pin(async {
            AccountReply::Models(vec![brn_ai::ModelOption {
                id: "invalid identifier".into(),
                live_qualified: false,
            }])
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: None,
        account: Some(hook),
    });
    let id = Uuid::new_v4();
    worker
        .submit(
            id,
            AppCommand::Account {
                id,
                command: AccountCommand::Models(Provider::Copilot),
            },
        )
        .unwrap();
    assert!(
        matches!(event(&worker), (actual, AppEvent::Account(AccountEvent::Finished { reply: AccountReply::Rejected(error), .. }))
        if actual == id && error.kind == ErrorKind::ModelRefused)
    );
    worker.shutdown().unwrap();
    let (store, _) = brn_store::WorkStore::open(&fixture.base.path().join("data")).unwrap();
    assert!(store.setting("ai.models.copilot").unwrap().is_none());
}

#[test]
fn new_ask_refreshes_the_library_before_the_model_can_read_the_tools() {
    let fixture = Fixture::new();
    let hook: AnswerHook = Arc::new(|_, _, tools, _, emit| {
        Box::pin(async move {
            let text = tools
                .list_notes(None, None)
                .unwrap()
                .notes
                .len()
                .to_string();
            emit(AiEvent::Text(text.clone()));
            drop(tools);
            AiAnswer {
                text,
                terminal: AiTerminal::Completed,
            }
        })
    });
    let mut worker = fixture.start(Hooks {
        rewrite: None,
        proposal_answer: None,
        answer: Some(hook),
        account: None,
    });
    std::fs::write(fixture.base.path().join("vault/new.md"), b"new fresh note").unwrap();
    let request = fixture.request();
    worker
        .submit(request.id, AppCommand::Ask(request.clone()))
        .unwrap();
    assert_eq!(terminal(&worker, request.id).answer, "2");
    worker.shutdown().unwrap();
}

// Full retained records are seeded through actual native approval/completion.
// Private RPC/error/lease tests above that boundary remain portable.
#[cfg(target_os = "macos")]
#[path = "action_read_tools_tests.rs"]
mod action_reads;

#[cfg(target_os = "macos")]
#[path = "action_proposal_tools_tests.rs"]
mod action_proposals;

#[cfg(target_os = "macos")]
#[path = "inbox_actions_tests.rs"]
mod inbox_actions;
