//! Contextual actions use real dialogs with synthetic saved records, offline.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    view: Entity<ThreadsUi>,
    window: gpui_kit::AnyWindowHandle,
    note: Record,
    thread: String,
    reply: Record,
}
fn fixture(cx: &mut TestAppContext) -> Fixture {
    cx.update(gpui_kit::init);
    cx.update(|cx| cx.bind_keys(bindings()));
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("context-actions");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    let note = workspace
        .new_note("Route plan", "Original saved note")
        .unwrap();
    let thread = workspace.new_thread("Route choice").unwrap().id;
    workspace
        .link_writing_thread("link", &note.id, note.version, &thread)
        .unwrap();
    let op = workspace.store.allocate_operation("reply").unwrap();
    let ApplyOutcome::Applied(receipt) = workspace
        .owner_change(
            "reply",
            &ChangeRequest {
                reason: "Synthetic completed answer".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: op.creation_id(0),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: thread.clone(),
                        role: "assistant".into(),
                        text: "Choose Alpha · õun".into(),
                    }),
                }],
            },
        )
        .unwrap()
    else {
        panic!()
    };
    let reply = receipt.writes[0].after.clone();
    let (window, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(1400.), px(1100.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let mut ui = ThreadsUi::new(workspace, data, window, cx);
                    ui.poll = None;
                    ui.open_note(&note.id, None, window, cx);
                    ui.open_thread(&thread, window, cx);
                    ui.composer.update(cx, |editor, cx| {
                        editor.set_value("Unsent chat draft", window, cx)
                    });
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    Fixture {
        _root: root,
        view,
        window,
        note,
        thread,
        reply,
    }
}
fn click(cx: &mut TestAppContext, f: &Fixture, id: impl Into<String>) {
    cx.update_window(f.window, |_, window, cx| window.click(id.into(), cx))
        .unwrap();
    cx.run_until_parked();
}

#[gpui_kit::test]
fn note_edit_is_contextual_exact_and_keeps_the_chat_draft(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        assert!(window.try_find("mode-pill").is_none());
        assert!(window.find("edit-note-context").visible());
        assert!(matches!(f.view.read(cx).edit_note_authority(&f.note).unwrap(), brn_ai::threads::OwnerRequest::EditNote { id, version } if id == f.note.id && version == f.note.version));
    }).unwrap();
    click(cx, &f, "edit-note-context");
    click(cx, &f, "context-edit-cancel");
    cx.read(|cx| {
        assert_eq!(
            f.view.read(cx).workspace.messages(&f.thread).unwrap().len(),
            1
        )
    });
    click(cx, &f, "edit-note-context");
    click(cx, &f, "context-edit-instruction");
    cx.simulate_input(f.window, "Tighten this route plan · õun");
    cx.run_until_parked();
    click(cx, &f, "context-edit-submit");
    cx.read(|cx| {
        let ui = f.view.read(cx);
        let messages = ui.workspace.messages(&f.thread).unwrap();
        assert_eq!(messages.len(), 2);
        assert!(matches!(&messages[1].data, RecordData::Message(m) if m.role == "owner" && m.text == "Tighten this route plan · õun"));
        assert_eq!(ui.composer.read(cx).value(), "Unsent chat draft");
        assert!(ui.notice.contains("Choose a provider"));
        assert_eq!(ui.workspace.store.record(&f.note.id).unwrap().unwrap(), f.note);
    });
}

#[gpui_kit::test]
fn contextual_edit_rechecks_saved_note_and_queue_before_dispatch(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            ui.workspace
                .queue_message("waiting", &f.thread, "Waiting request")
                .unwrap();
            assert!(!ui.can_edit_note_context());
            assert!(ui.submit_note_edit(&f.note, "Edit", window, cx).is_err());
            let queued = ui.queue_state().queue[0].id.clone();
            ui.workspace.remove_queued("remove", &queued).unwrap();
            ui.note_changed("Dirty human draft".into());
            assert!(!ui.can_edit_note_context());
            assert!(ui.edit_note_authority(&f.note).is_err());
        })
    })
    .unwrap();
    // A second fixture pins the stale dialog snapshot independently of dirty writing.
    let f = fixture(cx);
    click(cx, &f, "edit-note-context");
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            let mut data = f.note.data.clone();
            if let RecordData::Note(n) = &mut data {
                n.markdown = "New saved revision".into();
            }
            ui.workspace
                .owner_change(
                    "external-save",
                    &ChangeRequest {
                        reason: "Synthetic other writer".into(),
                        inputs: vec![],
                        writes: vec![Put {
                            id: f.note.id.clone(),
                            expected_version: Some(f.note.version),
                            archived: false,
                            data,
                        }],
                    },
                )
                .unwrap();
            assert!(ui.submit_note_edit(&f.note, "Edit", window, cx).is_err());
            assert_eq!(ui.workspace.messages(&f.thread).unwrap().len(), 1);
        })
    })
    .unwrap();
}

