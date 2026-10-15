//! Omakase shell journeys with the real headless GPUI harness, synthetic data
//! and no provider. Native pointer feel, pixels and IME remain owner checks.
use super::*;
use gpui_kit::{Focusable, TestAppContext, test::TestWindowExt};
use std::rc::Rc;

struct Fixture {
    _root: tempfile::TempDir,
    data: PathBuf,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
}

fn fixture(cx: &mut TestAppContext, seed: impl FnOnce(&mut Workspace)) -> Fixture {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("omakase");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    seed(&mut workspace);
    cx.update(gpui_kit::init);
    cx.update(|cx| cx.bind_keys(bindings()));
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1400.), px(1000.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut view = ThreadsUi::new(workspace, data.clone(), window, cx);
                    view.poll = None;
                    view
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
fn press(cx: &mut TestAppContext, f: &Fixture, key: &str) {
    on_window(cx, f, |window, cx| window.press(key, cx));
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
fn kinds(cx: &mut TestAppContext, f: &Fixture) -> Vec<TileKind> {
    cx.read(|cx| {
        f.view
            .read(cx)
            .layout
            .visible()
            .iter()
            .map(|tile| tile.kind)
            .collect()
    })
}
fn thread_state(view: &ThreadsUi, id: &str) -> ThreadState {
    match view.workspace.store.record(id).unwrap().unwrap().data {
        RecordData::Thread(thread) => thread.state,
        _ => unreachable!(),
    }
}
fn record_run(workspace: &mut Workspace, thread: &str, state: RunState) -> String {
    let settings = workspace.settings().unwrap();
    let op = workspace
        .store
        .allocate_operation(&format!("run-{thread}"))
        .unwrap();
    let run = op.creation_id(0);
    workspace
        .owner_change(
            &format!("run-{thread}"),
            &ChangeRequest {
                reason: "Synthetic run".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: run.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Run(Run {
                        instruction: None,
                        queue_claim: None,
                        thread: thread.into(),
                        provider: settings.provider.clone(),
                        model: settings.model.clone(),
                        guide_identity: "fixture".into(),
                        budget: 4,
                        effort: settings.effort.clone(),
                        loaded_skills: vec![],
                        fence: 1,
                        state,
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

#[gpui_kit::test]
fn layouts_resize_swap_maximize_and_tile_menu_work_by_mouse_and_keys(cx: &mut TestAppContext) {
    let f = fixture(cx, |_| {});
    assert_eq!(
        kinds(cx, &f),
        [TileKind::Threads, TileKind::Note, TileKind::Chat]
    );
    on_window(cx, &f, |window, _| {
        assert!(window.find("layout-write").visible());
    });
    assert_eq!(
        cx.read(|cx| f.view.read(cx).layout.preset()),
        Some(Preset::Write)
    );
    click(cx, &f, "layout-chat");
    assert_eq!(
        kinds(cx, &f),
        [TileKind::Threads, TileKind::Chat, TileKind::Linked]
    );
    press(cx, &f, "alt-1");
    assert_eq!(
        kinds(cx, &f),
        [TileKind::Threads, TileKind::Note, TileKind::Chat]
    );

    // Drag the first gap to the right: the threads tile grows, others keep order.
    let before = cx.read(|cx| f.view.read(cx).layout.visible()[0].weight);
    on_window(cx, &f, |window, cx| {
        let gap = window.find("split-0").bounds();
        let from = gap.center();
        window.drag(from, point(from.x + px(220.), from.y), cx);
    });
    let after = cx.read(|cx| f.view.read(cx).layout.visible()[0].weight);
    assert!(after > before, "{before} -> {after}");
    assert_eq!(
        cx.read(|cx| f.view.read(cx).layout.preset()),
        Some(Preset::Write)
    );

    // Drag a header onto another tile to swap them.
    on_window(cx, &f, |window, cx| {
        window.drag_to("tile-header-threads", "tile-chat", cx)
    });
    assert_eq!(
        kinds(cx, &f),
        [TileKind::Chat, TileKind::Note, TileKind::Threads]
    );

    // Double-click maximizes and restores.
    on_window(cx, &f, |window, cx| {
        window.double_click("tile-header-note", cx)
    });
    assert_eq!(kinds(cx, &f), [TileKind::Note]);
    on_window(cx, &f, |window, _| {
        assert!(
            window
                .find("status-layout")
                .label()
                .unwrap()
                .contains("max")
        );
    });
    on_window(cx, &f, |window, cx| {
        window.double_click("tile-header-note", cx)
    });
    assert_eq!(kinds(cx, &f).len(), 3);
    click(cx, &f, "tile-max-chat");
    assert_eq!(kinds(cx, &f), [TileKind::Chat]);
    press(cx, &f, "cmd-shift-enter");
    assert_eq!(kinds(cx, &f).len(), 3);

    // Right-click menu: move and close; reopen through the layout menu.
    on_window(cx, &f, |window, cx| {
        window.right_click("tile-header-threads", cx)
    });
    choose(cx, &f, "Move left");
    assert_eq!(
        kinds(cx, &f),
        [TileKind::Chat, TileKind::Threads, TileKind::Note]
    );
    on_window(cx, &f, |window, cx| {
        window.right_click("tile-header-threads", cx)
    });
    choose(cx, &f, "Close");
    assert!(!kinds(cx, &f).contains(&TileKind::Threads));
    press(cx, &f, "cmd-\\");
    assert!(kinds(cx, &f).contains(&TileKind::Threads));
    click(cx, &f, "tile-close-threads");
    click(cx, &f, "status-layout");
    choose(cx, &f, "chats");
    assert!(kinds(cx, &f).contains(&TileKind::Threads));
    click(cx, &f, "settings");
    assert_eq!(
        cx.read(|cx| f.view.read(cx).layout.focus()),
        Some(TileKind::Settings)
    );
    press(cx, &f, "escape");
    let arrangement = kinds(cx, &f);
    press(cx, &f, "alt-3");
    assert_eq!(kinds(cx, &f).len(), 1);
    press(cx, &f, "alt-3");
    assert_eq!(kinds(cx, &f), arrangement);
    press(cx, &f, "alt-2");
    assert_eq!(
        kinds(cx, &f),
        [TileKind::Threads, TileKind::Chat, TileKind::Linked]
    );
    update(cx, &f, |view, _, _| {
        view.layout.set_focus(TileKind::Threads)
    });
    press(cx, &f, "cmd-]");
    assert_eq!(
        cx.read(|cx| f.view.read(cx).layout.focus()),
        Some(TileKind::Chat)
    );
    on_window(cx, &f, |window, cx| {
        assert!(
            f.view.read(cx).composer.focus_handle(cx).is_focused(window),
            "focusing the chat tile focuses its composer"
        );
    });
}

#[gpui_kit::test]
fn themes_switch_from_status_bar_launcher_and_settings_and_wait_for_running_work(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, |_| {});
    assert_eq!(theme::pal(), ThemeId::Sammal.palette());
    click(cx, &f, "status-theme");
    choose(cx, &f, "Raba · dark bog peat · red");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.theme, ThemeId::Raba);
        assert!(!view.theme_pending);
        assert_eq!(
            view.workspace.settings().unwrap().theme.as_deref(),
            Some("raba")
        );
    });
    assert_eq!(theme::pal().accent, 0xdf6d66);
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("status-theme").label(), Some("Raba"));
    });
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, cx| window.input("paber", cx));
    press(cx, &f, "enter");
    assert_eq!(cx.read(|cx| f.view.read(cx).theme), ThemeId::Paber);
    assert!(!cx.read(|cx| f.view.read(cx).launcher_open));
    update(cx, &f, |view, _, cx| {
        focus_tests::settings(view, SettingsSection::General, cx)
    });
    click(cx, &f, "settings-theme-virmalised");
    assert_eq!(theme::pal(), ThemeId::Virmalised.palette());

    // A working run is bound to the Settings version, so saving waits.
    let thread = update(cx, &f, |view, _, _| {
        view.workspace.new_thread("Busy").unwrap().id
    });
    let run = update(cx, &f, |view, _, _| {
        record_run(&mut view.workspace, &thread, RunState::Working)
    });
    click(cx, &f, "settings-theme-laanemeri");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.theme, ThemeId::Laanemeri, "applied immediately");
        assert!(view.theme_pending);
        assert_eq!(
            view.workspace.settings().unwrap().theme.as_deref(),
            Some("virmalised")
        );
        assert!(view.notice.contains("once no work is running"));
        assert!(matches!(
            view.workspace.store.record(&run).unwrap().unwrap().data,
            RecordData::Run(Run {
                state: RunState::Working,
                ..
            })
        ));
    });
    update(cx, &f, |view, window, cx| {
        view.resolve_thread(&thread, cx);
        view.choose_theme(ThemeId::Laanemeri, window, cx);
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(!view.theme_pending);
        assert_eq!(
            view.workspace.settings().unwrap().theme.as_deref(),
            Some("laanemeri")
        );
    });
    // Reopening the workspace restores the saved theme.
    let reopened = Workspace::open(&f.data).unwrap();
    assert_eq!(
        ThemeId::from_key(reopened.settings().unwrap().theme.as_deref()),
        ThemeId::Laanemeri
    );
}

