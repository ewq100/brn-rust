use brn_workflow::{
    ErrorKind, ModelOption, Provider, Selection,
    app::{App, AppConfig},
    library::SearchMode,
};

fn base() -> tempfile::TempDir {
    tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()
}

fn config(base: &std::path::Path) -> AppConfig {
    AppConfig {
        vault_root: None,
        credentials_dir: Some(base.join("credentials")),
        model_dir: None,
    }
}

#[test]
fn new_app_refuses_legacy_data_without_touching_it() {
    let data = base();
    let creds = base();
    let legacy = data.path().join("brn.sqlite3");
    std::fs::write(&legacy, b"synthetic legacy sentinel").unwrap();
    assert_eq!(
        App::open(data.path(), config(creds.path()))
            .err()
            .unwrap()
            .kind,
        ErrorKind::WorkspaceModeConflict
    );
    assert_eq!(std::fs::read(legacy).unwrap(), b"synthetic legacy sentinel");
    assert!(!data.path().join("brn.sqlite").exists());
}

#[test]
fn unbound_history_settings_and_first_successful_binding_survive_restart() {
    let data = base();
    let creds = base();
    let vault = base();
    let other = base();
    let mut app = App::open(data.path(), config(creds.path())).unwrap();
    assert!(app.conversations().unwrap().is_empty());
    assert_eq!(app.selection().unwrap(), None);
    assert_eq!(app.note("a.md").unwrap_err().kind, ErrorKind::VaultNotBound);
    assert_eq!(app.tools().err().unwrap().kind, ErrorKind::VaultNotBound);
    assert!(app.bind_vault(&vault.path().join("missing")).is_err());
    std::fs::write(
        vault.path().join("a.md"),
        b"\xef\xbb\xbf# Apple\r\nexact\r\n",
    )
    .unwrap();
    app.bind_vault(vault.path()).unwrap();
    assert!(app.bind_vault(other.path()).is_err());
    assert_eq!(app.notes(None, None).unwrap().notes[0].title, "Apple");
    std::fs::write(vault.path().join("b.md"), b"fresh apple").unwrap();
    assert_eq!(
        app.search("apple", SearchMode::Hybrid, 10)
            .unwrap()
            .hits
            .len(),
        2
    );
    app.select(Selection {
        provider: Provider::Chatgpt,
        model: "gpt-5.5".into(),
    })
    .unwrap();
    let saved = app.selection().unwrap();
    assert!(
        app.select(Selection {
            provider: Provider::Chatgpt,
            model: "bad".into()
        })
        .is_err()
    );
    assert_eq!(app.selection().unwrap(), saved);
    drop(app);
    let header = std::fs::read(data.path().join("index.sqlite")).unwrap();
    assert_eq!(
        i32::from_be_bytes(header[68..72].try_into().unwrap()),
        0x4252_4e49,
        "dropping App must close readers before the writer checkpoints its BRNI header"
    );
    let app = App::open(data.path(), config(creds.path())).unwrap();
    assert_eq!(app.selection().unwrap(), saved);
    assert_eq!(
        app.note("a.md").unwrap().text.as_bytes(),
        b"\xef\xbb\xbf# Apple\r\nexact\r\n"
    );
    drop(app);
    std::fs::remove_file(vault.path().join("a.md")).unwrap();
    std::fs::remove_file(vault.path().join("b.md")).unwrap();
    std::fs::remove_dir(vault.path()).unwrap();
    let app = App::open(data.path(), config(creds.path())).unwrap();
    assert_eq!(
        app.note("a.md").unwrap_err().kind,
        ErrorKind::VaultUnavailable
    );
    assert!(app.selection().unwrap().is_some());
    assert!(app.conversations().unwrap().is_empty());
}

#[test]
fn copilot_selection_requires_explicit_discovery_membership_without_network() {
    let data = base();
    let creds = base();
    let mut app = App::open(data.path(), config(creds.path())).unwrap();
    let selection = Selection {
        provider: Provider::Copilot,
        model: "synthetic-model".into(),
    };
    assert_eq!(
        app.select(selection.clone()).unwrap_err().kind,
        ErrorKind::ModelRefused
    );
    app.record_models(
        Provider::Copilot,
        &[ModelOption {
            id: selection.model.clone(),
            live_qualified: false,
        }],
    )
    .unwrap();
    app.select(selection.clone()).unwrap();
    assert_eq!(app.selection().unwrap(), Some(selection));
}

