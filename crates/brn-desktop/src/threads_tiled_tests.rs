//! Offline app-wide component witnesses. OS focus, rendering and IME remain native acceptance.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    data: PathBuf,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
    note: Option<Record>,
    thread: Option<String>,
    width: f32,
    height: f32,
}

fn fixture(cx: &mut TestAppContext, seeded: bool, width: f32, height: f32) -> Fixture {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("tiled-offline");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let (note, thread) = if seeded {
        let note = workspace
            .new_note("Tiled saved note", "A saved harborneedle passage. õun")
            .unwrap();
        let thread = workspace.new_thread("Tiled conversation").unwrap().id;
        workspace
            .link_writing_thread("tiled-link", &note.id, note.version, &thread)
            .unwrap();
        (Some(note), Some(thread))
    } else {
        (None, None)
    };
    cx.update(gpui_kit::init);
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
        data,
        window,
        view,
        note,
        thread,
        width,
        height,
    }
}

fn click(cx: &mut TestAppContext, f: &Fixture, id: impl Into<String>) {
    let id = id.into();
    cx.update_window(f.window, |_, window, cx| window.click(id, cx))
        .unwrap();
    cx.run_until_parked();
}

/// Choose a popup menu row by its accessible label, as a mouse user would.
fn choose(cx: &mut TestAppContext, f: &Fixture, label: &str) {
    cx.update_window(f.window, |_, window, cx| {
        let mut menu = window.within("popup-menu");
        let index = (0..60usize)
            .find(|index| {
                menu.try_find(*index)
                    .is_some_and(|row| row.label() == Some(label))
            })
            .unwrap_or_else(|| panic!("menu row {label} not found"));
        menu.click(index, cx);
    })
    .unwrap();
    cx.run_until_parked();
}

fn query(cx: &mut TestAppContext, f: &Fixture, value: &str) {
    cx.update_window(f.window, |_, window, cx| {
        f.view
            .read(cx)
            .search
            .clone()
            .update(cx, |input, cx| input.focus(window, cx));
        window.press(super::focus_tests::SELECT_ALL, cx);
        window.press("backspace", cx);
        if !value.is_empty() {
            window.input(value, cx);
        }
    })
    .unwrap();
    cx.run_until_parked();
}

fn assert_in_window(window: &Window, f: &Fixture, id: &str) {
    let element = window.find(id.to_owned());
    assert!(element.visible(), "{id} must be reachable");
    let bounds = element.bounds();
    assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
    assert!(bounds.origin.x >= px(0.), "{id} starts outside the window");
    assert!(bounds.origin.y >= px(0.), "{id} starts outside the window");
    assert!(
        bounds.origin.x + bounds.size.width <= px(f.width + 1.),
        "{id} extends past the window width"
    );
    assert!(
        bounds.origin.y + bounds.size.height <= px(f.height + 1.),
        "{id} extends past the window height"
    );
}

fn reveal(cx: &mut TestAppContext, f: &Fixture, container: &str, target: &str) {
    // Exercise real scrolling rather than counting an offscreen test element as accessible.
    for _ in 0..20 {
        // Wholly inside the scroller, so a click at its center lands on it.
        let visible = cx
            .update_window(f.window, |_, window, _| {
                let shown = window.find(target.to_owned());
                let (item, frame) = (shown.bounds(), window.find(container.to_owned()).bounds());
                shown.visible()
                    && item.origin.y >= frame.origin.y
                    && item.origin.y + item.size.height <= frame.origin.y + frame.size.height
            })
            .unwrap();
        if visible {
            cx.update_window(f.window, |_, window, _| assert_in_window(window, f, target))
                .unwrap();
            return;
        }
        cx.update_window(f.window, |_, window, cx| {
            window.scroll(
                container.to_owned(),
                gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-120.))),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
    }
    panic!("{target} could not be reached by scrolling {container}");
}

