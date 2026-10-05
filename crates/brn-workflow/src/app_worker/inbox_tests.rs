use super::*;
use crate::inbox::{CaptureInboxRequest, InboxKind, InboxOriginal};
use crate::inbox_processing::{
    InboxCandidateRequest, InboxProcessBatch, InboxProcessOutcome, ProcessInboxRequest,
};

fn processing_fixture() -> (tempfile::TempDir, std::path::PathBuf, AppWorker) {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let worker = AppWorker::start(
        data.clone(),
        AppConfig {
            vault_root: None,
            credentials_dir: Some(owner.path().join("credentials")),
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
    (owner, data, worker)
}
fn processing_capture(worker: &AppWorker, kind: InboxKind, text: &str) -> crate::inbox::InboxItem {
    let request = CaptureInboxRequest {
        id: Uuid::new_v4(),
        kind,
        title: "Processing synthetic text õ".into(),
        original_name: None,
        text: text.into(),
    };
    worker
        .submit(request.id, AppCommand::CaptureInbox(request.clone()))
        .unwrap();
    match worker.recv_event_timeout(Duration::from_secs(10)).unwrap() {
        (id, AppEvent::InboxCaptured(item)) if id == request.id => *item,
        _ => panic!("capture reply"),
    }
}
fn terminal_batch(worker: &AppWorker, id: Uuid) -> InboxProcessBatch {
    loop {
        match worker.recv_event_timeout(Duration::from_secs(10)).unwrap() {
            (reply, AppEvent::InboxProcessing(batch))
                if reply == id && batch.pending_count() == 0 =>
            {
                return *batch;
            }
            (reply, AppEvent::InboxProcessing(_)) if reply == id => {}
            (reply, AppEvent::Failed(error)) if reply == id => {
                panic!("processing failure: {error}")
            }
            _ => {}
        }
    }
}
#[test]
fn storage_failure_during_admitted_step_emits_batch_error_and_retains_restart_work() {
    let (owner, data, mut worker) = processing_fixture();
    let item = processing_capture(&worker, InboxKind::Text, "keep original");
    let request = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![item.clone()],
    };
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
        .submit(request.id, AppCommand::ProcessInbox(request.clone()))
        .unwrap();
    let (entered_tx, entered) = mpsc::channel();
    let (release_next, release_rx) = mpsc::channel();
    worker
        .submit(
            Uuid::new_v4(),
            AppCommand::TestPause {
                entered: entered_tx,
                release: release_rx,
            },
        )
        .unwrap();
    release.send(()).unwrap();
    entered.recv_timeout(Duration::from_secs(10)).unwrap();
    let writer = rusqlite::Connection::open(data.join("brn.sqlite")).unwrap();
    writer.execute_batch("BEGIN IMMEDIATE").unwrap();
    release_next.send(()).unwrap();
    loop {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(20)).unwrap();
        if id == request.id {
            match event {
                AppEvent::Failed(_) => break,
                AppEvent::InboxProcessing(batch) => {
                    assert_eq!(batch.entries[0].outcome, InboxProcessOutcome::Queued)
                }
                _ => panic!("unexpected processing reply"),
            }
        }
    }
    writer.execute_batch("ROLLBACK").unwrap();
    assert!(worker.shutdown().is_err());
    drop(worker);
    let app = App::open(
        &data,
        AppConfig {
            vault_root: None,
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert_eq!(
        app.inbox_processing(request.id).unwrap().entries[0].outcome,
        InboxProcessOutcome::Cancelled
    );
    assert_eq!(
        std::fs::read(item.capture.copy.directory.join(item.capture.copy_name())).unwrap(),
        b"keep original"
    );
}
#[test]
fn owned_batch_converts_exact_text_reports_missing_and_qualifies_restart_preview() {
    let (owner, data, mut worker) = processing_fixture();
    let exact = "\u{feff}---\r\nbrn_kind: current\r\n---\r\nImported 日本語";
    let markdown = processing_capture(&worker, InboxKind::Markdown, exact);
    let text = processing_capture(
        &worker,
        InboxKind::Email,
        "\u{feff}From: x\r\n\r\n````\n~~~\nbody\0",
    );
    let missing = processing_capture(&worker, InboxKind::Teams, "retain catalog");
    std::fs::remove_file(
        missing
            .capture
            .copy
            .directory
            .join(missing.capture.copy_name()),
    )
    .unwrap();
    let request = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![markdown.clone(), text.clone(), missing],
    };
    worker
        .submit(request.id, AppCommand::ProcessInbox(request.clone()))
        .unwrap();
    let batch = terminal_batch(&worker, request.id);
    assert!(matches!(
        batch.entries[0].outcome,
        InboxProcessOutcome::Converted { .. }
    ));
    assert!(matches!(
        batch.entries[1].outcome,
        InboxProcessOutcome::Converted { .. }
    ));
    assert!(
        matches!(&batch.entries[2].outcome, InboxProcessOutcome::Failed { code } if code == "original_missing")
    );
    worker.shutdown().unwrap();
    drop(worker);
    let mut app = App::open(
        &data,
        AppConfig {
            vault_root: None,
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert_eq!(app.process_inbox(&request).unwrap(), batch);
    let preview = app
        .inbox_candidate(&InboxCandidateRequest {
            batch_id: request.id,
            index: 0,
        })
        .unwrap();
    assert_eq!(preview.markdown, exact);
    assert!(preview.needs_semantic_review);
    assert_eq!(preview.original, markdown);
    let preview = app
        .inbox_candidate(&InboxCandidateRequest {
            batch_id: request.id,
            index: 1,
        })
        .unwrap();
    assert!(
        preview
            .markdown
            .contains("\u{feff}From: x\r\n\r\n````\n~~~\nbody\0")
    );
    std::fs::write(
        text.capture.copy.directory.join(text.capture.copy_name()),
        "changed",
    )
    .unwrap();
    assert_eq!(
        app.inbox_candidate(&InboxCandidateRequest {
            batch_id: request.id,
            index: 1
        })
        .unwrap_err()
        .kind,
        ErrorKind::ContextStale
    );
    assert_eq!(app.inbox_processing(request.id).unwrap(), batch);
    assert!(!data.join("index.sqlite").exists());
    assert!(
        std::fs::read_dir(owner.path().join("credentials"))
            .unwrap()
            .next()
            .is_none()
    );
}
#[test]
fn immediate_cancel_behind_blocked_lane_and_joined_quit_settle_admitted_jobs() {
    for explicit_cancel in [true, false] {
        let (owner, data, mut worker) = processing_fixture();
        let item = processing_capture(&worker, InboxKind::Text, "retained\r\nõ");
        let request = ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![item.clone()],
        };
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
            .submit(request.id, AppCommand::ProcessInbox(request.clone()))
            .unwrap();
        if explicit_cancel {
            worker
                .submit(
                    Uuid::new_v4(),
                    AppCommand::CancelInboxProcessing(request.id),
                )
                .unwrap();
            release.send(()).unwrap();
            let batch = terminal_batch(&worker, request.id);
            assert_eq!(batch.entries[0].outcome, InboxProcessOutcome::Cancelled);
            worker.shutdown().unwrap();
        } else {
            let stopping = worker.stopping.clone();
            let join = std::thread::spawn(move || {
                worker.shutdown().unwrap();
                worker
            });
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            while !stopping.load(Ordering::Acquire) {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            release.send(()).unwrap();
            worker = join.join().unwrap();
        }
        drop(worker);
        let app = App::open(
            &data,
            AppConfig {
                vault_root: None,
                credentials_dir: Some(owner.path().join("credentials")),
                model_dir: None,
            },
        )
        .unwrap();
        assert_eq!(
            app.inbox_processing(request.id).unwrap().entries[0].outcome,
            InboxProcessOutcome::Cancelled
        );
        assert_eq!(
            std::fs::read(item.capture.copy.directory.join(item.capture.copy_name())).unwrap(),
            "retained\r\nõ".as_bytes()
        );
    }
}
#[test]
fn shutdown_drains_an_admitted_capture_cancels_a_queued_read_and_refuses_new_work() {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = base.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let config = || AppConfig {
        vault_root: None,
        credentials_dir: Some(base.path().join("credentials")),
        model_dir: None,
    };
    let mut worker = AppWorker::start(data.clone(), config()).unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready { .. }
    ));
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
    let request = CaptureInboxRequest {
        id: Uuid::new_v4(),
        kind: InboxKind::Markdown,
        title: "Admitted exact copy λ".into(),
        original_name: None,
        text: "\u{feff}quoted õ\r\n".into(),
    };
    worker
        .submit(request.id, AppCommand::CaptureInbox(request.clone()))
        .unwrap();
    let read_id = Uuid::new_v4();
    worker
        .submit(read_id, AppCommand::InboxItem(request.id))
        .unwrap();
    let stopping = worker.stopping.clone();
    let join = std::thread::spawn(move || {
        worker.shutdown().unwrap();
        worker
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while !stopping.load(Ordering::Acquire) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(1));
    }
    release.send(()).unwrap();
    let worker = join.join().unwrap();
    let mut item = None;
    let mut cancelled = false;
    while let Some((id, event)) = worker.try_event() {
        if id == request.id {
            match event {
                AppEvent::InboxCaptured(value) => item = Some(*value),
                AppEvent::Failed(e) => panic!("admitted capture failed: {e}"),
                _ => panic!("capture reply"),
            }
        } else if id == read_id {
            assert!(matches!(event,AppEvent::Failed(e) if e.kind==ErrorKind::Cancelled));
            cancelled = true;
        }
    }
    let item = item.expect("admitted capture must settle before shutdown");
    assert!(cancelled);
    assert_eq!(
        worker
            .submit(request.id, AppCommand::CaptureInbox(request.clone()))
            .unwrap_err()
            .kind,
        ErrorKind::Cancelled
    );
    drop(worker);
    let mut app = App::open(&data, config()).unwrap();
    assert_eq!(app.capture_inbox(&request).unwrap(), item);
    assert_eq!(
        app.inbox_item(request.id).unwrap().original,
        InboxOriginal::Available { text: request.text }
    );
}
