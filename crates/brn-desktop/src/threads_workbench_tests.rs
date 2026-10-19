//! Native workbench journeys that keep writing attached to the document.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
    first: Record,
    second: Record,
}

fn fixture(cx: &mut TestAppContext) -> Fixture {
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("workbench-native");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let first = workspace
        .new_note("First note", "Tere, this is the saved original.")
        .unwrap();
    let second = workspace
        .new_note("Second note", "A separate saved note.")
        .unwrap();
    cx.update(gpui_kit::init);
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
        window,
        view,
        first,
        second,
    }
}

fn open_note(cx: &mut TestAppContext, f: &Fixture, note: &Record) {
    cx.update_window(f.window, |_, window, cx| {
        f.view
            .update(cx, |view, cx| view.open_note(&note.id, None, window, cx));
    })
    .unwrap();
    cx.run_until_parked();
}

fn request_message(view: &ThreadsUi, note: &str) -> String {
    view.workspace
        .writing_threads(note)
        .unwrap()
        .into_iter()
        .find_map(|thread| {
            view.workspace
                .messages(&thread.id)
                .ok()?
                .into_iter()
                .rev()
                .find_map(|record| match &record.data {
                    RecordData::Message(message) if message.role == "writing-request" => {
                        Some(record.id.clone())
                    }
                    _ => None,
                })
        })
        .expect("saved writing request")
}

#[gpui_kit::test]
fn closed_chat_rewrite_retry_review_apply_address_and_undo_stay_in_note(cx: &mut TestAppContext) {
    let f = fixture(cx);
    open_note(cx, &f, &f.first);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.layout.close(TileKind::Chat);
            view.writing.general_feedback.update(cx, |editor, cx| {
                editor.set_value(
                    "Keep the greeting and make the sentence clearer.",
                    window,
                    cx,
                )
            });
            view.writing.comment_anchor =
                Some((f.first.id.clone(), f.first.version, 6..10, "this".into()));
            view.writing.comment_input.update(cx, |editor, cx| {
                editor.set_value("Keep this exact phrase.", window, cx)
            });
            view.post_comment(window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();

    // Offline startup failure leaves a durable snapshot and a retry affordance.
    cx.update_window(f.window, |_, window, cx| window.click("rewrite-note", cx))
        .unwrap();
    cx.run_until_parked();
    let (request, original_snapshot, comment_id) = cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            assert!(view.worker.is_none());
            assert!(!view.layout.contains(TileKind::Chat));
            let request = request_message(view, &f.first.id);
            let snapshot = view.workspace.writing_request(&request).unwrap();
            assert_eq!(
                snapshot.general_feedback,
                "Keep the greeting and make the sentence clearer."
            );
            assert_eq!(snapshot.comments.len(), 1);
            assert!(snapshot.messages.is_empty());
            (request, snapshot.clone(), snapshot.comments[0].id.clone())
        })
    });
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("retry-rewrite").is_some());
        window.click("inspect-rewrite-inputs", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            assert!(view.writing.expanded_requests.contains(&request));
        })
    });
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("writing-requests").is_some());
        window.click("retry-rewrite", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            assert_eq!(
                view.workspace.writing_request(&request).unwrap(),
                original_snapshot
            );
            assert_eq!(
                view.workspace.writing_threads(&f.first.id).unwrap()[0].id,
                original_snapshot.thread
            );
            assert!(!view.layout.contains(TileKind::Chat));
        })
    });

    // A fresh request captures the changed draft; Retry above did not.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.writing.general_feedback.update(cx, |editor, cx| {
                editor.set_value("New direction for the next rewrite.", window, cx)
            });
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| window.click("rewrite-note", cx))
        .unwrap();
    cx.run_until_parked();
    let (new_request, new_snapshot, candidate, operation) = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let message = request_message(view, &f.first.id);
                assert_ne!(message, request);
                let snapshot = view.workspace.writing_request(&message).unwrap();
                assert_eq!(
                    snapshot.general_feedback,
                    "New direction for the next rewrite."
                );
                assert_eq!(snapshot.comments[0].id, comment_id);
                let operation = view
                    .workspace
                    .store
                    .allocate_operation("workbench-proposal")
                    .unwrap();
                let candidate = view
                    .workspace
                    .store
                    .prepare(
                        &operation,
                        &snapshot
                            .change_request("Tere, this is the clearer saved sentence.")
                            .unwrap(),
                    )
                    .unwrap();
                view.refresh();
                cx.notify();
                assert!(view.candidates.iter().any(|item| {
                    item.operation == candidate.operation
                        && view.writing_candidate_note(item).as_deref() == Some(f.first.id.as_str())
                }));
                assert!(!view.layout.contains(TileKind::Chat));
                let _ = window;
                (message, snapshot, candidate, operation.as_str().to_owned())
            })
        })
        .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        assert!(
            window
                .try_find(format!("writing-review-{operation}"))
                .is_some()
        );
        window.click(format!("writing-review-{operation}"), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("review-panel").is_some());
        f.view.update(cx, |view, _| {
            assert!(view.writing.review.is_some());
            assert!(!view.layout.contains(TileKind::Chat));
        });
        window.click("apply-final", cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("confirm-addressed").is_some());
        let confirm = window.find("confirm-addressed");
        assert_ne!(confirm.disabled(), Some(true), "{confirm:?}");
        f.view.update(cx, |view, _| {
            assert_eq!(
                view.writing.applied_rewrite.get(&f.first.id).unwrap().0,
                new_request
            );
            assert_eq!(
                view.workspace.writing_comments(&f.first.id).unwrap().len(),
                1
            );
            assert!(!view.workspace.writing_comments(&f.first.id).unwrap()[0].addressed);
        });
        window.click("confirm-addressed", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            assert!(view.workspace.writing_comments(&f.first.id).unwrap()[0].addressed);
            assert!(!view.layout.contains(TileKind::Chat));
        })
    });
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("undo-writing-apply").is_some());
        window.click("undo-writing-apply", cx);
    })
    .unwrap();
    cx.run_until_parked();

    cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            let RecordData::Note(current) = &view
                .workspace
                .store
                .note(&f.first.id)
                .unwrap()
                .unwrap()
                .data
            else {
                panic!("expected saved note")
            };
            assert_eq!(current.markdown, "Tere, this is the saved original.");
            let comments = view.workspace.writing_comments(&f.first.id).unwrap();
            assert_eq!(comments.len(), 1);
            assert!(
                !comments[0].addressed,
                "Undo restores the captured feedback state"
            );
            assert!(!view.layout.contains(TileKind::Chat));
            assert!(view.writing.review.is_none());
            assert_eq!(
                view.workspace.writing_request(&new_request).unwrap(),
                new_snapshot
            );
            let prepared = view.workspace.store.prepared(&candidate.operation).unwrap();
            assert_eq!(prepared, candidate);
        })
    });
}