#[test]
fn credentials_inside_vault_or_repository_are_refused_before_cache_access() {
    let data = base();
    let vault = base();
    let mut c = config(vault.path());
    c.vault_root = Some(vault.path().to_owned());
    assert_eq!(
        App::open(data.path(), c).err().unwrap().kind,
        ErrorKind::UnsafeCredentials
    );
    let repo = base();
    std::fs::write(repo.path().join(".git"), b"synthetic").unwrap();
    assert_eq!(
        App::open(data.path(), config(repo.path()))
            .err()
            .unwrap()
            .kind,
        ErrorKind::UnsafeCredentials
    );
    assert!(!repo.path().join("credentials").exists());
}

#[test]
fn missing_initial_vault_does_not_bind_or_disable_history_and_settings() {
    let data = base();
    let creds = base();
    let vault = base();
    let mut c = config(creds.path());
    c.vault_root = Some(vault.path().join("missing"));
    let mut app = App::open(data.path(), c).unwrap();
    assert_eq!(app.vault_root(), None);
    assert_eq!(app.note("a.md").unwrap_err().kind, ErrorKind::VaultNotBound);
    assert!(app.conversations().unwrap().is_empty());
    app.select(Selection {
        provider: Provider::Chatgpt,
        model: "gpt-5.5".into(),
    })
    .unwrap();
    assert!(app.selection().unwrap().is_some());
    app.bind_vault(vault.path()).unwrap();
}

#[test]
fn data_cannot_overlap_vault_and_a_missing_different_root_cannot_rebind() {
    let data = base();
    let creds = base();
    let mut app = App::open(data.path(), config(creds.path())).unwrap();
    assert!(app.bind_vault(data.path()).is_err());
    let vault = base();
    app.bind_vault(vault.path()).unwrap();
    drop(app);
    let mut c = config(creds.path());
    c.vault_root = Some(vault.path().join("different-missing"));
    assert_eq!(
        App::open(data.path(), c).err().unwrap().kind,
        ErrorKind::WorkspaceModeConflict
    );
}

#[test]
fn bind_is_persisted_only_after_library_initializes_and_auth_open_never_reads_caches() {
    let data = base();
    let creds = base();
    let vault = base();
    std::fs::write(data.path().join("index.sqlite"), b"foreign sentinel").unwrap();
    let mut app = App::open(data.path(), config(creds.path())).unwrap();
    // Deliberately malformed, synthetic caches: startup/selection/read must not inspect them.
    std::fs::write(
        creds.path().join("credentials/chatgpt.json"),
        b"not credentials",
    )
    .unwrap();
    std::fs::write(
        creds.path().join("credentials/copilot.json"),
        b"not credentials",
    )
    .unwrap();
    assert!(app.bind_vault(vault.path()).is_err());
    assert_eq!(app.work_store().setting("vault.root").unwrap(), None);
    assert_eq!(app.vault_root(), None);
    assert_eq!(
        std::fs::read(data.path().join("index.sqlite")).unwrap(),
        b"foreign sentinel"
    );
    app.select(Selection {
        provider: Provider::Chatgpt,
        model: "gpt-5.5".into(),
    })
    .unwrap();
    drop(app);
    assert!(
        App::open(data.path(), config(creds.path()))
            .unwrap()
            .selection()
            .unwrap()
            .is_some()
    );
    assert_eq!(
        std::fs::read(creds.path().join("credentials/chatgpt.json")).unwrap(),
        b"not credentials"
    );
}

#[test]
fn restoration_report_and_local_history_are_available_without_vault() {
    use brn_store::work::WorkTurnStatus;
    let data = base();
    let creds = base();
    let mut app = App::open(data.path(), config(creds.path())).unwrap();
    let turn = app
        .work_store_mut()
        .begin_turn(
            uuid::Uuid::new_v4(),
            None,
            "question\r\n",
            "chatgpt",
            "gpt-5.5",
        )
        .unwrap();
    app.work_store_mut()
        .finish_turn(turn.id, WorkTurnStatus::Interrupted, "partial\r\n", None)
        .unwrap();
    drop(app);
    // A new open backs up the durable history before a later synthetic corruption.
    drop(App::open(data.path(), config(creds.path())).unwrap());
    std::fs::write(data.path().join("brn.sqlite"), b"synthetic corruption").unwrap();
    let app = App::open(data.path(), config(creds.path())).unwrap();
    assert!(app.open_report().restored_from.is_some());
    assert!(app.open_report().corrupt_moved_to.is_some());
    assert_eq!(app.conversations().unwrap()[0].id, turn.conversation_id);
    assert_eq!(
        app.turns(turn.conversation_id).unwrap()[0].answer,
        "partial\r\n"
    );
}