fn update(
    cx: &mut TestAppContext,
    f: &Fixture,
    run: impl FnOnce(&mut ThreadsUi, &mut Window, &mut Context<ThreadsUi>),
) {
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| run(view, window, cx))
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn global_tiles_navigation_preserves_human_buffer_and_each_chat_draft(cx: &mut TestAppContext) {
    let f = fixture(cx, true, 1260., 1100.);
    let note = f.note.as_ref().unwrap();
    let thread = f.thread.as_ref().unwrap();
    update(cx, &f, |view, window, cx| {
        view.composer.update(cx, |editor, cx| {
            editor.set_value("Standalone unsent draft", window, cx)
        });
        view.open_note(&note.id, None, window, cx);
        view.select_writing_thread(note.id.clone(), thread.clone(), window, cx);
        view.composer.update(cx, |editor, cx| {
            editor.set_value("Selected chat unsent draft", window, cx)
        });
        view.writing.note_editor.update(cx, |editor, cx| {
            editor.set_value("Human unsaved draft õun", window, cx)
        });
    });
    let editor = cx.read(|cx| f.view.read(cx).writing.note_editor.clone());
    for (button, kind, tile) in [
        ("home", TileKind::Overview, "home-needs-you"),
        ("actions", TileKind::Actions, "actions-content"),
        ("history", TileKind::History, "history-content"),
        ("settings", TileKind::Settings, "settings-general"),
        ("open-notes", TileKind::Notes, "notes-list"),
    ] {
        click(cx, &f, button);
        cx.update_window(f.window, |_, window, cx| {
            assert!(window.try_find(tile).is_some(), "{tile}");
            let view = f.view.read(cx);
            assert_eq!(view.layout.focus(), Some(kind));
            assert_eq!(view.writing.note_editor, editor);
            assert_eq!(
                view.writing.note_editor.read(cx).value().as_ref(),
                "Human unsaved draft õun"
            );
            assert_eq!(
                view.composer.read(cx).value().as_ref(),
                "Selected chat unsent draft"
            );
            assert!(view.worker.is_none());
            assert_eq!(view.workspace.store.note(&note.id).unwrap().unwrap(), *note);
            assert!(view.workspace.messages(thread).unwrap().is_empty());
        })
        .unwrap();
        update(cx, &f, |view, _, cx| {
            view.layout.close(kind);
            cx.notify();
        });
    }
    update(cx, &f, |view, window, cx| view.new_thread(window, cx));
    cx.read(|cx| {
        assert_eq!(
            f.view.read(cx).composer.read(cx).value().as_ref(),
            "Standalone unsent draft"
        )
    });
    click(cx, &f, "open-notes");
    click(cx, &f, format!("note-{}", note.id));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.writing.note_editor.read(cx).value().as_ref(),
            "Human unsaved draft õun"
        );
        assert_eq!(
            view.composer.read(cx).value().as_ref(),
            "Standalone unsent draft"
        );
        assert!(view.writing.note.as_ref().unwrap().session().is_some());
        assert_eq!(view.workspace.store.note(&note.id).unwrap().unwrap(), *note);
    });
    let reopened = Workspace::open(&f.data).unwrap();
    assert_eq!(reopened.store.note(&note.id).unwrap().unwrap(), *note);
    assert!(reopened.messages(thread).unwrap().is_empty());
}

#[gpui_kit::test]
fn search_tiles_open_saved_hits_exclude_dirty_text_and_clear_to_navigation(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, true, 1400., 1100.);
    let note = f.note.as_ref().unwrap();
    click(cx, &f, "open-notes");
    click(cx, &f, format!("note-{}", note.id));
    update(cx, &f, |view, window, cx| {
        view.writing.note_editor.update(cx, |editor, cx| {
            editor.set_value("Unsaved scratchneedle", window, cx)
        });
    });
    query(cx, &f, "scratchneedle");
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("search-empty").visible());
        assert!(f.view.read(cx).hits.is_empty());
        assert!(f.view.read(cx).notice.text().is_empty());
    })
    .unwrap();
    query(cx, &f, "harborneedle");
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("search-results").visible());
        assert_eq!(f.view.read(cx).hits.len(), 1);
        assert_eq!(f.view.read(cx).hits[0].note, note.id);
        assert!(f.view.read(cx).hits[0].quote.contains("harborneedle"));
    })
    .unwrap();
    click(cx, &f, "hit-0");
    query(cx, &f, "");
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("search-results").is_none());
        assert!(window.find(format!("note-{}", note.id)).visible());
        assert!(window.find("notes-list").visible());
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(note.id.as_str()));
        assert!(view.hits.is_empty() && view.notice.text().is_empty());
        assert_eq!(
            view.writing.note_editor.read(cx).value().as_ref(),
            "Unsaved scratchneedle"
        );
        assert_eq!(view.workspace.store.note(&note.id).unwrap().unwrap(), *note);
    })
    .unwrap();
    click(cx, &f, "home");
    cx.read(|cx| assert_eq!(f.view.read(cx).layout.focus(), Some(TileKind::Overview)));
}

