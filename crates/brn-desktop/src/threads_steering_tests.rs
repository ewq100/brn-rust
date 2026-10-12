//! Production native dispatch with synthetic channels and persisted offline settings.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    data: PathBuf,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
    thread: String,
    other: String,
    run: String,
    sender: mpsc::SyncSender<WorkerEvent>,
    cancel: CancellationToken,
}
fn fixture(cx: &mut TestAppContext, state: RunState) -> Fixture {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("steering");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let settings = workspace.settings().unwrap();
    let thread = workspace.new_thread("Active destination").unwrap().id;
    let other = workspace.new_thread("Other destination").unwrap().id;
    workspace
        .post_message(&thread, "Original instruction")
        .unwrap();
    let operation = workspace.store.allocate_operation("synthetic-run").unwrap();
    let run = operation.creation_id(0);
    workspace
        .owner_change(
            "synthetic-run",
            &ChangeRequest {
                reason: "Synthetic host invocation".into(),
                writes: vec![Put {
                    id: run.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Run(Run {
                        thread: thread.clone(),
                        provider: "offline-fixture".into(),
                        model: settings.model.clone(),
                        guide_identity: "synthetic".into(),
                        budget: 1,
                        effort: settings.effort.clone(),
                        loaded_skills: vec![],
                        fence: 0,
                        state,
                        progress: "Synthetic progress".into(),
                        operations: vec![],
                        sources: vec![],
                    }),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    let (sender, receiver) = mpsc::sync_channel(16);
    let cancel = CancellationToken::new();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1260.), px(1100.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    ui.worker = Some(Worker {
                        receiver,
                        cancel: cancel.clone(),
                        conversion_cancel: Arc::new(AtomicBool::new(false)),
                        kind: WorkerKind::Run,
                        thread: Some(thread.clone()),
                        run_id: Some(run.clone()),
                    });
                    ui.open_thread(&thread, window, cx);
                    ui.composer.update(cx, |editor, cx| {
                        editor.set_value("New exact instruction · õun", window, cx)
                    });
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    Fixture {
        _root: root,
        data,
        window,
        view,
        thread,
        other,
        run,
        sender,
        cancel,
    }
}
fn assert_saved(f: &Fixture, expected_state: RunState, expected_fence: u64) {
    let workspace = Workspace::open(&f.data).unwrap();
    let messages = workspace.messages(&f.thread).unwrap();
    assert_eq!(messages.len(), 2);
    assert!(
        matches!(&messages[1].data, RecordData::Message(message) if message.role == "owner" && message.text == "New exact instruction · õun")
    );
    let RecordData::Run(run) = workspace.store.record(&f.run).unwrap().unwrap().data else {
        panic!("run")
    };
    assert_eq!(run.state, expected_state);
    assert_eq!(run.fence, expected_fence);
    let settings = workspace.settings().unwrap();
    assert_eq!(settings.provider, "offline-fixture");
    assert!(settings.auth_file.is_none() && settings.credentials_dir.is_none());
}

#[gpui_kit::test]
fn stop_and_send_click_is_durable_and_waits_then_drains_without_retargeting(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, RunState::Working);
    cx.update_window(f.window, |_, window, cx| {
        assert_eq!(window.find("ask-delegate").label(), Some("Stop & send ↵"));
        window.click("ask-delegate", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(f.cancel.is_cancelled());
    assert_saved(&f, RunState::Interrupted, 1);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            assert_eq!(ui.composer.read(cx).value().as_ref(), "");
            ui.composer.update(cx, |editor, cx| {
                editor.set_value("Later A draft", window, cx)
            });
            ui.ask(window, cx); // A second click cannot replace the pending instruction.
            assert_eq!(
                ui.pending_send.as_ref().unwrap().prompt,
                "New exact instruction · õun"
            );
            assert_eq!(
                ui.worker.as_ref().unwrap().run_id.as_deref(),
                Some(f.run.as_str())
            );
            f.sender
                .send(WorkerEvent::Text("Old partial answer".into()))
                .unwrap();
            ui.poll_worker(window, cx);
            assert!(ui.pending_send.is_some());
            assert!(ui.worker.is_some(), "no second invocation before Finished");
            ui.open_thread(&f.other, window, cx);
            assert!(!ui.can_stop_and_send());
            ui.composer
                .update(cx, |editor, cx| editor.set_value("New B draft", window, cx));
            // A stale cached provider would start a worker. Production dispatch must reload settings.
            ui.settings.provider = "codex".into();
            f.sender
                .send(WorkerEvent::Finished(Ok("Old worker finished".into())))
                .unwrap();
            f.sender
                .send(WorkerEvent::Text("Late old text".into()))
                .unwrap();
            ui.poll_worker(window, cx);
            assert!(ui.pending_send.is_none());
            assert!(
                ui.worker.is_none(),
                "persisted offline provider refuses startup without credential access"
            );
            assert!(
                ui.stream.is_empty(),
                "old batch cannot contaminate replacement preview"
            );
            assert_eq!(ui.chat, ChatTarget::Thread(f.other.clone()));
            assert_eq!(ui.composer.read(cx).value().as_ref(), "New B draft");
            assert!(ui.notice.contains("Choose a provider"));
            ui.open_thread(&f.thread, window, cx);
            assert_eq!(ui.composer.read(cx).value().as_ref(), "Later A draft");
            assert!(can_continue_thread(
                &ui.records,
                &ui.history,
                &f.thread,
                &ThreadState::Open,
                false
            ));
        })
    })
    .unwrap();
    assert_saved(&f, RunState::Interrupted, 1);
}

#[gpui_kit::test]
fn pending_disconnect_retains_instruction_and_offers_continue_after_reopen(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, RunState::Working);
    cx.update_window(f.window, |_, window, cx| window.click("ask-delegate", cx))
        .unwrap();
    cx.run_until_parked();
    f.sender
        .send(WorkerEvent::Text(
            "Trailing progress before disconnect".into(),
        ))
        .unwrap();
    drop(f.sender);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.poll_worker(window, cx);
            assert!(ui.worker.is_none() && ui.pending_send.is_none());
            assert!(ui.notice.contains("unexpectedly"));
            assert!(ui.stream.is_empty());
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find(format!("continue-{}", f.run)).visible());
    })
    .unwrap();
    let workspace = Workspace::open(&f.data).unwrap();
    assert!(can_continue_thread(
        &workspace.records().unwrap(),
        &workspace.history().unwrap(),
        &f.thread,
        &ThreadState::Open,
        false
    ));
    assert_eq!(workspace.messages(&f.thread).unwrap().len(), 2);
}

