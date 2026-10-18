//! Offline native writing flow; real Mac selection and pixels remain acceptance work.
use super::*;
use gpui_kit::TestAppContext;

#[gpui_kit::test]
fn adding_passage_feedback_keeps_the_document_and_does_not_start_ai(cx: &mut TestAppContext) {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("writing");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let note = workspace.new_note("Draft", "tere tere õun").unwrap();
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
            view.open_note(&note.id, None, window, cx);
            view.writing.comment_anchor =
                Some((note.id.clone(), note.version, 5..9, "tere".into()));
            view.writing.comment_input.update(cx, |editor, cx| {
                editor.set_value("Clarify second greeting", window, cx)
            });
            view.post_comment(window, cx);
            assert_eq!(view.open_note_id(), Some(note.id.as_str()));
            assert!(view.worker.is_none());
            let current = view.workspace.store.note(&note.id).unwrap().unwrap();
            assert_eq!(current, note);
        });
    })
    .unwrap();
}

use gpui_kit::{Focusable, test::TestWindowExt};
struct Fixture {
    _root: tempfile::TempDir,
    data: PathBuf,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
    a: Record,
    b: Record,
    chat_a: String,
    chat_b: String,
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    fixture_size(cx, 1260., 1100.)
}
fn fixture_size(cx: &mut TestAppContext, width: f32, height: f32) -> Fixture {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("writing-native");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let a = workspace.new_note("Note A", "tere tere õun").unwrap();
    let b = workspace
        .new_note("Note B", "Second saved paragraph.")
        .unwrap();
    let chat_a = workspace.new_thread("Chat A").unwrap().id;
    let chat_b = workspace.new_thread("Chat B").unwrap().id;
    workspace
        .link_writing_thread("link-a", &a.id, a.version, &chat_a)
        .unwrap();
    workspace
        .link_writing_thread("link-a-b", &a.id, a.version, &chat_b)
        .unwrap();
    workspace
        .link_writing_thread("link-b", &b.id, b.version, &chat_b)
        .unwrap();
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
    Fixture {
        _root: root,
        data,
        window,
        view,
        a,
        b,
        chat_a,
        chat_b,
    }
}
fn feedback(
    view: &mut ThreadsUi,
    note: &Record,
    quote: &str,
    range: std::ops::Range<usize>,
    body: &str,
    window: &mut Window,
    cx: &mut Context<ThreadsUi>,
) {
    view.writing.comment_anchor = Some((note.id.clone(), note.version, range, quote.into()));
    view.writing
        .comment_input
        .update(cx, |editor, cx| editor.set_value(body, window, cx));
    view.post_comment(window, cx);
}
#[gpui_kit::test]
fn keyboard_selection_shows_strip_and_comment_key_anchors_saved_passage(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update(|cx| cx.bind_keys(bindings()));
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        window.press(super::focus_tests::LINE_START, cx);
        window.press("shift-right", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            assert_eq!(
                view.writing.selection.as_ref().map(|s| s.1.as_str()),
                Some("t")
            );
            assert!(
                view.writing.comment_anchor.is_none(),
                "selecting never starts a comment"
            );
            assert!(
                view.writing
                    .note_editor
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window)
            );
        });
        assert!(window.find("selection-toolbar").visible());
        assert_eq!(window.find("selection-comment").disabled(), None);
        window.press("shift-right", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.press("cmd-/", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            assert_eq!(
                view.writing
                    .comment_anchor
                    .as_ref()
                    .map(|anchor| anchor.3.as_str()),
                Some("te")
            );
            assert_eq!(view.writing.comment_anchor.as_ref().unwrap().2, 0..2);
            assert!(
                view.writing
                    .comment_input
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window),
                "the comment composer opens at the margin"
            );
            assert!(view.worker.is_none());
        });
        assert!(window.find("comment-composer").visible());
        assert!(window.try_find("post-comment").is_some());
        assert!(window.try_find("thread-conversation").is_some());
    })
    .unwrap();
}
#[gpui_kit::test]
fn note_and_selected_chat_drafts_and_dirty_buffers_survive_navigation(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            view.composer.update(cx, |editor, cx| {
                editor.set_value("Chat A draft", window, cx)
            });
            view.writing.comment_anchor = Some((f.a.id.clone(), f.a.version, 5..9, "tere".into()));
            view.writing.comment_input.update(cx, |editor, cx| {
                editor.set_value("A passage draft", window, cx)
            });
            view.select_writing_thread(f.a.id.clone(), f.chat_b.clone(), window, cx);
            assert!(view.composer.read(cx).value().is_empty());
            view.composer.update(cx, |editor, cx| {
                editor.set_value("Chat B draft", window, cx)
            });
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Chat A draft");
            view.writing.note_editor.update(cx, |editor, cx| {
                editor.set_value("Human unsaved õun", window, cx)
            });
            view.preserve_visible_note_input(cx);
            view.open_note(&f.b.id, None, window, cx);
            assert!(view.writing.comment_anchor.is_none());
            assert!(view.writing.comment_input.read(cx).value().is_empty());
            assert_eq!(view.composer.read(cx).value().as_ref(), "Chat A draft");
            view.writing
                .comment_input
                .update(cx, |editor, cx| editor.set_value("B draft", window, cx));
            view.open_note(&f.a.id, None, window, cx);
            assert_eq!(
                view.writing.note_editor.read(cx).value().as_ref(),
                "Human unsaved õun"
            );
            assert!(view.writing.note.as_ref().unwrap().session().is_some());
            assert_eq!(view.writing.comment_anchor.as_ref().unwrap().2, 5..9);
            assert_eq!(
                view.writing.comment_input.read(cx).value().as_ref(),
                "A passage draft"
            );
            assert_eq!(view.composer.read(cx).value().as_ref(), "Chat A draft");
            view.post_comment(window, cx);
            assert!(view.workspace.writing_comments(&f.a.id).unwrap().is_empty());
            assert_eq!(
                view.writing.comment_input.read(cx).value().as_ref(),
                "A passage draft"
            );
            assert!(view.notice.contains("Save your edit"));
        })
    })
    .unwrap();
}
#[gpui_kit::test]
fn batch_rewrite_captures_selected_chat_and_retry_keeps_saved_inputs(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut request = String::new();
    cx.update_window(f.window,|_,window,cx| f.view.update(cx,|view,cx| {
        view.open_note(&f.a.id,None,window,cx);
        view.select_writing_thread(f.a.id.clone(),f.chat_a.clone(),window,cx);
        view.workspace.post_message(&f.chat_a,"Captured broad direction").unwrap();
        view.workspace.post_message(&f.chat_b,"Other chat must stay outside").unwrap();
        feedback(view,&f.a,"tere",5..9,"Clarify second greeting",window,cx);
        feedback(view,&f.a,"õun",10..14,"Keep Estonian fruit",window,cx);
        let comments=view.workspace.writing_comments(&f.a.id).unwrap();
        assert_eq!(comments.len(),2);
        assert_eq!(view.workspace.writing_threads(&f.a.id).unwrap().len(),2);
        view.writing.excluded_comments.insert(comments[1].record.id.clone());
        view.composer.update(cx,|editor,cx|editor.set_value("Unsent must stay unsent",window,cx));
        view.rewrite_note(cx);
        assert!(view.worker.is_none());
        assert!(view.notice.contains("AI has not started"));
        assert_eq!(view.open_note_id(),Some(f.a.id.as_str()));
        assert_eq!(view.composer.read(cx).value().as_ref(),"Unsent must stay unsent");
        request=view.workspace.messages(&f.chat_a).unwrap().into_iter().find(|record|matches!(&record.data,RecordData::Message(body) if body.role=="writing-request")).unwrap().id;
        let snapshot=view.workspace.writing_request(&request).unwrap();
        assert_eq!(snapshot.comments.len(),1);
        assert_eq!(snapshot.messages.len(),1);
        assert!(matches!(&snapshot.messages[0].data,RecordData::Message(body) if body.text=="Captured broad direction"));
        view.workspace.post_message(&f.chat_a,"Later addition").unwrap();
        let before=view.workspace.messages(&f.chat_a).unwrap().len();
        view.retry_rewrite(request.clone(),cx);
        assert_eq!(view.workspace.messages(&f.chat_a).unwrap().len(),before);
        assert_eq!(view.workspace.writing_request(&request).unwrap(),snapshot);
        view.refresh();
    })).unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("retry-rewrite").is_some());
        window.click("inspect-rewrite-inputs", cx);
    })
    .unwrap();
    cx.run_until_parked();
    // Restart retains latest saved request and its explicit rewrite mode without duplicating it.
    let reopened = Workspace::open(&f.data).unwrap();
    let fresh = cx
        .update_window(f.window, |_, window, cx| {
            cx.new(|cx| {
                let mut view = ThreadsUi::new(reopened, f.data.clone(), window, cx);
                view.poll = None;
                view.open_note(&f.a.id, None, window, cx);
                view.retry_rewrite(request.clone(), cx);
                view
            })
        })
        .unwrap();
    cx.read(|cx| {
        assert_eq!(fresh.read(cx).writing_threads.get(&f.a.id), Some(&f.chat_a));
        assert!(fresh.read(cx).worker.is_none());
    });
}
#[gpui_kit::test]
fn stale_comment_and_saved_rewrite_preserve_feedback_with_actionable_recovery(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window,|_,window,cx| f.view.update(cx,|view,cx| {
        view.open_note(&f.a.id,None,window,cx);
        view.select_writing_thread(f.a.id.clone(),f.chat_a.clone(),window,cx);
        view.rewrite_note(cx);
        let request=view.workspace.messages(&f.chat_a).unwrap().into_iter().find(|record|matches!(&record.data,RecordData::Message(body) if body.role=="writing-request")).unwrap().id;
        let edit=view.workspace.store.begin_edit(&f.a.id,f.a.version).unwrap();
        view.workspace.store.update_buffer(&edit.id,1,"Changed saved note").unwrap();
        assert!(matches!(view.workspace.store.save(&edit.id,1).unwrap(),SaveOutcome::Saved(_)));
        feedback(view,&f.a,"tere",5..9,"Retain stale passage",window,cx);
        assert_eq!(view.writing.comment_input.read(cx).value().as_ref(),"Retain stale passage");
        assert!(view.writing.comment_anchor.is_some());
        assert!(view.workspace.writing_comments(&f.a.id).unwrap().is_empty());
        view.retry_rewrite(request,cx);
        assert!(view.notice.contains("Reload saved note"));
        assert!(view.worker.is_none());
        assert_eq!(view.writing.comment_input.read(cx).value().as_ref(),"Retain stale passage");
    })).unwrap();
}