#[gpui_kit::test]
fn note_navigation_restores_overall_and_unsent_chat_drafts_without_posting(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    open_note(cx, &f, &f.first);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.layout.close(TileKind::Chat);
            view.writing.general_feedback.update(cx, |editor, cx| {
                editor.set_value("First note direction", window, cx)
            });
            view.composer.update(cx, |editor, cx| {
                editor.set_value("Unsent chat draft for the first note", window, cx)
            });
        });
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.second.id, None, window, cx);
            assert_eq!(view.writing.general_feedback.read(cx).value().as_ref(), "");
            assert_eq!(
                view.composer.read(cx).value().as_ref(),
                "Unsent chat draft for the first note"
            );
            view.writing.general_feedback.update(cx, |editor, cx| {
                editor.set_value("Second note direction", window, cx)
            });
        });
    })
    .unwrap();
    cx.run_until_parked();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.open_note(&f.first.id, None, window, cx);
            assert_eq!(
                view.writing.general_feedback.read(cx).value().as_ref(),
                "First note direction"
            );
            assert_eq!(
                view.composer.read(cx).value().as_ref(),
                "Unsent chat draft for the first note"
            );
            view.open_note(&f.second.id, None, window, cx);
            assert_eq!(
                view.writing.general_feedback.read(cx).value().as_ref(),
                "Second note direction"
            );
            assert_eq!(
                view.composer.read(cx).value().as_ref(),
                "Unsent chat draft for the first note"
            );
            assert!(!view.layout.contains(TileKind::Chat));
            assert!(
                view.workspace
                    .writing_threads(&f.first.id)
                    .unwrap()
                    .is_empty()
            );
            assert!(
                view.workspace
                    .writing_threads(&f.second.id)
                    .unwrap()
                    .is_empty()
            );
            assert!(
                view.workspace
                    .records()
                    .unwrap()
                    .iter()
                    .all(|record| !matches!(record.data, RecordData::Message(_)))
            );
        });
    })
    .unwrap();
}