#[gpui_kit::test]
fn archived_chats_leave_the_sidebar_stay_findable_and_require_unarchive(cx: &mut TestAppContext) {
    let mut ids = vec![];
    let f = fixture(cx, |workspace| {
        for i in 0..13 {
            ids.push(
                workspace
                    .new_thread(&format!("Open question {i:02}"))
                    .unwrap()
                    .id,
            );
        }
    });
    let done = ids[0].clone();
    update(cx, &f, |view, window, cx| {
        view.open_thread(&done, window, cx);
        view.resolve_thread(&done, cx);
        view.new_thread(window, cx);
    });
    on_window(cx, &f, |window, _| {
        assert!(
            window.try_find(format!("thread-{done}")).is_none(),
            "resolved never listed inline"
        );
        assert_eq!(window.find("resolved-count").label(), Some("Archived · 1"));
        assert_eq!(window.find("all-threads").label(), Some("All · 13"));
        let shown = ids
            .iter()
            .filter(|id| window.try_find(format!("thread-{id}")).is_some())
            .count();
        assert_eq!(shown, lists::SIDEBAR_OPEN);
    });
    click(cx, &f, "resolved-count");
    on_window(cx, &f, |window, _| {
        assert!(window.find(format!("all-thread-{done}")).visible());
        assert!(window.try_find(format!("all-thread-{}", ids[1])).is_none());
    });
    click(cx, &f, "all-threads");
    on_window(cx, &f, |window, cx| {
        f.view
            .read(cx)
            .threads_query
            .clone()
            .update(cx, |input, cx| input.focus(window, cx));
        window.input("question 07", cx);
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find(format!("all-thread-{}", ids[7])).visible());
        assert!(window.try_find(format!("all-thread-{}", ids[8])).is_none());
    });
    // The launcher includes resolved threads.
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, cx| window.input("question 00", cx));
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("launcher-item-0").label(),
            Some("chat · Open question 00 · archived")
        );
    });
    click(cx, &f, "launcher-item-0");
    on_window(cx, &f, |window, _| {
        assert!(window.find("reopen-thread").visible());
        assert!(
            window
                .find("composer-destination")
                .label()
                .unwrap()
                .contains("Unarchive to reply")
        );
    });
    update(cx, &f, |view, window, cx| {
        view.composer.update(cx, |editor, cx| {
            editor.set_value("One more thing", window, cx)
        });
        view.ask(window, cx);
        assert!(
            view.workspace
                .store
                .record(&done)
                .unwrap()
                .unwrap()
                .archived
        );
        assert!(view.notice.contains("Unarchive"));
    });
    click(cx, &f, "reopen-thread");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(
            !view
                .workspace
                .store
                .record(&done)
                .unwrap()
                .unwrap()
                .archived
        );
        assert!(view.queue_state().paused);
    });
    click(cx, &f, "resolve-thread");
    cx.read(|cx| {
        assert!(
            f.view
                .read(cx)
                .workspace
                .store
                .record(&done)
                .unwrap()
                .unwrap()
                .archived
        )
    });
    click(cx, &f, "reopen-thread");
    cx.read(|cx| {
        assert!(
            !f.view
                .read(cx)
                .workspace
                .store
                .record(&done)
                .unwrap()
                .unwrap()
                .archived
        )
    });
}

