//! Synthetic component controls and captured handoffs; no OS/native-human observation.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

#[gpui_kit::test]
fn documents_controls_persist_independently_and_do_not_rewrite_notes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let note = workspace.new_note("Unchanged", "Original content").unwrap();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1400.), px(1600.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    ui.layout.open(TileKind::Settings);
                    ui.settings_section = SettingsSection::Documents;
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(window.find("documents-xberg").visible());
        assert!(window.find("documents-ai-off").visible());
        assert_eq!(
            view.read(cx).settings.document_conversion,
            DocumentConversionSettings::default()
        );
        window.click("documents-xberg-brn", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.click("documents-ai-on", cx))
        .unwrap();
    cx.run_until_parked();
    let reopened = Workspace::open_existing_no_upgrade(&data).unwrap();
    assert_eq!(
        reopened.settings().unwrap().document_conversion,
        DocumentConversionSettings {
            converter: DocumentConverter::XbergBrn,
            ai: true
        }
    );
    assert_eq!(
        reopened
            .records()
            .unwrap()
            .into_iter()
            .find(|r| r.id == note.id)
            .unwrap(),
        note
    );
    cx.update_window(window, |_, window, cx| window.click("documents-xberg", cx))
        .unwrap();
    cx.run_until_parked();
    assert!(
        Workspace::open_existing_no_upgrade(&data)
            .unwrap()
            .settings()
            .unwrap()
            .document_conversion
            .ai
    );
    cx.update_window(window, |_, window, cx| window.click("documents-ai-off", cx))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(
        Workspace::open_existing_no_upgrade(&data)
            .unwrap()
            .settings()
            .unwrap()
            .document_conversion,
        DocumentConversionSettings::default()
    );
}

#[gpui_kit::test]
fn documents_completion_uses_captured_settings_and_stored_provenance(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                ui.poll = None;
                ui
            })
        })
        .unwrap()
    });
    let captured = DocumentConversionSettings::default();
    let changed = DocumentConversionSettings {
        converter: DocumentConverter::XbergBrn,
        ai: true,
    };
    Workspace::open_existing_no_upgrade(&data)
        .unwrap()
        .choose_document_conversion(changed.clone())
        .unwrap();
    let (sender, receiver) = mpsc::sync_channel(2);
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |ui, cx| {
            ui.worker = Some(Worker::import(
                receiver,
                CancellationToken::new(),
                Arc::new(AtomicBool::new(false)),
            ));
            sender
                .send(WorkerEvent::Converted {
                    result: Box::new(Ok(ConversionResult {
                        intent: ImportIntent::FullNote,
                        markdown: "Captured stock result".into(),
                        assets: vec![],
                        source_sha256: [9; 32],
                        source_reference: "/synthetic/snapshot.txt".into(),
                        converter: brn_threads_intake::document_conversion_label("xberg", false)
                            .unwrap(),
                        sources: vec![],
                        coverage: brn_threads_intake::Coverage {
                            status: brn_threads_intake::CoverageStatus::Unchecked,
                            checked_items: vec![],
                            gaps: vec![],
                        },
                    })),
                    title: "Snapshot".into(),
                    chat: None,
                    settings: captured.clone(),
                    useful_project: None,
                })
                .unwrap();
            ui.poll_worker(window, cx);
            assert_eq!(ui.settings.document_conversion, changed);
            let report = ui
                .records
                .iter()
                .find_map(|r| match &r.data {
                    RecordData::Note(n) => n.import.as_ref().and_then(|i| i.report.as_ref()),
                    _ => None,
                })
                .unwrap();
            assert_eq!(report.conversion.as_ref().unwrap().settings, captured);
            assert!(documents::report_summary(report).contains("Xberg · AI conversion: Off"));
            assert!(!documents::report_summary(report).contains("BRN layer"));
            let mut historical = report.clone();
            historical.conversion = None;
            assert_eq!(documents::report_summary(&historical), historical.summary());
            assert_eq!(
                documents::report_details(&historical),
                historical
                    .issues
                    .iter()
                    .map(ImportIssue::display)
                    .collect::<Vec<_>>()
            );
        })
    })
    .unwrap();
}

