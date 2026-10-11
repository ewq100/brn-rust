use brn_threads_app::*;

fn fixture() -> (tempfile::TempDir, Workspace, String, String) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open(root.path().join("fresh")).unwrap();
    let mut settings = app.settings().unwrap();
    settings.provider = "Offline".into();
    settings.model = "synthetic".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    app.configure(settings).unwrap();
    let thread = app.new_thread("Synthetic steering").unwrap().id;
    let op = app.store.allocate_operation("synthetic worker").unwrap();
    let run = op.creation_id(0);
    app.owner_change(
        "synthetic worker",
        &ChangeRequest {
            reason: "Synthetic worker".into(),
            inputs: vec![],
            writes: vec![Put {
                id: run.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Run(Run {
                    thread: thread.clone(),
                    provider: "Offline".into(),
                    model: "synthetic".into(),
                    guide_identity: "fixture".into(),
                    budget: 4,
                    effort: "medium".into(),
                    loaded_skills: vec![],
                    fence: 1,
                    state: RunState::Working,
                    progress: String::new(),
                    operations: vec![],
                    sources: vec![],
                }),
            }],
        },
    )
    .unwrap();
    (root, app, thread, run)
}

#[test]
fn stable_key_replays_one_durable_message_after_resolution_and_reopen() {
    let (root, mut app, thread, run) = fixture();
    let receipt = app
        .stop_and_post("synthetic send", &run, &thread, "New instruction")
        .unwrap();
    let saved = app.messages(&thread).unwrap();
    assert_eq!(saved.len(), 1);
    assert!(
        matches!(&saved[0].data, RecordData::Message(m) if m.role=="owner" && m.text=="New instruction")
    );
    app.resolve_thread(&thread).unwrap();
    drop(app);
    let mut app = Workspace::open(root.path().join("fresh")).unwrap();
    assert_eq!(
        app.stop_and_post("synthetic send", &run, &thread, "New instruction")
            .unwrap(),
        receipt
    );
    assert_eq!(app.messages(&thread).unwrap(), saved);
    for (run, thread, text) in [
        (&run[..], &thread[..], "Changed"),
        ("other run", &thread[..], "New instruction"),
        (&run[..], "other thread", "New instruction"),
    ] {
        assert!(matches!(
            app.stop_and_post("synthetic send", run, thread, text),
            Err(AppError::Core(Error::ImmutableRequest))
        ));
    }
}

#[test]
fn failed_save_keeps_exact_key_intent_bound_and_has_no_message_or_fence() {
    let (_root, mut app, thread, run) = fixture();
    let before = app.records().unwrap();
    assert!(
        app.stop_and_post("failed send", "missing run", &thread, "Instruction")
            .is_err()
    );
    assert_eq!(app.records().unwrap(), before);
    assert!(app.messages(&thread).unwrap().is_empty());
    assert!(matches!(
        app.stop_and_post("failed send", &run, &thread, "Instruction"),
        Err(AppError::Core(Error::ImmutableRequest))
    ));
    assert!(
        app.stop_and_post("new send", &run, &thread, "Instruction")
            .is_ok()
    );
}
