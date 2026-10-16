//! Offline witnesses for dialogs, focus landing, Escape order and small
//! windows: each dialog's ↵ class and footer, where focus lands after
//! navigation, what Escape closes, and the 900×600 fit. Synthetic data only;
//! pixels and real pointer feel remain native acceptance.
use super::*;
use gpui_kit::{Focusable, TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
    width: f32,
    height: f32,
}

fn fixture(
    cx: &mut TestAppContext,
    width: f32,
    height: f32,
    seed: impl FnOnce(&mut Workspace),
) -> Fixture {
    cx.update(|cx| {
        gpui_kit::init(cx);
        // Keep dialog hit targets at their resting positions.
        cx.set_reduce_motion(true);
    });
    cx.update(|cx| cx.bind_keys(bindings()));
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("dialogs-offline");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    seed(&mut workspace);
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
        window,
        view,
        width,
        height,
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
fn press(cx: &mut TestAppContext, f: &Fixture, key: &str) {
    on_window(cx, f, |window, cx| window.press(key, cx));
}
/// Deliver the next platform frame, which the test harness never sends on
/// its own: a panel body moves focus to its first control then.
fn next_frame(cx: &mut TestAppContext, f: &Fixture) {
    on_window(cx, f, |window, cx| {
        window.simulate_next_frame(cx);
    });
}
fn window_bounds(f: &Fixture) -> Bounds<gpui_kit::Pixels> {
    Bounds::new(point(px(0.), px(0.)), size(px(f.width), px(f.height)))
}
fn assert_inside(window: &Window, id: &str, outer: Bounds<gpui_kit::Pixels>) {
    let element = window.find(id.to_owned());
    assert!(element.visible(), "{id} is visible");
    let bounds = element.bounds();
    assert!(
        bounds.left() >= outer.left() - px(0.5)
            && bounds.right() <= outer.right() + px(0.5)
            && bounds.top() >= outer.top() - px(0.5)
            && bounds.bottom() <= outer.bottom() + px(0.5),
        "{id} {bounds:?} inside {outer:?}"
    );
}
/// The open dialog's surface (Kit's layer 0).
fn dialog_surface(window: &mut Window) -> Bounds<gpui_kit::Pixels> {
    window.within("dialog").find(0usize).bounds()
}
fn note_titles(view: &ThreadsUi) -> Vec<String> {
    view.workspace
        .notes()
        .unwrap()
        .iter()
        .filter_map(|record| match &record.data {
            RecordData::Note(note) => Some(note.title.clone()),
            _ => None,
        })
        .collect()
}

#[gpui_kit::test]
fn new_note_dialog_creates_on_enter_and_stays_open_without_a_title(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |_| {});
    press(cx, &f, "cmd-n");
    on_window(cx, &f, |window, cx| {
        assert!(window.has_active_dialog(cx));
        assert!(window.find("create-named-note").visible());
        assert!(window.find("new-note-cancel").visible());
        // The action row is Kit's footer, below the field.
        let field = window.find("new-note-title").bounds();
        assert!(window.find("create-named-note").bounds().top() >= field.bottom());
    });
    // ↵ with an empty title keeps the dialog open and says why.
    press(cx, &f, "enter");
    on_window(cx, &f, |window, cx| {
        assert!(window.has_active_dialog(cx), "an empty title stays open");
        assert_eq!(
            window.find("new-note-error").label(),
            Some("Enter a title.")
        );
        assert!(note_titles(f.view.read(cx)).is_empty());
    });
    // The title field has focus from the start, so typing fills it.
    cx.simulate_input(f.window, "Pilot proposal");
    cx.run_until_parked();
    press(cx, &f, "enter");
    on_window(cx, &f, |window, cx| {
        assert!(!window.has_active_dialog(cx), "↵ creates and closes");
        let view = f.view.read(cx);
        assert_eq!(note_titles(view), ["Pilot proposal"]);
        assert!(
            view.writing
                .note
                .as_ref()
                .is_some_and(|open| title(&open.record) == "Pilot proposal")
        );
        assert!(
            view.writing.note_editor.focus_handle(cx).is_focused(window),
            "the new note's text has focus"
        );
    });
    // Escape cancels through Kit's Cancel.
    press(cx, &f, "cmd-n");
    press(cx, &f, "escape");
    on_window(cx, &f, |window, cx| {
        assert!(!window.has_active_dialog(cx));
        assert_eq!(note_titles(f.view.read(cx)).len(), 1);
    });
}

