use brn_threads_app::*;
use serde_json::json;
fn policy(access: Vec<McpAccess>) -> McpPolicy {
    McpPolicy {
        endpoint: "http://127.0.0.1:8077/mcp".into(),
        server_digest: "a".repeat(64),
        enabled: true,
        tools: vec![McpApprovedTool {
            name: "synthetic_read".into(),
            metadata_digest: "b".repeat(64),
            schema_digest: mcp_input_schema_digest(McpInputContract::Query),
            contract: McpInputContract::Query,
        }],
        access,
    }
}
#[test]
fn scoped_status_reopen_backup_and_disable_never_reconstitute_transient_authority() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut app =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    let first = app.new_thread("public context").unwrap();
    let second = app.new_thread("other context").unwrap();
    let note = app.new_note("Canonical positive", "Retained work").unwrap();
    assert!(!app.mcp_status(Some(&first.id)).unwrap().configured);
    let p = policy(vec![McpAccess::Chat(first.id.clone())]);
    app.configure_mcp(Some(p.clone())).unwrap();
    assert_eq!(app.mcp_status(Some(&first.id)).unwrap().tools, p.tools);
    let hidden = app.mcp_status(Some(&second.id)).unwrap();
    assert!(!hidden.scope_authorized);
    assert!(hidden.tools.is_empty());
    let encoded = serde_json::to_string(&hidden).unwrap();
    assert!(!encoded.contains("synthetic_read"));
    assert!(!encoded.contains(&first.id));
    assert!(app.mcp_status(None).unwrap().tools.is_empty());
    let backup = root.path().join("policy.sqlite3");
    app.store.backup(&backup).unwrap();
    let restored = root.path().join("restored");
    drop(Store::restore(&backup, &restored).unwrap());
    let restored = Workspace::open_existing_no_upgrade(restored).unwrap();
    assert_eq!(restored.settings().unwrap().mcp, Some(p.clone()));
    assert_eq!(
        restored.store.record(&note.id).unwrap(),
        app.store.record(&note.id).unwrap()
    );
    assert!(restored.mcp_status(Some(&first.id)).unwrap().grant_required);
    drop(app);
    let mut app = Workspace::open_existing_no_upgrade(data).unwrap();
    assert_eq!(app.settings().unwrap().mcp, Some(p));
    app.disable_mcp().unwrap();
    assert!(!app.mcp_status(Some(&first.id)).unwrap().connection_enabled);
    assert!(app.mcp_status(Some(&first.id)).unwrap().tools.is_empty());
    app.configure_mcp(None).unwrap();
    assert!(app.settings().unwrap().mcp.is_none());
}
#[test]
fn invalid_owner_configuration_does_not_change_canonical_state() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::create_without_default_credentials(
        root.path().join("data"),
        WorkspacePurpose::Test,
    )
    .unwrap();
    let before = app.records().unwrap();
    for value in [
        json!({"endpoint":"http://localhost:8077/mcp","enabled":false,"tools":[],"access":[]}),
        json!({"endpoint":"http://127.0.0.1:8077/mcp?token=secret","enabled":false,"tools":[],"access":[]}),
    ] {
        let p: McpPolicy = serde_json::from_value(value).unwrap();
        assert!(app.configure_mcp(Some(p)).is_err());
        assert_eq!(app.records().unwrap(), before);
    }
}

#[test]
fn project_policy_metadata_is_scoped_and_movement_never_copies_approval() {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let mut app = Workspace::create_without_default_credentials(
        root.path().join("data"),
        WorkspacePurpose::Test,
    )
    .unwrap();
    let general = app.new_thread("General synthetic context").unwrap().id;
    let create = |app: &mut Workspace, title: &str| {
        let receipt = app
            .project_command(
                &uuid::Uuid::new_v4().to_string(),
                ProjectCommand::Create {
                    title: title.into(),
                },
            )
            .unwrap();
        receipt
            .writes
            .into_iter()
            .find(|w| matches!(w.after.data, RecordData::Project(_)))
            .unwrap()
            .after
            .id
    };
    let first = create(&mut app, "First synthetic Project");
    let second = create(&mut app, "Second synthetic Project");
    let receipt = app
        .project_command(
            &uuid::Uuid::new_v4().to_string(),
            ProjectCommand::CreateChat {
                project: first.clone(),
                title: "Scoped chat".into(),
            },
        )
        .unwrap();
    let chat = receipt
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Thread(_)))
        .unwrap()
        .after
        .id;
    let configured = policy(vec![McpAccess::Project(first.clone())]);
    app.configure_mcp(Some(configured.clone())).unwrap();
    assert_eq!(app.mcp_status(Some(&chat)).unwrap().tools, configured.tools);
    for context in [None, Some(general.as_str())] {
        let hidden = app.mcp_status(context).unwrap();
        assert!(!hidden.scope_authorized);
        assert!(hidden.tools.is_empty());
        let text = serde_json::to_string(&hidden).unwrap();
        for secret in [
            "synthetic_read",
            first.as_str(),
            configured.tools[0].metadata_digest.as_str(),
        ] {
            assert!(!text.contains(secret), "no out-of-scope policy metadata");
        }
    }
    app.project_command(
        &uuid::Uuid::new_v4().to_string(),
        ProjectCommand::MoveChat {
            chat: chat.clone(),
            project: Some(second),
        },
    )
    .unwrap();
    let moved = app.mcp_status(Some(&chat)).unwrap();
    assert!(!moved.scope_authorized);
    assert!(moved.tools.is_empty());
    assert!(
        !serde_json::to_string(&moved)
            .unwrap()
            .contains("synthetic_read")
    );
    assert_eq!(
        app.settings().unwrap().mcp,
        Some(configured),
        "movement never copies or rewrites owner approval"
    );
}
