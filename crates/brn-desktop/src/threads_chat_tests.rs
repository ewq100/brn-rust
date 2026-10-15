//! Offline chat witnesses for the UI polish: the recovery card and its
//! recorded reason, the confirmed Stop for a saved Working reply, review
//! cards that name their notes, queue notices, Archive and the composer.
//! Every run is synthetic; pixels and pointer feel remain native acceptance.
//! Fixtures that seed a Working run with no live worker never Resume or
//! Dispatch after seeding it (integration watch item).
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
    /// "Delivery chat", with one saved owner message. `seed` also receives
    /// "Other chat".
    thread: String,
}

/// A window of this size showing "Delivery chat"; `seed` adds runs, queue
/// state and proposals before the window opens.
fn fixture(
    cx: &mut TestAppContext,
    width: f32,
    height: f32,
    seed: impl FnOnce(&mut Workspace, &str, &str),
) -> Fixture {
    cx.update(|cx| {
        gpui_kit::init(cx);
        // Keep dialog hit targets at their resting positions before clicks.
        cx.set_reduce_motion(true);
    });
    cx.update(|cx| cx.bind_keys(bindings()));
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("chat-polish");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let thread = workspace.new_thread("Delivery chat").unwrap().id;
    let other = workspace.new_thread("Other chat").unwrap().id;
    workspace
        .post_message(&thread, "Draft the delivery update")
        .unwrap();
    seed(&mut workspace, &thread, &other);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(width), px(height)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    ui.open_thread(&thread, window, cx);
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    Fixture {
        _root: root,
        window,
        view,
        thread,
    }
}
fn update<R>(
    cx: &mut TestAppContext,
    f: &Fixture,
    run: impl FnOnce(&mut ThreadsUi, &mut Window, &mut Context<ThreadsUi>) -> R,
) -> R {
    let result = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| run(view, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}
fn on_window<R>(
    cx: &mut TestAppContext,
    f: &Fixture,
    run: impl FnOnce(&mut Window, &mut App) -> R,
) -> R {
    let result = cx
        .update_window(f.window, |_, window, cx| run(window, cx))
        .unwrap();
    cx.run_until_parked();
    result
}
fn click(cx: &mut TestAppContext, f: &Fixture, id: impl Into<String>) {
    let id = id.into();
    on_window(cx, f, |window, cx| window.click(id, cx));
}
/// A synthetic run with its own recorded reason.
fn record_run(workspace: &mut Workspace, thread: &str, state: RunState, progress: &str) -> String {
    let key = format!("chat-run-{}", uuid::Uuid::new_v4());
    let settings = workspace.settings().unwrap();
    let id = workspace
        .store
        .allocate_operation(&key)
        .unwrap()
        .creation_id(0);
    workspace
        .owner_change(
            &key,
            &ChangeRequest {
                reason: "Synthetic chat run".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Run(Run {
                        instruction: None,
                        queue_claim: None,
                        thread: thread.into(),
                        provider: "offline-fixture".into(),
                        model: settings.model,
                        guide_identity: "chat-polish".into(),
                        budget: 1,
                        effort: settings.effort,
                        loaded_skills: vec![],
                        fence: 0,
                        state,
                        progress: progress.into(),
                        operations: vec![],
                        sources: vec![],
                    }),
                }],
            },
        )
        .unwrap();
    id
}
fn run_state(cx: &mut TestAppContext, f: &Fixture, run: &str) -> RunState {
    cx.read(|cx| {
        match f
            .view
            .read(cx)
            .workspace
            .store
            .record(run)
            .unwrap()
            .unwrap()
            .data
        {
            RecordData::Run(run) => run.state,
            _ => unreachable!(),
        }
    })
}
fn label(window: &mut Window, id: impl Into<String>) -> String {
    window
        .find(id.into())
        .label()
        .unwrap_or_default()
        .to_owned()
}
/// A live local worker for the chat's run, as a running reply has.
fn hold_worker(ui: &mut ThreadsUi, thread: &str, run: &str) -> mpsc::SyncSender<WorkerEvent> {
    let (sender, receiver) = mpsc::sync_channel(16);
    ui.worker = Some(Worker {
        receiver,
        cancel: CancellationToken::new(),
        conversion_cancel: Arc::new(AtomicBool::new(false)),
        kind: WorkerKind::Run,
        thread: Some(thread.into()),
        run_id: Some(run.into()),
    });
    sender
}
fn notice_tone(cx: &mut TestAppContext, f: &Fixture) -> Option<String> {
    on_window(cx, f, |window, _| {
        window
            .try_find("workspace-notice-tone")
            .and_then(|tone| tone.label().map(str::to_owned))
    })
}