fn proposal(
    view: &mut ThreadsUi,
    note: &Record,
    thread: &str,
    ids: &[String],
    text: &str,
    key: &str,
) -> (WritingSnapshot, Prepared) {
    let snapshot = view
        .workspace
        .snapshot_writing_request(
            &format!("request-{key}"),
            &note.id,
            note.version,
            thread,
            ids,
        )
        .unwrap();
    let operation = view
        .workspace
        .store
        .allocate_operation(&format!("proposal-{key}"))
        .unwrap();
    let candidate = view
        .workspace
        .store
        .prepare(&operation, &snapshot.change_request(text).unwrap())
        .unwrap();
    (snapshot, candidate)
}
#[gpui_kit::test]
fn private_writing_disclosure_is_mandatory_and_wrong_displayed_scope_hides_review_and_compare(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (project, other, reference, candidate, private_request) = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                // Substantial unrelated writing history must not be canonically
                // revalidated on this Note's typing path.
                for index in 0..100 {
                    let unrelated = view
                        .workspace
                        .new_note(&format!("Unrelated {index}"), "Unrelated saved body")
                        .unwrap();
                    view.workspace
                        .link_writing_thread(
                            &format!("unrelated-link-{index}"),
                            &unrelated.id,
                            unrelated.version,
                            &f.chat_b,
                        )
                        .unwrap();
                    view.workspace
                        .snapshot_writing_request(
                            &format!("unrelated-request-{index}"),
                            &unrelated.id,
                            unrelated.version,
                            &f.chat_b,
                            &[],
                        )
                        .unwrap();
                }
                let mut projects = Vec::new();
                for key in ["native-P1", "native-P2"] {
                    let id = view
                        .workspace
                        .project_command(key, ProjectCommand::Create { title: key.into() })
                        .unwrap()
                        .writes[0]
                        .after
                        .id
                        .clone();
                    view.workspace
                        .project_command(
                            &format!("{key}-link"),
                            ProjectCommand::LinkDocument {
                                project: id.clone(),
                                note: f.a.id.clone(),
                            },
                        )
                        .unwrap();
                    projects.push(id);
                }
                let other_context = view
                    .workspace
                    .ensure_writing_context("native-other-writing", &f.a.id, Some(&projects[1]))
                    .unwrap();
                view.workspace
                    .snapshot_writing_request(
                        "native-other-request",
                        &f.a.id,
                        1,
                        other_context.conversation.as_deref().unwrap(),
                        &[],
                    )
                    .unwrap();
                let reference = view
                    .workspace
                    .new_note("PRIVATE NAMED REQUIREMENTS", "PRIVATE EXACT SOURCE QUOTE")
                    .unwrap();
                let RecordData::Note(mut data) = reference.data.clone() else {
                    unreachable!()
                };
                data.chat = Some(projects[0].clone());
                assert!(matches!(
                    view.workspace
                        .owner_change(
                            "private-native-reference",
                            &ChangeRequest {
                                reason: "Synthetic private evidence".into(),
                                writes: vec![Put {
                                    id: reference.id.clone(),
                                    expected_version: Some(1),
                                    archived: false,
                                    data: RecordData::Note(data)
                                }],
                                inputs: vec![]
                            }
                        )
                        .unwrap(),
                    ApplyOutcome::Applied(_)
                ));
                view.workspace
                    .project_command(
                        "native-private-link",
                        ProjectCommand::LinkDocument {
                            project: projects[0].clone(),
                            note: reference.id.clone(),
                        },
                    )
                    .unwrap();
                let context = view
                    .workspace
                    .ensure_writing_context("native-writing", &f.a.id, Some(&projects[0]))
                    .unwrap();
                let thread = context.conversation.unwrap();
                view.workspace
                    .set_writing_setup(
                        "native-setup",
                        None,
                        &WritingSetup {
                            note: f.a.id.clone(),
                            chat: Some(projects[0].clone()),
                            brief: Some("PRIVATE BRIEF".into()),
                            references: vec![WritingReference {
                                note: reference.id.clone(),
                                purpose: WritingPurpose::Requirements,
                            }],
                        },
                    )
                    .unwrap();
                let snapshot = view
                    .workspace
                    .snapshot_writing_request_with_feedback(
                        "native-review-request",
                        &f.a.id,
                        1,
                        &thread,
                        &[],
                        "PRIVATE REQUEST FEEDBACK",
                    )
                    .unwrap();
                let op = view
                    .workspace
                    .store
                    .allocate_operation("native-evidence-candidate")
                    .unwrap();
                let mut request = snapshot.change_request("PRIVATE GENERATED QUOTE").unwrap();
                request.inputs.push(RevisionInput {
                    record: reference.id.clone(),
                    version: 2,
                    expectation: InputExpectation::Historical,
                });
                let quote = "PRIVATE EXACT SOURCE QUOTE";
                let private_request = snapshot.request_message.clone();
                let proof = WritingCandidateEvidence {
                    version: 1,
                    request_message: snapshot.request_message,
                    thread: thread.clone(),
                    chat: Some(projects[0].clone()),
                    reads: vec![WritingRead {
                        id: reference.id.clone(),
                        version: 2,
                        covered_retained_bytes: quote.len(),
                        total_retained_bytes: quote.len(),
                        complete: true,
                    }],
                    quotations: vec![WritingQuotation {
                        id: reference.id.clone(),
                        version: 2,
                        start: 0,
                        end: quote.len(),
                        quote: quote.into(),
                    }],
                    requirements_reported: vec![],
                    explanation: "Synthetic host explanation".into(),
                };
                request.writes.push(Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread,
                        role: WRITING_EVIDENCE_ROLE.into(),
                        text: serde_json::to_string(&proof).unwrap(),
                    }),
                });
                let candidate = view.workspace.store.prepare(&op, &request).unwrap();
                view.open_project_note(&projects[0], &f.a.id, window, cx);
                view.open_review(candidate.clone(), window, cx);
                assert!(!view.writing.review.inputs_shown());
                (
                    projects[0].clone(),
                    projects[1].clone(),
                    reference.id,
                    candidate,
                    private_request,
                )
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        let disclosure = window.find("writing-private-disclosure");
        assert!(disclosure.visible());
        assert!(
            disclosure
                .label()
                .unwrap()
                .contains("PRIVATE NAMED REQUIREMENTS")
        );
    })
    .unwrap();
    let loaded = cx.read(|cx| f.view.read(cx).writing_review_loads);
    for _ in 0..10 {
        cx.update_window(f.window, |_, _window, cx| {
            f.view.update(cx, |_, cx| cx.notify());
        })
        .unwrap();
        cx.run_until_parked();
    }
    assert_eq!(
        cx.read(|cx| f.view.read(cx).writing_review_loads),
        loaded,
        "unchanged repeated native renders reuse one checked batch instead of rescanning records/receipts"
    );
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.keep_review(window, cx);
            view.set_note_reading(false, window, cx);
            view.writing.note_editor.update(cx, |editor, cx| {
                editor.set_selected_range(0..0, cx);
                editor.focus(window, cx);
            });
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    let before_typing = cx.read(|cx| f.view.read(cx).writing_review_loads);
    for character in "cachetyping".chars() {
        cx.simulate_input(f.window, &character.to_string());
        cx.run_until_parked();
    }
    cx.read(|cx| {
        let view = f.view.read(cx);
        let session = view.writing.note.as_ref().unwrap().session().expect("real typing creates a recovery buffer");
        assert!(session.generation >= "cachetyping".len() as u64);
        assert!(session.markdown.contains("cachetyping"));
        assert_eq!(session.markdown, view.writing.note_editor.read(cx).value().to_string());
        assert_eq!(view.workspace.store.edit_session(&session.id).unwrap(), *session);
        assert_eq!(view.workspace.store.note(&f.a.id).unwrap().unwrap().version, 1);
        assert_eq!(view.writing_review_loads, before_typing, "real typing generations must not reload/verify the workspace-wide writing batch per keystroke");
    });
    // Explicitly discard only this test's typed buffer before continuing the
    // unchanged exact-candidate/context/access assertions below.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let session = view
                .writing
                .note
                .as_ref()
                .unwrap()
                .session()
                .cloned()
                .unwrap();
            view.discard_session(session, window, cx);
            view.open_project_note(&project, &f.a.id, window, cx);
            view.open_review(candidate.clone(), window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    // A newly displayed candidate is checked by content even when SQLite is unchanged.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let mut unbound = candidate.clone();
            unbound.request.reason = "UNBOUND PRIVATE REASON".into();
            view.writing.review.keep().unwrap();
            view.open_review(unbound, window, cx);
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("writing-evidence-unavailable").visible());
        assert!(
            !format!("{:?}", gpui_kit::base::test_support::snapshots(window))
                .contains("UNBOUND PRIVATE REASON")
        );
        f.view.update(cx, |view, cx| {
            view.writing.review.keep().unwrap();
            view.open_review(candidate.clone(), window, cx);
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("writing-private-disclosure").visible());
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_project_note(&other, &f.a.id, window, cx);
            view.writing
                .expanded_requests
                .insert(private_request.clone());
            view.writing.review.toggle_inputs();
            view.writing.review.set_reading(true);
            view.layout.open(TileKind::Compare);
            view.apply_review(window, cx);
            assert!(view.writing.review.is_some());
            assert!(
                view.workspace
                    .store
                    .receipt(&candidate.operation)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                view.workspace.store.note(&f.a.id).unwrap().unwrap().version,
                1
            );
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("writing-evidence-unavailable").visible());
        assert!(window.find("compare-evidence-unavailable").visible());
        assert!(window.try_find("writing-private-disclosure").is_none());
        let observed = format!("{:?}", gpui_kit::base::test_support::snapshots(window));
        for private in [
            "PRIVATE NAMED REQUIREMENTS",
            "PRIVATE EXACT SOURCE QUOTE",
            "PRIVATE GENERATED QUOTE",
            "PRIVATE REQUEST FEEDBACK",
            "PRIVATE BRIEF",
        ] {
            assert!(
                !observed.contains(private),
                "wrong displayed context exposed {private}"
            );
        }
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.keep_review(window, cx);
            assert_eq!(view.note_writing_scope.as_deref(), Some(other.as_str()));
            assert_ne!(
                view.latest_note_writing_request(&f.a.id).unwrap().id,
                private_request
            );
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(
            window
                .try_find(format!("writing-proposal-{}", candidate.operation.as_str()))
                .is_none()
        );
        assert_eq!(
            window.find("writing-run-status").label(),
            Some("Rewrite inputs saved")
        );
        let observed = format!("{:?}", gpui_kit::base::test_support::snapshots(window));
        for private in [
            "PRIVATE NAMED REQUIREMENTS",
            "PRIVATE EXACT SOURCE QUOTE",
            "PRIVATE GENERATED QUOTE",
            "PRIVATE REQUEST FEEDBACK",
            "PRIVATE BRIEF",
            "Rewrite ready · 0 comments",
        ] {
            assert!(
                !observed.contains(private),
                "wrong-context bar/request exposed {private}"
            );
        }
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_project_note(&project, &f.a.id, window, cx);
            view.open_review(candidate.clone(), window, cx);
            let saved = view.workspace.store.note(&reference).unwrap().unwrap();
            assert!(matches!(
                view.workspace
                    .owner_change(
                        "native-reference-revoked",
                        &ChangeRequest {
                            reason: "Synthetic access loss".into(),
                            writes: vec![Put {
                                id: reference.clone(),
                                expected_version: Some(saved.version),
                                archived: true,
                                data: saved.data
                            }],
                            inputs: vec![]
                        }
                    )
                    .unwrap(),
                ApplyOutcome::Applied(_)
            ));
            view.apply_review(window, cx);
            assert!(
                view.workspace
                    .store
                    .receipt(&candidate.operation)
                    .unwrap()
                    .is_none()
            );
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("writing-evidence-unavailable").visible());
        assert!(window.try_find("writing-private-disclosure").is_none());
    })
    .unwrap();
    cx.update_window(f.window, |_, _window, cx| {
        f.view.update(cx, |view, cx| {
            let saved = view.workspace.store.note(&reference).unwrap().unwrap();
            let mut external =
                Workspace::open_without_default_credentials(view.workspace.store.directory())
                    .unwrap();
            assert!(matches!(
                external
                    .owner_change(
                        "native-reference-restored",
                        &ChangeRequest {
                            reason: "Restore existing access".into(),
                            writes: vec![Put {
                                id: reference.clone(),
                                expected_version: Some(saved.version),
                                archived: false,
                                data: saved.data
                            }],
                            inputs: vec![]
                        }
                    )
                    .unwrap(),
                ApplyOutcome::Applied(_)
            ));
            assert!(
                view.workspace
                    .store
                    .receipt(&candidate.operation)
                    .unwrap()
                    .is_none()
            );
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    // External revocation and same-connection restoration also invalidate on
    // notify alone, without granting cached review permission to Apply.
    cx.update_window(f.window, |_, _window, cx| {
        f.view.update(cx, |view, cx| {
            let mut external =
                Workspace::open_without_default_credentials(view.workspace.store.directory())
                    .unwrap();
            let saved = external.store.note(&reference).unwrap().unwrap();
            assert!(matches!(
                external
                    .owner_change(
                        "native-external-revoke",
                        &ChangeRequest {
                            reason: "Revoke existing access externally".into(),
                            writes: vec![Put {
                                id: reference.clone(),
                                expected_version: Some(saved.version),
                                archived: true,
                                data: saved.data
                            }],
                            inputs: vec![],
                        }
                    )
                    .unwrap(),
                ApplyOutcome::Applied(_)
            ));
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("writing-evidence-unavailable").visible());
        assert!(
            !format!("{:?}", gpui_kit::base::test_support::snapshots(window))
                .contains("PRIVATE NAMED REQUIREMENTS")
        );
        assert!(
            !format!("{:?}", gpui_kit::base::test_support::snapshots(window))
                .contains("PRIVATE GENERATED QUOTE")
        );
        f.view.update(cx, |view, cx| {
            view.apply_review(window, cx);
            assert!(
                view.workspace
                    .store
                    .receipt(&candidate.operation)
                    .unwrap()
                    .is_none()
            );
            let saved = view.workspace.store.note(&reference).unwrap().unwrap();
            assert!(matches!(
                view.workspace
                    .owner_change(
                        "native-local-restore",
                        &ChangeRequest {
                            reason: "Restore existing access locally".into(),
                            writes: vec![Put {
                                id: reference.clone(),
                                expected_version: Some(saved.version),
                                archived: false,
                                data: saved.data
                            }],
                            inputs: vec![],
                        }
                    )
                    .unwrap(),
                ApplyOutcome::Applied(_)
            ));
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("writing-private-disclosure").visible());
        f.view.update(cx, |view, cx| {
            view.apply_review(window, cx);
            assert!(view.workspace.store.receipt(&candidate.operation).unwrap().is_some());
            assert!(matches!(view.workspace.store.note(&f.a.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "PRIVATE GENERATED QUOTE"));
        });
    }).unwrap();
}

#[gpui_kit::test]
fn review_thread_switch_preserves_both_chat_drafts_and_the_human_buffer(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            view.composer.update(cx, |editor, cx| {
                editor.set_value("A request draft", window, cx)
            });
            let (_, candidate) =
                proposal(view, &f.a, &f.chat_a, &[], "Whole note proposal", "review");
            view.select_writing_thread(f.a.id.clone(), f.chat_b.clone(), window, cx);
            view.composer.update(cx, |editor, cx| {
                editor.set_value("B must never go to A", window, cx)
            });
            view.writing.note_editor.update(cx, |editor, cx| {
                editor.set_value("Preserved human buffer", window, cx)
            });
            view.preserve_visible_note_input(cx);
            view.open_review(candidate, window, cx);
            assert_eq!(view.open_note_id(), Some(f.a.id.as_str()));
            assert_eq!(view.writing_threads.get(&f.a.id), Some(&f.chat_a));
            assert_eq!(view.composer.read(cx).value().as_ref(), "A request draft");
            assert_eq!(
                view.writing.note_editor.read(cx).value().as_ref(),
                "Preserved human buffer"
            );
            assert_eq!(
                view.composer_drafts
                    .text(&ComposerDestination::Thread(f.chat_b.clone())),
                "B must never go to A"
            );
            view.apply_review(window, cx);
            assert!(view.writing.review.is_some());
            assert!(view.notice.contains("open edit"));
            assert_eq!(
                view.writing.note_editor.read(cx).value().as_ref(),
                "Preserved human buffer"
            );
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("review-panel").is_some());
        assert!(window.try_find("thread-conversation").is_some());
        assert!(window.try_find("ask-delegate").is_some());
        assert!(
            window.try_find("note-body").is_none(),
            "review replaces editing in place"
        );
    })
    .unwrap();
}
#[gpui_kit::test]
fn addressed_subset_and_restart_keep_actual_apply_undo_for_each_note(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut applied_a = String::new();
    let mut applied_b = String::new();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            feedback(view, &f.a, "tere", 0..4, "First greeting", window, cx);
            feedback(view, &f.a, "õun", 10..14, "Fruit instruction", window, cx);
            let ids = view
                .workspace
                .writing_comments(&f.a.id)
                .unwrap()
                .into_iter()
                .map(|comment| comment.record.id)
                .collect::<Vec<_>>();
            let (snapshot, candidate) =
                proposal(view, &f.a, &f.chat_a, &ids, "Rewritten whole note A", "a");
            applied_a = candidate.operation.as_str().into();
            view.open_review(candidate, window, cx);
            view.apply_review(window, cx);
            assert!(view.writing.review.is_none());
            assert!(
                view.workspace
                    .writing_comments(&f.a.id)
                    .unwrap()
                    .iter()
                    .all(|comment| !comment.addressed)
            );
            view.confirm_addressed(&snapshot.request_message, &applied_a, &ids[..1], cx);
            let comments = view.workspace.writing_comments(&f.a.id).unwrap();
            assert_eq!(
                comments.iter().filter(|comment| comment.addressed).count(),
                1
            );
            assert_eq!(
                view.writing.applied_rewrite.get(&f.a.id).unwrap().1,
                applied_a
            );
            let current = view.workspace.store.note(&f.a.id).unwrap().unwrap();
            // A later request contains the same marker but is not an Apply/Undo receipt.
            view.workspace
                .snapshot_writing_request(
                    "later-request-a",
                    &f.a.id,
                    current.version,
                    &f.chat_a,
                    &[],
                )
                .unwrap();
            view.open_note(&f.b.id, None, window, cx);
            let (_, candidate) =
                proposal(view, &f.b, &f.chat_b, &[], "Rewritten whole note B", "b");
            applied_b = candidate.operation.as_str().into();
            view.open_review(candidate, window, cx);
            view.apply_review(window, cx);
            view.refresh();
            assert_eq!(
                view.writing.applied_rewrite.get(&f.a.id).unwrap().1,
                applied_a
            );
            assert_eq!(
                view.writing.applied_rewrite.get(&f.b.id).unwrap().1,
                applied_b
            );
        })
    })
    .unwrap();
    let reopened = Workspace::open(&f.data).unwrap();
    let fresh = cx
        .update_window(f.window, |_, window, cx| {
            cx.new(|cx| {
                let mut view = ThreadsUi::new(reopened, f.data.clone(), window, cx);
                view.poll = None;
                assert_eq!(
                    view.writing.applied_rewrite.get(&f.a.id).unwrap().1,
                    applied_a
                );
                assert_eq!(
                    view.writing.applied_rewrite.get(&f.b.id).unwrap().1,
                    applied_b
                );
                assert!(matches!(
                    view.workspace.undo(&applied_a).unwrap(),
                    ApplyOutcome::Applied(_)
                ));
                view.refresh();
                assert!(!view.writing.applied_rewrite.contains_key(&f.a.id));
                assert_eq!(
                    view.writing.applied_rewrite.get(&f.b.id).unwrap().1,
                    applied_b
                );
                assert!(
                    view.workspace
                        .writing_comments(&f.a.id)
                        .unwrap()
                        .iter()
                        .all(|comment| !comment.addressed)
                );
                view
            })
        })
        .unwrap();
    cx.read(|cx| assert!(fresh.read(cx).worker.is_none()));
}
#[gpui_kit::test]
fn archived_writing_chat_preserves_feedback_and_requires_explicit_unarchive(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let state = |view: &ThreadsUi, id: &str| match view
                .workspace
                .store
                .record(id)
                .unwrap()
                .unwrap()
                .data
            {
                RecordData::Thread(thread) => thread.state,
                _ => unreachable!(),
            };
            view.open_note(&f.a.id, None, window, cx);
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            view.resolve_thread(&f.chat_a, cx);
            assert_eq!(state(view, &f.chat_a), ThreadState::Open);
            assert!(
                view.workspace
                    .store
                    .record(&f.chat_a)
                    .unwrap()
                    .unwrap()
                    .archived
            );
            feedback(
                view,
                &f.a,
                "tere",
                5..9,
                "Feedback after resolution",
                window,
                cx,
            );
            assert_eq!(view.workspace.writing_comments(&f.a.id).unwrap().len(), 0);
            assert!(view.notice.contains("Unarchive"));
            assert!(
                view.writing
                    .comment_input
                    .read(cx)
                    .value()
                    .contains("Feedback after resolution")
            );
            assert!(
                view.workspace
                    .store
                    .record(&f.chat_a)
                    .unwrap()
                    .unwrap()
                    .archived
            );
            cx.notify();
        })
    })
    .unwrap();
    cx.run_until_parked();
    // The archived chat's only Unarchive is the composer banner's.
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("reopen-thread").visible());
        assert!(window.try_find("chat-unarchive").is_none());
        window.click("reopen-thread", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let state = |view: &ThreadsUi, id: &str| match view.workspace.store.record(id).unwrap().unwrap().data {
                RecordData::Thread(thread) => thread.state,
                _ => unreachable!(),
            };
            assert!(!view.workspace.store.record(&f.chat_a).unwrap().unwrap().archived);
            view.composer.update(cx, |editor, cx| {
                editor.set_value("Replying reopens", window, cx)
            });
            view.ask(window, cx);
            assert_eq!(state(view, &f.chat_a), ThreadState::Open);
            assert!(view.queue_state().paused);
            assert!(view.workspace.messages(&f.chat_a).unwrap().iter().any(
                |record| matches!(&record.data, RecordData::Message(m) if m.text == "Replying reopens")
            ));
            assert!(view.worker.is_none(), "offline fixture starts no provider");
            assert_eq!(view.chat, ChatTarget::Thread(f.chat_a.clone()));
            view.composer.update(cx, |editor, cx| {
                editor.set_value("Draft stays with A", window, cx)
            });
            view.new_writing_thread(window, cx);
            let new = view.writing_threads.get(&f.a.id).unwrap().clone();
            assert_ne!(new, f.chat_a);
            assert_eq!(view.chat, ChatTarget::Thread(new.clone()));
            assert_eq!(
                view.composer_drafts
                    .text(&ComposerDestination::Thread(f.chat_a.clone())),
                "Draft stays with A"
            );
            assert_eq!(view.open_note_id(), Some(f.a.id.as_str()));
        })
    })
    .unwrap();
}

