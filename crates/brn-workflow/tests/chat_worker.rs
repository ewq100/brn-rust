use brn_store::work::WorkTurnStatus;
use brn_workflow::{
    ErrorKind, Provider, Selection,
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
    chat_worker::{AskRequest, ChatEvent},
};
use std::time::Duration;
use uuid::Uuid;

fn next(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}

#[test]
fn terminal_replay_precedes_unavailable_vault_and_current_selection_validation() {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = base.path().join("data");
    let vault = base.path().join("vanished");
    std::fs::create_dir(&data).unwrap();
    let (mut store, _) = brn_store::WorkStore::open(&data).unwrap();
    store
        .set_setting("vault.root", vault.to_str().unwrap())
        .unwrap();
    let id = Uuid::new_v4();
    let turn = store
        .begin_turn(id, None, "earlier", "copilot", "discovered-then")
        .unwrap();
    store
        .finish_turn(id, WorkTurnStatus::Failed, "partial", Some("network"))
        .unwrap();
    drop(store);
    let mut worker = AppWorker::start(
        data,
        AppConfig {
            vault_root: None,
            credentials_dir: Some(base.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    let request = AskRequest {
        id,
        conversation: Some(turn.conversation_id),
        question: "earlier".into(),
        selection: Selection {
            provider: Provider::Copilot,
            model: "discovered-then".into(),
        },
        generation: 73,
    };
    worker.submit(id, AppCommand::Ask(request.clone())).unwrap();
    assert!(
        matches!(next(&worker), (actual, AppEvent::Chat(ChatEvent::Finished { generation: 73, turn, .. }))
        if actual == id && turn.answer == "partial" && turn.status == WorkTurnStatus::Failed)
    );
    let mut conflict = request;
    conflict.question = "different".into();
    worker.submit(id, AppCommand::Ask(conflict)).unwrap();
    assert!(
        matches!(next(&worker).1, AppEvent::Chat(ChatEvent::Rejected { error, .. })
        if error.kind == ErrorKind::OperationConflict)
    );
    worker.shutdown().unwrap();
}

#[test]
fn unbound_new_ask_is_refused_without_insertion_and_outer_uuid_must_match() {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = base.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let mut worker = AppWorker::start(
        data.clone(),
        AppConfig {
            vault_root: None,
            credentials_dir: Some(base.path().join("credentials")),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    let id = Uuid::new_v4();
    let request = AskRequest {
        id,
        conversation: None,
        question: "new".into(),
        selection: Selection {
            provider: Provider::Chatgpt,
            model: "gpt-5.5".into(),
        },
        generation: 1,
    };
    assert!(
        worker
            .submit(Uuid::new_v4(), AppCommand::Ask(request.clone()))
            .is_err()
    );
    worker.submit(id, AppCommand::Ask(request)).unwrap();
    assert!(
        matches!(next(&worker).1, AppEvent::Chat(ChatEvent::Rejected { error, .. })
        if error.kind == ErrorKind::VaultNotBound)
    );
    worker.shutdown().unwrap();
    let (store, _) = brn_store::WorkStore::open(&data).unwrap();
    assert!(store.conversations().unwrap().is_empty());
}

#[test]
fn unknown_conversation_fails_before_insertion_even_without_a_vault_or_account() {
    let base = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = base.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let mut worker = AppWorker::start(
        data.clone(),
        AppConfig {
            vault_root: None,
            credentials_dir: Some(base.path().join("credentials")),
            model_dir: None,
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
                conversation: Some(Uuid::new_v4()),
                question: "unknown conversation".into(),
                selection: Selection {
                    provider: Provider::Chatgpt,
                    model: "gpt-5.5".into(),
                },
                generation: 8,
            }),
        )
        .unwrap();
    assert!(
        matches!(next(&worker).1, AppEvent::Chat(ChatEvent::Rejected { generation: 8, error, .. }) if error.kind == ErrorKind::NotFound)
    );
    worker.shutdown().unwrap();
    let (store, _) = brn_store::WorkStore::open(&data).unwrap();
    assert!(store.conversations().unwrap().is_empty());
}