fn finish_run(cx: &mut TestAppContext, f: &Fixture, thread: &str) {
    update(cx, f, |view, window, cx| {
        let (sender, receiver) = mpsc::sync_channel(4);
        view.worker = Some(Worker {
            receiver,
            cancel: CancellationToken::new(),
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            kind: WorkerKind::Run,
            thread: Some(thread.into()),
            run_id: None,
        });
        sender
            .send(WorkerEvent::Finished(Ok("Finished.".into())))
            .unwrap();
        view.poll_worker(window, cx);
    });
}

#[gpui_kit::test]
fn completed_chats_remain_in_history_without_auto_resolving(cx: &mut TestAppContext) {
    let mut quick = String::new();
    let mut linked_thread = String::new();
    let mut twice = String::new();
    let mut failed = String::new();
    let mut attention = String::new();
    let f = fixture(cx, |workspace| {
        let note = workspace.new_note("Linked plan", "Plan text").unwrap();
        for (name, slot) in [
            ("What is BRN?", &mut quick),
            ("About the plan", &mut linked_thread),
            ("Two questions", &mut twice),
            ("Failed run", &mut failed),
        ] {
            let thread = workspace.new_thread(name).unwrap().id;
            workspace.post_message(&thread, name).unwrap();
            *slot = thread;
        }
        workspace
            .link_writing_thread("plan-link", &note.id, note.version, &linked_thread)
            .unwrap();
        workspace.post_message(&twice, "And another?").unwrap();
        let asked = workspace.new_thread("Needs a decision").unwrap();
        workspace.post_message(&asked.id, "Which date?").unwrap();
        workspace
            .owner_change(
                "attention-fixture",
                &ChangeRequest {
                    reason: "Synthetic attention".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: asked.id.clone(),
                        expected_version: Some(asked.version),
                        archived: false,
                        data: RecordData::Thread(Thread {
                            private_context: false,
                            context_epoch: 0,
                            title: "Needs a decision".into(),
                            state: ThreadState::Open,
                            attention: vec![Attention {
                                kind: AttentionKind::Question,
                                reason: "Choose the renewal date".into(),
                                record: None,
                            }],
                        }),
                    }],
                },
            )
            .unwrap();
        record_run(workspace, &asked.id, RunState::Completed);
        attention = asked.id;
        for thread in [&quick, &linked_thread, &twice] {
            record_run(workspace, thread, RunState::Completed);
        }
        record_run(workspace, &failed, RunState::Failed);
    });
    for thread in [&quick, &linked_thread, &twice, &failed, &attention] {
        finish_run(cx, &f, thread);
    }
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(thread_state(view, &quick), ThreadState::Open);
        assert!(
            !view
                .workspace
                .store
                .record(&quick)
                .unwrap()
                .unwrap()
                .archived
        );
        for thread in [&linked_thread, &twice, &failed, &attention] {
            assert_eq!(thread_state(view, thread), ThreadState::Open, "{thread}");
        }
    });
    update(cx, &f, |view, window, cx| {
        view.open_thread(&quick, window, cx)
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find(format!("thread-{quick}")).visible());
    });
}

#[gpui_kit::test]
fn notes_tile_sections_letters_and_filter_stay_bounded_with_many_notes(cx: &mut TestAppContext) {
    let f = fixture(cx, |workspace| {
        for i in 0..240 {
            let prefix = ["Alpha", "Beta", "Õun"][i % 3];
            let note = workspace
                .new_note(&format!("{prefix} note {i:03}"), "Synthetic text")
                .unwrap();
            if i % 40 == 0 {
                let thread = workspace.new_thread(&format!("Chat {i}")).unwrap();
                workspace
                    .link_writing_thread(&format!("link-{i}"), &note.id, note.version, &thread.id)
                    .unwrap();
            }
        }
    });
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::Notes, cx)
    });
    let rows = |window: &Window, prefix: &str, ids: &[String]| {
        ids.iter()
            .filter(|id| window.try_find(format!("{prefix}{id}")).is_some())
            .count()
    };
    let ids: Vec<String> = cx.read(|cx| {
        f.view
            .read(cx)
            .note_rows
            .iter()
            .map(|row| row.id.clone())
            .collect()
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(rows(window, "notes-recent-", &ids), lists::SECTION_LIMIT);
        assert_eq!(rows(window, "notes-linked-", &ids), 6);
        assert_eq!(rows(window, "note-", &ids), 80, "only the selected letter");
    });
    click(cx, &f, format!("notes-letter-{}", 'Õ' as u32));
    on_window(cx, &f, |window, _| {
        assert_eq!(rows(window, "note-", &ids), 80);
    });
    on_window(cx, &f, |window, cx| {
        f.view
            .read(cx)
            .search
            .clone()
            .update(cx, |input, cx| input.focus(window, cx));
        window.input("note 12", cx);
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(rows(window, "note-", &ids), 10, "titles 120–129");
        assert!(window.try_find("search-results").is_some());
    });
    let target = cx.read(|cx| {
        f.view
            .read(cx)
            .note_rows
            .iter()
            .find(|row| row.title == "Beta note 121")
            .unwrap()
            .id
            .clone()
    });
    on_window(cx, &f, |window, cx| {
        window.right_click(format!("note-{target}"), cx)
    });
    choose(cx, &f, "New chat about this note");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(target.as_str()));
        let ChatTarget::Thread(thread) = &view.chat else {
            panic!("a linked chat is created")
        };
        assert_eq!(view.writing_threads.get(&target), Some(thread));
        assert_eq!(view.workspace.writing_threads(&target).unwrap().len(), 1);
    });
}

