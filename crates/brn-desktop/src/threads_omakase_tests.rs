//! Omakase shell journeys with the real headless GPUI harness, synthetic data
//! and no provider. Native pointer feel, pixels and IME remain owner checks.
use super::*;
use gpui_kit::{Focusable, TestAppContext, test::TestWindowExt};

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
    choose(cx, &f, "threads");
    assert!(kinds(cx, &f).contains(&TileKind::Threads));
    on_window(cx, &f, |window, cx| {
        window.right_click("tile-header-note", cx)
    });
    choose(cx, &f, "settings");
    assert_eq!(
        cx.read(|cx| f.view.read(cx).layout.focus()),
        Some(TileKind::Settings)
    );
    press(cx, &f, "alt-3");
    assert_eq!(kinds(cx, &f), [TileKind::Settings]);
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
        assert_eq!(window.find("status-theme").label(), Some("◐ Raba"));
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
fn resolved_threads_leave_the_sidebar_stay_findable_and_reply_reopens(cx: &mut TestAppContext) {
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
        assert_eq!(
            window.find("resolved-count").label(),
            Some("✓ resolved · 1")
        );
        assert_eq!(
            window.find("all-threads").label(),
            Some("all threads · 13 →")
        );
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
    click(cx, &f, "threads-filter-all");
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
            Some("thread · Open question 00 · ✓ resolved")
        );
    });
    click(cx, &f, "launcher-item-0");
    on_window(cx, &f, |window, _| {
        assert!(window.find("thread-resolved").visible());
        assert!(window.find("reopen-thread").visible());
        assert!(
            window
                .find("composer-destination")
                .label()
                .unwrap()
                .contains("sending reopens it")
        );
    });
    update(cx, &f, |view, window, cx| {
        view.composer.update(cx, |editor, cx| {
            editor.set_value("One more thing", window, cx)
        });
        view.ask(window, cx);
        assert_eq!(thread_state(view, &done), ThreadState::Open);
        assert!(
            view.history
                .iter()
                .any(|receipt| receipt.reason == "Owner replied to a resolved thread")
        );
    });
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("resolved-count").is_none());
        assert!(window.find(format!("thread-{done}")).visible());
    });
    // Explicit resolve and reopen from the header.
    click(cx, &f, "resolve-thread");
    cx.read(|cx| assert_eq!(thread_state(f.view.read(cx), &done), ThreadState::Resolved));
    click(cx, &f, "reopen-thread");
    cx.read(|cx| assert_eq!(thread_state(f.view.read(cx), &done), ThreadState::Open));
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
fn quick_questions_auto_resolve_only_when_nothing_else_is_involved(cx: &mut TestAppContext) {
    let mut quick = String::new();
    let mut linked_thread = String::new();
    let mut twice = String::new();
    let mut failed = String::new();
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
        for thread in [&quick, &linked_thread, &twice] {
            record_run(workspace, thread, RunState::Completed);
        }
        record_run(workspace, &failed, RunState::Failed);
    });
    for thread in [&quick, &linked_thread, &twice, &failed] {
        finish_run(cx, &f, thread);
    }
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(thread_state(view, &quick), ThreadState::Resolved);
        let receipt = view
            .history
            .iter()
            .rev()
            .find(|receipt| receipt.writes.iter().any(|write| write.after.id == quick))
            .unwrap();
        assert_eq!(receipt.reason, lists::AUTO_RESOLVE_REASON);
        assert!(
            receipt.authority.starts_with("owner:"),
            "checked owner resolve path"
        );
        for thread in [&linked_thread, &twice, &failed] {
            assert_eq!(thread_state(view, thread), ThreadState::Open, "{thread}");
        }
    });
    update(cx, &f, |view, window, cx| {
        view.open_thread(&quick, window, cx)
    });
    on_window(cx, &f, |window, _| {
        assert!(
            window
                .find("thread-resolved")
                .label()
                .unwrap()
                .contains("resolved automatically")
        );
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
                let thread = workspace.new_thread(&format!("Thread {i}")).unwrap();
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
    choose(cx, &f, "New thread about this note");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(target.as_str()));
        let ChatTarget::Thread(thread) = &view.chat else {
            panic!("a linked thread is created")
        };
        assert_eq!(view.writing_threads.get(&target), Some(thread));
        assert_eq!(view.workspace.writing_threads(&target).unwrap().len(), 1);
    });
}

