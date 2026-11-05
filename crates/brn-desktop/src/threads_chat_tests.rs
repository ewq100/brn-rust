//! Offline chat witnesses for the UI polish: the recovery card and its
//! recorded reason, the confirmed Stop for a saved Working reply, review
//! cards that name their notes, queue notices, Archive and the composer.
//! Every run is synthetic; pixels and pointer feel remain native acceptance.
//! Fixtures that seed a Working run with no live worker never Resume or
//! Dispatch after seeding it (integration watch item).
use super::*;
use gpui_kit::{Focusable, TestAppContext, test::TestWindowExt};

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
/// The open popup menu's row labels.
fn menu_rows(cx: &mut TestAppContext, f: &Fixture) -> Vec<String> {
    on_window(cx, f, |window, _| {
        let menu = window.within("popup-menu");
        (0..60usize)
            .filter_map(|index| {
                menu.try_find(index)
                    .and_then(|row| row.label().map(str::to_owned))
            })
            .collect()
    })
}
/// Choose a popup menu row by its accessible label, as a mouse user would.
fn choose(cx: &mut TestAppContext, f: &Fixture, label: &str) {
    on_window(cx, f, |window, cx| {
        let mut menu = window.within("popup-menu");
        let index = (0..60usize)
            .find(|index| {
                menu.try_find(*index)
                    .is_some_and(|row| row.label() == Some(label))
            })
            .unwrap_or_else(|| panic!("menu row {label} not found"));
        menu.click(index, cx);
    });
}
/// A synthetic run with its own recorded reason.
pub(super) fn record_run(
    workspace: &mut Workspace,
    thread: &str,
    state: RunState,
    progress: &str,
) -> String {
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
                        loaded_skill_details: vec![],
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
/// A live local worker for the chat's run, as a running reply has. It holds
/// the run's lease as a real worker does; keep the lease for as long as the
/// worker stands for live work, or every stop path treats the run as
/// crashed.
fn hold_worker(
    ui: &mut ThreadsUi,
    thread: &str,
    run: &str,
) -> (mpsc::SyncSender<WorkerEvent>, brn_ai::run_lease::RunLease) {
    let lease =
        brn_ai::run_lease::RunLease::acquire(&ui.workspace.store.derived_directory().unwrap(), run)
            .unwrap();
    let (sender, receiver) = mpsc::sync_channel(16);
    ui.worker = Some(Worker::run(
        receiver,
        CancellationToken::new(),
        thread.into(),
        run.into(),
        None,
    ));
    (sender, lease)
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
    // The assistant at its narrowest beside a document.
    update(cx, &f, |view, window, cx| {
        view.relayout(window, cx, Layout::narrow_assistant)
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
    // Continue is the chat's one filled control: Send is outline beside
    // the card, and filled again in a chat with no card.
    cx.read(|cx| assert!(f.view.read(cx).recovery_card_leads()));
    let other = cx.read(|cx| {
        f.view
            .read(cx)
            .thread_rows
            .iter()
            .find(|row| row.title == "Other chat")
            .unwrap()
            .id
            .clone()
    });
    update(cx, &f, |view, window, cx| {
        view.open_thread(&other, window, cx)
    });
    cx.read(|cx| assert!(!f.view.read(cx).recovery_card_leads()));
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
                                loaded_skill_details: vec![],
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
        // The pending item's run is saved as Working, so Resume queue stops
        // it first and the line beside the button says so.
        assert_eq!(
            label(window, format!("{card}-next")),
            "Your message is saved and the queue is paused. Resume queue stops the reply that may still be running, then runs 2 waiting messages in order, including 1 from another chat."
        );
        assert!(
            label(window, &card).contains("Resume queue stops the reply that may still be running")
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

/// Gives `run` the chat's recorded notice, as the service leaves it when a
/// run ends Failed or Interrupted: a Blocker whose record is that run.
fn record_notice(workspace: &mut Workspace, thread: &str, run: &str, notice: &str) {
    let chat = workspace.store.record(thread).unwrap().unwrap();
    let RecordData::Thread(mut data) = chat.data else {
        unreachable!()
    };
    data.attention = vec![Attention {
        kind: AttentionKind::Blocker,
        reason: notice.into(),
        record: Some(run.into()),
    }];
    workspace
        .owner_change(
            &format!("notice-{}", uuid::Uuid::new_v4()),
            &ChangeRequest {
                reason: "Synthetic terminal notice".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: thread.into(),
                    expected_version: Some(chat.version),
                    archived: false,
                    data: RecordData::Thread(data),
                }],
            },
        )
        .unwrap();
}

/// U10: the recovery card's reason is the chat's recorded notice, the same
/// text Needs you shows; the run's progress is a secondary line.
#[gpui_kit::test]
fn a_failed_reply_reads_as_its_recorded_notice_on_the_card_and_in_needs_you(
    cx: &mut TestAppContext,
) {
    let notice =
        "Run failed: The provider account needs reconnecting. Continue when the provider is ready.";
    let mut run = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        run = record_run(
            workspace,
            thread,
            RunState::Failed,
            "Read the delivery note",
        );
        record_notice(workspace, thread, &run, notice);
    });
    let card = format!("recovery-card-{}", f.thread);
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, format!("{card}-state")), "Failed");
        assert_eq!(label(window, format!("{card}-reason")), notice);
        assert_eq!(
            label(window, format!("{card}-detail")),
            "Read the delivery note"
        );
        assert_eq!(label(window, format!("continue-{run}")), "Continue");
    });
    cx.read(|cx| {
        let ui = f.view.read(cx);
        let needs = lists::sidebar(&ui.thread_rows, None).needs;
        let row = needs.iter().find(|row| row.id == f.thread).unwrap();
        assert_eq!(row.reasons, vec![notice.to_owned()]);
    });
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
            ui.notice.text(),
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
    // Control real-time dialog entrance motion while testing confirmation behavior.
    cx.update(|cx| cx.set_reduce_motion(true));
    let f = fixture(cx, 1250., 860., "Saved text for a rewrite.");
    let thread = update(cx, &f, |view, window, cx| {
        let thread = view.workspace.new_thread("Rewrite chat").unwrap().id;
        view.workspace
            .link_writing_thread("chat-polish-link", &f.note.id, f.note.version, &thread)
            .unwrap();
        view.refresh();
        view.select_writing_thread(f.note.id.clone(), thread.clone(), window, cx);
        // Offline: the request is saved and no AI starts.
        view.rewrite_note(window, cx);
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
fn the_note_leaves_stop_to_the_chat_card_only_while_that_card_shows_it(cx: &mut TestAppContext) {
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
        view.rewrite_note(window, cx);
        thread
    });
    record_rewrite_run(cx, &f, RunState::Working, "Started elsewhere");
    // A later queued message in this chat is the queue's pending item. The
    // saved control is written directly: nothing is resumed or dispatched
    // after the Working run is seeded.
    update(cx, &f, |view, _, cx| {
        view.workspace
            .queue_message("rewrite-chat-queued", &thread, "After the rewrite")
            .unwrap();
        let control = view.workspace.chat_control().unwrap();
        let RecordData::ChatControl(mut queue) = control.data.clone() else {
            unreachable!()
        };
        let mut pending = queue.queue.remove(0);
        pending.dispatch = Some("synthetic-claim".into());
        queue.pending = Some(pending);
        view.workspace
            .owner_change(
                "rewrite-chat-pending",
                &ChangeRequest {
                    reason: "Synthetic pending item".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: control.id.clone(),
                        expected_version: Some(control.version),
                        archived: false,
                        data: RecordData::ChatControl(queue),
                    }],
                },
            )
            .unwrap();
        view.refresh();
        cx.notify();
    });
    let run = cx.read(|cx| {
        let view = f.view.read(cx);
        latest_run(&view.records, &view.history, &thread)
            .unwrap()
            .id
            .clone()
    });
    let stop = format!("stop-saved-run-{run}");
    // The visible chat's recovery card offers Stop… for this run, so the
    // Note does not repeat it.
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("writing-run-status").label(),
            Some("This rewrite may still be running elsewhere. Retry starts a new attempt.")
        );
        assert!(
            window
                .within(format!("recovery-card-{thread}"))
                .try_find(stop.clone())
                .is_some()
        );
        assert!(window.within("tile-note").try_find(stop.clone()).is_none());
    });
    // With the Chat tile hidden the Note offers it.
    update(cx, &f, |view, _, cx| {
        view.layout.close(TileKind::Chat);
        cx.notify();
    });
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("tile-chat").is_none());
        assert!(window.within("tile-note").try_find(stop.clone()).is_some());
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.worker.is_none());
        let RecordData::Run(saved) = view.workspace.store.record(&run).unwrap().unwrap().data
        else {
            unreachable!()
        };
        assert_eq!(saved.state, RunState::Working);
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
        view.rewrite_note(window, cx);
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
            view.notice.text().to_owned()
        })
    };
    let (sender, lease) = update(cx, &f, |view, _, cx| {
        let held = hold_worker(view, &f.thread, &run);
        cx.notify();
        held
    });
    // Running with an empty composer, the one primary control is Stop;
    // with text it is Queue, Stop beside it.
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "run-stop"), "Stop");
        assert!(window.try_find("ask-delegate").is_none());
    });
    update(cx, &f, |view, window, cx| {
        view.composer
            .update(cx, |editor, cx| editor.set_value("Draft", window, cx));
        cx.notify();
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "ask-delegate"), "Queue");
        assert_eq!(label(window, "run-stop"), "Stop");
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
        let item = view.queue_state().queue[0].clone();
        view.editing_queue = Some(QueuedEdit {
            item: item.id,
            thread: item.thread,
        });
    });
    assert_eq!(send(cx, "First queued, edited"), "Queued message updated.");
    // Stop pauses the queue before the worker finishes: Paused wins.
    update(cx, &f, |view, _, cx| view.cancel(cx));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.worker.is_some() && view.queue_state().paused);
        // The owner's Stop of live work adds no notice to the chat.
        let RecordData::Thread(chat) = view
            .workspace
            .store
            .record(&f.thread)
            .unwrap()
            .unwrap()
            .data
        else {
            unreachable!()
        };
        assert!(
            chat.attention
                .iter()
                .all(|item| item.record.as_deref() != Some(run.as_str())),
            "{:?}",
            chat.attention
        );
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
    drop(lease);
}

