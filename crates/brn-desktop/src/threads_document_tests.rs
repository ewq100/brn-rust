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
            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_value(typed, window, cx));
            view.preserve_visible_note_input(cx);
            view.set_note_reading(true, window, cx);
            assert!(view.writing.note.is_reading());
            assert!(view.writing.note.as_ref().unwrap().session().is_some());
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), typed);
            assert_eq!(
                view.writing
                    .note_read_view
                    .read(cx)
                    .rendered_text()
                    .source(),
                typed
            );
            assert!(
                view.writing
                    .note_editor
                    .read(cx)
                    .value()
                    .contains("**formatted**")
            );

            view.set_note_reading(false, window, cx);
            assert!(!view.writing.note.is_reading());
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), typed);
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
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), typed);
            assert!(view.writing.note.as_ref().unwrap().session().is_some());
            view.set_note_reading(true, window, cx);
            assert_eq!(
                view.writing
                    .note_read_view
                    .read(cx)
                    .rendered_text()
                    .source(),
                typed
            );
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
    cx.update_window(f.window, |_, window, cx| {
        window.press(super::focus_tests::SELECT_ALL, cx)
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.track_selection(cx);
            assert_eq!(
                view.writing.selection.as_ref(),
                Some(&(0..source.len(), source.to_owned())),
                "rendered select-all must map to the exact original Markdown bytes"
            );
            view.comment_on_selection(window, cx);
            assert_eq!(
                view.writing.comment_anchor,
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
            view.writing.comment_input.update(cx, |editor, cx| {
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
    cx.update_window(f.window, |_, window, cx| {
        window.press(super::focus_tests::SELECT_ALL, cx)
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.track_selection(cx);
            assert_eq!(
                view.writing.note_read_view.read(cx).selected_source_range(),
                Some(0..source.len())
            );
            assert!(view.writing.selection.is_some());
            view.set_note_reading(false, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();

    let edited = format!("{source}\n\nA new ending.");
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.writing.note_editor.update(cx, |editor, cx| {
                editor.set_value(edited.clone(), window, cx)
            });
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), edited);
            view.preserve_visible_note_input(cx);
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), edited);
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
            assert_eq!(view.writing.read_source, edited);
            assert_eq!(
                view.writing
                    .note_read_view
                    .read(cx)
                    .rendered_text()
                    .source(),
                edited
            );
            assert_eq!(
                view.writing.note_read_view.read(cx).selected_source_range(),
                None,
                "a source change must not retain rendered ranges from the prior source"
            );
            assert!(view.writing.selection.is_none());
        });
    })
    .unwrap();

    cx.update_window(f.window, |_, window, cx| window.press("cmd-/", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("comment-composer").is_none());
        f.view.update(cx, |view, _| {
            assert!(view.writing.selection.is_none());
            assert!(view.writing.comment_anchor.is_none());
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
            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_value(typed, window, cx));
            view.preserve_visible_note_input(cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |view, cx| {
            view.writing.note_editor.update(cx, |editor, cx| {
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
            assert!(view.writing.note.as_ref().unwrap().session().is_none());
            assert_eq!(
                view.writing.comment_anchor,
                Some((f.note.id.clone(), saved.version, range, quote.into()))
            );
            assert!(
                view.workspace
                    .writing_comments(&f.note.id)
                    .unwrap()
                    .is_empty()
            );
            assert!(view.worker.is_none());
            assert!(view.writing.comment_input.read(cx).value().is_empty());
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
            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_value(typed, window, cx));
            view.preserve_visible_note_input(cx);
            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_selected_range(range, cx));
            view.track_selection(cx);
            view.writing.comment_input.update(cx, |editor, cx| {
                editor.set_value("Preserve the date uncertainty.", window, cx)
            });
            view.save_and_comment(window, cx);

            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), typed);
            assert!(view.writing.note.as_ref().unwrap().session().is_some());
            let RecordData::Note(latest) =
                view.workspace.store.note(&f.note.id).unwrap().unwrap().data
            else {
                unreachable!()
            };
            assert_eq!(latest.markdown, "A newer concurrent version.");
            assert_eq!(
                view.writing.comment_input.read(cx).value().as_ref(),
                "Preserve the date uncertainty."
            );
            assert!(
                view.writing.comment_anchor.is_none(),
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
fn late_checked_refresh_preserves_newer_typed_buffer_and_stale_save(cx: &mut TestAppContext) {
    let initial = "Version one: saved text.";
    let typed = "## Owner draft — Café 🦀\n\nFirst line with naïve details.\nSecond line keeps the exact wording.";
    let f = fixture(cx, initial);
    open_note(cx, &f);

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let current = view.workspace.store.note(&f.note.id).unwrap().unwrap();
            let RecordData::Note(mut changed) = current.data.clone() else {
                panic!("expected a note");
            };
            changed.markdown = "Version two: checked external update.".into();
            assert!(matches!(
                view.workspace
                    .owner_change(
                        "late-refresh-external-v2",
                        &ChangeRequest {
                            reason: "Synthetic checked update before completion refresh".into(),
                            writes: vec![Put {
                                id: current.id.clone(),
                                expected_version: Some(current.version),
                                archived: current.archived,
                                data: RecordData::Note(changed),
                            }],
                            inputs: vec![],
                        },
                    )
                    .unwrap(),
                ApplyOutcome::Applied(_)
            ));
            let external_v2 = view.workspace.store.note(&f.note.id).unwrap().unwrap();

            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_value(typed, window, cx));
            assert!(
                view.writing.note.as_ref().unwrap().session().is_none(),
                "the Change event must not have persisted an edit guard before refresh"
            );

            // This is the completion refresh boundary: do not preserve the input
            // manually or yield to a queued Change event before delivering it.
            view.reload_open_note(window, cx);

            assert_eq!(
                view.writing.note_editor.read(cx).value().as_ref(),
                typed,
                "late refresh must preserve the visible text"
            );
            let session = view
                .writing
                .note
                .as_ref()
                .unwrap()
                .session()
                .expect("refresh must preserve the visible edit")
                .clone();
            assert_eq!(session.note, f.note.id);
            assert_eq!(session.base_version, f.note.version);
            assert_eq!(session.generation, 1);
            assert_eq!(session.markdown, typed);
            assert!(!session.closed);

            let recovery = view
                .workspace
                .store
                .recovery_buffers()
                .unwrap()
                .into_iter()
                .find(|buffer| buffer.note == f.note.id)
                .expect("the preserved edit must be durable");
            assert_eq!(
                recovery, session,
                "UI and durable session identity/generation must agree"
            );
            assert_eq!(external_v2.version, f.note.version + 1);
            assert!(matches!(
                &external_v2.data,
                RecordData::Note(note) if note.markdown == "Version two: checked external update."
            ));

            let outcome = view
                .writing
                .save_note(&mut view.workspace, window, cx)
                .unwrap()
                .unwrap();
            let SaveOutcome::Stale(kept) = outcome else {
                panic!("expected stale Save, got {outcome:?}");
            };
            assert_eq!(kept, session);
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), typed);
            assert_eq!(
                view.workspace.store.recovery_buffers().unwrap(),
                vec![session],
                "stale Save must retain the same recovery session and generation"
            );
            assert_eq!(
                view.workspace.store.note(&f.note.id).unwrap().unwrap(),
                external_v2,
                "stale Save must not replace checked canonical v2"
            );
        });
    })
    .unwrap();
    cx.run_until_parked();
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
            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_selected_range(0..cafe_end, cx));
            view.set_note_reading(true, window, cx);
            view.format_note(formatting::Format::Bold, window, cx);
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), source);
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
            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_selected_range(0..cafe_end, cx));
            view.format_note(formatting::Format::Bold, window, cx);
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), source);
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
        assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), source);
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
                    &snapshot.change_request("Changed proposal text").unwrap(),
                )
                .unwrap();
            view.open_review(candidate, window, cx);
            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_selected_range(0..cafe_end, cx));
            view.format_note(formatting::Format::Bold, window, cx);
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), source);
            assert!(view.writing.review.is_some());
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