#[gpui_kit::test]
fn documents_useful_handoff_keeps_snapshot_and_settings_defer_while_working(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let mut workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    let mut settings = workspace.settings().unwrap();
    settings.provider = "Codex".into();
    settings.model = "gpt-6.1-sol".into();
    settings.effort = "medium".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    workspace.configure(settings).unwrap();
    let thread = workspace.new_thread("Working control").unwrap().id;
    let held = workspace
        .prepare_thread_run(ThreadRunStart {
            thread,
            prompt: "Synthetic held work".into(),
            request_message: None,
            queue_claim: None,
            owner_request: None,
            sources: vec![],
            max_tool_rounds: Some(2),
            trace: brn_ai::threads::TraceLevel::Off,
            offered_copilot_models: None,
        })
        .unwrap();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                ui.poll = None;
                ui
            })
        })
        .unwrap()
    });
    cx.update_window(window, |_, _, cx| {
        view.update(cx, |ui, cx| {
            ui.choose_document_conversion(Some(DocumentConverter::XbergBrn), Some(true), cx);
            assert_eq!(
                ui.settings.document_conversion,
                DocumentConversionSettings::default()
            );
        })
    })
    .unwrap();
    drop(held);
    // Explicitly recover the synthetic held Run before starting the offline useful agent.
    Workspace::open_existing_no_upgrade(&data)
        .unwrap()
        .recover_queue("documents-held-recovery")
        .unwrap();
    let captured = DocumentConversionSettings::default();
    Workspace::open_existing_no_upgrade(&data)
        .unwrap()
        .choose_document_conversion(DocumentConversionSettings {
            converter: DocumentConverter::XbergBrn,
            ai: true,
        })
        .unwrap();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |ui, cx| {
            ui.useful_intake(
                ConversionResult {
                    intent: ImportIntent::UsefulInformation,
                    markdown: "Synthetic useful body".into(),
                    assets: vec![],
                    source_sha256: [11; 32],
                    source_reference: "/synthetic/useful.txt".into(),
                    converter: brn_threads_intake::document_conversion_label("xberg", false)
                        .unwrap(),
                    sources: vec![],
                    coverage: brn_threads_intake::Coverage {
                        status: brn_threads_intake::CoverageStatus::Unchecked,
                        checked_items: vec![],
                        gaps: vec![],
                    },
                },
                "Useful snapshot".into(),
                captured.clone(),
                None,
                window,
                cx,
            );
        })
    })
    .unwrap();
    for _ in 0..1000 {
        let done = cx
            .update_window(window, |_, window, cx| {
                view.update(cx, |ui, cx| {
                    ui.poll_worker(window, cx);
                    ui.worker.is_none()
                })
            })
            .unwrap();
        if done {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    cx.update_window(window, |_, _, cx| assert!(view.read(cx).worker.is_none()))
        .unwrap();
    let records = Workspace::open_existing_no_upgrade(&data)
        .unwrap()
        .records()
        .unwrap();
    let source = records
        .iter()
        .find_map(|r| match &r.data {
            RecordData::Source(s) if s.locator == "/synthetic/useful.txt" => Some(s),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        source
            .report
            .as_ref()
            .unwrap()
            .conversion
            .as_ref()
            .unwrap()
            .settings,
        captured
    );
}

#[gpui_kit::test]
#[cfg(any(target_os = "macos", all(target_os = "linux", target_arch = "x86_64")))]
fn documents_native_worker_runs_selected_helper_without_credentials(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let source = root.path().join("document.txt");
    std::fs::write(&source, b"Native source\r\n").unwrap();
    let mut workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                ui.poll = None;
                ui
            })
        })
        .unwrap()
    });
    for converter in [DocumentConverter::Xberg, DocumentConverter::XbergBrn] {
        Workspace::open_existing_no_upgrade(&data)
            .unwrap()
            .choose_document_conversion(DocumentConversionSettings {
                converter,
                ai: false,
            })
            .unwrap();
        cx.update_window(window, |_, _, cx| {
            view.update(cx, |ui, cx| {
                ui.convert_file(
                    data.clone(),
                    source.clone(),
                    ImportIntent::FullNote,
                    None,
                    cx,
                )
            })
        })
        .unwrap();
        for _ in 0..1000 {
            let done = cx
                .update_window(window, |_, window, cx| {
                    view.update(cx, |ui, cx| {
                        ui.poll_worker(window, cx);
                        ui.worker.is_none()
                    })
                })
                .unwrap();
            if done {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        cx.update_window(window, |_, _, cx| assert!(view.read(cx).worker.is_none()))
            .unwrap();
        let records = Workspace::open_existing_no_upgrade(&data)
            .unwrap()
            .records()
            .unwrap();
        let note = records
            .iter()
            .find_map(|r| match &r.data {
                RecordData::Note(n)
                    if n.import
                        .as_ref()
                        .and_then(|i| i.report.as_ref())
                        .and_then(|r| r.conversion.as_ref())
                        .is_some_and(|p| p.settings.converter == converter) =>
                {
                    Some(n)
                }
                _ => None,
            })
            .unwrap();
        assert!(note.protected);
        assert!(note.markdown.contains("Native source"));
        assert_eq!(
            note.import
                .as_ref()
                .unwrap()
                .report
                .as_ref()
                .unwrap()
                .conversion
                .as_ref()
                .unwrap()
                .settings,
            DocumentConversionSettings {
                converter,
                ai: false
            }
        );
    }
}

fn useful_navigation(cx: &mut TestAppContext, from_project: bool, to_project: bool, remove: bool) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("data");
    let source = root.path().join("scope.txt");
    let bytes = b"Source stays in its requested scope\n";
    std::fs::write(&source, bytes).unwrap();
    let mut workspace =
        Workspace::create_without_default_credentials(&data, WorkspacePurpose::Test).unwrap();
    let mut settings = workspace.settings().unwrap();
    settings.provider = "Codex".into();
    settings.model = "gpt-6.1-sol".into();
    settings.effort = "medium".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    workspace.configure(settings).unwrap();
    let mut projects = Vec::new();
    let mut chats = Vec::new();
    for name in ["A", "B"] {
        let project = workspace
            .project_command(
                &format!("project-{name}"),
                ProjectCommand::Create { title: name.into() },
            )
            .unwrap()
            .writes[0]
            .after
            .id
            .clone();
        let chat = workspace
            .project_command(
                &format!("chat-{name}"),
                ProjectCommand::CreateChat {
                    project: project.clone(),
                    title: name.into(),
                },
            )
            .unwrap()
            .writes
            .into_iter()
            .find(|w| matches!(w.after.data, RecordData::Thread(_)))
            .unwrap()
            .after
            .id;
        projects.push(project);
        chats.push(chat);
    }
    let general = workspace.new_thread("General").unwrap().id;
    let first = if from_project {
        chats[0].clone()
    } else {
        general.clone()
    };
    let next = if to_project {
        chats[1].clone()
    } else {
        general
    };
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                ui.poll = None;
                ui.chat = ChatTarget::Thread(first);
                ui.refresh();
                ui
            })
        })
        .unwrap()
    });
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |ui, cx| {
            ui.convert_file(
                data.clone(),
                source.clone(),
                ImportIntent::UsefulInformation,
                None,
                cx,
            );
            assert!(ui.worker.is_some());
            // Conversion may finish in the background, but completion is deliberately not polled yet.
            ui.set_chat(ChatTarget::Thread(next), window, cx);
        })
    })
    .unwrap();
    if remove {
        let mut other = Workspace::open_existing_no_upgrade(&data).unwrap();
        let preview = other.project_delete_preview(&projects[0]).unwrap();
        other
            .project_command(
                "delete-captured-project",
                ProjectCommand::Delete {
                    preview,
                    actions: BTreeMap::new(),
                },
            )
            .unwrap();
    }
    for _ in 0..1000 {
        let done = cx
            .update_window(window, |_, window, cx| {
                view.update(cx, |ui, cx| {
                    ui.poll_worker(window, cx);
                    ui.worker.is_none()
                })
            })
            .unwrap();
        if done {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let records = Workspace::open_existing_no_upgrade(&data)
        .unwrap()
        .records()
        .unwrap();
    let saved = records.iter().find_map(|r| match &r.data {
        RecordData::Source(s) if s.locator == source.to_str().unwrap() => Some(s),
        _ => None,
    });
    if let Some(path) = std::env::var_os("BRN_TASK3_CAPTURES") {
        let path = PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(
            path.join(format!("navigation-{from_project}-{to_project}-{remove}.json")),
            serde_json::to_vec_pretty(&serde_json::json!({"from_project":from_project,"to_project":to_project,"removed":remove,"projects":projects,"records":records})).unwrap(),
        ).unwrap();
    }
    if remove {
        assert!(
            saved.is_none(),
            "a deleted captured destination must not be replaced"
        );
        cx.update_window(window, |_, _, cx| {
            assert!(
                view.read(cx)
                    .notice
                    .text()
                    .contains("Choose an active project for this source"),
                "{}",
                view.read(cx).notice.text()
            )
        })
        .unwrap();
    } else {
        let saved = saved.expect("actual useful Source observation");
        assert_eq!(
            saved.external_version.as_deref(),
            Some(brn_intake::hex(&brn_intake::digest(bytes)).as_str())
        );
        let actual = saved
            .chat
            .as_deref()
            .and_then(|chat| project_for_chat(&records, chat))
            .map(|p| p.id.as_str());
        let expected = from_project.then_some(projects[0].as_str());
        assert_eq!(
            actual, expected,
            "navigation must not select the useful-intake destination"
        );
        assert_eq!(
            saved
                .report
                .as_ref()
                .unwrap()
                .conversion
                .as_ref()
                .unwrap()
                .settings,
            DocumentConversionSettings::default()
        );
    }
    cx.update_window(window, |_, _, cx| assert!(view.read(cx).worker.is_none()))
        .unwrap();
}
#[gpui_kit::test]
fn documents_useful_scope_project_a_to_b(cx: &mut TestAppContext) {
    useful_navigation(cx, true, true, false);
}
#[gpui_kit::test]
fn documents_useful_scope_project_to_general(cx: &mut TestAppContext) {
    useful_navigation(cx, true, false, false);
}
#[gpui_kit::test]
fn documents_useful_scope_general_to_project(cx: &mut TestAppContext) {
    useful_navigation(cx, false, true, false);
}
#[gpui_kit::test]
fn documents_useful_scope_deleted_project_refuses(cx: &mut TestAppContext) {
    useful_navigation(cx, true, false, true);
}