#[gpui_kit::test]
fn action_status_tile_writes_one_checked_receipt_and_history_undo_compensates(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, false, 1260., 1100.);
    let description = "Review every retained result and record the decision. ".repeat(8);
    let mut action = None;
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |view, cx| {
            let op = view
                .workspace
                .store
                .allocate_operation("tiled-action")
                .unwrap();
            let id = op.creation_id(0);
            view.workspace
                .owner_change(
                    "tiled-action",
                    &ChangeRequest {
                        reason: "Synthetic Action fixture".into(),
                        writes: vec![Put {
                            id: id.clone(),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Action(Action {
                                kept_outside_project: false,
                                chat: None,
                                description: description.clone(),
                                state: ActionState::Open,
                                internal: false,
                                evidence: None,
                                due: Some("2026-10-15".into()),
                            }),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            action = view.workspace.store.record(&id).unwrap();
            focus_tests::solo(view, TileKind::Actions, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    let action = action.unwrap();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("actions-group-open").visible());
        assert_eq!(
            window
                .find(format!("action-description-{}", action.id))
                .label(),
            Some(description.as_str())
        );
    })
    .unwrap();
    let before = cx.read(|cx| f.view.read(cx).workspace.history().unwrap().len());
    // Waiting moved into the task row's ⋯; Confirm done is the main action.
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find(format!("done-action-{}", action.id)).visible());
    })
    .unwrap();
    click(cx, &f, format!("task-menu-{}", action.id));
    choose(cx, &f, "Waiting");
    let receipt = cx.read(|cx| {
        let view = f.view.read(cx);
        let current = view.workspace.store.record(&action.id).unwrap().unwrap();
        assert_eq!(current.version, action.version + 1);
        assert!(matches!(
            current.data,
            RecordData::Action(Action {
                state: ActionState::Waiting,
                ..
            })
        ));
        let history = view.workspace.history().unwrap();
        assert_eq!(history.len(), before + 1);
        let receipt = history.last().unwrap().clone();
        assert_eq!(receipt.writes.len(), 1);
        assert_eq!(receipt.writes[0].after.id, action.id);
        assert_eq!(receipt.writes[0].before.as_ref(), Some(&action));
        receipt
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("actions-group-waiting").visible())
    })
    .unwrap();
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::History, cx)
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(
            window
                .find(format!("receipt-{}", receipt.operation.as_str()))
                .visible()
        )
    })
    .unwrap();
    click(cx, &f, format!("undo-{}", receipt.operation.as_str()));
    cx.read(|cx| {
        let view = f.view.read(cx);
        let current = view.workspace.store.record(&action.id).unwrap().unwrap();
        assert_eq!(current.data, action.data);
        assert_eq!(current.version, action.version + 2);
        let history = view.workspace.history().unwrap();
        assert_eq!(history.len(), before + 2);
        assert_eq!(
            history.last().unwrap().undo_of.as_ref(),
            Some(&receipt.operation)
        );
        assert_eq!(history[before].operation, receipt.operation);
        assert!(view.worker.is_none());
    });

    // Undo restored the original open Action. Exercise the row's actual
    // Confirm done callback and its own receipt-specific Undo.
    let (history_before_done, action_before_done) = cx.read(|cx| {
        let view = f.view.read(cx);
        (
            view.workspace.history().unwrap().len(),
            view.workspace.store.record(&action.id).unwrap().unwrap(),
        )
    });
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::Actions, cx)
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("actions-group-open").visible());
        assert!(window.find(format!("done-action-{}", action.id)).visible());
    })
    .unwrap();
    click(cx, &f, format!("done-action-{}", action.id));
    let done_receipt = cx.read(|cx| {
        let view = f.view.read(cx);
        let current = view.workspace.store.record(&action.id).unwrap().unwrap();
        assert_eq!(current.version, action.version + 3);
        let mut expected = match action.data.clone() {
            RecordData::Action(action) => action,
            _ => unreachable!(),
        };
        expected.state = ActionState::Done;
        expected.evidence = Some("Owner confirmed completion in BRN.".into());
        assert_eq!(current.data, RecordData::Action(expected));

        let history = view.workspace.history().unwrap();
        assert_eq!(history.len(), history_before_done + 1);
        let receipt = history.last().unwrap().clone();
        assert_eq!(receipt.writes.len(), 1);
        assert_eq!(receipt.writes[0].after.id, action.id);
        assert_eq!(receipt.writes[0].after, current);
        assert_eq!(receipt.writes[0].before.as_ref(), Some(&action_before_done));
        receipt
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("actions-group-done").visible());
    })
    .unwrap();
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::History, cx)
    });
    cx.update_window(f.window, |_, window, _| {
        let id = done_receipt.operation.as_str();
        assert!(window.find(format!("receipt-{id}")).visible());
        assert!(window.find(format!("undo-{id}")).visible());
    })
    .unwrap();
    click(cx, &f, format!("undo-{}", done_receipt.operation.as_str()));
    cx.read(|cx| {
        let view = f.view.read(cx);
        let current = view.workspace.store.record(&action.id).unwrap().unwrap();
        assert_eq!(current.data, action.data);
        assert_eq!(current.version, action.version + 4);
        let history = view.workspace.history().unwrap();
        assert_eq!(history.len(), history_before_done + 2);
        assert_eq!(
            history[history_before_done].operation,
            done_receipt.operation
        );
        assert_eq!(
            history[history_before_done + 1].undo_of.as_ref(),
            Some(&done_receipt.operation)
        );
        assert!(view.worker.is_none());
    });
}