#[test]
fn the_model_control_shows_display_names() {
    use shell::model_display;
    assert_eq!(
        model_display("codex", "gpt-6.1-sol"),
        "ChatGPT · gpt-6.1-sol"
    );
    assert_eq!(
        model_display("chatgpt", "gpt-6.1-sol"),
        "ChatGPT · gpt-6.1-sol"
    );
    assert_eq!(
        model_display("copilot", "gpt-6.1-sol"),
        "GitHub Copilot · gpt-6.1-sol"
    );
    assert_eq!(
        model_display("offline-fixture", "gpt-6.1-sol"),
        "gpt-6.1-sol"
    );
    assert_eq!(model_display("", "gpt-6.1-sol"), "gpt-6.1-sol");
}

/// U6: the seeded failure text, not the service's own wording.
const FAILURE: &str =
    "Run failed: Reconnect this account before continuing. Continue when the provider is ready.";

#[gpui_kit::test]
fn a_failed_reply_shows_its_whole_recorded_reason_wrapping_at_300pt(cx: &mut TestAppContext) {
    let mut run = String::new();
    let f = fixture(cx, 900., 600., |workspace, thread, _| {
        run = record_run(workspace, thread, RunState::Failed, FAILURE);
    });
    let card = format!("recovery-card-{}", f.thread);
    on_window(cx, &f, |window, _| {
        let chat = window.find("tile-chat").bounds();
        assert!(chat.size.width <= px(310.), "a 300 pt chat: {chat:?}");
        assert!(label(window, &card).contains(FAILURE));
        assert_eq!(label(window, format!("{card}-state")), "Failed");
        assert_eq!(label(window, format!("{card}-tone")), "Error");
        assert_eq!(label(window, format!("{card}-reason")), FAILURE);
        assert_eq!(
            label(window, format!("{card}-next")),
            "Your message is saved. Continue starts a fresh reply with the saved chat and current notes."
        );
        assert_eq!(label(window, format!("continue-{run}")), "Continue");
        assert!(window.try_find(format!("stop-saved-run-{run}")).is_none());
        // The reason wraps in full rather than truncating to one line.
        let state = window.find(format!("{card}-state")).bounds();
        let reason = window.find(format!("{card}-reason")).bounds();
        assert!(
            reason.size.height > state.size.height * 1.5,
            "the reason wraps: {reason:?} vs one line {state:?}"
        );
        let shown = window.find(card.clone()).bounds();
        assert!(shown.origin.x >= chat.origin.x);
        assert!(shown.origin.x + shown.size.width <= chat.origin.x + chat.size.width);
    });
}

#[gpui_kit::test]
fn a_failed_reply_with_a_paused_queue_names_what_stays_paused(cx: &mut TestAppContext) {
    // Shot 29: Failed, no pending item, paused, one queued message.
    let mut run = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, other| {
        run = record_run(
            workspace,
            thread,
            RunState::Failed,
            "Provider unavailable (synthetic failure)",
        );
        workspace
            .queue_message("shot-29-queued", other, "Check the venue")
            .unwrap();
        workspace.pause_queue("shot-29-paused").unwrap();
    });
    let card = format!("recovery-card-{}", f.thread);
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, format!("{card}-reason")),
            "Provider unavailable (synthetic failure)"
        );
        assert_eq!(label(window, format!("continue-{run}")), "Continue");
        assert_eq!(
            label(window, format!("{card}-waiting")),
            "1 waiting message stays paused until you resume the queue."
        );
        // The queue panel is a warning card holding Resume queue, and names
        // the other chat's item.
        assert_eq!(label(window, "queue-panel-tone"), "Warning");
        assert_eq!(label(window, "queue-resume"), "Resume queue");
        assert_eq!(
            label(window, "queue-label-0"),
            "Waiting · Other chat · Check the venue"
        );
    });
}