#[gpui_kit::test]
fn save_decision_reviews_exact_text_cancels_without_writes_and_preserves_drafts(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    let action = format!("answer-decision-{}", f.reply.id);
    click(cx, &f, &action);
    click(cx, &f, "decision-cancel");
    cx.read(|cx| assert_eq!(f.view.read(cx).workspace.notes().unwrap().len(), 1));
    click(cx, &f, &action);
    click(cx, &f, "decision-title");
    cx.simulate_keystrokes(f.window, "cmd-a");
    cx.simulate_input(f.window, "Owner route decision");
    cx.run_until_parked();
    click(cx, &f, "decision-text");
    cx.simulate_keystrokes(f.window, "cmd-a");
    cx.simulate_input(f.window, "Decision with ![missing](brn-asset://missing)");
    cx.run_until_parked();
    click(cx, &f, "decision-save");
    cx.update_window(f.window, |_, window, cx| {
        assert!(
            window
                .find("decision-error")
                .label()
                .unwrap()
                .contains("figure")
        );
        assert_eq!(f.view.read(cx).workspace.notes().unwrap().len(), 1);
    })
    .unwrap();
    click(cx, &f, "decision-text");
    cx.simulate_keystrokes(f.window, "cmd-a");
    cx.simulate_input(f.window, "We choose Gamma · õun");
    cx.run_until_parked();
    click(cx, &f, "decision-save");
    cx.update_window(f.window, |_, window, cx| f.view.update(cx, |ui, cx| {
        let notes = ui.workspace.notes().unwrap();
        assert_eq!(notes.len(), 2);
        assert!(notes.iter().any(|r| matches!(&r.data, RecordData::Note(n) if n.title == "Owner route decision" && n.markdown == "We choose Gamma · õun" && n.protected && n.confirmed && n.chat.is_none())));
        assert_eq!(ui.workspace.store.record(&f.reply.id).unwrap().unwrap(), f.reply);
        assert!(ui.worker.is_none(), "saving reviewed text must not invoke a model");
        ui.open_thread(&f.thread, window, cx);
        assert_eq!(ui.composer.read(cx).value(), "Unsent chat draft");
    })).unwrap();
}

#[gpui_kit::test]
fn first_contextual_note_edit_moves_the_unsent_draft_into_its_new_chat(cx: &mut TestAppContext) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            let note = ui
                .workspace
                .new_note("Unlinked note", "Saved source")
                .unwrap();
            ui.open_note(&note.id, None, window, cx);
            assert!(matches!(&ui.chat, ChatTarget::NewForNote(id) if id == &note.id));
            ui.composer.update(cx, |editor, cx| {
                editor.set_value("A preserved note draft · õun", window, cx)
            });
            ui.submit_note_edit(&note, "Explicit edit instruction", window, cx)
                .unwrap();
            let ChatTarget::Thread(thread) = &ui.chat else {
                panic!()
            };
            assert_eq!(ui.composer.read(cx).value(), "A preserved note draft · õun");
            let drafts = ui.workspace.store.chat_drafts().unwrap();
            let key = serde_json::to_string(&ComposerDestination::Thread(thread.clone())).unwrap();
            assert!(
                drafts
                    .iter()
                    .any(|d| d.destination == key && d.text == "A preserved note draft · õun")
            );
            let old = serde_json::to_string(&ComposerDestination::Note(note.id, None)).unwrap();
            assert!(
                drafts
                    .iter()
                    .all(|d| d.destination != old || d.text.is_empty())
            );
        })
    })
    .unwrap();
}

#[gpui_kit::test]
fn refused_context_draft_handoff_posts_no_instruction_and_retry_posts_once(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx);
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |ui, cx| {
            let note = ui.workspace.new_note("Retry note", "Saved text").unwrap();
            ui.open_note(&note.id, None, window, cx);
            ui.composer.update(cx, |editor, cx| {
                editor.set_value("Original unsent draft", window, cx)
            });
            let thread = ui.ensure_writing_thread(&note.id, note.version).unwrap();
            let key = serde_json::to_string(&ComposerDestination::Thread(thread.clone())).unwrap();
            ui.workspace
                .store
                .save_chat_draft(&key, 100, "Newer recovery draft")
                .unwrap();
            for _ in 0..2 {
                assert!(
                    ui.submit_note_edit(&note, "Exact explicit instruction", window, cx)
                        .is_err()
                );
                assert!(ui.workspace.messages(&thread).unwrap().is_empty());
                assert_eq!(ui.composer.read(cx).value(), "Original unsent draft");
            }
            ui.draft_generations.insert(key, 100);
            ui.submit_note_edit(&note, "Exact explicit instruction", window, cx)
                .unwrap();
            assert_eq!(ui.workspace.messages(&thread).unwrap().len(), 1);
            assert_eq!(ui.composer.read(cx).value(), "Original unsent draft");
        })
    })
    .unwrap();
}