fn run_fixture(
    cx: &mut TestAppContext,
    writing: bool,
) -> (
    Fixture,
    String,
    String,
    Option<WritingSnapshot>,
    CancellationToken,
) {
    let f = fixture(cx);
    let (thread, request, run, cancel) = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let thread = view
                    .workspace
                    .new_thread("Synthetic writing host")
                    .unwrap()
                    .id;
                view.workspace
                    .link_writing_thread(
                        "workbench-run-link",
                        &f.first.id,
                        f.first.version,
                        &thread,
                    )
                    .unwrap();
                let request = if writing {
                    Some(
                        view.workspace
                            .snapshot_writing_request(
                                "workbench-run-request",
                                &f.first.id,
                                f.first.version,
                                &thread,
                                &[],
                            )
                            .unwrap(),
                    )
                } else {
                    None
                };
                let operation = view
                    .workspace
                    .store
                    .allocate_operation("workbench-synthetic-run")
                    .unwrap();
                let run = operation.creation_id(0);
                let settings = view.workspace.settings().unwrap();
                view.workspace
                    .owner_change(
                        "workbench-synthetic-run",
                        &ChangeRequest {
                            reason: "Synthetic native workbench run".into(),
                            inputs: vec![],
                            writes: vec![Put {
                                id: run.clone(),
                                expected_version: None,
                                archived: false,
                                data: RecordData::Run(Run {
                                    loaded_skill_details: vec![],
                                    instruction: request
                                        .as_ref()
                                        .map(|snapshot| snapshot.request_message.clone()),
                                    queue_claim: None,
                                    thread: thread.clone(),
                                    provider: "offline-fixture".into(),
                                    model: settings.model,
                                    guide_identity: "workbench-fixture".into(),
                                    budget: 1,
                                    effort: settings.effort,
                                    loaded_skills: vec![],
                                    fence: 0,
                                    state: RunState::Working,
                                    progress: "Synthetic progress".into(),
                                    operations: vec![],
                                    sources: vec![],
                                }),
                            }],
                        },
                    )
                    .unwrap();
                view.refresh();
                view.writing_threads
                    .insert(f.first.id.clone(), thread.clone());
                view.open_note(&f.first.id, None, window, cx);
                view.layout.close(TileKind::Chat);
                let (sender, receiver) = mpsc::sync_channel(16);
                drop(sender);
                let cancel = CancellationToken::new();
                view.worker = Some(Worker::run(
                    receiver,
                    cancel.clone(),
                    thread.clone(),
                    run.clone(),
                    None,
                ));
                view.progress = "Synthetic rewrite in progress".into();
                (
                    thread,
                    request.map(|snapshot| snapshot.request_message.clone()),
                    run.as_str().to_owned(),
                    cancel,
                )
            })
        })
        .unwrap();
    cx.run_until_parked();
    let snapshot = request.as_ref().map(|id| {
        f.view
            .read_with(cx, |view, _| view.workspace.writing_request(id).unwrap())
    });
    (f, thread, run, snapshot, cancel)
}

