use brn_threads_app::*;

fn workspace() -> (tempfile::TempDir, Workspace) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::open(root.path().join("fresh")).unwrap();
    let mut settings = app.settings().unwrap();
    settings.provider = "Offline".into();
    settings.model = "synthetic".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    app.configure(settings).unwrap();
    (root, app)
}

fn working_run(app: &mut Workspace, thread: &str) -> String {
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
                    loaded_skill_details: vec![],
                    instruction: None,
                    queue_claim: None,
                    thread: thread.into(),
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
    run
}

fn run_state(app: &Workspace, run: &str) -> RunState {
    match app.store.record(run).unwrap().unwrap().data {
        RecordData::Run(run) => run.state,
        _ => unreachable!(),
    }
}

#[test]
fn theme_waits_for_working_runs_and_never_stops_them() {
    let (_root, mut app) = workspace();
    let thread = app.new_thread("Synthetic theme witness").unwrap().id;
    let run = working_run(&mut app, &thread);
    let before = app.settings().unwrap();
    assert!(matches!(
        app.choose_theme("raba").unwrap(),
        ApplyOutcome::Deferred { guarded } if guarded == vec![run.clone()]
    ));
    assert_eq!(app.settings().unwrap(), before, "deferral writes nothing");
    assert_eq!(run_state(&app, &run), RunState::Working);

    app.resolve_thread(&thread).unwrap();
    assert_eq!(run_state(&app, &run), RunState::Cancelled);
    let ApplyOutcome::Applied(receipt) = app.choose_theme("raba").unwrap() else {
        panic!("idle workspace saves the theme")
    };
    assert_eq!(receipt.reason, "Owner chose the workspace theme");
    let saved = app.settings().unwrap();
    assert_eq!(saved.theme.as_deref(), Some("raba"));
    let mut unchanged = saved.clone();
    unchanged.theme = None;
    assert_eq!(unchanged, before, "only the theme changed");

    // An ordinary configuration keeps the separately chosen theme.
    let mut configured = app.settings().unwrap();
    configured.theme = None;
    configured.effort = "high".into();
    app.configure(configured).unwrap();
    assert_eq!(app.settings().unwrap().theme.as_deref(), Some("raba"));
    assert!(app.choose_theme("Not A Theme").is_err());
}

#[test]
fn legacy_settings_without_theme_keep_their_serialized_form() {
    let (_root, app) = workspace();
    let settings = app.settings().unwrap();
    assert_eq!(settings.theme, None);
    let json = serde_json::to_string(&settings).unwrap();
    assert!(!json.contains("theme"));
    let parsed: WorkspaceSettings = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, settings);
}

#[test]
fn reopen_records_reason_and_resolution_reason_is_kept() {
    let (_root, mut app) = workspace();
    let thread = app.new_thread("Quick question").unwrap().id;
    let ApplyOutcome::Applied(resolved) = app
        .resolve_thread_because(&thread, "Answered quick question resolved automatically")
        .unwrap()
    else {
        panic!("resolved")
    };
    assert_eq!(
        resolved.reason,
        "Answered quick question resolved automatically"
    );
    let ApplyOutcome::Applied(reopened) = app
        .reopen_thread(&thread, "Owner replied to a resolved thread")
        .unwrap()
    else {
        panic!("reopened")
    };
    assert_eq!(reopened.reason, "Owner replied to a resolved thread");
    assert!(matches!(
        app.store.record(&thread).unwrap().unwrap().data,
        RecordData::Thread(Thread {
            state: ThreadState::Open,
            ..
        })
    ));
    assert!(
        app.reopen_thread(&thread, "again").is_err(),
        "an open thread is not reopened"
    );
}