#[gpui_kit::test]
fn resume_queue_names_the_waiting_messages_while_the_pending_reply_may_still_run(
    cx: &mut TestAppContext,
) {
    let mut run = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, other| {
        // Claim the pending item before any Working run exists, then start
        // its run the way a run start binds it (one checked group with the
        // queue). Nothing is resumed or dispatched after that.
        workspace
            .queue_message("pending-item", thread, "Draft the delivery update again")
            .unwrap();
        let pending = workspace.dispatch_queued("claim-pending").unwrap().unwrap();
        workspace
            .queue_message("other-item", other, "Check the venue")
            .unwrap();
        let control = workspace.chat_control().unwrap();
        let RecordData::ChatControl(mut queue) = control.data.clone() else {
            unreachable!()
        };
        let key = "bound-working-run";
        run = workspace
            .store
            .allocate_operation(key)
            .unwrap()
            .creation_id(0);
        queue.pending.as_mut().unwrap().run = Some(run.clone());
        let settings = workspace.settings().unwrap();
        workspace
            .owner_change(
                key,
                &ChangeRequest {
                    reason: "Synthetic queued run start".into(),
                    inputs: vec![],
                    writes: vec![
                        Put {
                            id: run.clone(),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Run(Run {
                                instruction: pending.message.clone(),
                                queue_claim: pending.dispatch.clone(),
                                thread: thread.into(),
                                provider: "offline-fixture".into(),
                                model: settings.model,
                                guide_identity: "chat-polish".into(),
                                budget: 1,
                                effort: settings.effort,
                                loaded_skills: vec![],
                                fence: 0,
                                state: RunState::Working,
                                progress: "Started in another window".into(),
                                operations: vec![],
                                sources: vec![],
                            }),
                        },
                        Put {
                            id: control.id.clone(),
                            expected_version: Some(control.version),
                            archived: false,
                            data: RecordData::ChatControl(queue),
                        },
                    ],
                },
            )
            .unwrap();
    });
    let card = format!("recovery-card-{}", f.thread);
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, format!("continue-{run}")), "Resume queue");
        assert_eq!(
            label(window, format!("{card}-state")),
            "May still be running"
        );
        assert_eq!(label(window, format!("{card}-tone")), "Warning");
        assert_eq!(
            label(window, format!("{card}-reason")),
            "Started in another window"
        );
        // Opening BRN keeps a pending item paused until explicit Resume.
        assert_eq!(
            label(window, format!("{card}-next")),
            "Your message is saved and the queue is paused. Resume queue runs 2 waiting messages in order, including 1 from another chat."
        );
        assert!(window.try_find(format!("{card}-waiting")).is_none());
        assert!(window.find(format!("stop-saved-run-{run}")).visible());
        // Only the other chat's item names its chat.
        assert_eq!(
            label(window, "queue-label-0"),
            "Next · Draft the delivery update again"
        );
        assert_eq!(
            label(window, "queue-label-1"),
            "Waiting · Other chat · Check the venue"
        );
    });
    cx.read(|cx| assert!(f.view.read(cx).queue_state().paused));
    assert_eq!(run_state(cx, &f, &run), RunState::Working);
}

