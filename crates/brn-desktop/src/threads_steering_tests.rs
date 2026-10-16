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
    /// A seeded Working run stands for this window's live worker, so it
    /// holds the run's lease as a real worker does; without it every stop
    /// path would treat the run as crashed.
    lease: Option<brn_ai::run_lease::RunLease>,
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
    let live = state == RunState::Working;
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
    let lease = live.then(|| {
        brn_ai::run_lease::RunLease::acquire(&workspace.store.derived_directory().unwrap(), &run)
            .unwrap()
    });
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
        lease,
    }
}
/// The chat's attention reasons, read from the saved chat.
fn saved_reasons(f: &Fixture) -> Vec<String> {
    let workspace = Workspace::open(&f.data).unwrap();
    match workspace.store.record(&f.thread).unwrap().unwrap().data {
        RecordData::Thread(thread) => thread.attention.into_iter().map(|a| a.reason).collect(),
        _ => panic!("Thread"),
    }
}
fn lease_exists(f: &Fixture) -> bool {
    Workspace::open(&f.data)
        .unwrap()
        .store
        .derived_directory()
        .unwrap()
        .join("run-leases")
        .join(format!("{}.lock", f.run.replace(':', "_")))
        .exists()
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
    // Steering live work adds no notice, and the worker keeps its lease.
    assert!(
        !saved_reasons(&f)
            .iter()
            .any(|r| r == brn_ai::threads::INTERRUPTED_ATTENTION),
        "{:?}",
        saved_reasons(&f)
    );
    assert!(lease_exists(&f));
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

/// A seeded Working run whose process ended (no lease) counts as crashed:
/// Steer stops it with the Interrupted notice and removes its free lease
/// file. The follow-up run's start would clear the notice (decision 1);
/// offline, no run starts here.
#[gpui_kit::test]
fn steer_of_a_reply_no_process_holds_records_the_interrupted_notice(cx: &mut TestAppContext) {
    let mut f = fixture(cx, RunState::Working);
    // The process ended: its lease is gone, its file left behind.
    drop(f.lease.take());
    let leftover = Workspace::open(&f.data)
        .unwrap()
        .store
        .derived_directory()
        .unwrap()
        .join("run-leases")
        .join(format!("{}.lock", f.run.replace(':', "_")));
    std::fs::write(&leftover, b"").unwrap();
    cx.update_window(f.window, |_, window, cx| window.click("chat-steer", cx))
        .unwrap();
    cx.run_until_parked();
    assert_saved(&f, RunState::Interrupted, 1);
    assert_eq!(
        saved_reasons(&f),
        vec![brn_ai::threads::INTERRUPTED_ATTENTION.to_owned()]
    );
    assert!(!lease_exists(&f));
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
        // The live line names the status beside an icon, as the status bar does.
        assert!(
            window
                .find("run-line")
                .label()
                .is_some_and(|label| label.starts_with("Reading a note passage"))
        );
        assert_eq!(
            window.find("run-activity-step-0").label(),
            Some("Finding current notes…")
        );
        assert_eq!(
            window.find("chat-quiet").label(),
            Some("Quiet off · hide activity")
        );
        assert!(window.try_find("composer-destination").is_none());
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
        assert_eq!(
            window.find("chat-quiet").label(),
            Some("Quiet on · show activity")
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
                "{id} starts outside the panel"
            );
            assert!(
                bounds.origin.x + bounds.size.width <= tile.origin.x + tile.size.width,
                "{id} overflows the Chat panel"
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
    cx.update_window(f.window, |_, window, cx| {
        assert_eq!(window.find("idle-status").label(), Some("Idle"));
        assert!(
            window.try_find("status-model").is_none(),
            "model selection belongs in the composer"
        );
        // A chat below 420 pt names the effort in the model control, whose
        // menu holds the effort choices.
        let settings = f.view.read(cx).settings.clone();
        assert!(f.view.read(cx).chat_narrow);
        assert!(window.find("model-pill").visible());
        assert_eq!(
            window.find("model-pill").label(),
            Some(format!("Model {}, effort {}", settings.model, settings.effort).as_str())
        );
        assert!(window.try_find("effort-pill").is_none());
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
            ui.last_work_notice = Some((Tone::Success, "Previous work finished".into()));
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
                        "completion must not reopen a hidden Note panel"
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

/// The workspace banner's announced tone, as a screen reader hears it.
fn banner_tone(cx: &mut TestAppContext, f: &Fixture) -> Option<String> {
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        window
            .try_find("workspace-notice-tone")
            .and_then(|tone| tone.label().map(str::to_owned))
    })
    .unwrap()
}
fn finish(cx: &mut TestAppContext, f: &Fixture, result: Result<String, String>) {
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            f.sender.send(WorkerEvent::Finished(result)).unwrap();
            ui.poll_worker(window, cx);
        })
    })
    .unwrap();
}
fn synthetic_run(ui: &mut ThreadsUi, key: &str, thread: &str, run: Run) -> String {
    let id = ui
        .workspace
        .store
        .allocate_operation(key)
        .unwrap()
        .creation_id(0);
    ui.workspace
        .owner_change(
            key,
            &ChangeRequest {
                reason: "Synthetic host invocation".into(),
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Run(Run {
                        thread: thread.into(),
                        ..run
                    }),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    id
}
fn run_template(ui: &ThreadsUi, state: RunState) -> Run {
    let settings = ui.workspace.settings().unwrap();
    Run {
        instruction: None,
        queue_claim: None,
        thread: String::new(),
        provider: "offline-fixture".into(),
        model: settings.model,
        guide_identity: "synthetic".into(),
        budget: 1,
        effort: settings.effort,
        loaded_skills: vec![],
        fence: 0,
        state,
        progress: "Synthetic progress".into(),
        operations: vec![],
        sources: vec![],
    }
}

#[gpui_kit::test]
fn a_failed_finish_is_an_error_banner_and_restores_as_an_error(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Working);
    finish(cx, &f, Err("Synthetic failure remains visible".into()));
    let shown = cx.read(|cx| f.view.read(cx).notice.clone());
    assert!(
        shown.contains("Synthetic failure remains visible"),
        "{shown}"
    );
    assert_eq!(banner_tone(cx, &f).as_deref(), Some("Error"));
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
            .expect("the failure can be restored");
        menu.click(index, cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert_eq!(
            window.find("workspace-notice").label(),
            Some(shown.as_str())
        );
    })
    .unwrap();
    assert_eq!(
        banner_tone(cx, &f).as_deref(),
        Some("Error"),
        "restoring keeps the failure's tone"
    );
}