#[gpui_kit::test]
fn an_idle_send_skips_the_queue_and_replaces_an_earlier_queue_notice(cx: &mut TestAppContext) {
    // An idle, unpaused send starts directly (start_run clears the notice
    // when a run starts); offline the provider refusal replaces it instead.
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        record_run(workspace, thread, RunState::Completed, "Done");
    });
    let notice = update(cx, &f, |view, window, cx| {
        view.inform("Queued; runs after the current reply.");
        view.composer.update(cx, |editor, cx| {
            editor.set_value("Idle message", window, cx)
        });
        view.ask(window, cx);
        view.notice.text().to_owned()
    });
    assert!(!notice.contains("Queued"), "{notice}");
    assert!(notice.contains("Choose a provider"), "{notice}");
    cx.read(|cx| {
        let queue = f.view.read(cx).queue_state();
        assert!(
            queue.pending.is_none() && queue.queue.is_empty(),
            "{queue:?}"
        );
    });
}

#[gpui_kit::test]
fn archive_states_its_consequence_and_an_archived_chat_has_one_unarchive(cx: &mut TestAppContext) {
    // The fixture's owner message is unanswered, so the recovery card shows
    // (shot 25's state) with Continue as its primary.
    let f = fixture(cx, 1250., 860., |_, _, _| {});
    let card = format!("recovery-card-{}", f.thread);
    let next = format!("{card}-next");
    let fresh = "Your message is saved. Continue starts a fresh reply with the saved chat and current notes.";
    let continue_id = format!("continue-{}", f.thread);
    // Archive lives in the conversation's ⋯ menu.
    click(cx, &f, "chat-menu");
    assert!(menu_rows(cx, &f).iter().any(|row| row == "Archive"));
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("chat-archived").is_none());
        assert!(window.try_find("reopen-thread").is_none());
        assert_eq!(label(window, next.clone()), fresh);
        assert_eq!(label(window, continue_id.clone()), "Continue");
    });
    choose(cx, &f, "Archive");
    let archived =
        "Chat archived. Work stopped and the queue is paused. Tasks keep their progress.";
    cx.read(|cx| assert_eq!(f.view.read(cx).notice.text(), archived));
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "chat-archived"), "Archived");
        assert!(window.try_find("chat-unarchive").is_none());
        assert!(
            window
                .within("composer-destination")
                .try_find("reopen-thread")
                .is_some()
        );
        // The card keeps its state word but waits for Unarchive: the service
        // refuses a run in an archived chat, so Continue is unavailable.
        assert_eq!(label(window, format!("{card}-state")), "Not answered yet");
        assert_eq!(label(window, next.clone()), "Unarchive to continue.");
        // Kit snapshots do not expose a Button's disabled state; the label
        // and the click below witness it.
        assert_eq!(
            label(window, continue_id.clone()),
            "Continue, unavailable: unarchive to continue"
        );
    });
    // Clicking the unavailable Continue starts nothing.
    click(cx, &f, continue_id.clone());
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.worker.is_none());
        assert_eq!(view.notice.text(), archived);
    });
    // U11: opening the archived chat never focuses its composer; focus
    // lands on Unarchive, the one next step.
    update(cx, &f, |ui, window, cx| {
        ui.open_thread(&f.thread, window, cx)
    });
    on_window(cx, &f, |window, cx| window.simulate_next_frame(cx));
    on_window(cx, &f, |window, cx| {
        assert!(!f.view.read(cx).composer.focus_handle(cx).is_focused(window));
        assert_eq!(window.find("reopen-thread").focused(), Some(true));
        // One Unarchive: the composer banner's.
        assert!(window.try_find("chat-unarchive").is_none());
    });
    // ⌘L (focus the composer) lands on Unarchive too, from anywhere.
    update(cx, &f, |ui, window, cx| ui.focus_root(window, cx));
    on_window(cx, &f, |window, cx| window.press("cmd-l", cx));
    on_window(cx, &f, |window, cx| window.simulate_next_frame(cx));
    on_window(cx, &f, |window, cx| {
        assert!(!f.view.read(cx).composer.focus_handle(cx).is_focused(window));
        assert_eq!(window.find("reopen-thread").focused(), Some(true));
    });
    click(cx, &f, "reopen-thread");
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("chat-archived").is_none());
        assert!(window.try_find("composer-destination").is_none());
        assert_eq!(label(window, next.clone()), fresh);
        assert_eq!(label(window, continue_id.clone()), "Continue");
    });
    // Unarchived, the chat's landing is its composer again, and so is ⌘L's.
    update(cx, &f, |ui, window, cx| {
        ui.open_thread(&f.thread, window, cx)
    });
    on_window(cx, &f, |window, cx| {
        assert!(f.view.read(cx).composer.focus_handle(cx).is_focused(window));
    });
    update(cx, &f, |ui, window, cx| ui.focus_root(window, cx));
    on_window(cx, &f, |window, cx| window.press("cmd-l", cx));
    on_window(cx, &f, |window, cx| {
        assert!(f.view.read(cx).composer.focus_handle(cx).is_focused(window));
    });
}