/// At 900×600 a long pick list scrolls inside the dialog body, which stays
/// at most window − 96 tall, and the footer's buttons stay in the window.
#[gpui_kit::test]
fn a_dialog_at_900x600_keeps_its_footer_in_the_window_while_the_body_scrolls(
    cx: &mut TestAppContext,
) {
    let mut chat = String::new();
    let f = fixture(cx, 900., 600., |workspace| {
        chat = workspace.new_thread("Moving chat").unwrap().id;
    });
    update(cx, &f, |view, _, cx| {
        for index in 0..20 {
            view.apply_project(
                ProjectCommand::Create {
                    title: format!("Project {index:02}"),
                },
                cx,
            )
            .unwrap();
        }
    });
    update(cx, &f, |view, window, cx| {
        view.move_chat_dialog(&chat, window, cx)
    });
    on_window(cx, &f, |window, _| {
        let screen = window_bounds(&f);
        let surface = dialog_surface(window);
        assert!(
            surface.size.height <= px(f.height - components::DIALOG_CLEARANCE) + px(0.5),
            "{surface:?}"
        );
        assert_inside(window, "move-chat-cancel", screen);
        assert_inside(window, "move-chat-cancel", surface);
        // The list is longer than its box: the last row is below it.
        let list = window.find("move-chat-list").bounds();
        let last = window.find("move-chat-20").bounds();
        assert!(last.bottom() > list.bottom(), "{last:?} below {list:?}");
        assert!(window.find("move-chat-0").visible());
    });
    // The pick list's rows are tab stops: ↵ chooses nothing, Space on a
    // row reached by Tab moves the chat.
    press(cx, &f, "enter");
    on_window(cx, &f, |window, cx| assert!(window.has_active_dialog(cx)));
    let reached = (0..10).any(|_| {
        press(cx, &f, "tab");
        on_window(cx, &f, |window, _| {
            window.find("move-chat-1").focused() == Some(true)
        })
    });
    assert!(reached, "Tab reaches the first project row");
    let chosen = on_window(cx, &f, |window, _| {
        window.find("move-chat-1").label().unwrap().to_owned()
    });
    press(cx, &f, "space");
    on_window(cx, &f, |window, cx| {
        assert!(!window.has_active_dialog(cx));
        let view = f.view.read(cx);
        assert!(view.records.iter().any(|record| matches!(
            &record.data,
            RecordData::Project(project) if project.title == chosen && project.chats == [chat.clone()]
        )), "the chat moved into {chosen}");
    });
}

/// After navigation, focus lies inside the destination: the editor, the
/// composer, the Notes search, the All chats filter, or a panel's first
/// control. Recent chats never focuses the All chats filter.
#[gpui_kit::test]
fn focus_lands_inside_each_destination_panel(cx: &mut TestAppContext) {
    let (mut note, mut chat) = (String::new(), String::new());
    let f = fixture(cx, 1400., 1000., |workspace| {
        note = workspace
            .new_note("Focus witness", "Saved text")
            .unwrap()
            .id;
        chat = workspace.new_thread("Focus chat").unwrap().id;
    });
    let inside = |cx: &mut TestAppContext, kind: TileKind| {
        next_frame(cx, &f);
        on_window(cx, &f, |window, cx| {
            let view = f.view.read(cx);
            assert_eq!(view.layout.focus(), Some(kind));
            assert!(
                view.panel_body(kind).contains_focused(window, cx),
                "focus is inside {kind:?}"
            );
        });
    };
    update(cx, &f, |view, window, cx| {
        view.open_note(&note, None, window, cx)
    });
    inside(cx, TileKind::Note);
    on_window(cx, &f, |window, cx| {
        assert!(
            f.view
                .read(cx)
                .writing
                .note_editor
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    update(cx, &f, |view, window, cx| {
        view.open_thread(&chat, window, cx)
    });
    inside(cx, TileKind::Chat);
    on_window(cx, &f, |window, cx| {
        assert!(f.view.read(cx).composer.focus_handle(cx).is_focused(window));
    });
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Notes, window, cx)
    });
    inside(cx, TileKind::Notes);
    on_window(cx, &f, |window, cx| {
        assert!(f.view.read(cx).search.focus_handle(cx).is_focused(window));
    });
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Threads, window, cx)
    });
    inside(cx, TileKind::Threads);
    on_window(cx, &f, |window, cx| {
        assert!(
            !f.view
                .read(cx)
                .threads_query
                .focus_handle(cx)
                .is_focused(window)
        );
        assert_eq!(window.find("chats-recent").focused(), Some(true));
    });
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::AllThreads, window, cx)
    });
    inside(cx, TileKind::AllThreads);
    on_window(cx, &f, |window, cx| {
        assert!(
            f.view
                .read(cx)
                .threads_query
                .focus_handle(cx)
                .is_focused(window)
        );
    });
    for kind in [
        TileKind::Actions,
        TileKind::History,
        TileKind::Overview,
        TileKind::Projects,
    ] {
        update(cx, &f, |view, window, cx| view.show_tile(kind, window, cx));
        inside(cx, kind);
    }
    // Settings lands on its section switch.
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Settings, window, cx)
    });
    inside(cx, TileKind::Settings);
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("settings-section-general").focused(),
            Some(true)
        );
    });
}

