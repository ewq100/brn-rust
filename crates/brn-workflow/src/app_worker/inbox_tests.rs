use super::*;
use crate::inbox::{CaptureInboxRequest, InboxKind, InboxOriginal};
use crate::inbox_processing::{
    InboxCandidateRequest, InboxProcessBatch, InboxProcessOutcome, InboxSourceRequest,
    ProcessInboxRequest,
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
fn processing_reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let operation = Uuid::new_v4();
    worker.submit(operation, command).unwrap();
    loop {
        let (id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        if id == operation {
            return event;
        }
    }
}
#[test]
fn prepared_inbox_source_uses_exact_proposal_approval_and_source_scope() {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    let credentials = owner.path().join("credentials");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    let mut worker = AppWorker::start(
        data,
        AppConfig {
            vault_root: Some(vault.clone()),
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
        AppEvent::Ready { .. }
    ));
    let imported_id = Uuid::new_v4();
    let exact = format!(
        "\u{feff}---\r\nbrn_id: {imported_id}\r\nbrn_kind: knowledge\r\n---\r\nOriginal õ 日本語\r\n"
    );
    let original = processing_capture(&worker, InboxKind::Markdown, &exact);
    let process = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![original.clone()],
    };
    worker
        .submit(process.id, AppCommand::ProcessInbox(process.clone()))
        .unwrap();
    let batch = terminal_batch(&worker, process.id);
    assert!(matches!(
        batch.entries[0].outcome,
        InboxProcessOutcome::Converted { .. }
    ));
    let request = InboxSourceRequest {
        candidate: InboxCandidateRequest {
            batch_id: process.id,
            index: 0,
        },
        proposal_id: Uuid::new_v4(),
        note_id: Uuid::new_v4(),
        path: "source.md".into(),
        title: "Review exact original".into(),
    };
    let AppEvent::InboxSourceDraft(draft) =
        processing_reply(&worker, AppCommand::PrepareInboxSource(request.clone()))
    else {
        panic!("source draft response");
    };
    request.validate_draft(&draft).unwrap();
    let [crate::proposals::DraftNoteChange::Create { text, .. }] = draft.changes.as_slice() else {
        panic!("source Create");
    };
    let approved_text = text.clone();
    assert!(approved_text.ends_with(&exact));
    assert_eq!(
        brn_store::note_identity::read(&approved_text).unwrap(),
        Some(request.note_id)
    );
    assert!(!vault.join(&request.path).exists());
    assert!(
        matches!(processing_reply(&worker, AppCommand::Proposals(None)), AppEvent::Proposals(records) if records.is_empty())
    );
    let AppEvent::Proposal(proposal) =
        processing_reply(&worker, AppCommand::CreateProposal(*draft))
    else {
        panic!("source proposal response");
    };
    assert!(!vault.join(&request.path).exists());
    let approval = crate::proposal_apply::ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    };
    assert!(
        matches!(processing_reply(&worker, AppCommand::ApproveProposal(approval)), AppEvent::ProposalApplied(receipt) if receipt.outcome == crate::proposal_apply::ApplyOutcome::Applied)
    );
    assert_eq!(
        std::fs::read(vault.join(&request.path)).unwrap(),
        approved_text.as_bytes()
    );
    assert!(
        matches!(processing_reply(&worker, AppCommand::Note(request.path.clone())), AppEvent::Failed(error) if error.kind == ErrorKind::ToolRejected)
    );
    assert!(matches!(processing_reply(&worker, AppCommand::ScopedNote {
        scope: crate::library::KnowledgeScope::Source,
        path: request.path.clone(),
    }), AppEvent::Note(note) if note.text == approved_text));
    let AppEvent::InboxItem(read) =
        processing_reply(&worker, AppCommand::InboxItem(original.capture.id))
    else {
        panic!("retained original response");
    };
    assert_eq!(read.item, original);
    assert_eq!(
        read.original,
        InboxOriginal::Available {
            text: exact.clone()
        }
    );
    assert_eq!(
        std::fs::read(
            original
                .capture
                .copy
                .directory
                .join(original.capture.copy_name())
        )
        .unwrap(),
        exact.as_bytes()
    );
    let AppEvent::InboxReview(review) =
        processing_reply(&worker, AppCommand::InboxReview(original.capture.id))
    else {
        panic!("approved Source review");
    };
    assert_eq!(review.manifest.proposals.len(), 1);
    assert_eq!(
        review.manifest.proposals[0].record.state,
        crate::proposals::ProposalState::Applied
    );
    assert_eq!(review.manifest.approvals.len(), 1);
    assert_eq!(
        review.manifest.approvals[0]
            .approved
            .draft
            .inbox_source
            .as_ref()
            .unwrap()
            .original,
        original
    );
    assert!(review.needs_semantic_review);
    assert_eq!(std::fs::read_dir(credentials).unwrap().count(), 0);
    worker.shutdown().unwrap();
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

