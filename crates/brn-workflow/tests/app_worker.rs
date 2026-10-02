use brn_workflow::{
    ErrorKind,
    app::AppConfig,
    app_worker::{AppCommand, AppEvent, AppWorker},
};
use std::{path::PathBuf, time::Duration};
use uuid::Uuid;

fn fixtures() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn next(worker: &AppWorker) -> (Uuid, AppEvent) {
    worker.recv_event_timeout(Duration::from_secs(10)).unwrap()
}

#[test]
fn first_binding_selection_recovery_and_restart_use_the_application_owner() {
    let base = fixtures();
    let data = base.path().join("data");
    let vault = base.path().join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    std::fs::write(vault.join("a.md"), b"original").unwrap();
    let config = || AppConfig {
        vault_root: None,
        credentials_dir: base.path().join("credentials"),
        model_dir: None,
    };
    let mut worker = AppWorker::start(data.clone(), config()).unwrap();
    assert!(matches!(
        next(&worker).1,
        AppEvent::Ready {
            vault_bound: false,
            ..
        }
    ));
    let bind = Uuid::new_v4();
    worker.submit(bind, AppCommand::BindVault(vault)).unwrap();
    assert!(matches!(next(&worker), (id, AppEvent::VaultBound) if id == bind));
    let recover = Uuid::new_v4();
    worker
        .submit(
            recover,
            AppCommand::RecoverEdit {
                path: "a.md".into(),
                base_sha256: [7; 32],
                text: "unsaved exact\r\n".into(),
            },
        )
        .unwrap();
    assert!(matches!(next(&worker), (id, AppEvent::EditRecovered) if id == recover));
    worker.shutdown().unwrap();
    let (store, _) = brn_store::WorkStore::open(&data).unwrap();
    let edit = store.unsaved_edit("a.md").unwrap().unwrap();
    assert_eq!(edit.text, "unsaved exact\r\n");
    assert_eq!(edit.base_sha256, [7; 32]);
    drop(store);
    let mut worker = AppWorker::start(data, config()).unwrap();
    assert!(matches!(
        next(&worker).1,
        AppEvent::Ready {
            vault_bound: true,
            ..
        }
    ));
    let selection = Uuid::new_v4();
    worker.submit(selection, AppCommand::Selection).unwrap();
    assert!(matches!(next(&worker), (id, AppEvent::Selection(None)) if id == selection));
    worker.shutdown().unwrap();
}