#[gpui_kit::test]
fn a_finish_takes_its_tone_from_its_own_recorded_run(cx: &mut TestAppContext) {
    for (state, tone) in [
        (RunState::Failed, "Error"),
        (RunState::Completed, "Success"),
        (RunState::Interrupted, "Warning"),
        (RunState::Working, "Warning"),
    ] {
        let f = fixture(cx, state.clone());
        // A newer completed run in the same chat must not lend its tone.
        cx.update_window(f.window, |_, _, cx| {
            f.view.update(cx, |ui, _| {
                let newer = run_template(ui, RunState::Completed);
                synthetic_run(ui, "newer-run", &f.thread, newer);
            })
        })
        .unwrap();
        finish(
            cx,
            &f,
            Ok("Finished. The answer and saved changes are in this chat.".into()),
        );
        assert_eq!(banner_tone(cx, &f).as_deref(), Some(tone), "{state:?}");
        let last = cx.read(|cx| f.view.read(cx).last_work_notice.clone());
        assert_eq!(last.map(|(tone, _)| tone.word()), Some(tone), "{state:?}");
    }
    // Work without a run (export, backup, import, connect) that finishes is a success.
    let f = fixture(cx, RunState::Completed);
    let (sender, receiver) = mpsc::sync_channel(4);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.worker = Some(Worker {
                receiver,
                cancel: CancellationToken::new(),
                conversion_cancel: Arc::new(AtomicBool::new(false)),
                kind: WorkerKind::Backup,
                thread: None,
                run_id: None,
            });
            sender
                .send(WorkerEvent::Finished(Ok(
                    "Full backup created at /private/tmp/brn-synthetic".into(),
                )))
                .unwrap();
            ui.poll_worker(window, cx);
        })
    })
    .unwrap();
    assert_eq!(banner_tone(cx, &f).as_deref(), Some("Success"));
}

