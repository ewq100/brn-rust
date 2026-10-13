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
                        instruction: None,
                        queue_claim: None,
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
        assert_eq!(window.find("chat-steer").label(), Some("Steer"));
        window.click("chat-steer", cx);
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
            assert_eq!(ui.composer.read(cx).value().as_ref(), "");
            assert_eq!(ui.queue_state().queue[0].text, "Later A draft");
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
    cx.update_window(f.window, |_, window, cx| window.click("chat-steer", cx))
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
    cx.update_window(f.window, |_, window, cx| window.click("chat-steer", cx))
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
    cx.update_window(f.window, |_, window, cx| window.click("chat-steer", cx))
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

#[gpui_kit::test]
fn enter_queues_while_working_and_stop_pauses_saved_queue_across_late_finish(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, RunState::Working);
    cx.update_window(f.window,|_,window,cx|{
        f.view.update(cx,|ui,cx|{
            ui.ask(window,cx);
            assert!(!f.cancel.is_cancelled());assert!(ui.pending_send.is_none());
            assert_eq!(ui.queue_state().queue[0].text,"New exact instruction · õun");assert_eq!(ui.composer.read(cx).value().as_ref(),"");
            ui.stream="Displayed partial answer".into();ui.cancel(cx);assert!(ui.queue_state().paused);
            f.sender.send(WorkerEvent::Finished(Ok("Late finished event".into()))).unwrap();ui.poll_worker(window,cx);
            assert!(ui.worker.is_none());assert!(ui.queue_state().paused);assert_eq!(ui.queue_state().queue.len(),1);
            assert!(ui.workspace.messages(&f.thread).unwrap().iter().any(|r|matches!(&r.data,RecordData::Message(m) if m.role=="assistant-partial" && m.text=="Displayed partial answer")));
        });
    }).unwrap();
    let mut reopened = Workspace::open(&f.data).unwrap();
    assert!(
        matches!(reopened.chat_control().unwrap().data,RecordData::ChatControl(c) if c.paused && c.queue.len()==1)
    );
    assert!(reopened.dispatch_queued("still-paused").unwrap().is_none());
}

#[gpui_kit::test]
fn quiet_display_keeps_stream_and_stop_and_saved_drafts_survive_reopening(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Working);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.workspace.choose_quiet("quiet-fixture", true).unwrap();
            ui.stream = "Readable streaming answer".into();
            ui.progress = "Detailed tool activity".into();
            let before = ui.worker.as_ref().unwrap().run_id.clone();
            ui.open_thread(&f.other, window, cx);
            ui.composer
                .update(cx, |e, cx| e.set_value("Other unsent õun", window, cx));
            ui.open_thread(&f.thread, window, cx);
            assert_eq!(
                ui.composer.read(cx).value().as_ref(),
                "New exact instruction · õun"
            );
            assert_eq!(ui.worker.as_ref().unwrap().run_id, before);
            assert_eq!(ui.stream_for(&f.thread), Some("Readable streaming answer"));
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("run-stop").visible());
        assert!(window.find("chat-quiet").visible());
    })
    .unwrap();
    let w = Workspace::open(&f.data).unwrap();
    let drafts = w.store.chat_drafts().unwrap();
    assert!(drafts.iter().any(|d| d.text == "Other unsent õun"));
    assert!(
        drafts
            .iter()
            .any(|d| d.text == "New exact instruction · õun")
    );
    assert!(matches!(w.chat_control().unwrap().data,RecordData::ChatControl(c) if c.quiet));
}

#[gpui_kit::test]
fn archive_and_steer_preserve_partial_answers_and_repeated_stop_does_not_duplicate(
    cx: &mut TestAppContext,
) {
    for archive in [true, false] {
        let f = fixture(cx, RunState::Working);
        cx.update_window(f.window,|_,window,cx|{f.view.update(cx,|ui,cx|{
            ui.stream="Useful displayed partial answer".into();
            if archive {ui.resolve_thread(&f.thread,cx);}else{ui.stop_and_send(window,cx);}
            ui.cancel(cx);
            f.sender.send(WorkerEvent::Finished(Ok("Late terminal".into()))).unwrap();ui.poll_worker(window,cx);
            let messages=ui.workspace.messages(&f.thread).unwrap();assert_eq!(messages.iter().filter(|r|matches!(&r.data,RecordData::Message(m) if m.role=="assistant-partial" && m.text=="Useful displayed partial answer")).count(),1);
        });}).unwrap();
        let w = Workspace::open(&f.data).unwrap();
        assert!(w.messages(&f.thread).unwrap().iter().any(|r|matches!(&r.data,RecordData::Message(m) if m.role=="assistant-partial" && m.text=="Useful displayed partial answer")));
    }
}