#[gpui_kit::test]
fn dirty_note_requires_an_explicit_save_before_commenting(cx: &mut TestAppContext) {
    let mut note = None;
    let f = fixture(cx, |workspace| {
        note = Some(workspace.new_note("Anchored", "tere tere õun").unwrap());
    });
    let note = note.unwrap();
    update(cx, &f, |view, window, cx| {
        view.open_note(&note.id, None, window, cx);
        view.note_editor
            .update(cx, |editor, cx| editor.set_selected_range(5..9, cx));
        view.track_selection(cx);
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("selection-comment").disabled(), None);
        assert!(window.try_find("selection-comment-reason").is_none());
    });
    click(cx, &f, "selection-comment");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.comment_anchor.as_ref().unwrap().3, "tere");
        assert_eq!(view.comment_anchor.as_ref().unwrap().1, note.version);
    });
    update(cx, &f, |view, window, cx| {
        view.comment_anchor = None;
        view.note_editor.update(cx, |editor, cx| {
            editor.set_value("tere tere õun and an unsaved tail", window, cx)
        });
        view.preserve_visible_note_input(cx);
        view.focus_visible_editor(window, cx);
    });
    update(cx, &f, |view, _, cx| {
        assert!(view.note.as_ref().unwrap().session.is_some());
        view.note_editor
            .update(cx, |editor, cx| editor.set_selected_range(0..4, cx));
        view.track_selection(cx);
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("selection-comment-reason").visible());
        assert!(window.find("note-unsaved-chip").visible());
        assert!(
            window
                .find("status-save")
                .label()
                .unwrap()
                .contains("unsaved")
        );
    });
    press(cx, &f, "cmd-/");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.comment_anchor.is_none(), "no anchor on unsaved text");
        assert!(view.notice.contains("anchor to the saved version"));
        assert!(
            view.workspace
                .writing_comments(&note.id)
                .unwrap()
                .is_empty()
        );
    });
    click(cx, &f, "note-menu");
    choose(cx, &f, "Comment on selection · save first");
    cx.read(|cx| {
        assert!(
            f.view.read(cx).comment_anchor.is_none(),
            "disabled menu row"
        )
    });
    press(cx, &f, "escape");
    click(cx, &f, "selection-comment");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(
            view.note.as_ref().unwrap().session.is_none(),
            "Save and comment saves before opening the composer"
        );
        let anchor = view
            .comment_anchor
            .as_ref()
            .expect("saved passage composer");
        assert_eq!(anchor.1, note.version + 1);
        assert_eq!(anchor.2, 0..4);
        assert_eq!(anchor.3, "tere");
        assert!(
            view.workspace
                .writing_comments(&note.id)
                .unwrap()
                .is_empty()
        );
        assert!(view.worker.is_none());
    });
}