#[gpui_kit::test]
fn a_newer_message_reads_not_answered_yet_and_warns_about_an_earlier_working_reply(
    cx: &mut TestAppContext,
) {
    let mut done = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        done = record_run(workspace, thread, RunState::Completed, "Done");
        workspace.post_message(thread, "One more question").unwrap();
    });
    let card = format!("recovery-card-{}", f.thread);
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, format!("{card}-state")), "Not answered yet");
        assert!(window.try_find(format!("{card}-tone")).is_none(), "neutral");
        assert!(window.try_find(format!("{card}-reason")).is_none());
        assert_eq!(
            label(window, format!("{card}-next")),
            "Your message is saved. Continue starts a fresh reply with the saved chat and current notes."
        );
        assert_eq!(label(window, format!("continue-{done}")), "Continue");
    });
    let mut working = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        working = record_run(workspace, thread, RunState::Working, "Started elsewhere");
        workspace.post_message(thread, "One more question").unwrap();
    });
    let card = format!("recovery-card-{}", f.thread);
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, format!("{card}-state")), "Not answered yet");
        assert_eq!(label(window, format!("{card}-tone")), "Warning");
        assert!(window.try_find(format!("{card}-reason")).is_none());
        assert_eq!(
            label(window, format!("{card}-next")),
            "An earlier reply may still be running elsewhere. Continue starts a fresh reply and replaces it."
        );
        assert_eq!(label(window, format!("continue-{working}")), "Continue");
        assert!(window.find(format!("stop-saved-run-{working}")).visible());
    });
    assert_eq!(run_state(cx, &f, &working), RunState::Working);
}

#[gpui_kit::test]
fn a_saved_working_reply_stops_only_through_its_confirming_dialog(cx: &mut TestAppContext) {
    // U1. The seeded Working run has no live worker; this test never
    // resumes or dispatches.
    let mut run = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        run = record_run(
            workspace,
            thread,
            RunState::Working,
            "Started in another window",
        );
    });
    let card = format!("recovery-card-{}", f.thread);
    let stop = format!("stop-saved-run-{run}");
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, format!("{card}-state")),
            "May still be running"
        );
        assert_eq!(
            label(window, format!("{card}-next")),
            "This reply may still be running elsewhere. Continue starts a fresh reply and replaces it."
        );
        assert_eq!(label(window, &stop), "Stop…");
        assert_eq!(label(window, format!("continue-{run}")), "Continue");
    });
    click(cx, &f, &stop);
    on_window(cx, &f, |window, cx| {
        assert!(window.has_active_dialog(cx));
        assert_eq!(
            label(window, "stop-saved-run-body"),
            "BRN can't tell whether another BRN window or the CLI is still running it. Stopping marks it stopped and pauses the queue; changes it tries to save after that are refused. You can Continue afterwards."
        );
        assert_eq!(label(window, "stop-saved-run-confirm"), "Stop reply");
        window.press("enter", cx);
    });
    on_window(cx, &f, |window, cx| {
        assert!(window.has_active_dialog(cx), "↵ never confirms");
    });
    assert_eq!(run_state(cx, &f, &run), RunState::Working);
    click(cx, &f, "stop-saved-run-cancel");
    on_window(cx, &f, |window, cx| assert!(!window.has_active_dialog(cx)));
    assert_eq!(run_state(cx, &f, &run), RunState::Working);
    cx.read(|cx| assert!(!f.view.read(cx).queue_state().paused));
    click(cx, &f, &stop);
    click(cx, &f, "stop-saved-run-confirm");
    on_window(cx, &f, |window, cx| assert!(!window.has_active_dialog(cx)));
    assert_eq!(run_state(cx, &f, &run), RunState::Interrupted);
    cx.read(|cx| {
        let ui = f.view.read(cx);
        assert!(ui.queue_state().paused);
        assert_eq!(
            ui.notice,
            "Reply stopped. The queue is paused; Continue or Resume queue when ready."
        );
    });
    assert_eq!(notice_tone(cx, &f).as_deref(), Some("Warning"));
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, format!("{card}-state")), "Interrupted");
        assert_eq!(label(window, format!("continue-{run}")), "Continue");
        assert!(window.try_find(stop.clone()).is_none());
    });
}

