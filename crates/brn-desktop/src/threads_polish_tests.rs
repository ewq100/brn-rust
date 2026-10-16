//! Offline Note-tile witnesses for the UI polish: toolbar fit, banners, the
//! writing bar's one primary and the Discard dialog. Pixels and real
//! pointer behavior remain native acceptance.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) window: gpui_kit::AnyWindowHandle,
    pub(super) view: Entity<ThreadsUi>,
    pub(super) note: Record,
}

pub(super) fn fixture(cx: &mut TestAppContext, width: f32, height: f32, markdown: &str) -> Fixture {
    fixture_with(cx, width, height, markdown, false)
}

/// The widest toolbar a note shows: "Protected · v12" and 12 open comments.
const WIDEST: &str =
    "# Widest\n\nalpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu";

fn widest(cx: &mut TestAppContext, width: f32, height: f32) -> Fixture {
    fixture_with(cx, width, height, WIDEST, true)
}

fn fixture_with(
    cx: &mut TestAppContext,
    width: f32,
    height: f32,
    markdown: &str,
    widest: bool,
) -> Fixture {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("polish-offline");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let mut note = workspace.new_note("Polish witness", markdown).unwrap();
    if widest {
        note = widen(&mut workspace, note);
    }
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
    let f = Fixture {
        _root: root,
        window,
        view,
        note,
    };
    let id = f.note.id.clone();
    update(cx, &f, |view, window, cx| {
        view.open_note(&id, None, window, cx)
    });
    f
}

pub(super) fn update<R>(
    cx: &mut TestAppContext,
    f: &Fixture,
    run: impl FnOnce(&mut ThreadsUi, &mut Window, &mut Context<ThreadsUi>) -> R,
) -> R {
    let result = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| run(view, window, cx))
        })
        .unwrap();
    cx.run_until_parked();
    result
}

pub(super) fn on_window<R>(
    cx: &mut TestAppContext,
    f: &Fixture,
    run: impl FnOnce(&mut Window, &mut App) -> R,
) -> R {
    let result = cx
        .update_window(f.window, |_, window, cx| run(window, cx))
        .unwrap();
    cx.run_until_parked();
    result
}

/// Ten owner saves, protection (v12) and a comment on each of 12 words.
fn widen(workspace: &mut Workspace, mut note: Record) -> Record {
    for draft in 0..10 {
        let edit = workspace.store.begin_edit(&note.id, note.version).unwrap();
        let text = if draft == 9 {
            WIDEST.to_owned()
        } else {
            format!("Draft {draft}")
        };
        workspace.store.update_buffer(&edit.id, 1, &text).unwrap();
        assert!(matches!(
            workspace.store.save(&edit.id, 1).unwrap(),
            SaveOutcome::Saved(_)
        ));
        note = workspace.store.note(&note.id).unwrap().unwrap();
    }
    let RecordData::Note(mut data) = note.data.clone() else {
        unreachable!()
    };
    data.protected = true;
    workspace
        .owner_change(
            "widest-protect",
            &ChangeRequest {
                reason: "Protect the widest witness".into(),
                writes: vec![Put {
                    id: note.id.clone(),
                    expected_version: Some(note.version),
                    archived: false,
                    data: RecordData::Note(data),
                }],
                inputs: vec![],
            },
        )
        .unwrap();
    let note = workspace.store.note(&note.id).unwrap().unwrap();
    assert_eq!(note.version, 12);
    let thread = workspace.new_thread("Widest chat").unwrap().id;
    workspace
        .link_writing_thread("widest-link", &note.id, note.version, &thread)
        .unwrap();
    let words = WIDEST.split("\n\n").nth(1).unwrap();
    let offset = WIDEST.len() - words.len();
    let mut start = offset;
    for (index, word) in words.split(' ').enumerate() {
        workspace
            .add_writing_comment(
                &format!("widest-comment-{index}"),
                &WritingCommentInput {
                    note: note.id.clone(),
                    version: note.version,
                    thread: thread.clone(),
                    range: (start, start + word.len()),
                    quote: word.into(),
                    body: format!("Comment {index}"),
                },
            )
            .unwrap();
        start += word.len() + 1;
    }
    note
}