#[gpui_kit::test]
fn narrow_workbench_scrolls_tiles_to_usable_chat_and_keeps_one_editor(cx: &mut TestAppContext) {
    let f = fixture_size(cx, 760., 820.);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            view.workspace
                .post_message(&f.chat_a, "A real posted chat remains accessible.")
                .unwrap();
            feedback(view, &f.a, "tere", 0..4, "First greeting", window, cx);
            feedback(view, &f.a, "tere", 5..9, "Second greeting", window, cx);
            view.rewrite_note(cx);
        })
    })
    .unwrap();
    cx.run_until_parked();
    let editor = cx.read(|cx| f.view.read(cx).writing.note_editor.clone());
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("note-body").bounds().size.width > px(200.));
        window.scroll(
            "tile-row",
            gpui_kit::ScrollDelta::Pixels(point(px(-2000.), px(0.))),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("chat-tile").bounds().size.height >= px(120.));
        let posted = f.view.read(cx).workspace.messages(&f.chat_a).unwrap().into_iter().find(|record| matches!(&record.data, RecordData::Message(message) if message.role == "owner")).unwrap();
        assert!(window.find(format!("chat-message-{}", posted.id)).visible());
        assert!(window.find("ask-delegate").visible());
        assert_eq!(f.view.read(cx).writing.note_editor, editor);
    })
    .unwrap();
}

#[gpui_kit::test]
fn browsing_notes_keeps_selected_chat_and_replacing_main_preserves_review_text(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            view.composer.update(cx, |editor, cx| {
                editor.set_value("Keep this conversation draft", window, cx)
            });
            view.open_note(&f.b.id, None, window, cx);
            assert_eq!(view.chat_thread(), Some(f.chat_a.as_str()));
            assert_eq!(
                view.composer.read(cx).value().as_ref(),
                "Keep this conversation draft"
            );
            view.open_note(&f.a.id, None, window, cx);
            let (_, candidate) = proposal(
                view,
                &f.a,
                &f.chat_a,
                &[],
                "Exact proposed wording",
                "area-review",
            );
            let candidate_id = candidate.operation.clone();
            view.open_review(candidate, window, cx);
            view.start_review_edit(window, cx);
            view.writing.review_editor.update(cx, |editor, cx| {
                editor.set_value("Owner's unsubmitted final text", window, cx)
            });
            view.show_tile(TileKind::Actions, window, cx);
            assert_eq!(
                view.writing.review.as_ref().unwrap().candidate.operation,
                candidate_id
            );
            assert_eq!(
                view.writing.review_editor.read(cx).value().as_ref(),
                "Owner's unsubmitted final text"
            );
            view.show_tile(TileKind::Note, window, cx);
            assert!(view.writing.review.is_editing());
            assert_eq!(
                view.writing.review.as_ref().unwrap().candidate.operation,
                candidate_id
            );
            assert_eq!(
                view.writing.review_editor.read(cx).value().as_ref(),
                "Owner's unsubmitted final text"
            );
            assert_eq!(view.workspace.store.note(&f.a.id).unwrap().unwrap(), f.a);
        });
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn reopening_current_review_from_solo_chat_removes_hidden_composer_focus(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let mut operation = String::new();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            let (_, candidate) =
                proposal(view, &f.a, &f.chat_a, &[], "Exact proposal", "solo-reopen");
            operation = candidate.operation.as_str().to_owned();
            view.open_review(candidate, window, cx);
            view.start_review_edit(window, cx);
            view.writing.review_editor.update(cx, |editor, cx| {
                editor.set_value("Unsubmitted reviewed text", window, cx)
            });
            view.show_tile(TileKind::Chat, window, cx);
            view.apply_layout(Preset::Solo, window, cx);
            assert!(view.composer.focus_handle(cx).is_focused(window));
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let candidate = view
                .candidates
                .iter()
                .find(|candidate| candidate.operation.as_str() == operation)
                .unwrap()
                .clone();
            view.open_review(candidate, window, cx);
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        let view = f.view.read(cx);
        assert_eq!(view.layout.maximized(), Some(TileKind::Note));
        assert!(!view.composer.focus_handle(cx).is_focused(window));
        assert!(
            view.writing
                .review_editor
                .focus_handle(cx)
                .is_focused(window)
        );
        assert_eq!(
            view.writing.review_editor.read(cx).value().as_ref(),
            "Unsubmitted reviewed text"
        );
    })
    .unwrap();
}

/// U4: typing starts an edit, but another edit of the note opened after this
/// window loaded it (another window or the CLI). The typed
/// `EditSessionOpen` names its base version; the other edit is kept and the
/// note offers Compare (that edit beside the saved note) and Discard (the
/// usual confirmation), which keeps what was typed here as the new edit.
#[gpui_kit::test]
fn an_edit_already_open_elsewhere_offers_compare_and_discard(cx: &mut TestAppContext) {
    // Control real-time dialog entrance motion while testing confirmation behavior.
    cx.update(|cx| cx.set_reduce_motion(true));
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view
            .update(cx, |view, cx| view.open_note(&f.a.id, None, window, cx))
    })
    .unwrap();
    cx.run_until_parked();
    let mut other = Workspace::open(&f.data).unwrap();
    let elsewhere = other.store.begin_edit(&f.a.id, f.a.version).unwrap();
    other
        .store
        .update_buffer(&elsewhere.id, 1, "Edit from elsewhere")
        .unwrap();
    drop(other);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.writing.note_editor
                .update(cx, |editor, cx| editor.set_value("What I typed here", window, cx));
            view.preserve_visible_note_input(cx);
            assert_eq!(view.writing.note.open_edit_elsewhere(), Some((f.a.id.as_str(), f.a.version)));
            assert_eq!(
                view.notice,
                format!(
                    "An edit of this note is already open from v{}. What you typed here isn't saved.",
                    f.a.version
                )
            );
            // Before: "Could not preserve this edit: this note already has
            // an open unsaved edit. …" and nothing to act on.
            assert!(!view.notice.contains("Could not preserve"));
            assert!(view.writing.note.as_ref().unwrap().session().is_none());
            cx.notify();
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("note-edit-open-banner").visible());
        assert!(window.try_find("note-recover-banner").is_none());
        for id in [
            "recover-note-edit",
            "note-edit-open-compare",
            "note-edit-open-discard",
        ] {
            assert!(
                window.within("note-edit-open-banner").find(id).visible(),
                "{id}"
            );
        }
    })
    .unwrap();
    // Compare: the other edit beside the saved note.
    cx.update_window(f.window, |_, window, cx| {
        window.click("note-edit-open-compare", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.has_active_dialog(cx));
        assert_eq!(
            window.find("compare-open-edit-buffer").label(),
            Some("Edit from elsewhere")
        );
        assert_eq!(
            window.find("compare-open-edit-saved").label(),
            Some("tere tere õun")
        );
        window.click("compare-open-edit-close", cx);
    })
    .unwrap();
    cx.run_until_parked();
    // Discard: the existing confirmation, worded for the other edit (the
    // owner's own Discard… says "Your changes since vN will be lost").
    cx.update_window(f.window, |_, window, cx| {
        assert!(!window.has_active_dialog(cx));
        window.click("note-edit-open-discard", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.has_active_dialog(cx));
        assert_eq!(
            window.find("discard-note-body").label(),
            Some(
                format!(
                    "Its changes since v{} will be lost. What you typed here is kept. This can't be undone.",
                    f.a.version
                )
                .as_str()
            )
        );
        window.click("discard-note-confirm", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        let view = f.view.read(cx);
        assert_eq!(view.writing.note.open_edit_elsewhere(), None);
        let session = view
            .writing
            .note
            .as_ref()
            .unwrap()
            .session()
            .cloned()
            .unwrap();
        assert_ne!(session.id, elsewhere.id);
        assert_eq!(session.markdown, "What I typed here");
        assert!(
            view.workspace
                .store
                .edit_session(&elsewhere.id)
                .unwrap()
                .closed
        );
        assert_eq!(
            view.notice,
            "Discarded the open edit. What you typed here is now your unsaved edit."
        );
        assert!(window.try_find("note-edit-open-banner").is_none());
        assert_eq!(
            view.workspace.store.note(&f.a.id).unwrap().unwrap(),
            f.a,
            "nothing was saved"
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn panel_shared_start_keeps_scoped_saved_retry_when_chat_is_other_and_hidden(
    cx: &mut TestAppContext,
) {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};

    let f = fixture(cx);
    let thread = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let project = view
                    .workspace
                    .project_command(
                        "b1b-project",
                        ProjectCommand::Create {
                            title: "B1b scoped writing".into(),
                        },
                    )
                    .unwrap()
                    .writes[0]
                    .after
                    .id
                    .clone();
                view.workspace
                    .project_command(
                        "b1b-document",
                        ProjectCommand::LinkDocument {
                            project: project.clone(),
                            note: f.a.id.clone(),
                        },
                    )
                    .unwrap();
                let context = view
                    .workspace
                    .ensure_writing_context("b1b-context", &f.a.id, Some(&project))
                    .unwrap();
                let thread = context.conversation.unwrap();
                view.open_note_in_context(&f.a.id, None, Some(&project), window, cx);
                view.select_writing_thread(f.a.id.clone(), thread.clone(), window, cx);
                view.workspace
                    .post_message(&thread, "B1b frozen scoped direction")
                    .unwrap();
                view.workspace
                    .post_message(&f.chat_b, "B1b unrelated chat must stay outside")
                    .unwrap();
                view.set_chat(ChatTarget::Thread(f.chat_b.clone()), window, cx);
                view.layout.close(TileKind::Chat);
                view.writing.general_feedback.update(cx, |editor, cx| {
                    editor.set_value("B1b frozen feedback", window, cx)
                });
                let mut settings = view.workspace.settings().unwrap();
                settings.provider = "Copilot".into();
                settings.model = "b1b-unoffered".into();
                settings.auth_file = None;
                settings.credentials_dir = None;
                view.workspace.configure(settings).unwrap();
                view.copilot_models = vec!["b1b-only-offered".into()];
                view.refresh();
                cx.notify();
                thread
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.click("rewrite-note", cx))
        .unwrap();
    cx.run_until_parked();
    let snapshot = cx.read(|cx| {
        let view = f.view.read(cx);
        assert!(
            view.worker.is_none(),
            "catalogue refusal must not access a provider"
        );
        assert!(view.notice.contains("not offered by GitHub Copilot"));
        assert!(view.notice.contains("b1b-only-offered"));
        assert!(
            view.notice
                .contains("Retry rewrite uses these same saved inputs")
        );
        assert_eq!(view.notice_tone(), Tone::Warning);
        let request = view.latest_note_writing_request(&f.a.id).unwrap();
        let snapshot = view.workspace.writing_request(&request.id).unwrap();
        assert_eq!(snapshot.thread, thread);
        assert_eq!(snapshot.general_feedback, "B1b frozen feedback");
        assert!(snapshot.setup.as_ref().unwrap().chat.is_some());
        snapshot
    });
    let before_retry = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                view.workspace
                    .post_message(&thread, "B1b later chat is not captured")
                    .unwrap();
                view.writing.general_feedback.update(cx, |editor, cx| {
                    editor.set_value("B1b unsent next feedback", window, cx)
                });
                let records = view.workspace.records().unwrap();
                let receipts = view.workspace.store.receipts().unwrap();
                cx.notify();
                (records, receipts)
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.click("retry-rewrite", cx))
        .unwrap();
    cx.run_until_parked();
    let prepared = cx
        .update_window(f.window, |_, _, cx| {
            f.view.update(cx, |view, _| {
                assert_eq!(view.workspace.records().unwrap(), before_retry.0);
                assert_eq!(view.workspace.store.receipts().unwrap(), before_retry.1);
                assert_eq!(
                    view.workspace
                        .writing_request(&snapshot.request_message)
                        .unwrap(),
                    snapshot
                );
                assert_eq!(view.chat, ChatTarget::Thread(f.chat_b.clone()));
                assert!(!view.layout.contains(TileKind::Chat));
                // Consume the same real panel request with the shared preparer and
                // scripted runtime. No alternate desktop driver/provider hook.
                let mut settings = view.workspace.settings().unwrap();
                settings.provider = "Codex".into();
                settings.model = "gpt-6.1-sol".into();
                settings.effort = "medium".into();
                view.workspace.configure(settings).unwrap();
                let start = view
                    .writing
                    .rewrite_run_start(
                        &view.workspace,
                        &snapshot.request_message,
                        view.note_writing_scope.as_deref(),
                        vec![],
                    )
                    .unwrap();
                view.workspace.prepare_thread_run(start).unwrap()
            })
        })
        .unwrap();
    let run_id = prepared.prepared.run_id().to_owned();
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call(
                "b1b-prepare",
                "prepare_changes",
                serde_json::json!({
                    "request_key": "b1b-whole-note",
                    "request": snapshot.change_request("B1b proposed complete note").unwrap(),
                    "writing": {"quotations": [], "requirements_reported": []}
                }),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text("B1b whole-note proposal is ready for exact review."),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let observed = model.clone();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(brn_ai::threads::run_prepared_scripted(
            prepared.prepared,
            model,
        ))
        .unwrap();
    assert_eq!(result.run_id, run_id);
    assert!(matches!(
        result.answer.terminal,
        brn_ai::AiTerminal::Completed
    ));
    let delivered = serde_json::to_string(&observed.requests()).unwrap();
    assert!(delivered.contains("B1b frozen scoped direction"));
    assert!(delivered.contains("B1b frozen feedback"));
    for excluded in [
        "B1b unrelated chat must stay outside",
        "B1b later chat is not captured",
        "B1b unsent next feedback",
    ] {
        assert!(
            !delivered.contains(excluded),
            "saved retry delivered {excluded}"
        );
    }
    cx.read(|cx| {
        let view = f.view.read(cx);
        let record = view.workspace.store.record(&run_id).unwrap().unwrap();
        let RecordData::Run(run) = record.data else {
            panic!("prepared Run")
        };
        assert_eq!(run.thread, thread);
        assert_eq!(
            run.instruction.as_deref(),
            Some(snapshot.request_message.as_str())
        );
        assert_eq!(run.state, RunState::Completed);
        let candidates = view.workspace.candidates().unwrap();
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].request.inputs.iter().any(|input| {
            input.record == snapshot.request_message
        }));
        assert!(candidates[0].request.writes.iter().any(|write| {
            write.id == f.a.id && matches!(&write.data, RecordData::Note(note) if note.markdown == "B1b proposed complete note")
        }));

        assert_eq!(view.workspace.store.note(&f.a.id).unwrap().unwrap(), f.a);
        assert_eq!(
            view.workspace
                .writing_request(&snapshot.request_message)
                .unwrap(),
            snapshot
        );
        assert_eq!(view.chat, ChatTarget::Thread(f.chat_b.clone()));
        assert!(!view.layout.contains(TileKind::Chat));
        assert_eq!(
            view.writing.general_feedback.read(cx).value().as_ref(),
            "B1b unsent next feedback"
        );
    });
}

