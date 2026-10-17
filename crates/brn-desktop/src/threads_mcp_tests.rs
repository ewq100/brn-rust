//! Synthetic native controls/state only; no GUI or real MCP/provider endpoint.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};
fn policy(chat: &str) -> McpPolicy {
    McpPolicy {
        endpoint: "http://127.0.0.1:8077/mcp".into(),
        server_digest: "a".repeat(64),
        enabled: true,
        tools: vec![McpApprovedTool {
            name: "public_read".into(),
            metadata_digest: "b".repeat(64),
            schema_digest: mcp_input_schema_digest(McpInputContract::Query),
            contract: McpInputContract::Query,
        }],
        access: vec![McpAccess::Chat(chat.into())],
    }
}
#[gpui_kit::test]
fn native_exact_input_is_transient_once_and_cleared_on_scope_policy_or_run_change(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let mut selected = workspace.settings().unwrap();
    selected.provider = "Codex".into();
    selected.model = "gpt-6.1-sol".into();
    selected.effort = "medium".into();
    selected.auth_file = None;
    selected.credentials_dir = None;
    workspace.configure(selected).unwrap();
    let chat = workspace.new_thread("MCP synthetic scope").unwrap().id;
    let other = workspace.new_thread("Other scope").unwrap().id;
    workspace.configure_mcp(Some(policy(&chat))).unwrap();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1400.), px(1600.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    ui.layout.open(TileKind::Settings);
                    ui.settings_section = SettingsSection::Mcp;
                    ui.chat = ChatTarget::Thread(chat.clone());
                    ui.refresh();
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(window.find("mcp-discover").visible());
        assert!(window.find("mcp-review-public_read").visible());
        view.update(cx, |ui, cx| {
            ui.composer.update(cx, |e, cx| {
                e.set_value("Inspect synthetic public source", window, cx)
            });
            ui.begin_mcp_input("public_read", window, cx);
            let query = ui.mcp_view.query.as_ref().unwrap().clone();
            query.update(cx, |q, cx| {
                q.set_value("TRANSIENT_NATIVE_QUERY_CANARY", window, cx)
            });
            ui.review_mcp_input(cx);
            assert!(ui.mcp_view.pending.is_some());
            assert!(
                !serde_json::to_string(&ui.workspace.records().unwrap())
                    .unwrap()
                    .contains("TRANSIENT_NATIVE_QUERY_CANARY")
            );
            assert!(
                ui.take_mcp_review(&chat, "Changed owner prompt", false)
                    .is_err()
            );
            assert!(ui.mcp_view.pending.is_none());
            ui.begin_mcp_input("public_read", window, cx);
            ui.review_mcp_input(cx);
            assert!(
                ui.take_mcp_review(&chat, "Inspect synthetic public source", true)
                    .is_err(),
                "queued work must not inherit a review"
            );
            ui.begin_mcp_input("public_read", window, cx);
            ui.review_mcp_input(cx);
            let reviewed = ui
                .take_mcp_review(&chat, "Inspect synthetic public source", false)
                .unwrap()
                .unwrap();
            let prepared = ui
                .workspace
                .prepare_thread_run(ThreadRunStart {
                    thread: chat.clone(),
                    prompt: "Inspect synthetic public source".into(),
                    request_message: None,
                    queue_claim: None,
                    owner_request: None,
                    sources: vec![],
                    max_tool_rounds: Some(2),
                    trace: brn_ai::threads::TraceLevel::Off,
                    offered_copilot_models: None,
                })
                .unwrap();
            // Real checked attachment; no connection is made before worker.
            let prepared = reviewed.attach(prepared.prepared).unwrap();
            assert!(ui.mcp_view.pending.is_none());
            drop(prepared);
            ui.refresh();
            ui.begin_mcp_input("public_read", window, cx);
            ui.review_mcp_input(cx);
            assert!(ui.mcp_view.pending.is_some());
            ui.set_chat(ChatTarget::Thread(other.clone()), window, cx);
            assert!(ui.mcp_view.pending.is_none());
            assert!(ui.mcp_view.query.is_none());
            assert!(ui.mcp_view.status.as_ref().unwrap().tools.is_empty());
            ui.set_chat(ChatTarget::Thread(chat.clone()), window, cx);
            ui.disable_mcp(cx);
            assert!(!ui.workspace.settings().unwrap().mcp.unwrap().enabled);
            assert!(ui.mcp_view.status.as_ref().unwrap().tools.is_empty());
        });
    })
    .unwrap();
    cx.run_until_parked();
    let reopened = Workspace::open_existing_no_upgrade(data).unwrap();
    assert!(!reopened.settings().unwrap().mcp.unwrap().enabled);
    assert!(
        !serde_json::to_string(&reopened.records().unwrap())
            .unwrap()
            .contains("TRANSIENT_NATIVE_QUERY_CANARY")
    );
}