/// The Read view's Markdown policy is observable in the rendered plain copy:
/// block HTML stays literal text (the default parser strips its tags), and the
/// text of a `javascript:` link is still rendered, while stored images and
/// link destinations contribute nothing. An image whose asset is not stored
/// renders a bounded placeholder with its alternative text (spec §10.1). The
/// exact string also fails if the plugin registration changes (for example both plugins becoming block parsers would
/// render the inline tags literally). Inline HTML literalness is a visual
/// property the plain copy excludes by design, so it stays a native check.
#[gpui_kit::test]
fn read_view_shows_block_html_literally_and_renders_all_link_text(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("document-policy");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let note = workspace.new_note("Policy fixture", "placeholder").unwrap();
    let note_id = note.id.clone();
    let op = workspace.store.allocate_operation("seed-policy").unwrap();
    let asset_id = op.creation_id(0);
    let source = format!(
        "Intro with <b>inline html</b>.\n\n<div>block html</div>\n\n![fig](brn-asset://{a})\n\n![gone](brn-asset://gone)\n\n[good](https://example.com) and [bad](javascript:alert(1))",
        a = asset_id
    );
    workspace
        .owner_change(
            "seed-policy",
            &ChangeRequest {
                reason: "Seed asset and Markdown".into(),
                inputs: vec![],
                writes: vec![
                    Put {
                        id: asset_id.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Asset(Asset {
                            note: note_id.clone(),
                            source: None,
                            media_type: "image/png".into(),
                            bytes: vec![1, 2, 3],
                        }),
                    },
                    Put {
                        id: note_id.clone(),
                        expected_version: Some(1),
                        archived: false,
                        data: {
                            let mut data = note.data;
                            if let RecordData::Note(ref mut n) = data {
                                n.markdown = source.clone();
                            }
                            data
                        },
                    },
                ],
            },
        )
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
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |view, cx| {
            view.open_note(&note_id, None, window, cx);
            view.set_note_reading(true, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, _window, cx| {
        view.update(cx, |view, cx| {
            let asset_key = format!("brn-asset://{asset_id}");
            assert!(
                view.writing_view().note_assets().contains_key(&asset_key),
                "the stored figure must be visible to the note's chat scope"
            );
            let rendered = view.writing.note_read_view.read(cx).rendered_text();
            assert_eq!(rendered.source(), source);
            assert_eq!(
                rendered.as_str(),
                "Intro with inline html.\n<div>block html</div>\nImage unavailable · gone\ngood and bad\n",
                "block HTML stays literal, a missing image names its alt text and \
                 link text renders; the default parser would strip the tags and \
                 drop the javascript: link text"
            );
        });
    })
    .unwrap();
}

/// Local editor history must not treat a checked Apply or saved refresh as an
/// owner edit that Cmd-Z can resurrect and Cmd-S can accidentally persist.
#[gpui_kit::test]
fn local_editor_undo_after_apply_and_refresh_cannot_resurrect_or_save_stale_text(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, "Original saved paragraph.");
    cx.update(|cx| cx.bind_keys(bindings()));
    open_note(cx, &f);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.writing.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(0..0, cx);
                editor.focus(window, cx);
            });
        });
    })
    .unwrap();
    cx.simulate_input(f.window, "Owner edit. ");
    cx.run_until_parked();
    f.view.read_with(cx, |view, cx| {
        assert!(
            view.writing
                .note_editor
                .read(cx)
                .value()
                .starts_with("Owner edit. ")
        );
        assert!(view.writing.note.as_ref().unwrap().session().is_some());
    });
    // Establish that this is the real editor's working local Undo path.
    cx.update_window(f.window, |_, window, cx| {
        window.press(
            if cfg!(target_os = "macos") {
                "cmd-z"
            } else {
                "ctrl-z"
            },
            cx,
        )
    })
    .unwrap();
    cx.run_until_parked();
    f.view.read_with(cx, |view, cx| {
        assert_eq!(
            view.writing.note_editor.read(cx).value().as_ref(),
            "Original saved paragraph."
        );
    });
    cx.simulate_input(f.window, "Saved owner edit. ");
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.press("cmd-s", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let note = view.workspace.store.note(&f.note.id).unwrap().unwrap();
            assert!(matches!(&note.data, RecordData::Note(note) if note.markdown.starts_with("Saved owner edit. ")));
            assert!(view.writing.note.as_ref().unwrap().session().is_none());
            let thread = view.workspace.new_thread("Local Undo witness").unwrap();
            view.workspace.link_writing_thread("local-undo-link", &note.id, note.version, &thread.id).unwrap();
            let snapshot = view.workspace.snapshot_writing_request("local-undo-request", &note.id, note.version, &thread.id, &[]).unwrap();
            let operation = view.workspace.store.allocate_operation("local-undo-proposal").unwrap();
            let candidate = view.workspace.store.prepare(&operation, &snapshot.change_request("AI final paragraph.").unwrap()).unwrap();
            view.open_review(candidate, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.click("apply-final", cx))
        .unwrap();
    cx.run_until_parked();

    for refreshed in [false, true] {
        if refreshed {
            cx.update_window(f.window, |_, window, cx| {
                f.view.update(cx, |view, cx| {
                    let current = view.workspace.store.note(&f.note.id).unwrap().unwrap();
                    let RecordData::Note(mut note) = current.data else {
                        panic!("fixture Note")
                    };
                    note.markdown = "New externally saved paragraph.".into();
                    let outcome = view
                        .workspace
                        .owner_change(
                            "local-undo-refresh",
                            &ChangeRequest {
                                reason: "Synthetic saved refresh witness".into(),
                                inputs: vec![],
                                writes: vec![Put {
                                    id: current.id,
                                    expected_version: Some(current.version),
                                    archived: current.archived,
                                    data: RecordData::Note(note),
                                }],
                            },
                        )
                        .unwrap();
                    assert!(matches!(outcome, ApplyOutcome::Applied(_)));
                    view.reload_open_note(window, cx);
                    cx.notify();
                });
            })
            .unwrap();
            cx.run_until_parked();
        }
        let expected = if refreshed {
            "New externally saved paragraph."
        } else {
            "AI final paragraph."
        };
        let (saved, receipts) = f.view.read_with(cx, |view, cx| {
            assert!(view.writing.review.is_none());
            assert!(view.writing.note.as_ref().unwrap().session().is_none());
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), expected);
            (
                view.workspace.store.note(&f.note.id).unwrap().unwrap(),
                view.history.len(),
            )
        });
        cx.update_window(f.window, |_, window, cx| {
            window.press(
                if cfg!(target_os = "macos") {
                    "cmd-z"
                } else {
                    "ctrl-z"
                },
                cx,
            )
        })
        .unwrap();
        cx.run_until_parked();
        f.view.read_with(cx, |view, cx| {
            assert_eq!(
                view.writing.note_editor.read(cx).value().as_ref(),
                expected,
                "local Undo must not restore text from before programmatic replacement"
            );
            assert!(
                view.writing.note.as_ref().unwrap().session().is_none(),
                "programmatic replacement and local Undo must not create a dirty buffer"
            );
            assert_eq!(
                view.workspace.store.note(&f.note.id).unwrap().unwrap(),
                saved
            );
        });
        cx.update_window(f.window, |_, window, cx| window.press("cmd-s", cx))
            .unwrap();
        cx.run_until_parked();
        f.view.read_with(cx, |view, cx| {
            assert_eq!(view.writing.note_editor.read(cx).value().as_ref(), expected);
            assert!(view.writing.note.as_ref().unwrap().session().is_none());
            assert_eq!(
                view.workspace.store.note(&f.note.id).unwrap().unwrap(),
                saved,
                "attempted Save must not persist stale text or create a revision"
            );
            assert_eq!(
                view.history.len(),
                receipts,
                "attempted Save after local Undo must not create a receipt"
            );
        });
    }
}

