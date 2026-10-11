//! Real component/keyboard regression; OS IME and rendered pixels remain native acceptance.
use super::*;
use gpui_kit::{Focusable, TestAppContext, test::TestWindowExt};

pub(super) fn configure_offline_fixture(workspace: &mut Workspace) {
    // Persist this before UI creation: navigation reloads the saved settings.
    // Every component fixture refuses provider startup without credential access.
    let mut settings = workspace.settings().unwrap();
    settings.provider = "offline-fixture".into();
    settings.auth_file = None;
    settings.credentials_dir = None;
    workspace.configure(settings).unwrap();
}

#[gpui_kit::test]
fn first_use_choices_preserve_manual_work_and_persist_explicit_policy(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for (first_page, first_button, review_first, later_button) in [
        (
            Page::Home,
            "delegation-maintain",
            false,
            "delegation-review-first",
        ),
        (
            Page::Settings,
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
        cx.update_window(window, |_, window, cx| window.click("toggle-edit", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.input("Manual change. ", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.click("save-note", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |view, cx| {
                assert!(!view.workspace.settings().unwrap().delegation_chosen);
                let RecordData::Note(saved) =
                    view.workspace.store.note(&note.id).unwrap().unwrap().data
                else {
                    panic!("note")
                };
                assert!(saved.markdown.contains("Manual change."));
                view.navigate(Page::Settings, window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| window.click("save-settings", cx))
            .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |view, cx| {
                assert!(!view.workspace.settings().unwrap().delegation_chosen);
                assert!(!view.workspace.settings().unwrap().maintenance);
                view.navigate(first_page.clone(), window, cx);
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
                assert!(view.notice.starts_with("Saved."));
                view.navigate(Page::Home, window, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert!(window.try_find("delegation-maintain").is_none());
            view.update(cx, |view, cx| view.navigate(Page::Settings, window, cx));
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
            assert_eq!(
                window.find("maintenance-setting").label(),
                Some("Pause maintenance")
            );
            window.click("maintenance-setting", cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert_eq!(
                window.find("maintenance-setting").label(),
                Some("Resume maintenance")
            );
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
    let a = workspace.new_thread("Draft thread A").unwrap();
    let b = workspace.new_thread("Draft thread B").unwrap();
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
            view.navigate(Page::Thread(a.id.clone()), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            view.composer.update(cx, |editor, cx| editor.set_value("A draft õun", window, cx));
            view.navigate(Page::Thread(b.id.clone()), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            view.composer.update(cx, |editor, cx| editor.set_value("B draft", window, cx));
            view.navigate(Page::Thread(a.id.clone()), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "A draft õun");
            view.open_note(&note.id, None, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            assert_eq!(view.composer_destination_label(), "Start a conversation about: Draft note context");
            view.navigate(Page::Thread(a.id.clone()), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "A draft õun");
            view.navigate(Page::Home, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Home draft");
            view.navigate(Page::Thread(b.id.clone()), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "B draft");
            view.navigate(Page::Thread(a.id.clone()), window, cx);
            view.ask(window, cx);
            assert_eq!(view.page, Page::Thread(a.id.clone()));
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            assert!(view.worker.is_none());
            let messages = view.workspace.messages(&a.id).unwrap();
            assert_eq!(messages.len(), 1);
            assert!(matches!(&messages[0].data, RecordData::Message(message) if message.text == "A draft õun" && message.thread == a.id));
            assert!(view.workspace.messages(&b.id).unwrap().is_empty());
            view.navigate(Page::Home, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Home draft");
            view.ask(window, cx);
            let Page::Thread(home_thread) = &view.page else { panic!("Home sends to a new thread") };
            assert_ne!(home_thread, &a.id);
            assert_ne!(home_thread, &b.id);
            let messages = view.workspace.messages(home_thread).unwrap();
            assert!(matches!(&messages[0].data, RecordData::Message(message) if message.text == "Home draft"));
            view.navigate(Page::Home, window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "");
            view.navigate(Page::Thread(b.id.clone()), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "B draft");
            // Missing target causes the real checked posting service to fail.
            view.navigate(Page::Thread("missing-thread".into()), window, cx);
            view.composer.update(cx, |editor, cx| editor.set_value("Preserve failed send", window, cx));
            view.ask(window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Preserve failed send");
            assert!(view.notice.contains("missing reference"), "{}", view.notice);
            view.navigate(Page::Home, window, cx);
            view.navigate(Page::Thread("missing-thread".into()), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Preserve failed send");
        });
    }).unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, _| {
        assert_eq!(
            window.find("composer-destination").label(),
            Some("Send to thread: missing-thread")
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
        let cancel = CancellationToken::new();
        let conversion_cancel = Arc::new(AtomicBool::new(false));
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |view, cx| {
                view.worker = Some(Worker {
                    receiver,
                    cancel: cancel.clone(),
                    conversion_cancel: conversion_cancel.clone(),
                    kind,
                    thread: (kind == WorkerKind::Run).then(|| thread.id.clone()),
                    run_id: None,
                });
                view.progress = "Synthetic progress".into();
                view.composer.update(cx, |editor, cx| {
                    editor.set_value("Home waiting draft", window, cx)
                });
                view.navigate(Page::Thread(thread.id.clone()), window, cx);
                assert_eq!(view.composer.read(cx).value().as_ref(), "");
                view.composer.update(cx, |editor, cx| {
                    editor.set_value("Thread waiting draft", window, cx)
                });
                view.ask(window, cx);
                assert!(
                    view.notice
                        .contains(if cancellable { "stop" } else { "Wait for" })
                );
                assert_eq!(
                    view.composer.read(cx).value().as_ref(),
                    "Thread waiting draft"
                );
                view.navigate(Page::Home, window, cx);
                assert_eq!(
                    view.composer.read(cx).value().as_ref(),
                    "Home waiting draft"
                );
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(window, |_, window, cx| {
            assert_eq!(window.try_find("stop-work").is_some(), cancellable);
            assert_eq!(window.find("worker-status").label(), Some(expected_status));
            assert_eq!(
                window.find("composer-destination").label(),
                Some("Start a new thread · Home draft")
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
            assert!(window.find("section-0").visible());
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
                    view.review.is_some(),
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
            let review = view.review.as_ref().unwrap();
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
                    view.review_editor.read(cx).value().as_ref(),
                    proposed_note.markdown
                );
                assert_eq!(
                    view.current_view.read(cx).rendered_text().source(),
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
            assert!(view.review.is_none());
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
                    view.navigate(Page::History, window, cx);
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
        assert!(view.page == Page::Note(derived_id.clone()));
        assert_eq!(view.note.as_ref().unwrap().record.id, derived_id);
        assert_eq!(
            view.note_view.read(cx).rendered_text().source(),
            "Derived wording stays preserved"
        );
    });
    let mut followup = None;
    for _ in 0..2 {
        cx.update_window(window, |_, window, cx| {
            view.update(cx, |view, cx| view.navigate(Page::History, window, cx))
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
            let Page::Thread(id) = &view.page else {
                panic!("follow-up must open its discussion")
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
        view.update(cx, |view, cx| view.navigate(Page::History, window, cx))
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
        let Page::Thread(id) = &view.page else {
            panic!("Undo refresh callback opens a discussion")
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
            view.toggle_note_edit(window, cx);
            view.note_editor.update(cx, |editor, cx| {
                editor.set_value("Owner typed from old base", window, cx)
            });
            view.save(window, cx);
            assert_eq!(
                view.note_editor.read(cx).value().as_ref(),
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
            view.read(cx).current_view.read(cx).rendered_text().source(),
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
    let a = workspace.new_thread("Working thread A").unwrap();
    let b = workspace.new_thread("Unrelated thread B").unwrap();
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
            view.worker = Some(Worker {
                receiver,
                cancel: cancel.clone(),
                conversion_cancel: Arc::new(AtomicBool::new(false)),
                kind: WorkerKind::Run,
                thread: Some(a.id.clone()),
                run_id: None,
            });
            view.navigate(Page::Thread(b.id.clone()), window, cx);
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
            view.resolve_discussion(&b.id, cx);
            assert!(!cancel.is_cancelled(), "resolving B must not cancel A");
            assert_eq!(
                view.worker.as_ref().unwrap().thread.as_deref(),
                Some(a.id.as_str())
            );
            let RecordData::Thread(a_state) =
                view.workspace.store.record(&a.id).unwrap().unwrap().data
            else {
                panic!("expected thread")
            };
            let RecordData::Thread(b_state) =
                view.workspace.store.record(&b.id).unwrap().unwrap().data
            else {
                panic!("expected thread")
            };
            assert_eq!(a_state.state, ThreadState::Open);
            assert_eq!(b_state.state, ThreadState::Resolved);
            view.navigate(Page::Thread(a.id.clone()), window, cx);
            assert_eq!(view.stream_for(&a.id), Some("Authoritative A"));
            view.resolve_discussion(&a.id, cx);
            assert!(
                cancel.is_cancelled(),
                "resolving the worker's A must cancel A"
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn hidden_note_editor_cannot_gain_edits_from_tab_after_save_read_or_navigation(
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
    cx.update_window(window, |_, window, cx| {
        window.click("toggle-edit", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.input("Typed. ", cx);
    })
    .unwrap();
    cx.run_until_parked();
    let typed = cx.read(|cx| view.read(cx).note_editor.read(cx).value().to_string());
    assert!(typed.contains("Typed. "));
    cx.update_window(window, |_, window, cx| {
        window.click("save-note", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(
            !view
                .read(cx)
                .note_editor
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        window.press("tab", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = view.read(cx);
        assert_eq!(view.note_editor.read(cx).value().as_ref(), typed);
        assert!(view.workspace.store.recovery_buffers().unwrap().is_empty());
        let saved = view.workspace.store.note(&note.id).unwrap().unwrap();
        let RecordData::Note(saved) = saved.data else {
            panic!("expected note")
        };
        assert_eq!(saved.markdown, typed);
    });

    // Reading an unsaved buffer must retain it, while Tab cannot extend that buffer.
    cx.update_window(window, |_, window, cx| window.click("toggle-edit", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.input("Recoverable. ", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.click("toggle-edit", cx))
        .unwrap();
    cx.run_until_parked();
    let recovery =
        cx.read(|cx| view.read(cx).workspace.store.recovery_buffers().unwrap()[0].clone());
    cx.update_window(window, |_, window, cx| window.press("tab", cx))
        .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let buffer = &view.read(cx).workspace.store.recovery_buffers().unwrap()[0];
        assert_eq!(buffer.markdown, recovery.markdown);
        assert_eq!(buffer.generation, recovery.generation);
    });

    // Navigate while the editor is focused, then press Tab in the other page.
    cx.update_window(window, |_, window, cx| window.click("toggle-edit", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.click("home", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(
            !view
                .read(cx)
                .note_editor
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
        );
        window.press("tab", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = view.read(cx);
        assert_eq!(
            view.note_editor.read(cx).value().as_ref(),
            recovery.markdown
        );
        let buffer = &view.workspace.store.recovery_buffers().unwrap()[0];
        assert_eq!(buffer.markdown, recovery.markdown);
        assert_eq!(buffer.generation, recovery.generation);
    });
}