#[gpui_kit::test]
fn review_in_place_applies_exact_candidate_then_marks_addressed_and_undoes(
    cx: &mut TestAppContext,
) {
    let mut fixture_ids = None;
    let f = fixture(cx, |workspace| {
        let note = workspace
            .new_note("Essay", "# Essay\n\nThe quick brown fox.\n\nKeep this.")
            .unwrap();
        let thread = workspace.new_thread("Writing: Essay").unwrap().id;
        workspace
            .link_writing_thread("essay-link", &note.id, note.version, &thread)
            .unwrap();
        fixture_ids = Some((note, thread));
    });
    let (note, thread) = fixture_ids.unwrap();
    let candidate = update(cx, &f, |view, window, cx| {
        view.open_note(&note.id, None, window, cx);
        view.comment_anchor = Some((note.id.clone(), note.version, 13..18, "quick".into()));
        view.comment_input
            .update(cx, |e, cx| e.set_value("Sharper verb", window, cx));
        view.post_comment(window, cx);
        let ids: Vec<String> = view
            .workspace
            .writing_comments(&note.id)
            .unwrap()
            .into_iter()
            .map(|entry| entry.record.id)
            .collect();
        let snapshot = view
            .workspace
            .snapshot_writing_request("essay-request", &note.id, note.version, &thread, &ids)
            .unwrap();
        let op = view
            .workspace
            .store
            .allocate_operation("essay-proposal")
            .unwrap();
        let candidate = view
            .workspace
            .store
            .prepare(
                &op,
                &snapshot
                    .change_request("# Essay\n\nThe swift brown fox.\n\nKeep this.")
                    .unwrap(),
            )
            .unwrap();
        view.refresh();
        cx.notify();
        candidate
    });
    let op = candidate.operation.as_str().to_owned();
    on_window(cx, &f, |window, _| {
        assert!(
            window.find(format!("writing-proposal-{op}")).visible(),
            "proposal ready in the writing tile"
        );
        assert!(window.try_find(format!("card-apply-{op}")).is_none());
    });
    click(cx, &f, format!("writing-review-{op}"));
    cx.read(|cx| {
        let view = f.view.read(cx);
        let shown = view.diff_editor.read(cx).value().to_string();
        assert!(
            shown.contains("quick") && shown.contains("swift"),
            "{shown}"
        );
        assert_eq!(
            view.review.as_ref().unwrap().candidate.operation.as_str(),
            op
        );
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("review-bar").visible());
        assert!(window.try_find("note-body").is_none());
        assert_eq!(window.find("apply-final").label(), Some("Apply whole note"));
        assert!(window.find("writing-feedback").visible());
    });
    click(cx, &f, "review-read-result");
    update(cx, &f, |view, window, cx| {
        assert!(view.review_reading);
        assert!(
            view.candidate_view
                .read(cx)
                .focus_handle()
                .is_focused(window)
        );
        assert!(!view.diff_editor.focus_handle(cx).is_focused(window));
        assert_eq!(
            view.review.as_ref().unwrap().candidate.operation.as_str(),
            op
        );
    });
    click(cx, &f, "review-read-result");
    update(cx, &f, |view, window, cx| {
        assert!(!view.review_reading);
        assert!(view.diff_editor.focus_handle(cx).is_focused(window));
        assert_eq!(
            view.review.as_ref().unwrap().candidate.operation.as_str(),
            op
        );
    });
    click(cx, &f, "review-compare");
    on_window(cx, &f, |window, _| {
        assert!(window.find("compare-content").visible())
    });
    // Edit final text creates a new exact candidate; the old one is not applied.
    click(cx, &f, "review-edit");
    update(cx, &f, |view, window, cx| {
        assert!(view.review_editor.focus_handle(cx).is_focused(window));
        view.review_editor.update(cx, |editor, cx| {
            editor.set_value(
                "# Essay\n\nThe swift brown fox runs.\n\nKeep this.",
                window,
                cx,
            )
        });
        if let Some(review) = &mut view.review {
            review.edited = true;
        }
    });
    click(cx, &f, "apply-final");
    update(cx, &f, |view, window, cx| {
        view.card_apply(candidate.clone(), window, cx);
        view.card_dismiss(candidate.clone(), window, cx);
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(
            view.review.is_some(),
            "edited text must be prepared before Apply"
        );
        assert!(
            view.review.as_ref().unwrap().edited,
            "card actions keep edited text"
        );
        assert!(view.notice.contains("editing this proposal"));
        assert!(view.candidates.iter().any(|c| c.operation.as_str() == op));
        let RecordData::Note(saved) = view.workspace.store.note(&note.id).unwrap().unwrap().data
        else {
            panic!("note")
        };
        assert!(saved.markdown.contains("quick"));
    });
    click(cx, &f, "prepare-final");
    let replaced = cx.read(|cx| {
        let view = f.view.read(cx);
        let review = view.review.as_ref().unwrap();
        assert_ne!(review.candidate.operation.as_str(), op);
        assert!(!view.review_editing && !review.edited);
        assert!(view.diff_editor.read(cx).value().contains("runs."));
        review.candidate.operation.as_str().to_owned()
    });
    on_window(cx, &f, |window, _| {
        assert!(window.try_find(format!("card-apply-{replaced}")).is_none());
        assert!(window.find("apply-final").visible())
    });
    click(cx, &f, "apply-final");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.review.is_none());
        let RecordData::Note(saved) = view.workspace.store.note(&note.id).unwrap().unwrap().data
        else {
            panic!("note")
        };
        assert_eq!(
            saved.markdown,
            "# Essay\n\nThe swift brown fox runs.\n\nKeep this."
        );
        assert_eq!(view.note_editor.read(cx).value().as_ref(), saved.markdown);
        assert!(
            view.workspace
                .writing_comments(&note.id)
                .unwrap()
                .iter()
                .all(|c| !c.addressed)
        );
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("applied-state").visible());
        assert_eq!(
            window.find("confirm-addressed").label(),
            Some("mark 1 comments addressed")
        );
    });
    click(cx, &f, "confirm-addressed");
    cx.read(|cx| {
        assert!(
            f.view
                .read(cx)
                .workspace
                .writing_comments(&note.id)
                .unwrap()
                .iter()
                .all(|c| c.addressed)
        )
    });
    click(cx, &f, "undo-writing-apply");
    cx.read(|cx| {
        let view = f.view.read(cx);
        let RecordData::Note(saved) = view.workspace.store.note(&note.id).unwrap().unwrap().data
        else {
            panic!("note")
        };
        assert_eq!(
            saved.markdown,
            "# Essay\n\nThe quick brown fox.\n\nKeep this."
        );
        assert_eq!(view.note_editor.read(cx).value().as_ref(), saved.markdown);
        assert!(
            view.workspace
                .writing_comments(&note.id)
                .unwrap()
                .iter()
                .all(|c| !c.addressed)
        );
        assert!(!view.applied_rewrite.contains_key(&note.id));
    });
}

#[gpui_kit::test]
fn proposal_card_dismiss_keeps_the_note_and_launcher_runs_commands(cx: &mut TestAppContext) {
    let mut fixture_ids = None;
    let f = fixture(cx, |workspace| {
        let note = workspace.new_note("Card note", "Original text.").unwrap();
        let thread = workspace.new_thread("Writing: Card note").unwrap().id;
        workspace
            .link_writing_thread("card-link", &note.id, note.version, &thread)
            .unwrap();
        fixture_ids = Some((note, thread));
    });
    let (note, thread) = fixture_ids.unwrap();
    let op = update(cx, &f, |view, window, cx| {
        view.open_note(&note.id, None, window, cx);
        let snapshot = view
            .workspace
            .snapshot_writing_request("card-request", &note.id, note.version, &thread, &[])
            .unwrap();
        let op = view
            .workspace
            .store
            .allocate_operation("card-proposal")
            .unwrap();
        let candidate = view
            .workspace
            .store
            .prepare(&op, &snapshot.change_request("Rewritten text.").unwrap())
            .unwrap();
        view.refresh();
        cx.notify();
        candidate.operation.as_str().to_owned()
    });
    click(cx, &f, format!("writing-dismiss-{op}"));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.candidates.is_empty());
        assert_eq!(view.workspace.store.note(&note.id).unwrap().unwrap(), note);
    });
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, cx| window.input("changes", cx));
    press(cx, &f, "enter");
    assert_eq!(
        cx.read(|cx| f.view.read(cx).layout.focus()),
        Some(TileKind::History)
    );
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, cx| window.input("layout: solo", cx));
    press(cx, &f, "enter");
    assert_eq!(kinds(cx, &f), [TileKind::History]);
    press(cx, &f, "cmd-k");
    cx.read(|cx| assert!(f.view.read(cx).launcher_open));
    press(cx, &f, "escape");
    cx.read(|cx| assert!(!f.view.read(cx).launcher_open));
    press(cx, &f, "cmd-t");
    cx.read(|cx| assert_eq!(f.view.read(cx).chat, ChatTarget::New));
    assert!(kinds(cx, &f).contains(&TileKind::Chat));
}