const TOOLBAR_IDS: [&str; 13] = [
    "note-write",
    "note-read",
    "format-heading",
    "format-bold",
    "format-italic",
    "format-list",
    "format-quote",
    "format-menu",
    "note-outline",
    "note-status-chip",
    "toggle-margin",
    "edit-note-context",
    "note-connections",
];

/// Type into the open note so it has an unsaved, recoverable edit.
pub(super) fn type_edit(cx: &mut TestAppContext, f: &Fixture, tail: &str) {
    update(cx, f, |view, window, cx| {
        let text = format!("{}{tail}", view.note_editor.read(cx).value());
        view.note_editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        view.preserve_visible_note_input(cx);
    });
    assert!(cx.read(|cx| f.view.read(cx).note.as_ref().unwrap().session.is_some()));
}

/// Change the saved note outside this view, as another window or BRN would.
pub(super) fn change_saved(cx: &mut TestAppContext, f: &Fixture, change: impl FnOnce(&mut Note)) {
    update(cx, f, |view, _, cx| {
        let current = view.workspace.store.note(&f.note.id).unwrap().unwrap();
        let RecordData::Note(mut note) = current.data.clone() else {
            unreachable!()
        };
        change(&mut note);
        assert!(matches!(
            view.workspace
                .owner_change(
                    &format!("external-{}", uuid::Uuid::new_v4()),
                    &ChangeRequest {
                        reason: "Synthetic change elsewhere".into(),
                        writes: vec![Put {
                            id: current.id.clone(),
                            expected_version: Some(current.version),
                            archived: false,
                            data: RecordData::Note(note),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        view.refresh();
        cx.notify();
    });
}

pub(super) fn assert_in(window: &Window, id: &str, outer: Bounds<gpui_kit::Pixels>) {
    let element = window.find(id.to_owned());
    assert!(element.visible(), "{id} is visible");
    let bounds = element.bounds();
    assert!(
        bounds.left() >= outer.left()
            && bounds.right() <= outer.right()
            && bounds.top() >= outer.top()
            && bounds.bottom() <= outer.bottom(),
        "{id} {bounds:?} inside {outer:?}"
    );
}

pub(super) fn window_bounds(width: f32, height: f32) -> Bounds<gpui_kit::Pixels> {
    Bounds::new(point(px(0.), px(0.)), size(px(width), px(height)))
}

/// Labels of the open popup menu's rows, in order.
pub(super) fn menu_rows(cx: &mut TestAppContext, f: &Fixture) -> Vec<String> {
    on_window(cx, f, |window, _| {
        let menu = window.within("popup-menu");
        (0..60usize)
            .filter_map(|index| menu.try_find(index))
            .filter_map(|row| row.label().map(str::to_owned))
            .collect()
    })
}

pub(super) fn choose(cx: &mut TestAppContext, f: &Fixture, label: &str) {
    on_window(cx, f, |window, cx| {
        let mut menu = window.within("popup-menu");
        let index = (0..60usize)
            .find(|index| {
                menu.try_find(*index)
                    .is_some_and(|row| row.label() == Some(label))
            })
            .unwrap_or_else(|| panic!("menu row {label} not found"));
        menu.click(index, cx);
    });
}

fn row(cx: &mut TestAppContext, f: &Fixture) -> Option<workbench::WritingRow> {
    cx.read(|cx| f.view.read(cx).writing_bar_row())
}

#[gpui_kit::test]
fn note_toolbar_is_one_row_at_each_window_size_dirty_or_clean(cx: &mut TestAppContext) {
    for (width, height) in [(900., 600.), (1050., 700.), (1250., 860.)] {
        for navigation in [TileKind::Threads, TileKind::Notes] {
            for margin in [true, false] {
                let f = widest(cx, width, height);
                update(cx, &f, |view, window, cx| {
                    // Comments open or closed, beside the text or stacked.
                    view.margin = Some(margin);
                    view.comments_expanded = margin;
                    view.show_tile(navigation, window, cx);
                });
                assert!(cx.read(|cx| f.view.read(cx).layout.contains(navigation)));
                for dirty in [false, true] {
                    if dirty {
                        type_edit(cx, &f, " edited");
                    }
                    on_window(cx, &f, |window, cx| {
                        let fit = f.view.read(cx).note_fit;
                        let case = format!(
                            "{width}x{height} {navigation:?} margin {margin} dirty {dirty} {fit:?}"
                        );
                        let bar = window.find("note-toolbar").bounds();
                        assert_eq!(bar.size.height, px(36.), "{case}");
                        let shown: Vec<&str> = TOOLBAR_IDS
                            .into_iter()
                            .filter(|id| window.try_find(*id).is_some_and(|e| e.visible()))
                            .collect();
                        for id in [
                            "note-write",
                            "note-read",
                            "note-outline",
                            "note-status-chip",
                            "toggle-margin",
                        ] {
                            assert!(shown.contains(&id), "{case}: {id} always shows");
                        }
                        for id in &shown {
                            assert_in(window, id, bar);
                        }
                        assert_eq!(
                            window.find("note-status-chip").label(),
                            Some("Protected · v12"),
                            "{case}"
                        );
                        // The unsaved state lives in the writing bar.
                        assert!(
                            window
                                .within("note-toolbar")
                                .try_find("note-unsaved-chip")
                                .is_none()
                        );
                    });
                }
            }
        }
    }
}

/// INT-3: the toolbar folds by what this note's row needs. At a width where
/// the widest row ("Protected · v12", 12 comments) moves Edit note and
/// Connections into the Note ⋯, a plain note ("Working · v1", no comments)
/// keeps both in its one-row toolbar.
#[gpui_kit::test]
fn edit_note_and_connections_stay_in_the_toolbar_when_this_notes_row_fits(cx: &mut TestAppContext) {
    for (plain, f) in [
        (true, fixture(cx, 1330., 860., "Saved text")),
        (false, widest(cx, 1330., 860.)),
    ] {
        on_window(cx, &f, |window, cx| {
            let view = f.view.read(cx);
            let width = view.note_panel_width(window).unwrap();
            let needs = view.note_toolbar_needs(window);
            assert!(
                width < layout::NoteNeeds::WIDEST.format_menu
                    && (needs.format_menu <= width) == plain,
                "plain {plain}: {needs:?} at {width}"
            );
            let bar = window.find("note-toolbar").bounds();
            for id in ["edit-note-context", "note-connections"] {
                assert_eq!(
                    window.try_find(id).is_some(),
                    plain,
                    "plain {plain}: {id} in the toolbar"
                );
                if plain {
                    assert_in(window, id, bar);
                }
            }
            for id in TOOLBAR_IDS {
                if window.try_find(id).is_some_and(|e| e.visible()) {
                    assert_in(window, id, bar);
                }
            }
        });
    }
}

#[gpui_kit::test]
fn format_menu_holds_five_actions_and_outline_lists_headings_in_write_and_read(
    cx: &mut TestAppContext,
) {
    let f = fixture(
        cx,
        1250.,
        860.,
        "# Alpha\n\nOne two.\n\n## **Beta**\n\nThree.",
    );
    on_window(cx, &f, |window, cx| {
        assert!(f.view.read(cx).note_fit.format_menu());
        assert!(window.try_find("format-bold").is_none());
        window.click("format-menu", cx);
    });
    assert_eq!(
        menu_rows(cx, &f),
        ["Heading", "Bold", "Italic", "List", "Quote"]
    );
    on_window(cx, &f, |window, cx| window.press("escape", cx));
    update(cx, &f, |view, _, cx| {
        let start = "# Alpha\n\n".len();
        view.note_editor.update(cx, |editor, cx| {
            editor.set_selected_range(start..start + 3, cx)
        });
    });
    on_window(cx, &f, |window, cx| window.click("format-menu", cx));
    choose(cx, &f, "Bold");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(view.note_editor.read(cx).value().contains("**One** two."));
    });
    // Write: Outline lists the headings and reveals the chosen one.
    on_window(cx, &f, |window, cx| window.click("note-outline", cx));
    assert_eq!(menu_rows(cx, &f), ["Alpha", "Beta"]);
    choose(cx, &f, "Beta");
    cx.read(|cx| {
        let editor = f.view.read(cx).note_editor.read(cx);
        let range = editor.selected_range();
        assert_eq!(editor.value().get(range), Some("**Beta**"));
    });
    // Read shares the same Outline and has no formatting.
    update(cx, &f, |view, window, cx| {
        view.set_note_reading(true, window, cx)
    });
    on_window(cx, &f, |window, cx| {
        assert!(window.try_find("format-menu").is_none());
        window.click("note-outline", cx);
    });
    assert_eq!(menu_rows(cx, &f), ["Alpha", "Beta"]);
}

#[gpui_kit::test]
fn the_four_note_banners_stay_visible_in_write_at_900x600(cx: &mut TestAppContext) {
    let screen = window_bounds(900., 600.);
    // A kept edit from elsewhere makes the note read-only until recovered.
    let f = fixture(cx, 900., 600., "Saved text");
    update(cx, &f, |view, _, cx| {
        let edit = view
            .workspace
            .store
            .begin_edit(&f.note.id, f.note.version)
            .unwrap();
        view.workspace
            .store
            .update_buffer(&edit.id, 1, "Kept text")
            .unwrap();
        view.refresh();
        cx.notify();
    });
    on_window(cx, &f, |window, _| {
        assert_in(window, "note-recover-banner", screen);
        assert_in(window, "recover-note-edit", screen);
        assert_eq!(
            window.find("note-recover-banner-tone").label(),
            Some("Warning")
        );
        // Recover edit is the panel's one primary; Rewrite with AI waits
        // for it and says so.
        assert_eq!(
            window.find("rewrite-note").label(),
            Some("Rewrite with AI, unavailable: Recover the kept edit first")
        );
    });
    assert_eq!(row(cx, &f), Some(workbench::WritingRow::Recover));
    on_window(cx, &f, |window, cx| window.click("recover-note-edit", cx));
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("note-recover-banner").is_none())
    });

    // A newer saved version, with no open edit.
    let f = fixture(cx, 900., 600., "Saved text");
    change_saved(cx, &f, |note| note.markdown = "Changed elsewhere".into());
    on_window(cx, &f, |window, _| {
        assert_in(window, "note-newer-banner", screen);
        assert_in(window, "show-current-note", screen);
        assert!(window.try_find("note-stale-banner").is_none());
    });

    // The edit's base fell behind: one banner states both facts, with the
    // saved version below it and Save as new note as the panel's primary.
    type_edit(cx, &f, " and my words");
    on_window(cx, &f, |window, _| {
        assert!(window.try_find("note-newer-banner").is_none());
        let banner = window.find("note-stale-banner").bounds();
        assert_in(window, "note-stale-banner", screen);
        assert_in(window, "save-buffer-copy", banner);
        assert_in(window, "show-current-note", banner);
        assert_in(window, "note-stale-preview", screen);
        let preview = window.find("note-stale-preview").bounds();
        assert!(preview.size.height <= px(212.), "{preview:?}");
        assert!(preview.top() >= banner.bottom());
        // Save as new note renders once; the bar keeps Save as a ghost.
        assert!(
            window
                .within("note-bar")
                .try_find("save-buffer-copy")
                .is_none()
        );
        // The text keeps a few lines; the preview gives up height first.
        assert!(window.find("note-body").bounds().size.height >= px(48.));
        assert_in(window, "save-note", screen);
        assert!(window.find("save-note").visible());
        assert!(window.find("note-unsaved-chip").visible());
    });
    assert_eq!(row(cx, &f), Some(workbench::WritingRow::DirtyStale));
    on_window(cx, &f, |window, cx| window.click("save-buffer-copy", cx));
    cx.read(|cx| {
        let view = f.view.read(cx);
        let open = view.note.as_ref().unwrap();
        assert_ne!(open.record.id, f.note.id, "the text became a new note");
        assert!(matches!(&open.record.data, RecordData::Note(n) if n.markdown == "Saved text and my words"));
    });

    // Import summary: warning while coverage is partial, info when checked.
    for (status, issues, tone) in [
        (
            ImportStatus::Partial,
            vec![ImportIssue {
                kind: ImportIssueKind::Omission,
                locator: "page 2".into(),
                detail: "A chart was not converted.".into(),
            }],
            "Warning",
        ),
        (ImportStatus::Complete, vec![], "Info"),
    ] {
        let f = fixture(cx, 900., 600., "Imported text");
        let source = update(cx, &f, |view, _, _| {
            let key = format!("polish-source-{tone}");
            let source = view
                .workspace
                .store
                .allocate_operation(&key)
                .unwrap()
                .creation_id(0);
            view.workspace
                .owner_change(
                    &key,
                    &ChangeRequest {
                        reason: "Synthetic import source".into(),
                        writes: vec![Put {
                            id: source.clone(),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Source(Source {
                                chat: None,
                                locator: "synthetic://polish-import".into(),
                                label: "Synthetic source".into(),
                                observed_at: "synthetic".into(),
                                external_version: None,
                                outcome: "Partial".into(),
                                gaps: vec![],
                                report: None,
                            }),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            view.refresh();
            source
        });
        change_saved(cx, &f, |note| {
            note.import = Some(ImportCoverage {
                source: source.clone(),
                full_note: false,
                gaps: vec![],
                report: Some(ImportReport {
                    status,
                    issues: issues.clone(),
                }),
            })
        });
        let id = f.note.id.clone();
        update(cx, &f, |view, window, cx| {
            view.open_note(&id, None, window, cx)
        });
        on_window(cx, &f, |window, _| {
            assert_in(window, "note-import-summary", screen);
            assert_in(window, "open-import-original", screen);
            assert_eq!(window.find("note-import-summary-tone").label(), Some(tone));
            assert_eq!(
                window
                    .try_find("toggle-coverage")
                    .map(|e| e.label().map(str::to_owned)),
                (!issues.is_empty()).then(|| Some("Coverage details (1)".to_owned()))
            );
        });
    }
}

#[gpui_kit::test]
fn a_dirty_protected_note_keeps_protection_and_shows_unsaved_in_the_bar(cx: &mut TestAppContext) {
    let f = widest(cx, 1250., 860.);
    type_edit(cx, &f, " edited");
    update(cx, &f, |view, _, cx| {
        view.note_editor
            .update(cx, |editor, cx| editor.set_selected_range(10..15, cx));
        view.track_selection(cx);
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("note-status-chip").label(),
            Some("Protected · v12")
        );
        let bar = window.find("note-bar").bounds();
        assert_in(window, "note-unsaved-chip", bar);
        assert_eq!(
            window.find("note-unsaved-chip").label(),
            Some("Unsaved edit · recoverable")
        );
        assert_eq!(
            window.find("selection-comment").label(),
            Some("Save and comment")
        );
        assert_eq!(
            window.find("selection-comment-reason").label(),
            Some("Save and comment saves your edit first, then comments on this passage.")
        );
        // The strip sits directly above the text.
        assert!(
            window.find("selection-toolbar").bounds().bottom()
                <= window.find("note-body").bounds().top()
        );
    });
    assert_eq!(row(cx, &f), Some(workbench::WritingRow::Dirty));
    // In a narrow Note the chip shows its version and keeps its full name.
    let f = widest(cx, 900., 600.);
    on_window(cx, &f, |window, cx| {
        // 900 pt stacks the comments, so the Note keeps about 406 pt.
        assert_eq!(f.view.read(cx).margin_mode, layout::MarginMode::Stacked);
        assert!(f.view.read(cx).note_fit.icons());
        assert_eq!(
            window.find("note-status-chip").label(),
            Some("Protected · v12")
        );
        assert_eq!(window.find("note-outline").label(), Some("Outline"));
        assert_eq!(window.find("format-menu").label(), Some("Format"));
    });
    // Edit note and Connections move into the Note ⋯ below the threshold.
    on_window(cx, &f, |window, cx| {
        assert!(window.try_find("edit-note-context").is_none());
        assert!(window.try_find("note-connections").is_none());
        window.click("note-menu", cx);
    });
    let rows = menu_rows(cx, &f);
    assert!(rows.contains(&"Edit note…".to_owned()), "{rows:?}");
    assert!(rows.contains(&"Connections".to_owned()), "{rows:?}");
    choose(cx, &f, "Connections");
    cx.read(|cx| assert!(f.view.read(cx).layout.contains(TileKind::Connections)));
}

/// Record a writing run for the open note's latest saved rewrite request.
pub(super) fn record_rewrite_run(
    cx: &mut TestAppContext,
    f: &Fixture,
    state: RunState,
    progress: &str,
) {
    update(cx, f, |view, _, cx| {
        let note = f.note.id.clone();
        let thread = view.writing_threads.get(&note).unwrap().clone();
        let request = view.latest_note_writing_request(&note).unwrap().id;
        let key = format!("rewrite-run-{}", uuid::Uuid::new_v4());
        let operation = view.workspace.store.allocate_operation(&key).unwrap();
        let settings = view.workspace.settings().unwrap();
        view.workspace
            .owner_change(
                &key,
                &ChangeRequest {
                    reason: "Synthetic rewrite run".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: operation.creation_id(0),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Run(Run {
                            instruction: Some(request),
                            queue_claim: None,
                            thread,
                            provider: "offline-fixture".into(),
                            model: settings.model,
                            guide_identity: "polish-fixture".into(),
                            budget: 1,
                            effort: settings.effort,
                            loaded_skills: vec![],
                            fence: 0,
                            state,
                            progress: progress.into(),
                            operations: vec![],
                            sources: vec![],
                        }),
                    }],
                },
            )
            .unwrap();
        view.refresh();
        cx.notify();
    });
}

#[gpui_kit::test]
fn each_writing_state_has_one_primary_and_run_outcomes_show_their_full_reason(
    cx: &mut TestAppContext,
) {
    use workbench::WritingRow;
    let f = fixture(cx, 1250., 860., "Saved text for a rewrite.");
    update(cx, &f, |view, window, cx| {
        let thread = view.workspace.new_thread("Rewrite chat").unwrap().id;
        view.workspace
            .link_writing_thread("polish-link", &f.note.id, f.note.version, &thread)
            .unwrap();
        view.refresh();
        view.select_writing_thread(f.note.id.clone(), thread, window, cx);
    });
    assert_eq!(row(cx, &f), Some(WritingRow::Idle));
    on_window(cx, &f, |window, _| {
        assert!(window.find("rewrite-note").visible());
        assert!(window.try_find("writing-run-status").is_none());
    });
    // Offline: the request is saved and no AI starts.
    update(cx, &f, |view, _, cx| view.rewrite_note(cx));
    assert_eq!(row(cx, &f), Some(WritingRow::Retry));
    on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("writing-run-status").label(),
            Some("Rewrite inputs saved")
        );
        assert!(
            window.try_find("writing-run-status-tone").is_none(),
            "neutral"
        );
        assert!(window.find("retry-rewrite").visible());
        assert!(window.find("rewrite-note").visible());
    });
    // U6: a long recorded failure reason passes through whole and wraps.
    let reason = format!(
        "Run failed: Reconnect this account before continuing. {}",
        "Synthetic provider detail that the owner needs to read in full. ".repeat(4)
    );
    record_rewrite_run(cx, &f, RunState::Failed, &reason);
    assert_eq!(row(cx, &f), Some(WritingRow::Retry));
    on_window(cx, &f, |window, _| {
        let status = window.find("writing-run-status");
        assert_eq!(
            status.label(),
            Some(format!("Rewrite failed · {reason}").as_str())
        );
        assert_eq!(
            window.find("writing-run-status-tone").label(),
            Some("Error")
        );
        let note = window.find("tile-note").bounds();
        assert_in(window, "writing-run-status", note);
        assert!(
            status.bounds().size.height > px(40.),
            "the reason wraps instead of truncating: {:?}",
            status.bounds()
        );
    });
    for (state, progress, text, tone, expected) in [
        (
            RunState::Interrupted,
            "Lost connection",
            "Rewrite interrupted · Lost connection",
            Some("Warning"),
            WritingRow::Retry,
        ),
        (
            RunState::Cancelled,
            "",
            "Rewrite stopped",
            Some("Warning"),
            WritingRow::Retry,
        ),
        (
            RunState::Working,
            "Started elsewhere",
            "This rewrite may still be running elsewhere. Retry starts a new attempt.",
            Some("Warning"),
            WritingRow::Retry,
        ),
        (
            RunState::Completed,
            "Done",
            "Rewrite finished · no proposal",
            None,
            WritingRow::Finished,
        ),
    ] {
        record_rewrite_run(cx, &f, state, progress);
        assert_eq!(row(cx, &f), Some(expected), "{text}");
        on_window(cx, &f, |window, _| {
            assert_eq!(window.find("writing-run-status").label(), Some(text));
            assert_eq!(
                window
                    .try_find("writing-run-status-tone")
                    .and_then(|e| e.label().map(str::to_owned)),
                tone.map(str::to_owned)
            );
            assert!(window.find("retry-rewrite").visible());
            assert!(window.find("rewrite-note").visible());
        });
    }
    // A ready proposal owns the primary; an open edit then takes it over.
    let operation = update(cx, &f, |view, _, cx| {
        let request = view.latest_note_writing_request(&f.note.id).unwrap().id;
        let snapshot = view.workspace.writing_request(&request).unwrap();
        let operation = view
            .workspace
            .store
            .allocate_operation("polish-proposal")
            .unwrap();
        view.workspace
            .store
            .prepare(
                &operation,
                &snapshot.change_request("A rewritten saved text.").unwrap(),
            )
            .unwrap();
        view.refresh();
        cx.notify();
        operation.as_str().to_owned()
    });
    assert_eq!(row(cx, &f), Some(WritingRow::ProposalReady));
    on_window(cx, &f, |window, _| {
        assert!(window.find(format!("writing-review-{operation}")).visible());
        assert!(window.try_find("writing-run-status").is_none());
    });
    type_edit(cx, &f, " Edited.");
    assert_eq!(row(cx, &f), Some(WritingRow::Dirty));
    on_window(cx, &f, |window, _| {
        assert!(window.find("save-note").visible());
        assert!(window.find(format!("writing-review-{operation}")).visible());
        assert!(window.find("discard-note").visible());
        assert!(window.find("save-buffer-copy").visible());
        // Blocked controls say why in their names.
        assert_eq!(
            window.find("rewrite-note").label(),
            Some("Rewrite with AI, unavailable: Save your edit first")
        );
    });
}