/// The reading-quality fixture (spec §6.5.1), repository copy.
const ARTICLE: &str =
    include_str!("../../../docs/product/ux-rebuild/fixtures/reading-quality-article.md");

/// The outline of the open document once its reading parse has landed.
fn outline_entries(
    cx: &mut TestAppContext,
    f: &super::polish_tests::Fixture,
) -> Vec<brn_writing_ui::outline::OutlineEntry> {
    super::polish_tests::on_window(cx, f, |window, cx| {
        window.refresh();
        let _ = cx;
    });
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        match view.writing.outline_state(cx) {
            brn_writing_ui::outline::OutlineState::Ready { entries, .. } => entries,
            brn_writing_ui::outline::OutlineState::Updating => panic!("outline still updating"),
        }
    })
}

/// Draw the next frame, so element queries see the latest state.
fn redraw(cx: &mut TestAppContext, f: &super::polish_tests::Fixture) {
    super::polish_tests::on_window(cx, f, |window, _| window.refresh());
}

fn read_mode(cx: &mut TestAppContext, f: &super::polish_tests::Fixture) {
    super::polish_tests::update(cx, f, |view, window, cx| {
        view.set_note_reading(true, window, cx);
        cx.notify();
    });
}

#[gpui_kit::test]
fn the_outline_lists_the_rendered_fixture_headings_with_distinct_targets(cx: &mut TestAppContext) {
    let f = super::polish_tests::fixture(cx, 1280., 900., ARTICLE);
    read_mode(cx, &f);
    let entries = outline_entries(cx, &f);
    let shown: Vec<(u8, &str)> = entries
        .iter()
        .map(|e| (e.level, e.label.as_str()))
        .collect();
    assert_eq!(
        shown,
        vec![
            (
                1,
                "Cedar Reading Room: a synthetic article for careful reading"
            ),
            (2, "The question"),
            (3, "Review"),
            (4, "Evidence boundary"),
            (2, "Walking through one session"),
            (
                3,
                "A deliberately long heading about preserving the reader's place while supporting material opens, closes and changes shape in a compact window"
            ),
            (3, "Märkmed ja ülevaade — Õ, ä, ö, ü and 東京"),
            (2, "Results from the invented rounds"),
            (3, "Review"),
            (2, "A deliberately skipped heading level"),
            (4, "Detail without an intermediate subheading"),
            (2, "Interpretation and next questions"),
            (2, "Source A"),
            (2, "Source B"),
        ],
        "every written heading, in order, with its level; the fenced fake headings are absent"
    );
    // The two Review headings are distinct targets in distinct blocks, and
    // each source range is exactly that heading's line.
    let reviews: Vec<_> = entries.iter().filter(|e| e.label == "Review").collect();
    assert_ne!(reviews[0].source, reviews[1].source);
    assert_ne!(reviews[0].block, reviews[1].block);
    for review in &reviews {
        assert_eq!(&ARTICLE[review.source.clone()], "### Review");
    }
    assert!(entries.iter().all(|e| !e.nested));
    assert!(!entries.iter().any(|e| e.label.contains("code example")));

    // The outline starts closed; with the assistant closed the toolbar's
    // Outline docks it at 1280 and the second Review reveals its own block.
    // (Beside the docked assistant the 734 document cannot keep 560 next to
    // it; that case is the contents view, below.)
    super::polish_tests::update(cx, &f, |view, window, cx| {
        view.relayout(window, cx, |layout| layout.set_support_open(false));
    });
    redraw(cx, &f);
    super::polish_tests::on_window(cx, &f, |window, cx| {
        assert!(
            window.try_find("note-outline-panel").is_none(),
            "closed on first open"
        );
        window.click("note-outline", cx);
    });
    redraw(cx, &f);
    let second = entries.iter().rposition(|e| e.label == "Review").unwrap();
    super::polish_tests::on_window(cx, &f, |window, cx| {
        assert!(window.find("note-outline-panel").visible());
        assert!(window.try_find("note-outline-view").is_none());
        window.click(format!("outline-entry-{second}"), cx);
    });
    redraw(cx, &f);
    cx.read(|cx| {
        let view = f.view.read(cx);
        let top = view
            .writing
            .note_read_view
            .read(cx)
            .list_state()
            .logical_scroll_top();
        assert_eq!(top.item_ix, entries[second].block);
        assert_eq!(view.writing.outline_current.get(), Some(second));
        assert!(
            view.writing.outline_preferred(),
            "the explicit choice is stored"
        );
    });
    super::polish_tests::on_window(cx, &f, |window, _| {
        let entry = window.find(format!("outline-entry-{second}"));
        assert!(
            entry
                .label()
                .is_some_and(|label| label.contains("current section")),
            "the current section is announced, not only colored"
        );
    });
}