#[gpui_kit::test]
fn exact_writing_run_shows_note_stop_and_cancel_keeps_saved_request(cx: &mut TestAppContext) {
    let (f, _thread, _run, snapshot, cancel) = run_fixture(cx, true);
    let snapshot = snapshot.unwrap();
    let (candidate, operation) = cx
        .update_window(f.window, |_, _, cx| {
            f.view.update(cx, |view, cx| {
                let operation = view
                    .workspace
                    .store
                    .allocate_operation("active-rewrite-existing-review")
                    .unwrap();
                let candidate = view
                    .workspace
                    .store
                    .prepare(
                        &operation,
                        &snapshot
                            .change_request("An earlier rewrite still ready to review.")
                            .unwrap(),
                    )
                    .unwrap();
                view.refresh();
                cx.notify();
                (candidate, operation.as_str().to_owned())
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("stop-writing").is_some());
        assert!(window.try_find("writing-progress").is_some());
        assert!(
            window
                .try_find(format!("writing-review-{operation}"))
                .is_some()
        );
        window.click(format!("writing-review-{operation}"), cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("review-panel").is_some());
        assert!(window.try_find("stop-writing").is_some());
        assert!(window.try_find("writing-progress").is_some());
        f.view
            .update(cx, |view, _| assert!(!view.layout.contains(TileKind::Chat)));
        window.click("stop-writing", cx);
    })
    .unwrap();
    cx.run_until_parked();
    assert!(cancel.is_cancelled());
    cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            assert!(!view.layout.contains(TileKind::Chat));
            let run_id = &view.worker.as_ref().unwrap().run_identity().unwrap().run_id;
            let RecordData::Run(run) = &view.workspace.store.record(run_id).unwrap().unwrap().data
            else {
                panic!("expected synthetic run")
            };
            assert_eq!(run.state, RunState::Interrupted);
            assert_eq!(
                view.writing.review.as_ref().unwrap().candidate.operation,
                candidate.operation
            );
            assert_eq!(
                view.workspace
                    .writing_request(&snapshot.request_message)
                    .unwrap(),
                snapshot
            );
        })
    });
}

#[gpui_kit::test]
fn an_open_edit_keeps_save_beside_stop_while_its_rewrite_runs(cx: &mut TestAppContext) {
    let (f, _thread, _run, _snapshot, _cancel) = run_fixture(cx, true);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.writing.note_editor.update(cx, |editor, cx| {
                editor.set_value("Edited while BRN rewrites", window, cx)
            });
            view.preserve_visible_note_input(cx);
            cx.notify();
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("save-note").visible());
        assert!(window.find("stop-writing").visible());
        assert!(window.find("writing-progress").visible());
        f.view.update(cx, |view, _| {
            assert_eq!(view.writing_bar_row(), Some(workbench::WritingRow::Dirty));
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn ordinary_chat_run_does_not_show_as_writing_activity_in_note(cx: &mut TestAppContext) {
    let (f, _thread, _run, snapshot, _cancel) = run_fixture(cx, false);
    assert!(snapshot.is_none());
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("stop-writing").is_none());
        assert!(window.try_find("writing-progress").is_none());
        f.view.update(cx, |view, _cx| {
            assert!(view.worker.is_some());
            assert!(!view.writing_worker_for(&f.first.id));
            assert!(!view.layout.contains(TileKind::Chat));
        });
    })
    .unwrap();
}