#[gpui_kit::test]
fn quiet_toggle_removes_activity_from_every_live_status_without_changing_work(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, RunState::Working);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.workspace.choose_quiet("start-detailed", false).unwrap();
            f.sender
                .send(WorkerEvent::Progress("Finding current notes…".into()))
                .unwrap();
            f.sender
                .send(WorkerEvent::Progress("Reading a note passage…".into()))
                .unwrap();
            ui.poll_worker(window, cx);
            ui.stream = "The answer keeps streaming".into();
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(
            window
                .find("worker-status")
                .label()
                .unwrap()
                .contains("Reading a note passage")
        );
        assert!(window.find("run-activity").visible());
        assert_eq!(
            window.find("run-activity-step-0").label(),
            Some("Finding current notes…")
        );
        window.click("chat-quiet", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(
            !window
                .find("worker-status")
                .label()
                .unwrap()
                .contains("Reading a note passage"),
            "Quiet must suppress activity in the workspace status bar too"
        );
        assert!(
            window
                .find("worker-status")
                .label()
                .unwrap()
                .contains("Working quietly")
        );
        assert!(window.find("run-stop").visible());
        assert!(window.find("stop-work").visible());
        assert!(window.try_find("run-activity").is_none());
        f.view.update(cx, |ui, _| {
            assert_eq!(ui.stream_for(&f.thread), Some("The answer keeps streaming"));
            assert_eq!(
                ui.worker.as_ref().unwrap().run_id.as_deref(),
                Some(f.run.as_str())
            );
            assert!(!ui.worker.as_ref().unwrap().cancel.is_cancelled());
            assert_eq!(ui.workspace.settings().unwrap().provider, "offline-fixture");
        });
        window.click("chat-quiet", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("run-activity").visible());
        assert_eq!(
            window.find("run-activity-step-1").label(),
            Some("Reading a note passage…")
        );
        f.view.update(cx, |ui, cx| {
            for i in 0..40 {
                f.sender
                    .send(WorkerEvent::Progress(format!("Working step {i}")))
                    .unwrap();
                ui.poll_worker(window, cx);
            }
            assert_eq!(ui.run_activity_labels.len(), 32);
            f.sender
                .send(WorkerEvent::Finished(Err(
                    "Synthetic failure remains visible".into(),
                )))
                .unwrap();
            ui.poll_worker(window, cx);
            assert!(
                !ui.run_activity_labels.is_empty(),
                "latest activity remains inspectable after the answer ends"
            );
            assert!(ui.notice.contains("Synthetic failure remains visible"));
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("run-activity").visible());
        let before = f.view.read(cx).run_activity_scroll.offset();
        window.scroll(
            "run-activity-steps",
            gpui_kit::ScrollDelta::Pixels(point(px(0.), px(120.))),
            cx,
        );
        let after = f.view.read(cx).run_activity_scroll.offset();
        assert!(
            after.y > before.y,
            "the activity pane must scroll independently of the conversation"
        );
        f.view.update(cx, |ui, cx| {
            let (_, receiver) = mpsc::sync_channel(16);
            ui.worker = Some(Worker {
                receiver,
                cancel: CancellationToken::new(),
                conversion_cancel: Arc::new(AtomicBool::new(false)),
                kind: WorkerKind::Import,
                thread: Some(f.other.clone()),
                run_id: None,
            });
            ui.progress = "Unrelated conversion".into();
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("run-activity").visible());
        assert!(
            !window
                .find("run-line")
                .label()
                .unwrap()
                .contains("Unrelated conversion")
        );
        assert!(
            window.try_find("run-stop").is_none(),
            "a completed chat must not offer Stop for another worker"
        );
        f.view.update(cx, |ui, cx| {
            let retained_status = ui.run_activity_status.clone();
            ui.poll_worker(window, cx);
            assert!(ui.worker.is_none());
            assert_eq!(
                ui.run_activity_status, retained_status,
                "unrelated disconnect must not rewrite the retained run outcome"
            );
            ui.open_thread(&f.other, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("run-activity").is_none())
    })
    .unwrap();
}
#[gpui_kit::test]
fn queued_question_dispatches_after_successful_attachment_preparation(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Completed);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.worker.as_mut().unwrap().kind = WorkerKind::Import;
            ui.ask(window, cx);
            assert_eq!(ui.queue_state().queue.len(), 1);
            let converted = ConversionResult {
                intent: ImportIntent::FullNote,
                markdown: "# Saved attachment\nContent".into(),
                assets: vec![],
                source_sha256: [0; 32],
                source_reference: "/missing/file.md".into(),
                converter: "synthetic".into(),
                sources: vec![],
                coverage: brn_threads_intake::Coverage {
                    status: brn_threads_intake::CoverageStatus::Unchecked,
                    checked_items: vec![],
                    gaps: vec![],
                },
            };
            f.sender
                .send(WorkerEvent::Converted {
                    result: Box::new(Ok(converted)),
                    title: "Attachment".into(),
                    chat: Some(f.thread.clone()),
                })
                .unwrap();
            ui.poll_worker(window, cx);
            // Offline provider refuses startup, proving dispatch reached current settings;
            // the claimed message remains saved and explicitly paused for retry.
            assert!(ui.queue_state().pending.is_some());
            assert!(ui.queue_state().paused);
            assert!(ui.notice.contains("Choose a provider"));
            assert!(ui.records.iter().any(
                |r| matches!(&r.data,RecordData::Note(n) if n.chat.as_ref()==Some(&f.thread))
            ));
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn long_queue_and_attachment_labels_keep_controls_inside_the_chat_tile(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Working);
    let mut attachment = String::new();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.workspace
                .queue_message("long-queue-card", &f.thread, &"A long waiting message ".repeat(15))
                .unwrap();
            ui.workspace
                .queue_message("second-queue-card", &f.thread, &"Another long message ".repeat(15))
                .unwrap();
            let converted = ConversionResult {
                intent: ImportIntent::FullNote,
                markdown: "Synthetic saved document".into(),
                assets: vec![],
                source_sha256: [0; 32],
                source_reference: "/missing/long-name.md".into(),
                converter: "synthetic".into(),
                sources: vec![],
                coverage: brn_threads_intake::Coverage {
                    status: brn_threads_intake::CoverageStatus::Unchecked,
                    checked_items: vec![],
                    gaps: vec![],
                },
            };
            ui.workspace
                .attach_full("long-attachment-card", &f.thread, &"Long document title ".repeat(10), &converted)
                .unwrap();
            ui.refresh();
            attachment = ui.records.iter().find(|r| matches!(&r.data, RecordData::Note(n) if n.chat.as_ref() == Some(&f.thread))).unwrap().id.clone();
            ui.open_thread(&f.thread, window, cx);
        });
    }).unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        let tile = window.find("tile-chat").bounds();
        for id in [
            "queue-edit-0".to_owned(),
            "queue-remove-0".to_owned(),
            "queue-up-1".to_owned(),
            format!("attachment-open-{attachment}"),
            format!("attachment-keep-{attachment}"),
            format!("attachment-coverage-{attachment}"),
        ] {
            let control = window.find(id.clone());
            let bounds = control.bounds();
            assert!(control.visible(), "{id} must remain reachable");
            assert!(
                bounds.origin.x >= tile.origin.x,
                "{id} starts outside the tile"
            );
            assert!(
                bounds.origin.x + bounds.size.width <= tile.origin.x + tile.size.width,
                "{id} overflows the chat tile"
            );
        }
    })
    .unwrap();
}