#[gpui_kit::test]
fn outline_targets_are_bound_to_their_document_generation(cx: &mut TestAppContext) {
    let f = super::polish_tests::fixture(cx, 1280., 900., ARTICLE);
    read_mode(cx, &f);
    let before = outline_entries(cx, &f);
    let old = cx.read(|cx| {
        let state = f.view.read(cx).writing.outline_state(cx);
        let first_review = before.iter().position(|e| e.label == "Review").unwrap();
        state.target(first_review).unwrap()
    });
    // Rename the first Review and drop the skipped H4's marker in the
    // unsaved buffer: the outline refreshes from the displayed draft.
    super::polish_tests::update(cx, &f, |view, window, cx| {
        let text = ARTICLE
            .replacen("### Review\n", "### Review of the question\n", 1)
            .replace(
                "#### Detail without an intermediate subheading",
                "Detail without an intermediate subheading",
            );
        view.set_note_reading(false, window, cx);
        view.writing
            .note_editor
            .update(cx, |editor, cx| editor.set_value(text, window, cx));
        view.preserve_visible_note_input(cx);
        view.set_note_reading(true, window, cx);
    });
    let after = outline_entries(cx, &f);
    let labels: Vec<&str> = after.iter().map(|e| e.label.as_str()).collect();
    assert!(labels.contains(&"Review of the question"));
    assert_eq!(
        labels.iter().filter(|l| **l == "Review").count(),
        1,
        "the second Review stays"
    );
    assert!(!labels.contains(&"Detail without an intermediate subheading"));
    // The old target names the committed version's outline: refused, and
    // the reading position does not move.
    let top_before = cx.read(|cx| {
        f.view
            .read(cx)
            .writing
            .note_read_view
            .read(cx)
            .list_state()
            .logical_scroll_top()
            .item_ix
    });
    let refused = super::polish_tests::update(cx, &f, |view, window, cx| {
        view.writing.outline_go(&old, window, cx).is_err()
    });
    assert!(
        refused,
        "a target from an earlier generation never navigates"
    );
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.writing
                .note_read_view
                .read(cx)
                .list_state()
                .logical_scroll_top()
                .item_ix,
            top_before
        );
        // The draft is untouched by navigation.
        assert!(view.writing.note.as_ref().unwrap().session().is_some());
    });
    // A target of the refreshed outline works.
    let fresh = cx.read(|cx| {
        let state = f.view.read(cx).writing.outline_state(cx);
        let index = after.iter().position(|e| e.label == "Source B").unwrap();
        state.target(index).unwrap()
    });
    assert!(super::polish_tests::update(cx, &f, |view, window, cx| {
        view.writing.outline_go(&fresh, window, cx).is_ok()
    }));

    // Another document's outline never accepts this document's target.
    let other = super::polish_tests::update(cx, &f, |view, window, cx| {
        let other = view
            .workspace
            .new_note("Similar headings", "# Cedar\n\n## Source B\n")
            .unwrap();
        view.preserve_visible_note_input(cx);
        view.writing.read_next();
        view.open_note(&other.id, None, window, cx);
        other
    });
    assert_eq!(
        cx.read(|cx| f.view.read(cx).open_note_id().map(str::to_owned)),
        Some(other.id)
    );
    let _ = outline_entries(cx, &f);
    assert!(super::polish_tests::update(cx, &f, |view, window, cx| {
        view.writing.outline_go(&fresh, window, cx).is_err()
    }));
}

