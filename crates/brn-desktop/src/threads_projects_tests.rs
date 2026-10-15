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
        let (sender, receiver) = mpsc::sync_channel(8); let cancel = CancellationToken::new(); ui.worker = Some(Worker { receiver, cancel: cancel.clone(), conversion_cancel: Arc::new(AtomicBool::new(false)), kind: WorkerKind::Run, thread: Some(f.chat.clone()), run_id: Some(run) }); ui.stream = "Displayed useful partial · õun".into();
        let preview = ui.workspace.project_delete_preview(&p).unwrap(); let receipt = ui.apply_project(ProjectCommand::Delete { preview, actions: BTreeMap::new() }, cx).unwrap();
        assert!(receipt.writes.iter().any(|w| w.after.archived && matches!(&w.after.data, RecordData::Message(m) if m.role == "assistant-partial" && m.text == "Displayed useful partial · õun"))); assert!(cancel.is_cancelled()); assert!(ui.queue_state().paused);
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