#[gpui_kit::test]
fn a_note_draft_follows_the_first_thread_created_for_that_note(cx: &mut TestAppContext) {
    let mut ids = None;
    let f = fixture(cx, |workspace| {
        let note = workspace.new_note("Unthreaded", "tere tere").unwrap();
        let other = workspace.new_thread("Standalone").unwrap().id;
        ids = Some((note, other));
    });
    let (note, other) = ids.unwrap();
    update(cx, &f, |view, window, cx| {
        view.open_note(&note.id, None, window, cx);
        assert_eq!(view.chat, ChatTarget::NewForNote(note.id.clone()));
        view.composer.update(cx, |editor, cx| {
            editor.set_value("Draft for the note", window, cx)
        });
        view.open_thread(&other, window, cx);
        assert_eq!(view.composer.read(cx).value().as_ref(), "");
        view.comment_anchor = Some((note.id.clone(), note.version, 0..4, "tere".into()));
        view.comment_input.update(cx, |editor, cx| {
            editor.set_value("First feedback", window, cx)
        });
        view.post_comment(window, cx);
        let created = view.writing_threads.get(&note.id).unwrap().clone();
        assert_eq!(
            view.chat,
            ChatTarget::Thread(other.clone()),
            "chat stays put"
        );
        view.open_note(&note.id, None, window, cx);
        assert_eq!(view.chat, ChatTarget::Thread(other.clone()));
        view.open_thread(&created, window, cx);
        assert_eq!(view.chat, ChatTarget::Thread(created));
        assert_eq!(
            view.composer.read(cx).value().as_ref(),
            "Draft for the note"
        );
    });
}

#[gpui_kit::test]
fn chat_scroller_keeps_reading_position_across_updates_and_per_chat_navigation(
    cx: &mut TestAppContext,
) {
    let mut a = String::new();
    let mut b = String::new();
    let f = fixture(cx, |w| {
        a = w.new_thread("Long chat").unwrap().id;
        b = w.new_thread("Other chat").unwrap().id;
        for i in 0..60 {
            w.post_message(
                &a,
                &format!("Message {i}\n{}", "Long readable text. ".repeat(80)),
            )
            .unwrap();
        }
    });
    update(cx, &f, |ui, window, cx| ui.open_thread(&a, window, cx));
    let state = cx.read(|cx| {
        f.view
            .read(cx)
            .scrollers
            .borrow()
            .get(&a)
            .unwrap()
            .state
            .clone()
    });
    update(cx, &f, |_, _, cx| {
        state.update(cx, |s, cx| assert!(s.scroll_to_item(0, cx)));
    });
    cx.read(|cx| assert!(!state.read(cx).is_following_tail()));
    update(cx, &f, |ui, window, cx| {
        ui.workspace.post_message(&a, "New latest message").unwrap();
        ui.refresh();
        ui.open_thread(&b, window, cx);
        ui.open_thread(&a, window, cx);
    });
    cx.read(|cx| {
        assert_eq!(state.read(cx).item_count(), 62);
        assert!(!state.read(cx).is_following_tail());
        assert_eq!(
            f.view
                .read(cx)
                .scrollers
                .borrow()
                .get(&a)
                .unwrap()
                .state
                .entity_id(),
            state.entity_id()
        );
    });
    update(cx, &f, |_, _, cx| {
        state.update(cx, |s, cx| s.scroll_to_end(cx))
    });
    cx.read(|cx| assert!(state.read(cx).is_following_tail()));
}

#[gpui_kit::test]
fn chat_and_note_figure_resolvers_exclude_foreign_private_assets(cx: &mut TestAppContext) {
    let mut owner_chat = String::new();
    let mut other_chat = String::new();
    let mut note_id = String::new();
    let mut asset_id = String::new();
    let f = fixture(cx, |w| {
        owner_chat = w.new_thread("Private figures").unwrap().id;
        other_chat = w.new_thread("Other conversation").unwrap().id;
        let op = w.store.allocate_operation("private-asset-fixture").unwrap();
        note_id = op.creation_id(0);
        asset_id = op.creation_id(1);
        let mut n = Note::working("Private", "Figure");
        n.chat = Some(owner_chat.clone());
        w.owner_change(
            "private-asset-fixture",
            &ChangeRequest {
                reason: "Private figure".into(),
                inputs: vec![],
                writes: vec![
                    Put {
                        id: note_id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Note(n),
                    },
                    Put {
                        id: asset_id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Asset(Asset {
                            note: note_id.clone(),
                            source: None,
                            media_type: "image/png".into(),
                            bytes: vec![1, 2, 3],
                        }),
                    },
                    Put {
                        id: op.creation_id(2),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Link(Link {
                            from: owner_chat.clone(),
                            to: note_id.clone(),
                            relation: "attachment".into(),
                        }),
                    },
                ],
            },
        )
        .unwrap();
    });
    update(cx, &f, |ui, window, cx| {
        let key = format!("brn-asset://{asset_id}");
        ui.open_thread(&other_chat, window, cx);
        assert!(!ui.assets_in_chat(Some(&other_chat)).contains_key(&key));
        assert!(!ui.assets_in_chat(None).contains_key(&key));
        assert!(ui.assets_in_chat(Some(&owner_chat)).contains_key(&key));
        ui.open_note(&note_id, None, window, cx);
        assert_eq!(ui.chat, ChatTarget::Thread(other_chat.clone()));
        assert!(ui.note_assets().contains_key(&key));
    });
}