#[gpui_kit::test]
fn saved_retry_refuses_wrong_displayed_context_and_preserves_archived_state_at_offline_selection(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let project = view
                .workspace
                .project_command(
                    "b1b-refusal-project",
                    ProjectCommand::Create {
                        title: "B1b refusal scope".into(),
                    },
                )
                .unwrap()
                .writes[0]
                .after
                .id
                .clone();
            view.workspace
                .project_command(
                    "b1b-refusal-document",
                    ProjectCommand::LinkDocument {
                        project: project.clone(),
                        note: f.a.id.clone(),
                    },
                )
                .unwrap();
            let context = view
                .workspace
                .ensure_writing_context("b1b-refusal-context", &f.a.id, Some(&project))
                .unwrap();
            let thread = context.conversation.unwrap();
            let snapshot = view
                .workspace
                .snapshot_writing_request_with_feedback(
                    "b1b-refusal-request",
                    &f.a.id,
                    f.a.version,
                    &thread,
                    &[],
                    "B1b preserved saved feedback",
                )
                .unwrap();
            // Raw saved evidence still exists; UI start must use the existing
            // contextual reader, independently of a provider-selection failure.
            assert!(
                view.workspace
                    .writing_request(&snapshot.request_message)
                    .is_ok()
            );
            for wrong_note in [&f.b.id, &f.a.id] {
                view.open_note(wrong_note, None, window, cx);
                view.writing.general_feedback.update(cx, |editor, cx| {
                    editor.set_value("B1b preserved unsent feedback", window, cx)
                });
                let before = (
                    view.workspace.records().unwrap(),
                    view.workspace.store.receipts().unwrap(),
                );
                view.retry_rewrite(snapshot.request_message.clone(), cx);
                assert!(view.worker.is_none());
                assert!(
                    view.notice
                        .contains("unavailable in this displayed context")
                );
                assert!(
                    !view.notice.contains("Choose a provider"),
                    "context refusal precedes selection"
                );
                assert_eq!(view.workspace.records().unwrap(), before.0);
                assert_eq!(view.workspace.store.receipts().unwrap(), before.1);
                assert_eq!(
                    view.writing.general_feedback.read(cx).value().as_ref(),
                    "B1b preserved unsent feedback"
                );
            }
            view.open_note_in_context(&f.a.id, None, Some(&project), window, cx);
            let replacement = view
                .workspace
                .new_writing_conversation("b1b-explicit-new", &f.a.id, Some(&project))
                .unwrap()
                .conversation
                .unwrap();
            assert_ne!(replacement, thread);
            let before = (
                view.workspace.records().unwrap(),
                view.workspace.store.receipts().unwrap(),
            );
            view.retry_rewrite(snapshot.request_message.clone(), cx);
            assert!(
                view.notice
                    .contains("unavailable in this displayed context")
            );
            assert_eq!(view.workspace.records().unwrap(), before.0);
            assert_eq!(view.workspace.store.receipts().unwrap(), before.1);
            view.workspace
                .select_writing_conversation("b1b-explicit-return", &f.a.id, &thread)
                .unwrap();
            view.workspace
                .archive_chat("b1b-refusal-archive", &thread)
                .unwrap();
            let before = (
                view.workspace.records().unwrap(),
                view.workspace.store.receipts().unwrap(),
            );
            view.retry_rewrite(snapshot.request_message.clone(), cx);
            assert!(view.worker.is_none());
            // Archived saved context remains readable; this offline fixture
            // stops at provider selection, not an archive-specific retry gate.
            assert_eq!(
                view.notice,
                "Rewrite request saved; AI has not started. Choose a provider and model in Settings before asking BRN. Retry rewrite uses these same saved inputs."
            );
            assert_eq!(view.workspace.records().unwrap(), before.0);
            assert_eq!(view.workspace.store.receipts().unwrap(), before.1);
            assert_eq!(
                view.workspace
                    .writing_request(&snapshot.request_message)
                    .unwrap(),
                snapshot
            );
            assert!(
                view.workspace
                    .store
                    .record(&thread)
                    .unwrap()
                    .unwrap()
                    .archived
            );
        });
    })
    .unwrap();
}

// B3 fixtures below are source-prepared only until the principal grants execution.
#[gpui_kit::test]
fn b3_brief_reference_purpose_persist_through_chats_navigation_closed_chat_and_reopen(
    cx: &mut TestAppContext,
) {
    let f = fixture_size(cx, 900., 600.);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            view.writing.setup_brief.update(cx, |input, cx| input.set_value("Purpose: explain. Audience: owners. Tone: plain. Requirements: cite retained evidence.", window, cx));
            // Use the real scoped search service; the attach click consumes its exact usable result.
            let page = view.workspace.search_handle().unwrap().request_page("Second saved paragraph", None, 12, None).wait().unwrap();
            assert!(page.matches.iter().any(|hit| hit.note == f.b.id));
            view.writing.setup.as_mut().unwrap().matches = page.matches;
            cx.notify();
        });
    }).unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "attach-writing-reference-0");
    cx.update_window(f.window, |_, window, cx| {
        window.click("attach-writing-reference-0", cx)
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "reference-purpose-0-Requirements");
    cx.update_window(f.window, |_, window, cx| {
        window.click("reference-purpose-0-Requirements", cx)
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "save-writing-setup");
    cx.update_window(f.window, |_, window, cx| {
        window.click("save-writing-setup", cx)
    })
    .unwrap();
    cx.run_until_parked();
    let saved = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let saved = view
                    .workspace
                    .writing_setup(&f.a.id, None)
                    .unwrap()
                    .record
                    .unwrap();
                assert_eq!(
                    view.workspace.store.note(&f.a.id).unwrap().unwrap(),
                    f.a,
                    "setup never changes the working Note version/content"
                );
                let RecordData::WritingSetup(config) = &saved.data else {
                    panic!("setup")
                };
                assert_eq!(
                    config.references,
                    vec![WritingReference {
                        note: f.b.id.clone(),
                        purpose: WritingPurpose::Requirements
                    }]
                );
                assert!(
                    config
                        .brief
                        .as_deref()
                        .unwrap()
                        .contains("Audience: owners")
                );
                let receipt_count = view.workspace.store.receipts().unwrap().len();
                view.save_writing_setup(window, cx); // repeated Save of an unchanged form is not another configuration edit
                assert_eq!(
                    view.workspace.store.receipts().unwrap().len(),
                    receipt_count
                );
                view.new_writing_thread(window, cx);
                view.set_chat(ChatTarget::Thread(f.chat_b.clone()), window, cx);
                view.layout.close(TileKind::Chat);
                view.open_note(&f.b.id, None, window, cx);
                view.open_note(&f.a.id, None, window, cx);
                view.load_writing_setup(window, cx);
                assert_eq!(view.writing.setup.as_ref().unwrap().saved, *config);
                assert_eq!(
                    view.workspace
                        .writing_setup(&f.a.id, None)
                        .unwrap()
                        .record
                        .unwrap(),
                    saved
                );
                saved
            })
        })
        .unwrap();
    let reopened = Workspace::open_without_default_credentials(&f.data).unwrap();
    assert_eq!(
        reopened
            .writing_setup(&f.a.id, None)
            .unwrap()
            .record
            .unwrap(),
        saved
    );
    b3_reveal_setup_control(cx, &f, "remove-reference-0");
    cx.update_window(f.window, |_, window, cx| {
        window.click("remove-reference-0", cx)
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "save-writing-setup");
    cx.update_window(f.window, |_, window, cx| {
        window.click("save-writing-setup", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        let current = view.workspace.writing_setup(&f.a.id, None).unwrap();
        assert!(current.references.is_empty());
        let RecordData::WritingSetup(config) = current.record.unwrap().data else {
            panic!("setup")
        };
        assert!(config.brief.unwrap().contains("Audience: owners"));
        assert_eq!(view.workspace.store.note(&f.a.id).unwrap().unwrap(), f.a);
    });
}

#[gpui_kit::test]
fn b3_stale_setup_save_refuses_preserves_form_and_checked_replay_is_exact(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            view.writing.setup_brief.update(cx, |input, cx| {
                input.set_value("Unsaved local brief", window, cx)
            });
            let newer = WritingSetup {
                note: f.a.id.clone(),
                chat: None,
                brief: Some("Other window saved brief".into()),
                references: vec![],
            };
            let receipt = view
                .workspace
                .set_writing_setup("b3-other-window", None, &newer)
                .unwrap();
            let count = view.workspace.store.receipts().unwrap().len();
            assert_eq!(
                view.workspace
                    .set_writing_setup("b3-other-window", None, &newer)
                    .unwrap(),
                receipt
            );
            assert_eq!(view.workspace.store.receipts().unwrap().len(), count);
            view.save_writing_setup(window, cx);
            assert!(view.notice.contains("changed"));
            assert_eq!(
                view.writing.setup_brief.read(cx).value().as_ref(),
                "Unsaved local brief"
            );
            assert_eq!(view.workspace.store.receipts().unwrap().len(), count);
            assert!(!view.writing_setup_form_current());
            let RecordData::WritingSetup(saved) = view
                .workspace
                .writing_setup(&f.a.id, None)
                .unwrap()
                .record
                .unwrap()
                .data
            else {
                panic!("setup")
            };
            assert_eq!(saved, newer);
            assert_eq!(view.workspace.store.note(&f.a.id).unwrap().unwrap(), f.a);
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn b3_setup_labels_and_drafts_fail_closed_on_scope_switch_and_notify_only_revocation(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let project = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let project = view
                    .workspace
                    .project_command(
                        "b3-private-project",
                        ProjectCommand::Create {
                            title: "B3 private scope".into(),
                        },
                    )
                    .unwrap()
                    .writes[0]
                    .after
                    .id
                    .clone();
                view.workspace
                    .project_command(
                        "b3-share-document",
                        ProjectCommand::LinkDocument {
                            project: project.clone(),
                            note: f.a.id.clone(),
                        },
                    )
                    .unwrap();
                let mut reference = match f.b.data.clone() {
                    RecordData::Note(note) => note,
                    _ => unreachable!(),
                };
                reference.title = "B3 PRIVATE REFERENCE TITLE".into();
                reference.chat = Some(project.clone());
                assert!(matches!(
                    view.workspace
                        .owner_change(
                            "b3-private-reference",
                            &ChangeRequest {
                                reason: "Synthetic private reference".into(),
                                writes: vec![Put {
                                    id: f.b.id.clone(),
                                    expected_version: Some(1),
                                    archived: false,
                                    data: RecordData::Note(reference)
                                }],
                                inputs: vec![]
                            }
                        )
                        .unwrap(),
                    ApplyOutcome::Applied(_)
                ));
                view.workspace
                    .project_command(
                        "b3-link-private-reference",
                        ProjectCommand::LinkDocument {
                            project: project.clone(),
                            note: f.b.id.clone(),
                        },
                    )
                    .unwrap();
                view.workspace
                    .set_writing_setup(
                        "b3-private-setup",
                        None,
                        &WritingSetup {
                            note: f.a.id.clone(),
                            chat: Some(project.clone()),
                            brief: Some("B3 PRIVATE BRIEF".into()),
                            references: vec![WritingReference {
                                note: f.b.id.clone(),
                                purpose: WritingPurpose::Style,
                            }],
                        },
                    )
                    .unwrap();
                view.open_note_in_context(&f.a.id, None, Some(&project), window, cx);
                view.load_writing_setup(window, cx);
                assert!(view.writing_setup_form_current());
                assert_eq!(
                    view.writing.setup.as_ref().unwrap().labels[&f.b.id].title,
                    "B3 PRIVATE REFERENCE TITLE"
                );
                view.open_note_in_context(&f.a.id, None, None, window, cx);
                assert!(
                    !view.writing_setup_form_current(),
                    "same Note never reuses a different scope's draft"
                );
                cx.notify();
                project
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-reference-0").is_none());
        assert!(window.try_find("save-writing-setup").is_none());
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.load_writing_setup(window, cx);
            assert!(view.writing.setup.as_ref().unwrap().saved.brief.is_none());
            assert!(view.writing.setup.as_ref().unwrap().references.is_empty());
            view.open_note_in_context(&f.a.id, None, Some(&project), window, cx);
            view.load_writing_setup(window, cx);
            let project_record = view.workspace.store.record(&project).unwrap().unwrap();
            view.workspace
                .project_command(
                    "b3-limit-project-knowledge",
                    ProjectCommand::Configure {
                        project: project.clone(),
                        version: project_record.version,
                        title: "B3 private scope".into(),
                        instructions: String::new(),
                        use_brn_knowledge: false,
                    },
                )
                .unwrap();
            view.workspace
                .project_command(
                    "b3-revoke-document",
                    ProjectCommand::UnlinkDocument {
                        project: project.clone(),
                        note: f.a.id.clone(),
                    },
                )
                .unwrap();
            assert!(!view.writing_setup_form_current());
            cx.notify(); // no refresh: token must hide cached labels/draft on the next render
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-reference-0").is_none());
        assert!(window.try_find("save-writing-setup").is_none());
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.workspace
                .project_command(
                    "b3-restore-document",
                    ProjectCommand::LinkDocument {
                        project: project.clone(),
                        note: f.a.id.clone(),
                    },
                )
                .unwrap();
            view.load_writing_setup(window, cx);
            assert!(view.writing_setup_form_current());
            assert_eq!(
                view.writing.setup.as_ref().unwrap().labels[&f.b.id].title,
                "B3 PRIVATE REFERENCE TITLE"
            );
        });
    })
    .unwrap();
}

