//! Native journeys for the selectable Markdown reading and formatting controls.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
    note: Record,
}

fn fixture(cx: &mut TestAppContext, markdown: &str) -> Fixture {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("document-native");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let note = workspace.new_note("Document fixture", markdown).unwrap();
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
    Fixture {
        _root: root,
        window,
        view,
        note,
    }
}

fn open_note(cx: &mut TestAppContext, fixture: &Fixture) {
    cx.update_window(fixture.window, |_, window, cx| {
        fixture.view.update(cx, |view, cx| {
            view.open_note(&fixture.note.id, None, window, cx)
        });
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn writing_read_switch_and_navigation_keep_the_same_recoverable_buffer(cx: &mut TestAppContext) {
    let initial = "## Café 🦀\n\nA **formatted** paragraph.";
    let typed = "## Café 🦀\n\nA **formatted** paragraph with an edit.";
    let f = fixture(cx, initial);
    open_note(cx, &f);

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.note_editor
                .update(cx, |editor, cx| editor.set_value(typed, window, cx));
            view.preserve_visible_note_input(cx);
            view.set_note_reading(true, window, cx);
            assert!(view.note_reading);
            assert!(view.note.as_ref().unwrap().session.is_some());
            assert_eq!(view.note_editor.read(cx).value().as_ref(), typed);
            assert_eq!(view.note_read_view.read(cx).rendered_text().source(), typed);
            assert!(view.note_editor.read(cx).value().contains("**formatted**"));

            view.set_note_reading(false, window, cx);
            assert!(!view.note_reading);
            assert_eq!(view.note_editor.read(cx).value().as_ref(), typed);
        });
    })
    .unwrap();
    cx.run_until_parked();

    // Navigate away and back to the real note; its typed text remains in the
    // persisted edit session and the derived Read view follows that source.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let second = view.workspace.new_note("Other", "Other note").unwrap();
            view.open_note(&second.id, None, window, cx);
            view.open_note(&f.note.id, None, window, cx);
            assert_eq!(view.note_editor.read(cx).value().as_ref(), typed);
            assert!(view.note.as_ref().unwrap().session.is_some());
            view.set_note_reading(true, window, cx);
            assert_eq!(view.note_read_view.read(cx).rendered_text().source(), typed);
            assert_eq!(
                view.workspace.store.note(&f.note.id).unwrap().unwrap(),
                f.note,
                "switching and navigating must not save the dirty buffer"
            );
        });
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn read_select_all_comments_the_exact_markdown_source_without_starting_ai(cx: &mut TestAppContext) {
    let source = "# Café 🦀\n\nFirst **echo**.\n\nSecond echo.";
    let f = fixture(cx, source);
    open_note(cx, &f);

    cx.update_window(f.window, |_, window, cx| {
        f.view
            .update(cx, |view, cx| view.set_note_reading(true, window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.press("cmd-a", cx))
        .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.track_selection(cx);
            assert_eq!(
                view.selection.as_ref(),
                Some(&(0..source.len(), source.to_owned())),
                "rendered select-all must map to the exact original Markdown bytes"
            );
            view.comment_on_selection(window, cx);
            assert_eq!(
                view.comment_anchor,
                Some((
                    f.note.id.clone(),
                    f.note.version,
                    0..source.len(),
                    source.into()
                ))
            );
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("comment-composer").is_some());
        f.view.update(cx, |view, cx| {
            view.comment_input.update(cx, |editor, cx| {
                editor.set_value(
                    "Keep the heading and emphasize the first occurrence.",
                    window,
                    cx,
                )
            });
            view.post_comment(window, cx);

            let comments = view.workspace.writing_comments(&f.note.id).unwrap();
            assert_eq!(comments.len(), 1);
            let RecordData::Comment(comment) = &comments[0].record.data else {
                panic!("expected a passage comment");
            };
            assert_eq!(comment.range, Some((0, source.len())));
            assert_eq!(comment.quote, source);
            assert!(view.worker.is_none(), "adding feedback must not start AI");
            let thread = view.workspace.writing_threads(&f.note.id).unwrap();
            assert_eq!(thread.len(), 1);
            assert!(view.workspace.messages(&thread[0].id).unwrap().is_empty());
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn editing_after_read_selection_clears_old_range_before_comment_shortcut(cx: &mut TestAppContext) {
    let source = "## Café 🦀\n\nOriginal paragraph.";
    let f = fixture(cx, source);
    open_note(cx, &f);

    cx.update_window(f.window, |_, window, cx| {
        f.view
            .update(cx, |view, cx| view.set_note_reading(true, window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.press("cmd-a", cx))
        .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.track_selection(cx);
            assert_eq!(
                view.note_read_view.read(cx).selected_source_range(),
                Some(0..source.len())
            );
            assert!(view.selection.is_some());
            view.set_note_reading(false, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();

    let edited = format!("{source}\n\nA new ending.");
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.note_editor.update(cx, |editor, cx| {
                editor.set_value(edited.clone(), window, cx)
            });
            assert_eq!(view.note_editor.read(cx).value().as_ref(), edited);
            view.preserve_visible_note_input(cx);
            assert_eq!(view.note_editor.read(cx).value().as_ref(), edited);
        });
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.set_note_reading(true, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, _, cx| {
        f.view.read_with(cx, |view, cx| {
            assert_eq!(view.read_source, edited);
            assert_eq!(
                view.note_read_view.read(cx).rendered_text().source(),
                edited
            );
            assert_eq!(
                view.note_read_view.read(cx).selected_source_range(),
                None,
                "a source change must not retain rendered ranges from the prior source"
            );
            assert!(view.selection.is_none());
        });
    })
    .unwrap();

    cx.update_window(f.window, |_, window, cx| window.press("cmd-/", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("comment-composer").is_none());
        f.view.update(cx, |view, _| {
            assert!(view.selection.is_none());
            assert!(view.comment_anchor.is_none());
            assert!(
                view.workspace
                    .writing_comments(&f.note.id)
                    .unwrap()
                    .is_empty()
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn save_and_comment_saves_then_opens_exact_quote_without_posting(cx: &mut TestAppContext) {
    let initial = "A plain sentence.";
    let typed = "A clear café sentence.";
    let quote = "café";
    let range = typed.find(quote).unwrap()..typed.find(quote).unwrap() + quote.len();
    let f = fixture(cx, initial);
    open_note(cx, &f);

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.note_editor
                .update(cx, |editor, cx| editor.set_value(typed, window, cx));
            view.preserve_visible_note_input(cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |view, cx| {
            view.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(range.clone(), cx)
            });
            view.track_selection(cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(
            window
                .find("selection-comment")
                .label()
                .unwrap()
                .contains("Save and comment")
        );
        window.click("selection-comment", cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let saved = view.workspace.store.note(&f.note.id).unwrap().unwrap();
            let RecordData::Note(note) = &saved.data else {
                panic!("expected a note");
            };
            assert_eq!(note.markdown, typed);
            assert!(view.note.as_ref().unwrap().session.is_none());
            assert_eq!(
                view.comment_anchor,
                Some((f.note.id.clone(), saved.version, range, quote.into()))
            );
            assert!(
                view.workspace
                    .writing_comments(&f.note.id)
                    .unwrap()
                    .is_empty()
            );
            assert!(view.worker.is_none());
            assert!(view.comment_input.read(cx).value().is_empty());
        });
        assert!(window.try_find("comment-composer").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn stale_save_and_comment_keeps_typed_source_and_comment_draft(cx: &mut TestAppContext) {
    let initial = "Saved base with date.";
    let typed = "My draft with a clearer date.";
    let f = fixture(cx, initial);
    open_note(cx, &f);

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let RecordData::Note(mut current) = f.note.data.clone() else {
                panic!("expected a note");
            };
            current.markdown = "A newer concurrent version.".into();
            assert!(matches!(
                view.workspace
                    .owner_change(
                        "document-test-concurrent-save",
                        &ChangeRequest {
                            reason: "Concurrent synthetic update".into(),
                            writes: vec![Put {
                                id: f.note.id.clone(),
                                expected_version: Some(f.note.version),
                                archived: false,
                                data: RecordData::Note(current),
                            }],
                            inputs: vec![],
                        },
                    )
                    .unwrap(),
                ApplyOutcome::Applied(_)
            ));

            let needle = "clearer date";
            let range = typed.find(needle).unwrap()..typed.find(needle).unwrap() + needle.len();
            view.note_editor
                .update(cx, |editor, cx| editor.set_value(typed, window, cx));
            view.preserve_visible_note_input(cx);
            view.note_editor
                .update(cx, |editor, cx| editor.set_selected_range(range, cx));
            view.track_selection(cx);
            view.comment_input.update(cx, |editor, cx| {
                editor.set_value("Preserve the date uncertainty.", window, cx)
            });
            view.save_and_comment(window, cx);

            assert_eq!(view.note_editor.read(cx).value().as_ref(), typed);
            assert!(view.note.as_ref().unwrap().session.is_some());
            let RecordData::Note(latest) =
                view.workspace.store.note(&f.note.id).unwrap().unwrap().data
            else {
                unreachable!()
            };
            assert_eq!(latest.markdown, "A newer concurrent version.");
            assert_eq!(
                view.comment_input.read(cx).value().as_ref(),
                "Preserve the date uncertainty."
            );
            assert!(
                view.comment_anchor.is_none(),
                "a stale range must not become a saved anchor"
            );
            assert!(
                view.workspace
                    .writing_comments(&f.note.id)
                    .unwrap()
                    .is_empty()
            );
            assert!(view.worker.is_none());
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn formatting_controls_refuse_read_recovery_and_review_states(cx: &mut TestAppContext) {
    let source = "Café and 🦀";
    let cafe_end = "Café".len();
    let f = fixture(cx, source);
    open_note(cx, &f);

    // Read mode has no formatting actions, and a programmatic call is inert.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.note_editor
                .update(cx, |editor, cx| editor.set_selected_range(0..cafe_end, cx));
            view.set_note_reading(true, window, cx);
            view.format_note(formatting::Format::Bold, window, cx);
            assert_eq!(view.note_editor.read(cx).value().as_ref(), source);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("format-bold").is_none())
    })
    .unwrap();

    // A recoverable buffer opened by another editor blocks all writes.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.set_note_reading(false, window, cx);
            let edit = view
                .workspace
                .store
                .begin_edit(&f.note.id, f.note.version)
                .unwrap();
            view.workspace
                .store
                .update_buffer(&edit.id, 1, "External recoverable draft")
                .unwrap();
            view.refresh();
            assert!(
                view.recovery_blocked(),
                "the synthetic recovery buffer should block formatting"
            );
            view.note_editor
                .update(cx, |editor, cx| editor.set_selected_range(0..cafe_end, cx));
            view.format_note(formatting::Format::Bold, window, cx);
            assert_eq!(view.note_editor.read(cx).value().as_ref(), source);
        });
    })
    .unwrap();
    cx.run_until_parked();
    // The toolkit button's disabled state is enforced by pointer behavior and
    // styling but is not exposed in the test accessibility snapshot. Exercise
    // the actual control; the blocked recovery buffer must remain untouched.
    cx.update_window(f.window, |_, window, cx| window.click("format-bold", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("format-bold").visible());
    })
    .unwrap();
    f.view.read_with(cx, |view, cx| {
        assert_eq!(view.note_editor.read(cx).value().as_ref(), source);
        assert!(view.recovery_blocked());
    });

    // A real prepared candidate owns the Note tile while displayed review is active.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let thread = view.workspace.new_thread("Review fixture").unwrap();
            view.workspace
                .link_writing_thread(
                    "document-review-link",
                    &f.note.id,
                    f.note.version,
                    &thread.id,
                )
                .unwrap();
            let snapshot = view
                .workspace
                .snapshot_writing_request(
                    "document-review-request",
                    &f.note.id,
                    f.note.version,
                    &thread.id,
                    &[],
                )
                .unwrap();
            let operation = view
                .workspace
                .store
                .allocate_operation("document-review-candidate")
                .unwrap();
            let candidate = view
                .workspace
                .store
                .prepare(
                    &operation,
                    &snapshot.change_request("Changed candidate text").unwrap(),
                )
                .unwrap();
            view.open_review(candidate, window, cx);
            view.note_editor
                .update(cx, |editor, cx| editor.set_selected_range(0..cafe_end, cx));
            view.format_note(formatting::Format::Bold, window, cx);
            assert_eq!(view.note_editor.read(cx).value().as_ref(), source);
            assert!(view.review.is_some());
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("format-bold").is_none());
        assert!(window.try_find("review-bar").is_some());
    })
    .unwrap();
}
