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
        assert!(host.panel.note.as_ref().unwrap().session().is_none());
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
        window.press(
            if cfg!(target_os = "macos") {
                "cmd-a"
            } else {
                "ctrl-a"
            },
            cx,
        );
        window.input("Owner typed a new synthetic document", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        let open = host.panel.note.as_ref().unwrap();
        let session = open
            .session()
            .expect("first input must be synchronously guarded");
        assert_eq!(session.markdown, "Owner typed a new synthetic document");
        assert_eq!(
            host.workspace.store.recovery_buffers().unwrap()[0].markdown,
            session.markdown
        );
        let RecordData::Note(saved) = &open.record().data else {
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
            let saved = host.panel.note.as_ref().unwrap().record().clone();
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
        let saved = host.panel.note.as_ref().unwrap().record();
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
        window.press(
            if cfg!(target_os = "macos") {
                "cmd-a"
            } else {
                "ctrl-a"
            },
            cx,
        );
        window.input(fixture::ORIGINAL, cx);
        window.input("Owner shortcut edit.\n", cx);
    })
    .unwrap();
    cx.run_until_parked();
    let edited = cx.read(|cx| view.read(cx).panel.note_editor.read(cx).value().to_string());
    cx.read(|cx| {
        assert!(
            view.read(cx)
                .panel
                .note
                .as_ref()
                .unwrap()
                .session()
                .is_some()
        )
    });
    cx.update_window(window, |_, window, cx| window.press("cmd-s", cx))
        .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        assert!(
            host.panel.note.as_ref().unwrap().session().is_none(),
            "cmd-s must save the editor session"
        );
        assert!(host.workspace.store.recovery_buffers().unwrap().is_empty());
        let RecordData::Note(saved) = &host.panel.note.as_ref().unwrap().record().data else {
            panic!("note")
        };
        assert_eq!(saved.markdown, edited);
        assert!(host.last_run.is_none());
    });
    cx.update_window(window, |_, window, cx| window.press("cmd-shift-r", cx))
        .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        assert!(
            host.last_run.is_some(),
            "cmd-shift-r must reach the existing shared scripted start: {}",
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

#[gpui_kit::test]
fn edited_review_survives_reopen_then_prepares_a_new_exact_candidate(cx: &mut TestAppContext) {
    use brn_writing_ui::native::WritingRequest;
    let (window, view) = host(cx);
    cx.update_window(window, |_, window, cx| window.click("rewrite-note", cx))
        .unwrap();
    cx.run_until_parked();
    let candidate = cx.read(|cx| view.read(cx).workspace.candidates().unwrap().pop().unwrap());
    cx.update_window(window, |_, window, cx| {
        window.click(
            format!("writing-review-{}", candidate.operation.as_str()),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            host.dispatch(WritingRequest::StartReviewEdit, window, cx);
            let length = host.panel.review_editor.read(cx).value().len();
            host.panel
                .review_editor
                .update(cx, |editor, cx| editor.set_selected_range(0..length, cx));
        });
        window.input("Owner's exact final text", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            assert!(host.panel.review.is_editing());
            assert!(
                host.panel.review.is_edited(),
                "real InputEvent reaches shared review owner"
            );
            host.dispatch(WritingRequest::OpenReview(candidate.clone()), window, cx);
            host.dispatch(WritingRequest::ReviewReading(true), window, cx);
            host.dispatch(WritingRequest::ApplyReview, window, cx);
            assert!(host.panel.review.is_edited());
            assert!(!host.panel.review.is_reading());
            assert_eq!(
                host.panel.review_editor.read(cx).value().as_ref(),
                "Owner's exact final text"
            );
            assert!(
                host.workspace
                    .store
                    .receipt(&candidate.operation)
                    .unwrap()
                    .is_none()
            );
            host.dispatch(WritingRequest::PrepareReviewEdit, window, cx);
            let replacement = host.panel.review.as_ref().expect("replacement displayed");
            assert_ne!(replacement.candidate.operation, candidate.operation);
            assert!(!host.panel.review.is_edited());
            assert!(!host.panel.review.is_editing());
            // A late completion for the retired candidate cannot close this review.
            host.panel.review.completed(candidate.operation.as_str());
            assert!(host.panel.review.is_some());
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.click("apply-final", cx))
        .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let host = view.read(cx);
        assert!(host.panel.review.is_none());
        assert!(!host.panel.review.is_editing() && !host.panel.review.is_reading());
        let record = host
            .workspace
            .store
            .note(host.panel.open_note_id().unwrap())
            .unwrap()
            .unwrap();
        let RecordData::Note(note) = record.data else {
            panic!("note")
        };
        assert_eq!(note.markdown, "Owner's exact final text");
    });
}

