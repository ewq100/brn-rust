use brn_threads_app::*;

#[test]
fn fresh_workspace_starts_pending_and_resume_requires_a_choice() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open(root.path().join("fresh")).unwrap();
    assert!(!app.settings().unwrap().maintenance);
    assert!(!app.settings().unwrap().review_first);
    assert!(!app.settings().unwrap().delegation_chosen);
    let error = app.pause(false).unwrap_err().to_string();
    assert!(error.contains("Choose"), "{error}");
}

#[test]
fn configure_cannot_choose_and_manual_work_remains_available_while_pending() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open(root.path().join("fresh")).unwrap();
    let mut settings = app.settings().unwrap();
    settings.delegation_chosen = true;
    settings.maintenance = true;
    settings.review_first = true;
    settings.provider = "Offline".into();
    settings.model = "synthetic".into();
    settings.auth_file = None;
    app.configure(settings).unwrap();
    let current = app.settings().unwrap();
    assert_eq!(current.provider, "Offline");
    assert!(!current.delegation_chosen && !current.maintenance && !current.review_first);
    let note = app
        .new_note("Owner note", "Synthetic local writing")
        .unwrap();
    assert_eq!(app.search("Synthetic").unwrap()[0].note, note.id);
    assert!(app.pause(false).unwrap_err().to_string().contains("Choose"));
    app.pause(true).unwrap();
    assert!(!app.settings().unwrap().delegation_chosen);
}

#[test]
fn both_choices_persist_and_pause_resume_preserve_selected_mode() {
    for choice in [
        DelegationChoice::MaintainOrdinaryNotes,
        DelegationChoice::ReviewFirst,
    ] {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("fresh");
        let mut app = Workspace::open(&data).unwrap();
        assert!(matches!(
            app.choose_delegation(choice).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        app.pause(true).unwrap();
        drop(app);
        let mut app = Workspace::open(&data).unwrap();
        let settings = app.settings().unwrap();
        assert!(settings.delegation_chosen);
        assert!(!settings.maintenance);
        assert_eq!(
            settings.review_first,
            choice == DelegationChoice::ReviewFirst
        );
        app.pause(false).unwrap();
        assert!(app.settings().unwrap().maintenance);
        assert_eq!(
            app.settings().unwrap().review_first,
            choice == DelegationChoice::ReviewFirst
        );
    }
}