#[gpui_kit::test]
fn settings_save_preserves_backup_and_requires_explicit_delegation_choice(cx: &mut TestAppContext) {
    let f = fixture(cx, false, 1260., 1100.);
    let backup = f._root.path().join("separate backups");
    std::fs::create_dir(&backup).unwrap();
    update(cx, &f, |view, window, cx| {
        view.workspace.choose_backup_directory(&backup).unwrap();
        focus_tests::settings(view, SettingsSection::Models, cx);
        view.model.update(cx, |input, cx| {
            input.set_value("offline-model-choice", window, cx)
        });
    });
    click(cx, &f, "effort-high");
    click(cx, &f, "save-settings");
    let saved = Workspace::open(&f.data).unwrap().settings().unwrap();
    assert_eq!(saved.provider, "offline-fixture");
    assert_eq!(saved.model, "offline-model-choice");
    assert_eq!(saved.effort, "high");
    assert_eq!(saved.backup_directory.as_deref(), backup.to_str());
    assert!(!saved.delegation_chosen && !saved.maintenance);
    assert!(saved.auth_file.is_none() && saved.credentials_dir.is_none());
    click(cx, &f, "settings-section-delegation");
    click(cx, &f, "delegation-review-first");
    click(cx, &f, "settings-section-models");
    click(cx, &f, "save-settings");
    let chosen = Workspace::open(&f.data).unwrap().settings().unwrap();
    assert!(chosen.delegation_chosen && chosen.maintenance && chosen.review_first);
    assert_eq!(chosen.backup_directory, saved.backup_directory);
    assert_eq!(chosen.model, saved.model);
    assert_eq!(chosen.effort, saved.effort);
    cx.read(|cx| assert!(f.view.read(cx).worker.is_none()));
}

#[gpui_kit::test]
fn empty_tiles_explain_home_search_actions_and_history_without_starting_work(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, false, 1260., 1100.);
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("note-empty").visible());
        assert!(window.find("threads-empty").visible());
        assert!(
            window.find("delegation-pending").visible(),
            "first use offers the choice"
        );
    })
    .unwrap();
    click(cx, &f, "home");
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("home-needs-you").visible());
        assert!(window.try_find("home-notes").is_some());
        assert!(window.try_find("home-recent-changes").is_some());
        assert!(
            window.find("delegation-pending").visible(),
            "rendered once, in the overview"
        );
    })
    .unwrap();
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::Notes, cx)
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("notes-empty").visible())
    })
    .unwrap();
    query(cx, &f, "NoSavedMatch");
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("search-empty").visible())
    })
    .unwrap();
    query(cx, &f, "");
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::Actions, cx)
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("actions-empty").visible())
    })
    .unwrap();
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::History, cx);
        // Bootstrap itself records settings receipts; exercise an empty displayed receipt list.
        view.history.clear();
    });
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("history-empty").visible());
        assert!(f.view.read(cx).worker.is_none());
        assert!(
            f.view
                .read(cx)
                .records
                .iter()
                .all(|record| !matches!(record.data, RecordData::Run(_)))
        );
    })
    .unwrap();
}