/// Clicking a Settings section moves focus off the section focus landed on,
/// so the ring never marks an unselected section. A key press keeps focus on
/// the pressed section.
#[gpui_kit::test]
fn a_clicked_settings_section_takes_the_focus_ring_off_general(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |_| {});
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Settings, window, cx)
    });
    next_frame(cx, &f);
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("settings-section-general").focused(),
            Some(true)
        );
    });
    on_window(cx, &f, |window, cx| {
        window.click("settings-section-models", cx)
    });
    on_window(cx, &f, |window, cx| {
        let view = f.view.read(cx);
        assert_eq!(view.settings_section, SettingsSection::Models);
        assert_ne!(
            window.find("settings-section-general").focused(),
            Some(true)
        );
        assert!(
            view.panel_body(TileKind::Settings)
                .contains_focused(window, cx)
        );
    });
    let reached = (0..12).any(|_| {
        press(cx, &f, "tab");
        on_window(cx, &f, |window, _| {
            window.find("settings-section-data").focused() == Some(true)
        })
    });
    assert!(reached, "Tab reaches the Data section");
    press(cx, &f, "space");
    on_window(cx, &f, |window, cx| {
        assert_eq!(f.view.read(cx).settings_section, SettingsSection::Data);
        assert_eq!(window.find("settings-section-data").focused(), Some(true));
    });
}

/// Choose a row of the open popup menu by its accessible label.
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

/// A dialog opened from a ⋯, ＋ or row menu item gives focus back to its
/// panel's landing when Escape closes it, so ⌘K works without a click: the
/// note text after Edit note… and Discard edit…, the Chats panel after New
/// note and after a chat row's Move to project….
#[gpui_kit::test]
fn escape_from_a_menu_launched_dialog_returns_focus_to_the_panel(cx: &mut TestAppContext) {
    let (mut note, mut chat) = (String::new(), String::new());
    let f = fixture(cx, 900., 600., |workspace| {
        note = workspace.new_note("Menu witness", "Saved text").unwrap().id;
        chat = workspace.new_thread("Row menu chat").unwrap().id;
    });
    update(cx, &f, |view, window, cx| {
        view.apply_layout(Preset::Write, window, cx);
        view.open_note(&note, None, window, cx);
    });
    let escape_returns = |cx: &mut TestAppContext, landing: &dyn Fn(&ThreadsUi, &Window, &App)| {
        on_window(cx, &f, |window, cx| assert!(window.has_active_dialog(cx)));
        press(cx, &f, "escape");
        on_window(cx, &f, |window, cx| {
            assert!(!window.has_active_dialog(cx));
            landing(f.view.read(cx), window, cx);
        });
        press(cx, &f, "cmd-k");
        cx.read(|cx| assert!(f.view.read(cx).launcher_open, "⌘K works"));
        press(cx, &f, "escape");
    };
    let in_note_text = |view: &ThreadsUi, window: &Window, cx: &App| {
        assert!(view.writing.note_editor.focus_handle(cx).is_focused(window));
    };
    // At 900×600 Edit note… is in the Note ⋯.
    on_window(cx, &f, |window, cx| window.click("note-menu", cx));
    choose(cx, &f, "Edit note…");
    escape_returns(cx, &in_note_text);
    // A dirty note offers Discard edit…; Escape keeps the edit.
    update(cx, &f, |view, window, cx| {
        view.writing.note_editor.update(cx, |editor, cx| {
            editor.set_value("Saved text, edited", window, cx)
        });
        view.preserve_visible_note_input(cx);
    });
    on_window(cx, &f, |window, cx| window.click("note-menu", cx));
    choose(cx, &f, "Discard edit…");
    escape_returns(cx, &in_note_text);
    cx.read(|cx| {
        assert!(
            f.view
                .read(cx)
                .writing
                .note
                .as_ref()
                .unwrap()
                .session
                .is_some()
        );
    });
    // The Chats ＋ menu's New note.
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Threads, window, cx)
    });
    on_window(cx, &f, |window, cx| window.click("threads-new", cx));
    choose(cx, &f, "New note");
    let in_the_workspace = |view: &ThreadsUi, window: &Window, cx: &App| {
        assert!(view.root_focus.contains_focused(window, cx));
        assert!(window.focused(cx).is_some());
    };
    escape_returns(cx, &in_the_workspace);
    // A chat row's menu: Move to project…, then Escape moves nothing.
    let version = |cx: &mut TestAppContext| {
        cx.read(|cx| {
            let view = f.view.read(cx);
            view.workspace.store.record(&chat).unwrap().unwrap().version
        })
    };
    let before = version(cx);
    on_window(cx, &f, |window, cx| {
        window.right_click(format!("thread-{chat}"), cx)
    });
    choose(cx, &f, "Move to project…");
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("move-chat-0").label(), Some("Standalone chat"));
    });
    escape_returns(cx, &in_the_workspace);
    assert_eq!(version(cx), before, "Escape moves nothing");
}