#[gpui_kit::test]
fn a_compact_window_uses_a_temporary_contents_view_that_keeps_the_preference(
    cx: &mut TestAppContext,
) {
    // 620 wide: the document region (564 beside the rail) cannot keep 560
    // beside a docked outline.
    let f = super::polish_tests::fixture(cx, 620., 600., ARTICLE);
    read_mode(cx, &f);
    let entries = outline_entries(cx, &f);
    super::polish_tests::on_window(cx, &f, |window, cx| window.click("note-outline", cx));
    redraw(cx, &f);
    super::polish_tests::on_window(cx, &f, |window, _| {
        assert!(
            window.find("note-outline-view").visible(),
            "temporary contents view"
        );
        assert!(
            window.try_find("note-outline-panel").is_none(),
            "no docked column at 900"
        );
    });
    cx.read(|cx| {
        assert!(
            !f.view.read(cx).writing.outline_preferred(),
            "fallback stores nothing"
        )
    });
    // Choosing a heading returns to the document at that section.
    let target = entries.iter().position(|e| e.label == "Source A").unwrap();
    super::polish_tests::on_window(cx, &f, |window, cx| {
        window.click(format!("outline-entry-{target}"), cx)
    });
    redraw(cx, &f);
    super::polish_tests::on_window(cx, &f, |window, _| {
        assert!(window.try_find("note-outline-view").is_none());
        assert!(window.find("note-body").visible());
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.writing
                .note_read_view
                .read(cx)
                .list_state()
                .logical_scroll_top()
                .item_ix,
            entries[target].block
        );
    });
    // Escape closes the view and returns focus to the Outline toggle.
    super::polish_tests::on_window(cx, &f, |window, cx| window.click("note-outline", cx));
    redraw(cx, &f);
    super::polish_tests::on_window(cx, &f, |window, cx| {
        assert!(window.find("note-outline-view").visible());
        window.press("escape", cx);
    });
    redraw(cx, &f);
    super::polish_tests::on_window(cx, &f, |window, cx| {
        assert!(window.try_find("note-outline-view").is_none());
        let view = f.view.read(cx);
        assert!(view.writing.outline_toggle_focus.is_focused(window));
    });
}