#[gpui_kit::test]
fn the_composer_is_one_row_and_names_the_effort_in_a_narrow_chat(cx: &mut TestAppContext) {
    let f = fixture(cx, 900., 600., |_, _, _| {});
    update(cx, &f, |view, window, cx| {
        view.relayout(window, cx, Layout::narrow_assistant)
    });
    let (model, effort) = cx.read(|cx| {
        let settings = &f.view.read(cx).settings;
        (settings.model.clone(), settings.effort.clone())
    });
    on_window(cx, &f, |window, _| {
        let row: Vec<_> = [
            "composer-add",
            "composer-context",
            "model-pill",
            "ask-delegate",
        ]
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
    let _lease = update(cx, &f, |view, window, cx| {
        let (_, lease) = hold_worker(view, &f.thread, &run);
        // The conversation as the primary view (was: maximized chat).
        view.layout.open_primary(TileKind::Chat);
        view.composer
            .update(cx, |editor, cx| editor.set_value("Next step", window, cx));
        cx.notify();
        lease
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
        assert_eq!(label(window, "run-stop"), "Stop");
        // The reply's column is at most the conversation width of text plus
        // 24 px gutters, centered in the primary conversation.
        let chat = window.find("tile-chat").bounds();
        let copy = window.find(format!("answer-copy-{reply}")).bounds();
        let column = px(theme::metrics::CONVERSATION_WIDTH + 48.);
        let column_right = chat.origin.x + (chat.size.width + column) / 2.;
        assert!(chat.size.width > px(1000.), "{chat:?}");
        assert!(
            copy.origin.x + copy.size.width <= column_right + px(1.),
            "{copy:?} within the measure ending at {column_right:?}"
        );
    });
}

/// Below 700 pt the paused queue is one row, "Paused · N waiting" with
/// Resume queue; its disclosure shows the items. Taller windows list them.
/// In the narrow Write companion "Paused" stays whole: the count gives way.
#[gpui_kit::test]
fn a_short_window_collapses_the_paused_queue_to_one_row(cx: &mut TestAppContext) {
    let seed = |workspace: &mut Workspace, _: &str, other: &str| {
        workspace
            .queue_message("short-first", other, "Check the venue")
            .unwrap();
        workspace
            .queue_message("short-second", other, "Book the room")
            .unwrap();
        workspace.pause_queue("short-paused").unwrap();
    };
    let f = fixture(cx, 900., 600., seed);
    update(cx, &f, |view, window, cx| {
        view.relayout(window, cx, Layout::document_with_assistant)
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "queue-summary"), "Paused · 2 waiting");
        assert_eq!(label(window, "queue-resume"), "Resume queue");
        assert_eq!(label(window, "queue-disclosure"), "Show waiting messages");
        assert!(window.try_find("queue-label-0").is_none());
        // One row: the summary, the disclosure and Resume queue line up.
        let (summary, resume) = (
            window.find("queue-summary").bounds(),
            window.find("queue-resume").bounds(),
        );
        assert!(resume.top() < summary.bottom() && summary.top() < resume.bottom());
        // The narrow companion: "Paused" ends before Resume queue, whole.
        assert!(window.find("tile-chat").bounds().size.width < px(400.));
        let state = window.find("queue-summary-state").bounds();
        assert!(state.left() >= summary.left() && state.right() <= resume.left());
        assert!(state.size.width > px(30.), "{state:?}");
    });
    click(cx, &f, "queue-disclosure");
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "queue-label-0"),
            "Waiting · Other chat · Check the venue"
        );
        assert_eq!(
            label(window, "queue-label-1"),
            "Waiting · Other chat · Book the room"
        );
        assert_eq!(label(window, "queue-disclosure"), "Hide waiting messages");
        assert!(window.find("queue-resume").visible());
    });
    let f = fixture(cx, 1250., 860., seed);
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "queue-summary"), "Queue paused · 2 waiting");
        assert!(window.find("queue-summary-state").visible());
        assert!(window.try_find("queue-disclosure").is_none());
        assert_eq!(
            label(window, "queue-label-0"),
            "Waiting · Other chat · Check the venue"
        );
    });
}

