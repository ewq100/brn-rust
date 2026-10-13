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

fn query(cx: &mut TestAppContext, f: &Fixture, value: &str) {
    cx.update_window(f.window, |_, window, cx| {
        f.view
            .read(cx)
            .search
            .clone()
            .update(cx, |input, cx| input.focus(window, cx));
        window.press("cmd-a", cx);
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
        let visible = cx
            .update_window(f.window, |_, window, _| {
                window.find(target.to_owned()).visible()
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
            editor.set_value("Selected thread unsent draft", window, cx)
        });
        view.note_editor.update(cx, |editor, cx| {
            editor.set_value("Human unsaved draft õun", window, cx)
        });
    });
    let editor = cx.read(|cx| f.view.read(cx).note_editor.clone());
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
            assert_eq!(view.note_editor, editor);
            assert_eq!(
                view.note_editor.read(cx).value().as_ref(),
                "Human unsaved draft õun"
            );
            assert_eq!(
                view.composer.read(cx).value().as_ref(),
                "Selected thread unsent draft"
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
            view.note_editor.read(cx).value().as_ref(),
            "Human unsaved draft õun"
        );
        assert_eq!(
            view.composer.read(cx).value().as_ref(),
            "Selected thread unsent draft"
        );
        assert!(view.note.as_ref().unwrap().session.is_some());
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
        view.note_editor.update(cx, |editor, cx| {
            editor.set_value("Unsaved scratchneedle", window, cx)
        });
    });
    query(cx, &f, "scratchneedle");
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("search-empty").visible());
        assert!(f.view.read(cx).hits.is_empty());
        assert!(f.view.read(cx).notice.is_empty());
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
        assert!(window.find("workspace-nav").visible());
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(note.id.as_str()));
        assert!(view.hits.is_empty() && view.notice.is_empty());
        assert_eq!(
            view.note_editor.read(cx).value().as_ref(),
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
    click(cx, &f, format!("wait-action-{}", action.id));
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
            "status-theme",
            "status-keys",
        ] {
            assert_in_window(window, &f, id);
        }
    })
    .unwrap();
    reveal(cx, &f, "tile-row", "composer-add");
    click(cx, &f, "settings");
    click(cx, &f, "layout-solo");
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
        assert!(view.notice.contains("not offered by GitHub Copilot"));
        assert!(view.notice.contains("synthetic-copilot-model"));
    });
}