#[gpui_kit::test]
fn starting_a_replacement_run_clears_the_old_paused_notice(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Interrupted);
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |ui, cx| {
            ui.worker = None;
            let mut settings = ui.workspace.settings().unwrap();
            settings.provider = "Codex".into();
            // An explicit nonexistent fixture path prevents any real credential access.
            settings.auth_file = Some(f.data.join("no-credentials.json").display().to_string());
            ui.workspace.configure(settings).unwrap();
            ui.notice = "Work stopped: Interrupted. The queue is paused; Resume when ready.".into();
            ui.start_run(
                "Replacement instruction".into(),
                f.thread.clone(),
                vec![],
                None,
                cx,
            );
            assert!(ui.worker.is_some());
            assert_eq!(ui.progress, "Starting…");
            assert!(
                ui.notice.is_empty(),
                "old interruption must not describe new work"
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn dismissed_completion_banner_can_be_restored_from_layout_menu(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Completed);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            f.sender
                .send(WorkerEvent::Finished(Ok(
                    "Finished. The answer and saved changes are in this chat.".into(),
                )))
                .unwrap();
            ui.poll_worker(window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert_eq!(window.find("idle-status").label(), Some("○ Idle"));
        assert!(
            window.try_find("status-model").is_none(),
            "model selection belongs in the composer"
        );
        assert!(window.find("model-pill").visible());
        assert!(window.find("effort-pill").visible());
    })
    .unwrap();
    let original = cx.read(|cx| f.view.read(cx).notice.clone());
    assert!(original.contains("Work in Active destination · Finished"));
    for _ in 0..2 {
        cx.update_window(f.window, |_, window, cx| window.click("dismiss-notice", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(f.window, |_, window, cx| {
            assert!(window.try_find("workspace-notice").is_none());
            window.click("status-layout", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(f.window, |_, window, cx| {
            let mut menu = window.within("popup-menu");
            let index = (0..60usize)
                .find(|i| {
                    menu.try_find(*i)
                        .is_some_and(|row| row.label() == Some("Last work status"))
                })
                .expect("dismissed work must have a restore option");
            menu.click(index, cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(f.window, |_, window, cx| {
            assert_eq!(
                window.find("workspace-notice").label(),
                Some(original.as_str())
            );
            assert!(
                f.view.read(cx).worker.is_none(),
                "restoring a banner must not restart work"
            );
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn restoring_old_work_status_preserves_newer_errors_and_active_work(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Working);
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |ui, cx| {
            ui.last_work_notice = Some("Previous work finished".into());
            ui.notice.clear();
            ui.show_last_work_notice(cx);
            assert!(
                ui.notice.is_empty(),
                "active work cannot display an old completion"
            );
            ui.worker = None;
            ui.notice = "New error needs attention".into();
            ui.show_last_work_notice(cx);
            assert_eq!(ui.notice, "New error needs attention");
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn completed_context_edit_refreshes_saved_text_and_preserves_human_changes(
    cx: &mut TestAppContext,
) {
    for (human_typing, hide_note) in [(false, false), (true, false), (false, true)] {
        let f = fixture(cx, RunState::Completed);
        cx.update_window(f.window, |_, window, cx| {
            f.view.update(cx, |ui, cx| {
                let note = ui
                    .workspace
                    .new_note("Context edit target", "Before edit")
                    .unwrap();
                ui.workspace
                    .link_writing_thread("context-link", &note.id, note.version, &f.thread)
                    .unwrap();
                ui.refresh();
                ui.open_note(&note.id, None, window, cx);
                ui.context_edit_note = Some((f.run.clone(), note.id.clone(), note.version));
                ui.workspace
                    .owner_change(
                        "saved-context-edit",
                        &ChangeRequest {
                            reason: "Synthetic committed contextual edit".into(),
                            inputs: vec![],
                            writes: vec![Put {
                                id: note.id.clone(),
                                expected_version: Some(note.version),
                                archived: false,
                                data: RecordData::Note(Note::working(
                                    "Context edit target",
                                    "After edit",
                                )),
                            }],
                        },
                    )
                    .unwrap();
                if human_typing {
                    ui.note_editor.update(cx, |editor, cx| {
                        editor.set_value("Human draft stays visible", window, cx)
                    });
                }
                if hide_note {
                    ui.layout.close(TileKind::Note);
                }
                f.sender
                    .send(WorkerEvent::Finished(Ok("Finished".into())))
                    .unwrap();
                ui.poll_worker(window, cx);
                assert!(ui.context_edit_note.is_none());
                if hide_note {
                    assert!(
                        !ui.note_tile_visible(),
                        "completion must not reopen a hidden note tile"
                    );
                } else if human_typing {
                    assert_eq!(ui.note_editor.read(cx).value(), "Human draft stays visible");
                } else {
                    assert_eq!(ui.note.as_ref().unwrap().record.version, 2);
                    assert_eq!(ui.note_editor.read(cx).value(), "After edit");
                }
            })
        })
        .unwrap();
    }
}
