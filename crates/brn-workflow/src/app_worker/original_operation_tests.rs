use super::*;
use crate::{
    inbox::{CaptureInboxRequest, InboxKind, InboxOriginal},
    inbox_processing::{InboxCandidateRequest, InboxSourceRequest, ProcessInboxRequest},
};
fn fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    App,
    Uuid,
) {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    let vault = owner.path().join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    let mut app = App::open(
        &data,
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: Some(owner.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    let item = app
        .capture_inbox(&CaptureInboxRequest {
            id: Uuid::new_v4(),
            kind: InboxKind::Email,
            title: "Synthetic owner original".into(),
            original_name: None,
            text: "\u{feff}Complete õ 日本語\r\n".into(),
        })
        .unwrap();
    let batch = app
        .process_inbox(&ProcessInboxRequest {
            id: Uuid::new_v4(),
            items: vec![item.clone()],
        })
        .unwrap();
    app.advance_inbox_processing(batch.request.id, &AtomicBool::new(false))
        .unwrap();
    let draft = app
        .prepare_inbox_source(&InboxSourceRequest {
            candidate: InboxCandidateRequest {
                batch_id: batch.request.id,
                index: 0,
            },
            proposal_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            path: "source.md".into(),
            title: "Source".into(),
        })
        .unwrap();
    let proposal = app.create_proposal(&draft).unwrap();
    app.approve_proposal(&crate::proposal_apply::ApprovalRequest {
        operation_id: Uuid::new_v4(),
        expected: proposal.stamp(),
    })
    .unwrap();
    (owner, data, vault, app, item.capture.id)
}
fn processing_reply(worker: &AppWorker, command: AppCommand) -> AppEvent {
    let id = Uuid::new_v4();
    worker.submit(id, command).unwrap();
    loop {
        let (got, event) = worker
            .recv_event_timeout(Duration::from_secs(10))
            .expect("worker reply");
        if got == id {
            return match event {
                AppEvent::Failed(error) => panic!("worker reply: {error}"),
                event => event,
            };
        }
    }
}

#[test]
fn identified_original_operations_preserve_complete_worker_receipts_and_drain_on_shutdown() {
    use crate::inbox_original_operations::{
        InboxRemovalConfirmation, RemoveInboxOriginalRequest, RestoreInboxOriginalRequest,
    };
    let (owner, data, vault, mut app, item) = fixture();
    let preview = app.preview_inbox_removal(item).unwrap();
    let request = RemoveInboxOriginalRequest {
        operation_id: Uuid::new_v4(),
        item_id: item,
        preview_digest: preview.digest,
        previous_restore: None,
        confirmation: InboxRemovalConfirmation {
            version: 1,
            exact_copy_removal_intended: true,
        },
    };
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
    assert!(matches!(
        &*inspected,
        crate::inbox_original_operations::InboxOriginalOperation::Remove(_)
    ));
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
    assert!(matches!(
        &*inspected,
        crate::inbox_original_operations::InboxOriginalOperation::Restore(_)
    ));
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