#[gpui_kit::test]
fn discard_always_confirms_and_enter_never_discards(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., "Saved text");
    cx.update(|cx| cx.bind_keys(bindings()));
    type_edit(cx, &f, " unsaved");
    let unsaved = cx.read(|cx| f.view.read(cx).note_editor.read(cx).value().to_string());
    on_window(cx, &f, |window, cx| window.click("discard-note", cx));
    on_window(cx, &f, |window, cx| {
        assert!(window.has_active_dialog(cx));
        assert_eq!(
            window.find("discard-note-body").label(),
            Some("Your changes since v1 will be lost. This can't be undone.")
        );
        assert!(window.find("discard-note-confirm").visible());
        window.press("enter", cx);
    });
    on_window(cx, &f, |window, cx| {
        assert!(window.has_active_dialog(cx), "↵ leaves the dialog open");
        window.click("discard-note-cancel", cx);
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(
            view.note.as_ref().unwrap().session.is_some(),
            "Cancel keeps the edit"
        );
        assert_eq!(view.note_editor.read(cx).value().as_ref(), unsaved);
    });
    on_window(cx, &f, |window, cx| {
        assert!(!window.has_active_dialog(cx));
        window.click("note-menu", cx);
    });
    let rows = menu_rows(cx, &f);
    let panel = rows.iter().position(|row| row == "Panel").unwrap();
    assert_eq!(
        rows[panel - 1],
        "Discard edit…",
        "Discard comes last: {rows:?}"
    );
    choose(cx, &f, "Discard edit…");
    on_window(cx, &f, |window, cx| {
        assert!(
            window.has_active_dialog(cx),
            "the menu opens the same dialog"
        );
        window.click("discard-note-confirm", cx);
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        let open = view.note.as_ref().unwrap();
        assert!(open.session.is_none());
        assert_eq!(view.note_editor.read(cx).value().as_ref(), "Saved text");
        assert!(view.workspace.store.recovery_buffers().unwrap().is_empty());
    });
    on_window(cx, &f, |window, cx| assert!(!window.has_active_dialog(cx)));
}