/// Gives a new chat a synthetic question so it needs the owner.
fn ask_owner(view: &mut ThreadsUi, title: &str) {
    let asked = view.workspace.new_thread(title).unwrap();
    view.workspace
        .owner_change(
            "tiled-attention",
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
                        title: title.into(),
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
    view.refresh();
}

#[gpui_kit::test]
fn compact_chrome_shortens_the_needs_badge_and_keeps_its_full_name(cx: &mut TestAppContext) {
    let mut widths = vec![];
    for width in [1260., 900.] {
        let f = fixture(cx, false, width, 700.);
        update(cx, &f, |view, _, cx| {
            ask_owner(view, "Needs a decision");
            cx.notify();
        });
        cx.update_window(f.window, |_, window, _| {
            let badge = window.find("needs-badge");
            assert_eq!(badge.label(), Some("1 needs you"), "{width}");
            assert_in_window(window, &f, "needs-badge");
            widths.push(badge.bounds().size.width);
            let search = window.find("launcher");
            assert_eq!(search.label(), Some("Search notes, chats and commands"));
            if width < 1000. {
                assert!(
                    search.bounds().size.width <= px(32.),
                    "{:?}",
                    search.bounds()
                );
            } else {
                assert!(
                    search.bounds().size.width >= px(150.),
                    "{:?}",
                    search.bounds()
                );
            }
        })
        .unwrap();
    }
    assert!(widths[1] < widths[0], "Compact reads ● 1: {widths:?}");
}

#[gpui_kit::test]
fn narrow_settings_and_global_controls_remain_reachable_by_scrolling(cx: &mut TestAppContext) {
    let f = fixture(cx, false, 850., 600.);
    cx.update_window(f.window, |_, window, _| {
        for id in [
            "launcher",
            "layout-write",
            "layout-chat",
            "layout-solo",
            "threads-new",
            "home",
            "actions",
            "history",
            "settings",
            "status-layout",
            "status-keys",
            "open-beside",
        ] {
            assert_in_window(window, &f, id);
        }
        // Compact chrome: icon controls keep their full accessible names.
        assert!(window.try_find("status-theme").is_none());
        let search = window.find("launcher");
        assert_eq!(search.label(), Some("Search notes, chats and commands"));
        assert!(
            search.bounds().size.width <= px(32.),
            "{:?}",
            search.bounds()
        );
        let beside = window.find("open-beside");
        assert_eq!(beside.label(), Some("Open beside"));
        assert!(
            beside.bounds().size.width <= px(32.),
            "{:?}",
            beside.bounds()
        );
    })
    .unwrap();
    reveal(cx, &f, "tile-row", "composer-add");
    click(cx, &f, "settings");
    click(cx, &f, "settings-section-models");
    reveal(cx, &f, "settings-content", "connect-copilot");
    reveal(cx, &f, "settings-content", "save-settings");
    click(cx, &f, "save-settings");
    cx.read(|cx| {
        assert!(
            !f.view
                .read(cx)
                .workspace
                .settings()
                .unwrap()
                .delegation_chosen
        );
        assert!(f.view.read(cx).worker.is_none());
    });
    click(cx, &f, "settings-section-delegation");
    reveal(cx, &f, "settings-content", "delegation-review-first");
    click(cx, &f, "delegation-review-first");
    click(cx, &f, "settings-section-data");
    reveal(cx, &f, "settings-content", "export-now");
    reveal(cx, &f, "settings-content", "backup-change-folder");
    reveal(cx, &f, "settings-content", "backup-now");
    cx.read(|cx| {
        let settings = f.view.read(cx).workspace.settings().unwrap();
        assert!(settings.delegation_chosen && settings.review_first);
        assert!(settings.backup_directory.is_none());
        assert!(f.view.read(cx).worker.is_none());
    });
}

#[gpui_kit::test]
fn copilot_run_with_unoffered_model_does_not_start(cx: &mut TestAppContext) {
    let f = fixture(cx, false, 1260., 860.);
    update(cx, &f, |view, _, cx| {
        let mut settings = view.settings.clone();
        settings.provider = "copilot".into();
        settings.model = "gpt-6.1-sol".into();
        view.workspace.configure(settings).unwrap();
        view.copilot_models = vec!["synthetic-copilot-model".into()];
        view.start_run("hello".into(), "synthetic-thread".into(), vec![], None, cx);
        assert!(view.worker.is_none());
        assert!(view.notice.text().contains("not offered by GitHub Copilot"));
        assert!(view.notice.text().contains("synthetic-copilot-model"));
    });
}

#[gpui_kit::test]
fn search_pages_show_exact_counts_context_and_ignore_superseded_queries(cx: &mut TestAppContext) {
    let f = fixture(cx, false, 1400., 1100.);
    update(cx, &f, |view, _, _| {
        for i in 0..14 {
            view.workspace
                .new_note(
                    &format!("Curlew {i:02}"),
                    &format!("# Field log\n\nA curlew sighting number {i}."),
                )
                .unwrap();
        }
        view.workspace.new_note("Kestrel", "Kestrel only.").unwrap();
        view.refresh();
    });
    click(cx, &f, "open-notes");
    // Two queries before any answer: only the newer one may be shown.
    update(cx, &f, |view, window, cx| {
        view.search
            .clone()
            .update(cx, |input, cx| input.set_value("curlews", window, cx));
        view.search_changed("kestrel".into(), window, cx);
        view.search_changed("curlews".into(), window, cx);
    });
    cx.update_window(f.window, |_, window, cx| {
        let view = f.view.read(cx);
        assert_eq!(view.hits.len(), search_view::PAGE);
        assert!(view.hits.iter().all(|hit| hit.title.starts_with("Curlew")));
        let first = &view.hits[0];
        assert_eq!(first.heading_path, "Field log");
        assert!(first.snippet.contains("curlew"));
        assert_eq!(&first.snippet[first.highlights[0].clone()], "curlew");
        assert!(window.find("search-status").visible());
    })
    .unwrap();
    let page = |cx: &mut TestAppContext| {
        cx.read(|cx| {
            let page = f.view.read(cx).results.page.clone().unwrap();
            (page.total, page.offset, page.hits.len())
        })
    };
    assert_eq!(page(cx), (14, 0, 12));
    reveal(cx, &f, "notes-list", "search-next");
    click(cx, &f, "search-next");
    assert_eq!(page(cx), (14, 12, 2));
    reveal(cx, &f, "notes-list", "search-previous");
    click(cx, &f, "search-previous");
    assert_eq!(page(cx), (14, 0, 12));
    query(cx, &f, "zzznothing");
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("search-empty").visible());
        assert!(f.view.read(cx).results.error.is_none());
    })
    .unwrap();
}

fn message_id(view: &ThreadsUi, chat: &str, text: &str) -> String {
    view.workspace
        .messages(chat)
        .unwrap()
        .into_iter()
        .find(|r| matches!(&r.data, RecordData::Message(m) if m.text == text))
        .unwrap()
        .id
}