#[gpui_kit::test]
fn mode_pill_maps_to_exact_grants_and_resets_after_each_request(cx: &mut TestAppContext) {
    let mut note = None;
    let f = fixture(cx, |workspace| {
        note = Some(workspace.new_note("Grant target", "Text").unwrap());
    });
    let note = note.unwrap();
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("mode-pill").label(), Some("Ask · read only ▾"));
    });
    click(cx, &f, "mode-pill");
    choose(cx, &f, "Edit note · this request");
    cx.read(|cx| {
        assert_eq!(
            f.view.read(cx).mode,
            ComposeMode::Ask,
            "no open note grants nothing"
        )
    });
    press(cx, &f, "escape");
    update(cx, &f, |view, window, cx| {
        view.open_note(&note.id, None, window, cx)
    });
    click(cx, &f, "mode-pill");
    choose(cx, &f, "Edit Grant target · this request");
    cx.read(|cx| assert_eq!(f.view.read(cx).mode, ComposeMode::EditNote));
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("mode-pill").label(),
            Some("Edit Grant target · this request ▾")
        );
    });
    // The exact grant the send path uses for this state.
    cx.read(|cx| {
        let view = f.view.read(cx);
        let saved = view.records.iter().find(|r| r.id == note.id).unwrap();
        assert!(matches!(
            lists::grant(view.mode, Some((saved.id.as_str(), saved.version))).unwrap(),
            Some(brn_ai::threads::OwnerRequest::EditNote { id, version }) if id == note.id && version == saved.version
        ));
    });
    update(cx, &f, |view, window, cx| {
        view.composer
            .update(cx, |editor, cx| editor.set_value("Tighten it", window, cx));
        view.ask(window, cx);
        assert_eq!(view.mode, ComposeMode::Ask, "grants last one request");
        assert!(
            view.notice.contains("Choose a provider"),
            "offline fixture starts nothing"
        );
    });
    click(cx, &f, "mode-pill");
    choose(cx, &f, "Record decision · this request");
    cx.read(|cx| assert_eq!(f.view.read(cx).mode, ComposeMode::RecordDecision));
    update(cx, &f, |view, window, cx| view.new_thread(window, cx));
    cx.read(|cx| {
        assert_eq!(
            f.view.read(cx).mode,
            ComposeMode::Ask,
            "switching threads resets"
        )
    });
}

#[gpui_kit::test]
fn comment_is_disabled_with_reason_while_the_note_has_an_unsaved_edit(cx: &mut TestAppContext) {
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
    click(cx, &f, "selection-comment");
    cx.read(|cx| assert!(f.view.read(cx).comment_anchor.is_none(), "disabled control"));
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
    click(cx, &f, "status-save");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(
            view.note.as_ref().unwrap().session.is_none(),
            "status bar saves"
        );
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
            window.find(format!("proposal-{op}")).visible(),
            "card in the thread"
        );
        assert!(window.find(format!("card-apply-{op}")).visible());
    });
    click(cx, &f, format!("thread-review-{op}"));
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
    });
    click(cx, &f, "review-compare");
    on_window(cx, &f, |window, _| {
        assert!(window.find("compare-content").visible())
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
            "# Essay\n\nThe swift brown fox.\n\nKeep this."
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
    click(cx, &f, format!("card-dismiss-{op}"));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.candidates.is_empty());
        assert_eq!(view.workspace.store.note(&note.id).unwrap().unwrap(), note);
    });
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, cx| window.input("what changed", cx));
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