/// While a reply runs, a short window's queue is one quiet row, "Queue · N
/// waiting", with no Resume queue; its disclosure lists the waiting items.
/// The run is held by a local worker and nothing resumes or dispatches.
#[gpui_kit::test]
fn a_short_window_keeps_a_running_queue_to_one_quiet_row(cx: &mut TestAppContext) {
    let mut other = String::new();
    let f = fixture(cx, 900., 600., |_, _, chat| other = chat.to_owned());
    // The window is open before the run exists, so startup recovery never
    // sees it; this window's worker holds it.
    let _lease = update(cx, &f, |view, window, cx| {
        let run = record_run(&mut view.workspace, &f.thread, RunState::Working, "");
        let (_, lease) = hold_worker(view, &f.thread, &run);
        view.workspace
            .queue_message("running-first", &other, "Check the venue")
            .unwrap();
        view.refresh();
        view.relayout(window, cx, Layout::document_with_assistant);
        cx.notify();
        lease
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "queue-summary"), "Queue · 1 waiting");
        assert_eq!(label(window, "queue-disclosure"), "Show waiting messages");
        assert!(window.try_find("queue-resume").is_none());
        assert!(window.try_find("queue-label-0").is_none());
        assert!(window.find("queue-summary-state").visible());
    });
    click(cx, &f, "queue-disclosure");
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "queue-label-0"),
            "Waiting · Other chat · Check the venue"
        );
        assert!(window.find("queue-edit-0").visible());
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.worker.is_some(), "the reply keeps running");
        assert!(!view.queue_state().paused);
    });
}

