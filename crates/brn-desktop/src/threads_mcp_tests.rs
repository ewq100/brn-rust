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

const NF1_REFUSAL: &str = "MCP input review no longer matches this exact owner request. No new run was started; review the destination, scope and input again.";
const NF1_REWRITE: &str = "Rewrite the saved document using the captured feedback and selected chat; prepare a whole-note proposal for review.";

struct StartFixture {
    _root: tempfile::TempDir,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
    chat: String,
    note: Record,
}

fn start_fixture(cx: &mut TestAppContext, prepare: bool) -> StartFixture {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("nf1");
    let mut workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    if prepare {
        let mut settings = workspace.settings().unwrap();
        settings.provider = "Codex".into();
        settings.model = "gpt-6.1-sol".into();
        settings.effort = "medium".into();
        settings.auth_file = Some(
            root.path()
                .join("nonexistent-auth.json")
                .to_string_lossy()
                .into(),
        );
        settings.credentials_dir = None;
        workspace.configure(settings).unwrap();
    }
    let note = workspace
        .new_note("NF1 saved draft", "Keep this exact saved text.")
        .unwrap();
    let chat = workspace
        .ensure_writing_context("nf1-writing-context", &note.id, None)
        .unwrap()
        .conversation
        .unwrap();
    workspace.configure_mcp(Some(policy(&chat))).unwrap();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut ui = ThreadsUi::new(workspace, data, window, cx);
                ui.poll = None;
                ui.open_note(&note.id, None, window, cx);
                ui.chat = ChatTarget::Thread(chat.clone());
                ui.refresh();
                ui
            })
        })
        .unwrap()
    });
    StartFixture {
        _root: root,
        window,
        view,
        chat,
        note,
    }
}

fn review_start(
    ui: &mut ThreadsUi,
    prompt: &str,
    window: &mut Window,
    cx: &mut Context<ThreadsUi>,
) {
    ui.composer
        .update(cx, |editor, cx| editor.set_value(prompt, window, cx));
    ui.begin_mcp_input("public_read", window, cx);
    ui.mcp_view
        .query
        .as_ref()
        .unwrap()
        .clone()
        .update(cx, |editor, cx| {
            editor.set_value("NF1_TRANSIENT_QUERY", window, cx)
        });
    ui.review_mcp_input(cx);
    assert!(ui.mcp_view.pending.is_some());
}

#[gpui_kit::test]
fn nf1_retry_consumes_mismatch_before_prepare_and_second_retry_progresses(cx: &mut TestAppContext) {
    let f = start_fixture(cx, false);
    cx.update_window(f.window, |_, window, cx| f.view.update(cx, |ui, cx| {
        let saved = ui.workspace.snapshot_writing_request_with_feedback("nf1-snapshot", &f.note.id, f.note.version, &f.chat, &[], "Captured feedback").unwrap();
        ui.writing.general_feedback.update(cx, |editor, cx| editor.set_value("Unsent next feedback", window, cx));
        review_start(ui, "Different composer instruction", window, cx);
        let records = ui.workspace.records().unwrap();
        let receipts = ui.workspace.history().unwrap();
        ui.retry_rewrite(saved.request_message.clone(), cx);
        assert!(ui.worker.is_none());
        assert!(ui.mcp_view.pending.is_none());
        assert!(ui.mcp_view.query.is_none());
        assert_eq!(ui.notice, format!("Rewrite request saved; AI has not started. {NF1_REFUSAL} Retry rewrite uses these same saved inputs."));
        assert_eq!(ui.notice_tone(), Tone::Warning);
        assert_eq!(ui.workspace.records().unwrap(), records);
        assert_eq!(ui.workspace.history().unwrap(), receipts);
        assert_eq!(ui.workspace.writing_request(&saved.request_message).unwrap(), saved);
        assert_eq!(ui.writing.general_feedback.read(cx).value().as_ref(), "Unsent next feedback");
        ui.retry_rewrite(saved.request_message.clone(), cx);
        assert!(ui.worker.is_none());
        assert_eq!(ui.notice, "Rewrite request saved; AI has not started. Choose a provider and model in Settings before asking BRN. Retry rewrite uses these same saved inputs.");
        assert_eq!(ui.workspace.records().unwrap(), records);
        assert_eq!(ui.workspace.history().unwrap(), receipts);
        assert_eq!(ui.workspace.writing_request(&saved.request_message).unwrap(), saved);
        assert_eq!(ui.writing.general_feedback.read(cx).value().as_ref(), "Unsent next feedback");
    })).unwrap();
}

#[gpui_kit::test]
fn nf1_chat_mismatch_refuses_without_a_run(cx: &mut TestAppContext) {
    let f = start_fixture(cx, true);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            review_start(ui, "Reviewed prompt", window, cx);
            let records = ui.workspace.records().unwrap();
            let receipts = ui.workspace.history().unwrap();
            ui.start_run("Different prompt".into(), f.chat.clone(), vec![], None, cx);
            assert_eq!(ui.notice, NF1_REFUSAL);
            assert_eq!(ui.notice_tone(), Tone::Warning);
            assert!(ui.worker.is_none());
            assert!(ui.mcp_view.pending.is_none());
            assert!(ui.mcp_view.query.is_none());
            assert_eq!(ui.workspace.records().unwrap(), records);
            assert_eq!(ui.workspace.history().unwrap(), receipts);
        })
    })
    .unwrap();
}

