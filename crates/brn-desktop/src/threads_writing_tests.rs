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
            view.comment_anchor = Some((note.id.clone(), note.version, 5..9, "tere".into()));
            view.comment_input.update(cx, |editor, cx| {
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
    view.comment_anchor = Some((note.id.clone(), note.version, range, quote.into()));
    view.comment_input
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
        window.press("cmd-left", cx);
        window.press("shift-right", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            assert_eq!(view.selection.as_ref().map(|s| s.1.as_str()), Some("t"));
            assert!(
                view.comment_anchor.is_none(),
                "selecting never starts a comment"
            );
            assert!(
                view.note_editor
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
                view.comment_anchor.as_ref().map(|anchor| anchor.3.as_str()),
                Some("te")
            );
            assert_eq!(view.comment_anchor.as_ref().unwrap().2, 0..2);
            assert!(
                view.comment_input
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
            view.comment_anchor = Some((f.a.id.clone(), f.a.version, 5..9, "tere".into()));
            view.comment_input.update(cx, |editor, cx| {
                editor.set_value("A passage draft", window, cx)
            });
            view.select_writing_thread(f.a.id.clone(), f.chat_b.clone(), window, cx);
            assert!(view.composer.read(cx).value().is_empty());
            view.composer.update(cx, |editor, cx| {
                editor.set_value("Chat B draft", window, cx)
            });
            view.select_writing_thread(f.a.id.clone(), f.chat_a.clone(), window, cx);
            assert_eq!(view.composer.read(cx).value().as_ref(), "Chat A draft");
            view.note_editor.update(cx, |editor, cx| {
                editor.set_value("Human unsaved õun", window, cx)
            });
            view.preserve_visible_note_input(cx);
            view.open_note(&f.b.id, None, window, cx);
            assert!(view.comment_anchor.is_none());
            assert!(view.comment_input.read(cx).value().is_empty());
            assert_eq!(view.composer.read(cx).value().as_ref(), "Chat A draft");
            view.comment_input
                .update(cx, |editor, cx| editor.set_value("B draft", window, cx));
            view.open_note(&f.a.id, None, window, cx);
            assert_eq!(
                view.note_editor.read(cx).value().as_ref(),
                "Human unsaved õun"
            );
            assert!(view.note.as_ref().unwrap().session.is_some());
            assert_eq!(view.comment_anchor.as_ref().unwrap().2, 5..9);
            assert_eq!(
                view.comment_input.read(cx).value().as_ref(),
                "A passage draft"
            );
            assert_eq!(view.composer.read(cx).value().as_ref(), "Chat A draft");
            view.post_comment(window, cx);
            assert!(view.workspace.writing_comments(&f.a.id).unwrap().is_empty());
            assert_eq!(
                view.comment_input.read(cx).value().as_ref(),
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
        view.excluded_comments.insert(comments[1].record.id.clone());
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
        assert_eq!(view.comment_input.read(cx).value().as_ref(),"Retain stale passage");
        assert!(view.comment_anchor.is_some());
        assert!(view.workspace.writing_comments(&f.a.id).unwrap().is_empty());
        view.retry_rewrite(request,cx);
        assert!(view.notice.contains("Reload saved note"));
        assert!(view.worker.is_none());
        assert_eq!(view.comment_input.read(cx).value().as_ref(),"Retain stale passage");
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
                assert!(!view.review_inputs);
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
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_project_note(&other, &f.a.id, window, cx);
            view.expanded_requests.insert(private_request.clone());
            view.review_inputs = true;
            view.review_reading = true;
            view.layout.open(TileKind::Compare);
            view.apply_review(window, cx);
            assert!(view.review.is_some());
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
            assert!(matches!(
                view.workspace
                    .owner_change(
                        "native-reference-restored",
                        &ChangeRequest {
                            reason: "Restore existing access".into(),
                            writes: vec![Put {
                                id: reference,
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
            view.note_editor.update(cx, |editor, cx| {
                editor.set_value("Preserved human buffer", window, cx)
            });
            view.preserve_visible_note_input(cx);
            view.open_review(candidate, window, cx);
            assert_eq!(view.open_note_id(), Some(f.a.id.as_str()));
            assert_eq!(view.writing_threads.get(&f.a.id), Some(&f.chat_a));
            assert_eq!(view.composer.read(cx).value().as_ref(), "A request draft");
            assert_eq!(
                view.note_editor.read(cx).value().as_ref(),
                "Preserved human buffer"
            );
            assert_eq!(
                view.composer_drafts
                    .text(&ComposerDestination::Thread(f.chat_b.clone())),
                "B must never go to A"
            );
            view.apply_review(window, cx);
            assert!(view.review.is_some());
            assert!(view.notice.contains("open edit"));
            assert_eq!(
                view.note_editor.read(cx).value().as_ref(),
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
            assert!(view.review.is_none());
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
            assert_eq!(view.applied_rewrite.get(&f.a.id).unwrap().1, applied_a);
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
            assert_eq!(view.applied_rewrite.get(&f.a.id).unwrap().1, applied_a);
            assert_eq!(view.applied_rewrite.get(&f.b.id).unwrap().1, applied_b);
        })
    })
    .unwrap();
    let reopened = Workspace::open(&f.data).unwrap();
    let fresh = cx
        .update_window(f.window, |_, window, cx| {
            cx.new(|cx| {
                let mut view = ThreadsUi::new(reopened, f.data.clone(), window, cx);
                view.poll = None;
                assert_eq!(view.applied_rewrite.get(&f.a.id).unwrap().1, applied_a);
                assert_eq!(view.applied_rewrite.get(&f.b.id).unwrap().1, applied_b);
                assert!(matches!(
                    view.workspace.undo(&applied_a).unwrap(),
                    ApplyOutcome::Applied(_)
                ));
                view.refresh();
                assert!(!view.applied_rewrite.contains_key(&f.a.id));
                assert_eq!(view.applied_rewrite.get(&f.b.id).unwrap().1, applied_b);
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
                view.comment_input
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
    let editor = cx.read(|cx| f.view.read(cx).note_editor.clone());
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
        assert_eq!(f.view.read(cx).note_editor, editor);
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
            view.review_editor.update(cx, |editor, cx| {
                editor.set_value("Owner's unsubmitted final text", window, cx)
            });
            view.show_tile(TileKind::Actions, window, cx);
            assert_eq!(
                view.review.as_ref().unwrap().candidate.operation,
                candidate_id
            );
            assert_eq!(
                view.review_editor.read(cx).value().as_ref(),
                "Owner's unsubmitted final text"
            );
            view.show_tile(TileKind::Note, window, cx);
            assert!(view.review_editing);
            assert_eq!(
                view.review.as_ref().unwrap().candidate.operation,
                candidate_id
            );
            assert_eq!(
                view.review_editor.read(cx).value().as_ref(),
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
            view.review_editor.update(cx, |editor, cx| {
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
        assert!(view.review_editor.focus_handle(cx).is_focused(window));
        assert_eq!(
            view.review_editor.read(cx).value().as_ref(),
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
            view.note_editor
                .update(cx, |editor, cx| editor.set_value("What I typed here", window, cx));
            view.preserve_visible_note_input(cx);
            assert_eq!(view.edit_open, Some((f.a.id.clone(), f.a.version)));
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
            assert!(view.note.as_ref().unwrap().session.is_none());
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
        assert_eq!(view.edit_open, None);
        let session = view.note.as_ref().unwrap().session.clone().unwrap();
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