fn b3_converted_reference() -> ConversionResult {
    ConversionResult { intent: ImportIntent::FullNote, markdown: "# Requirements\n- Preserve retained source.\n| Item | Limit |\n| --- | --- |\n| Reading | bounded |".into(), assets: vec![], source_sha256: [73; 32], source_reference: "synthetic/b3-reference.md".into(), converter: brn_threads_intake::document_conversion_label("xberg", false).unwrap(), sources: vec![], coverage: brn_threads_intake::Coverage { status: brn_threads_intake::CoverageStatus::Partial, checked_items: vec![], gaps: vec![] } }
}

#[gpui_kit::test]
fn b3_full_note_import_attaches_once_and_wrong_target_completion_keeps_canonical_result(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            let target = view.writing.setup.as_ref().unwrap().target.clone();
            let intent = ReferenceImportIntent { target, purpose: WritingPurpose::Requirements, key: "b3-import-attach".into() };
            let converted = b3_converted_reference();
            assert!(view.complete_writing_reference_import(&intent, "B3 retained reference", &converted, &DocumentConversionSettings::default(), window, cx));
            let setup = view.workspace.writing_setup(&f.a.id, None).unwrap();
            assert_eq!(setup.references.len(), 1);
            let id = setup.references[0].reference.note.clone();
            let RecordData::Note(note) = view.workspace.store.note(&id).unwrap().unwrap().data else { panic!("imported Note") };
            assert!(note.protected);
            assert!(note.markdown.contains("| Reading | bounded |"));
            assert_eq!(note.import.unwrap().report.unwrap().status, ImportStatus::Partial);
            let before = view.workspace.records().unwrap();
            let receipts = view.workspace.store.receipts().unwrap();
            // Existing importer key replays the canonical outcome. A stale target must not rebind attachment.
            view.complete_writing_reference_import(&intent, "B3 retained reference", &converted, &DocumentConversionSettings::default(), window, cx);
            assert!(view.notice.contains("already attached in this writing scope"));
            assert_eq!(view.workspace.records().unwrap(), before);
            assert_eq!(view.workspace.store.receipts().unwrap(), receipts);
            let target = view.writing.setup.as_ref().unwrap().target.clone();
            let wrong = ReferenceImportIntent { target, purpose: WritingPurpose::Evidence, key: "b3-import-after-navigation".into() };
            view.open_note(&f.b.id, None, window, cx);
            let mut other = converted.clone();
            other.source_reference = "synthetic/b3-other-reference.md".into();
            other.source_sha256 = [74; 32];
            assert!(view.complete_writing_reference_import(&wrong, "B3 separately retained", &other, &DocumentConversionSettings::default(), window, cx));
            assert!(view.notice.contains("retained in its original scope; no reference was attached"));
            assert!(view.workspace.writing_setup(&f.b.id, None).unwrap().references.is_empty());
            assert_eq!(view.workspace.writing_setup(&f.a.id, None).unwrap().references, setup.references);
            assert_eq!(view.workspace.notes().unwrap().iter().filter(|r| matches!(&r.data, RecordData::Note(n) if n.title == "B3 separately retained")).count(), 1);
        });
    }).unwrap();
}

// Real wheel events to an observed control, including the drawer's compact viewport.
fn b3_reveal_setup_control(cx: &mut TestAppContext, f: &Fixture, id: &'static str) {
    for _ in 0..64 {
        let delta = cx
            .update_window(f.window, |_, window, _| {
                let panel = window.find("writing-setup").bounds();
                let control = window.find(id);
                if control.visible() && panel.contains(&control.bounds().center()) {
                    None
                } else if control.bounds().center().y < panel.center().y {
                    Some(20.)
                } else {
                    Some(-20.)
                }
            })
            .unwrap();
        let Some(delta) = delta else { return };
        b3_scroll_setup(cx, f, delta);
    }
    panic!("setup control {id} remained unreachable after bounded real scrolling");
}

fn b3_scroll_setup(cx: &mut TestAppContext, f: &Fixture, delta: f32) {
    cx.update_window(f.window, |_, window, cx| {
        window.scroll(
            "writing-setup",
            gpui_kit::ScrollDelta::Pixels(point(px(0.), px(delta))),
            cx,
        );
    })
    .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn b3_import_completion_refuses_stale_setup_and_revoked_working_context(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            let intent = ReferenceImportIntent { target: view.writing.setup.as_ref().unwrap().target.clone(), purpose: WritingPurpose::Evidence, key: "b3-stale-import".into() };
            let newer = WritingSetup { note: f.a.id.clone(), chat: None, brief: Some("Newer setup remains exact".into()), references: vec![] };
            view.workspace.set_writing_setup("b3-import-competing-setup", None, &newer).unwrap();
            assert!(view.complete_writing_reference_import(&intent, "B3 stale retained", &b3_converted_reference(), &DocumentConversionSettings::default(), window, cx));
            assert!(view.notice.contains("no reference was attached"));
            let RecordData::WritingSetup(saved) = view.workspace.writing_setup(&f.a.id, None).unwrap().record.unwrap().data else { panic!("setup") };
            assert_eq!(saved, newer);
            assert_eq!(view.workspace.store.note(&f.a.id).unwrap().unwrap(), f.a);

            let project = view.workspace.project_command("b3-import-project", ProjectCommand::Create { title: "B3 import project".into() }).unwrap().writes[0].after.id.clone();
            view.workspace.project_command("b3-import-working-link", ProjectCommand::LinkDocument { project: project.clone(), note: f.a.id.clone() }).unwrap();
            let record = view.workspace.store.record(&project).unwrap().unwrap();
            view.workspace.project_command("b3-import-knowledge-off", ProjectCommand::Configure { project: project.clone(), version: record.version, title: "B3 import project".into(), instructions: String::new(), use_brn_knowledge: false }).unwrap();
            view.open_note_in_context(&f.a.id, None, Some(&project), window, cx);
            view.load_writing_setup(window, cx);
            let intent = ReferenceImportIntent { target: view.writing.setup.as_ref().unwrap().target.clone(), purpose: WritingPurpose::Evidence, key: "b3-revoked-import".into() };
            view.workspace.project_command("b3-import-revoke", ProjectCommand::UnlinkDocument { project: project.clone(), note: f.a.id.clone() }).unwrap();
            assert!(view.complete_writing_reference_import(&intent, "B3 project-retained import", &b3_converted_reference(), &DocumentConversionSettings::default(), window, cx));
            assert!(view.notice.contains("no reference was attached"));
            assert!(view.workspace.writing_setup(&f.a.id, Some(&project)).is_err());
            // The explicitly requested scoped import remains legitimate; no general/other setup fallback occurs.
            let imported = view.workspace.records().unwrap().into_iter().find(|r| matches!(&r.data, RecordData::Note(n) if n.title == "B3 project-retained import")).unwrap();
            assert!(matches!(imported.data, RecordData::Note(n) if n.chat.as_deref() == Some(project.as_str())));
            view.workspace.project_command("b3-import-restore-working-link", ProjectCommand::LinkDocument { project: project.clone(), note: f.a.id.clone() }).unwrap();
            assert!(view.workspace.writing_setup(&f.a.id, Some(&project)).unwrap().references.is_empty());
        });
    }).unwrap();
}

#[gpui_kit::test]
fn b3_sources_used_separates_import_reading_claims_quotes_and_changed_removed_material(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (candidate, source) = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let imported = view
                    .workspace
                    .import_full(
                        "b3-evidence-import",
                        "B3 retained requirements",
                        &b3_converted_reference(),
                    )
                    .unwrap();
                let ApplyOutcome::Applied(receipt) = imported else {
                    panic!("full import")
                };
                let source = receipt
                    .writes
                    .iter()
                    .find(|w| matches!(w.after.data, RecordData::Note(_)))
                    .unwrap()
                    .after
                    .clone();
                view.workspace
                    .set_writing_setup(
                        "b3-review-setup",
                        None,
                        &WritingSetup {
                            note: f.a.id.clone(),
                            chat: None,
                            brief: Some("B3 brief captured once".into()),
                            references: vec![WritingReference {
                                note: source.id.clone(),
                                purpose: WritingPurpose::Requirements,
                            }],
                        },
                    )
                    .unwrap();
                view.open_note(&f.a.id, None, window, cx);
                let snapshot = view
                    .workspace
                    .snapshot_writing_request_with_requirements(
                        "b3-evidence-request",
                        &f.a.id,
                        f.a.version,
                        &f.chat_a,
                        &[],
                        "Use supported passages",
                        &WritingReadRequirements {
                            exhaustive: false,
                            must_be_current: vec![],
                        },
                    )
                    .unwrap();
                let operation = view
                    .workspace
                    .store
                    .allocate_operation("b3-evidence-candidate")
                    .unwrap();
                let mut request = snapshot.change_request("B3 proposed revision").unwrap();
                request.inputs.push(RevisionInput {
                    record: source.id.clone(),
                    version: source.version,
                    expectation: InputExpectation::Historical,
                });
                let RecordData::Note(note) = &source.data else {
                    panic!("source")
                };
                let quote = "Requirements";
                let start = note.markdown.find(quote).unwrap();
                let evidence = WritingCandidateEvidence {
                    version: 1,
                    request_message: snapshot.request_message,
                    thread: f.chat_a.clone(),
                    chat: None,
                    reads: vec![WritingRead {
                        id: source.id.clone(),
                        version: source.version,
                        covered_retained_bytes: start + quote.len(),
                        total_retained_bytes: note.markdown.len(),
                        complete: false,
                    }],
                    quotations: vec![WritingQuotation {
                        id: source.id.clone(),
                        version: source.version,
                        start,
                        end: start + quote.len(),
                        quote: quote.into(),
                    }],
                    requirements_reported: vec![WritingRequirementsClaim {
                        claim: "A model claim with exact quotation provenance".into(),
                        quotations: vec![0],
                    }],
                    explanation: "Model explanation remains distinct".into(),
                };
                request.writes.push(Put {
                    id: operation.creation_id(request.writes.len()),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: f.chat_a.clone(),
                        role: WRITING_EVIDENCE_ROLE.into(),
                        text: serde_json::to_string(&evidence).unwrap(),
                    }),
                });
                let candidate = view.workspace.store.prepare(&operation, &request).unwrap();
                view.refresh();
                view.open_review(candidate.clone(), window, cx);
                assert!(!view.writing.review.inputs_shown());
                (candidate, source)
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("writing-sources-used").visible());
        assert!(
            window
                .find("writing-source-used-0")
                .label()
                .unwrap()
                .contains("Actually used (host-recorded)")
        );
        assert_eq!(
            window.find("writing-requirements-reading").label().unwrap(),
            "Requirements reading: partial or no exhaustive check requested"
        );
        assert!(window.try_find("writing-source-quotation-0").is_some());
        let quotation = window.find("writing-source-quotation-0");
        let RecordData::Note(note) = &source.data else {
            panic!("source")
        };
        assert!(quotation.label().unwrap().contains(&note.title));
        assert!(!quotation.label().unwrap().contains(&source.id));
        assert!(window.try_find("writing-requirement-claim-0").is_some());
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let RecordData::Note(mut note) = source.data.clone() else {
                panic!("source")
            };
            note.markdown.push_str("\nLater owner edit");
            assert!(matches!(
                view.workspace
                    .owner_change(
                        "b3-source-later-edit",
                        &ChangeRequest {
                            reason: "Explicit synthetic owner source edit".into(),
                            writes: vec![Put {
                                id: source.id.clone(),
                                expected_version: Some(source.version),
                                archived: false,
                                data: RecordData::Note(note)
                            }],
                            inputs: vec![]
                        }
                    )
                    .unwrap(),
                ApplyOutcome::Applied(_)
            ));
            let setup = view
                .workspace
                .writing_setup(&f.a.id, None)
                .unwrap()
                .record
                .unwrap();
            view.workspace
                .set_writing_setup(
                    "b3-source-remove-current-setup",
                    Some(setup.version),
                    &WritingSetup {
                        note: f.a.id.clone(),
                        chat: None,
                        brief: Some("B3 newer brief".into()),
                        references: vec![],
                    },
                )
                .unwrap();
            assert_eq!(
                view.workspace.store.prepared(&candidate.operation).unwrap(),
                candidate,
                "setup/reference edits never rewrite old candidate evidence"
            );
            view.refresh();
            view.open_review(candidate.clone(), window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        let source = window
            .find("writing-source-used-0")
            .label()
            .unwrap()
            .to_owned();
        assert!(source.contains("Changed since"));
        assert!(source.contains("Removed from current references"));
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let mut forged = candidate.clone();
            for write in &mut forged.request.writes {
                if let RecordData::Message(message) = &mut write.data
                    && message.role == WRITING_EVIDENCE_ROLE
                {
                    message.text = "malformed evidence must reveal no source/claim/quote".into();
                }
            }
            // Reopening the same operation intentionally keeps the current review;
            // clear only the local view to exercise a new forged display batch.
            view.writing.review.keep().unwrap();
            view.open_review(forged, window, cx);
            assert_eq!(
                view.workspace.store.prepared(&candidate.operation).unwrap(),
                candidate
            );
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("writing-evidence-unavailable").visible());
        assert!(window.try_find("writing-sources-used").is_none());
        assert!(window.try_find("writing-source-quotation-0").is_none());
        assert!(window.try_find("writing-requirement-claim-0").is_none());
    })
    .unwrap();
}