#[gpui_kit::test]
fn a_saved_working_rewrite_offers_the_same_confirmed_stop_in_the_note(cx: &mut TestAppContext) {
    use polish_tests::{fixture, on_window, record_rewrite_run, update};
    let f = fixture(cx, 1250., 860., "Saved text for a rewrite.");
    let thread = update(cx, &f, |view, window, cx| {
        let thread = view.workspace.new_thread("Rewrite chat").unwrap().id;
        view.workspace
            .link_writing_thread("chat-polish-link", &f.note.id, f.note.version, &thread)
            .unwrap();
        view.refresh();
        view.select_writing_thread(f.note.id.clone(), thread.clone(), window, cx);
        // Offline: the request is saved and no AI starts.
        view.rewrite_note(cx);
        thread
    });
    record_rewrite_run(cx, &f, RunState::Working, "Started elsewhere");
    let run = cx.read(|cx| {
        let view = f.view.read(cx);
        latest_run(&view.records, &view.history, &thread)
            .unwrap()
            .id
            .clone()
    });
    let stop = format!("stop-saved-run-{run}");
    on_window(cx, &f, |window, cx| {
        assert_eq!(
            window.find("writing-run-status").label(),
            Some("This rewrite may still be running elsewhere. Retry starts a new attempt.")
        );
        assert!(window.within("tile-note").try_find(stop.clone()).is_some());
        // The chat leaves the writing continuation to the Note.
        assert!(window.try_find(format!("recovery-card-{thread}")).is_none());
        window.click(stop.clone(), cx);
    });
    on_window(cx, &f, |window, cx| {
        assert!(window.has_active_dialog(cx));
        assert!(window.find("stop-saved-run-body").visible());
        window.click("stop-saved-run-confirm", cx);
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        let RecordData::Run(saved) = view.workspace.store.record(&run).unwrap().unwrap().data
        else {
            unreachable!()
        };
        assert_eq!(saved.state, RunState::Interrupted);
        assert!(view.queue_state().paused);
    });
    on_window(cx, &f, |window, _| {
        assert!(
            window
                .find("writing-run-status")
                .label()
                .unwrap()
                .starts_with("Rewrite interrupted")
        );
        assert!(window.try_find(stop.clone()).is_none());
        assert!(window.find("retry-rewrite").visible());
    });
}

#[gpui_kit::test]
fn a_linked_chat_whose_newest_request_is_a_rewrite_leaves_continue_to_the_note(
    cx: &mut TestAppContext,
) {
    use polish_tests::{fixture, on_window, record_rewrite_run, update};
    let f = fixture(cx, 1250., 860., "Saved text for a rewrite.");
    let thread = update(cx, &f, |view, window, cx| {
        let thread = view.workspace.new_thread("Rewrite chat").unwrap().id;
        view.workspace
            .link_writing_thread("chat-polish-link", &f.note.id, f.note.version, &thread)
            .unwrap();
        view.refresh();
        view.select_writing_thread(f.note.id.clone(), thread.clone(), window, cx);
        view.rewrite_note(cx);
        thread
    });
    record_rewrite_run(cx, &f, RunState::Failed, FAILURE);
    on_window(cx, &f, |window, _| {
        assert!(window.find("tile-chat").visible());
        assert!(window.try_find(format!("recovery-card-{thread}")).is_none());
        assert!(
            window
                .within("thread-conversation")
                .try_find(format!("continue-{thread}"))
                .is_none()
        );
        assert!(window.find("retry-rewrite").visible());
        assert_eq!(
            window.find("writing-run-status").label(),
            Some(format!("Rewrite failed · {FAILURE}").as_str())
        );
    });
}