#[test]
fn opening_failure_is_asynchronous_and_typed() {
    let base = fixtures();
    let mut worker = AppWorker::start(
        base.path().join("missing"),
        AppConfig {
            vault_root: None,
            credentials_dir: base.path().join("credentials"),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(next(&worker).1, AppEvent::Failed(_)));
    assert!(worker.shutdown().is_err());
}

#[test]
fn application_status_is_local_and_has_no_implicit_model_or_provider() {
    let base = fixtures();
    let data = base.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let mut worker = AppWorker::start(
        data,
        AppConfig {
            vault_root: None,
            credentials_dir: base.path().join("credentials"),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    let id = Uuid::new_v4();
    worker.submit(id, AppCommand::Status).unwrap();
    assert!(matches!(next(&worker), (actual, AppEvent::Status(status))
        if actual == id && status.vault_root.is_none() && !status.model_installed && status.model_download.is_none()));
    worker.shutdown().unwrap();
}

#[test]
fn unsupported_download_has_no_prompt_or_approval_mutation() {
    if cfg!(feature = "native-retrieval") {
        return;
    }
    let base = fixtures();
    let data = base.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let mut worker = AppWorker::start(
        data.clone(),
        AppConfig {
            vault_root: None,
            credentials_dir: base.path().join("credentials"),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    let id = Uuid::new_v4();
    worker.submit(id, AppCommand::ModelPrompt).unwrap();
    assert!(matches!(next(&worker), (actual, AppEvent::ModelPrompt(None)) if actual == id));
    worker
        .submit(
            Uuid::new_v4(),
            AppCommand::DownloadModel {
                consent: true,
                target: PathBuf::from("/unused/synthetic-model"),
            },
        )
        .unwrap();
    assert!(
        matches!(next(&worker).1, AppEvent::Failed(e) if e.kind == ErrorKind::SemanticUnavailableInBuild)
    );
    worker.shutdown().unwrap();
    let (store, _) = brn_store::WorkStore::open(&data).unwrap();
    assert_eq!(store.setting("model.download_decision").unwrap(), None);
}

#[test]
fn read_selection_refresh_search_and_invalid_recovery_events_echo_the_request() {
    let base = fixtures();
    let data = base.path().join("data");
    let vault = base.path().join("vault");
    std::fs::create_dir(&data).unwrap();
    std::fs::create_dir(&vault).unwrap();
    std::fs::write(vault.join("a.md"), b"\xef\xbb\xbf# Apple\r\nexact\r\n").unwrap();
    let mut worker = AppWorker::start(
        data.clone(),
        AppConfig {
            vault_root: Some(vault.clone()),
            credentials_dir: base.path().join("credentials"),
            model_dir: None,
        },
    )
    .unwrap();
    assert!(matches!(next(&worker).1, AppEvent::Ready { .. }));
    let id = Uuid::new_v4();
    worker.submit(id, AppCommand::Note("a.md".into())).unwrap();
    assert!(
        matches!(next(&worker), (actual, AppEvent::Note(note)) if actual == id && note.text.as_bytes() == b"\xef\xbb\xbf# Apple\r\nexact\r\n")
    );
    std::fs::write(vault.join("b.md"), b"fresh apple").unwrap();
    let id = Uuid::new_v4();
    worker.submit(id, AppCommand::Refresh).unwrap();
    assert!(
        matches!(next(&worker), (actual, AppEvent::Refreshed(report)) if actual == id && report.added == 1)
    );
    let id = Uuid::new_v4();
    worker
        .submit(
            id,
            AppCommand::Notes {
                folder: None,
                cursor: None,
            },
        )
        .unwrap();
    assert!(
        matches!(next(&worker), (actual, AppEvent::Notes(page)) if actual == id && page.notes.len() == 2)
    );
    let id = Uuid::new_v4();
    worker
        .submit(
            id,
            AppCommand::Search {
                query: "apple".into(),
                mode: brn_workflow::library::SearchMode::Keyword,
                limit: 10,
            },
        )
        .unwrap();
    assert!(
        matches!(next(&worker), (actual, AppEvent::Search(result)) if actual == id && result.hits.len() == 2)
    );
    let id = Uuid::new_v4();
    worker
        .submit(
            id,
            AppCommand::Select(brn_workflow::Selection {
                provider: brn_workflow::Provider::Chatgpt,
                model: "gpt-5.5".into(),
            }),
        )
        .unwrap();
    assert!(matches!(next(&worker), (actual, AppEvent::SelectionSaved) if actual == id));
    let id = Uuid::new_v4();
    worker
        .submit(
            id,
            AppCommand::RecoverEdit {
                path: "../escape.md".into(),
                base_sha256: [0; 32],
                text: "refused".into(),
            },
        )
        .unwrap();
    assert!(
        matches!(next(&worker), (actual, AppEvent::Failed(error)) if actual == id && error.kind == ErrorKind::ToolRejected)
    );
    let id = Uuid::new_v4();
    worker.submit(id, AppCommand::Conversations).unwrap();
    assert!(
        matches!(next(&worker), (actual, AppEvent::Conversations(conversations)) if actual == id && conversations.is_empty())
    );
    let id = Uuid::new_v4();
    worker
        .submit(id, AppCommand::Turns(Uuid::new_v4()))
        .unwrap();
    assert!(
        matches!(next(&worker), (actual, AppEvent::Failed(error)) if actual == id && error.kind == ErrorKind::NotFound)
    );
    worker.shutdown().unwrap();
    assert_eq!(
        std::fs::read(vault.join("a.md")).unwrap(),
        b"\xef\xbb\xbf# Apple\r\nexact\r\n"
    );
    let (store, _) = brn_store::WorkStore::open(&data).unwrap();
    assert!(store.unsaved_edit("../escape.md").unwrap().is_none());
}

#[test]
fn restored_backup_event_precedes_ready_on_the_owned_application_lane() {
    let base = fixtures();
    let data = base.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let (store, _) = brn_store::WorkStore::open(&data).unwrap();
    drop(store);
    std::fs::write(data.join("brn.sqlite"), b"synthetic corrupt database").unwrap();
    let mut worker = AppWorker::start(
        data,
        AppConfig {
            vault_root: None,
            credentials_dir: base.path().join("credentials"),
            model_dir: None,
        },
    )
    .unwrap();
    let (startup, restored) = next(&worker);
    assert!(matches!(restored, AppEvent::Restored { backup } if backup.is_file()));
    assert!(matches!(next(&worker), (id, AppEvent::Ready { .. }) if id == startup));
    worker.shutdown().unwrap();
}