/// Editing one chat's waiting item must not redirect a later send after the
/// owner navigates to another chat. The held local worker keeps both messages
/// waiting and prevents any provider dispatch.
#[gpui_kit::test]
fn editing_a_queued_message_does_not_redirect_the_next_chat_send(cx: &mut TestAppContext) {
    let mut other = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, other_chat| {
        other = other_chat.to_owned();
        workspace
            .queue_message("destination-regression-a", thread, "Original A message")
            .unwrap();
        workspace
            .pause_queue("destination-regression-paused")
            .unwrap();
    });
    let (run, sender, lease) = update(cx, &f, |view, window, cx| {
        let run = record_run(&mut view.workspace, &f.thread, RunState::Working, "");
        let (sender, lease) = hold_worker(view, &f.thread, &run);
        view.refresh();
        view.relayout(window, cx, Layout::document_with_assistant);
        cx.notify();
        (run, sender, lease)
    });

    // Activate the real Edit control for A's queue row, then type its unsaved
    // replacement. Navigating to B must preserve that text as A's draft.
    click(cx, &f, "queue-edit-0");
    update(cx, &f, |view, window, cx| {
        view.composer.update(cx, |editor, cx| {
            editor.set_value("A's edited draft", window, cx)
        });
        view.open_thread(&other, window, cx);
        assert_eq!(view.composer.read(cx).value().as_ref(), "");
        assert_eq!(
            view.composer_drafts
                .text(&ComposerDestination::Thread(f.thread.clone())),
            "A's edited draft"
        );
        view.composer.update(cx, |editor, cx| {
            editor.set_value("B's independent message", window, cx)
        });
    });

    // Use the actual composer action. Since the worker is held locally, this
    // must append a waiting B item and cannot contact a provider.
    click(cx, &f, "ask-delegate");
    cx.read(|cx| {
        let view = f.view.read(cx);
        let queue = view.queue_state().queue;
        assert_eq!(queue[0].text, "Original A message", "{queue:?}");
        assert_eq!(queue.len(), 2, "{queue:?}");
        assert_eq!(queue[0].thread, f.thread);
        assert_eq!(queue[1].thread, other);
        assert_eq!(queue[1].text, "B's independent message");
        assert!(view.queue_state().paused);
        assert!(
            view.worker.is_some(),
            "the synthetic local worker stays held"
        );
        assert!(matches!(
            view.workspace.store.record(&run).unwrap().unwrap().data,
            RecordData::Run(Run {
                state: RunState::Working,
                ..
            })
        ));
    });

    // The edit remains available when returning to A. Saving it updates the
    // original row, while B's independent queued message stays in place.
    update(cx, &f, |view, window, cx| {
        view.open_thread(&f.thread, window, cx);
        assert_eq!(view.composer.read(cx).value().as_ref(), "A's edited draft");
    });
    click(cx, &f, "ask-delegate");
    cx.read(|cx| {
        let view = f.view.read(cx);
        let queue = view.queue_state().queue;
        assert_eq!(queue.len(), 2, "{queue:?}");
        assert_eq!(queue[0].thread, f.thread);
        assert_eq!(queue[0].text, "A's edited draft");
        assert_eq!(queue[1].thread, other);
        assert_eq!(queue[1].text, "B's independent message");
    });
    drop(sender);
    drop(lease);
}

/// CHAT-01 with the real editor and key bindings: Shift-↵ adds a line, ↵
/// queues the whole text once while work runs and clears the composer once,
/// and ↵ during an IME composition never submits.
#[gpui_kit::test]
fn shift_enter_adds_a_line_enter_queues_once_and_ime_composition_never_submits(
    cx: &mut TestAppContext,
) {
    let mut run = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        run = record_run(workspace, thread, RunState::Working, "");
    });
    let (_sender, _lease) = update(cx, &f, |view, window, cx| {
        let held = hold_worker(view, &f.thread, &run);
        view.composer.focus_handle(cx).focus(window, cx);
        cx.notify();
        held
    });
    cx.simulate_input(f.window, "Line one");
    cx.simulate_keystrokes(f.window, "shift-enter");
    cx.simulate_input(f.window, "Line two");
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.composer.read(cx).value().as_ref(),
            "Line one\nLine two"
        );
        assert!(view.queue_state().queue.is_empty(), "Shift-↵ never submits");
    });
    cx.simulate_keystrokes(f.window, "enter");
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        let queue = view.queue_state().queue;
        assert_eq!(queue.len(), 1, "{queue:?}");
        assert_eq!(queue[0].text, "Line one\nLine two");
        assert_eq!(view.composer.read(cx).value().as_ref(), "");
    });
    // An active composition owns ↵.
    use gpui_kit::EntityInputHandler;
    update(cx, &f, |view, window, cx| {
        view.composer.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "かな", None, window, cx)
        });
    });
    cx.simulate_keystrokes(f.window, "enter");
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.queue_state().queue.len(),
            1,
            "composition did not submit"
        );
    });
}