fn queued_start_refuses(cx: &mut TestAppContext, resume: bool, rewrite: bool) {
    let f = start_fixture(cx, true);
    cx.update_window(f.window, |_, window, cx| f.view.update(cx, |ui, cx| {
        let saved = rewrite.then(|| ui.workspace.snapshot_writing_request_with_feedback("nf1-queued-rewrite", &f.note.id, f.note.version, &f.chat, &[], "Captured queued feedback").unwrap());
        let prompt = if rewrite { NF1_REWRITE } else { "Exact queued instruction" };
        ui.workspace.queue_message("nf1-enqueue", &f.chat, prompt).unwrap();
        ui.workspace.resume_queue("nf1-initial-resume").unwrap();
        assert!(ui.workspace.dispatch_queued("nf1-dispatch").unwrap().unwrap().dispatch.is_some());
        if resume { ui.workspace.resume_queue("nf1-resume-without-claim").unwrap(); }
        let pending = ui.queue_state().pending.unwrap();
        assert!(pending.run.is_none());
        assert_eq!(pending.dispatch.is_none(), resume);
        ui.refresh();
        review_start(ui, prompt, window, cx);
        let records = ui.workspace.records().unwrap();
        let receipts = ui.workspace.history().unwrap();
        if let Some(saved) = saved {
            ui.retry_rewrite(saved.request_message, cx);
            assert_eq!(ui.notice, format!("Rewrite request saved; AI has not started. {NF1_REFUSAL} Retry rewrite uses these same saved inputs."));
        } else {
            ui.start_run(prompt.into(), f.chat.clone(), vec![], None, cx);
            assert_eq!(ui.notice, NF1_REFUSAL);
        }
        assert!(ui.worker.is_none());
        assert!(ui.mcp_view.pending.is_none());
        assert!(ui.mcp_view.query.is_none());
        assert_eq!(ui.notice_tone(), Tone::Warning);
        assert_eq!(ui.workspace.records().unwrap(), records);
        assert_eq!(ui.workspace.history().unwrap(), receipts);
    })).unwrap();
}

#[gpui_kit::test]
fn nf1_queued_chat_claim_refuses_before_prepare(cx: &mut TestAppContext) {
    queued_start_refuses(cx, false, false);
}
#[gpui_kit::test]
fn nf1_queued_chat_without_claim_refuses_before_prepare(cx: &mut TestAppContext) {
    queued_start_refuses(cx, true, false);
}
#[gpui_kit::test]
fn nf1_retry_fixed_prompt_queue_without_claim_preserves_refusal(cx: &mut TestAppContext) {
    queued_start_refuses(cx, true, true);
}

#[gpui_kit::test]
fn nf1_exact_chat_start_attaches_then_spawns_exactly_one_run(cx: &mut TestAppContext) {
    let f = start_fixture(cx, true);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            review_start(ui, "Exact owner prompt", window, cx);
            let before = ui.workspace.records().unwrap();
            ui.start_run(
                "Exact owner prompt".into(),
                f.chat.clone(),
                vec![],
                None,
                cx,
            );
            let worker = ui
                .worker
                .as_ref()
                .expect("matching review reaches synchronous spawn");
            let run = worker.run_identity().map(|run| &run.run_id).unwrap();
            assert!(!ui.notice.contains("MCP input review no longer matches"));
            assert!(!ui.notice.contains("MCP input authorization refused"));
            assert!(ui.mcp_view.pending.is_none());
            assert!(ui.mcp_view.query.is_none());
            let new_runs: Vec<_> = ui
                .workspace
                .records()
                .unwrap()
                .into_iter()
                .filter(|r| {
                    matches!(r.data, RecordData::Run(_)) && !before.iter().any(|old| old.id == r.id)
                })
                .collect();
            assert_eq!(new_runs.len(), 1);
            assert_eq!(&new_runs[0].id, run);
            worker.cancel.cancel();
        })
    })
    .unwrap();
}

#[gpui_kit::test]
fn nf1_busy_chat_and_retry_leave_exact_review_available(cx: &mut TestAppContext) {
    let f = start_fixture(cx, false);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            let saved = ui
                .workspace
                .snapshot_writing_request_with_feedback(
                    "nf1-busy-snapshot",
                    &f.note.id,
                    f.note.version,
                    &f.chat,
                    &[],
                    "Captured feedback",
                )
                .unwrap();
            let run = chat_tests::record_run(
                &mut ui.workspace,
                &f.chat,
                RunState::Working,
                "Synthetic busy run",
            );
            // Review the current Run before exercising busy starts; creating a
            // newer Run after review must invalidate the exact input stamp.
            ui.refresh();
            review_start(ui, "Exact after busy", window, cx);
            let records = ui.workspace.records().unwrap();
            let receipts = ui.workspace.history().unwrap();
            let (_sender, receiver) = mpsc::sync_channel(1);
            ui.worker = Some(Worker::run(
                receiver,
                CancellationToken::new(),
                f.chat.clone(),
                run,
            ));
            ui.start_run("Exact after busy".into(), f.chat.clone(), vec![], None, cx);
            assert_eq!(ui.notice, WorkerKind::Run.busy_notice());
            assert!(ui.mcp_view.pending.is_some());
            assert!(ui.mcp_view.query.is_some());
            ui.retry_rewrite(saved.request_message, cx);
            assert_eq!(ui.notice, WorkerKind::Run.busy_notice());
            assert!(ui.mcp_view.pending.is_some());
            assert!(ui.mcp_view.query.is_some());
            assert_eq!(ui.workspace.records().unwrap(), records);
            assert_eq!(ui.workspace.history().unwrap(), receipts);
            ui.worker = None;
            assert!(
                ui.take_mcp_review(&f.chat, "Exact after busy", false)
                    .unwrap()
                    .is_some()
            );
        })
    })
    .unwrap();
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