#[gpui_kit::test]
fn ordinary_question_attention_survives_rewrite_and_proposal_stays_with_note(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let mut thread_id = String::new();
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let thread = view.workspace.new_thread("Question thread").unwrap();
            thread_id = thread.id.clone();
            view.workspace
                .post_message(&thread_id, "Which renewal date should we use?")
                .unwrap();
            view.workspace
                .link_writing_thread(
                    "question-workbench-link",
                    &f.first.id,
                    f.first.version,
                    &thread_id,
                )
                .unwrap();
            let current = view.workspace.store.record(&thread_id).unwrap().unwrap();
            view.workspace
                .owner_change(
                    "question-workbench-attention",
                    &ChangeRequest {
                        reason: "Preserve the ordinary thread's question attention".into(),
                        inputs: vec![],
                        writes: vec![Put {
                            id: thread_id.clone(),
                            expected_version: Some(current.version),
                            archived: false,
                            data: RecordData::Thread(Thread {
                                private_context: false,
                                context_epoch: 0,
                                title: "Question thread".into(),
                                state: ThreadState::Open,
                                attention: vec![Attention {
                                    kind: AttentionKind::Question,
                                    reason: "Choose the renewal date".into(),
                                    record: None,
                                }],
                            }),
                        }],
                    },
                )
                .unwrap();
            view.refresh();
            view.open_note(&f.first.id, None, window, cx);
            assert!(view.layout.contains(TileKind::Chat));
            assert_eq!(view.chat_thread(), Some(thread_id.as_str()));
            view.rewrite_note(cx);
            assert!(view.worker.is_none());
        });
    })
    .unwrap();
    cx.run_until_parked();

    let (request, snapshot) = cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            let request = request_message(view, &f.first.id);
            let snapshot = view.workspace.writing_request(&request).unwrap();
            assert_eq!(snapshot.thread, thread_id);
            let RecordData::Thread(thread) = &view
                .workspace
                .store
                .record(&thread_id)
                .unwrap()
                .unwrap()
                .data
            else {
                panic!("expected ordinary question thread")
            };
            assert_eq!(thread.attention.len(), 1);
            assert_eq!(thread.attention[0].reason, "Choose the renewal date");
            assert_eq!(thread.attention[0].record, None);
            assert_eq!(
                view.writing_run_outcome().map(|outcome| outcome.text()),
                Some("Rewrite inputs saved".into())
            );
            assert!(view.layout.contains(TileKind::Chat));
            (request, snapshot)
        })
    });
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("attention-card-0").is_some());
        assert!(
            window
                .within("thread-conversation")
                .try_find("writing-proposals")
                .is_none()
        );
        window.click("inspect-rewrite-inputs", cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            assert!(view.writing.expanded_requests.contains(&request));
        })
    });
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-requests").is_some());
    })
    .unwrap();

    let (candidate, operation) = cx
        .update_window(f.window, |_, _, cx| {
            f.view.update(cx, |view, cx| {
                let operation = view
                    .workspace
                    .store
                    .allocate_operation("question-thread-proposal")
                    .unwrap();
                let candidate = view
                    .workspace
                    .store
                    .prepare(
                        &operation,
                        &snapshot.change_request("A proposed renewal note.").unwrap(),
                    )
                    .unwrap();
                view.refresh();
                cx.notify();
                (candidate, operation.as_str().to_owned())
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("attention-card-0").is_some());
        assert!(
            window
                .try_find(format!("writing-review-{operation}"))
                .is_some()
        );
        assert!(
            window
                .within("thread-conversation")
                .try_find(format!("writing-review-{operation}"))
                .is_none()
        );
        assert!(
            window
                .try_find(format!("writing-proposal-{operation}"))
                .is_some()
        );
    })
    .unwrap();
    cx.read(|cx| {
        f.view.read_with(cx, |view, _| {
            assert_eq!(view.workspace.writing_request(&request).unwrap(), snapshot);
            assert!(view.layout.contains(TileKind::Chat));
            assert!(view.layout.contains(TileKind::Note));
            assert!(
                view.candidates
                    .iter()
                    .any(|stored| stored.operation == candidate.operation)
            );
        })
    });
}

#[gpui_kit::test]
fn repeated_rewrite_requests_preserve_pending_review_and_reading_space(cx: &mut TestAppContext) {
    let f = fixture(cx);
    open_note(cx, &f, &f.first);
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |view, cx| {
            view.layout.close(TileKind::Chat);
            let thread = view.ensure_writing_thread(&f.first.id).unwrap();
            for index in 0..8 {
                let snapshot = view
                    .workspace
                    .snapshot_writing_request_with_feedback(
                        &format!("retained-request-{index}"),
                        &f.first.id,
                        f.first.version,
                        &thread,
                        &[],
                        &format!("Direction {index}"),
                    )
                    .unwrap();
                let operation = view
                    .workspace
                    .store
                    .allocate_operation(&format!("retained-candidate-{index}"))
                    .unwrap();
                let result = view.workspace.store.prepare(
                    &operation,
                    &snapshot
                        .change_request(&format!("Proposed wording {index}"))
                        .unwrap(),
                );
                if index == 0 {
                    result.unwrap();
                } else {
                    assert!(result.unwrap_err().to_string().contains("active candidate"));
                }
            }
            view.refresh();
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert_eq!(f.view.read(cx).candidates.len(), 1);
        assert!(window.find("writing-proposals").bounds().size.height <= px(110.));
        assert!(window.find("note-body").bounds().size.height > px(200.));
        assert!(window.find("overall-feedback").visible());
    })
    .unwrap();
}

