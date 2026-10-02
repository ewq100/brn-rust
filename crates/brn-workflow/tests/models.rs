use brn_workflow::{
    ErrorKind,
    app::{App, AppConfig},
    models::DownloadDecision,
};

fn base() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}
fn open(
    data: &std::path::Path,
    credentials: &std::path::Path,
    model: Option<std::path::PathBuf>,
) -> brn_workflow::Result<App> {
    App::open(
        data,
        AppConfig {
            vault_root: None,
            credentials_dir: credentials.join("credentials"),
            model_dir: model,
        },
    )
}

#[test]
fn decline_is_durable_and_open_search_never_execute_persisted_consent() {
    let data = base();
    let credentials = base();
    let vault = base();
    std::fs::write(vault.path().join("a.md"), b"apple").unwrap();
    let mut app = open(data.path(), credentials.path(), None).unwrap();
    let prompt = app.model_download_prompt().unwrap().unwrap();
    assert!(prompt.source.contains("Xenova/all-MiniLM-L6-v2"));
    assert_eq!(prompt.bytes, 91_100_408);
    assert_eq!(prompt.destination, data.path().join("models/minilm"));
    assert!(
        app.prepare_model_download(false, &prompt.destination)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        app.model_download_decision().unwrap(),
        Some(DownloadDecision::Declined)
    );
    drop(app);
    let mut app = open(data.path(), credentials.path(), None).unwrap();
    assert!(app.model_download_prompt().unwrap().is_none());
    app.bind_vault(vault.path()).unwrap();
    assert!(
        app.search("apple", brn_workflow::library::SearchMode::Hybrid, 10)
            .unwrap()
            .keyword_only
    );
    assert!(!prompt.destination.exists());
    assert_eq!(std::fs::read(vault.path().join("a.md")).unwrap(), b"apple");
}

#[cfg(not(feature = "native-retrieval"))]
#[test]
fn nonnative_model_dir_download_and_activation_are_typed_refusals() {
    let data = base();
    let credentials = base();
    let target = data.path().join("models/minilm");
    assert_eq!(
        open(data.path(), credentials.path(), Some(target.clone()))
            .err()
            .unwrap()
            .kind,
        ErrorKind::SemanticUnavailableInBuild
    );
    let mut app = open(data.path(), credentials.path(), None).unwrap();
    assert_eq!(
        app.prepare_model_download(true, &target)
            .err()
            .unwrap()
            .kind,
        ErrorKind::SemanticUnavailableInBuild
    );
    assert_eq!(
        app.activate_model(&target).unwrap_err().kind,
        ErrorKind::SemanticUnavailableInBuild
    );
    assert!(!target.exists());
}

#[cfg(feature = "native-retrieval")]
#[test]
fn fresh_explicit_approval_returns_owned_job_but_never_downloads_on_restart() {
    let data = base();
    let credentials = base();
    let mut app = open(data.path(), credentials.path(), None).unwrap();
    let target = data.path().join("models/minilm");
    assert!(
        app.prepare_model_download(false, &target)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        app.model_download_decision().unwrap(),
        Some(DownloadDecision::Declined)
    );
    let request = app.prepare_model_download(true, &target).unwrap().unwrap();
    assert_eq!(request.target(), target);
    assert_eq!(
        app.model_download_decision().unwrap(),
        Some(DownloadDecision::Approved)
    );
    drop(request);
    drop(app);
    let mut app = open(data.path(), credentials.path(), None).unwrap();
    assert!(app.model_download_prompt().unwrap().is_none());
    assert!(!target.exists());
    let retry = app.prepare_model_download(true, &target).unwrap().unwrap();
    assert_eq!(
        retry
            .install(&std::sync::atomic::AtomicBool::new(true), |_, _| {})
            .unwrap_err()
            .kind,
        ErrorKind::Cancelled
    );
    assert!(!target.exists());
}

#[cfg(feature = "native-retrieval")]
#[test]
fn existing_invalid_native_model_is_an_error_not_absence_or_inference() {
    let data = base();
    let credentials = base();
    let target = data.path().join("models/minilm");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("config.json"), b"synthetic incomplete model").unwrap();
    assert_eq!(
        open(data.path(), credentials.path(), Some(target))
            .err()
            .unwrap()
            .kind,
        ErrorKind::ModelInvalid
    );
}

#[test]
fn installer_target_must_not_overlap_vault_or_repository() {
    let data = base();
    let credentials = base();
    let vault = base();
    let mut app = open(data.path(), credentials.path(), None).unwrap();
    app.bind_vault(vault.path()).unwrap();
    assert_eq!(
        app.prepare_model_download(true, &vault.path().join("model"))
            .err()
            .unwrap()
            .kind,
        ErrorKind::ModelInvalid
    );
    let repo = base();
    std::fs::write(repo.path().join(".git"), b"synthetic").unwrap();
    assert_eq!(
        app.prepare_model_download(true, &repo.path().join("model"))
            .err()
            .unwrap()
            .kind,
        ErrorKind::ModelInvalid
    );
    assert!(!vault.path().join("model").exists());
}