/// ⌘↵ in the composer sends through the window's Ask binding exactly once.
#[gpui_kit::test]
fn command_enter_in_the_composer_queues_exactly_once(cx: &mut TestAppContext) {
    let mut run = String::new();
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        run = record_run(workspace, thread, RunState::Working, "");
    });
    let (_sender, _lease) = update(cx, &f, |view, window, cx| {
        let held = hold_worker(view, &f.thread, &run);
        view.composer.focus_handle(cx).focus(window, cx);
        held
    });
    cx.simulate_input(f.window, "Through the shortcut");
    cx.simulate_keystrokes(f.window, "cmd-enter");
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        let queue = view.queue_state().queue;
        assert_eq!(queue.len(), 1, "{queue:?}");
        assert_eq!(queue[0].text, "Through the shortcut");
        assert_eq!(view.composer.read(cx).value().as_ref(), "");
    });
}

/// CHAT-02: a refused send leaves the draft in its chat, posts nothing and
/// names the retry; the composer clears only after the service accepts.
#[gpui_kit::test]
fn a_refused_send_keeps_the_text_and_names_the_retry(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        workspace.archive_chat("archive:test", thread).unwrap();
    });
    let before = cx.read(|cx| f.view.read(cx).workspace.messages(&f.thread).unwrap().len());
    update(cx, &f, |view, window, cx| {
        view.composer.update(cx, |editor, cx| {
            editor.set_value("Keep this draft", window, cx)
        });
        view.ask(window, cx);
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.composer.read(cx).value().as_ref(), "Keep this draft");
        assert!(view.worker.is_none());
        assert_eq!(view.workspace.messages(&f.thread).unwrap().len(), before);
        let notice = view.notice.text();
        assert!(notice.starts_with("Could not send: "), "{notice}");
        assert!(
            notice.ends_with("Your text is preserved; Send again to retry."),
            "{notice}"
        );
    });
}

/// Waiting messages reorder with Move up / Move down controls (no drag),
/// and editing one keeps the composer's own draft: it comes back after
/// Save queued and after Cancel edit.
#[gpui_kit::test]
fn queue_items_move_down_and_up_and_editing_keeps_the_composer_draft(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        for (key, text) in [("q-a", "Alpha"), ("q-b", "Bravo"), ("q-c", "Charlie")] {
            workspace.queue_message(key, thread, text).unwrap();
        }
        workspace.pause_queue("q-paused").unwrap();
    });
    let order = |cx: &mut TestAppContext| -> Vec<String> {
        cx.read(|cx| {
            f.view
                .read(cx)
                .queue_state()
                .queue
                .into_iter()
                .map(|q| q.text)
                .collect()
        })
    };
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "queue-down-0"), "Move down");
        assert!(window.try_find("queue-up-0").is_none());
        assert!(window.try_find("queue-down-2").is_none());
    });
    click(cx, &f, "queue-down-0");
    assert_eq!(order(cx), ["Bravo", "Alpha", "Charlie"]);
    click(cx, &f, "queue-down-1");
    assert_eq!(order(cx), ["Bravo", "Charlie", "Alpha"]);
    click(cx, &f, "queue-up-2");
    assert_eq!(order(cx), ["Bravo", "Alpha", "Charlie"]);
    // Edit keeps the unsent draft aside.
    update(cx, &f, |view, window, cx| {
        view.composer.update(cx, |editor, cx| {
            editor.set_value("My unsent draft", window, cx)
        });
    });
    click(cx, &f, "queue-edit-0");
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "ask-delegate"), "Save queued");
        assert!(window.find("queue-edit-cancel").visible());
    });
    cx.read(|cx| assert_eq!(f.view.read(cx).composer.read(cx).value().as_ref(), "Bravo"));
    click(cx, &f, "queue-edit-cancel");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.composer.read(cx).value().as_ref(), "My unsent draft");
        assert!(view.editing_queue.is_none());
    });
    assert_eq!(order(cx), ["Bravo", "Alpha", "Charlie"]);
    click(cx, &f, "queue-edit-1");
    update(cx, &f, |view, window, cx| {
        view.composer.update(cx, |editor, cx| {
            editor.set_value("Alpha, edited", window, cx)
        });
    });
    click(cx, &f, "ask-delegate");
    assert_eq!(order(cx), ["Bravo", "Alpha, edited", "Charlie"]);
    cx.read(|cx| {
        assert_eq!(
            f.view.read(cx).composer.read(cx).value().as_ref(),
            "My unsent draft"
        )
    });
}

