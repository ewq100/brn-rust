//! Real component/keyboard regression; OS IME and rendered pixels remain native acceptance.
use super::*;
use gpui_kit::{Focusable, TestAppContext, test::TestWindowExt};

// Match the editor toolkit’s native bindings while exercising the same action.
pub(super) const SELECT_ALL: &str = if cfg!(target_os = "macos") {
    "cmd-a"
} else {
    "ctrl-a"
};
pub(super) const LINE_START: &str = if cfg!(target_os = "macos") {
    "cmd-left"
} else {
    "home"
};

pub(super) fn configure_offline_fixture(workspace: &mut Workspace) {
    // Persist this before UI creation: navigation reloads the saved settings.
    // Every component fixture refuses provider startup without credential access.
    let mut settings = workspace.settings().unwrap();
    settings.provider = "offline-fixture".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    workspace.configure(settings).unwrap();
}

/// Show one tile alone, as the Solo layout does.
pub(super) fn solo(view: &mut ThreadsUi, kind: TileKind, cx: &mut Context<ThreadsUi>) {
    view.layout.open(kind);
    if kind != TileKind::Settings && view.layout.maximized() != Some(kind) {
        view.layout.toggle_maximize(kind);
    }
    view.refresh();
    cx.notify();
}
pub(super) fn settings(
    view: &mut ThreadsUi,
    section: SettingsSection,
    cx: &mut Context<ThreadsUi>,
) {
    view.settings_section = section;
    solo(view, TileKind::Settings, cx);
}