#[gpui_kit::test]
fn b3_configured_requirements_run_uses_real_shared_start_and_recorded_read_ledger(
    cx: &mut TestAppContext,
) {
    use rig::test_utils::{MockCompletionModel, MockStreamEvent};
    let f = fixture(cx);
    let reference = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let ApplyOutcome::Applied(receipt) = view
                    .workspace
                    .import_full(
                        "b3-scripted-import",
                        "B3 partially imported original",
                        &b3_converted_reference(),
                    )
                    .unwrap()
                else {
                    panic!("import")
                };
                let reference = receipt
                    .writes
                    .iter()
                    .find(|w| matches!(w.after.data, RecordData::Note(_)))
                    .unwrap()
                    .after
                    .clone();
                view.workspace
                    .set_writing_setup(
                        "b3-scripted-setup",
                        None,
                        &WritingSetup {
                            note: f.a.id.clone(),
                            chat: None,
                            brief: Some("B3 immutable audience and requirements brief".into()),
                            references: vec![
                                WritingReference {
                                    note: reference.id.clone(),
                                    purpose: WritingPurpose::Requirements,
                                },
                                WritingReference {
                                    note: f.b.id.clone(),
                                    purpose: WritingPurpose::Style,
                                },
                            ],
                        },
                    )
                    .unwrap();
                view.open_note(&f.a.id, None, window, cx);
                view.load_writing_setup(window, cx);
                view.writing
                    .setup
                    .as_mut()
                    .unwrap()
                    .must_be_current
                    .insert(reference.id.clone());
                view.set_chat(ChatTarget::Thread(f.chat_b.clone()), window, cx);
                view.layout.close(TileKind::Chat);
                cx.notify();
                reference
            })
        })
        .unwrap();
    cx.run_until_parked();
    b3_scroll_setup(cx, &f, -1000.);
    cx.update_window(f.window, |_, window, cx| {
        window.click("check-writing-requirements", cx)
    })
    .unwrap();
    cx.run_until_parked();
    let (snapshot, prepared) = cx
        .update_window(f.window, |_, _, cx| {
            f.view.update(cx, |view, _| {
                // Inspect the checked display map only after the actual click's render.
                assert!(view.worker.is_none());
                let record = view
                    .latest_note_writing_request(&f.a.id)
                    .unwrap_or_else(|| panic!("configured request missing: {}", view.notice));
                let snapshot = view.workspace.writing_request(&record.id).unwrap();
                assert_eq!(
                    snapshot.setup.as_ref().unwrap().brief.as_deref(),
                    Some("B3 immutable audience and requirements brief")
                );
                assert!(snapshot.setup.as_ref().unwrap().requirements.exhaustive);
                assert_eq!(
                    snapshot
                        .setup
                        .as_ref()
                        .unwrap()
                        .requirements
                        .must_be_current,
                    vec![reference.id.clone()]
                );
                let mut settings = view.workspace.settings().unwrap();
                settings.provider = "Codex".into();
                settings.model = "gpt-6.1-sol".into();
                settings.effort = "medium".into();
                view.workspace.configure(settings).unwrap();
                let start = view
                    .writing
                    .rewrite_run_start(&view.workspace, &snapshot.request_message, None, vec![])
                    .unwrap();
                (snapshot, view.workspace.prepare_thread_run(start).unwrap())
            })
        })
        .unwrap();
    let RecordData::Note(note) = &reference.data else {
        panic!("reference")
    };
    let quote = "Requirements";
    let start = note.markdown.find(quote).unwrap();
    let model = MockCompletionModel::from_stream_turns([
        vec![
            MockStreamEvent::tool_call(
                "b3-read",
                "read_note",
                serde_json::json!({"id":reference.id,"version":reference.version,"start":0,"max_bytes":131072,"section":false}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::tool_call(
                "b3-prepare",
                "prepare_changes",
                serde_json::json!({"request_key":"b3-scripted-proposal","request":snapshot.change_request("B3 proposed paragraph from actually read retained material").unwrap(),"writing":{"quotations":[{"id":reference.id,"version":reference.version,"start":start,"end":start+quote.len(),"quote":quote}],"requirements_reported":[{"claim":"Model reports a requirement checked; this is not independent proof","quotations":[0]}]}}),
            ),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
        vec![
            MockStreamEvent::text("A proposal is ready; original-import coverage remains partial."),
            MockStreamEvent::final_response_with_total_tokens(1),
        ],
    ]);
    let observed = model.clone();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(brn_ai::threads::run_prepared_scripted(
            prepared.prepared,
            model,
        ))
        .unwrap();
    assert!(matches!(
        result.answer.terminal,
        brn_ai::AiTerminal::Completed
    ));
    let delivered = serde_json::to_string(&observed.requests()).unwrap();
    assert!(delivered.contains("B3 immutable audience and requirements brief"));
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.refresh();
            let candidate = view
                .workspace
                .candidates()
                .unwrap()
                .into_iter()
                .find(|c| {
                    c.request
                        .inputs
                        .iter()
                        .any(|i| i.record == snapshot.request_message)
                })
                .unwrap();
            let review = view
                .workspace
                .writing_candidate_review(candidate.operation.as_str(), None)
                .unwrap();
            assert!(review.complete_over_retained_text);
            let actual = review
                .sources
                .iter()
                .find(|s| s.id == reference.id)
                .unwrap();
            assert!(actual.retained_read.as_ref().unwrap().complete);
            assert_eq!(
                actual
                    .retained_read
                    .as_ref()
                    .unwrap()
                    .covered_retained_bytes,
                note.markdown.len()
            );
            assert_eq!(
                actual
                    .import_coverage
                    .as_ref()
                    .unwrap()
                    .report
                    .as_ref()
                    .unwrap()
                    .status,
                ImportStatus::Partial
            );
            assert!(
                review
                    .sources
                    .iter()
                    .find(|s| s.id == f.b.id)
                    .unwrap()
                    .retained_read
                    .is_none(),
                "attached Style reference is not labeled used/read"
            );
            assert_eq!(review.host_verified_quotations[0].quote, quote);
            assert_eq!(review.requirements_reported.len(), 1);
            assert_eq!(view.workspace.store.note(&f.a.id).unwrap().unwrap(), f.a);
            view.open_review(candidate, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    let index = cx.read(|cx| {
        f.view
            .read(cx)
            .writing_reviews
            .values()
            .next()
            .unwrap()
            .sources
            .iter()
            .position(|s| s.id == reference.id)
            .unwrap()
    });
    cx.update_window(f.window, |_, window, _| {
        assert_eq!(
            window.find("writing-requirements-reading").label().unwrap(),
            "Requirements reading: complete over retained text"
        );
        assert!(
            window
                .find(format!("writing-source-reading-{index}"))
                .label()
                .unwrap()
                .contains("Read in full")
        );
        assert!(
            window
                .find(format!("writing-source-import-{index}"))
                .label()
                .unwrap()
                .contains("Partial")
        );
        assert!(window.try_find("writing-requirement-claim-0").is_some());
    })
    .unwrap();
}

#[gpui_kit::test]
fn b3_exhaustive_check_refuses_unavailable_requirements_before_new_saved_request(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.workspace
                .set_writing_setup(
                    "b3-unavailable-setup",
                    None,
                    &WritingSetup {
                        note: f.a.id.clone(),
                        chat: None,
                        brief: None,
                        references: vec![WritingReference {
                            note: f.b.id.clone(),
                            purpose: WritingPurpose::Requirements,
                        }],
                    },
                )
                .unwrap();
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            view.workspace
                .owner_change(
                    "b3-unavailable-reference",
                    &ChangeRequest {
                        reason: "Synthetic reference archived".into(),
                        writes: vec![Put {
                            id: f.b.id.clone(),
                            expected_version: Some(f.b.version),
                            archived: true,
                            data: f.b.data.clone(),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            view.load_writing_setup(window, cx);
            assert!(
                !view
                    .writing
                    .setup
                    .as_ref()
                    .unwrap()
                    .labels
                    .contains_key(&f.b.id)
            );
            let before = (
                view.workspace.records().unwrap(),
                view.workspace.store.receipts().unwrap(),
            );
            view.rewrite_with_setup(true, cx);
            assert!(
                view.notice
                    .contains("Requirements reference is no longer available here")
            );
            assert!(view.worker.is_none());
            assert_eq!(view.workspace.records().unwrap(), before.0);
            assert_eq!(view.workspace.store.receipts().unwrap(), before.1);
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn b3_saved_host_evidence_has_visible_sources_used_without_opening_inputs(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.workspace
                .set_writing_setup(
                    "b3-before-ui-setup",
                    None,
                    &WritingSetup {
                        note: f.a.id.clone(),
                        chat: None,
                        brief: Some("Retained writing brief".into()),
                        references: vec![WritingReference {
                            note: f.b.id.clone(),
                            purpose: WritingPurpose::Requirements,
                        }],
                    },
                )
                .unwrap();
            let snapshot = view
                .workspace
                .snapshot_writing_request(
                    "b3-before-ui-request",
                    &f.a.id,
                    f.a.version,
                    &f.chat_a,
                    &[],
                )
                .unwrap();
            let operation = view
                .workspace
                .store
                .allocate_operation("b3-before-ui-proposal")
                .unwrap();
            let mut request = snapshot
                .change_request("A reviewable proposed revision")
                .unwrap();
            request.inputs.push(RevisionInput {
                record: f.b.id.clone(),
                version: f.b.version,
                expectation: InputExpectation::Historical,
            });
            let RecordData::Note(note) = &f.b.data else {
                panic!("reference")
            };
            let proof = WritingCandidateEvidence {
                version: 1,
                request_message: snapshot.request_message,
                thread: f.chat_a.clone(),
                chat: None,
                reads: vec![WritingRead {
                    id: f.b.id.clone(),
                    version: f.b.version,
                    covered_retained_bytes: 6,
                    total_retained_bytes: note.markdown.len(),
                    complete: false,
                }],
                quotations: vec![WritingQuotation {
                    id: f.b.id.clone(),
                    version: f.b.version,
                    start: 0,
                    end: 6,
                    quote: "Second".into(),
                }],
                requirements_reported: vec![],
                explanation: String::new(),
            };
            request.writes.push(Put {
                id: operation.creation_id(request.writes.len()),
                expected_version: None,
                archived: false,
                data: RecordData::Message(Message {
                    thread: f.chat_a.clone(),
                    role: WRITING_EVIDENCE_ROLE.into(),
                    text: serde_json::to_string(&proof).unwrap(),
                }),
            });
            let candidate = view.workspace.store.prepare(&operation, &request).unwrap();
            view.open_note(&f.a.id, None, window, cx);
            view.refresh();
            view.open_review(candidate, window, cx);
            assert!(!view.writing.review.inputs_shown());
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(
            window.try_find("writing-sources-used").is_some(),
            "saved host-recorded evidence needs a Sources used pane outside Inputs"
        );
        assert!(
            window
                .find("writing-source-reading-0")
                .label()
                .unwrap()
                .contains("Partially read")
        );
    })
    .unwrap();
}

#[gpui_kit::test]
fn b3_cancelled_reference_conversion_never_imports_or_attaches_and_clears_intent(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            view.reference_import = Some(ReferenceImportIntent {
                target: view.writing.setup.as_ref().unwrap().target.clone(),
                purpose: WritingPurpose::Evidence,
                key: "b3-cancelled-reference".into(),
            });
            let relevant = |workspace: &Workspace| {
                workspace
                    .records()
                    .unwrap()
                    .into_iter()
                    .filter(|r| {
                        matches!(
                            r.data,
                            RecordData::Note(_)
                                | RecordData::Source(_)
                                | RecordData::Asset(_)
                                | RecordData::WritingSetup(_)
                        )
                    })
                    .collect::<Vec<_>>()
            };
            let before = relevant(&view.workspace);
            let (sender, receiver) = mpsc::sync_channel(4);
            view.worker = Some(Worker {
                receiver,
                cancel: CancellationToken::new(),
                conversion_cancel: Arc::new(AtomicBool::new(false)),
                kind: WorkerKind::Import,
                thread: None,
                run_id: None,
            });
            sender
                .send(WorkerEvent::Converted {
                    settings: DocumentConversionSettings::default(),
                    useful_project: None,
                    result: Box::new(Ok(b3_converted_reference())),
                    title: "B3 cancelled canonical reference".into(),
                    chat: None,
                })
                .unwrap();
            view.cancel(cx);
            view.poll_worker(window, cx);
            assert!(view.worker.is_none());
            assert!(view.reference_import.is_none());
            assert!(view.notice.contains("stopped before saving"));
            assert_eq!(relevant(&view.workspace), before);
            assert!(
                view.workspace
                    .writing_setup(&f.a.id, None)
                    .unwrap()
                    .references
                    .is_empty()
            );
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn b3_delayed_scoped_search_reply_cannot_publish_titles_or_counts_into_another_form(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            let target = view.writing.setup.as_ref().unwrap().target.clone();
            let observed = view.writing.setup.as_ref().unwrap().observed.clone();
            view.reference_search_generation += 1;
            let generation = view.reference_search_generation;
            let page = view
                .workspace
                .search_handle()
                .unwrap()
                .request_page("Second saved paragraph", None, 12, None)
                .wait()
                .unwrap();
            assert!(!page.matches.is_empty());
            view.open_note(&f.b.id, None, window, cx);
            view.load_writing_setup(window, cx);
            view.finish_reference_search(generation, target, observed, Ok(page), cx);
            assert!(view.writing.setup.as_ref().unwrap().matches.is_empty());
            assert_eq!(view.writing.setup.as_ref().unwrap().match_total, 0);

            // Same target/generation, but canonical visibility/configuration changed before reply delivery.
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            let target = view.writing.setup.as_ref().unwrap().target.clone();
            let observed = view.writing.setup.as_ref().unwrap().observed.clone();
            view.reference_search_generation += 1;
            let generation = view.reference_search_generation;
            let page = view
                .workspace
                .search_handle()
                .unwrap()
                .request_page("Second saved paragraph", None, 12, None)
                .wait()
                .unwrap();
            view.workspace
                .owner_change(
                    "b3-search-access-loss",
                    &ChangeRequest {
                        reason: "Synthetic access loss before search reply".into(),
                        writes: vec![Put {
                            id: f.b.id.clone(),
                            expected_version: Some(f.b.version),
                            archived: true,
                            data: f.b.data.clone(),
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            view.finish_reference_search(generation, target, observed, Ok(page), cx);
            assert!(!view.writing_setup_form_current());
            assert!(view.writing.setup.as_ref().unwrap().matches.is_empty());
            assert_eq!(view.writing.setup.as_ref().unwrap().match_total, 0);
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn b3_reimport_same_bytes_attaches_existing_canonical_note_without_a_duplicate(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let converted = b3_converted_reference();
            let ApplyOutcome::Applied(first) = view.workspace.import_full_scoped_with_conversion("b3-preexisting-full-note", "B3 preexisting canonical reference", &converted, None, &DocumentConversionSettings::default()).unwrap() else { panic!("import") };
            let note = first.writes.iter().find(|w| matches!(w.after.data, RecordData::Note(_))).unwrap().after.clone();
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            let intent = ReferenceImportIntent { target: view.writing.setup.as_ref().unwrap().target.clone(), purpose: WritingPurpose::Evidence, key: "b3-observation-only-reimport".into() };
            assert!(view.complete_writing_reference_import(&intent, "B3 preexisting canonical reference", &converted, &DocumentConversionSettings::default(), window, cx));
            assert_eq!(view.workspace.writing_setup(&f.a.id, None).unwrap().references[0].reference.note, note.id);
            assert_eq!(view.workspace.store.note(&note.id).unwrap().unwrap(), note, "re-import observation retains legitimate canonical content/version");
            assert_eq!(view.workspace.records().unwrap().iter().filter(|r| matches!(&r.data, RecordData::Note(n) if n.import.as_ref().is_some_and(|i| i.source == match &note.data { RecordData::Note(n) => n.import.as_ref().unwrap().source.clone(), _ => unreachable!() }))).count(), 1);
            // Replay the actual observation-only receipt after its canonical Note becomes unavailable.
            // A new import key would legitimately create a fresh Note instead.
            let current = view.workspace.store.note(&note.id).unwrap().unwrap();
            view.workspace.owner_change("b3-observation-archive", &ChangeRequest { reason: "Synthetic archived imported Note".into(), writes: vec![Put { id: current.id.clone(), expected_version: Some(current.version), archived: true, data: current.data.clone() }], inputs: vec![] }).unwrap();
            view.load_writing_setup(window, cx);
            let missing = ReferenceImportIntent { target: view.writing.setup.as_ref().unwrap().target.clone(), purpose: WritingPurpose::Evidence, key: intent.key.clone() };
            let saved = view.workspace.writing_setup(&f.a.id, None).unwrap().record;
            assert!(view.complete_writing_reference_import(&missing, "B3 preexisting canonical reference", &converted, &DocumentConversionSettings::default(), window, cx));
            assert!(view.notice.contains("No eligible canonical full Note was resolved"));
            assert!(!view.notice.contains("Imported Note retained"));
            assert_eq!(view.workspace.writing_setup(&f.a.id, None).unwrap().record, saved);
            let archived = view.workspace.store.note(&note.id).unwrap().unwrap();
            view.workspace.owner_change("b3-observation-restore", &ChangeRequest { reason: "Synthetic restore".into(), writes: vec![Put { id: archived.id.clone(), expected_version: Some(archived.version), archived: false, data: archived.data }], inputs: vec![] }).unwrap();
            view.open_note(&note.id, None, window, cx);
            view.load_writing_setup(window, cx);
            let self_reference = ReferenceImportIntent { target: view.writing.setup.as_ref().unwrap().target.clone(), purpose: WritingPurpose::Evidence, key: "b3-observation-self-reference".into() };
            assert!(view.complete_writing_reference_import(&self_reference, "B3 preexisting canonical reference", &converted, &DocumentConversionSettings::default(), window, cx));
            assert!(view.notice.contains("cannot be its own reference"));
            assert!(!view.notice.contains("Target, saved Note, setup or access changed"));
            assert!(view.workspace.writing_setup(&note.id, None).unwrap().record.is_none());
        });
    }).unwrap();
}

#[gpui_kit::test]
fn b3_dirty_setup_survives_converted_completion_and_rechecks_without_retargeting(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let (sender, receiver) = mpsc::sync_channel(4);
    let (saved, project) = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let removed = view
                    .workspace
                    .new_note("Remove during conversion", "Saved reference")
                    .unwrap();
                let pending = view
                    .workspace
                    .new_note("Pending during conversion", "Pending reference")
                    .unwrap();
                let project = view
                    .workspace
                    .project_command(
                        "b3-dirty-project",
                        ProjectCommand::Create {
                            title: "Other writing scope".into(),
                        },
                    )
                    .unwrap()
                    .writes[0]
                    .after
                    .id
                    .clone();
                view.workspace
                    .project_command(
                        "b3-dirty-project-link",
                        ProjectCommand::LinkDocument {
                            project: project.clone(),
                            note: f.a.id.clone(),
                        },
                    )
                    .unwrap();
                view.workspace
                    .set_writing_setup(
                        "b3-dirty-initial",
                        None,
                        &WritingSetup {
                            note: f.a.id.clone(),
                            chat: None,
                            brief: Some("Saved before conversion".into()),
                            references: vec![
                                WritingReference {
                                    note: f.b.id.clone(),
                                    purpose: WritingPurpose::Style,
                                },
                                WritingReference {
                                    note: removed.id,
                                    purpose: WritingPurpose::Evidence,
                                },
                            ],
                        },
                    )
                    .unwrap();
                view.open_note(&f.a.id, None, window, cx);
                view.load_writing_setup(window, cx);
                let page = view
                    .workspace
                    .search_handle()
                    .unwrap()
                    .request_page("Pending during conversion", None, 12, None)
                    .wait()
                    .unwrap();
                assert_eq!(page.matches[0].note, pending.id);
                view.writing.setup.as_mut().unwrap().matches = page.matches;
                view.reference_import = Some(ReferenceImportIntent {
                    target: view.writing.setup.as_ref().unwrap().target.clone(),
                    purpose: WritingPurpose::Evidence,
                    key: "b3-dirty-conversion".into(),
                });
                view.worker = Some(Worker {
                    receiver,
                    cancel: CancellationToken::new(),
                    conversion_cancel: Arc::new(AtomicBool::new(false)),
                    kind: WorkerKind::Import,
                    thread: None,
                    run_id: None,
                });
                view.writing
                    .setup_brief
                    .read(cx)
                    .focus_handle(cx)
                    .focus(window, cx);
                cx.notify();
                (
                    view.workspace
                        .writing_setup(&f.a.id, None)
                        .unwrap()
                        .record
                        .unwrap(),
                    project,
                )
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        window.press(super::focus_tests::SELECT_ALL, cx);
        window.input("Owner typed this brief during conversion", cx);
    })
    .unwrap();
    cx.run_until_parked();
    for id in [
        "reference-purpose-0-Requirements",
        "remove-reference-1",
        "attach-writing-reference-0",
    ] {
        b3_reveal_setup_control(cx, &f, id);
        cx.update_window(f.window, |_, window, cx| window.click(id, cx))
            .unwrap();
        cx.run_until_parked();
    }
    let pending = cx.read(|cx| {
        f.view
            .read(cx)
            .writing
            .setup
            .as_ref()
            .unwrap()
            .references
            .clone()
    });
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].purpose, WritingPurpose::Requirements);
    sender
        .send(WorkerEvent::Converted {
            settings: DocumentConversionSettings::default(),
            useful_project: None,
            result: Box::new(Ok(b3_converted_reference())),
            title: "B3 dirty canonical import".into(),
            chat: None,
        })
        .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.poll_worker(window, cx);
            assert!(view.worker.is_none());
            assert_eq!(
                view.writing.setup_brief.read(cx).value().as_ref(),
                "Owner typed this brief during conversion",
                "Converted completion must not discard owner-entered brief text"
            );
            assert_eq!(
                view.writing.setup.as_ref().unwrap().references,
                pending,
                "pending purpose, removal and attachment edits survive completion"
            );
            assert_eq!(
                view.workspace
                    .writing_setup(&f.a.id, None)
                    .unwrap()
                    .record
                    .unwrap(),
                saved,
                "dirty completion must not attach or overwrite setup"
            );
            assert!(view.workspace.notes().unwrap().iter().any(
                |r| matches!(&r.data, RecordData::Note(n) if n.title == "B3 dirty canonical import")
            ));
            assert!(view.notice.to_lowercase().contains("unsaved"));
            view.workspace
                .post_message(&f.chat_b, "Unrelated canonical write while dirty")
                .unwrap();
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(
            window.try_find("writing-setup-brief").is_none(),
            "stale labels and brief remain hidden until a fresh context check"
        );
        assert!(window.try_find("writing-reference-0").is_none());
    })
    .unwrap();
    b3_reveal_setup_control(cx, &f, "recheck-writing-setup");
    cx.update_window(f.window, |_, window, cx| {
        window.click("recheck-writing-setup", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("writing-setup-brief").is_some());
        f.view.update(cx, |view, cx| {
            assert_eq!(
                view.writing.setup_brief.read(cx).value().as_ref(),
                "Owner typed this brief during conversion"
            );
            assert_eq!(view.writing.setup.as_ref().unwrap().references, pending);
            view.open_note_in_context(&f.a.id, None, Some(&project), window, cx);
            view.load_writing_setup(window, cx);
            assert_eq!(view.writing.setup.as_ref().unwrap().references, pending);
            assert!(
                view.workspace
                    .writing_setup(&f.a.id, Some(&project))
                    .unwrap()
                    .record
                    .is_none()
            );
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-setup-brief").is_none());
        assert!(window.try_find("writing-reference-0").is_none());
        assert!(window.try_find("save-writing-setup").is_none());
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            let current = view.workspace.store.note(&f.a.id).unwrap().unwrap();
            view.workspace
                .owner_change(
                    "b3-dirty-revoke",
                    &ChangeRequest {
                        reason: "Synthetic working Note access loss".into(),
                        writes: vec![Put {
                            id: current.id,
                            expected_version: Some(current.version),
                            archived: true,
                            data: current.data,
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            view.load_writing_setup(window, cx);
            assert_eq!(
                view.writing.setup_brief.read(cx).value().as_ref(),
                "Owner typed this brief during conversion"
            );
            assert_eq!(view.writing.setup.as_ref().unwrap().references, pending);
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-setup-brief").is_none());
        assert!(window.try_find("writing-reference-0").is_none());
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let current = view.workspace.store.note(&f.a.id).unwrap().unwrap();
            view.workspace
                .owner_change(
                    "b3-dirty-restore",
                    &ChangeRequest {
                        reason: "Synthetic working Note access restore".into(),
                        writes: vec![Put {
                            id: current.id,
                            expected_version: Some(current.version),
                            archived: false,
                            data: current.data,
                        }],
                        inputs: vec![],
                    },
                )
                .unwrap();
            view.open_note(&f.a.id, None, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "recheck-writing-setup");
    cx.update_window(f.window, |_, window, cx| {
        window.click("recheck-writing-setup", cx)
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "save-writing-setup");
    cx.update_window(f.window, |_, window, cx| {
        window.click("save-writing-setup", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let config = view
                .workspace
                .writing_setup(&f.a.id, None)
                .unwrap()
                .record
                .unwrap();
            let RecordData::WritingSetup(value) = config.data else {
                panic!("setup")
            };
            assert_eq!(
                value.brief.as_deref(),
                Some("Owner typed this brief during conversion")
            );
            assert_eq!(value.references, pending);
            assert_eq!(
                view.workspace
                    .writing_setup(&f.a.id, Some(&project))
                    .unwrap()
                    .record,
                None
            );
            view.writing
                .setup_brief
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
        });
        window.press(super::focus_tests::SELECT_ALL, cx);
        window.input("Explicitly discard this later draft", cx);
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "reload-writing-setup");
    cx.update_window(f.window, |_, window, cx| {
        assert!(
            window
                .find("reload-writing-setup")
                .label()
                .unwrap()
                .contains("discard"),
            "owner sees discard semantics before clicking"
        );
        window.click("reload-writing-setup", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.writing.setup_brief.read(cx).value().as_ref(),
            "Owner typed this brief during conversion"
        );
        assert_eq!(view.writing.setup.as_ref().unwrap().references, pending);
        assert!(view.notice.to_lowercase().contains("discard"));
    });
    // Recheck exposes the retained form, never permission to overwrite a newer setup.
    let newer = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                view.writing
                    .setup_brief
                    .read(cx)
                    .focus_handle(cx)
                    .focus(window, cx);
            });
            window.press(super::focus_tests::SELECT_ALL, cx);
            window.input("Keep this draft after a concurrent setup edit", cx);
            f.view.update(cx, |view, cx| {
                let current = view
                    .workspace
                    .writing_setup(&f.a.id, None)
                    .unwrap()
                    .record
                    .unwrap();
                let RecordData::WritingSetup(mut value) = current.data else {
                    panic!("setup")
                };
                value.brief = Some("Newer saved owner setup".into());
                let receipt = view
                    .workspace
                    .set_writing_setup("b3-dirty-concurrent", Some(current.version), &value)
                    .unwrap();
                cx.notify();
                receipt
            })
        })
        .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "recheck-writing-setup");
    cx.update_window(f.window, |_, window, cx| {
        window.click("recheck-writing-setup", cx)
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "save-writing-setup");
    let receipts = cx.read(|cx| f.view.read(cx).workspace.store.receipts().unwrap().len());
    cx.update_window(f.window, |_, window, cx| {
        window.click("save-writing-setup", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.writing.setup_brief.read(cx).value().as_ref(),
            "Keep this draft after a concurrent setup edit"
        );
        assert_eq!(view.workspace.store.receipts().unwrap().len(), receipts);
        assert_eq!(
            view.workspace
                .writing_setup(&f.a.id, None)
                .unwrap()
                .record
                .unwrap(),
            newer.writes[0].after
        );
        assert!(view.notice.contains("changed"));
    });
}

#[gpui_kit::test]
async fn b3_reference_next_keeps_its_query_until_owner_starts_a_new_search(
    cx: &mut TestAppContext,
) {
    use gpui_kit::test::TestAppContextExt;
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            for index in 0..14 {
                view.workspace
                    .new_note(
                        &format!("PagingAlpha reference {index:02}"),
                        "Alpha retained material",
                    )
                    .unwrap();
            }
            for index in 0..3 {
                view.workspace
                    .new_note(
                        &format!("PagingBeta reference {index:02}"),
                        "Beta retained material",
                    )
                    .unwrap();
            }
            view.open_note(&f.a.id, None, window, cx);
            view.load_writing_setup(window, cx);
            view.writing
                .reference_query
                .update(cx, |input, cx| input.set_value("PagingAlpha", window, cx));
        });
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "find-writing-reference");
    cx.update_window(f.window, |_, window, cx| {
        window.click("find-writing-reference", cx)
    })
    .unwrap();
    cx.wait_for(f.window, Duration::from_secs(3), |_, cx| {
        !f.view
            .read(cx)
            .writing
            .setup
            .as_ref()
            .unwrap()
            .search_pending
    })
    .await;
    cx.read(|cx| {
        let draft = f.view.read(cx).writing.setup.as_ref().unwrap();
        assert_eq!(draft.matches.len(), 12);
        assert!(draft.next.is_some());
        assert!(
            draft
                .matches
                .iter()
                .all(|hit| hit.title.starts_with("PagingAlpha"))
        );
    });
    cx.update_window(f.window, |_, window, cx| {
        f.view
            .read(cx)
            .writing
            .reference_query
            .clone()
            .update(cx, |input, cx| input.focus(window, cx));
        window.press(super::focus_tests::SELECT_ALL, cx);
        window.input("PagingBeta", cx);
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "next-writing-references");
    cx.update_window(f.window, |_, window, cx| {
        window.click("next-writing-references", cx)
    })
    .unwrap();
    cx.wait_for(f.window, Duration::from_secs(3), |_, cx| {
        !f.view
            .read(cx)
            .writing
            .setup
            .as_ref()
            .unwrap()
            .search_pending
    })
    .await;
    cx.read(|cx| {
        let draft = f.view.read(cx).writing.setup.as_ref().unwrap();
        assert_eq!(
            draft.matches.len(),
            2,
            "Next continues Alpha instead of skipping Beta's first page"
        );
        assert!(
            draft
                .matches
                .iter()
                .all(|hit| hit.title.starts_with("PagingAlpha"))
        );
    });
    b3_reveal_setup_control(cx, &f, "find-writing-reference");
    cx.update_window(f.window, |_, window, cx| {
        window.click("find-writing-reference", cx)
    })
    .unwrap();
    cx.wait_for(f.window, Duration::from_secs(3), |_, cx| {
        !f.view
            .read(cx)
            .writing
            .setup
            .as_ref()
            .unwrap()
            .search_pending
    })
    .await;
    cx.read(|cx| {
        let draft = f.view.read(cx).writing.setup.as_ref().unwrap();
        assert_eq!(draft.matches.len(), 3);
        assert!(draft.next.is_none());
        assert!(
            draft
                .matches
                .iter()
                .all(|hit| hit.title.starts_with("PagingBeta"))
        );
    });
}

#[gpui_kit::test]
fn b3_import_completion_keeps_another_notes_hidden_dirty_setup(cx: &mut TestAppContext) {
    let f = fixture(cx);
    let (sender, receiver) = mpsc::sync_channel(4);
    let (saved_a, saved_b) = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let pending = view
                    .workspace
                    .new_note("Hidden B pending reference", "Retained pending material")
                    .unwrap();
                for (key, note, brief, references) in [
                    ("b3-other-dirty-a", &f.a, "Clean A setup", vec![]),
                    (
                        "b3-other-dirty-b",
                        &f.b,
                        "Saved B brief",
                        vec![WritingReference {
                            note: f.a.id.clone(),
                            purpose: WritingPurpose::Style,
                        }],
                    ),
                ] {
                    view.workspace
                        .set_writing_setup(
                            key,
                            None,
                            &WritingSetup {
                                note: note.id.clone(),
                                chat: None,
                                brief: Some(brief.into()),
                                references,
                            },
                        )
                        .unwrap();
                }
                view.open_note(&f.a.id, None, window, cx);
                view.load_writing_setup(window, cx);
                assert!(!view.writing_setup_dirty(cx));
                // The existing picker/worker contract is injected; conversion itself is synthetic.
                view.reference_import = Some(ReferenceImportIntent {
                    target: view.writing.setup.as_ref().unwrap().target.clone(),
                    purpose: WritingPurpose::Evidence,
                    key: "b3-other-dirty-converted".into(),
                });
                view.worker = Some(Worker {
                    receiver,
                    cancel: CancellationToken::new(),
                    conversion_cancel: Arc::new(AtomicBool::new(false)),
                    kind: WorkerKind::Import,
                    thread: None,
                    run_id: None,
                });
                view.open_note(&f.b.id, None, window, cx);
                view.load_writing_setup(window, cx);
                assert_eq!(view.writing.setup.as_ref().unwrap().target.note, f.b.id);
                let page = view
                    .workspace
                    .search_handle()
                    .unwrap()
                    .request_page("Hidden B pending reference", None, 12, None)
                    .wait()
                    .unwrap();
                assert_eq!(page.matches[0].note, pending.id);
                view.writing.setup.as_mut().unwrap().matches = page.matches;
                view.writing
                    .setup_brief
                    .read(cx)
                    .focus_handle(cx)
                    .focus(window, cx);
                cx.notify();
                (
                    view.workspace.writing_setup(&f.a.id, None).unwrap().record,
                    view.workspace.writing_setup(&f.b.id, None).unwrap().record,
                )
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        window.press(super::focus_tests::SELECT_ALL, cx);
        window.input("Private B brief typed during A conversion", cx);
    })
    .unwrap();
    cx.run_until_parked();
    for id in [
        "reference-purpose-0-Requirements",
        "attach-writing-reference-0",
    ] {
        b3_reveal_setup_control(cx, &f, id);
        cx.update_window(f.window, |_, window, cx| window.click(id, cx))
            .unwrap();
        cx.run_until_parked();
    }
    let pending = cx.read(|cx| {
        let view = f.view.read(cx);
        let draft = view.writing.setup.as_ref().unwrap();
        assert_eq!(draft.references.len(), 2);
        assert_eq!(draft.references[0].purpose, WritingPurpose::Requirements);
        draft.references.clone()
    });
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.a.id, None, window, cx);
            assert!(!view.writing_setup_form_current());
            assert!(view.writing_setup_dirty(cx));
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-setup-brief").is_none());
        assert!(window.try_find("writing-reference-0").is_none());
        assert!(window.try_find("save-writing-setup").is_none());
    })
    .unwrap();
    sender
        .send(WorkerEvent::Converted {
            settings: DocumentConversionSettings::default(),
            useful_project: None,
            result: Box::new(Ok(b3_converted_reference())),
            title: "A conversion retained without attaching".into(),
            chat: None,
        })
        .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.poll_worker(window, cx);
            assert!(view.worker.is_none());
            assert_eq!(
                view.writing.setup_brief.read(cx).value().as_ref(),
                "Private B brief typed during A conversion",
                "A completion must retain the other Note's hidden dirty brief"
            );
            let draft = view.writing.setup.as_ref().unwrap();
            assert_eq!(draft.target.note, f.b.id);
            assert_eq!(draft.target.scope, None);
            assert_eq!(draft.references, pending);
            assert_eq!(view.workspace.writing_setup(&f.a.id, None).unwrap().record, saved_a);
            assert_eq!(view.workspace.writing_setup(&f.b.id, None).unwrap().record, saved_b);
            assert!(view.workspace.notes().unwrap().iter().any(|r| {
                matches!(&r.data, RecordData::Note(n) if n.title == "A conversion retained without attaching")
            }));
            assert!(view.notice.contains("no reference was attached"));
            for private in [
                "Note B",
                "Private B brief typed during A conversion",
                "Hidden B pending reference",
            ] {
                assert!(!view.notice.contains(private));
            }
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-setup-brief").is_none());
        assert!(window.try_find("writing-reference-0").is_none());
        assert!(window.try_find("attach-writing-reference-0").is_none());
        assert!(window.try_find("save-writing-setup").is_none());
    })
    .unwrap();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.b.id, None, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    for id in ["recheck-writing-setup", "save-writing-setup"] {
        b3_reveal_setup_control(cx, &f, id);
        cx.update_window(f.window, |_, window, cx| window.click(id, cx))
            .unwrap();
        cx.run_until_parked();
    }
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let RecordData::WritingSetup(saved) = view
                .workspace
                .writing_setup(&f.b.id, None)
                .unwrap()
                .record
                .unwrap()
                .data
            else {
                panic!("B saved setup")
            };
            assert_eq!(
                saved.brief.as_deref(),
                Some("Private B brief typed during A conversion")
            );
            assert_eq!(saved.references, pending);
            assert_eq!(
                view.workspace.writing_setup(&f.a.id, None).unwrap().record,
                saved_a
            );
            view.writing
                .setup_brief
                .read(cx)
                .focus_handle(cx)
                .focus(window, cx);
        });
        window.press(super::focus_tests::SELECT_ALL, cx);
        window.input("B owner explicitly discards this later edit", cx);
    })
    .unwrap();
    cx.run_until_parked();
    b3_reveal_setup_control(cx, &f, "reload-writing-setup");
    cx.update_window(f.window, |_, window, cx| {
        assert!(
            window
                .find("reload-writing-setup")
                .label()
                .unwrap()
                .contains("discard")
        );
        window.click("reload-writing-setup", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.writing.setup_brief.read(cx).value().as_ref(),
            "Private B brief typed during A conversion"
        );
        assert_eq!(view.writing.setup.as_ref().unwrap().target.note, f.b.id);
        assert_eq!(view.writing.setup.as_ref().unwrap().references, pending);
    });
}
/// A general-looking Note may still be private through its retained Source.
/// Scoped lists must use canonical visibility, including title/count metadata.
#[gpui_kit::test]
fn int_w_general_notes_list_hides_p1_reference_with_scoped_retained_source(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let p2 = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let mut projects = vec![];
                for key in ["int-w-list-p1", "int-w-list-p2"] {
                    let project = view
                        .workspace
                        .project_command(key, ProjectCommand::Create { title: key.into() })
                        .unwrap()
                        .writes[0]
                        .after
                        .id
                        .clone();
                    view.workspace
                        .project_command(
                            &format!("{key}-working"),
                            ProjectCommand::LinkDocument {
                                project: project.clone(),
                                note: f.a.id.clone(),
                            },
                        )
                        .unwrap();
                    projects.push(project);
                }
                let op = view
                    .workspace
                    .store
                    .allocate_operation("int-w-list-private-source")
                    .unwrap();
                let source = op.creation_id(0);
                let RecordData::Note(mut reference) = f.b.data.clone() else {
                    panic!("Note")
                };
                reference.title = "INTW PRIVATE RETAINED REFERENCE TITLE".into();
                reference.markdown = "INTW PRIVATE RETAINED REFERENCE BODY".into();
                assert!(
                    reference.chat.is_none(),
                    "retained Source, not Note.chat, defines this scope edge"
                );
                reference.import = Some(ImportCoverage {
                    source: source.clone(),
                    full_note: false,
                    gaps: vec![],
                    report: None,
                });
                assert!(matches!(
                    view.workspace
                        .owner_change(
                            "int-w-list-private-source",
                            &ChangeRequest {
                                reason: "Synthetic P1-only retained reference".into(),
                                writes: vec![
                                    Put {
                                        id: source,
                                        expected_version: None,
                                        archived: false,
                                        data: RecordData::Source(Source {
                                            chat: Some(projects[0].clone()),
                                            locator: "synthetic:int-w-private".into(),
                                            label: "INTW PRIVATE SOURCE LABEL".into(),
                                            observed_at: "synthetic".into(),
                                            external_version: None,
                                            outcome: "Retained synthetic evidence".into(),
                                            gaps: vec![],
                                            report: None
                                        })
                                    },
                                    Put {
                                        id: f.b.id.clone(),
                                        expected_version: Some(f.b.version),
                                        archived: false,
                                        data: RecordData::Note(reference)
                                    },
                                    Put {
                                        id: op.creation_id(1),
                                        expected_version: None,
                                        archived: false,
                                        data: RecordData::Link(Link {
                                            from: f.b.id.clone(),
                                            to: f.a.id.clone(),
                                            relation: "evidence".into(),
                                        }),
                                    },
                                ],
                                inputs: vec![],
                            }
                        )
                        .unwrap(),
                    ApplyOutcome::Applied(_)
                ));
                view.workspace
                    .project_command(
                        "int-w-list-link-reference",
                        ProjectCommand::LinkDocument {
                            project: projects[0].clone(),
                            note: f.b.id.clone(),
                        },
                    )
                    .unwrap();
                view.workspace
                    .set_writing_setup(
                        "int-w-list-setup",
                        None,
                        &WritingSetup {
                            note: f.a.id.clone(),
                            chat: Some(projects[0].clone()),
                            brief: Some("INTW PRIVATE LIST BRIEF".into()),
                            references: vec![WritingReference {
                                note: f.b.id.clone(),
                                purpose: WritingPurpose::Evidence,
                            }],
                        },
                    )
                    .unwrap();
                assert!(
                    view.workspace
                        .store
                        .record_visible_in_chat(&f.b.id, Some(&projects[0]))
                        .unwrap()
                );
                assert!(
                    !view
                        .workspace
                        .store
                        .record_visible_in_chat(&f.b.id, None)
                        .unwrap()
                );
                assert!(
                    !view
                        .workspace
                        .store
                        .record_visible_in_chat(&f.b.id, Some(&projects[1]))
                        .unwrap()
                );
                view.refresh();
                view.open_note_in_context(&f.a.id, None, None, window, cx);
                focus_tests::solo(view, TileKind::Notes, cx);
                projects[1].clone()
            })
        })
        .unwrap();
    cx.run_until_parked();
    for displayed in [None, Some(p2.as_str())] {
        cx.update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                view.open_note_in_context(&f.a.id, None, displayed, window, cx);
                focus_tests::solo(view, TileKind::Notes, cx);
            });
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(f.window, |_, window, _| {
            assert!(
                window
                    .try_find(format!("notes-recent-{}", f.a.id))
                    .is_some(),
                "visible general Note remains a real rendered positive control"
            );
            assert!(
                window
                    .try_find(format!("notes-recent-{}", f.b.id))
                    .is_none(),
                "general Notes UI must not reveal a P1-private retained-source reference title"
            );
            assert!(window.try_find(format!("notes-kept-{}", f.b.id)).is_none());
        })
        .unwrap();
        cx.read(|cx| {
            assert!(
                f.view.read(cx).note_rows.iter().all(|row| row.id != f.b.id),
                "general Notes count/title rows must exclude the hidden reference"
            );
            assert_eq!(
                f.view
                    .read(cx)
                    .note_rows
                    .iter()
                    .find(|row| row.id == f.a.id)
                    .unwrap()
                    .inbound,
                0,
                "a hidden reference must not contribute an inbound count to a visible Note"
            )
        });
    }
}
