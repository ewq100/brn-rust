use super::*;
use crate::backups::BackupStatus;

fn fixture() -> (tempfile::TempDir, PathBuf, AppConfig) {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = base.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let config = AppConfig {
        vault_root: None,
        credentials_dir: Some(base.path().join("credentials")),
        model_dir: None,
    };
    (base, data, config)
}

fn next(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}

fn query(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    loop {
        let (actual, event) = next(worker);
        if actual == id {
            return event;
        }
        assert!(actual.is_nil() && matches!(event, AppEvent::BackupStatus(_)));
    }
}

fn status(worker: &AppWorker, command: AppCommand) -> BackupStatus {
    let AppEvent::BackupStatus(status) = query(worker, command) else {
        panic!("expected typed checkpoint reply");
    };
    status
}

#[test]
fn backup_worker_manual_checkpoint_reads_and_failure_keep_committed_recovery_successful() {
    let (_base, data, config) = fixture();
    let mut worker = AppWorker::start(data.clone(), config).unwrap();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    for command in [AppCommand::BackupStatus, AppCommand::CheckpointBackup] {
        assert!(worker.submit(Uuid::nil(), command).is_err());
    }
    let initial = status(&worker, AppCommand::CheckpointBackup);
    let unchanged = status(&worker, AppCommand::CheckpointBackup);
    assert_eq!(initial, unchanged);
    assert!(matches!(
        query(
            &worker,
            AppCommand::RecoverEdit {
                path: "draft.md".into(),
                base_sha256: [7; 32],
                text: "Exact õ\r\n".into()
            }
        ),
        AppEvent::EditRecovered
    ));
    let saved = status(&worker, AppCommand::CheckpointBackup);
    assert_ne!(saved.latest_path, initial.latest_path);
    assert!(saved.completed_at_ms.is_some() && saved.last_error.is_none());
    let backup_bytes = std::fs::read(&saved.latest_path).unwrap();
    let backups = data.join("backups");
    let held = data.join("held-backups");
    std::fs::rename(&backups, &held).unwrap();
    std::fs::write(&backups, b"occupied foreign file").unwrap();
    assert!(matches!(
        query(
            &worker,
            AppCommand::RecoverEdit {
                path: "draft.md".into(),
                base_sha256: [7; 32],
                text: "Later committed λ\r\n".into()
            }
        ),
        AppEvent::EditRecovered
    ));
    let failure = status(&worker, AppCommand::CheckpointBackup);
    assert!(failure.last_error.is_some());
    assert_eq!(failure.latest_path, saved.latest_path);
    worker.shutdown().unwrap();
    assert!(worker.events.try_iter().any(|(id, event)| matches!(event,
        AppEvent::BackupStatus(s) if id.is_nil() && s.last_error.is_some())));
    assert_eq!(std::fs::read(&backups).unwrap(), b"occupied foreign file");
    assert_eq!(
        std::fs::read(held.join(saved.latest_path.file_name().unwrap())).unwrap(),
        backup_bytes
    );
    std::fs::remove_file(&backups).unwrap();
    std::fs::rename(held, backups).unwrap();
    let (store, _) = brn_store::WorkStore::open(&data).unwrap();
    assert_eq!(
        store.unsaved_edit("draft.md").unwrap().unwrap().text,
        "Later committed λ\r\n"
    );
}