#[gpui_kit::test]
fn normal_navigation_stays_bounded_and_explicit_splits_are_reused(cx: &mut TestAppContext) {
    let f = fixture(cx, |_| {});
    for _ in 0..5 {
        for id in [
            "open-notes",
            "browse-chats",
            "all-threads",
            "home",
            "actions",
            "history",
            "browse-chats",
        ] {
            click(cx, &f, id);
            assert!(kinds(cx, &f).len() <= 3);
        }
    }
    click(cx, &f, "layout-write");
    click(cx, &f, "open-beside");
    choose(cx, &f, "connections");
    assert_eq!(kinds(cx, &f).len(), 4);
    click(cx, &f, "open-beside");
    choose(cx, &f, "connections");
    assert_eq!(kinds(cx, &f).len(), 4, "an existing panel is reused");
    click(cx, &f, "actions");
    assert_eq!(kinds(cx, &f).len(), 4);
    assert!(kinds(cx, &f).contains(&TileKind::Connections));
    assert!(!kinds(cx, &f).contains(&TileKind::Note));
    click(cx, &f, "open-beside");
    choose(cx, &f, "Close extra splits");
    assert_eq!(kinds(cx, &f).len(), 3);
    assert!(!kinds(cx, &f).contains(&TileKind::Connections));
    cx.read(|cx| assert!(f.view.read(cx).worker.is_none()));
}

#[gpui_kit::test]
fn settings_and_solo_restore_arrangement_and_preserve_dirty_input(cx: &mut TestAppContext) {
    let mut note = String::new();
    let f = fixture(cx, |workspace| {
        note = workspace
            .new_note("Layout witness", "Saved text")
            .unwrap()
            .id;
    });
    update(cx, &f, |view, window, cx| {
        view.open_note(&note, None, window, cx);
        view.note_editor.update(cx, |editor, cx| {
            editor.set_value("Unsaved work", window, cx)
        });
    });
    let before = kinds(cx, &f);
    click(cx, &f, "settings");
    on_window(cx, &f, |window, cx| {
        assert!(window.find("settings-surface").visible());
        assert!(window.try_find("note-body").is_none());
        assert!(
            !f.view
                .read(cx)
                .note_editor
                .focus_handle(cx)
                .is_focused(window)
        );
        window.press("tab", cx);
        window.input("Should not reach the note", cx);
    });
    press(cx, &f, "cmd-shift-enter");
    press(cx, &f, "escape");
    assert_eq!(kinds(cx, &f), before);
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(!view.layout.settings_open());
        assert_eq!(view.note_editor.read(cx).value().as_ref(), "Unsaved work");
        assert_eq!(
            view.workspace.store.recovery_buffers().unwrap()[0].markdown,
            "Unsaved work"
        );
    });
    click(cx, &f, "layout-solo");
    assert_eq!(kinds(cx, &f), [TileKind::Note]);
    click(cx, &f, "layout-solo");
    assert_eq!(kinds(cx, &f), before);
    click(cx, &f, "settings");
    click(cx, &f, "close-settings");
    assert_eq!(kinds(cx, &f), before);
}

#[gpui_kit::test]
fn explicit_offscreen_split_and_keyboard_focus_are_revealed(cx: &mut TestAppContext) {
    let f = fixture(cx, |_| {});
    for kind in [
        TileKind::Overview,
        TileKind::Actions,
        TileKind::History,
        TileKind::Connections,
    ] {
        update(cx, &f, |view, window, cx| {
            view.show_beside(kind, window, cx)
        });
    }
    on_window(cx, &f, |window, _| {
        let target = window.find("tile-connections").bounds();
        assert!(target.left() >= px(0.) && target.right() <= px(1400.));
    });
    // Manually scroll the already-focused target away; reopening must still reveal it.
    update(cx, &f, |view, _, cx| {
        view.tile_scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("tile-connections").bounds().right() > px(1400.))
    });
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Connections, window, cx)
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("tile-connections").bounds().right() <= px(1400.))
    });
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Threads, window, cx)
    });
    on_window(cx, &f, |window, _| {
        let target = window.find("tile-threads").bounds();
        assert!(target.left() >= px(0.) && target.right() <= px(1400.));
    });
    press(cx, &f, "cmd-[");
    on_window(cx, &f, |window, _| {
        let target = window.find("tile-connections").bounds();
        assert!(target.left() >= px(0.) && target.right() <= px(1400.));
    });
}

#[gpui_kit::test]
fn new_chat_from_solo_note_menu_focuses_the_visible_composer(cx: &mut TestAppContext) {
    let mut note = String::new();
    let f = fixture(cx, |workspace| {
        note = workspace.new_note("Solo note", "Saved wording").unwrap().id;
    });
    update(cx, &f, |view, window, cx| {
        view.open_note(&note, None, window, cx);
        view.apply_layout(Preset::Solo, window, cx);
    });
    click(cx, &f, "note-menu");
    choose(cx, &f, "New chat for this note");
    on_window(cx, &f, |window, cx| {
        let view = f.view.read(cx);
        assert_eq!(view.layout.maximized(), Some(TileKind::Chat));
        assert!(view.composer.focus_handle(cx).is_focused(window));
        assert!(!view.note_editor.focus_handle(cx).is_focused(window));
        assert!(view.chat_thread().is_some());
        assert!(view.worker.is_none());
    });
}

#[gpui_kit::test]
fn simplified_navigation_tracks_focus_and_keeps_chat_filters_and_draft(cx: &mut TestAppContext) {
    let mut chat = String::new();
    let f = fixture(cx, |workspace| {
        chat = workspace.new_thread("Draft witness").unwrap().id;
    });
    update(cx, &f, |view, window, cx| {
        view.open_thread(&chat, window, cx);
        view.composer.update(cx, |editor, cx| {
            editor.set_value("Keep my unsent draft", window, cx)
        });
    });
    for id in [
        "browse-chats",
        "all-threads",
        "resolved-count",
        "chats-recent",
    ] {
        click(cx, &f, id);
        on_window(cx, &f, |window, cx| {
            assert!(window.find("browse-chats").visible());
            let view = f.view.read(cx);
            assert_eq!(view.navigation_focus(), Some(TileKind::Threads));
            assert_eq!(view.chat, ChatTarget::Thread(chat.clone()));
            assert_eq!(
                view.composer.read(cx).value().as_str(),
                "Keep my unsent draft"
            );
            assert!(view.layout.visible().len() <= 3);
        });
    }
    for (id, kind, label) in [
        ("home", TileKind::Overview, "Home"),
        ("open-notes", TileKind::Notes, "Notes"),
        ("actions", TileKind::Actions, "Tasks"),
        ("history", TileKind::History, "Changes"),
        ("settings", TileKind::Settings, "Settings"),
    ] {
        click(cx, &f, id);
        on_window(cx, &f, |window, cx| {
            assert_eq!(f.view.read(cx).layout.focus(), Some(kind));
            assert_eq!(window.find(id).label(), Some(label));
            assert_eq!(f.view.read(cx).navigation_focus(), Some(kind));
        });
    }
    press(cx, &f, "escape");
    assert_eq!(
        cx.read(|cx| f.view.read(cx).layout.focus()),
        Some(TileKind::History)
    );
}

