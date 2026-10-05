use super::*;
use crate::{
    action_completion::CompleteActionRequest,
    actions::{ActionData, ActionState},
    proposal_apply::ApprovalRequest,
    proposals::{ActionChange, DraftRequest},
};

#[test]
fn shutdown_drains_an_admitted_exact_completion_and_refuses_new_admission() {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = base.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let config = || AppConfig {
        vault_root: None,
        credentials_dir: Some(base.path().join("credentials")),
        model_dir: None,
    };
    let request = {
        let mut app = App::open(&data, config()).unwrap();
        let id = Uuid::new_v4();
        let review = app
            .create_proposal(&DraftRequest {
                inbox_source: None,
                id: Uuid::new_v4(),
                group_id: None,
                session_id: None,
                title: "Approve user action".into(),
                changes: vec![],
                sources: vec![],
                action_changes: vec![ActionChange::Create {
                    id,
                    data: ActionData {
                        title: "Awaiting explicit complete".into(),
                        description: "Exact λ\r\n".into(),
                        state: ActionState::Open,
                        owner: None,
                        related_person: None,
                        related_project: None,
                        sources: vec![],
                        thread: None,
                        due_on: None,
                        follow_up_on: None,
                        dependencies: vec![],
                        parent: None,
                        follows_up: None,
                        priority: None,
                    },
                }],
            })
            .unwrap();
        app.approve_proposal(&ApprovalRequest {
            operation_id: Uuid::new_v4(),
            expected: review.stamp(),
        })
        .unwrap();
        CompleteActionRequest {
            operation_id: Uuid::new_v4(),
            before: Box::new(app.action(id).unwrap()),
        }
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
    let pause = Uuid::new_v4();
    worker
        .submit(
            pause,
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
            AppCommand::CompleteAction(request.clone()),
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
    let mut receipt = None;
    while let Some((id, event)) = worker.try_event() {
        if id == request.operation_id {
            match event {
                AppEvent::ActionCompleted(value) => receipt = Some(*value),
                AppEvent::Failed(error) => {
                    panic!("admitted completion failed during shutdown: {error}")
                }
                _ => panic!("completion reply"),
            }
        }
    }
    let receipt =
        receipt.expect("shutdown must drain and acknowledge the admitted durable command");
    assert_eq!(receipt.request, request);
    assert_eq!(
        worker
            .submit(
                request.operation_id,
                AppCommand::CompleteAction(request.clone())
            )
            .unwrap_err()
            .kind,
        ErrorKind::Cancelled
    );
    drop(worker);
    let mut app = App::open(&data, config()).unwrap();
    assert_eq!(app.action(request.before.origin.id).unwrap(), receipt.after);
    assert_eq!(app.complete_action(&request).unwrap(), receipt);
}