#[test]
fn backup_worker_idle_and_queued_commands_reach_due_checkpoint() {
    for queued in [false, true] {
        let (_base, data, config) = fixture();
        let mut worker = AppWorker::start_owned(
            data.clone(),
            config,
            Hooks {
                backup_interval: Some(if queued {
                    Duration::ZERO
                } else {
                    Duration::from_millis(5)
                }),
                ..Hooks::default()
            },
        )
        .unwrap();
        assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
        status(&worker, AppCommand::CheckpointBackup);
        let (entered_tx, entered) = mpsc::channel();
        let (release, release_rx) = mpsc::channel();
        worker
            .submit(
                Uuid::new_v4(),
                AppCommand::TestPause {
                    entered: entered_tx,
                    release: release_rx,
                },
            )
            .unwrap();
        entered.recv_timeout(Duration::from_secs(10)).unwrap();
        worker
            .submit(
                Uuid::new_v4(),
                AppCommand::RecoverEdit {
                    path: "queued.md".into(),
                    base_sha256: [8; 32],
                    text: "Retained while open".into(),
                },
            )
            .unwrap();
        if queued {
            for _ in 0..100 {
                worker
                    .submit(Uuid::new_v4(), AppCommand::Selection)
                    .unwrap();
            }
        }
        release.send(()).unwrap();
        let mut selections = 0;
        let checkpoint = loop {
            match next(&worker) {
                (id, AppEvent::BackupStatus(s)) => {
                    assert!(id.is_nil());
                    if s.completed_at_ms.is_some() && s.last_error.is_none() {
                        break s;
                    }
                }
                (_, AppEvent::Selection(_)) => selections += 1,
                _ => {}
            }
        };
        if queued {
            assert!(selections < 100, "traffic starved the due checkpoint");
        }
        assert!(checkpoint.latest_path.exists());
        let conn = rusqlite::Connection::open_with_flags(
            &checkpoint.latest_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let text: String = conn
            .query_row(
                "SELECT text FROM unsaved_edits WHERE path='queued.md'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(text, "Retained while open");
        worker.shutdown().unwrap();
    }
}

#[test]
fn backup_worker_shutdown_copies_final_cancelled_chat_settlement() {
    let (base, data, mut config) = fixture();
    let vault = base.path().join("vault");
    std::fs::create_dir(&vault).unwrap();
    config.vault_root = Some(vault);
    let (started, ready) = mpsc::channel();
    let answer: crate::simple_worker_tests::AnswerHook =
        Arc::new(move |_, _, tools, cancel, emit| {
            let started = started.clone();
            Box::pin(async move {
                emit(brn_ai::AiEvent::Text("Retained partial õ\r\n".into()));
                started.send(()).unwrap();
                cancel.cancelled().await;
                drop(tools);
                brn_ai::AiAnswer {
                    text: "Retained partial õ\r\n".into(),
                    terminal: brn_ai::AiTerminal::Failed(brn_ai::AiError::new(
                        brn_ai::AiErrorKind::Other,
                    )),
                }
            })
        });
    let mut worker = AppWorker::start_owned(
        data,
        config,
        Hooks {
            chat: chat_worker::Hooks {
                answer: Some(answer),
                ..chat_worker::Hooks::default()
            },
            ..Hooks::default()
        },
    )
    .unwrap();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    let id = Uuid::new_v4();
    worker
        .submit(
            id,
            AppCommand::Ask(AskRequest {
                id,
                budget: None,
                conversation: None,
                question: "Synthetic pending question".into(),
                selection: Selection {
                    provider: Provider::Chatgpt,
                    model: "synthetic-model".into(),
                },
                effort: Some(brn_ai::ReasoningEffort::Medium),
                generation: 1,
            }),
        )
        .unwrap();
    ready.recv_timeout(Duration::from_secs(10)).unwrap();
    worker.shutdown().unwrap();
    let mut terminal = None;
    let mut checkpoint = None;
    for (actual, event) in worker.events.try_iter() {
        match event {
            AppEvent::Chat(ChatEvent::Finished { turn, .. }) if actual == id => {
                terminal = Some(turn)
            }
            AppEvent::BackupStatus(status) if actual.is_nil() => checkpoint = Some(status),
            _ => {}
        }
    }
    let turn = terminal.expect("chat must settle before checkpoint");
    assert_eq!(turn.status, brn_store::work::WorkTurnStatus::Interrupted);
    assert_eq!(turn.answer, "Retained partial õ\r\n");
    let checkpoint = checkpoint.unwrap();
    assert!(checkpoint.last_error.is_none() && checkpoint.completed_at_ms.is_some());
    let conn = rusqlite::Connection::open_with_flags(
        checkpoint.latest_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let copied: (String, String) = conn
        .query_row(
            "SELECT text,status FROM messages WHERE turn_id=? AND role='assistant'",
            [id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(copied, (turn.answer, "interrupted".into()));
}