#[test]
fn complete_original_review_uses_worker_and_reports_changed_copy_without_effects() {
    let (_owner, data, mut worker) = processing_fixture();
    let exact = "\u{feff}Original õ\r\n日本語\r\n";
    let original = processing_capture(&worker, InboxKind::Email, exact);
    let AppEvent::InboxReview(first) =
        processing_reply(&worker, AppCommand::InboxReview(original.capture.id))
    else {
        panic!("complete review reply");
    };
    assert_eq!(first.manifest.original, original);
    assert_eq!(
        first.original,
        InboxOriginal::Available { text: exact.into() }
    );
    assert!(first.needs_semantic_review);
    assert_eq!(first.digest, first.manifest.digest().unwrap());
    let process = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![original.clone()],
    };
    worker
        .submit(process.id, AppCommand::ProcessInbox(process.clone()))
        .unwrap();
    terminal_batch(&worker, process.id);
    let AppEvent::InboxReview(processed) =
        processing_reply(&worker, AppCommand::InboxReview(original.capture.id))
    else {
        panic!("processed review");
    };
    assert_eq!(processed.manifest.processing.len(), 1);
    assert_ne!(processed.digest, first.digest);
    assert!(processed.needs_semantic_review);
    let path = data.join("inbox").join(original.capture.copy_name());
    let changed = b"changed original stays retained";
    std::fs::write(&path, changed).unwrap();
    let AppEvent::InboxReview(stale) =
        processing_reply(&worker, AppCommand::InboxReview(original.capture.id))
    else {
        panic!("stale review");
    };
    assert!(matches!(stale.original, InboxOriginal::Changed { .. }));
    assert_eq!(stale.digest, processed.digest);
    assert_eq!(std::fs::read(path).unwrap(), changed);
    worker.shutdown().unwrap();
}

#[test]
fn identified_original_operations_preserve_complete_worker_receipts_and_drain_on_shutdown() {
    use crate::inbox_original_operations::{
        InboxRemovalAttestation, RemoveInboxOriginalRequest, RestoreInboxOriginalRequest,
    };
    let mut fixture = crate::inbox_removal::tests::Fixture::new();
    fixture.source();
    let preview = fixture.app.preview_inbox_removal(fixture.item).unwrap();
    let request = RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id: fixture.item,
        preview_digest: preview.digest,
        previous_restore: None,
        attestation: InboxRemovalAttestation {
            version: 1,
            copy_disposable: true,
            meaningful_content_preserved: true,
            consequences_reviewed: true,
            conflicts_acknowledged: true,
            exact_copy_removal_intended: true,
        },
    };
    let crate::inbox_removal::tests::Fixture {
        _owner: owner,
        data,
        vault,
        app,
        item,
    } = fixture;
    drop(app);
    let config = || AppConfig {
        vault_root: Some(vault.clone()),
        credentials_dir: Some(owner.path().join("credentials")),
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
    assert_eq!(
        worker
            .submit(
                Uuid::new_v4(),
                AppCommand::RemoveInboxOriginal(request.clone())
            )
            .unwrap_err()
            .kind,
        ErrorKind::OperationConflict
    );
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
            request.operation_id,
            AppCommand::RemoveInboxOriginal(request.clone()),
        )
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
    let record = std::iter::from_fn(|| worker.try_event())
        .find_map(|(id, event)| {
            if id != request.operation_id {
                return None;
            }
            match event {
                AppEvent::InboxOriginalRemoved(record) => Some(*record),
                AppEvent::Failed(error) => panic!("admitted removal: {error}"),
                _ => panic!("removal event"),
            }
        })
        .expect("admitted critical removal settles at shutdown");
    record.validate().unwrap();
    assert_eq!(record.request, request);
    assert!(record.removed_at_ms.is_some());
    drop(worker);
    let mut worker = AppWorker::start(data.clone(), config()).unwrap();
    assert!(matches!(
        worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1,
        AppEvent::Ready { .. }
    ));
    let AppEvent::InboxOriginalRemoval {
        operation_id,
        record: Some(inspected),
    } = processing_reply(
        &worker,
        AppCommand::InboxOriginalRemoval(request.operation_id),
    )
    else {
        panic!("retained removal event");
    };
    assert_eq!(operation_id, request.operation_id);
    assert_eq!(inspected.digest().unwrap(), record.digest().unwrap());
    let restore = RestoreInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        removal_operation_id: request.operation_id,
        removal_digest: record.digest().unwrap(),
    };
    assert_eq!(
        worker
            .submit(
                Uuid::new_v4(),
                AppCommand::RestoreInboxOriginal(restore.clone())
            )
            .unwrap_err()
            .kind,
        ErrorKind::OperationConflict
    );
    worker
        .submit(
            restore.operation_id,
            AppCommand::RestoreInboxOriginal(restore.clone()),
        )
        .unwrap();
    let (id, AppEvent::InboxOriginalRestored(restored)) =
        worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
    else {
        panic!("restoration event");
    };
    assert_eq!(id, restore.operation_id);
    restored.validate().unwrap();
    assert_eq!(restored.request, restore);
    let AppEvent::InboxOriginalRestore {
        operation_id,
        record: Some(inspected),
    } = processing_reply(
        &worker,
        AppCommand::InboxOriginalRestore(restore.operation_id),
    )
    else {
        panic!("retained restore event");
    };
    assert_eq!(operation_id, restore.operation_id);
    assert_eq!(inspected.digest().unwrap(), restored.digest().unwrap());
    let AppEvent::InboxOriginalOperations {
        item_id,
        operations,
    } = processing_reply(&worker, AppCommand::InboxOriginalOperations(item))
    else {
        panic!("complete history event");
    };
    assert_eq!(item_id, item);
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].record_sha256, record.digest().unwrap());
    assert_eq!(operations[1].record_sha256, restored.digest().unwrap());
    let absent = Uuid::new_v4();
    assert!(
        matches!(processing_reply(&worker, AppCommand::ArchivedInboxAnalysis(absent)), AppEvent::ArchivedInboxAnalysis { operation_id, analysis: None } if operation_id == absent)
    );
    worker.shutdown().unwrap();
    let app = App::open(&data, config()).unwrap();
    assert!(matches!(
        app.inbox_item(item).unwrap().original,
        InboxOriginal::Available { .. }
    ));
    assert_eq!(
        std::fs::read_dir(owner.path().join("credentials"))
            .unwrap()
            .count(),
        0
    );
}
