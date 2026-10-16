//! Real native dialogs and checked services with disposable offline data.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};
struct Fixture {
    _root: tempfile::TempDir,
    view: Entity<ThreadsUi>,
    window: gpui_kit::AnyWindowHandle,
    chat: String,
    note: String,
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(|cx| {
        gpui_kit::init(cx);
        // Keep dialog hit targets at their resting positions before simulated clicks.
        cx.set_reduce_motion(true);
    });
    cx.update(|cx| cx.bind_keys(bindings()));
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("projects");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let chat = workspace.new_thread("Existing conversation").unwrap().id;
    let note = workspace
        .new_note("Shared reference", "One canonical reference")
        .unwrap()
        .id;
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1450.), px(1000.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data, window, cx);
                    ui.poll = None;
                    ui.open_thread(&chat, window, cx);
                    ui.composer
                        .update(cx, |e, cx| e.set_value("Unsent draft · õun", window, cx));
                    ui.show_tile(TileKind::Projects, window, cx);
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    Fixture {
        _root: root,
        view,
        window,
        chat,
        note,
    }
}
fn click(cx: &mut TestAppContext, f: &Fixture, id: impl Into<String>) {
    cx.update_window(f.window, |_, window, cx| window.click(id.into(), cx))
        .unwrap();
    cx.run_until_parked();
}
/// Choose a row of the open popup menu by its accessible label.
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
/// Run a selected project's action from the Projects ⋯.
fn project_action(cx: &mut TestAppContext, f: &Fixture, label: &str) {
    click(cx, f, "tile-menu-projects");
    choose(cx, f, label);
}
fn create(cx: &mut TestAppContext, f: &Fixture, name: &str) -> String {
    click(cx, f, "project-new");
    click(cx, f, "project-name-input");
    cx.simulate_input(f.window, name);
    cx.run_until_parked();
    click(cx, f, "project-name-submit");
    cx.read(|cx| f.view.read(cx).selected_project.clone().unwrap())
}
#[gpui_kit::test]
fn project_creation_move_and_settings_preserve_saved_chat_and_draft(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let p = create(cx, &f, "Japan trip");
    cx.update_window(f.window, |_, window, cx| {
        f.view
            .update(cx, |ui, cx| ui.move_chat_dialog(&f.chat, window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    click(cx, &f, "move-chat-1");
    project_action(cx, &f, "Instructions & settings");
    click(cx, &f, "project-instructions");
    cx.simulate_input(f.window, "For beginners, in Estonian");
    cx.run_until_parked();
    click(cx, &f, "project-knowledge-toggle");
    click(cx, &f, "project-settings-save");
    cx.read(|cx| { let ui = f.view.read(cx); let r = ui.workspace.store.record(&p).unwrap().unwrap(); assert!(matches!(r.data, RecordData::Project(p) if p.chats == [f.chat.clone()] && !p.use_brn_knowledge && p.instructions == "For beginners, in Estonian")); assert_eq!(ui.composer.read(cx).value(), "Unsent draft · õun"); });
}
#[gpui_kit::test]
fn linked_document_survives_project_delete_and_trash_restore(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let p = create(cx, &f, "Travel writing");
    project_action(cx, &f, "Link document…");
    click(cx, &f, format!("project-link-doc-{}", f.note));
    project_action(cx, &f, "Move to Trash…");
    click(cx, &f, "project-delete-confirm");
    cx.read(|cx| {
        let ui = f.view.read(cx);
        assert!(ui.workspace.store.record(&p).unwrap().unwrap().archived);
        assert!(
            !ui.workspace
                .store
                .record(&f.note)
                .unwrap()
                .unwrap()
                .archived
        );
    });
    click(cx, &f, "project-trash");
    click(cx, &f, format!("project-row-{p}"));
    click(cx, &f, "project-restore");
    cx.read(|cx| {
        let ui = f.view.read(cx);
        let r = ui.workspace.store.record(&p).unwrap().unwrap();
        assert!(!r.archived);
        assert!(matches!(r.data, RecordData::Project(p) if p.documents == [f.note.clone()]));
        assert_eq!(ui.composer.read(cx).value(), "Unsent draft · õun");
    });
}
#[gpui_kit::test]
fn exclusive_project_note_opens_and_writing_chat_inherits_project(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let p = create(cx, &f, "Japan");
    project_action(cx, &f, "New document");
    click(cx, &f, "project-name-input");
    cx.simulate_input(f.window, "Itinerary");
    cx.run_until_parked();
    click(cx, &f, "project-name-submit");
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            let note = ui.open_note_id().unwrap().to_owned();
            assert!(ui.workspace.notes().unwrap().iter().all(|r| r.id != note));
            ui.new_writing_thread(window, cx);
            let writing = ui.writing_threads[&note].clone();
            let membership = ui
                .workspace
                .store
                .project_context_record(&writing)
                .unwrap()
                .unwrap();
            assert_eq!(membership.id, p);
        })
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn a_shared_private_document_uses_the_selected_projects_writing_chat(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| f.view.update(cx, |ui, cx| {
        let a = ui.apply_project(ProjectCommand::Create { title: "Origin".into() }, cx).unwrap().writes[0].after.id.clone();
        let b = ui.apply_project(ProjectCommand::Create { title: "Destination".into() }, cx).unwrap().writes[0].after.id.clone();
        let source_chat = ui.apply_project(ProjectCommand::CreateChat { project: a, title: "Private original chat".into() }, cx).unwrap().writes.into_iter().find(|w| matches!(w.after.data, RecordData::Thread(_))).unwrap().after.id;
        let op = ui.workspace.store.allocate_operation("shared-private-doc").unwrap(); let note = op.creation_id(0); let mut data = Note::working("Private reference", "Only the linked document is shared"); data.chat = Some(source_chat.clone());
        ui.workspace.owner_change("shared-private-doc", &ChangeRequest { reason: "Owner attachment".into(), inputs: vec![], writes: vec![Put { id: note.clone(), expected_version: None, archived: false, data: RecordData::Note(data) }] }).unwrap();
        ui.apply_project(ProjectCommand::LinkDocument { project: b.clone(), note: note.clone() }, cx).unwrap();
        ui.open_project_note(&b, &note, window, cx); ui.new_writing_thread(window, cx);
        let writing = ui.writing_threads[&note].clone(); assert_ne!(writing, source_chat); assert_eq!(ui.workspace.store.project_context_record(&writing).unwrap().unwrap().id, b);
        assert!(matches!(ui.workspace.store.record(&note).unwrap().unwrap().data, RecordData::Note(n) if n.chat == Some(source_chat)));
    })).unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn native_project_delete_preserves_displayed_partial_and_pauses_even_an_empty_queue(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| f.view.update(cx, |ui, cx| {
        let p = ui.apply_project(ProjectCommand::Create { title: "Working project".into() }, cx).unwrap().writes[0].after.id.clone(); ui.apply_project(ProjectCommand::MoveChat { chat: f.chat.clone(), project: Some(p.clone()) }, cx).unwrap();
        let settings = ui.workspace.settings().unwrap(); let op = ui.workspace.store.allocate_operation("project-synthetic-worker").unwrap(); let run = op.creation_id(0);
        ui.workspace.owner_change("project-synthetic-worker", &ChangeRequest { reason: "Offline displayed worker".into(), inputs: vec![], writes: vec![Put { id: run.clone(), expected_version: None, archived: false, data: RecordData::Run(Run { instruction: None, queue_claim: None, thread: f.chat.clone(), provider: settings.provider, model: settings.model, guide_identity: "synthetic".into(), budget: 1, effort: settings.effort, loaded_skills: vec![], fence: 1, state: RunState::Working, progress: "Saved useful summary".into(), operations: vec![], sources: vec![] }) }] }).unwrap();
        // The displayed worker is live, so it holds the run's lease as a real worker does.
        let lease = brn_ai::run_lease::RunLease::acquire(&ui.workspace.store.derived_directory().unwrap(), &run).unwrap();
        let (sender, receiver) = mpsc::sync_channel(8); let cancel = CancellationToken::new(); ui.worker = Some(Worker { receiver, cancel: cancel.clone(), conversion_cancel: Arc::new(AtomicBool::new(false)), kind: WorkerKind::Run, thread: Some(f.chat.clone()), run_id: Some(run.clone()) }); ui.stream = "Displayed useful partial · õun".into();
        let preview = ui.workspace.project_delete_preview(&p).unwrap(); let receipt = ui.apply_project(ProjectCommand::Delete { preview, actions: BTreeMap::new() }, cx).unwrap();
        assert!(receipt.writes.iter().any(|w| w.after.archived && matches!(&w.after.data, RecordData::Message(m) if m.role == "assistant-partial" && m.text == "Displayed useful partial · õun"))); assert!(cancel.is_cancelled()); assert!(ui.queue_state().paused);
        // The owner acted on the project, not in the chat: the stopped Run leaves its Interrupted notice in the chat's own write, live or not.
        assert!(receipt.writes.iter().any(|w| w.after.id == f.chat && matches!(&w.after.data, RecordData::Thread(t) if t.attention.iter().any(|a| a.reason == brn_ai::threads::INTERRUPTED_ATTENTION))));
        drop(lease);
        sender.send(WorkerEvent::RunFinished { result: Ok("Old worker returned".into()), terminal: None }).unwrap(); ui.poll_worker(window, cx); assert!(ui.queue_state().paused);
    })).unwrap();
    cx.run_until_parked();
}

#[test]
fn each_projects_view_has_at_most_one_primary() {
    use projects_view::projects_primary;
    assert_eq!(projects_primary(false, false), Some("project-new"));
    assert_eq!(projects_primary(false, true), Some("project-chat"));
    assert_eq!(projects_primary(true, true), Some("project-restore"));
    assert_eq!(projects_primary(true, false), None);
}

#[test]
fn the_delete_preview_names_only_the_other_kinds_present() {
    use projects_view::other_items_line;
    let record = |id: &str, data: RecordData| Record {
        id: id.into(),
        version: 1,
        archived: false,
        data,
    };
    let note = record("n", RecordData::Note(Note::working("Brief", "text")));
    assert_eq!(other_items_line(std::slice::from_ref(&note)), None);
    let link = record(
        "l",
        RecordData::Link(Link {
            from: "n".into(),
            to: "m".into(),
            relation: "related".into(),
        }),
    );
    let message = record(
        "m",
        RecordData::Message(Message {
            thread: "t".into(),
            role: "owner".into(),
            text: "Hello".into(),
        }),
    );
    assert_eq!(
        other_items_line(&[note, link.clone(), message.clone(), link]),
        Some("Also moves 3 other items (messages, links).".into())
    );
    assert_eq!(
        other_items_line(&[message]),
        Some("Also moves 1 other item (messages).".into())
    );
}

#[gpui_kit::test]
fn projects_primaries_follow_the_view_and_trash_restores_from_active_and_trash(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("project-new").visible());
        assert!(window.find("project-active").visible());
        assert!(window.try_find("project-chat").is_none());
        assert!(window.find("projects-empty").visible());
    })
    .unwrap();
    let p = create(cx, &f, "Website refresh");
    project_action(cx, &f, "Link document…");
    click(cx, &f, format!("project-link-doc-{}", f.note));
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("project-chat").visible());
        assert!(
            window.find("project-new").visible(),
            "New project stays, quieter"
        );
        assert_eq!(
            window.find("project-status").label(),
            Some("BRN knowledge: on · No project instructions")
        );
    })
    .unwrap();
    // The delete dialog states the consequence and ↵ never confirms.
    project_action(cx, &f, "Move to Trash…");
    cx.update_window(f.window, |_, window, cx| {
        assert_eq!(
            window.find("project-delete-moves").label(),
            Some("Moves to Trash: Website refresh.")
        );
        assert_eq!(
            window.find("project-delete-stays").label(),
            Some("Stays active: Shared reference.")
        );
        assert!(window.find("project-delete-cancel").visible());
        assert!(window.find("project-delete-confirm").visible());
        window.press("enter", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.has_active_dialog(cx), "↵ leaves the dialog open");
        let ui = f.view.read(cx);
        assert!(!ui.workspace.store.record(&p).unwrap().unwrap().archived);
    })
    .unwrap();
    click(cx, &f, "project-delete-confirm");
    click(cx, &f, "project-trash");
    cx.update_window(f.window, |_, window, _| {
        assert!(
            window.try_find("project-restore").is_none(),
            "no primary yet"
        );
        assert!(window.try_find("project-new").is_none());
        assert!(window.find("projects-trash-hint").visible());
    })
    .unwrap();
    click(cx, &f, format!("project-row-{p}"));
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("project-restore").visible());
        assert_eq!(
            window.find("project-restore-line").label(),
            Some(
                "Restoring returns contents and settings. Cancelled tasks stay cancelled; tasks kept outside stay outside. Work stays paused."
            )
        );
    })
    .unwrap();
    click(cx, &f, "project-active");
    cx.read(|cx| {
        let ui = f.view.read(cx);
        assert!(!ui.projects_trash);
        assert!(ui.workspace.store.record(&p).unwrap().unwrap().archived);
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("project-new").visible());
        assert!(window.try_find(format!("project-row-{p}")).is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn the_projects_page_keeps_its_column_when_maximized(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let p = create(cx, &f, "Website refresh");
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |view, cx| {
            focus_tests::solo(view, TileKind::Projects, cx)
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        let page = window.find("projects-tile").bounds();
        assert!(page.size.width > px(1000.), "maximized: {page:?}");
        // Pages hold one column of at most 720 px, as Home, Tasks and Changes do.
        for id in [format!("project-row-{p}"), "project-status".into()] {
            let line = window.find(id.clone()).bounds();
            assert!(line.size.width <= px(720.), "{id}: {line:?}");
        }
    })
    .unwrap();
}

fn dialog_open(cx: &mut TestAppContext, f: &Fixture) -> bool {
    cx.update_window(f.window, |_, window, cx| window.has_active_dialog(cx))
        .unwrap()
}
fn press(cx: &mut TestAppContext, f: &Fixture, key: &str) {
    cx.update_window(f.window, |_, window, cx| window.press(key, cx))
        .unwrap();
    cx.run_until_parked();
}
fn archived(cx: &mut TestAppContext, f: &Fixture, id: &str) -> bool {
    cx.read(|cx| {
        f.view
            .read(cx)
            .workspace
            .store
            .record(id)
            .unwrap()
            .unwrap()
            .archived
    })
}

/// The destructive dialog: ↵ moves nothing, even on the focused confirm;
/// Escape closes it; Space on the confirm, reached by Tab, moves the
/// project to Trash when no task choice is pending.
#[gpui_kit::test]
fn project_delete_dialog_confirms_only_by_space_on_its_danger_button(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let p = create(cx, &f, "Website refresh");
    project_action(cx, &f, "Move to Trash…");
    press(cx, &f, "enter");
    assert!(dialog_open(cx, &f), "↵ leaves the dialog open");
    assert!(!archived(cx, &f, &p), "↵ moves nothing");
    press(cx, &f, "escape");
    assert!(!dialog_open(cx, &f), "Escape closes it");
    assert!(!archived(cx, &f, &p));
    project_action(cx, &f, "Move to Trash…");
    let reached = (0..12).any(|_| {
        press(cx, &f, "tab");
        cx.update_window(f.window, |_, window, _| {
            window.find("project-delete-confirm").focused() == Some(true)
        })
        .unwrap()
    });
    assert!(reached, "Tab reaches the danger button");
    press(cx, &f, "enter");
    assert!(dialog_open(cx, &f), "↵ on the focused confirm is inert");
    assert!(!archived(cx, &f, &p));
    press(cx, &f, "space");
    assert!(!dialog_open(cx, &f));
    assert!(archived(cx, &f, &p), "Space presses the focused confirm");
}

/// Escape on a dialog opened from a Projects ⋯ item gives focus back to the
/// Projects panel (the focus table's landing), so ⌘K and Tab work at once.
/// The menu that opened the dialog is gone by then.
#[gpui_kit::test]
fn escape_from_a_projects_menu_dialog_keeps_the_keyboard_working(cx: &mut TestAppContext) {
    let f = fixture(cx);
    create(cx, &f, "Focus trip");
    for label in [
        "Instructions & settings",
        "New document",
        "Link document…",
        "Move to Trash…",
    ] {
        project_action(cx, &f, label);
        assert!(dialog_open(cx, &f), "{label} opens a dialog");
        press(cx, &f, "escape");
        assert!(!dialog_open(cx, &f), "Escape closes {label}");
        cx.update_window(f.window, |_, window, cx| {
            let view = f.view.read(cx);
            assert!(
                view.panel_body(TileKind::Projects)
                    .contains_focused(window, cx),
                "after {label}, focus is in the Projects panel"
            );
        })
        .unwrap();
        press(cx, &f, "cmd-k");
        cx.read(|cx| assert!(f.view.read(cx).launcher_open, "⌘K after {label}"));
        press(cx, &f, "escape");
        press(cx, &f, "tab");
        cx.update_window(f.window, |_, window, cx| {
            assert!(
                f.view.read(cx).root_focus.contains_focused(window, cx),
                "Tab after {label} moves within the workspace"
            );
        })
        .unwrap();
    }
}

/// Link document… is a pick list: Tab reaches a document row and Space
/// links it; ↵ does nothing there.
#[gpui_kit::test]
fn link_document_rows_are_reached_by_tab_and_pressed_by_space(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let p = create(cx, &f, "Keyboard trip");
    project_action(cx, &f, "Link document…");
    let row = format!("project-link-doc-{}", f.note);
    let reached = (0..12).any(|_| {
        press(cx, &f, "tab");
        cx.update_window(f.window, |_, window, _| {
            window.find(row.clone()).focused() == Some(true)
        })
        .unwrap()
    });
    assert!(reached, "Tab reaches the document row");
    let linked = |cx: &mut TestAppContext| {
        cx.read(|cx| {
            let record = f.view.read(cx).workspace.store.record(&p).unwrap().unwrap();
            matches!(&record.data, RecordData::Project(project) if project.documents == [f.note.clone()])
        })
    };
    press(cx, &f, "enter");
    assert!(dialog_open(cx, &f), "↵ leaves the pick list open");
    assert!(!linked(cx));
    press(cx, &f, "space");
    assert!(!dialog_open(cx, &f));
    assert!(linked(cx), "Space on the focused row links the document");
}

/// Project instructions are a multi-line form: ↵ adds a line, ⌘↵ saves.
#[gpui_kit::test]
fn project_settings_dialog_adds_a_line_on_enter_and_saves_on_cmd_enter(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let p = create(cx, &f, "Japan trip");
    project_action(cx, &f, "Instructions & settings");
    click(cx, &f, "project-instructions");
    cx.simulate_input(f.window, "For beginners");
    cx.simulate_keystrokes(f.window, "enter");
    cx.simulate_input(f.window, "In Estonian");
    cx.run_until_parked();
    assert!(dialog_open(cx, &f));
    cx.simulate_keystrokes(f.window, "cmd-enter");
    cx.run_until_parked();
    assert!(!dialog_open(cx, &f));
    cx.read(|cx| {
        let record = f.view.read(cx).workspace.store.record(&p).unwrap().unwrap();
        assert!(
            matches!(&record.data, RecordData::Project(project) if project.instructions == "For beginners\nIn Estonian"),
            "{:?}",
            record.data
        );
    });
}