/// F-UI1 (round 5 native recheck, section F): typing "berth" in Notes lists
/// the saved chat reply under "In chats" with its chat, role and highlighted
/// snippet; activating it opens the chat scrolled to that Message; after
/// Archive the hit stays, labelled Archived (owner decision 3).
#[gpui_kit::test]
fn notes_search_lists_saved_chat_messages_and_opens_their_chat(cx: &mut TestAppContext) {
    let f = fixture(cx, false, 1400., 1100.);
    let berth = "Berth survey reply draft 1.4 m";
    let mut chat = String::new();
    let mut message = String::new();
    update(cx, &f, |view, _, _| {
        chat = view.workspace.new_thread("Replies").unwrap().id;
        view.workspace.post_message(&chat, berth).unwrap();
        // Enough later Messages that the transcript opens far below it.
        for i in 0..30 {
            view.workspace
                .post_message(
                    &chat,
                    &format!(
                        "Later unrelated message {i}. {}",
                        "Filler words. ".repeat(20)
                    ),
                )
                .unwrap();
        }
        message = message_id(view, &chat, berth);
        view.refresh();
    });
    click(cx, &f, "open-notes");
    query(cx, &f, "berth");
    cx.update_window(f.window, |_, window, cx| {
        let view = f.view.read(cx);
        assert!(view.hits.is_empty());
        assert_eq!(view.results.message_hits().len(), 1);
        // Before: only the note-text empty state, nothing for the Message.
        assert!(window.find("search-empty").visible());
        assert!(window.find("search-chats").visible());
        assert!(window.find("search-chats-status").visible());
        assert_eq!(window.find("chat-hit-0").label(), Some("Replies · You"));
        assert_eq!(window.find("chat-hit-0-snippet").label(), Some(berth));
        assert!(window.try_find("chat-hit-0-archived").is_none());
        // Chat Messages never count as note hits.
        let page = view.results.page.clone().unwrap();
        assert_eq!((page.total, page.message_total), (0, 1));
    })
    .unwrap();
    // Archived: the same hit, labelled (owner decision 3).
    update(cx, &f, |view, _, cx| view.resolve_thread(&chat, cx));
    query(cx, &f, "berth survey");
    cx.update_window(f.window, |_, window, _| {
        assert_eq!(
            window.find("chat-hit-0").label(),
            Some("Replies · You · Archived")
        );
        assert!(window.find("chat-hit-0-archived").visible());
    })
    .unwrap();
    reveal(cx, &f, "notes-list", "chat-hit-0");
    click(cx, &f, "chat-hit-0");
    cx.update_window(f.window, |_, window, cx| {
        let view = f.view.read(cx);
        assert_eq!(view.chat, ChatTarget::Thread(chat.clone()));
        assert!(view.reveal_message.borrow().is_none(), "revealed once");
        let row = window.find(format!("chat-message-{message}"));
        assert!(row.visible(), "the transcript shows the matching Message");
    })
    .unwrap();
}

/// F-UI1: note hits and chat Messages page independently.
#[gpui_kit::test]
fn note_hits_and_chat_messages_page_independently(cx: &mut TestAppContext) {
    let f = fixture(cx, false, 1400., 1100.);
    update(cx, &f, |view, _, _| {
        for i in 0..14 {
            view.workspace
                .new_note(
                    &format!("Curlew {i:02}"),
                    &format!("A curlew sighting {i}."),
                )
                .unwrap();
        }
        let chat = view.workspace.new_thread("Field chat").unwrap().id;
        for i in 0..14 {
            view.workspace
                .post_message(&chat, &format!("Curlew seen at the marsh, entry {i}"))
                .unwrap();
        }
        view.refresh();
    });
    click(cx, &f, "open-notes");
    query(cx, &f, "curlew");
    let pages = |cx: &mut TestAppContext| {
        cx.read(|cx| {
            let results = &f.view.read(cx).results;
            let notes = results.page.clone().unwrap();
            let chats = results.chats.clone().unwrap();
            (
                (notes.total, notes.offset, notes.hits.len()),
                (chats.message_total, chats.offset, chats.messages.len()),
            )
        })
    };
    assert_eq!(pages(cx), ((14, 0, 12), (14, 0, 12)));
    reveal(cx, &f, "notes-list", "search-chats-next");
    click(cx, &f, "search-chats-next");
    assert_eq!(pages(cx), ((14, 0, 12), (14, 12, 2)));
    cx.read(|cx| assert_eq!(f.view.read(cx).hits.len(), 12, "note hits stay"));
    reveal(cx, &f, "notes-list", "search-next");
    click(cx, &f, "search-next");
    assert_eq!(pages(cx), ((14, 12, 2), (14, 12, 2)));
    reveal(cx, &f, "notes-list", "search-chats-previous");
    click(cx, &f, "search-chats-previous");
    assert_eq!(pages(cx), ((14, 12, 2), (14, 0, 12)));
    // While a notes page loads, chat paging stays available and "In chats"
    // stays in view; with both lanes paging, the group shows its own
    // loading line rather than disappearing. Nothing answers until parked.
    reveal(cx, &f, "notes-list", "search-previous");
    cx.update_window(f.window, |_, window, cx| {
        window.click("search-previous", cx);
        window.render_frame(cx);
        assert!(window.find("search-status").visible(), "notes are loading");
        assert!(window.find("search-chats").visible());
        let next = window.find("search-chats-next");
        assert_eq!(next.label(), Some("Next →"), "not waiting on notes");
        // The note hits gave way to their loading line: scroll to the chats'
        // Next without letting either request answer.
        for _ in 0..20 {
            let (item, frame) = (
                window.find("search-chats-next").bounds(),
                window.find("notes-list").bounds(),
            );
            if window.find("search-chats-next").visible()
                && item.top() >= frame.top()
                && item.bottom() <= frame.bottom()
            {
                break;
            }
            let up = if item.top() < frame.top() {
                120.
            } else {
                -120.
            };
            window.scroll(
                "notes-list",
                gpui_kit::ScrollDelta::Pixels(point(px(0.), px(up))),
                cx,
            );
        }
        window.click("search-chats-next", cx);
        window.render_frame(cx);
        assert!(window.find("search-chats").visible(), "both lanes paging");
        assert_eq!(
            window.find("search-chats-status").label(),
            Some("Searching chats…")
        );
    })
    .unwrap();
    cx.run_until_parked();
    assert_eq!(pages(cx), ((14, 0, 12), (14, 12, 2)));
    // A new query starts both lanes at their first page.
    query(cx, &f, "curlew sighting");
    cx.read(|cx| {
        let results = &f.view.read(cx).results;
        assert_eq!(results.page.as_ref().unwrap().offset, 0);
        assert_eq!(results.chats.as_ref().unwrap().offset, 0);
    });
}