#[gpui_kit::test]
fn review_cards_name_their_target_notes_and_mark_a_protected_one(cx: &mut TestAppContext) {
    // U3: the service's reason stays "Review prepared changes"; the card
    // names the notes from the prepared candidate.
    let mut operation = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        let decision = workspace
            .new_note("Delivery decision: 22 November", "Deliver on 22 November.")
            .unwrap();
        let RecordData::Note(mut protected) = decision.data.clone() else {
            unreachable!()
        };
        protected.protected = true;
        workspace
            .owner_change(
                "protect-decision",
                &ChangeRequest {
                    reason: "Protect the decision".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: decision.id.clone(),
                        expected_version: Some(decision.version),
                        archived: false,
                        data: RecordData::Note(protected),
                    }],
                },
            )
            .unwrap();
        let decision = workspace.store.note(&decision.id).unwrap().unwrap();
        let survey = workspace
            .new_note("Survey", "Survey closes Thursday.")
            .unwrap();
        let put = |record: &Record, markdown: &str| {
            let RecordData::Note(mut note) = record.data.clone() else {
                unreachable!()
            };
            note.markdown = markdown.into();
            Put {
                id: record.id.clone(),
                expected_version: Some(record.version),
                archived: false,
                data: RecordData::Note(note),
            }
        };
        let op = workspace
            .store
            .allocate_operation("delivery-proposal")
            .unwrap();
        workspace
            .store
            .prepare(
                &op,
                &ChangeRequest {
                    reason: "Review prepared changes".into(),
                    inputs: vec![],
                    writes: vec![
                        put(&decision, "Deliver on 23 November."),
                        put(&survey, "Survey closes Friday."),
                    ],
                },
            )
            .unwrap();
        operation = op.as_str().to_owned();
        let current = workspace.store.record(thread).unwrap().unwrap();
        let RecordData::Thread(mut data) = current.data.clone() else {
            unreachable!()
        };
        data.attention = vec![Attention {
            kind: AttentionKind::Review,
            reason: "Review prepared changes".into(),
            record: Some(operation.clone()),
        }];
        workspace
            .owner_change(
                "review-attention",
                &ChangeRequest {
                    reason: "Synthetic review attention".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: thread.into(),
                        expected_version: Some(current.version),
                        archived: false,
                        data: RecordData::Thread(data),
                    }],
                },
            )
            .unwrap();
    });
    let expected = "Review changes to Delivery decision: 22 November and Survey";
    on_window(cx, &f, |window, _| {
        let card = label(window, "attention-card-0");
        assert!(
            card.starts_with(&format!("Needs you · {expected}")),
            "{card}"
        );
        assert!(card.to_lowercase().contains("protected"), "{card}");
        let heading = label(window, format!("proposal-heading-{operation}"));
        assert!(heading.starts_with(expected), "{heading}");
        assert!(heading.ends_with("· Protected"), "{heading}");
        assert_eq!(
            label(window, format!("proposal-protected-{operation}")),
            "Protected"
        );
        assert!(
            window
                .within("attention-card-0")
                .try_find(format!("proposal-{operation}"))
                .is_some()
        );
        assert_eq!(label(window, format!("card-apply-{operation}")), "Approve");
        assert!(window.find(format!("thread-review-{operation}")).visible());
        assert!(window.find(format!("card-dismiss-{operation}")).visible());
    });
    // The service's reason string is unchanged.
    cx.read(|cx| {
        let view = f.view.read(cx);
        let RecordData::Thread(thread) = &view
            .workspace
            .store
            .record(&f.thread)
            .unwrap()
            .unwrap()
            .data
        else {
            unreachable!()
        };
        assert_eq!(thread.attention[0].reason, "Review prepared changes");
    });
}

#[gpui_kit::test]
fn a_question_is_a_warning_card_with_its_reason(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        let current = workspace.store.record(thread).unwrap().unwrap();
        let RecordData::Thread(mut data) = current.data.clone() else {
            unreachable!()
        };
        data.attention = vec![Attention {
            kind: AttentionKind::Question,
            reason: "Choose the delivery date".into(),
            record: None,
        }];
        workspace
            .owner_change(
                "question-attention",
                &ChangeRequest {
                    reason: "Synthetic question".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: thread.into(),
                        expected_version: Some(current.version),
                        archived: false,
                        data: RecordData::Thread(data),
                    }],
                },
            )
            .unwrap();
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "attention-card-0"),
            "Needs you · Choose the delivery date"
        );
        assert_eq!(label(window, "attention-card-0-tone"), "Warning");
    });
}

