//! Historical discovery through the real owner without conversion or a model.
use super::*;
use crate::{
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    inbox::CaptureInboxRequest,
};
use std::{fs, time::Duration};

fn snapshot(original: crate::inbox::InboxItem, batch: u128, text: &str) -> IntakeSnapshot {
    IntakeSnapshot {
        id: Uuid::new_v4(),
        batch_id: Uuid::from_u128(batch),
        index: 0,
        original,
        extraction: brn_intake::Extraction {
            schema: 1,
            converter: "unavailable-historical-converter/1".into(),
            limits: Default::default(),
            consumed: None,
            original_sha256: digest(text.as_bytes()),
            markdown: text.into(),
            sources: vec![brn_intake::SourceNode {
                id: "root".into(),
                parent: None,
                name: "original.txt".into(),
                media_type: "text/plain".into(),
                locator: "original".into(),
                status: "complete".into(),
                bytes: text.as_bytes().into(),
                text: text.into(),
            }],
            assets: vec![],
            occurrences: vec![],
            gaps: vec![],
        },
    }
}

#[test]
fn saved_versions_survive_restart_and_missing_original_through_exact_worker_request() {
    let owner = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = owner.path().join("data");
    fs::create_dir(&data).unwrap();
    let config = || AppConfig {
        vault_root: None,
        credentials_dir: Some(owner.path().join("credentials")),
        model_dir: None,
    };
    let text = "Retained historical bytes λ\r\n";
    let mut app = App::open(&data, config()).unwrap();
    let mut request = CaptureInboxRequest {
        id: Uuid::new_v4(),
        kind: InboxKind::Text,
        title: "Exact retained import".into(),
        original_name: Some("original.txt".into()),
        text: text.into(),
    };
    let original = app.capture_inbox(&request).unwrap();
    request.id = Uuid::new_v4();
    let duplicate = app.capture_inbox(&request).unwrap();
    let earlier_uuid = snapshot(original.clone(), 10, text);
    let later_uuid = snapshot(original.clone(), 20, text);
    let other_import = snapshot(duplicate.clone(), 5, text);
    // Insertion order deliberately conveys no preferred/current version.
    for saved in [&later_uuid, &other_import, &earlier_uuid] {
        app.store.restore_intake_snapshot(saved).unwrap();
    }
    let expected = vec![earlier_uuid.clone(), later_uuid];
    assert_eq!(
        app.retained_intakes_for_item(original.capture.id).unwrap(),
        expected
    );
    assert_eq!(
        app.retained_intakes_for_item(duplicate.capture.id).unwrap(),
        vec![other_import.clone()]
    );
    drop(app);
    let original_path = original
        .capture
        .copy
        .directory
        .join(original.capture.copy_name());
    fs::remove_file(&original_path).unwrap();
    let app = App::open(&data, config()).unwrap();
    assert_eq!(
        app.retained_intakes_for_item(original.capture.id).unwrap(),
        expected
    );
    assert_eq!(app.retained_intake(earlier_uuid.id).unwrap(), earlier_uuid);
    assert!(!original_path.exists());
    drop(app);

    let mut worker = AppWorker::start(data.clone(), config()).unwrap();
    loop {
        match worker
            .recv_event_timeout(Duration::from_secs(10))
            .unwrap()
            .1
        {
            AppEvent::Ready { .. } => break,
            AppEvent::Restored { .. } => {}
            AppEvent::Failed(error) => panic!("worker startup: {error}"),
            _ => panic!("unexpected startup event"),
        }
    }
    for (item_id, snapshots) in [
        (original.capture.id, expected),
        (duplicate.capture.id, vec![other_import]),
    ] {
        let request_id = Uuid::new_v4();
        worker
            .submit(request_id, AppCommand::InboxRetainedExtractions(item_id))
            .unwrap();
        let (reply_id, event) = worker.recv_event_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(reply_id, request_id);
        let AppEvent::InboxRetainedExtractions {
            item_id: reply_item,
            snapshots: actual,
        } = event
        else {
            panic!("unexpected discovery reply");
        };
        assert_eq!(reply_item, item_id);
        assert_eq!(actual, snapshots);
    }
    assert!(!original_path.exists());
    assert!(!data.join("index.sqlite").exists());
    worker.shutdown().unwrap();
}