#[gpui_kit::test]
fn stop_during_pending_send_preserves_message_and_cancels_automatic_launch(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, RunState::Working);
    cx.update_window(f.window, |_, window, cx| window.click("ask-delegate", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.click("stop-work", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            assert!(ui.pending_send.is_none());
            assert!(ui.notice.contains("Automatic continuation stopped"));
            f.sender
                .send(WorkerEvent::Finished(Ok("Interrupted".into())))
                .unwrap();
            ui.poll_worker(window, cx);
            assert!(ui.worker.is_none());
            assert!(
                !ui.notice.contains("Choose a provider"),
                "Stop suppresses dispatch"
            );
        })
    })
    .unwrap();
    assert_saved(&f, RunState::Interrupted, 1);
}

#[gpui_kit::test]
fn failed_steering_persistence_retains_text_worker_and_controls(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Working);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.workspace.resolve_thread(&f.thread).unwrap();
            ui.stop_and_send(window, cx);
            assert_eq!(
                ui.composer.read(cx).value().as_ref(),
                "New exact instruction · õun"
            );
            assert!(ui.pending_send.is_none());
            assert!(!f.cancel.is_cancelled());
            assert!(ui.worker.is_some() && ui.can_stop_and_send());
            assert!(ui.notice.contains("Could not send"));
            assert_eq!(ui.workspace.messages(&f.thread).unwrap().len(), 1);
        })
    })
    .unwrap();
}

#[gpui_kit::test]
fn terminal_race_accepts_exact_message_and_completed_run_can_continue(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Completed);
    cx.update_window(f.window, |_, window, cx| window.click("ask-delegate", cx))
        .unwrap();
    cx.run_until_parked();
    assert_saved(&f, RunState::Completed, 0);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.cancel(cx);
            f.sender
                .send(WorkerEvent::Finished(Ok("Completed".into())))
                .unwrap();
            ui.poll_worker(window, cx);
            assert!(can_continue_thread(
                &ui.records,
                &ui.history,
                &f.thread,
                &ThreadState::Open,
                false
            ));
        })
    })
    .unwrap();
    let workspace = Workspace::open(&f.data).unwrap();
    assert!(can_continue_thread(
        &workspace.records().unwrap(),
        &workspace.history().unwrap(),
        &f.thread,
        &ThreadState::Open,
        false
    ));
}

#[gpui_kit::test]
fn no_run_continue_requires_saved_owner_message_and_dispatches_offline(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Completed);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.worker = None;
            ui.open_thread(&f.other, window, cx);
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find(format!("continue-{}", f.other)).is_none());
        f.view.update(cx, |ui, cx| {
            ui.workspace
                .post_message(&f.other, "Saved instruction after refused startup")
                .unwrap();
            ui.refresh();
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find(format!("continue-{}", f.other)).visible());
        window.click(format!("continue-{}", f.other), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let ui = f.view.read(cx);
        assert!(ui.worker.is_none());
        assert!(ui.notice.contains("Choose a provider"));
        assert_eq!(ui.workspace.messages(&f.other).unwrap().len(), 1);
    });
}
