use super::*;
use crate::inbox::{CaptureInboxRequest, InboxKind, InboxOriginal};
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
