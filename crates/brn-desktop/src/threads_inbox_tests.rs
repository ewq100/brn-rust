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
                    // The folder inbox lives in Inbox → Imports.
                    ui.layout.open(TileKind::Imports);
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

#[gpui_kit::test]
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
fn scan_button_runs_stock_worker_and_reopens_import_without_dispatching_private_queue(
    cx: &mut TestAppContext,
) {
    use std::time::{Duration, Instant};
    let f = fixture(cx);
    let bytes = include_bytes!("../../brn-intake/tests/fixtures/harbor.docx");
    let source_path = f.folder.join("harbor.docx");
    std::fs::write(&source_path, bytes).unwrap();
    std::fs::write(f.folder.join("broken.docx"), b"not a document").unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.workspace
                .choose_document_conversion(DocumentConversionSettings {
                    converter: DocumentConverter::Xberg,
                    ai: false,
                })
                .unwrap();
            // Synthetic picker result only; the real consent and Scan controls run below.
            ui.inbox_folder_chosen(Ok(Some(vec![f.folder.clone()])), window, cx);
            assert!(ui.workspace.inbox().unwrap().is_none());
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        window.click("inbox-consent-confirm", cx)
    })
    .unwrap();
    cx.run_until_parked();
    let before = cx.read(|cx| {
        let ui = f.view.read(cx);
        assert!(ui.worker.is_none());
        assert!(ui.workspace.inbox().unwrap().unwrap().entries.is_empty());
        ui.workspace.records().unwrap()
    });
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("inbox-scan").visible());
        window.click("inbox-scan", cx);
        assert!(
            f.view.read(cx).worker.is_some(),
            "Scan must create the real worker"
        );
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let done = cx
            .update_window(f.window, |_, window, cx| {
                f.view.update(cx, |ui, cx| {
                    ui.poll_worker(window, cx);
                    ui.worker.is_none()
                })
            })
            .unwrap();
        if done {
            break;
        }
        if Instant::now() >= deadline {
            cx.update_window(f.window, |_, _, cx| {
                f.view.update(cx, |ui, cx| ui.cancel(cx))
            })
            .unwrap();
            panic!("Stock inbox worker did not finish within twenty seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(
            f.view.read(cx).notice_tone(),
            Tone::Warning,
            "a scan with a failed file is not reported as a success"
        )
    });
    let reopened = Workspace::open_existing_no_upgrade(f._root.path().join("data")).unwrap();
    let mut state = reopened.inbox().unwrap().unwrap();
    assert_eq!(state.entries.len(), 2);
    let failed = state
        .entries
        .iter()
        .position(|entry| entry.name == "broken.docx")
        .unwrap();
    let failed = state.entries.remove(failed);
    assert_eq!(failed.status, InboxStatus::Failed, "{failed:?}");
    assert!(failed.note.is_none());
    assert_eq!(state.entries[0].status, InboxStatus::Imported, "{state:?}");
    let key = state.entries[0].operation_key.as_ref().unwrap();
    let operation = reopened
        .store
        .operation_by_host_key(&format!("full-import:{key}"))
        .unwrap()
        .unwrap();
    assert!(reopened.store.receipt(&operation).unwrap().is_some());
    let records = reopened.records().unwrap();
    for record in &before {
        assert!(
            records.contains(record),
            "pre-existing private context/queue changed: {}",
            record.id
        );
    }
    assert!(
        records
            .iter()
            .all(|r| !matches!(r.data, RecordData::Run(_)))
    );
    let general: Vec<_> = records
        .iter()
        .filter_map(|r| match &r.data {
            RecordData::Note(n) if n.chat.is_none() => Some((r, n)),
            _ => None,
        })
        .collect();
    assert_eq!(general.len(), 1);
    let (record, note) = general[0];
    assert_eq!(state.entries[0].note.as_ref(), Some(&record.id));
    assert!(note.protected);
    assert!(!note.markdown.contains("PRIVATE-INBOX-SENTINEL"));
    let provenance = note
        .import
        .as_ref()
        .unwrap()
        .report
        .as_ref()
        .unwrap()
        .conversion
        .as_ref()
        .unwrap();
    assert_eq!(
        provenance.settings,
        DocumentConversionSettings {
            converter: DocumentConverter::Xberg,
            ai: false
        }
    );
    assert_eq!(
        provenance.engine_identity,
        brn_threads_intake::expected_engine_identity("xberg").unwrap()
    );
    let assets: Vec<_> = records
        .iter()
        .filter_map(|r| match &r.data {
            RecordData::Asset(a) => Some((r, a)),
            _ => None,
        })
        .collect();
    assert_eq!(assets.len(), 1);
    assert_eq!(
        assets[0].1.bytes,
        include_bytes!("../../brn-intake/tests/fixtures/water-use.png")
    );
    assert!(
        note.markdown
            .contains(&format!("brn-asset://{}", assets[0].0.id))
    );
    assert_eq!(std::fs::read(source_path).unwrap(), bytes);
}