/// A 170 pt list leaves Maximize to its ⋯ menu so its title stays readable,
/// and the header ⋯'s Open beside submenu opens a split.
#[gpui_kit::test]
fn a_narrow_list_header_keeps_its_title_and_its_menu_opens_beside(cx: &mut TestAppContext) {
    let f = fixture(cx, 900., 600., |workspace| {
        workspace.new_thread("Header chat").unwrap();
    });
    update(cx, &f, |view, window, cx| {
        view.apply_layout(Preset::Write, window, cx);
        view.show_tile(TileKind::Threads, window, cx);
    });
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("tile-max-threads").is_none());
        assert!(window.find("tile-close-threads").visible());
        assert!(
            window.find("tile-max-chat").visible(),
            "wider panels keep it"
        );
        let title = window.find("tile-title-threads").bounds();
        assert!(title.size.width > px(36.), "the title reads: {title:?}");
    });
    on_window(cx, &f, |window, cx| window.click("tile-menu-threads", cx));
    choose(cx, &f, "Maximize");
    on_window(cx, &f, |window, cx| {
        assert_eq!(f.view.read(cx).layout.maximized(), Some(TileKind::Threads));
        assert_eq!(
            window.find("tile-max-threads").label(),
            Some("Restore Chats")
        );
        window.click("tile-max-threads", cx);
    });
    cx.read(|cx| assert_eq!(f.view.read(cx).layout.maximized(), None));
    // Open beside, from the Chat header's ⋯.
    on_window(cx, &f, |window, cx| window.click("tile-menu-chat", cx));
    on_window(cx, &f, |window, cx| {
        let mut menu = window.within("popup-menu");
        let index = (0..60usize)
            .find(|index| {
                menu.try_find(*index)
                    .is_some_and(|row| row.label() == Some("Open beside"))
            })
            .expect("Open beside row");
        menu.hover(index, cx);
    });
    on_window(cx, &f, |window, cx| {
        let mut submenu = window.within("submenu");
        let index = (0..60usize)
            .find(|index| {
                submenu
                    .try_find(*index)
                    .is_some_and(|row| row.label() == Some("Tasks"))
            })
            .expect("Tasks in the Open beside submenu");
        submenu.click(index, cx);
    });
    on_window(cx, &f, |window, cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.layout.area(TileKind::Actions),
            Some(layout::Area::Beside)
        );
        assert!(window.find("tile-split-actions").visible());
    });
}