/// Gives the fixture's chat a recorded notice for its run, the shape the
/// service leaves when a run ends Failed or Interrupted: a Blocker attention
/// whose record is the run, holding the text Needs you shows. The run's
/// progress (the agent's own summary) is set to something else, and an
/// older notice for another run stays beside it.
fn record_terminal(cx: &mut TestAppContext, f: &Fixture, text: &str) {
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |ui, _| {
            let older = run_template(ui, RunState::Failed);
            let older = synthetic_run(ui, "older-terminal-run", &f.thread, older);
            let record = ui.workspace.store.record(&f.run).unwrap().unwrap();
            let RecordData::Run(mut run) = record.data else {
                unreachable!()
            };
            run.progress = "Synthetic agent progress: drafted section 2".into();
            let chat = ui.workspace.store.record(&f.thread).unwrap().unwrap();
            let RecordData::Thread(mut data) = chat.data else {
                unreachable!()
            };
            data.attention = vec![
                Attention {
                    kind: AttentionKind::Blocker,
                    reason: "Synthetic notice for an older run".into(),
                    record: Some(older),
                },
                Attention {
                    kind: AttentionKind::Blocker,
                    reason: text.into(),
                    record: Some(f.run.clone()),
                },
            ];
            ui.workspace
                .owner_change(
                    "synthetic-terminal-notice",
                    &ChangeRequest {
                        reason: "Synthetic terminal notice".into(),
                        writes: vec![
                            Put {
                                id: f.run.clone(),
                                expected_version: Some(record.version),
                                archived: false,
                                data: RecordData::Run(run),
                            },
                            Put {
                                id: f.thread.clone(),
                                expected_version: Some(chat.version),
                                archived: false,
                                data: RecordData::Thread(data),
                            },
                        ],
                        inputs: vec![],
                    },
                )
                .unwrap();
        })
    })
    .unwrap();
}

/// U10: a run that ended Failed or Interrupted reads as the chat's
/// recorded notice for that run (the text Needs you shows), in that state's
/// tone, never the raw error or the agent's progress. Without a recorded
/// notice the error text stays. The owner's own Stop keeps its short
/// warning. The texts are seeded here; none is the service's wording.
#[gpui_kit::test]
fn a_failed_or_interrupted_finish_shows_the_recorded_terminal_text(cx: &mut TestAppContext) {
    let recorded = "Synthetic recorded terminal notice for this chat.";
    for (state, tone) in [
        (RunState::Failed, "Error"),
        (RunState::Interrupted, "Warning"),
    ] {
        let f = fixture(cx, state.clone());
        record_terminal(cx, &f, recorded);
        finish(cx, &f, Err("raw provider display text".into()));
        let shown = cx.read(|cx| f.view.read(cx).notice.clone());
        assert_eq!(shown, format!("Work in Active destination · {recorded}"));
        assert!(!shown.contains("raw provider"), "{shown}");
        assert!(!shown.contains("Synthetic agent progress"), "{shown}");
        assert!(!shown.contains("older run"), "{shown}");
        assert_eq!(banner_tone(cx, &f).as_deref(), Some(tone), "{state:?}");
        let last = cx.read(|cx| f.view.read(cx).last_work_notice.clone());
        assert_eq!(last.map(|(tone, _)| tone.word()), Some(tone));
    }
    // No recorded notice for this run: the error text, never the progress.
    let f = fixture(cx, RunState::Failed);
    finish(cx, &f, Err("Synthetic provider error".into()));
    let shown = cx.read(|cx| f.view.read(cx).notice.clone());
    assert_eq!(
        shown,
        "Work in Active destination · Work stopped: Synthetic provider error"
    );
    assert!(!shown.contains("Synthetic progress"), "{shown}");
    assert_eq!(banner_tone(cx, &f).as_deref(), Some("Error"));
    // The owner stopped it: the short Stop warning, not the recorded text.
    let f = fixture(cx, RunState::Interrupted);
    record_terminal(cx, &f, recorded);
    f.cancel.cancel();
    finish(cx, &f, Err("stopped by the owner".into()));
    let shown = cx.read(|cx| f.view.read(cx).notice.clone());
    assert!(shown.contains("Work stopped:"), "{shown}");
    assert!(!shown.contains(recorded), "{shown}");
    assert_eq!(banner_tone(cx, &f).as_deref(), Some("Warning"));
}