#[gpui_kit::test]
fn companion_focus_and_theme_changes_keep_navigation_readable(cx: &mut TestAppContext) {
    use gpui_kit::component::ActiveTheme;
    let f = fixture(cx, |_| {});
    for (kind, destination) in [
        (TileKind::Chat, TileKind::Threads),
        (TileKind::Linked, TileKind::Threads),
        (TileKind::Note, TileKind::Notes),
        (TileKind::Connections, TileKind::Notes),
        (TileKind::Compare, TileKind::Notes),
    ] {
        update(cx, &f, |view, window, cx| view.show_tile(kind, window, cx));
        assert_eq!(
            cx.read(|cx| f.view.read(cx).navigation_focus()),
            Some(destination)
        );
    }
    for theme in ThemeId::ALL {
        update(cx, &f, |view, window, cx| {
            view.choose_theme(theme, window, cx);
            let palette = theme.palette();
            // Ghost controls read these toolkit channels rather than button_foreground.
            assert_eq!(
                cx.theme().secondary_foreground,
                gpui_kit::Hsla::from(rgb(palette.ink))
            );
            assert_eq!(
                cx.theme().secondary_active,
                gpui_kit::Hsla::from(rgb(palette.active))
            );
            // Button::danger() and Tag read the tone channels.
            let toward = if palette.dark { 0xffffff } else { 0x000000 };
            for (channel, tone) in [
                (cx.theme().danger, palette.danger),
                (cx.theme().button_danger, palette.danger),
                (cx.theme().warning, palette.warning),
                (cx.theme().success, palette.success),
                (cx.theme().info, palette.info),
                (
                    cx.theme().button_danger_hover,
                    theme::mix(palette.danger, toward, 0.14),
                ),
                (
                    cx.theme().button_danger_active,
                    theme::mix(palette.danger, toward, 0.26),
                ),
                (cx.theme().button_danger_foreground, palette.tile),
                (cx.theme().info_foreground, palette.tile),
            ] {
                assert_eq!(
                    channel,
                    gpui_kit::Hsla::from(rgb(tone)),
                    "{}",
                    theme.label()
                );
            }
        });
    }
}

#[gpui_kit::test]
fn a_focused_overflow_menu_opens_and_runs_an_item_from_the_keyboard(cx: &mut TestAppContext) {
    let f = fixture(cx, |_| {});
    let note = update(cx, &f, |view, _, _| {
        view.workspace
            .new_note("Keyboard menu", "Saved text")
            .unwrap()
            .id
    });
    update(cx, &f, |view, window, cx| {
        view.open_note(&note, None, window, cx);
        view.note_editor.update(cx, |editor, cx| {
            editor.set_value("Saved from the keyboard", window, cx)
        });
        // Typing records the edit; set_value alone does not.
        view.preserve_visible_note_input(cx);
    });
    assert!(cx.read(|cx| f.view.read(cx).note.as_ref().unwrap().session.is_some()));
    // Start from the workspace root, as after clicking empty space.
    update(cx, &f, |view, window, cx| view.focus_root(window, cx));
    let reached = (0..80).any(|_| {
        press(cx, &f, "tab");
        on_window(cx, &f, |window, _| {
            window.find("note-menu").focused() == Some(true)
        })
    });
    assert!(reached, "Tab reaches the Note ⋯ trigger");
    press(cx, &f, "enter");
    on_window(cx, &f, |window, _| {
        assert!(
            window.within("popup-menu").try_find(0usize).is_some(),
            "↵ opens the focused trigger's menu"
        );
    });
    // The first row is Save, enabled while the note has an unsaved edit.
    press(cx, &f, "down");
    press(cx, &f, "enter");
    cx.read(|cx| {
        let view = f.view.read(cx);
        let open = view.note.as_ref().unwrap();
        assert!(open.session.is_none(), "the chosen item ran: {}", view.notice);
        assert!(matches!(&open.record.data, RecordData::Note(n) if n.markdown == "Saved from the keyboard"));
    });
}

#[gpui_kit::test]
fn consequence_dialogs_confirm_only_by_their_danger_button(cx: &mut TestAppContext) {
    let f = fixture(cx, |_| {});
    let confirmed = Rc::new(std::cell::Cell::new(false));
    let seen = confirmed.clone();
    on_window(cx, &f, move |window, cx| {
        let seen = seen.clone();
        window.open_dialog(cx, move |dialog, _, _| {
            let seen = seen.clone();
            components::consequence_dialog(
                dialog,
                "synthetic-discard",
                "Discard your edit?",
                "Your changes since v3 will be lost. This cannot be undone.",
                "Discard edit",
                Rc::new(move |_, _| seen.set(true)),
            )
        });
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("synthetic-discard-body").label(),
            Some("Your changes since v3 will be lost. This cannot be undone.")
        );
        assert!(window.find("synthetic-discard-confirm").visible());
    });
    press(cx, &f, "enter");
    assert!(!confirmed.get(), "↵ never confirms");
    on_window(cx, &f, |window, cx| {
        assert!(window.has_active_dialog(cx), "↵ leaves the dialog open");
        window.click("synthetic-discard-confirm", cx);
    });
    assert!(confirmed.get());
    on_window(cx, &f, |window, cx| assert!(!window.has_active_dialog(cx)));
}