/// Owner decision 3: a note hit from an archived chat carries the same
/// Archived chip as a Message hit, never a different wording. The Notes
/// search has no project scope, where the service lists such notes, so the
/// hit's `archived` field is set here directly.
#[gpui_kit::test]
fn an_archived_chats_note_hit_reads_archived_like_its_messages(cx: &mut TestAppContext) {
    let f = fixture(cx, false, 1400., 1100.);
    update(cx, &f, |view, _, _| {
        view.workspace
            .new_note("Curlew attachment", "A curlew at berth 4.")
            .unwrap();
        view.refresh();
    });
    click(cx, &f, "open-notes");
    query(cx, &f, "curlew");
    cx.update_window(f.window, |_, window, _| {
        assert_eq!(window.find("hit-0").label(), Some("Curlew attachment"));
        assert!(window.try_find("hit-0-archived").is_none());
    })
    .unwrap();
    update(cx, &f, |view, _, cx| {
        view.hits[0].archived = true;
        cx.notify();
    });
    cx.update_window(f.window, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(
            window.find("hit-0").label(),
            Some("Curlew attachment · Archived")
        );
        assert_eq!(window.find("hit-0-archived").label(), Some("Archived"));
        assert!(window.find("hit-0-archived").visible());
    })
    .unwrap();
}

fn owner_write(view: &mut ThreadsUi, key: &str, data: Vec<RecordData>) -> Vec<Record> {
    let op = view.workspace.store.allocate_operation(key).unwrap();
    let writes = data
        .into_iter()
        .enumerate()
        .map(|(i, data)| Put {
            id: op.creation_id(i),
            expected_version: None,
            archived: false,
            data,
        })
        .collect();
    let owner = HostAuthority::owner("fixture");
    view.workspace
        .store
        .prepare_owner(
            &op,
            &ChangeRequest {
                reason: key.into(),
                writes,
                inputs: vec![],
            },
            &owner,
        )
        .unwrap();
    let ApplyOutcome::Applied(receipt) = view.workspace.store.apply(&op, &owner).unwrap() else {
        panic!("fixture write did not apply")
    };
    receipt.writes.into_iter().map(|w| w.after).collect()
}

fn link_data(from: &str, to: &str, relation: &str) -> RecordData {
    RecordData::Link(Link {
        from: from.into(),
        to: to.into(),
        relation: relation.into(),
    })
}

#[gpui_kit::test]
fn topics_section_creates_the_topics_note_and_lists_linked_topic_pages(cx: &mut TestAppContext) {
    let f = fixture(cx, true, 1400., 1100.);
    click(cx, &f, "open-notes");
    reveal(cx, &f, "notes-list", "create-topics");
    click(cx, &f, "create-topics");
    let topics = cx.read(|cx| f.view.read(cx).topics.clone().unwrap());
    cx.read(|cx| assert_eq!(f.view.read(cx).open_note_id(), Some(topics.note.as_str())));
    let page = update_returning(cx, &f, |view| {
        let page = owner_write(
            view,
            "page",
            vec![RecordData::Note(Note::working(
                "Harbor area",
                "Delivery planning.\n",
            ))],
        )
        .remove(0);
        owner_write(
            view,
            "link",
            vec![link_data(&topics.note, &page.id, relations::RELATED)],
        );
        view.refresh();
        page
    });
    click(cx, &f, "open-notes");
    reveal(cx, &f, "notes-list", &format!("topic-{}", page.id));
    click(cx, &f, format!("topic-{}", page.id));
    cx.read(|cx| assert_eq!(f.view.read(cx).open_note_id(), Some(page.id.as_str())));
}

fn update_returning<T>(
    cx: &mut TestAppContext,
    f: &Fixture,
    run: impl FnOnce(&mut ThreadsUi) -> T,
) -> T {
    let out = cx.update(|cx| f.view.update(cx, |view, _| run(view)));
    cx.run_until_parked();
    out
}