#[gpui_kit::test]
fn queue_notices_follow_the_queue_state_and_paused_wins_after_stop(cx: &mut TestAppContext) {
    let mut run = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        run = record_run(workspace, thread, RunState::Working, "");
    });
    let send = |cx: &mut TestAppContext, text: &str| {
        let text = text.to_owned();
        update(cx, &f, move |view, window, cx| {
            view.composer
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
            view.ask(window, cx);
            view.notice.clone()
        })
    };
    let sender = update(cx, &f, |view, _, cx| {
        let sender = hold_worker(view, &f.thread, &run);
        cx.notify();
        sender
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "ask-delegate"), "Queue");
    });
    assert_eq!(
        send(cx, "First queued"),
        "Queued; runs after the current reply."
    );
    assert_eq!(notice_tone(cx, &f).as_deref(), Some("Info"));
    assert_eq!(
        send(cx, "Second queued"),
        "Queued; runs after 1 waiting message."
    );
    // Items in this chat do not repeat its name.
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "queue-label-0"), "Waiting · First queued");
        assert_eq!(label(window, "queue-label-1"), "Waiting · Second queued");
        assert!(window.try_find("queue-resume").is_none(), "not paused");
    });
    update(cx, &f, |view, _, _| {
        view.editing_queue = Some(view.queue_state().queue[0].id.clone());
    });
    assert_eq!(send(cx, "First queued, edited"), "Queued message updated.");
    // Stop pauses the queue before the worker finishes: Paused wins.
    update(cx, &f, |view, _, cx| view.cancel(cx));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.worker.is_some() && view.queue_state().paused);
    });
    assert_eq!(
        send(cx, "Third queued"),
        "Queued. The queue is paused; Resume queue to run it."
    );
    assert_eq!(notice_tone(cx, &f).as_deref(), Some("Warning"));
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "queue-panel-tone"), "Warning");
        assert_eq!(
            label(window, "queue-resume"),
            "Resume queue, unavailable: wait for current work to stop"
        );
    });
    update(cx, &f, |view, window, cx| {
        sender
            .send(WorkerEvent::Finished(Ok("Late finished event".into())))
            .unwrap();
        view.poll_worker(window, cx);
        assert!(view.worker.is_none());
        assert_eq!(view.queue_state().queue.len(), 3);
    });
}

#[gpui_kit::test]
fn an_idle_send_clears_an_earlier_queue_notice(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        record_run(workspace, thread, RunState::Completed, "Done");
    });
    let notice = update(cx, &f, |view, window, cx| {
        view.inform("Queued; runs after the current reply.");
        view.composer.update(cx, |editor, cx| {
            editor.set_value("Idle message", window, cx)
        });
        view.ask(window, cx);
        view.notice.clone()
    });
    // Offline, the send is saved and the provider refusal replaces the old
    // notice, which no longer describes the queue.
    assert!(!notice.contains("Queued"), "{notice}");
    assert!(notice.contains("Choose a provider"), "{notice}");
}

#[gpui_kit::test]
fn archive_states_its_consequence_and_an_archived_chat_has_one_unarchive(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |_, _, _| {});
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "resolve-thread"),
            "Archive this chat. Stops work and pauses the queue."
        );
        assert!(window.try_find("chat-archived").is_none());
        assert!(window.try_find("reopen-thread").is_none());
    });
    click(cx, &f, "resolve-thread");
    cx.read(|cx| {
        assert_eq!(
            f.view.read(cx).notice,
            "Chat archived. Work stopped and the queue is paused. Tasks keep their progress."
        )
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "chat-archived"), "Archived");
        assert!(window.try_find("resolve-thread").is_none());
        assert!(window.try_find("chat-unarchive").is_none());
        assert!(
            window
                .within("composer-destination")
                .try_find("reopen-thread")
                .is_some()
        );
    });
    click(cx, &f, "reopen-thread");
    on_window(cx, &f, |window, _| {
        assert!(window.find("resolve-thread").visible());
        assert!(window.try_find("composer-destination").is_none());
    });
}