#[gpui_kit::test]
fn stop_after_conversion_is_queued_prevents_the_import_commit(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("queued-import-stop");
    let mut workspace = Workspace::open(&data).unwrap();
    configure_offline_fixture(&mut workspace);
    let before: Vec<_> = workspace
        .records()
        .unwrap()
        .into_iter()
        .filter(|record| {
            matches!(
                record.data,
                RecordData::Note(_) | RecordData::Source(_) | RecordData::Asset(_)
            )
        })
        .collect();
    let settings_before = serde_json::to_value(workspace.settings().unwrap()).unwrap();
    cx.update(gpui_kit::init);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut view = ThreadsUi::new(workspace, data, window, cx);
                view.poll = None;
                view
            })
        })
        .unwrap()
    });
    let (sender, receiver) = mpsc::sync_channel(4);
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |view, cx| {
            view.worker = Some(Worker::import(
                receiver,
                CancellationToken::new(),
                Arc::new(AtomicBool::new(false)),
                None,
            ));
            sender
                .send(WorkerEvent::Converted {
                    result: Box::new(Ok(ConversionResult {
                        intent: ImportIntent::FullNote,
                        markdown: "Synthetic converted body".into(),
                        assets: vec![],
                        source_sha256: [1; 32],
                        source_reference: "synthetic/queued.pdf".into(),
                        converter: brn_threads_intake::document_conversion_label("xberg", false)
                            .unwrap(),
                        sources: vec![],
                        coverage: brn_threads_intake::Coverage {
                            status: brn_threads_intake::CoverageStatus::Partial,
                            checked_items: vec![],
                            gaps: vec![],
                        },
                    })),
                    title: "Stopped import".into(),
                    chat: None,
                    settings: DocumentConversionSettings::default(),
                    useful_project: None,
                })
                .unwrap();
            view.cancel(cx);
            view.poll_worker(window, cx);
            assert!(view.worker.is_none());
            assert!(view.notice.text().contains("stopped before saving"));
            let after: Vec<_> = view
                .workspace
                .records()
                .unwrap()
                .into_iter()
                .filter(|record| {
                    matches!(
                        record.data,
                        RecordData::Note(_) | RecordData::Source(_) | RecordData::Asset(_)
                    )
                })
                .collect();
            assert_eq!(after, before);
            assert_eq!(
                serde_json::to_value(view.workspace.settings().unwrap()).unwrap(),
                settings_before
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn first_use_choices_preserve_manual_work_and_persist_explicit_policy(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for (first_page, first_button, review_first, later_button) in [
        (
            None,
            "delegation-maintain",
            false,
            "delegation-review-first",
        ),
        (
            Some(SettingsSection::Delegation),
            "delegation-review-first",
            true,
            "delegation-maintain",
        ),
    ] {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("first-use-delegation");
        let mut workspace = Workspace::open(&data).unwrap();
        configure_offline_fixture(&mut workspace);
        let note = workspace
            .new_note("Manual pending note", "Owner baseline")
            .unwrap();
        let reopened_data = data.clone();
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(1260.), px(1100.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    cx.new(|cx| {
                        let mut view = ThreadsUi::new(workspace, data, window, cx);
                        view.poll = None;
                        view
                    })
                },
            )
            .unwrap()
        });
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert!(window.find("delegation-pending").visible());
            assert!(
                window
                    .find("delegation-status")
                    .label()
                    .unwrap()
                    .contains("Reading and editing remain available")
            );
            assert!(window.try_find("maintenance-setting").is_none());
            view.update(cx, |view, cx| view.open_note(&note.id, None, window, cx));
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.input("Manual change. ", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.click("save-note", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, _, cx| {
            view.update(cx, |view, cx| {
                assert!(!view.workspace.settings().unwrap().delegation_chosen);
                let RecordData::Note(saved) =
                    view.workspace.store.note(&note.id).unwrap().unwrap().data
                else {
                    panic!("note")
                };
                assert!(saved.markdown.contains("Manual change."));
                settings(view, SettingsSection::Models, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.click("save-settings", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, _, cx| {
            view.update(cx, |view, cx| {
                assert!(!view.workspace.settings().unwrap().delegation_chosen);
                assert!(!view.workspace.settings().unwrap().maintenance);
                match first_page {
                    Some(section) => settings(view, section, cx),
                    None => solo(view, TileKind::Overview, cx),
                }
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.click(first_button, cx))
            .unwrap();
        cx.run_until_parked();
        let selected = Workspace::open(&reopened_data).unwrap().settings().unwrap();
        assert!(selected.delegation_chosen);
        assert!(selected.maintenance);
        assert_eq!(selected.review_first, review_first);
        assert_eq!(selected.provider, "offline-fixture");
        assert!(selected.auth_file.is_none() && selected.credentials_dir.is_none());
        cx.update_window(window, |_, window, cx| {
            assert!(window.try_find("delegation-pending").is_none());
            view.update(cx, |view, cx| {
                assert!(view.worker.is_none());
                assert!(view.notice.text().starts_with("Saved."));
                solo(view, TileKind::Overview, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert!(window.try_find("delegation-maintain").is_none());
            view.update(cx, |view, cx| {
                settings(view, SettingsSection::Delegation, cx)
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert_eq!(
                window.find("delegation-status").label(),
                Some(if review_first {
                    "Review changes first is selected. Proposed changes wait for your approval."
                } else {
                    "BRN may maintain ordinary notes within your delegated scope."
                })
            );
            // The switch shows Running; switching it off pauses maintenance.
            let switch = window.find("maintenance-setting");
            assert_eq!(switch.label(), Some("Maintenance: Running"));
            assert_eq!(switch.checked(), Some(true));
            window.click("maintenance-setting", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            let switch = window.find("maintenance-setting");
            assert_eq!(switch.label(), Some("Maintenance: Paused"));
            assert_eq!(switch.checked(), Some(false));
            assert!(
                window
                    .find("delegation-status")
                    .label()
                    .unwrap()
                    .contains("paused")
            );
            let paused = Workspace::open(&reopened_data).unwrap().settings().unwrap();
            assert!(paused.delegation_chosen);
            assert!(!paused.maintenance);
            window.click("maintenance-setting", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.click(later_button, cx))
            .unwrap();
        cx.run_until_parked();
        let changed = Workspace::open(&reopened_data).unwrap().settings().unwrap();
        assert!(changed.delegation_chosen && changed.maintenance);
        assert_eq!(changed.review_first, !review_first);
        cx.update_window(window, |_, window, cx| {
            assert_eq!(
                window.find("delegation-status").label(),
                Some(if !review_first {
                    "Review changes first is selected. Proposed changes wait for your approval."
                } else {
                    "BRN may maintain ordinary notes within your delegated scope."
                })
            );
            assert!(view.read(cx).worker.is_none());
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn composer_drafts_follow_home_and_each_thread(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("composer-drafts-regression");
    let mut workspace = Workspace::open(&data).unwrap();
    configure_offline_fixture(&mut workspace);
    let a = workspace.new_thread("Draft chat A").unwrap();
    let b = workspace.new_thread("Draft chat B").unwrap();
    let note = workspace
        .new_note("Draft note context", "Synthetic note")
        .unwrap();
    cx.update(gpui_kit::init);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut view = ThreadsUi::new(workspace, data, window, cx);
                view.poll = None;
                view
            })
        })
        .unwrap()
    });
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |view, cx| {
            view.composer.update(cx, |editor, cx| editor.set_value("Home draft", window, cx));
            view.open_thread(&a.id, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            view.composer.update(cx, |editor, cx| editor.set_value("A draft õun", window, cx));
            view.open_thread(&b.id, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            view.composer.update(cx, |editor, cx| editor.set_value("B draft", window, cx));
            view.open_thread(&a.id, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "A draft õun");
            view.open_note(&note.id, None, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "A draft õun");
            assert_eq!(view.chat, ChatTarget::Thread(a.id.clone()));
            view.open_thread(&a.id, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "A draft õun");
            view.new_thread(window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Home draft");
            view.open_thread(&b.id, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "B draft");
            view.open_thread(&a.id, window, cx);
            view.ask(window, cx);
            assert_eq!(view.chat, ChatTarget::Thread(a.id.clone()));
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            assert!(view.worker.is_none());
            let messages = view.workspace.messages(&a.id).unwrap();
            assert_eq!(messages.len(), 1);
            assert!(matches!(&messages[0].data, RecordData::Message(message) if message.text == "A draft õun" && message.thread == a.id));
            assert!(view.workspace.messages(&b.id).unwrap().is_empty());
            view.new_thread(window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Home draft");
            view.ask(window, cx);
            let ChatTarget::Thread(home_thread) = view.chat.clone() else { panic!("a new chat is created") };
            let home_thread = &home_thread;
            assert_ne!(home_thread, &a.id);
            assert_ne!(home_thread, &b.id);
            let messages = view.workspace.messages(home_thread).unwrap();
            assert!(matches!(&messages[0].data, RecordData::Message(message) if message.text == "Home draft"));
            view.new_thread(window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            view.open_thread(&b.id, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "B draft");
            // Missing target causes the real checked posting service to fail.
            view.open_thread("missing-thread", window, cx);
            view.composer.update(cx, |editor, cx| editor.set_value("Preserve failed send", window, cx));
            view.ask(window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Preserve failed send");
            assert!(view.notice.text().contains("missing reference"), "{}", view.notice.text());
            view.new_thread(window, cx);
            view.open_thread("missing-thread", window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Preserve failed send");
        });
    }).unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, _| {
        assert_eq!(
            window.find("tile-title-chat").label(),
            Some("Chat: missing-thread")
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn worker_controls_match_real_cancellation_capabilities(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("worker-controls-regression");
    let mut workspace = Workspace::open(&data).unwrap();
    configure_offline_fixture(&mut workspace);
    let thread = workspace.new_thread("Worker destination").unwrap();
    cx.update(gpui_kit::init);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut view = ThreadsUi::new(workspace, data, window, cx);
                view.poll = None;
                view
            })
        })
        .unwrap()
    });
    for (kind, cancellable, expected_status) in [
        (
            WorkerKind::Run,
            true,
            "Work in Worker destination · Synthetic progress",
        ),
        (WorkerKind::Import, true, "Synthetic progress"),
        (
            WorkerKind::Export,
            false,
            "Finishing Markdown export… Please wait for completion.",
        ),
        (
            WorkerKind::Backup,
            false,
            "Finishing full backup… Please wait for completion.",
        ),
    ] {
        let (_sender, receiver) = mpsc::sync_channel(4);
        let (cancel, conversion_cancel, _lease) = cx
            .update_window(window, |_, window, cx| {
                view.update(cx, |view, cx| {
                    let mut lease = None;
                    let worker = match kind {
                        WorkerKind::Run => {
                            let run = chat_tests::record_run(
                                &mut view.workspace,
                                &thread.id,
                                RunState::Working,
                                "Synthetic progress",
                            );
                            lease = Some(
                                brn_ai::run_lease::RunLease::acquire(
                                    &view.workspace.store.derived_directory().unwrap(),
                                    &run,
                                )
                                .unwrap(),
                            );
                            Worker::run(
                                receiver,
                                CancellationToken::new(),
                                thread.id.clone(),
                                run,
                                None,
                            )
                        }
                        WorkerKind::Import => Worker::import(
                            receiver,
                            CancellationToken::new(),
                            Arc::new(AtomicBool::new(false)),
                            None,
                        ),
                        WorkerKind::Export | WorkerKind::Backup => {
                            Worker::output(receiver, kind == WorkerKind::Backup)
                        }
                        WorkerKind::Connect => Worker::connect(receiver, CancellationToken::new()),
                    };
                    let cancel = worker.cancel.clone();
                    let conversion_cancel = worker.conversion_cancel.clone();
                    view.worker = Some(worker);
                    view.progress = "Synthetic progress".into();
                    view.composer.update(cx, |editor, cx| {
                        editor.set_value("Home waiting draft", window, cx)
                    });
                    view.open_thread(&thread.id, window, cx);
                    assert_eq!(view.composer.read(cx).value().as_ref(), "");
                    view.composer.update(cx, |editor, cx| {
                        editor.set_value("Chat waiting draft", window, cx)
                    });
                    view.ask(window, cx);
                    // The queue notice says when it runs ("Queued; runs after …").
                    assert!(
                        view.notice.text().starts_with("Queued"),
                        "{}",
                        view.notice.text()
                    );
                    assert_eq!(view.composer.read(cx).value().as_ref(), "");
                    assert!(
                        view.queue_state()
                            .queue
                            .iter()
                            .any(|q| q.text == "Chat waiting draft")
                    );
                    view.new_thread(window, cx);
                    assert_eq!(
                        view.composer.read(cx).value().as_ref(),
                        "Home waiting draft"
                    );
                    (cancel, conversion_cancel, lease)
                })
            })
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert_eq!(window.try_find("stop-work").is_some(), cancellable);
            assert_eq!(window.find("worker-status").label(), Some(expected_status));
            assert_eq!(
                window.find("tile-title-chat").label(),
                Some("Chat: New chat")
            );
            if cancellable {
                window.click("stop-work", cx);
            } else {
                // The keyboard action uses the same capability guard.
                view.update(cx, |view, cx| view.cancel(cx));
            }
            assert_eq!(cancel.is_cancelled(), cancellable);
            assert_eq!(
                conversion_cancel.load(std::sync::atomic::Ordering::SeqCst),
                cancellable
            );
            view.update(cx, |view, _| {
                view.worker = None;
                // Each next worker begins with an untouched thread draft.
                view.composer_drafts
                    .posted(&ComposerDestination::Thread(thread.id.clone()));
            });
        })
        .unwrap();
    }
}

#[gpui_kit::test]
fn long_coverage_details_scroll_without_hiding_note_body_or_losing_issues(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for legacy in [false, true] {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("coverage-layout-regression");
        let mut workspace = Workspace::open(&data).unwrap();
        configure_offline_fixture(&mut workspace);
        let note = workspace
            .new_note(
                "Coverage layout witness",
                "# Purpose\n\nReadable note body stays available.",
            )
            .unwrap();
        let operation = workspace
            .store
            .allocate_operation("long coverage witness")
            .unwrap();
        let source_id = operation.creation_id(0);
        let report = ImportReport {
            conversion: None,
            status: ImportStatus::Partial,
            issues: (0..6)
                .map(|index| ImportIssue {
                    kind: ImportIssueKind::Omission,
                    locator: format!(
                        "/synthetic/long/source/reference/{index}/document.pdf · chart-{index}"
                    ),
                    detail:
                        "Retained prior input: Known missing chart qualifications remain visible. "
                            .repeat(70),
                })
                .collect(),
        };
        let mut imported = match &note.data {
            RecordData::Note(note) => note.clone(),
            _ => panic!("Note"),
        };
        imported.import = Some(ImportCoverage {
            source: source_id.clone(),
            full_note: false,
            gaps: report.issues.iter().map(ImportIssue::display).collect(),
            report: (!legacy).then_some(report.clone()),
        });
        workspace
            .owner_change(
                "long coverage witness",
                &ChangeRequest {
                    reason: "Synthetic long source coverage".into(),
                    writes: vec![
                        Put {
                            id: source_id,
                            expected_version: None,
                            archived: false,
                            data: RecordData::Source(Source {
                                chat: None,
                                locator: "synthetic://coverage-layout".into(),
                                label: "Coverage source".into(),
                                observed_at: "synthetic".into(),
                                external_version: None,
                                outcome: "Partial".into(),
                                gaps: report.issues.iter().map(ImportIssue::display).collect(),
                                report: (!legacy).then_some(report),
                            }),
                        },
                        Put {
                            id: note.id.clone(),
                            expected_version: Some(note.version),
                            archived: false,
                            data: RecordData::Note(imported),
                        },
                    ],
                    inputs: vec![],
                },
            )
            .unwrap();
        let expected = workspace.store.record(&note.id).unwrap().unwrap();
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(1260.), px(860.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    cx.new(|cx| {
                        let mut view = ThreadsUi::new(workspace, data, window, cx);
                        view.poll = None;
                        view.open_note(&note.id, None, window, cx);
                        view
                    })
                },
            )
            .unwrap()
        });
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert!(
                window.try_find("note-coverage-details").is_none(),
                "details are on demand"
            );
            window.click("toggle-coverage", cx);
        })
        .unwrap();
        cx.run_until_parked();
        let mut last_origin = None;
        cx.update_window(window, |_, window, cx| {
            let details = window.find("note-coverage-details");
            assert!(details.visible());
            assert!(details.bounds().size.height <= px(120.));
            let body = window.find("note-body");
            assert!(body.visible());
            assert!(
                body.bounds().size.height > px(150.),
                "coverage must leave a usable note body"
            );
            assert!(window.find("note-import-summary").visible());
            // Headings are reachable from the toolbar's Outline, not a row.
            assert!(window.find("note-outline").visible());
            assert!(window.try_find("section-0").is_none());
            last_origin = Some(window.find("note-coverage-detail-5").bounds().origin.y);
            window.scroll(
                "note-coverage-details",
                gpui_kit::ScrollDelta::Pixels(point(px(0.), px(-100_000.))),
                cx,
            );
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, _| {
            let last = window.find("note-coverage-detail-5");
            assert!(
                last.visible(),
                "the final issue remains reachable by scrolling"
            );
            assert!(last.bounds().origin.y < last_origin.unwrap());
            assert!(window.find("note-body").visible());
            assert!(window.find("note-import-summary").visible());
        })
        .unwrap();
        cx.read(|cx| {
            assert_eq!(
                view.read(cx)
                    .workspace
                    .store
                    .record(&note.id)
                    .unwrap()
                    .unwrap(),
                expected,
                "presentation must preserve all stored issues"
            );
        });
        // Outline lists the note's heading; choosing it selects that line.
        cx.update_window(window, |_, window, cx| window.click("note-outline", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            let mut menu = window.within("popup-menu");
            let index = (0..20usize)
                .find(|index| {
                    menu.try_find(*index)
                        .is_some_and(|row| row.label() == Some("Purpose"))
                })
                .expect("Outline lists the Purpose heading");
            menu.click(index, cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.read(|cx| {
            let editor = view.read(cx).writing.note_editor.read(cx);
            let range = editor.selected_range();
            assert_eq!(editor.value().get(range), Some("Purpose"));
        });
    }
}

#[gpui_kit::test]
fn native_whole_group_selection_prepares_new_comparison_before_exact_apply(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    for select_note in [true, false] {
        let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
        let data = root.path().join("group-selection-regression");
        let mut workspace = Workspace::open(&data).unwrap();
        configure_offline_fixture(&mut workspace);
        let note = workspace
            .new_note("Protected group", "Original complete wording")
            .unwrap();
        let RecordData::Note(mut protected) = note.data.clone() else {
            panic!("expected note")
        };
        protected.protected = true;
        workspace
            .owner_change(
                "protect group witness",
                &ChangeRequest {
                    reason: "Protect witness".into(),
                    writes: vec![Put {
                        id: note.id.clone(),
                        expected_version: Some(note.version),
                        archived: false,
                        data: RecordData::Note(protected),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        let note = workspace.store.record(&note.id).unwrap().unwrap();
        let action_operation = workspace
            .store
            .allocate_operation("Action group witness")
            .unwrap();
        let action_id = action_operation.creation_id(0);
        workspace
            .owner_change(
                "Action group witness",
                &ChangeRequest {
                    reason: "Create human Action".into(),
                    writes: vec![Put {
                        id: action_id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Action(Action {
                            kept_outside_project: false,
                            chat: None,
                            description: "Human wording check".into(),
                            state: ActionState::Waiting,
                            internal: false,
                            evidence: None,
                            due: None,
                        }),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        let action = workspace.store.record(&action_id).unwrap().unwrap();
        let operation = workspace
            .store
            .allocate_operation("Independent grouped candidate")
            .unwrap();
        let RecordData::Note(mut proposed_note) = note.data.clone() else {
            panic!("expected note")
        };
        proposed_note.markdown = "Selected complete proposed wording".into();
        let RecordData::Action(mut proposed_action) = action.data.clone() else {
            panic!("expected Action")
        };
        proposed_action.state = ActionState::Done;
        proposed_action.evidence = Some("Owner's explicit human completion witness".into());
        let request = ChangeRequest {
            reason: "Independent Note and Action review".into(),
            writes: vec![
                Put {
                    id: note.id.clone(),
                    expected_version: Some(note.version),
                    archived: false,
                    data: RecordData::Note(proposed_note.clone()),
                },
                Put {
                    id: action.id.clone(),
                    expected_version: Some(action.version),
                    archived: false,
                    data: RecordData::Action(proposed_action.clone()),
                },
            ],
            inputs: vec![],
        };
        let original = workspace.store.prepare(&operation, &request).unwrap();
        let omitted = if select_note { &action.id } else { &note.id };
        let selected = if select_note { &note.id } else { &action.id };
        let (window, view) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        point(px(0.), px(0.)),
                        size(px(1500.), px(1100.)),
                    ))),
                    ..Default::default()
                },
                cx,
                |window, cx| {
                    cx.new(|cx| {
                        let mut view = ThreadsUi::new(workspace, data, window, cx);
                        view.poll = None;
                        view.open_review(original.clone(), window, cx);
                        view
                    })
                },
            )
            .unwrap()
        });
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.click(format!("review-group-{omitted}"), cx)
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.apply_review(window, cx);
                assert!(
                    view.writing.review.is_some(),
                    "a changed selection is not yet approvable"
                );
                assert_eq!(
                    view.workspace.store.record(&note.id).unwrap().unwrap(),
                    note
                );
                assert_eq!(
                    view.workspace.store.record(&action.id).unwrap().unwrap(),
                    action
                );
            });
            window.click("prepare-selected", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.read(|cx| {
            let view = view.read(cx);
            let review = view.writing.review.as_ref().unwrap();
            assert_ne!(review.candidate.operation, original.operation);
            assert_ne!(review.candidate.request_hash, original.request_hash);
            assert_eq!(review.candidate.request.writes.len(), 1);
            assert_eq!(&review.candidate.request.writes[0].id, selected);
            assert_eq!(
                view.workspace
                    .store
                    .prepared(&original.operation)
                    .unwrap()
                    .request,
                request
            );
            assert_eq!(
                view.workspace.store.record(&note.id).unwrap().unwrap(),
                note
            );
            assert_eq!(
                view.workspace.store.record(&action.id).unwrap().unwrap(),
                action
            );
            if select_note {
                assert_eq!(
                    view.writing.review_editor.read(cx).value().as_ref(),
                    proposed_note.markdown
                );
                assert_eq!(
                    view.writing.current_view.read(cx).rendered_text().source(),
                    "Original complete wording"
                );
            } else {
                assert!(review.note.is_none());
            }
        });
        cx.update_window(window, |_, window, cx| window.click("apply-final", cx))
            .unwrap();
        cx.run_until_parked();
        cx.read(|cx| {
            let view = view.read(cx);
            assert!(view.writing.review.is_none());
            let saved_note = view.workspace.store.record(&note.id).unwrap().unwrap();
            let saved_action = view.workspace.store.record(&action.id).unwrap().unwrap();
            if select_note {
                assert_eq!(saved_note.data, RecordData::Note(proposed_note));
                assert_eq!(saved_action, action);
            } else {
                assert_eq!(saved_note, note);
                assert_eq!(saved_action.data, RecordData::Action(proposed_action));
            }
        });
    }
}

#[gpui_kit::test]
fn native_refresh_controls_open_named_note_and_one_followup_without_changing_derived_work(
    cx: &mut TestAppContext,
) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("refresh-controls-regression");
    let mut workspace = Workspace::open(&data).unwrap();
    configure_offline_fixture(&mut workspace);
    workspace
        .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
        .unwrap();
    let input = workspace
        .new_note("Changed input", "Original input")
        .unwrap();
    let operation = workspace
        .store
        .allocate_operation("Derived note witness")
        .unwrap();
    let derived_id = operation.creation_id(0);
    workspace
        .store
        .prepare(
            &operation,
            &ChangeRequest {
                reason: "Derived output".into(),
                writes: vec![Put {
                    id: derived_id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working(
                        "Named derived note",
                        "Derived wording stays preserved",
                    )),
                }],
                inputs: vec![RevisionInput {
                    record: input.id.clone(),
                    version: input.version,
                    expectation: InputExpectation::Current,
                }],
            },
        )
        .unwrap();
    let outcome = workspace
        .store
        .apply(
            &operation,
            &HostAuthority::maintenance("synthetic derivation"),
        )
        .unwrap();
    assert!(matches!(outcome, ApplyOutcome::Applied(_)));
    let session = workspace
        .store
        .begin_edit(&input.id, input.version)
        .unwrap();
    workspace
        .store
        .update_buffer(&session.id, 1, "New saved input")
        .unwrap();
    let SaveOutcome::Saved(receipt) = workspace.store.save(&session.id, 1).unwrap() else {
        panic!("expected Save")
    };
    assert_eq!(receipt.needs_refresh, vec![derived_id.clone()]);
    let derived = workspace.store.record(&derived_id).unwrap().unwrap();
    cx.update(gpui_kit::init);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1500.), px(1100.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut view = ThreadsUi::new(workspace, data, window, cx);
                    view.poll = None;
                    view.open_note(&input.id, None, window, cx);
                    view.show_tile(TileKind::History, window, cx);
                    view
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.click(
            format!("refresh-record-{}-{derived_id}", receipt.operation.as_str()),
            cx,
        )
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = view.read(cx);
        assert_eq!(view.open_note_id(), Some(derived_id.as_str()));
        assert_eq!(view.writing.note.as_ref().unwrap().record().id, derived_id);
        assert_eq!(
            view.writing.note_editor.read(cx).value().as_ref(),
            "Derived wording stays preserved"
        );
    });
    let mut followup = None;
    for _ in 0..2 {
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |view, cx| view.show_tile(TileKind::History, window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            window.click(
                format!("refresh-followup-{}", receipt.operation.as_str()),
                cx,
            )
        })
        .unwrap();
        cx.run_until_parked();
        cx.read(|cx| {
            let view = view.read(cx);
            let ChatTarget::Thread(id) = &view.chat else {
                panic!("follow-up must open its chat")
            };
            if let Some(original) = &followup {
                assert_eq!(id, original);
            } else {
                followup = Some(id.clone());
            }
            assert!(
                view.worker.is_none(),
                "follow-up creation requires no provider run"
            );
            assert_eq!(
                view.workspace.store.record(&derived_id).unwrap().unwrap(),
                derived
            );
            assert_eq!(view.workspace.messages(id).unwrap().len(), 1);
        });
    }
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |view, cx| view.show_tile(TileKind::History, window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.click(format!("undo-{}", receipt.operation.as_str()), cx)
    })
    .unwrap();
    cx.run_until_parked();
    let undo = cx.read(|cx| {
        let view = view.read(cx);
        let undo = view.history.last().unwrap();
        assert!(undo.reason.starts_with("Undo:"));
        assert_eq!(undo.needs_refresh, vec![derived_id.clone()]);
        assert_eq!(view.workspace.store.record(&derived_id).unwrap().unwrap(), derived);
        assert!(matches!(view.workspace.store.record(&input.id).unwrap().unwrap().data, RecordData::Note(note) if note.markdown == "Original input"));
        undo.operation.clone()
    });
    cx.update_window(window, |_, window, cx| {
        window.click(format!("refresh-followup-{}", undo.as_str()), cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = view.read(cx);
        let ChatTarget::Thread(id) = &view.chat else {
            panic!("Undo refresh callback opens a chat")
        };
        assert_ne!(Some(id), followup.as_ref());
        assert_eq!(
            view.workspace.store.record(&derived_id).unwrap().unwrap(),
            derived
        );
    });
}

#[gpui_kit::test]
fn stale_save_compares_latest_saved_text_and_preserves_owner_buffer(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("stale-comparison-regression");
    let mut workspace = Workspace::open(&data).unwrap();
    configure_offline_fixture(&mut workspace);
    let note = workspace
        .new_note("Stale witness", "Original saved base")
        .unwrap();
    cx.update(gpui_kit::init);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut view = ThreadsUi::new(workspace, data, window, cx);
                view.poll = None;
                view.open_note(&note.id, None, window, cx);
                view
            })
        })
        .unwrap()
    });
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |view, cx| {
            // The visible clean editor remembers v1 while another actor saves v2.
            let RecordData::Note(mut current) = note.data.clone() else {
                panic!("expected note")
            };
            current.markdown = "Latest saved concurrent witness".into();
            let outcome = view
                .workspace
                .owner_change(
                    "comparison-concurrent-edit",
                    &ChangeRequest {
                        reason: "Synthetic concurrent edit".into(),
                        writes: vec![Put {
                            id: note.id.clone(),
                            expected_version: Some(note.version),
                            archived: false,
                            data: RecordData::Note(current),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            assert!(matches!(outcome, ApplyOutcome::Applied(_)));
            view.writing.note_editor.update(cx, |editor, cx| {
                editor.set_value("Owner typed from old base", window, cx)
            });
            view.save(window, cx);
            assert_eq!(
                view.writing.note_editor.read(cx).value().as_ref(),
                "Owner typed from old base"
            );
            assert_eq!(
                view.workspace.store.recovery_buffers().unwrap()[0].markdown,
                "Owner typed from old base"
            );
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(
            view.read(cx)
                .writing
                .current_view
                .read(cx)
                .rendered_text()
                .source(),
            "Latest saved concurrent witness"
        );
    });
}

#[gpui_kit::test]
fn native_stream_and_resolution_keep_the_worker_thread_owner(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("worker-owner-regression");
    let mut workspace = Workspace::open(&data).unwrap();
    configure_offline_fixture(&mut workspace);
    let a = workspace.new_thread("Working chat A").unwrap();
    let b = workspace.new_thread("Unrelated chat B").unwrap();
    let run = chat_tests::record_run(
        &mut workspace,
        &a.id,
        RunState::Working,
        "Synthetic progress",
    );
    let _lease =
        brn_ai::run_lease::RunLease::acquire(&workspace.store.derived_directory().unwrap(), &run)
            .unwrap();
    cx.update(gpui_kit::init);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| {
                let mut view = ThreadsUi::new(workspace, data, window, cx);
                view.poll = None;
                view
            })
        })
        .unwrap()
    });
    let (sender, receiver) = mpsc::sync_channel(4);
    let cancel = CancellationToken::new();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |view, cx| {
            view.worker = Some(Worker::run(receiver, cancel.clone(), a.id.clone(), run.clone(), None));
            view.open_thread(&b.id, window, cx);
            sender
                .send(WorkerEvent::Text("Only A's synthetic answer".into()))
                .unwrap();
            view.poll_worker(window, cx);
            assert_eq!(view.stream_for(&a.id), Some("Only A's synthetic answer"));
            assert_eq!(view.stream_for(&b.id), None);
            sender
                .send(WorkerEvent::AnswerFinalized("Authoritative A".into()))
                .unwrap();
            view.poll_worker(window, cx);
            assert_eq!(view.stream_for(&a.id), Some("Authoritative A"));
            assert_eq!(view.stream_for(&b.id), None);
            view.resolve_thread(&b.id, cx);
            assert!(!cancel.is_cancelled(), "resolving B must not cancel A");
            assert_eq!(
                view.worker.as_ref().unwrap().run_identity().unwrap().thread,
                a.id
            );
            let RecordData::Thread(a_state) =
                view.workspace.store.record(&a.id).unwrap().unwrap().data
            else {
                panic!("expected chat")
            };
            let RecordData::Thread(b_state) =
                view.workspace.store.record(&b.id).unwrap().unwrap().data
            else {
                panic!("expected chat")
            };
            assert_eq!(a_state.state, ThreadState::Open);
            assert_eq!(b_state.state, ThreadState::Open);
            assert!(
                view.workspace
                    .store
                    .record(&b.id)
                    .unwrap()
                    .unwrap()
                    .archived
            );
            view.open_thread(&a.id, window, cx);
            assert_eq!(view.stream_for(&a.id), Some("Authoritative A"));
            view.resolve_thread(&a.id, cx);
            assert!(
                cancel.is_cancelled(),
                "resolving the worker's A must cancel A"
            );
            let RecordData::Run(stopped) = view.workspace.store.record(&run).unwrap().unwrap().data else { panic!("expected Run") };
            assert_eq!(stopped.state, RunState::Interrupted);
            assert!(view.workspace.messages(&a.id).unwrap().iter().any(|record| matches!(&record.data, RecordData::Message(message) if message.role == "assistant-partial" && message.text == "Authoritative A")));
            assert!(view.workspace.messages(&b.id).unwrap().is_empty());
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn hidden_note_editor_cannot_gain_edits_from_tab_after_save_or_layout_change(
    cx: &mut TestAppContext,
) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("focus-regression");
    let mut workspace = Workspace::open(&data).unwrap();
    configure_offline_fixture(&mut workspace);
    let note = workspace
        .new_note("Focus witness", "Harbor baseline. Õun ja öö.")
        .unwrap();
    cx.update(gpui_kit::init);
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1260.), px(860.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut view = ThreadsUi::new(workspace, data, window, cx);
                    // No provider worker is needed; keep this keyboard journey deterministic.
                    view.poll = None;
                    view.open_note(&note.id, None, window, cx);
                    view
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    // The note is typed in place: opening it focuses the visible editor.
    cx.update_window(window, |_, window, cx| {
        assert!(
            view.read(cx)
                .writing
                .note_editor
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        window.input("Typed. ", cx);
    })
    .unwrap();
    cx.run_until_parked();
    let typed = cx.read(|cx| {
        view.read(cx)
            .writing
            .note_editor
            .read(cx)
            .value()
            .to_string()
    });
    assert!(typed.contains("Typed. "));
    cx.update_window(window, |_, window, cx| window.click("save-note", cx))
        .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = view.read(cx);
        assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), typed);
        assert!(view.workspace.store.recovery_buffers().unwrap().is_empty());
        let RecordData::Note(saved) = view.workspace.store.note(&note.id).unwrap().unwrap().data
        else {
            panic!("expected note")
        };
        assert_eq!(saved.markdown, typed);
    });

    // An unsaved edit survives hiding the note tile; Tab elsewhere cannot extend it.
    cx.update_window(window, |_, window, cx| window.input("Recoverable. ", cx))
        .unwrap();
    cx.run_until_parked();
    let recovery =
        cx.read(|cx| view.read(cx).workspace.store.recovery_buffers().unwrap()[0].clone());
    cx.update_window(window, |_, window, cx| window.click("home", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.click("layout-solo", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(window.try_find("note-body").is_none(), "note tile hidden");
        window.press("tab", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = view.read(cx);
        assert_eq!(view.layout.visible().len(), 1);
        assert_eq!(
            view.writing.note_editor.read(cx).value().as_ref(),
            recovery.markdown
        );
        let buffer = &view.workspace.store.recovery_buffers().unwrap()[0];
        assert_eq!(buffer.markdown, recovery.markdown);
        assert_eq!(buffer.generation, recovery.generation);
    });
}