#[gpui_kit::test]
fn refused_review_apply_keeps_candidate_until_dirty_guard_is_resolved(cx: &mut TestAppContext) {
    use brn_writing_ui::native::WritingRequest;
    let (window, view) = host(cx);
    cx.update_window(window, |_, window, cx| window.click("rewrite-note", cx))
        .unwrap();
    cx.run_until_parked();
    let candidate = cx.read(|cx| view.read(cx).workspace.candidates().unwrap().pop().unwrap());
    cx.update_window(window, |_, window, cx| {
        window.click(
            format!("writing-review-{}", candidate.operation.as_str()),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let note = host.panel.note.as_ref().unwrap().record().clone();
            let session = host
                .workspace
                .store
                .begin_edit(&note.id, note.version)
                .unwrap();
            host.workspace
                .store
                .update_buffer(
                    &session.id,
                    session.generation + 1,
                    "Owner typing elsewhere",
                )
                .unwrap();
            host.dispatch(WritingRequest::ApplyReview, window, cx);
            assert_eq!(
                host.panel.review.as_ref().unwrap().candidate.operation,
                candidate.operation
            );
            assert!(
                host.workspace
                    .store
                    .receipt(&candidate.operation)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(host.workspace.store.note(&note.id).unwrap().unwrap(), note);
            let guarded = host
                .workspace
                .store
                .recovery_buffers()
                .unwrap()
                .pop()
                .unwrap();
            host.workspace
                .store
                .discard(&guarded.id, guarded.generation)
                .unwrap();
            host.dispatch(WritingRequest::ApplyReview, window, cx);
            assert!(host.panel.review.is_none());
            assert!(
                host.workspace
                    .store
                    .receipt(&candidate.operation)
                    .unwrap()
                    .is_some()
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn refused_preparation_retains_real_review_input_and_original_candidate(cx: &mut TestAppContext) {
    use brn_writing_ui::native::WritingRequest;
    let (window, view) = host(cx);
    cx.update_window(window, |_, window, cx| window.click("rewrite-note", cx))
        .unwrap();
    cx.run_until_parked();
    let candidate = cx.read(|cx| view.read(cx).workspace.candidates().unwrap().pop().unwrap());
    cx.update_window(window, |_, window, cx| {
        window.click(
            format!("writing-review-{}", candidate.operation.as_str()),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            host.dispatch(WritingRequest::StartReviewEdit, window, cx);
            let length = host.panel.review_editor.read(cx).value().len();
            host.panel
                .review_editor
                .update(cx, |editor, cx| editor.set_selected_range(0..length, cx));
        });
        window.input("Keep this draft when preparation fails", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            assert!(host.panel.review.is_edited());
            // Another checked caller retired the candidate before preparation.
            host.workspace
                .review_dismiss(candidate.operation.as_str())
                .unwrap();
            host.dispatch(WritingRequest::PrepareReviewEdit, window, cx);
            assert!(
                host.notice.contains("only active unapplied candidates"),
                "{}",
                host.notice
            );
            assert_eq!(
                host.panel.review.as_ref().unwrap().candidate.operation,
                candidate.operation
            );
            assert!(host.panel.review.is_editing() && host.panel.review.is_edited());
            assert_eq!(
                host.panel.review_editor.read(cx).value().as_ref(),
                "Keep this draft when preparation fails"
            );
            // Keep-for-later also refuses to discard the failed draft.
            host.dispatch(WritingRequest::KeepReview, window, cx);
            assert!(host.panel.review.is_edited());
            host.dispatch(WritingRequest::CancelReviewEdit, window, cx);
            assert!(!host.panel.review.is_edited());
            assert_eq!(
                host.panel.review_editor.read(cx).value().as_ref(),
                fixture::REVISED
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn note_stale_save_refresh_recovery_and_discard_preserve_the_typed_base(cx: &mut TestAppContext) {
    let (window, view) = host(cx);
    let note = cx.read(|cx| view.read(cx).panel.note.as_ref().unwrap().record().clone());
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let RecordData::Note(mut changed) = note.data.clone() else {
                panic!("note")
            };
            changed.markdown = "A newer saved version".into();
            host.workspace
                .owner_change(
                    "note-lifecycle-newer",
                    &ChangeRequest {
                        reason: "Checked external update before owner typing".into(),
                        writes: vec![Put {
                            id: note.id.clone(),
                            expected_version: Some(note.version),
                            archived: false,
                            data: RecordData::Note(changed),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            let length = host.panel.note_editor.read(cx).value().len();
            host.panel.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(0..length, cx);
                editor.focus(window, cx);
            });
        });
        window.input("Owner's stale-base text", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| window.click("save-note", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let recovery = host.panel.note.as_ref().unwrap().session().unwrap().clone();
            assert_eq!(recovery.base_version, note.version);
            assert_eq!(recovery.markdown, "Owner's stale-base text");
            assert!(host.notice.contains("Stale"), "{}", host.notice);
            host.panel
                .refresh_note(&mut host.workspace, window, cx)
                .unwrap();
            assert_eq!(
                host.panel.note.as_ref().unwrap().record().version,
                note.version
            );
            assert!(
                host.panel
                    .open_note(&mut host.workspace, "missing-note", None, window, cx)
                    .is_err()
            );
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Owner's stale-base text"
            );
            // Explicit recovery reopens the durable buffer, preserving its old
            // base even while the saved-current projection is now newer.
            host.panel
                .open_note(
                    &mut host.workspace,
                    &note.id,
                    Some(recovery.clone()),
                    window,
                    cx,
                )
                .unwrap();
            assert_eq!(
                host.panel.note.as_ref().unwrap().record().version,
                note.version + 1
            );
            assert_eq!(
                host.panel
                    .note
                    .as_ref()
                    .unwrap()
                    .session()
                    .unwrap()
                    .base_version,
                note.version
            );
            let outcome = host
                .panel
                .save_note(&mut host.workspace, window, cx)
                .unwrap()
                .unwrap();
            assert!(matches!(outcome, SaveOutcome::Stale(_)));
            assert_eq!(
                host.workspace.store.recovery_buffers().unwrap()[0].markdown,
                "Owner's stale-base text"
            );
            assert!(matches!(
                host.panel
                    .discard_note_session(&mut host.workspace, &recovery, window, cx)
                    .unwrap(),
                brn_writing_ui::native::NoteDiscard::Own
            ));
            assert!(host.panel.note.as_ref().unwrap().session().is_none());
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "A newer saved version"
            );
            let BufferOutcome::Ignored(closed) = host
                .workspace
                .store
                .update_buffer(
                    &recovery.id,
                    recovery.generation + 100,
                    "Late old buffer must not revive the guard",
                )
                .unwrap()
            else {
                panic!("closed session update must be ignored")
            };
            assert!(closed.closed);
            assert!(host.workspace.store.recovery_buffers().unwrap().is_empty());
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "A newer saved version"
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn note_refresh_does_not_adopt_another_edit_and_confirmed_discard_keeps_owner_typing(
    cx: &mut TestAppContext,
) {
    let (window, view) = host(cx);
    let note = cx.read(|cx| view.read(cx).panel.note.as_ref().unwrap().record().clone());
    let mut other_session = None;
    // Install a caller's guard after the clean surface has rendered; the input
    // races that guard through the real subscription and checked writer.
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let session = host
                .workspace
                .store
                .begin_edit(&note.id, note.version)
                .unwrap();
            let BufferOutcome::Updated(session) = host
                .workspace
                .store
                .update_buffer(&session.id, session.generation + 1, "Other caller's edit")
                .unwrap()
            else {
                panic!("updated")
            };
            other_session = Some(session);
            host.panel
                .refresh_note(&mut host.workspace, window, cx)
                .unwrap();
            assert!(
                host.panel.note.as_ref().unwrap().session().is_none(),
                "automatic refresh cannot adopt another caller's edit"
            );
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                fixture::ORIGINAL
            );
            let length = host.panel.note_editor.read(cx).value().len();
            host.panel.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(0..length, cx);
                editor.focus(window, cx);
                // TestWindowExt::input redraws first. Use GPUI's real input
                // replacement before that redraw to exercise the raced key.
                editor.replace("Owner text must survive the blocked edit", window, cx);
            });
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Owner text must survive the blocked edit",
                "the real input must reach the editor before owner-state assertions"
            );
            assert_eq!(
                host.panel.note.open_edit_elsewhere(),
                Some((note.id.as_str(), note.version))
            );
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Owner text must survive the blocked edit"
            );
            let result = host
                .panel
                .discard_note_session(
                    &mut host.workspace,
                    other_session.as_ref().unwrap(),
                    window,
                    cx,
                )
                .unwrap();
            assert!(matches!(
                result,
                brn_writing_ui::native::NoteDiscard::Other { adopted: true }
            ));
            assert!(host.panel.note.open_edit_elsewhere().is_none());
            assert_eq!(
                host.panel
                    .note
                    .as_ref()
                    .unwrap()
                    .session()
                    .unwrap()
                    .markdown,
                "Owner text must survive the blocked edit"
            );
            assert!(matches!(
                host.panel
                    .save_note(&mut host.workspace, window, cx)
                    .unwrap(),
                Some(SaveOutcome::Saved(_))
            ));
            assert!(host.workspace.store.recovery_buffers().unwrap().is_empty());
            let RecordData::Note(saved) =
                host.workspace.store.note(&note.id).unwrap().unwrap().data
            else {
                panic!("note")
            };
            assert_eq!(saved.markdown, "Owner text must survive the blocked edit");
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn note_newer_buffer_generation_refuses_save_and_discard_until_explicit_recovery(
    cx: &mut TestAppContext,
) {
    let (window, view) = host(cx);
    let other_note = cx
        .update_window(window, |_, _, cx| {
            view.update(cx, |host, _| {
                host.workspace
                    .new_note("Another synthetic document", "Other saved text")
                    .unwrap()
            })
        })
        .unwrap();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let length = host.panel.note_editor.read(cx).value().len();
            host.panel.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(0..length, cx);
                editor.focus(window, cx);
            });
        });
        window.input("Local first generation", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let session = host.panel.note.as_ref().unwrap().session().unwrap().clone();
            host.workspace
                .store
                .update_buffer(
                    &session.id,
                    session.generation + 2,
                    "A newer durable generation",
                )
                .unwrap();
            let saved_before = host.workspace.store.note(&session.note).unwrap().unwrap();
            let history_before = host.workspace.history().unwrap();
            assert!(matches!(
                host.panel
                    .save_note(&mut host.workspace, window, cx)
                    .unwrap(),
                Some(SaveOutcome::GenerationMismatch(_))
            ));
            assert_eq!(
                host.workspace.store.note(&session.note).unwrap().unwrap(),
                saved_before
            );
            assert_eq!(
                host.workspace.history().unwrap(),
                history_before,
                "refused Save must produce no receipt"
            );
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Local first generation"
            );
            assert!(matches!(
                host.panel
                    .discard_note_session(&mut host.workspace, &session, window, cx)
                    .unwrap(),
                brn_writing_ui::native::NoteDiscard::Newer
            ));
            assert_eq!(
                host.workspace.store.recovery_buffers().unwrap()[0].markdown,
                "A newer durable generation"
            );
            let local_record = host.panel.note.as_ref().unwrap().record().clone();
            let newer_buffer = host.workspace.store.recovery_buffers().unwrap();

            // A reload or navigation must not silently replace the locally
            // visible generation just because it still equals the stale local
            // session. Only explicit same-note Recover may adopt the newer
            // durable generation.
            assert!(host
                .panel
                .open_note(
                    &mut host.workspace,
                    &session.note,
                    None,
                    window,
                    cx,
                )
                .is_err());
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Local first generation"
            );
            assert_eq!(
                host.panel.note.as_ref().unwrap().session().unwrap(),
                &session
            );
            assert_eq!(
                host.panel.note.as_ref().unwrap().record(),
                &local_record
            );
            assert_eq!(
                host.workspace.store.recovery_buffers().unwrap(),
                newer_buffer
            );
            assert_eq!(
                host.workspace.store.note(&session.note).unwrap().unwrap(),
                saved_before
            );
            assert_eq!(host.workspace.history().unwrap(), history_before);

            assert!(host
                .panel
                .open_note(
                    &mut host.workspace,
                    &other_note.id,
                    None,
                    window,
                    cx,
                )
                .is_err());
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Local first generation"
            );
            assert_eq!(
                host.panel.note.as_ref().unwrap().session().unwrap(),
                &session
            );
            assert_eq!(
                host.panel.note.as_ref().unwrap().record(),
                &local_record
            );
            assert_eq!(
                host.workspace.store.recovery_buffers().unwrap(),
                newer_buffer
            );
            assert_eq!(
                host.workspace.store.note(&session.note).unwrap().unwrap(),
                saved_before
            );
            assert_eq!(host.workspace.history().unwrap(), history_before);

            host.panel
                .open_note(
                    &mut host.workspace,
                    &session.note,
                    Some(session.clone()),
                    window,
                    cx,
                )
                .unwrap();
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "A newer durable generation"
            );
            assert_eq!(
                host.panel
                    .note
                    .as_ref()
                    .unwrap()
                    .session()
                    .unwrap()
                    .generation,
                session.generation + 2
            );
            // Once the owner explicitly adopts the fresh recovery, ordinary
            // navigation can proceed and leaves that durable session intact.
            host.panel
                .open_note(&mut host.workspace, &other_note.id, None, window, cx)
                .unwrap();
            assert_eq!(host.panel.open_note_id(), Some(other_note.id.as_str()));
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Other saved text"
            );
            assert_eq!(
                host.workspace.store.recovery_buffers().unwrap(),
                newer_buffer
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn note_explicit_recovery_replaces_a_blocked_attempt_only_after_validating_the_hint(
    cx: &mut TestAppContext,
) {
    let (window, view) = host(cx);
    let note = cx.read(|cx| view.read(cx).panel.note.as_ref().unwrap().record().clone());
    let mut kept = None;
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            let session = host
                .workspace
                .store
                .begin_edit(&note.id, note.version)
                .unwrap();
            let BufferOutcome::Updated(session) = host
                .workspace
                .store
                .update_buffer(
                    &session.id,
                    session.generation + 1,
                    "The kept authoritative edit",
                )
                .unwrap()
            else {
                panic!("updated")
            };
            kept = Some(session);
            let length = host.panel.note_editor.read(cx).value().len();
            host.panel.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(0..length, cx);
                editor.focus(window, cx);
                // Exercise the real GPUI Change before the guard redraw.
                editor.replace("Blocked local attempt", window, cx);
            });
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(window, |_, window, cx| {
        view.update(cx, |host, cx| {
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Blocked local attempt",
                "the real input must reach the editor before owner-state assertions"
            );
            assert!(host.panel.note.open_edit_elsewhere().is_some());
            host.panel
                .set_note_reading(&mut host.workspace, true, cx)
                .unwrap();
            assert!(host.panel.note.is_reading());
            host.panel
                .refresh_note(&mut host.workspace, window, cx)
                .unwrap();
            host.panel
                .set_note_reading(&mut host.workspace, false, cx)
                .unwrap();
            assert!(!host.panel.note.is_reading());
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Blocked local attempt"
            );
            assert_eq!(
                host.workspace.store.recovery_buffers().unwrap()[0],
                kept.clone().unwrap()
            );
            let mut wrong = kept.clone().unwrap();
            wrong.id = "missing-session".into();
            assert!(
                host.panel
                    .open_note(&mut host.workspace, &note.id, Some(wrong), window, cx)
                    .is_err()
            );
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "Blocked local attempt"
            );
            host.panel
                .open_note(&mut host.workspace, &note.id, kept.clone(), window, cx)
                .unwrap();
            assert!(host.panel.note.open_edit_elsewhere().is_none());
            assert_eq!(
                host.panel.note.as_ref().unwrap().session().unwrap(),
                kept.as_ref().unwrap()
            );
            assert_eq!(
                host.panel.note_editor.read(cx).value().as_ref(),
                "The kept authoritative edit"
            );
            assert_eq!(
                host.workspace.store.recovery_buffers().unwrap(),
                vec![kept.clone().unwrap()]
            );
        });
    })
    .unwrap();
}