/// The context chip names the scope retrieval actually uses (spec §5): the
/// saved library for a general chat; a project's documents plus the library
/// when the project uses BRN knowledge, or only the project when it does
/// not; private attachments are named as private.
#[gpui_kit::test]
fn the_context_chip_names_the_real_scope(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |_, _, _| {});
    let scope = |cx: &mut TestAppContext| cx.read(|cx| f.view.read(cx).context_scope());
    let general = scope(cx);
    assert_eq!(general.label, "Saved library");
    assert!(
        general
            .lines
            .iter()
            .any(|line| line.starts_with("Your saved library"))
    );
    assert!(
        general
            .lines
            .iter()
            .any(|line| line.starts_with("Not used:"))
    );
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "composer-context"), "Context: Saved library");
    });
    let project = update(cx, &f, |view, _, _| {
        let project = view
            .workspace
            .project_command(
                "chip-project",
                ProjectCommand::Create {
                    title: "Atlas".into(),
                },
            )
            .unwrap();
        view.refresh();
        project
    });
    let _ = project;
    let project_id = cx.read(|cx| {
        f.view
            .read(cx)
            .records
            .iter()
            .find(|r| matches!(r.data, RecordData::Project(_)))
            .unwrap()
            .id
            .clone()
    });
    update(cx, &f, |view, _, _| {
        view.workspace
            .project_command(
                "chip-move",
                ProjectCommand::MoveChat {
                    chat: f.thread.clone(),
                    project: Some(project_id.clone()),
                },
            )
            .unwrap();
        view.refresh();
    });
    let inside = scope(cx);
    assert_eq!(inside.label, "Project Atlas + library");
    assert_eq!(inside.project.as_deref(), Some(project_id.as_str()));
    assert!(
        inside
            .lines
            .iter()
            .any(|line| line.starts_with("Project Atlas: "))
    );
    assert!(
        inside
            .lines
            .iter()
            .any(|line| line.starts_with("Your saved library"))
    );
    update(cx, &f, |view, _, _| {
        let record = view.workspace.store.record(&project_id).unwrap().unwrap();
        let RecordData::Project(data) = record.data else {
            unreachable!()
        };
        view.workspace
            .project_command(
                "chip-knowledge-off",
                ProjectCommand::Configure {
                    project: project_id.clone(),
                    version: record.version,
                    title: data.title,
                    instructions: data.instructions,
                    use_brn_knowledge: false,
                },
            )
            .unwrap();
        view.refresh();
    });
    let only = scope(cx);
    assert_eq!(only.label, "Project Atlas");
    assert!(
        only.lines
            .iter()
            .any(|line| line.starts_with("Saved library: off for this project")),
        "{only:?}"
    );
    assert!(
        !only
            .lines
            .iter()
            .any(|line| line.starts_with("Your saved library"))
    );
}

/// Keep in BRN shows its destination and asks first; Cancel changes
/// nothing, and confirming promotes the same note out of the chat (§6.8).
#[gpui_kit::test]
fn keep_in_brn_names_the_destination_before_it_promotes(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        let converted = ConversionResult {
            intent: ImportIntent::FullNote,
            markdown: "# Venue contract\nSynthetic attached document.".into(),
            assets: vec![],
            source_sha256: [3; 32],
            source_reference: "/missing/venue-contract.md".into(),
            converter: brn_threads_intake::document_conversion_label("xberg", false).unwrap(),
            sources: vec![],
            coverage: brn_threads_intake::Coverage {
                status: brn_threads_intake::CoverageStatus::Unchecked,
                checked_items: vec![],
                gaps: vec![],
            },
        };
        workspace
            .attach_full("keep-attachment", thread, "Venue contract", &converted)
            .unwrap();
    });
    let note = cx.read(|cx| {
        f.view
            .read(cx)
            .records
            .iter()
            .find(|r| matches!(&r.data, RecordData::Note(n) if n.chat.as_deref() == Some(f.thread.as_str())))
            .unwrap()
            .id
            .clone()
    });
    let chat_of = |cx: &mut TestAppContext| {
        cx.read(|cx| {
            match &f
                .view
                .read(cx)
                .workspace
                .store
                .note(&note)
                .unwrap()
                .unwrap()
                .data
            {
                RecordData::Note(n) => n.chat.clone(),
                _ => unreachable!(),
            }
        })
    };
    click(cx, &f, format!("attachment-keep-{note}"));
    on_window(cx, &f, |window, _| {
        let body = label(window, "keep-in-brn-body");
        assert!(body.contains("into your saved library"), "{body}");
        assert!(window.find("keep-in-brn-confirm").visible());
    });
    click(cx, &f, "keep-in-brn-cancel");
    assert_eq!(chat_of(cx).as_deref(), Some(f.thread.as_str()));
    click(cx, &f, format!("attachment-keep-{note}"));
    click(cx, &f, "keep-in-brn-confirm");
    assert_eq!(chat_of(cx), None);
    on_window(cx, &f, |window, _| {
        assert!(window.try_find(format!("attachment-keep-{note}")).is_none());
    });
}