#[gpui_kit::test]
fn the_text_gets_most_of_the_note_and_keeps_a_readable_measure(cx: &mut TestAppContext) {
    let markdown = (1..=24)
        .map(|index| format!("## Heading {index}\n\nParagraph {index}."))
        .collect::<Vec<_>>()
        .join("\n\n");
    let f = fixture(cx, 1250., 860., &markdown);
    on_window(cx, &f, |window, _| {
        let tile = window.find("tile-note").bounds();
        let body = window.find("note-body").bounds();
        let share = body.size.height / tile.size.height;
        assert!(share >= 0.7, "text gets {share:.2} of the Note");
        let bar = window.find("note-bar").bounds();
        assert!(bar.size.height <= px(72.), "{bar:?}");
        assert!(window.try_find("section-0").is_none());
    });
    // Maximized, the text keeps a 680 px measure with 24 px gutters and is
    // centered; in the split it fills the narrower panel from the left.
    on_window(cx, &f, |window, _| {
        let (body, column) = (
            window.find("note-body").bounds(),
            window.find("note-column").bounds(),
        );
        assert_eq!(column.left(), body.left());
    });
    update(cx, &f, |view, window, cx| {
        view.relayout(window, cx, |layout| layout.toggle_maximize(TileKind::Note))
    });
    on_window(cx, &f, |window, _| {
        let (body, column) = (
            window.find("note-body").bounds(),
            window.find("note-column").bounds(),
        );
        assert!(body.size.width > px(900.), "{body:?}");
        assert_eq!(column.size.width, px(728.));
        let (left, right) = (column.left() - body.left(), body.right() - column.right());
        assert!(
            (left - right).abs() <= px(1.),
            "centered: {left:?} {right:?}"
        );
    });
}