#[gpui_kit::test]
fn a_document_without_headings_says_no_headings_yet(cx: &mut TestAppContext) {
    let plain = "Plain prose without a heading.\n\n```markdown\n# Not a heading\n```\n";
    let f = super::polish_tests::fixture(cx, 1280., 900., plain);
    read_mode(cx, &f);
    assert!(outline_entries(cx, &f).is_empty());
    super::polish_tests::on_window(cx, &f, |window, cx| window.click("note-outline", cx));
    redraw(cx, &f);
    super::polish_tests::on_window(cx, &f, |window, _| {
        assert_eq!(
            window.find("note-outline-empty").label(),
            Some("No headings yet")
        );
    });
}

/// The launcher's Outline command (READ-01) docks the outline beside a wide
/// document (assistant closed) and records the owner's choice, like the toolbar toggle; a
/// second run hides it again.
#[gpui_kit::test]
fn the_launcher_outline_command_toggles_the_docked_outline(cx: &mut TestAppContext) {
    let f = super::polish_tests::fixture(cx, 1280., 900., ARTICLE);
    read_mode(cx, &f);
    let _ = outline_entries(cx, &f);
    super::polish_tests::update(cx, &f, |view, window, cx| {
        view.relayout(window, cx, |layout| layout.set_support_open(false));
    });
    redraw(cx, &f);
    let run = |cx: &mut TestAppContext| {
        cx.update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                view.run_command(lists::Command::Outline, window, cx)
            });
        })
        .unwrap();
        redraw(cx, &f);
    };
    run(cx);
    super::polish_tests::on_window(cx, &f, |window, _| {
        assert!(window.find("note-outline-panel").visible());
    });
    cx.read(|cx| assert!(f.view.read(cx).writing.outline_preferred()));
    run(cx);
    super::polish_tests::on_window(cx, &f, |window, _| {
        assert!(window.try_find("note-outline-panel").is_none());
    });
    cx.read(|cx| assert!(!f.view.read(cx).writing.outline_preferred()));
}
