//! Synthetic native controls only: no provider dispatch or OS picker.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};
fn fixture(
    cx: &mut TestAppContext,
) -> (
    tempfile::TempDir,
    Entity<ThreadsUi>,
    gpui_kit::AnyWindowHandle,
    String,
) {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
    });
    let root = tempfile::tempdir().unwrap();
    let data = root.path().join("data");
    let mut workspace = Workspace::open_without_default_credentials(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let chat = workspace.new_thread("Capture fixture").unwrap();
    workspace
        .post_message(
            &chat.id,
            "We discussed a design. No final approval or completed follow-up.",
        )
        .unwrap();
    let thread = chat.id.clone();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1260.), px(1000.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data, window, cx);
                    ui.poll = None;
                    ui.open_thread(&thread, window, cx);
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    (root, view, window, chat.id)
}
#[gpui_kit::test]
fn capture_confirmation_cancel_has_no_saved_request_or_run(cx: &mut TestAppContext) {
    let (_root, view, window, _chat) = fixture(cx);
    let before = cx.read(|cx| view.read(cx).workspace.records().unwrap());
    cx.update_window(window, |_, window, cx| {
        assert!(window.find("capture-outcomes").visible());
        window.click("capture-outcomes", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(window.find("capture-outcomes-confirm").visible());
        window.click("capture-outcomes-cancel", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let ui = view.read(cx);
        assert_eq!(ui.workspace.records().unwrap(), before);
        assert!(ui.worker.is_none());
    });
}
#[gpui_kit::test]
fn capture_confirmation_rejects_changed_destination_before_saving_or_dispatch(
    cx: &mut TestAppContext,
) {
    let (_root, view, window, chat) = fixture(cx);
    cx.update_window(window, |_, window, cx| window.click("capture-outcomes", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, _, cx| {
        view.update(cx, |ui, _| {
            let record = ui.workspace.store.record(&chat).unwrap().unwrap();
            let RecordData::Thread(mut thread) = record.data else {
                panic!()
            };
            thread.private_context = true;
            ui.workspace
                .owner_change(
                    "concurrent-private-context",
                    &ChangeRequest {
                        reason: "Synthetic context change while dialog is open".into(),
                        inputs: vec![],
                        writes: vec![Put {
                            id: chat.clone(),
                            expected_version: Some(record.version),
                            archived: false,
                            data: RecordData::Thread(thread),
                        }],
                    },
                )
                .unwrap();
        })
    })
    .unwrap();
    let before = cx.read(|cx| view.read(cx).workspace.records().unwrap());
    cx.update_window(window, |_, window, cx| {
        window.click("capture-outcomes-confirm", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let ui = view.read(cx);
        assert!(ui.worker.is_none());
        assert_eq!(ui.workspace.records().unwrap(), before);
        assert!(
            !ui.workspace
                .records()
                .unwrap()
                .iter()
                .any(|r| matches!(r.data, RecordData::Run(_)))
        );
    });
}
#[test]
fn capture_continue_binds_latest_run_not_newest_marker_and_later_owner_input_starts_fresh() {
    fn message(id: &str, role: &str) -> Record {
        Record {
            id: id.into(),
            version: 1,
            archived: false,
            data: RecordData::Message(Message {
                thread: "chat".into(),
                role: role.into(),
                text: "Synthetic".into(),
            }),
        }
    }
    let mut messages = vec![
        message("owner", "owner"),
        message("capture-old", CAPTURE_REQUEST_ROLE),
        message("capture-new", CAPTURE_REQUEST_ROLE),
    ];
    let run = Record {
        id: "run".into(),
        version: 1,
        archived: false,
        data: RecordData::Run(Run {
            instruction: Some("capture-old".into()),
            queue_claim: None,
            thread: "chat".into(),
            provider: "Codex".into(),
            model: "fixture".into(),
            guide_identity: "fixture".into(),
            budget: 4,
            effort: "medium".into(),
            loaded_skills: vec![],
            loaded_skill_details: vec![],
            fence: 1,
            state: RunState::Interrupted,
            progress: String::new(),
            operations: vec![],
            sources: vec![],
        }),
    };
    assert_eq!(
        capture_continuation(ContinuePath::FreshRun, &messages, Some(&run)),
        ContinuePath::RetryCapture("capture-old".into())
    );
    assert_eq!(
        capture_continuation(ContinuePath::FreshRun, &messages, None),
        ContinuePath::CaptureNeedsConfirmation
    );
    assert_eq!(
        capture_continuation(ContinuePath::ResumeQueue, &messages, Some(&run)),
        ContinuePath::ResumeQueue
    );
    messages.push(message("new-owner", "owner"));
    assert_eq!(
        capture_continuation(ContinuePath::FreshRun, &messages, Some(&run)),
        ContinuePath::FreshRun
    );
}