#[test]
fn default_credential_sibling_is_checked_without_creating_or_reading_caches() {
    let fixture = base();
    let data = fixture.path().join("data");
    std::fs::create_dir(&data).unwrap();
    let credentials = brn_workflow::app::default_credentials_dir(&data).unwrap();
    assert_eq!(credentials, fixture.path().join("data.credentials"));
    assert!(!credentials.exists());
    let desktop_data = fixture.path().join("BRN-simple");
    std::fs::create_dir(&desktop_data).unwrap();
    assert_eq!(
        brn_workflow::app::default_credentials_dir(&desktop_data).unwrap(),
        fixture.path().join("BRN-simple.credentials")
    );
    std::fs::write(fixture.path().join(".git"), b"synthetic repository").unwrap();
    assert_eq!(
        brn_workflow::app::default_credentials_dir(&data)
            .unwrap_err()
            .kind,
        ErrorKind::UnsafeCredentials
    );
    assert!(!credentials.exists());
}

#[cfg(not(feature = "native-retrieval"))]
#[test]
fn default_restart_ignores_saved_native_model_without_changing_assets_or_user_work() {
    use brn_store::work::WorkTurnStatus;
    let data = base();
    let creds = base();
    let vault = base();
    let model = data.path().join("models/saved-native");
    let note = b"# Apple\r\nexact vault bytes\r\n";
    std::fs::write(vault.path().join("a.md"), note).unwrap();
    std::fs::create_dir_all(&model).unwrap();
    let asset = model.join("config.json");
    std::fs::write(&asset, b"synthetic native asset sentinel").unwrap();
    let mut app = App::open(data.path(), config(creds.path())).unwrap();
    app.bind_vault(vault.path()).unwrap();
    app.work_store_mut()
        .set_setting("model.directory", model.to_str().unwrap())
        .unwrap();
    app.work_store_mut().set_setting("theme", "dark").unwrap();
    let turn = app
        .work_store_mut()
        .begin_turn(uuid::Uuid::new_v4(), None, "question", "chatgpt", "gpt-5.5")
        .unwrap();
    app.work_store_mut()
        .finish_turn(turn.id, WorkTurnStatus::Completed, "saved answer", None)
        .unwrap();
    drop(app);

    let mut app = App::open(data.path(), config(creds.path())).unwrap();
    assert!(!app.model_installed());
    assert_eq!(
        app.work_store()
            .setting("model.directory")
            .unwrap()
            .as_deref(),
        model.to_str()
    );
    assert_eq!(
        app.work_store().setting("theme").unwrap().as_deref(),
        Some("dark")
    );
    assert_eq!(app.conversations().unwrap()[0].id, turn.conversation_id);
    assert_eq!(
        app.turns(turn.conversation_id).unwrap()[0].answer,
        "saved answer"
    );
    assert_eq!(app.notes(None, None).unwrap().notes[0].path, "a.md");
    assert_eq!(app.note("a.md").unwrap().text.as_bytes(), note);
    for mode in [
        SearchMode::Keyword,
        SearchMode::Semantic,
        SearchMode::Hybrid,
    ] {
        let results = app.search("apple", mode, 10).unwrap();
        assert!(results.keyword_only);
        assert_eq!(results.hits.len(), 1);
    }
    assert!(
        brn_workflow::ReadTools::search_notes(&*app.tools().unwrap(), "apple", 10)
            .unwrap()
            .keyword_only
    );
    assert_eq!(
        std::fs::read(&asset).unwrap(),
        b"synthetic native asset sentinel"
    );
    assert_eq!(std::fs::read_dir(&model).unwrap().count(), 1);
    assert_eq!(std::fs::read(vault.path().join("a.md")).unwrap(), note);
    drop(app);
    let mut explicit = config(creds.path());
    explicit.model_dir = Some(model);
    assert_eq!(
        App::open(data.path(), explicit).err().unwrap().kind,
        ErrorKind::SemanticUnavailableInBuild
    );
}
