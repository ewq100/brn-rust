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
            credentials_dir: Some(credentials.join("credentials")),
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
    let target = data.path().join(brn_workflow::models::MODEL_RELATIVE_DIR);
    #[cfg(feature = "native-retrieval")]
    {
        let prompt = app.model_download_prompt().unwrap().unwrap();
        assert_eq!(prompt.source, brn_workflow::models::MODEL_SOURCE);
        assert_eq!(prompt.bytes, 135_392_488);
        assert_eq!(prompt.destination, target);
    }
    #[cfg(not(feature = "native-retrieval"))]
    assert!(app.model_download_prompt().unwrap().is_none());
    assert!(
        app.prepare_model_download(false, &target)
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
    assert!(!target.exists());
    assert_eq!(std::fs::read(vault.path().join("a.md")).unwrap(), b"apple");
}

#[test]
fn legacy_asset_consent_never_authorizes_or_suppresses_the_new_pinned_model() {
    const CURRENT: &str = "model.download_decision.2c4055b12046f11709e9df2c122e59ffbdc2f900";
    for legacy in ["approved", "declined"] {
        let data = base();
        let credentials = base();
        let mut app = open(data.path(), credentials.path(), None).unwrap();
        app.work_store_mut()
            .set_setting("model.download_decision", legacy)
            .unwrap();
        assert_eq!(app.model_download_decision().unwrap(), None);
        #[cfg(feature = "native-retrieval")]
        {
            let prompt = app.model_download_prompt().unwrap().unwrap();
            assert_eq!(prompt.bytes, 135_392_488);
            assert_eq!(
                prompt.destination,
                data.path().join("models/multilingual-minilm-l12-v2")
            );
            assert!(
                prompt
                    .source
                    .contains("2c4055b12046f11709e9df2c122e59ffbdc2f900")
            );
        }
        app.work_store_mut()
            .set_setting(CURRENT, "approved")
            .unwrap();
        assert_eq!(
            app.model_download_decision().unwrap(),
            Some(DownloadDecision::Approved)
        );
        assert_eq!(
            app.work_store()
                .setting("model.download_decision")
                .unwrap()
                .as_deref(),
            Some(legacy)
        );
        drop(app);
        let app = open(data.path(), credentials.path(), None).unwrap();
        assert_eq!(
            app.model_download_decision().unwrap(),
            Some(DownloadDecision::Approved)
        );
        assert!(app.model_download_prompt().unwrap().is_none());
        assert!(!data.path().join("models").exists());
    }
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
    assert_eq!(app.model_download_decision().unwrap(), None);
    assert_eq!(
        app.prepare_model_download(true, &target)
            .err()
            .unwrap()
            .kind,
        ErrorKind::SemanticUnavailableInBuild
    );
    assert_eq!(app.model_download_decision().unwrap(), None);
    assert_eq!(
        app.activate_model(&target).unwrap_err().kind,
        ErrorKind::SemanticUnavailableInBuild
    );
    assert!(!target.exists());
}

#[cfg(not(feature = "native-retrieval"))]
#[test]
fn unsupported_download_preserves_prior_consent_and_never_offers_a_prompt() {
    let data = base();
    let credentials = base();
    let target = data.path().join("models/minilm");
    let mut app = open(data.path(), credentials.path(), None).unwrap();
    assert!(
        app.prepare_model_download(false, &target)
            .unwrap()
            .is_none()
    );
    for (stored, expected) in [
        ("declined", DownloadDecision::Declined),
        ("approved", DownloadDecision::Approved),
    ] {
        app.work_store_mut()
            .set_setting(brn_workflow::models::MODEL_DECISION_KEY, stored)
            .unwrap();
        assert!(app.model_download_prompt().unwrap().is_none());
        assert_eq!(
            app.prepare_model_download(true, &target)
                .err()
                .unwrap()
                .kind,
            ErrorKind::SemanticUnavailableInBuild
        );
        assert_eq!(app.model_download_decision().unwrap(), Some(expected));
        drop(app);
        app = open(data.path(), credentials.path(), None).unwrap();
        assert_eq!(app.model_download_decision().unwrap(), Some(expected));
        assert!(app.model_download_prompt().unwrap().is_none());
        assert!(!target.exists());
    }
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

#[cfg(feature = "native-retrieval")]
#[test]
fn fresh_default_is_separate_from_untouched_legacy_assets_and_invalid_is_not_absence() {
    let data = base();
    let credentials = base();
    let legacy = data.path().join("models/minilm");
    std::fs::create_dir_all(&legacy).unwrap();
    let legacy_asset = legacy.join("config.json");
    std::fs::write(&legacy_asset, b"synthetic incomplete legacy model").unwrap();
    let app = open(data.path(), credentials.path(), None).unwrap();
    assert!(!app.model_installed());
    let prompt = app.model_download_prompt().unwrap().unwrap();
    assert_eq!(
        prompt.destination,
        data.path().join("models/multilingual-minilm-l12-v2")
    );
    assert!(!prompt.destination.exists());
    drop(app);
    std::fs::create_dir_all(&prompt.destination).unwrap();
    std::fs::write(prompt.destination.join("config.json"), b"invalid new model").unwrap();
    assert_eq!(
        open(data.path(), credentials.path(), None)
            .err()
            .unwrap()
            .kind,
        ErrorKind::ModelInvalid
    );
    assert_eq!(
        std::fs::read(legacy_asset).unwrap(),
        b"synthetic incomplete legacy model"
    );
}

#[cfg(feature = "native-retrieval")]
#[test]
fn native_restart_uses_saved_model_unless_configuration_explicitly_overrides_it() {
    let data = base();
    let credentials = base();
    let saved = data.path().join("models/saved-native");
    let mut app = open(data.path(), credentials.path(), None).unwrap();
    std::fs::create_dir_all(&saved).unwrap();
    let asset = saved.join("config.json");
    std::fs::write(&asset, b"synthetic incomplete saved model").unwrap();
    app.work_store_mut()
        .set_setting("model.directory", saved.to_str().unwrap())
        .unwrap();
    drop(app);
    assert_eq!(
        open(data.path(), credentials.path(), None)
            .err()
            .unwrap()
            .kind,
        ErrorKind::ModelInvalid
    );
    let app = open(
        data.path(),
        credentials.path(),
        Some(data.path().join("explicit-missing-model")),
    )
    .unwrap();
    assert!(!app.model_installed());
    assert_eq!(
        app.work_store()
            .setting("model.directory")
            .unwrap()
            .as_deref(),
        saved.to_str()
    );
    assert_eq!(
        std::fs::read(asset).unwrap(),
        b"synthetic incomplete saved model"
    );
}

#[cfg(feature = "native-retrieval")]
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
