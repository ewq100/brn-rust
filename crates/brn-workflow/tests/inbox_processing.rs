#![cfg(target_os = "macos")]
use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    inbox::{CaptureInboxRequest, InboxKind},
    inbox_processing::*,
};
use std::fs;
use uuid::Uuid;
fn config(data: &std::path::Path) -> AppConfig {
    AppConfig {
        vault_root: None,
        credentials_dir: Some(data.parent().unwrap().join("credentials")),
        model_dir: None,
    }
}
#[test]
fn exact_snapshot_admission_and_unapproved_candidates_have_no_vault_or_provider_effects() {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    fs::create_dir(&data).unwrap();
    let mut app = App::open(&data, config(&data)).unwrap();
    let original = CaptureInboxRequest {
        id: Uuid::new_v4(),
        kind: InboxKind::Email,
        title: "Exact text copy".into(),
        original_name: None,
        text: "\u{feff}From: synthetic@example.invalid\r\nSubject: õ\r\n\r\n```\nbody\0".into(),
    };
    let item = app.capture_inbox(&original).unwrap();
    let request = ProcessInboxRequest {
        id: Uuid::new_v4(),
        items: vec![item.clone()],
    };
    let batch = app.process_inbox(&request).unwrap();
    assert_eq!(batch.pending_count(), 1);
    assert!(
        app.inbox_candidate(&InboxCandidateRequest {
            batch_id: request.id,
            index: 0
        })
        .is_err()
    );
    // The public App boundary admits and inspects; its AppWorker owns execution.
    assert!(app.vault_root().is_none());
    assert!(!data.join("index.sqlite").exists());
    assert!(
        fs::read_dir(owner.path().join("credentials"))
            .unwrap()
            .next()
            .is_none()
    );
    assert_eq!(
        fs::read(item.capture.copy.directory.join(item.capture.copy_name())).unwrap(),
        original.text.as_bytes()
    );
    let mut changed = request.clone();
    changed.items[0].capture.title.push('x');
    assert_eq!(
        app.process_inbox(&changed).unwrap_err().kind,
        ErrorKind::OperationConflict
    );
    let cancelled = app.cancel_inbox_processing(request.id).unwrap();
    assert!(matches!(
        cancelled.entries[0].outcome,
        InboxProcessOutcome::Cancelled
    ));
    assert_eq!(app.process_inbox(&request).unwrap(), cancelled);
}