/// New chat is a calm start: a short greeting, the composer, at most three
/// starting actions and, while unchosen, the delegation choice (§4.3).
#[gpui_kit::test]
fn the_new_chat_start_has_a_greeting_the_composer_three_actions_and_the_delegation_choice(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, 1250., 860., |workspace, _, _| {
        let mut settings = workspace.settings().unwrap();
        settings.delegation_chosen = false;
        workspace.configure(settings).unwrap();
    });
    update(cx, &f, |view, window, cx| view.new_thread(window, cx));
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "chat-start-heading"),
            "What are we working on?"
        );
        assert!(window.find("composer-box").visible());
        let actions = window.find("chat-start-actions").bounds();
        for id in ["start-attach", "start-new-document", "start-web"] {
            let bounds = window.find(id).bounds();
            assert!(bounds.top() >= actions.top() && bounds.bottom() <= actions.bottom());
        }
        assert!(window.find("delegation-pending").visible());
        // The composer precedes the actions, below the greeting.
        let heading = window.find("chat-start-heading").bounds();
        let composer = window.find("composer-box").bounds();
        assert!(heading.bottom() <= composer.top() && composer.bottom() <= actions.top());
    });
}

/// Activity is a per-conversation disclosure: closed by default, it names
/// the last recorded run with the provider, model and effort that run saved.
#[gpui_kit::test]
fn the_activity_disclosure_names_the_last_recorded_run(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |workspace, thread, _| {
        record_run(
            workspace,
            thread,
            RunState::Completed,
            "Answered from 2 notes",
        );
    });
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("chat-activity-panel").is_none());
    });
    click(cx, &f, "chat-activity");
    let (model, effort) = cx.read(|cx| {
        let settings = &f.view.read(cx).settings;
        (settings.model.clone(), settings.effort.clone())
    });
    on_window(cx, &f, |window, _| {
        let last = label(window, "chat-last-work");
        assert!(last.starts_with("Last reply: Completed"), "{last}");
        assert!(
            last.contains(&model) && last.contains(&format!("{effort} effort")),
            "{last}"
        );
        assert!(last.ends_with("Answered from 2 notes"), "{last}");
        assert!(window.find("chat-quiet").visible());
    });
    click(cx, &f, "chat-activity");
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("chat-activity-panel").is_none());
    });
}

/// CHAT-04: a reader who scrolled up keeps their place while the reply
/// streams; the conversation stops following the tail until Jump to latest.
#[gpui_kit::test]
fn streaming_keeps_the_reading_position_when_scrolled_up(cx: &mut TestAppContext) {
    let mut run = String::new();
    let f = fixture(cx, 1250., 700., |workspace, thread, _| {
        for index in 0..12 {
            let key = format!("long-{index}");
            let op = workspace.store.allocate_operation(&key).unwrap();
            workspace
                .owner_change(
                    &key,
                    &ChangeRequest {
                        reason: "Synthetic long answer".into(),
                        inputs: vec![],
                        writes: vec![Put {
                            id: op.creation_id(0),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Message(Message {
                                thread: thread.into(),
                                role: "assistant".into(),
                                text: format!("Answer {index}. ").repeat(60),
                            }),
                        }],
                    },
                )
                .unwrap();
        }
        run = record_run(workspace, thread, RunState::Working, "");
    });
    let _held = update(cx, &f, |view, _, cx| {
        let held = hold_worker(view, &f.thread, &run);
        view.stream = "Streaming start".into();
        cx.notify();
        held
    });
    on_window(cx, &f, |window, cx| window.simulate_next_frame(cx));
    let state = cx.read(|cx| f.view.read(cx).scrollers.borrow()[&f.thread].state.clone());
    cx.read(|cx| assert!(state.read(cx).is_following_tail()));
    on_window(cx, &f, |window, cx| {
        window.scroll(
            "chat-tile",
            gpui_kit::ScrollDelta::Pixels(point(px(0.), px(900.))),
            cx,
        );
    });
    on_window(cx, &f, |window, cx| window.simulate_next_frame(cx));
    let before = cx.read(|cx| {
        let state = state.read(cx);
        assert!(
            state.is_scrolled_up(),
            "the reader scrolled away from the tail"
        );
        state.logical_scroll_top()
    });
    for chunk in 0..3 {
        update(cx, &f, |view, _, cx| {
            view.stream
                .push_str(&format!(" more streamed text {chunk}").repeat(20));
            cx.notify();
        });
        on_window(cx, &f, |window, cx| window.simulate_next_frame(cx));
    }
    cx.read(|cx| {
        let state = state.read(cx);
        assert!(state.is_scrolled_up());
        assert_eq!(
            state.logical_scroll_top(),
            before,
            "streaming did not move the reader"
        );
    });
    // Jump to latest is named and returns the reader to the live tail.
    let jump = gpui_kit::ElementId::from((
        gpui_kit::ElementId::from(format!("conversation-{}", f.thread)),
        "jump-to-latest",
    ));
    on_window(cx, &f, |window, cx| {
        assert_eq!(window.find(jump.clone()).label(), Some("Jump to latest"));
        window.click(jump, cx);
    });
    on_window(cx, &f, |window, cx| window.simulate_next_frame(cx));
    cx.read(|cx| assert!(state.read(cx).is_following_tail()));
}
