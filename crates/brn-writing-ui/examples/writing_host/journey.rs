//! Real synthetic services and owner events; no native-human or live-provider claim.
use super::{fixture, host::WritingHost};
use brn_threads_app::*;
use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Entity, Focusable, TestAppContext, WindowBounds,
    WindowOptions, point, px, size,
};

fn host(cx: &mut TestAppContext) -> (AnyWindowHandle, Entity<WritingHost>) {
    let f = fixture::Fixture::new().unwrap();
    cx.update(gpui_kit::init);
    let result = cx.update(|cx| {
        cx.bind_keys(super::host::bindings());
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1260.), px(1100.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| WritingHost::new(f, window, cx)),
        )
        .unwrap()
    });
    cx.run_until_parked();
    result
}

#[gpui_kit::test]
fn same_panel_comments_shared_scripted_start_exact_review_apply_and_undo(cx: &mut TestAppContext) {
    let (window, view) = host(cx);
    let note = cx.read(|cx| view.read(cx).panel.open_note_id().unwrap().to_owned());
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let start = fixture::ORIGINAL.find("Hello,").unwrap();
            host.panel.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(start..start, cx);
                editor.focus(window, cx);
            });
        });
        for _ in 0.."Hello,".len() {
            window.press("shift-right", cx);
        }
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.click("selection-comment", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.input(
            "Make the explanation clearer; preserve this exact greeting.",
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.click("post-comment", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        let comments = host.workspace.writing_comments(&note).unwrap();
        assert_eq!(comments.len(), 1);
        let RecordData::Comment(comment) = &comments[0].record.data else {
            panic!("comment")
        };
        assert_eq!(comment.quote, "Hello,");
        assert_eq!(
            comment.body,
            "Make the explanation clearer; preserve this exact greeting."
        );
        assert!(host.workspace.candidates().unwrap().is_empty());
    });
    cx.update_window(window, |_, window, cx| {
        window.click("rewrite-note", cx);
    })
    .unwrap();
    cx.run_until_parked();
    let operation = cx.read(|cx| {
        let host = view.read(cx);
        let context = host.workspace.active_writing_context(&note, None).unwrap();
        let message = host
            .last_request
            .as_ref()
            .unwrap_or_else(|| panic!("shared start: {}", host.notice));
        let snapshot = host
            .workspace
            .writing_request_in_context(message, &note, None)
            .unwrap();
        assert_eq!(Some(snapshot.thread.clone()), context.conversation);
        assert_eq!(snapshot.comments.len(), 1);
        assert_eq!(
            snapshot.setup.as_ref().unwrap().brief.as_deref(),
            Some(fixture::BRIEF)
        );
        assert!(host.last_run.is_some());
        assert!(host.model_requests.contains(fixture::BRIEF));
        assert!(host.model_requests.contains("Make the explanation clearer"));
        let candidate = host.workspace.candidates().unwrap().pop().unwrap();
        let review = host
            .workspace
            .writing_candidate_review(candidate.operation.as_str(), None)
            .unwrap();
        assert!(!snapshot.setup.as_ref().unwrap().requirements.exhaustive);
        assert!(
            !review.complete_over_retained_text,
            "A normal Rewrite must not advertise an exhaustive requirements check"
        );
        assert_eq!(review.host_verified_quotations[0].quote, fixture::QUOTE);
        assert_eq!(review.requirements_reported.len(), 1);
        let source = review
            .sources
            .iter()
            .find(|source| source.id == host.reference.id)
            .unwrap();
        assert!(source.retained_read.as_ref().unwrap().complete);
        assert_eq!(
            source
                .import_coverage
                .as_ref()
                .unwrap()
                .report
                .as_ref()
                .unwrap()
                .status,
            ImportStatus::Partial
        );
        assert_eq!(
            host.workspace.store.note(&note).unwrap().unwrap().data,
            snapshot.note.data
        );
        candidate.operation.as_str().to_owned()
    });
    cx.update_window(window, |_, window, cx| {
        window.click(format!("writing-review-{operation}"), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        assert!(window.try_find("writing-sources-used").is_some());
        assert!(window.try_find("writing-source-quotation-0").is_some());
        window.click("apply-final", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        let RecordData::Note(note) = host.workspace.store.note(&note).unwrap().unwrap().data else {
            panic!("note")
        };
        assert_eq!(note.markdown, fixture::REVISED);
        assert_eq!(
            host.panel.note_editor.read(cx).value().as_ref(),
            fixture::REVISED
        );
        assert!(host.workspace.candidates().unwrap().is_empty());
    });
    cx.update_window(window, |_, window, cx| {
        window.click("undo-writing-apply", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        let RecordData::Note(note) = host.workspace.store.note(&note).unwrap().unwrap().data else {
            panic!("note")
        };
        assert_eq!(note.markdown, fixture::ORIGINAL);
        assert_eq!(
            host.panel.note_editor.read(cx).value().as_ref(),
            fixture::ORIGINAL
        );
        assert!(host.panel.note.as_ref().unwrap().session.is_none());
    });
}

#[gpui_kit::test]
fn host_wires_first_dirty_guard_save_and_fresh_access_loss_before_render(cx: &mut TestAppContext) {
    let (window, view) = host(cx);
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            host.panel
                .note_editor
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
        });
        window.press("cmd-a", cx);
        window.input("Owner typed a new synthetic document", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        let open = host.panel.note.as_ref().unwrap();
        let session = open
            .session
            .as_ref()
            .expect("first input must be synchronously guarded");
        assert_eq!(session.markdown, "Owner typed a new synthetic document");
        assert_eq!(
            host.workspace.store.recovery_buffers().unwrap()[0].markdown,
            session.markdown
        );
        let RecordData::Note(saved) = &open.record.data else {
            panic!("note")
        };
        assert_eq!(saved.markdown, fixture::ORIGINAL);
        assert!(host.workspace.candidates().unwrap().is_empty());
    });
    cx.update_window(window, |_, window, cx| {
        window.click("save-note", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, _, cx| {
        view.update(cx, |host, cx| {
            let saved = host.panel.note.as_ref().unwrap().record.clone();
            let RecordData::Note(note) = &saved.data else {
                panic!("note")
            };
            assert_eq!(note.markdown, "Owner typed a new synthetic document");
            assert!(host.workspace.store.recovery_buffers().unwrap().is_empty());
            host.workspace
                .owner_change(
                    "b4-access-loss",
                    &ChangeRequest {
                        reason: "Synthetic working Note archived".into(),
                        writes: vec![Put {
                            id: saved.id,
                            expected_version: Some(saved.version),
                            archived: true,
                            data: saved.data,
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, _| {
        assert!(window.try_find("host-unavailable").is_some());
        assert!(window.try_find("note-tile").is_none());
        assert!(window.try_find("writing-sources-used").is_none());
    })
    .unwrap();
}

#[test]
fn disposable_fixture_reopens_real_workspace_and_removes_only_its_own_root() {
    let fixture::Fixture {
        workspace,
        note,
        reference,
        root,
    } = fixture::Fixture::new().unwrap();
    let path = root.path().to_owned();
    let expected = workspace.active_writing_context(&note.id, None).unwrap();
    drop(workspace);
    let reopened = Workspace::open(path.join("workspace")).unwrap();
    assert_eq!(
        reopened.active_writing_context(&note.id, None).unwrap(),
        expected
    );
    let setup = reopened.writing_setup(&note.id, None).unwrap();
    assert_eq!(setup.references[0].reference.note, reference.id);
    assert_eq!(
        setup.record.unwrap().data,
        RecordData::WritingSetup(WritingSetup {
            note: note.id,
            chat: None,
            brief: Some(fixture::BRIEF.into()),
            references: vec![WritingReference {
                note: reference.id,
                purpose: WritingPurpose::Requirements
            }],
        })
    );
    drop(reopened);
    drop(root);
    assert!(!path.exists());
}

/// Native keyboard dispatch must find the host context through the focused editor.
#[gpui_kit::test]
fn editor_shortcuts_open_saved_passage_comment_save_and_rewrite(cx: &mut TestAppContext) {
    let (window, view) = host(cx);
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let start = fixture::ORIGINAL.find("Hello,").unwrap();
            host.panel.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(start..start, cx);
                editor.focus(window, cx);
            });
        });
        for _ in 0.."Hello,".len() {
            window.press("shift-right", cx);
        }
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.press("cmd-/", cx))
        .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        let (note, version, range, quote) = host
            .panel
            .comment_anchor
            .as_ref()
            .expect("cmd-/ from the editor must open the saved passage comment anchor");
        let saved = &host.panel.note.as_ref().unwrap().record;
        assert_eq!(note, &saved.id);
        assert_eq!(*version, saved.version);
        assert_eq!(quote, "Hello,");
        assert_eq!(&fixture::ORIGINAL[range.clone()], "Hello,");
        assert!(host.panel.comments_expanded);
        assert!(host.workspace.writing_comments(note).unwrap().is_empty());
    });
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            host.panel
                .note_editor
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx)
        });
        window.press("cmd-a", cx);
        window.input(fixture::ORIGINAL, cx);
        window.input("Owner shortcut edit.\n", cx);
    })
    .unwrap();
    cx.run_until_parked();
    let edited = cx.read(|cx| view.read(cx).panel.note_editor.read(cx).value().to_string());
    cx.read(|cx| assert!(view.read(cx).panel.note.as_ref().unwrap().session.is_some()));
    cx.update_window(window, |_, window, cx| window.press("cmd-s", cx))
        .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        assert!(
            host.panel.note.as_ref().unwrap().session.is_none(),
            "cmd-s must save the editor session"
        );
        assert!(host.workspace.store.recovery_buffers().unwrap().is_empty());
        let RecordData::Note(saved) = &host.panel.note.as_ref().unwrap().record.data else {
            panic!("note")
        };
        assert_eq!(saved.markdown, edited);
        assert!(host.last_run.is_none());
    });
    cx.update_window(window, |_, window, cx| window.press("cmd-shift-enter", cx))
        .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        assert!(
            host.last_run.is_some(),
            "cmd-shift-enter must reach the existing shared scripted start: {}",
            host.notice
        );
        assert_eq!(host.workspace.candidates().unwrap().len(), 1);
    });
}