#[gpui_kit::test]
fn the_composer_is_one_row_and_names_the_effort_in_a_narrow_chat(cx: &mut TestAppContext) {
    let f = fixture(cx, 900., 600., |_, _, _| {});
    let (model, effort) = cx.read(|cx| {
        let settings = &f.view.read(cx).settings;
        (settings.model.clone(), settings.effort.clone())
    });
    on_window(cx, &f, |window, _| {
        let row: Vec<_> = ["composer-add", "model-pill", "chat-quiet", "ask-delegate"]
            .into_iter()
            .map(|id| (id, window.find(id).bounds()))
            .collect();
        let centre = |bounds: &Bounds<gpui_kit::Pixels>| bounds.origin.y + bounds.size.height / 2.;
        for (id, bounds) in &row {
            assert!(
                (centre(bounds) - centre(&row[0].1)).abs() <= px(2.),
                "{id} shares the composer row: {row:?}"
            );
        }
        let chat = window.find("tile-chat").bounds();
        let send = row[3].1;
        assert!(send.origin.x + send.size.width <= chat.origin.x + chat.size.width);
        assert_eq!(
            label(window, "model-pill"),
            format!("Model {model}, effort {effort}")
        );
        assert!(window.try_find("effort-pill").is_none());
        assert_eq!(label(window, "ask-delegate"), "Send");
    });
    // The model menu holds the effort choices and their handler.
    click(cx, &f, "model-pill");
    on_window(cx, &f, |window, cx| {
        let mut menu = window.within("popup-menu");
        let index = (0..60usize)
            .find(|index| {
                menu.try_find(*index)
                    .is_some_and(|row| row.label() == Some("high"))
            })
            .expect("the model menu lists the reasoning efforts");
        menu.click(index, cx);
    });
    cx.read(|cx| assert_eq!(f.view.read(cx).settings.effort, "high"));
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "model-pill"),
            format!("Model {model}, effort high")
        );
    });
}

#[gpui_kit::test]
fn while_work_runs_send_reads_queue_at_a_wide_chat_and_replies_keep_the_measure(
    cx: &mut TestAppContext,
) {
    let mut run = String::new();
    let mut reply = String::new();
    let f = fixture(cx, 1400., 1000., |workspace, thread, _| {
        let op = workspace.store.allocate_operation("chat-reply").unwrap();
        reply = op.creation_id(0);
        workspace
            .owner_change(
                "chat-reply",
                &ChangeRequest {
                    reason: "Synthetic completed answer".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: reply.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Message(Message {
                            thread: thread.into(),
                            role: "assistant".into(),
                            text: "The delivery update is drafted. ".repeat(12),
                        }),
                    }],
                },
            )
            .unwrap();
        run = record_run(workspace, thread, RunState::Working, "");
    });
    update(cx, &f, |view, _, cx| {
        hold_worker(view, &f.thread, &run);
        view.layout.toggle_maximize(TileKind::Chat);
        cx.notify();
    });
    let model = cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(!view.chat_narrow);
        view.settings.model.clone()
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "ask-delegate"), "Queue");
        // No provider is chosen offline: the model alone, no leading "·".
        assert_eq!(label(window, "model-pill"), model);
        assert!(window.find("effort-pill").visible());
        assert!(window.find("chat-steer").visible());
        assert_eq!(label(window, format!("answer-copy-{reply}")), "Copy reply");
        assert_eq!(
            label(window, format!("answer-decision-{reply}")),
            "Save as decision…"
        );
        // The reply's column is at most 680 px of text plus 24 px gutters,
        // centered in the maximized chat.
        let chat = window.find("tile-chat").bounds();
        let copy = window.find(format!("answer-copy-{reply}")).bounds();
        let column_right = chat.origin.x + (chat.size.width + px(728.)) / 2.;
        assert!(chat.size.width > px(1000.), "{chat:?}");
        assert!(
            copy.origin.x + copy.size.width <= column_right + px(1.),
            "{copy:?} within the measure ending at {column_right:?}"
        );
    });
}
