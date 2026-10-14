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
    cx.update(gpui_kit::init);
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
    click(cx, &f, "project-settings");
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
    click(cx, &f, "project-link");
    click(cx, &f, format!("project-link-doc-{}", f.note));
    click(cx, &f, "project-delete");
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
    click(cx, &f, "project-note");
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