/// macOS NativeMenu needs an NSView, unavailable in headless GPUI. Exercise its exact
/// selected-item actions via Window dispatch while the real shared editor owns focus;
/// this proves bubbling/handlers, not a native menu click or menu construction.
#[gpui_kit::test]
fn editor_menu_actions_bubble_to_comment_and_explicit_unavailable_ask(cx: &mut TestAppContext) {
    use brn_writing_ui::native::{AskAboutSelection, CommentSelection};
    let (window, view) = host(cx);
    let select = |window: &mut gpui_kit::Window, cx: &mut gpui_kit::App| {
        view.update(cx, |host, cx| {
            let start = fixture::ORIGINAL.find("Hello,").unwrap();
            host.panel.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(start..start, cx);
                editor.focus(window, cx);
            });
        });
        for _ in 0.."Hello,".len() {
            window.press("shift-right", cx);
        }
    };
    cx.update_window(window, |_, window, cx| select(window, cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.dispatch_action(Box::new(CommentSelection), cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        assert_eq!(
            host.panel
                .comment_anchor
                .as_ref()
                .expect("editor menu CommentSelection must reach the host")
                .3,
            "Hello,"
        );
        assert!(host.panel.comments_expanded);
        assert!(
            host.workspace
                .writing_comments(host.panel.open_note_id().unwrap())
                .unwrap()
                .is_empty()
        );
    });
    cx.update_window(window, |_, window, cx| select(window, cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        window.dispatch_action(Box::new(AskAboutSelection), cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        assert!(
            host.notice
                .starts_with("Unavailable in this development host."),
            "editor menu Ask must report its unsupported action: {}",
            host.notice
        );
        assert!(host.last_run.is_none());
        assert!(host.workspace.candidates().unwrap().is_empty());
        assert!(
            host.workspace
                .writing_comments(host.panel.open_note_id().unwrap())
                .unwrap()
                .is_empty()
        );
    });
}