/// A synthetic Working run held by a local worker in this window, which
/// holds the run's lease as a real worker does.
fn working_run(
    cx: &mut TestAppContext,
    f: &Fixture,
    thread: &str,
) -> (String, CancellationToken, brn_ai::run_lease::RunLease) {
    update(cx, f, |view, _, cx| {
        let key = "escape-working-run";
        let run = view
            .workspace
            .store
            .allocate_operation(key)
            .unwrap()
            .creation_id(0);
        let settings = view.workspace.settings().unwrap();
        view.workspace
            .owner_change(
                key,
                &ChangeRequest {
                    reason: "Synthetic running reply".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: run.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Run(Run {
                            instruction: None,
                            queue_claim: None,
                            thread: thread.into(),
                            provider: settings.provider,
                            model: settings.model,
                            guide_identity: "escape-fixture".into(),
                            budget: 1,
                            effort: settings.effort,
                            loaded_skills: vec![],
                            fence: 1,
                            state: RunState::Working,
                            progress: String::new(),
                            operations: vec![],
                            sources: vec![],
                        }),
                    }],
                },
            )
            .unwrap();
        let (_sender, receiver) = mpsc::sync_channel(4);
        let cancel = CancellationToken::new();
        view.worker = Some(Worker {
            receiver,
            cancel: cancel.clone(),
            conversion_cancel: Arc::new(AtomicBool::new(false)),
            kind: WorkerKind::Run,
            thread: Some(thread.into()),
            run_id: Some(run.clone()),
        });
        let lease = brn_ai::run_lease::RunLease::acquire(
            &view.workspace.store.derived_directory().unwrap(),
            &run,
        )
        .unwrap();
        view.refresh();
        cx.notify();
        (run, cancel, lease)
    })
}

/// Escape closes the launcher, a menu, a dialog, the stacked comments and
/// Settings, in that order, and never stops work.
#[gpui_kit::test]
fn escape_closes_one_layer_at_a_time_and_never_stops_work(cx: &mut TestAppContext) {
    let mut chat = String::new();
    let f = fixture(cx, 1400., 1000., |workspace| {
        chat = workspace.new_thread("Running chat").unwrap().id;
    });
    update(cx, &f, |view, window, cx| {
        view.open_thread(&chat, window, cx)
    });
    let (run, cancel, _lease) = working_run(cx, &f, &chat);
    let still_working = |cx: &mut TestAppContext| {
        cx.read(|cx| {
            let view = f.view.read(cx);
            assert!(view.worker.is_some() && !cancel.is_cancelled());
            assert!(matches!(
                view.workspace.store.record(&run).unwrap().unwrap().data,
                RecordData::Run(Run {
                    state: RunState::Working,
                    ..
                })
            ));
            assert!(!view.queue_state().paused, "the queue stays unpaused");
        });
    };
    // Nothing open: Escape does nothing to the run.
    update(cx, &f, |view, window, cx| view.focus_root(window, cx));
    press(cx, &f, "escape");
    still_working(cx);
    // The launcher closes first.
    press(cx, &f, "cmd-k");
    press(cx, &f, "escape");
    cx.read(|cx| assert!(!f.view.read(cx).launcher_open));
    still_working(cx);
    // In Settings an open menu closes before Settings does.
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Settings, window, cx)
    });
    next_frame(cx, &f);
    on_window(cx, &f, |window, cx| window.click("status-layout", cx));
    on_window(cx, &f, |window, _| {
        assert!(window.within("popup-menu").try_find(0usize).is_some());
    });
    press(cx, &f, "escape");
    on_window(cx, &f, |window, cx| {
        assert!(
            window.try_find("popup-menu").is_none(),
            "the first Escape closes the menu"
        );
        assert!(f.view.read(cx).layout.settings_open(), "Settings stays");
    });
    // A dialog over Settings closes before Settings does.
    on_window(cx, &f, |window, cx| window.click("settings-keys", cx));
    press(cx, &f, "escape");
    on_window(cx, &f, |window, cx| {
        assert!(!window.has_active_dialog(cx));
        assert!(f.view.read(cx).layout.settings_open());
    });
    // Then Settings, from where the dialog returned focus.
    press(cx, &f, "escape");
    cx.read(|cx| assert!(!f.view.read(cx).layout.settings_open()));
    still_working(cx);
}