#[gpui_kit::test]
fn bound_terminal_clarification_stays_in_note_and_unrelated_chat_cannot_replace_it(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    open_note(cx, &f, &f.first);
    let (thread, run_id, snapshot, applied_note, prior_request) = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                view.layout.close(TileKind::Chat);
                let thread = view.ensure_writing_thread(&f.first.id).unwrap();
                let first = view
                    .workspace
                    .snapshot_writing_request(
                        "result-first-request",
                        &f.first.id,
                        f.first.version,
                        &thread,
                        &[],
                    )
                    .unwrap();
                let operation = view
                    .workspace
                    .store
                    .allocate_operation("result-first-proposal")
                    .unwrap();
                let candidate = view
                    .workspace
                    .store
                    .prepare(
                        &operation,
                        &first
                            .change_request("Tere, the first refinement is saved.")
                            .unwrap(),
                    )
                    .unwrap();
                view.open_review(candidate, window, cx);
                view.apply_review(window, cx);
                assert!(view.writing.review.is_none());
                assert_eq!(
                    view.writing.applied_rewrite.get(&f.first.id).unwrap().0,
                    first.request_message
                );
                let applied_note = view.workspace.store.record(&f.first.id).unwrap().unwrap();
                assert!(applied_note.version > f.first.version);
                let second = view
                    .workspace
                    .snapshot_writing_request_with_feedback(
                        "result-second-request",
                        &applied_note.id,
                        applied_note.version,
                        &thread,
                        &[],
                        "Clarify whether the greeting should stay informal before rewriting.",
                    )
                    .unwrap();
                let run_operation = view
                    .workspace
                    .store
                    .allocate_operation("result-second-run")
                    .unwrap();
                let run_id = run_operation.creation_id(0);
                let settings = view.workspace.settings().unwrap();
                view.workspace
                    .owner_change(
                        "result-second-run",
                        &ChangeRequest {
                            reason: "Synthetic second writing run".into(),
                            inputs: vec![],
                            writes: vec![Put {
                                id: run_id.clone(),
                                expected_version: None,
                                archived: false,
                                data: RecordData::Run(Run {
                                    loaded_skill_details: vec![],
                                    instruction: Some(second.request_message.clone()),
                                    queue_claim: None,
                                    thread: thread.clone(),
                                    provider: "offline-fixture".into(),
                                    model: settings.model,
                                    guide_identity: "workbench-result-fixture".into(),
                                    budget: 1,
                                    effort: settings.effort,
                                    loaded_skills: vec![],
                                    fence: 0,
                                    state: RunState::Working,
                                    progress: String::new(),
                                    operations: vec![],
                                    sources: vec![],
                                }),
                            }],
                        },
                    )
                    .unwrap();
                view.refresh();
                cx.notify();
                (thread, run_id, second, applied_note, first.request_message)
            })
        })
        .unwrap();
    let reply = "Before I rewrite, please clarify: should the greeting remain informal?";
    let response_id = cx
        .update_window(f.window, |_, _, cx| {
            f.view.update(cx, |view, cx| {
                assert!(!view.layout.contains(TileKind::Chat));
                assert!(view.candidates.is_empty());
                let current = view.workspace.store.record(&run_id).unwrap().unwrap();
                let RecordData::Run(mut run) = current.data else {
                    unreachable!()
                };
                assert_eq!(
                    run.instruction.as_deref(),
                    Some(snapshot.request_message.as_str())
                );
                run.state = RunState::Completed;
                run.fence += 1;
                let operation = view
                    .workspace
                    .store
                    .allocate_operation("writing-clarification")
                    .unwrap();
                let response = operation.creation_id(0);
                view.workspace
                    .owner_change(
                        "writing-clarification",
                        &ChangeRequest {
                            reason: "Synthetic completed rewrite asks for clarification".into(),
                            inputs: vec![],
                            writes: vec![
                                Put {
                                    id: run_id.clone(),
                                    expected_version: Some(current.version),
                                    archived: false,
                                    data: RecordData::Run(run),
                                },
                                Put {
                                    id: response.clone(),
                                    expected_version: None,
                                    archived: false,
                                    data: RecordData::Message(Message {
                                        thread: thread.clone(),
                                        role: "assistant".into(),
                                        text: reply.into(),
                                    }),
                                },
                            ],
                        },
                    )
                    .unwrap();
                view.worker = None;
                view.refresh();
                let result = view
                    .writing_result_note()
                    .expect("bound clarification result");
                assert_eq!(result.id, response);
                assert!(
                    matches!(&result.data, RecordData::Message(message) if message.text == reply)
                );
                assert_eq!(
                    view.workspace.store.record(&f.first.id).unwrap().unwrap(),
                    applied_note
                );
                assert_eq!(
                    view.writing.applied_rewrite.get(&f.first.id).unwrap().0,
                    prior_request
                );
                assert!(view.candidates.is_empty());
                cx.notify();
                response
            })
        })
        .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.find("writing-result").visible());
        assert!(window.find("writing-result").bounds().size.height <= px(120.));
        assert!(window.find("retry-rewrite").visible());
        assert!(window.find("undo-writing-apply").visible());
        assert!(!f.view.read(cx).layout.contains(TileKind::Chat));
    })
    .unwrap();
    // As on the base, an open edit hides Undo (Changes keeps it); its
    // siblings say to save first. Discarding brings Undo back.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let text = format!("{} Edited here.", view.writing.note_editor.read(cx).value());
            view.writing
                .note_editor
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
            view.preserve_visible_note_input(cx);
            cx.notify();
        })
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(
            f.view
                .read(cx)
                .writing
                .note
                .as_ref()
                .unwrap()
                .session()
                .is_some()
        );
        assert!(window.try_find("undo-writing-apply").is_none());
        assert_eq!(
            window.find("retry-rewrite").label(),
            Some("Retry saved rewrite, unavailable: Save your edit first")
        );
        f.view.update(cx, |view, cx| {
            let session = view
                .writing
                .note
                .as_ref()
                .unwrap()
                .session()
                .cloned()
                .unwrap();
            view.discard_session(session, window, cx)
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, cx| {
        assert!(
            f.view
                .read(cx)
                .writing
                .note
                .as_ref()
                .unwrap()
                .session()
                .is_none()
        );
        assert!(window.find("undo-writing-apply").visible());
    })
    .unwrap();

    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let unrelated = view.workspace.new_thread("Selected unrelated chat").unwrap();
            view.set_chat(ChatTarget::Thread(unrelated.id.clone()), window, cx);
            assert_eq!(view.chat, ChatTarget::Thread(unrelated.id));
            assert!(!view.layout.contains(TileKind::Chat));
            let result = view.writing_result_note().unwrap();
            assert_eq!(result.id, response_id);
            assert!(matches!(&result.data, RecordData::Message(message) if message.thread == thread && message.text == reply));
            cx.notify();
        });
    }).unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("writing-result").visible())
    })
    .unwrap();

    // An ordinary assistant message has no receipt binding to the writing run.
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |view, cx| {
            let operation = view
                .workspace
                .store
                .allocate_operation("unrelated-assistant")
                .unwrap();
            view.workspace
                .owner_change(
                    "unrelated-assistant",
                    &ChangeRequest {
                        reason: "Synthetic ordinary assistant reply".into(),
                        inputs: vec![],
                        writes: vec![Put {
                            id: operation.creation_id(0),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Message(Message {
                                thread: thread.clone(),
                                role: "assistant".into(),
                                text:
                                    "Unrelated ordinary chat reply must not replace clarification."
                                        .into(),
                            }),
                        }],
                    },
                )
                .unwrap();
            view.refresh();
            assert_eq!(view.writing_result_note().unwrap().id, response_id);
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.find("writing-result").visible())
    })
    .unwrap();

    // The latest invocation is now an ordinary run, so the writing reply is stale UI context.
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |view, cx| {
            let current = view.workspace.store.record(&run_id).unwrap().unwrap();
            let RecordData::Run(mut ordinary) = current.data else {
                unreachable!()
            };
            ordinary.instruction = None;
            let operation = view
                .workspace
                .store
                .allocate_operation("later-ordinary-run")
                .unwrap();
            view.workspace
                .owner_change(
                    "later-ordinary-run",
                    &ChangeRequest {
                        reason: "Synthetic later ordinary invocation".into(),
                        inputs: vec![],
                        writes: vec![Put {
                            id: operation.creation_id(0),
                            expected_version: None,
                            archived: false,
                            data: RecordData::Run(ordinary),
                        }],
                    },
                )
                .unwrap();
            view.refresh();
            assert!(view.writing_result_note().is_none());
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-result").is_none())
    })
    .unwrap();
    cx.update_window(f.window, |_, _, cx| {
        f.view.update(cx, |view, cx| {
            view.workspace
                .post_message(&thread, "A later ordinary chat instruction")
                .unwrap();
            view.refresh();
            assert!(view.writing_result_note().is_none());
            assert!(!view.layout.contains(TileKind::Chat));
            cx.notify();
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(f.window, |_, window, _| {
        assert!(window.try_find("writing-result").is_none())
    })
    .unwrap();
}

#[gpui_kit::test]
fn pure_writing_queries_and_write_notifies_do_not_load_unconsumed_display_data(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    open_note(cx, &f, &f.first);
    let thread = cx
        .update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let thread = view
                    .workspace
                    .new_thread("Writing query fixture")
                    .unwrap()
                    .id;
                view.workspace
                    .link_writing_thread("work-bound-link", &f.first.id, f.first.version, &thread)
                    .unwrap();
                for index in 0..3 {
                    let record = if index == 0 {
                        f.second.clone()
                    } else {
                        view.workspace
                            .new_note(&format!("Unrelated fixture {index}"), "Other saved text")
                            .unwrap()
                    };
                    let operation = view
                        .workspace
                        .store
                        .allocate_operation(&format!("work-bound-candidate-{index}"))
                        .unwrap();
                    let mut note = record.data.clone();
                    let RecordData::Note(ref mut note_data) = note else {
                        panic!("fixture Note")
                    };
                    note_data.markdown = format!("Unrelated pending text {index}");
                    view.workspace
                        .store
                        .prepare(
                            &operation,
                            &ChangeRequest {
                                reason: format!("Unrelated pending edit {index}"),
                                inputs: vec![],
                                writes: vec![Put {
                                    id: record.id,
                                    expected_version: Some(record.version),
                                    archived: false,
                                    data: note,
                                }],
                            },
                        )
                        .unwrap();
                }
                view.open_note(&f.first.id, None, window, cx);
                cx.notify();
                thread
            })
        })
        .unwrap();
    cx.run_until_parked();
    let before = cx.read(|cx| {
        (
            WRITING_DISPLAY_LOADS.with(|loads| loads.get()),
            f.view.read(cx).writing_review_loads,
        )
    });
    for _ in 0..4 {
        cx.update_window(f.window, |_, window, cx| {
            f.view.update(cx, |view, cx| {
                let _ = view.comments_open();
                let _ = view.comments_beside();
                let _ = view.comments_stacked();
                let _ = view.note_toolbar_needs(window);
                assert_eq!(view.latest_writing_instruction(&thread), None);
                assert_eq!(view.candidates.len(), 3);
                for candidate in &view.candidates {
                    assert_eq!(view.writing_candidate_note(candidate), None);
                }
                cx.notify();
            });
        })
        .unwrap();
        cx.run_until_parked();
        let after = cx.read(|cx| {
            (
                WRITING_DISPLAY_LOADS.with(|loads| loads.get()),
                f.view.read(cx).writing_review_loads,
            )
        });
        assert_eq!(
            after, before,
            "without a saved writing request, pure queries, per-candidate filters and unchanged Write redraws must not resolve assets/read latest Run/reload the canonical batch"
        );
    }
    // Positive controls use real consumers, so a disconnected counter cannot
    // turn the no-work assertion into a vacuous pass.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            let context = view
                .workspace
                .ensure_writing_context("work-bound-context", &f.first.id, None)
                .unwrap();
            let thread = context.conversation.unwrap();
            view.workspace
                .snapshot_writing_request(
                    "work-bound-request",
                    &f.first.id,
                    f.first.version,
                    &thread,
                    &[],
                )
                .unwrap();
            view.open_note(&f.first.id, None, window, cx);
            let runs = WRITING_DISPLAY_LOADS.with(|loads| loads.get().1);
            assert!(matches!(
                view.writing_run_outcome(),
                Some(RunOutcome::InputsSaved)
            ));
            assert!(
                WRITING_DISPLAY_LOADS.with(|loads| loads.get().1) > runs,
                "the actual status query invokes its deferred latest-run reader"
            );
            view.set_note_reading(true, window, cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    cx.read(|_| {
        assert!(
            WRITING_DISPLAY_LOADS.with(|loads| loads.get().0) > before.0.0,
            "the actual formatted Read consumer resolves its scoped assets"
        );
    });
}