// Ordered host-action barriers isolate take -> checked change -> durable prepare.
// The corrected fixture calls the same saved-snapshot attachment as start_run.
fn reviewed_snapshot_handoff_barrier(cx: &mut TestAppContext, policy_switch: bool) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let mut selected = workspace.settings().unwrap();
    selected.provider = "Codex".into();
    selected.model = "gpt-6.1-sol".into();
    selected.effort = "medium".into();
    selected.auth_file = None;
    selected.credentials_dir = None;
    workspace.configure(selected).unwrap();
    let chat = workspace.new_thread("Reviewed scope").unwrap().id;
    let mut projects = vec![];
    for key in ["review-p1", "review-p2"] {
        let receipt = workspace
            .project_command(key, ProjectCommand::Create { title: key.into() })
            .unwrap();
        projects.push(
            receipt
                .writes
                .iter()
                .find(|w| matches!(w.after.data, RecordData::Project(_)))
                .unwrap()
                .after
                .id
                .clone(),
        );
    }
    workspace
        .project_command(
            "review-initial-move",
            ProjectCommand::MoveChat {
                chat: chat.clone(),
                project: Some(projects[0].clone()),
            },
        )
        .unwrap();
    // Exact Chat access permits both contexts: rejection cannot be an unrelated
    // missing Project permission, and unchanged preparation is a positive below.
    workspace.configure_mcp(Some(policy(&chat))).unwrap();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1400.), px(1600.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    ui.chat = ChatTarget::Thread(chat.clone());
                    ui.refresh();
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| view.update(cx, |ui,cx| {
        let prompt = "Review exact synthetic query";
        ui.composer.update(cx, |e,cx| e.set_value(prompt, window, cx));
        ui.begin_mcp_input("public_read", window, cx);
        ui.mcp_view.query.as_ref().unwrap().clone().update(cx, |e,cx| e.set_value("NATIVE_REVIEW_SNAPSHOT_CANARY", window, cx));
        ui.review_mcp_input(cx);
        let reviewed = ui.take_mcp_review(&chat, prompt, false).unwrap().unwrap();
        if policy_switch {
            let mut changed = policy(&chat);
            changed.endpoint = "http://127.0.0.1:8078/mcp".into();
            ui.workspace.configure_mcp(Some(changed)).unwrap();
        } else {
            ui.workspace.project_command("review-barrier-move", ProjectCommand::MoveChat { chat: chat.clone(), project: Some(projects[1].clone()) }).unwrap();
        }
        // Both changes happened BEFORE prepare; ordinary post-prepare liveness
        // therefore cannot detect that the native owner reviewed the old facts.
        let prepared = ui.workspace.prepare_thread_run(ThreadRunStart { thread: chat.clone(), prompt: prompt.into(), request_message: None, queue_claim: None, owner_request: None, sources: vec![], max_tool_rounds: Some(2), trace: brn_ai::threads::TraceLevel::Off, offered_copilot_models: None }).unwrap();
        let run_id = prepared.prepared.run_id().to_owned();
        let before = ui.workspace.records().unwrap().into_iter().find(|r| r.id==run_id).unwrap();
        let result = reviewed.attach(prepared.prepared);
        assert!(result.is_err(), "saved native review was silently rebound after policy/context changed before prepare");
        let after = ui.workspace.records().unwrap().into_iter().find(|r| r.id==run_id).unwrap();
        let (RecordData::Run(before),RecordData::Run(after)) = (before.data,after.data) else { panic!("Run records") };
        assert_eq!(before.guide_identity,after.guide_identity,"no guide mutation before mismatch refusal");
        assert_eq!(before.fence + 1,after.fence,"only the ordinary terminal fence advance, no catalogue-attachment advance");
        assert_eq!(after.state,RunState::Failed);
        assert!(after.instruction.is_some());
        assert!(ui.worker.is_none());
        assert!(ui.mcp_view.pending.is_none());
        let canonical = serde_json::to_string(&ui.workspace.records().unwrap()).unwrap();
        assert!(canonical.contains(prompt));
        assert!(!canonical.contains("NATIVE_REVIEW_SNAPSHOT_CANARY"));
    })).unwrap();
    cx.run_until_parked();
}
#[gpui_kit::test]
fn native_review_policy_switch_before_prepare_is_not_reauthorized(cx: &mut TestAppContext) {
    reviewed_snapshot_handoff_barrier(cx, true);
}
#[gpui_kit::test]
fn native_review_permitted_project_move_before_prepare_is_not_reauthorized(
    cx: &mut TestAppContext,
) {
    reviewed_snapshot_handoff_barrier(cx, false);
}
