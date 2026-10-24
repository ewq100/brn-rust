//! Offline native-state witnesses: picker is not consent, and inbox work cannot dispatch a private queue.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};
struct Fixture {
    _root: tempfile::TempDir,
    folder: PathBuf,
    view: Entity<ThreadsUi>,
    window: gpui_kit::AnyWindowHandle,
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
    });
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let folder = root.path().join("inbox");
    std::fs::create_dir(&folder).unwrap();
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let project = workspace
        .project_command(
            "inbox-private-project",
            ProjectCommand::Create {
                title: "Private project".into(),
            },
        )
        .unwrap()
        .writes[0]
        .after
        .id
        .clone();
    let chat = workspace
        .project_command(
            "inbox-private-chat",
            ProjectCommand::CreateChat {
                project,
                title: "Private discussion".into(),
            },
        )
        .unwrap()
        .writes
        .into_iter()
        .find(|w| matches!(w.after.data, RecordData::Thread(_)))
        .unwrap()
        .after
        .id;
    let op = workspace
        .store
        .allocate_operation("inbox-private-note")
        .unwrap();
    let mut private = Note::working("Private sentinel", "PRIVATE-INBOX-SENTINEL");
    private.chat = Some(chat.clone());
    workspace
        .owner_change(
            "inbox-private-note",
            &ChangeRequest {
                reason: "Synthetic private context".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(private),
                }],
            },
        )
        .unwrap();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1260.), px(1500.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data, window, cx);
                    ui.poll = None;
                    ui.open_thread(&chat, window, cx);
                    ui.settings_section = SettingsSection::Data;
                    ui.layout.open(TileKind::Settings);
                    ui.workspace
                        .queue_message(
                            "inbox-private-queued",
                            &chat,
                            "Private queued instruction must not start",
                        )
                        .unwrap();
                    ui.refresh();
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    Fixture {
        _root: root,
        folder,
        view,
        window,
    }
}
#[gpui_kit::test]
fn picker_alone_does_not_authorize_scan_and_general_confirmation_does_not_import(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.start_inbox_scan(cx);
            assert!(ui.worker.is_none());
            assert!(ui.workspace.inbox().unwrap().is_none());
            ui.inbox_folder_chosen(Ok(Some(vec![f.folder.clone()])), window, cx);
            assert!(ui.workspace.inbox().unwrap().is_none());
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("inbox-consent-confirm").visible());
        window.click("inbox-consent-cancel", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            assert!(ui.workspace.inbox().unwrap().is_none());
            ui.inbox_folder_chosen(Ok(Some(vec![f.folder.clone()])), window, cx);
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        window.click("inbox-consent-confirm", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let ui = f.view.read(cx);
        let state = ui.workspace.inbox().unwrap().unwrap();
        assert_eq!(state.destination, InboxDestination::General);
        assert!(state.entries.is_empty());
        assert!(ui.worker.is_none());
        assert!(
            ui.workspace
                .records()
                .unwrap()
                .iter()
                .all(|r| !matches!(&r.data, RecordData::Run(_)))
        );
    });
    // Exercise the same checked service used by the worker while a private chat
    // remains selected. This deterministic conversion receives only folder bytes.
    std::fs::write(f.folder.join("chosen.txt"), "Chosen public source only").unwrap();
    cx.update_window(f.window,|_,_,cx|f.view.update(cx,|ui,_cx|{
        let private_before=ui.workspace.records().unwrap().into_iter().find(|r|matches!(&r.data,RecordData::Note(n) if n.markdown=="PRIVATE-INBOX-SENTINEL")).unwrap();
        let summary=ui.workspace.scan_inbox(&AtomicBool::new(false),|doc,settings|{
            assert_eq!(doc.bytes,b"Chosen public source only");
            Ok(ConversionResult{intent:ImportIntent::FullNote,markdown:String::from_utf8(doc.bytes.clone()).unwrap(),assets:vec![],source_sha256:brn_intake::digest(&doc.bytes),source_reference:doc.source_reference,converter:brn_threads_intake::document_conversion_label(match settings.converter {DocumentConverter::Xberg=>"xberg",DocumentConverter::XbergBrn=>"xberg_brn"},settings.ai).unwrap(),sources:vec![],coverage:brn_threads_intake::Coverage{status:brn_threads_intake::CoverageStatus::Unchecked,checked_items:vec![],gaps:vec![]}})
        }).unwrap();assert_eq!(summary.imported,1);
        let records=ui.workspace.records().unwrap();assert!(records.contains(&private_before));
        let general:Vec<_>=records.iter().filter_map(|r|match &r.data{RecordData::Note(n) if n.chat.is_none()=>Some(n),_=>None}).collect();
        assert_eq!(general.len(),1);assert_eq!(general[0].markdown,"Chosen public source only");assert!(general[0].protected);
        assert!(records.iter().all(|r|!matches!(r.data,RecordData::Run(_))));
    })).unwrap();
}
#[gpui_kit::test]
fn inbox_finish_cancel_and_disconnect_leave_private_queue_unchanged(cx: &mut TestAppContext) {
    let f = fixture(cx);
    for action in ["finish", "cancel", "disconnect"] {
        cx.update_window(f.window, |_, window, cx| {
            f.view.update(cx, |ui, cx| {
                let before = ui.workspace.records().unwrap();
                let history = ui.workspace.history().unwrap();
                let (sender, receiver) = mpsc::sync_channel(4);
                let flag = Arc::new(AtomicBool::new(false));
                let cancel = CancellationToken::new();
                ui.worker = Some(Worker::inbox(receiver, cancel.clone(), flag.clone()));
                match action {
                    "finish" => {
                        sender
                            .send(WorkerEvent::Finished(Ok("Synthetic inbox finished".into())))
                            .unwrap();
                        ui.poll_worker(window, cx);
                        assert!(ui.worker.is_none());
                    }
                    "cancel" => {
                        ui.cancel(cx);
                        assert!(cancel.is_cancelled());
                        assert!(flag.load(std::sync::atomic::Ordering::Acquire));
                        ui.worker = None;
                    }
                    _ => {
                        drop(sender);
                        ui.poll_worker(window, cx);
                        assert!(ui.worker.is_none());
                    }
                }
                assert_eq!(ui.workspace.records().unwrap(), before);
                assert_eq!(ui.workspace.history().unwrap(), history);
            })
        })
        .unwrap();
    }
}