#[gpui_kit::test]
fn outcome_tones_and_a_direct_overwrite_never_inherit_an_older_tone(cx: &mut TestAppContext) {
    let f = fixture(cx, RunState::Completed);
    let receipt = |ui: &mut ThreadsUi, needs_refresh: Vec<String>| Receipt {
        operation: ui
            .workspace
            .store
            .allocate_operation("tone-receipt")
            .unwrap(),
        request_hash: String::new(),
        reason: "Synthetic change".into(),
        writes: vec![],
        authority: "owner".into(),
        committed_at: "2026-10-14T19:47:00Z".into(),
        undo_of: None,
        needs_refresh,
    };
    type Case = Box<dyn Fn(&mut ThreadsUi)>;
    let cases: [(&str, Case); 6] = [
        (
            "Warning",
            Box::new(|ui| {
                ui.report_outcome(&ApplyOutcome::Stale {
                    records: vec!["synthetic".into()],
                })
            }),
        ),
        (
            "Warning",
            Box::new(move |ui| {
                let receipt = receipt(ui, vec!["derived-a".into(), "derived-b".into()]);
                ui.report_outcome(&ApplyOutcome::Applied(receipt));
                assert!(
                    ui.notice
                        .contains("2 related items may need refresh. Open Changes"),
                    "{}",
                    ui.notice
                );
            }),
        ),
        (
            "Success",
            Box::new(move |ui| {
                let receipt = receipt(ui, vec![]);
                ui.report_outcome(&ApplyOutcome::Applied(receipt));
            }),
        ),
        (
            "Info",
            Box::new(|ui| {
                ui.report_outcome(&ApplyOutcome::NeedsReview {
                    denied: vec!["synthetic".into()],
                })
            }),
        ),
        ("Error", Box::new(|ui| ui.fail("Synthetic error"))),
        (
            "Info",
            Box::new(|ui| {
                ui.fail("Synthetic error");
                ui.notice = "A later direct notice".into();
            }),
        ),
    ];
    for (expected, case) in cases {
        cx.update_window(f.window, |_, _, cx| {
            f.view.update(cx, |ui, cx| {
                case(ui);
                cx.notify();
            })
        })
        .unwrap();
        let notice = cx.read(|cx| f.view.read(cx).notice.clone());
        assert_eq!(banner_tone(cx, &f).as_deref(), Some(expected), "{notice}");
    }
}

#[gpui_kit::test]
fn a_refused_undo_renders_as_an_error_banner(cx: &mut TestAppContext) {
    // Err arm in `undo_receipt`: the service refuses Undo of an operation it
    // never recorded. The refusal does not depend on run liveness, so no
    // Working run is resumed or dispatched here.
    let f = fixture(cx, RunState::Completed);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.undo_receipt("synthetic-missing-operation", window, cx);
        })
    })
    .unwrap();
    let notice = cx.read(|cx| f.view.read(cx).notice.clone());
    assert!(!notice.is_empty());
    assert_eq!(banner_tone(cx, &f).as_deref(), Some("Error"), "{notice}");
}

#[gpui_kit::test]
fn a_refused_stop_renders_as_an_error_banner(cx: &mut TestAppContext) {
    // if-let-Err in `cancel`: the service refuses to stop a run whose fence
    // cannot advance. (Resume's own if-let-Err needs a bound queued run,
    // which only a prepared provider run can create.)
    let f = fixture(cx, RunState::Completed);
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |ui, cx| {
            let template = run_template(ui, RunState::Working);
            let run = synthetic_run(
                ui,
                "unstoppable-run",
                &f.thread,
                Run {
                    fence: u64::MAX,
                    ..template
                },
            );
            ui.worker.as_mut().unwrap().run_id = Some(run);
            cx.notify();
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.click("stop-work", cx))
        .unwrap();
    let notice = cx.read(|cx| f.view.read(cx).notice.clone());
    assert!(!notice.is_empty());
    assert_eq!(banner_tone(cx, &f).as_deref(), Some("Error"), "{notice}");
    assert!(
        !f.cancel.is_cancelled(),
        "a refused stop leaves the worker running"
    );
}