#[gpui_kit::test]
fn connections_tile_lists_visible_links_filters_relations_and_recenters(cx: &mut TestAppContext) {
    let f = fixture(cx, true, 1600., 1100.);
    let centre = f.note.as_ref().unwrap().id.clone();
    let thread = f.thread.as_ref().unwrap().clone();
    let (evidence, related, second_hop) = update_returning(cx, &f, |view| {
        let notes = owner_write(
            view,
            "notes",
            vec![
                RecordData::Note(Note::working("Signed reference", "Evidence.\n")),
                RecordData::Note(Note::working("Operations", "Related.\n")),
                RecordData::Note(Note {
                    chat: Some(thread.clone()),
                    ..Note::working("Private draft", "Chat only.\n")
                }),
                RecordData::Note(Note::working(
                    "Second-hop-only page",
                    "Reached through Operations.\n",
                )),
            ],
        );
        owner_write(
            view,
            "links",
            vec![
                link_data(&centre, &notes[0].id, relations::SOURCE),
                link_data(&notes[1].id, &centre, relations::RELATED),
                link_data(&notes[2].id, &centre, relations::RELATED),
                link_data(&notes[1].id, &notes[3].id, relations::RELATED),
            ],
        );
        view.refresh();
        (
            notes[0].id.clone(),
            notes[1].id.clone(),
            notes[3].id.clone(),
        )
    });
    update(cx, &f, |view, window, cx| {
        view.open_note(&centre, None, window, cx)
    });
    click(cx, &f, "note-connections");
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("connections-status").visible());
        assert!(window.find(format!("connection-{evidence}")).visible());
        assert!(window.find(format!("connection-{related}")).visible());
        let view = f.view.read(cx);
        let found = view.graph.result.as_ref().unwrap();
        assert_eq!(
            found.hops[0].available, 2,
            "the chat-only link is not counted"
        );
    })
    .unwrap();
    click(cx, &f, "connections-filter-source");
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find(format!("connection-{evidence}")).visible());
        assert!(window.try_find(format!("connection-{related}")).is_none());
    })
    .unwrap();
    click(cx, &f, "connections-filter-all");

    // Dispatch the real control but inspect its immediate frame before the
    // fixture's usual run_until_parked drains the matching request.
    cx.update_window(f.window, |_, window, cx| {
        window.click("connections-depth-2", cx);
        assert!(
            window
                .try_find("connections-pending")
                .is_some_and(|pending| pending.visible()),
            "changing depth must hide the previous result and show pending"
        );
        assert!(window.try_find("connections-status").is_none());
        assert!(window.try_find(format!("connection-{related}")).is_none());
        assert!(
            window
                .try_find(format!("connection-{second_hop}"))
                .is_none()
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find("connections-status").visible());
        assert!(window.find(format!("connection-{related}")).visible());
        assert!(window.find(format!("connection-{second_hop}")).visible());
        assert!(window.try_find("connections-pending").is_none());
        f.view.read_with(cx, |view, _| {
            let result = view.graph.result.as_ref().unwrap();
            assert_eq!(result.centre.id, centre);
            assert_eq!(result.hops.len(), 2);
            let reached = result.hops[1]
                .shown
                .iter()
                .find(|node| node.id == second_hop)
                .expect("the independent second-hop Note is present");
            assert_eq!(reached.via.as_deref(), Some(related.as_str()));
        });
    })
    .unwrap();
    click(cx, &f, format!("connection-{related}"));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(related.as_str()));
        assert_eq!(view.graph.result.as_ref().unwrap().centre.id, related);
        assert_eq!(view.graph.result.as_ref().unwrap().hops.len(), 2);
    });
}

#[gpui_kit::test]
fn knowledge_check_runs_off_the_ui_thread_and_seeds_a_review_chat(cx: &mut TestAppContext) {
    let f = fixture(cx, true, 1400., 1100.);
    update_returning(cx, &f, |view| {
        owner_write(
            view,
            "dupes",
            vec![
                RecordData::Note(Note::working("Twin page", "Same text.\n")),
                RecordData::Note(Note::working("Twin pages", "Same text.\n")),
            ],
        );
        view.refresh();
    });
    click(cx, &f, "home");
    reveal(cx, &f, "home-content", "knowledge-run");
    click(cx, &f, "knowledge-run");
    let check = cx.read(|cx| f.view.read(cx).knowledge.result.clone().unwrap());
    assert_eq!(check.counts.get(&FindingKind::DuplicateTitle), Some(&1));
    reveal(cx, &f, "home-content", "knowledge-review");
    click(cx, &f, "knowledge-review");
    cx.read(|cx| {
        let view = f.view.read(cx);
        let chat = view.chat_thread().unwrap().to_owned();
        let messages = view.workspace.messages(&chat).unwrap();
        assert!(
            matches!(&messages[0].data, RecordData::Message(m) if m.text.contains("DuplicateTitle"))
        );
        assert!(view.worker.is_none(), "seeding a review starts no run");
    });
}