/// 900×600 in Write: no horizontal overflow with Chats or Notes in the
/// navigation area, comments stacked, and the launcher inside the window.
#[gpui_kit::test]
fn write_at_900x600_has_no_horizontal_overflow_and_the_launcher_fits(cx: &mut TestAppContext) {
    let mut note = String::new();
    let f = fixture(cx, 900., 600., |workspace| {
        note = workspace.new_note("Small window", "Saved text").unwrap().id;
        workspace.new_thread("Small chat").unwrap();
    });
    update(cx, &f, |view, window, cx| {
        view.apply_layout(Preset::Write, window, cx);
        view.open_note(&note, None, window, cx);
    });
    for navigation in [TileKind::Threads, TileKind::Notes] {
        update(cx, &f, |view, window, cx| {
            view.show_tile(navigation, window, cx);
            view.open_note(&note, None, window, cx);
        });
        on_window(cx, &f, |window, cx| {
            let view = f.view.read(cx);
            let kinds: Vec<TileKind> = view.layout.visible().iter().map(|t| t.kind).collect();
            assert!(kinds.contains(&navigation), "{kinds:?}");
            assert!(kinds.contains(&TileKind::Note) && kinds.contains(&TileKind::Chat));
            assert_eq!(
                view.writing.margin_mode,
                layout::MarginMode::Stacked,
                "{navigation:?}"
            );
            let overflow = view.tile_scroll.max_offset();
            assert!(
                overflow.x <= px(0.5),
                "{navigation:?} overflows: {overflow:?}"
            );
            let row = window.find("tile-row").bounds();
            assert!(row.right() <= px(f.width) + px(0.5), "{row:?}");
            for kind in kinds {
                assert_inside(window, &format!("tile-{}", kind.key()), row);
            }
        });
    }
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, _| {
        let launcher = window.within("launcher-scrim").find("launcher").bounds();
        let screen = window_bounds(&f);
        assert!(
            launcher.left() >= screen.left()
                && launcher.right() <= screen.right()
                && launcher.bottom() <= screen.bottom(),
            "{launcher:?}"
        );
    });
}

/// At 850×600 the keyboard-selected launcher row stays in view.
#[gpui_kit::test]
fn the_launcher_keeps_the_keyboard_selection_in_view_at_850x600(cx: &mut TestAppContext) {
    let f = fixture(cx, 850., 600., |workspace| {
        for index in 0..8 {
            workspace.new_thread(&format!("Chat {index}")).unwrap();
            workspace
                .new_note(&format!("Note {index}"), "Saved text")
                .unwrap();
        }
    });
    press(cx, &f, "cmd-k");
    let count = cx.read(|cx| f.view.read(cx).launcher_items(cx).len());
    assert!(count > 12, "{count} rows");
    for step in 1..count {
        press(cx, &f, "down");
        cx.read(|cx| assert_eq!(f.view.read(cx).launcher_index, step));
        on_window(cx, &f, |window, _| {
            let results = window.find("launcher-results").bounds();
            let row = window.find(format!("launcher-item-{step}")).bounds();
            assert!(
                row.top() >= results.top() - px(0.5) && row.bottom() <= results.bottom() + px(0.5),
                "row {step} {row:?} in {results:?}"
            );
        });
    }
    // Wrapping back to the first row shows it and its heading.
    press(cx, &f, "down");
    on_window(cx, &f, |window, _| {
        let results = window.find("launcher-results").bounds();
        let heading = window.find("launcher-group-chats").bounds();
        assert!(heading.top() >= results.top() - px(0.5), "{heading:?}");
    });
    // Moving up from the top wraps to the last row and keeps each in view.
    for step in (0..count).rev() {
        press(cx, &f, "up");
        on_window(cx, &f, |window, _| {
            let results = window.find("launcher-results").bounds();
            let row = window.find(format!("launcher-item-{step}")).bounds();
            assert!(
                row.top() >= results.top() - px(0.5) && row.bottom() <= results.bottom() + px(0.5),
                "row {step} {row:?} in {results:?}"
            );
        });
    }
}

/// Panel controls name their panel.
#[gpui_kit::test]
fn panel_close_and_maximize_controls_name_their_panel(cx: &mut TestAppContext) {
    let f = fixture(cx, 1400., 1000., |_| {});
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("tile-close-chat").label(), Some("Close Chat"));
        assert_eq!(window.find("tile-max-chat").label(), Some("Maximize Chat"));
        assert_eq!(
            window.find("tile-close-threads").label(),
            Some("Close Chats")
        );
    });
    on_window(cx, &f, |window, cx| window.click("tile-max-chat", cx));
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("tile-max-chat").label(), Some("Restore Chat"));
    });
}
