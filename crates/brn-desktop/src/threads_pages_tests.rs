//! Offline witnesses for the list rows and workspace pages: needs-you rows
//! that keep their titles beside long reasons, review rows that name their
//! notes and protection, Tasks, Changes, Home, Settings and Delegation. Data
//! is synthetic and no provider starts; pixels remain native acceptance.
use super::*;
use gpui_kit::{TestAppContext, test::TestWindowExt};

struct Fixture {
    _root: tempfile::TempDir,
    data: PathBuf,
    window: gpui_kit::AnyWindowHandle,
    view: Entity<ThreadsUi>,
}

/// A window of this size over a workspace `seed` fills before it opens.
fn fixture(
    cx: &mut TestAppContext,
    width: f32,
    height: f32,
    seed: impl FnOnce(&mut Workspace),
) -> Fixture {
    cx.update(|cx| {
        gpui_kit::init(cx);
        cx.set_reduce_motion(true);
    });
    cx.update(|cx| cx.bind_keys(bindings()));
    let root = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    let data = root.path().join("pages");
    let mut workspace = Workspace::open(&data).unwrap();
    focus_tests::configure_offline_fixture(&mut workspace);
    seed(&mut workspace);
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
                    let mut ui = ThreadsUi::new(workspace, data.clone(), window, cx);
                    ui.poll = None;
                    ui
                })
            },
        )
        .unwrap()
    });
    cx.run_until_parked();
    Fixture {
        _root: root,
        data,
        window,
        view,
    }
}
fn update<R>(
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
fn on_window<R>(
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
fn click(cx: &mut TestAppContext, f: &Fixture, id: impl Into<String>) {
    let id = id.into();
    on_window(cx, f, |window, cx| window.click(id, cx));
}
fn label(window: &mut Window, id: impl Into<String>) -> String {
    window
        .find(id.into())
        .label()
        .unwrap_or_default()
        .to_owned()
}
/// Labels of the open popup menu's rows, in order (separators have none).
fn menu_rows(cx: &mut TestAppContext, f: &Fixture) -> Vec<String> {
    on_window(cx, f, |window, _| {
        let menu = window.within("popup-menu");
        (0..60usize)
            .filter_map(|index| menu.try_find(index))
            .filter_map(|row| row.label().map(str::to_owned))
            .collect()
    })
}
fn choose(cx: &mut TestAppContext, f: &Fixture, label: &str) {
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
fn close_menu(cx: &mut TestAppContext, f: &Fixture) {
    on_window(cx, f, |window, cx| window.press("escape", cx));
}
/// Gives `thread` one needs-you item with this reason.
fn needs(
    workspace: &mut Workspace,
    thread: &str,
    kind: AttentionKind,
    reason: &str,
    record: Option<String>,
) {
    let current = workspace.store.record(thread).unwrap().unwrap();
    let RecordData::Thread(mut data) = current.data.clone() else {
        unreachable!()
    };
    data.attention = vec![Attention {
        kind,
        reason: reason.into(),
        record,
    }];
    workspace
        .owner_change(
            &format!("attention-{}", uuid::Uuid::new_v4()),
            &ChangeRequest {
                reason: "Synthetic attention".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: thread.into(),
                    expected_version: Some(current.version),
                    archived: false,
                    data: RecordData::Thread(data),
                }],
            },
        )
        .unwrap();
}
fn task(workspace: &mut Workspace, description: &str, state: ActionState) -> String {
    let done = state == ActionState::Done;
    let key = format!("task-{}", uuid::Uuid::new_v4());
    let id = workspace
        .store
        .allocate_operation(&key)
        .unwrap()
        .creation_id(0);
    workspace
        .owner_change(
            &key,
            &ChangeRequest {
                reason: "Synthetic task".into(),
                inputs: vec![],
                writes: vec![Put {
                    id: id.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Action(Action {
                        kept_outside_project: false,
                        owner: None,
                        waiting_on: None,
                        follow_up: None,
                        chat: None,
                        description: description.into(),
                        state,
                        internal: false,
                        evidence: done.then(|| "Synthetic completion evidence".into()),
                        due: Some("2026-10-15".into()),
                    }),
                }],
            },
        )
        .unwrap();
    id
}
fn task_state(cx: &mut TestAppContext, f: &Fixture, id: &str) -> ActionState {
    cx.read(|cx| {
        match f
            .view
            .read(cx)
            .workspace
            .store
            .record(id)
            .unwrap()
            .unwrap()
            .data
        {
            RecordData::Action(action) => action.state,
            _ => unreachable!(),
        }
    })
}

/// U6: the seeded failure text, not the service's own wording.
const FAILURE: &str =
    "Run failed: Reconnect this account before continuing. Continue when the provider is ready.";

#[test]
fn selected_rows_switch_their_metadata_to_ink() {
    let p = pal();
    assert_eq!(components::row_meta_color(true), p.ink);
    assert_eq!(components::row_meta_color(false), p.muted);
}

#[gpui_kit::test]
fn a_needs_you_title_stays_beside_a_long_reason_one_line_each(cx: &mut TestAppContext) {
    let mut thread = String::new();
    let mut long = String::new();
    let f = fixture(cx, 1250., 860., |workspace| {
        thread = workspace.new_thread("Delivery update").unwrap().id;
        needs(workspace, &thread, AttentionKind::Blocker, FAILURE, None);
        long = workspace
            .new_thread(&"A very long chat title that keeps going past the panel ".repeat(3))
            .unwrap()
            .id;
    });
    let row = format!("attention-{thread}");
    on_window(cx, &f, |window, _| {
        // The sidebar row is one line; its full text (title and reason) is
        // its accessible name and tooltip.
        assert_eq!(label(window, &row), format!("Delivery update · {FAILURE}"));
        let frame = window.find(row.clone()).bounds();
        let title = window.find(format!("{row}-title")).bounds();
        assert!(frame.size.width <= px(224.), "the sidebar row: {frame:?}");
        assert!(
            title.size.width > px(60.),
            "the title keeps its room: {title:?}"
        );
        assert!(title.size.height < px(28.), "one title line: {title:?}");
        assert!(frame.size.height <= px(36.5), "a 36 pt row: {frame:?}");
        // A long chat title stays on one line too.
        let chat = format!("thread-{long}");
        assert!(window.find(format!("{chat}-title")).bounds().size.height < px(28.));
        assert!(label(window, &chat).starts_with("A very long chat title"));
        assert_eq!(label(window, "nav-inbox"), "Inbox, 1 needs you");
    });
    // The Inbox names the chat above its reason, one line each.
    click(cx, &f, "nav-inbox");
    let inbox = format!("home-attention-{thread}");
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, &inbox),
            format!("Delivery update · {FAILURE}")
        );
        let frame = window.find(inbox.clone()).bounds();
        let title = window.find(format!("{inbox}-title")).bounds();
        let reason = window.find(format!("{inbox}-reason")).bounds();
        assert!(title.size.height < px(28.), "one title line: {title:?}");
        assert!(reason.size.height < px(24.), "one reason line: {reason:?}");
        assert!(reason.origin.y >= title.origin.y + title.size.height - px(1.));
        for line in [title, reason] {
            assert!(line.origin.x + line.size.width <= frame.origin.x + frame.size.width + px(1.));
        }
        assert!(frame.size.height <= px(45.), "two lines high: {frame:?}");
    });
    click(cx, &f, inbox.clone());
    cx.read(|cx| assert_eq!(f.view.read(cx).chat_thread(), Some(thread.as_str())));
    // The open chat's sidebar row reports its selection.
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find(row.clone()).selected(), Some(true));
        assert_ne!(window.find(format!("thread-{long}")).selected(), Some(true));
    });
}
/// U8: choosing delegation or pausing maintenance says "Saved." in the
/// owner's terms, never the recorded receipt reason.
#[gpui_kit::test]
fn delegation_and_maintenance_notices_never_show_a_receipt_reason(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |_| {});
    click(cx, &f, "nav-inbox");
    click(cx, &f, "delegation-review-first");
    let receipts = |cx: &mut TestAppContext| {
        cx.read(|cx| {
            let view = f.view.read(cx);
            (
                view.notice.text().to_owned(),
                view.history
                    .iter()
                    .map(|r| r.reason.clone())
                    .collect::<Vec<_>>(),
            )
        })
    };
    let (notice, reasons) = receipts(cx);
    assert_eq!(notice, "Saved.");
    assert!(
        reasons
            .iter()
            .all(|reason| !notice.contains(reason.as_str()))
    );
    update(cx, &f, |view, _, cx| {
        focus_tests::settings(view, SettingsSection::Delegation, cx)
    });
    click(cx, &f, "maintenance-setting");
    let (notice, reasons) = receipts(cx);
    assert_eq!(notice, "Saved.");
    assert!(
        reasons
            .iter()
            .all(|reason| !notice.contains(reason.as_str()))
    );
}

/// A protected note's row in the 170 pt Notes list shows the Lock whole
/// rather than "Protect…"; it is announced as Protected.
#[gpui_kit::test]
fn the_library_list_marks_protected_notes_with_the_lock(cx: &mut TestAppContext) {
    let mut note = String::new();
    let f = fixture(cx, 900., 600., |workspace| {
        let record = workspace
            .new_note("A protected decision with a long title", "Decided.")
            .unwrap();
        let RecordData::Note(mut data) = record.data.clone() else {
            unreachable!()
        };
        data.protected = true;
        workspace
            .owner_change(
                "protect-witness",
                &ChangeRequest {
                    reason: "Synthetic protection".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: record.id.clone(),
                        expected_version: Some(record.version),
                        archived: false,
                        data: RecordData::Note(data),
                    }],
                },
            )
            .unwrap();
        note = record.id;
    });
    update(cx, &f, |view, window, cx| {
        view.relayout(window, cx, Layout::document_with_assistant);
        view.show_tile(TileKind::Notes, window, cx);
        view.search.clone().update(cx, |input, cx| {
            input.set_value("A protected decision with a long title", window, cx);
            input.focus(window, cx);
        });
        cx.notify();
    });
    on_window(cx, &f, |window, _| {
        let row = format!("note-{note}");
        let frame = window.find(row.clone()).bounds();
        let lock = window.find(format!("{row}-protected")).bounds();
        assert!(lock.size.width > px(8.) && lock.right() <= frame.right() + px(0.5));
        assert!(label(window, &row).starts_with("A protected decision with a long title"));
        assert!(
            label(window, &row).ends_with("· Protected"),
            "{}",
            label(window, &row)
        );
        assert_eq!(
            window.find(row.clone()).role(),
            Some(gpui_kit::Role::Button)
        );
        assert_eq!(label(window, format!("{row}-protected")), "Protected");
    });

    cx.read(|cx| assert_eq!(f.view.read(cx).open_note_id(), None));
    press(cx, &f, "tab");
    press(cx, &f, "space");
    cx.read(|cx| assert_eq!(f.view.read(cx).open_note_id(), Some(note.as_str())));
}

/// U7: a chat that needs you is listed once in Recent chats, marked, with
/// the title first; the Inbox row keeps the chat title on its first line
/// with the reason under it.
/// The reason is seeded here, not the service's wording.
#[gpui_kit::test]
fn needs_you_rows_keep_the_chat_title_above_the_reason_in_every_list(cx: &mut TestAppContext) {
    let reason = "Run failed: The synthetic provider asked for a new sign-in first.";
    let mut thread = String::new();
    let f = fixture(cx, 1250., 860., |workspace| {
        thread = workspace.new_thread("Venue booking").unwrap().id;
        needs(workspace, &thread, AttentionKind::Blocker, reason, None);
    });
    let two_lines = |window: &mut Window, row: &str| {
        assert_eq!(
            label(window, row),
            format!("Venue booking · {reason}"),
            "{row}"
        );
        let title = window.find(format!("{row}-title")).bounds();
        let second = window.find(format!("{row}-reason")).bounds();
        assert!(title.size.width > px(60.), "{row} title: {title:?}");
        assert!(
            second.top() >= title.bottom() - px(1.),
            "{row}: {title:?} {second:?}"
        );
    };
    on_window(cx, &f, |window, _| {
        assert!(
            window.try_find(format!("thread-{thread}")).is_none(),
            "it is listed once, as needing you"
        );
        // The sidebar row is one line that leads with the title; its
        // accessible name carries the reason.
        let row = format!("attention-{thread}");
        assert_eq!(label(window, &row), format!("Venue booking · {reason}"));
        assert!(window.find(format!("{row}-title")).bounds().size.width > px(60.));
    });
    click(cx, &f, "nav-inbox");
    on_window(cx, &f, |window, _| {
        two_lines(window, &format!("home-attention-{thread}"))
    });
}

#[gpui_kit::test]
fn review_rows_name_their_target_notes_and_mark_a_protected_one(cx: &mut TestAppContext) {
    // U3: the service's reason stays "Review prepared changes".
    let (mut thread, mut operation) = (String::new(), String::new());
    let f = fixture(cx, 1250., 860., |workspace| {
        thread = workspace.new_thread("Delivery chat").unwrap().id;
        let decision = workspace
            .new_note("Delivery decision: 22 November", "Deliver on 22 November.")
            .unwrap();
        let RecordData::Note(mut protected) = decision.data.clone() else {
            unreachable!()
        };
        protected.protected = true;
        workspace
            .owner_change(
                "protect-decision",
                &ChangeRequest {
                    reason: "Protect the decision".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: decision.id.clone(),
                        expected_version: Some(decision.version),
                        archived: false,
                        data: RecordData::Note(protected),
                    }],
                },
            )
            .unwrap();
        let decision = workspace.store.note(&decision.id).unwrap().unwrap();
        let survey = workspace
            .new_note("Survey", "Survey closes Thursday.")
            .unwrap();
        let put = |record: &Record, markdown: &str| {
            let RecordData::Note(mut note) = record.data.clone() else {
                unreachable!()
            };
            note.markdown = markdown.into();
            Put {
                id: record.id.clone(),
                expected_version: Some(record.version),
                archived: false,
                data: RecordData::Note(note),
            }
        };
        let op = workspace
            .store
            .allocate_operation("delivery-proposal")
            .unwrap();
        workspace
            .store
            .prepare(
                &op,
                &ChangeRequest {
                    reason: "Review prepared changes".into(),
                    inputs: vec![],
                    writes: vec![
                        put(&decision, "Deliver on 23 November."),
                        put(&survey, "Survey closes Friday."),
                    ],
                },
            )
            .unwrap();
        operation = op.as_str().to_owned();
        needs(
            workspace,
            &thread,
            AttentionKind::Review,
            "Review prepared changes",
            Some(operation.clone()),
        );
    });
    let expected = "Review changes to Delivery decision: 22 November and Survey";
    on_window(cx, &f, |window, _| {
        // The sidebar's one-line row names the target notes and Protected.
        let row = format!("attention-{thread}");
        let text = label(window, &row);
        assert!(text.contains(expected), "{row}: {text}");
        assert!(text.ends_with("· Protected"), "{row}: {text}");
        assert!(!text.contains("Review · Review prepared changes"), "{text}");
    });
    click(cx, &f, "nav-inbox");
    on_window(cx, &f, |window, _| {
        for row in [
            format!("home-candidate-{operation}"),
            format!("home-attention-{thread}"),
        ] {
            let text = label(window, &row);
            assert!(
                text.contains(expected) && text.ends_with("· Protected"),
                "{text}"
            );
            assert!(window.find(format!("{row}-protected")).visible());
        }
    });
    cx.read(|cx| {
        let view = f.view.read(cx);
        let RecordData::Thread(data) = &view.workspace.store.record(&thread).unwrap().unwrap().data
        else {
            unreachable!()
        };
        assert_eq!(data.attention[0].reason, "Review prepared changes");
    });
}

#[gpui_kit::test]
fn kept_edits_read_recover_edit_with_an_unsaved_chip(cx: &mut TestAppContext) {
    let mut note = String::new();
    let f = fixture(cx, 1250., 860., |workspace| {
        let saved = workspace.new_note("Pilot proposal", "Saved text").unwrap();
        let session = workspace
            .store
            .begin_edit(&saved.id, saved.version)
            .unwrap();
        workspace
            .store
            .update_buffer(&session.id, 1, "Kept edit")
            .unwrap();
        note = saved.id;
    });
    // Kept edits are recovered from the Inbox.
    click(cx, &f, "nav-inbox");
    on_window(cx, &f, |window, _| {
        // R4: the note's title leads, "Recover edit" is the reason line, and
        // the Unsaved marker follows, announced and in the tooltip.
        assert_eq!(
            label(window, "home-recover-0"),
            "Pilot proposal · Recover edit · Unsaved"
        );
        assert_eq!(label(window, "home-recover-0-unsaved"), "Unsaved");
        let title = window.find("home-recover-0-title").bounds();
        let chip = window.find("home-recover-0-unsaved").bounds();
        assert!(title.size.width >= px(90.), "{title:?} {chip:?}");
        assert!(title.right() <= chip.left(), "{title:?} {chip:?}");
        assert!(window.find("home-recover-0-reason").visible());
    });
    click(cx, &f, "home-recover-0");
    cx.read(|cx| assert_eq!(f.view.read(cx).open_note_id(), Some(note.as_str())));
}

#[gpui_kit::test]
fn follow_up_views_show_dates_without_changing_task_state_or_due(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 1000., |_| {});
    let (ids, before) = update(cx, &f, |view, _, cx| {
        let today = lists::follow_up_today().to_string();
        let ids = [
            ("Old follow-up", ActionState::Open, Some("2000-01-01")),
            ("Waiting today", ActionState::Waiting, Some(today.as_str())),
            ("Later follow-up", ActionState::Open, Some("2099-01-01")),
            ("Undated waiting", ActionState::Waiting, None),
            ("Undated open", ActionState::Open, None),
        ]
        .map(|(description, state, date)| {
            let id = task(&mut view.workspace, description, state);
            let record = view.workspace.store.record(&id).unwrap().unwrap();
            let RecordData::Action(mut action) = record.data else {
                panic!()
            };
            action.follow_up = date.map(str::to_owned);
            view.workspace
                .owner_change(
                    &format!("set-follow-up-{id}"),
                    &ChangeRequest {
                        reason: "Synthetic follow-up date".into(),
                        inputs: vec![],
                        writes: vec![Put {
                            id: id.clone(),
                            expected_version: Some(record.version),
                            archived: false,
                            data: RecordData::Action(action),
                        }],
                    },
                )
                .unwrap();
            id
        });
        focus_tests::solo(view, TileKind::Actions, cx);
        (ids, view.workspace.records().unwrap())
    });
    on_window(cx, &f, |window, _| {
        for ((key, group_name), id) in [
            ("overdue", "Overdue follow-ups"),
            ("today", "Follow up today"),
            ("upcoming", "Upcoming follow-ups"),
            ("waiting", "Waiting"),
            ("open", "Open"),
        ]
        .into_iter()
        .zip(&ids)
        {
            let group = window.find(format!("actions-group-{key}"));
            assert!(group.visible());
            assert_eq!(group.role(), Some(gpui_kit::Role::Group));
            assert_eq!(group.label(), Some(format!("{group_name} · 1").as_str()));
            let description = window.find(format!("action-description-{id}"));
            assert_eq!(description.role(), Some(gpui_kit::Role::Label));
            let text = description.label().unwrap().to_owned();
            assert_eq!(description.value(), Some(text.as_str()));
            assert_eq!(
                label(window, format!("task-menu-{id}")),
                format!("Task actions: {text}")
            );
            assert_eq!(
                label(window, format!("done-action-{id}")),
                format!("Confirm done: {text}")
            );
            let facts = window.find(format!("action-facts-{id}"));
            assert_eq!(facts.role(), Some(gpui_kit::Role::Label));
            assert!(facts.label().unwrap().contains("Due"));
            assert_eq!(facts.value(), facts.label());
        }
    });
    update(cx, &f, |view, _, _| {
        assert_eq!(view.workspace.records().unwrap(), before);
        let current = view.workspace.store.record(&ids[1]).unwrap().unwrap();
        let RecordData::Action(action) = current.data else {
            panic!()
        };
        assert_eq!(action.state, ActionState::Waiting);
        assert_eq!(action.due.as_deref(), Some("2026-10-15"));
    });
}

#[gpui_kit::test]
fn each_open_task_has_one_quiet_main_action_and_a_menu_for_its_state(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |_| {});
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::Actions, cx)
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("actions-empty").visible());
        assert_eq!(
            label(window, "actions-empty"),
            "No tasks yet · Tasks keep their own progress, including after a chat is archived."
        );
        // Empty groups are hidden.
        for key in ["suggested", "open", "waiting", "done", "cancelled"] {
            assert!(
                window.try_find(format!("actions-group-{key}")).is_none(),
                "{key}"
            );
        }
    });
    let ids = update(cx, &f, |view, _, cx| {
        let ids = [
            ActionState::Suggested,
            ActionState::Open,
            ActionState::Waiting,
            ActionState::Done,
            ActionState::Cancelled,
        ]
        .map(|state| {
            task(
                &mut view.workspace,
                &format!("{state:?} synthetic task"),
                state,
            )
        });
        view.refresh();
        cx.notify();
        ids
    });
    let [suggested, open, waiting, done, cancelled] = ids.clone();
    for (id, main, absent, items) in [
        (
            &suggested,
            "open-action",
            "done-action",
            vec!["Edit details", "Waiting", "Confirm done", "Cancel task"],
        ),
        (
            &open,
            "done-action",
            "open-action",
            vec!["Edit details", "Waiting", "Cancel task"],
        ),
        (
            &waiting,
            "done-action",
            "open-action",
            vec!["Edit details", "Mark open", "Cancel task"],
        ),
    ] {
        on_window(cx, &f, |window, _| {
            let shown = window.find(format!("{main}-{id}"));
            assert!(shown.visible(), "{main}-{id}");
            assert_eq!(
                shown.label(),
                Some(if id == &suggested {
                    "Accept task: Suggested synthetic task"
                } else if id == &open {
                    "Confirm done: Open synthetic task"
                } else {
                    "Confirm done: Waiting synthetic task"
                })
            );
            assert!(window.try_find(format!("{absent}-{id}")).is_none());
            assert!(window.try_find(format!("wait-action-{id}")).is_none());
            assert!(window.try_find(format!("cancel-action-{id}")).is_none());
        });
        click(cx, &f, format!("task-menu-{id}"));
        assert_eq!(menu_rows(cx, &f), items, "{id}");
        close_menu(cx, &f);
    }
    // Done and Cancelled start collapsed and keep their counts. Their
    // metadata remains editable, but no state transition is offered.
    for (key, id) in [("done", &done), ("cancelled", &cancelled)] {
        on_window(cx, &f, |window, _| {
            assert!(window.find(format!("actions-group-{key}")).visible());
            assert!(window.try_find(format!("action-tile-{id}")).is_none());
            assert_eq!(label(window, format!("tasks-{key}-toggle")), "Show 1");
        });
        click(cx, &f, format!("tasks-{key}-toggle"));
        on_window(cx, &f, |window, _| {
            assert!(window.find(format!("action-tile-{id}")).visible());
            assert!(window.find(format!("task-menu-{id}")).visible());
            assert!(window.try_find(format!("done-action-{id}")).is_none());
            assert!(window.try_find(format!("open-action-{id}")).is_none());
        });
        click(cx, &f, format!("task-menu-{id}"));
        assert_eq!(menu_rows(cx, &f), vec!["Edit details"]);
        close_menu(cx, &f);
    }
    // The menu and the main action run the same checked change.
    click(cx, &f, format!("task-menu-{waiting}"));
    choose(cx, &f, "Mark open");
    assert_eq!(task_state(cx, &f, &waiting), ActionState::Open);
    click(cx, &f, format!("open-action-{suggested}"));
    assert_eq!(task_state(cx, &f, &suggested), ActionState::Open);
    click(cx, &f, format!("task-menu-{open}"));
    choose(cx, &f, "Cancel task");
    assert_eq!(task_state(cx, &f, &open), ActionState::Cancelled);
    cx.read(|cx| {
        let history = f.view.read(cx).workspace.history().unwrap();
        let reasons: Vec<_> = history
            .iter()
            .rev()
            .take(3)
            .map(|r| r.reason.as_str())
            .collect();
        assert_eq!(
            reasons,
            [
                "Owner cancelled a task",
                "Owner opened a task",
                "Owner opened a task"
            ]
        );
    });
}

#[test]
fn task_steps_offer_only_changes_of_state() {
    use lists::{TaskStep::*, task_steps};
    assert_eq!(
        task_steps(&ActionState::Suggested),
        (Some(Accept), vec![Waiting, ConfirmDone, Cancel])
    );
    assert_eq!(
        task_steps(&ActionState::Open),
        (Some(ConfirmDone), vec![Waiting, Cancel])
    );
    assert_eq!(
        task_steps(&ActionState::Waiting),
        (Some(ConfirmDone), vec![MarkOpen, Cancel])
    );
    for state in [ActionState::Done, ActionState::Cancelled] {
        assert_eq!(task_steps(&state), (None, vec![]));
    }
}

/// A saved change to `input` that leaves a derived note needing refresh.
fn derived_change(workspace: &mut Workspace) -> (Receipt, String) {
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
    let derived = operation.creation_id(0);
    workspace
        .store
        .prepare(
            &operation,
            &ChangeRequest {
                reason: "Derived output".into(),
                writes: vec![Put {
                    id: derived.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Note(Note::working("Named derived note", "Derived wording")),
                }],
                inputs: vec![RevisionInput {
                    record: input.id.clone(),
                    version: input.version,
                    expectation: InputExpectation::Current,
                }],
            },
        )
        .unwrap();
    workspace
        .store
        .apply(
            &operation,
            &HostAuthority::maintenance("synthetic derivation"),
        )
        .unwrap();
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
    assert_eq!(receipt.needs_refresh, vec![derived.clone()]);
    (receipt, derived)
}

#[gpui_kit::test]
fn background_changes_group_and_keep_undo_and_refresh_changes_never_group(cx: &mut TestAppContext) {
    let mut refreshed = None;
    let f = fixture(cx, 1250., 860., |workspace| {
        workspace.choose_quiet("quiet-fixture", true).unwrap();
        refreshed = Some(derived_change(workspace));
    });
    let (receipt, derived) = refreshed.unwrap();
    let op = receipt.operation.as_str().to_owned();
    let background: Vec<Receipt> = cx.read(|cx| {
        f.view
            .read(cx)
            .history
            .iter()
            .filter(|r| lists::is_background(r))
            .cloned()
            .collect()
    });
    assert!(background.len() >= 2, "initialize and the Quiet change");
    let quiet = background
        .iter()
        .find(|r| r.reason == "Chat control: change display detail")
        .unwrap()
        .operation
        .as_str()
        .to_owned();
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::History, cx)
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("history-background").visible());
        assert_eq!(
            label(window, "history-background-toggle"),
            format!("Show {}", background.len())
        );
        assert!(window.try_find(format!("receipt-{quiet}")).is_none());
        // A change that needs refresh is never background, and keeps its
        // sub-row and follow-up.
        assert!(window.find(format!("receipt-{op}")).visible());
        assert_eq!(
            label(window, format!("receipt-refresh-{op}")),
            "May need refresh"
        );
        assert_eq!(
            label(window, format!("refresh-record-{op}-{derived}")),
            "May need refresh: Named derived note"
        );
        assert!(window.find(format!("refresh-followup-{op}")).visible());
        assert_eq!(
            label(window, format!("undo-{op}")),
            format!("Undo: {}", receipt.reason)
        );
        // No raw version or chat-control text in the rows.
        assert!(!label(window, format!("receipt-reason-{op}")).contains("version"));
    });
    click(cx, &f, "history-background-toggle");
    on_window(cx, &f, |window, _| {
        assert!(window.find(format!("receipt-{quiet}")).visible());
        assert_eq!(
            label(window, format!("receipt-reason-{quiet}")),
            "Change display detail"
        );
        assert_eq!(
            label(window, format!("undo-{quiet}")),
            "Undo: Change display detail"
        );
        assert_eq!(label(window, "history-background-toggle"), "Hide");
    });
    // Background changes stay undoable.
    click(cx, &f, format!("undo-{quiet}"));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(
            view.history
                .last()
                .unwrap()
                .undo_of
                .as_ref()
                .map(OperationId::as_str),
            Some(quiet.as_str())
        );
        assert!(!view.queue_state().quiet);
    });
    // Home leaves bookkeeping out and marks the change that needs refresh.
    click(cx, &f, "nav-inbox");
    on_window(cx, &f, |window, _| {
        assert!(window.try_find(format!("home-change-{quiet}")).is_none());
        assert_eq!(
            label(window, format!("home-change-{op}-refresh")),
            "Needs refresh"
        );
        assert!(window.find("home-open-history").visible());
    });
}

#[gpui_kit::test]
fn home_shows_the_intro_only_on_first_run(cx: &mut TestAppContext) {
    let first = fixture(cx, 1250., 860., |_| {});
    click(cx, &first, "nav-inbox");
    on_window(cx, &first, |window, _| {
        assert!(window.find("home-intro").visible());
        assert!(window.find("delegation-pending").visible());
    });
    let mut note = String::new();
    let seeded = fixture(cx, 1250., 860., |workspace| {
        note = workspace
            .new_note("Pilot proposal", "Saved text")
            .unwrap()
            .id;
    });
    click(cx, &seeded, "nav-inbox");
    on_window(cx, &seeded, |window, _| {
        assert!(window.try_find("home-intro").is_none());
        // The pending choice is independent of first run.
        assert!(window.find("delegation-pending").visible());
        // Recent notes are rows with a readable time, not buttons.
        let row = window.find(format!("home-note-{note}"));
        assert!(row.visible());
        assert!(
            row.label().unwrap().starts_with("Pilot proposal · Today"),
            "{:?}",
            row.label()
        );
        assert_ne!(row.role(), Some(gpui_kit::Role::Button));
    });
}

/// The reworked empty states on first run: a title, one sentence naming the
/// next route, and at most one button.
#[gpui_kit::test]
fn first_run_empty_states_name_the_next_step(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |_| {});
    for (kind, id, text) in [
        (
            TileKind::Note,
            "note-empty",
            "No note open · Open one from Notes or ⌘K.",
        ),
        (
            TileKind::Notes,
            "notes-empty",
            "No notes yet · Create one with ⌘N, or add information with +.",
        ),
        (
            TileKind::Projects,
            "projects-empty",
            "No projects yet · Group related chats and documents with shared context. Start with New project.",
        ),
        (
            TileKind::Connections,
            "connections-empty",
            "No note open · Open a note to see the notes and sources it links with.",
        ),
    ] {
        update(cx, &f, |view, window, cx| view.show_tile(kind, window, cx));
        on_window(cx, &f, |window, _| {
            assert_eq!(label(window, id), text, "{kind:?}");
        });
    }
    // The Note's one button is New note.
    update(cx, &f, |view, window, cx| {
        view.show_tile(TileKind::Note, window, cx)
    });
    on_window(cx, &f, |window, _| {
        let empty = window.within("note-empty");
        assert_eq!(empty.find("new-note").label(), Some("New note"));
    });
}

/// In the narrow Write companion at 900×600 the pending delegation card
/// keeps its text inside: the question on one line and each explanation
/// wrapping within the card.
#[gpui_kit::test]
fn the_delegation_card_keeps_its_text_inside_a_narrow_chat(cx: &mut TestAppContext) {
    let f = fixture(cx, 900., 600., |_| {});
    update(cx, &f, |view, window, cx| {
        // New chat fills the workspace; then show it as the narrow assistant.
        view.new_thread(window, cx);
        view.relayout(window, cx, Layout::narrow_assistant);
    });
    on_window(cx, &f, |window, cx| {
        assert!(f.view.read(cx).chat_narrow);
        let card = window
            .within("tile-chat")
            .find("delegation-pending")
            .bounds();
        let question = window.find("delegation-question").bounds();
        let line = window.find("delegation-status").bounds();
        assert!(question.right() <= card.right(), "{question:?} in {card:?}");
        assert_eq!(
            label(window, "delegation-question"),
            "How should BRN update your notes?"
        );
        // The last word keeps its "?": wider than the mark alone, inside.
        let end = window.find("delegation-question-end").bounds();
        assert!(end.size.width > px(24.), "{end:?}");
        assert!(end.right() <= card.right() && end.bottom() <= question.bottom());
        assert!(line.right() <= card.right());
        for id in ["delegation-maintain-why", "delegation-review-first-why"] {
            let why = window.find(id).bounds();
            assert!(
                why.left() >= card.left() && why.right() <= card.right() + px(0.5),
                "{id} {why:?} inside {card:?}"
            );
        }
    });
}

#[gpui_kit::test]
fn delegation_is_two_radios_and_a_switch_in_settings_and_two_quiet_buttons_on_home(
    cx: &mut TestAppContext,
) {
    let f = fixture(cx, 1250., 860., |_| {});
    click(cx, &f, "nav-inbox");
    on_window(cx, &f, |window, _| {
        for (id, text) in [
            (
                "delegation-maintain",
                "Maintain ordinary notes: BRN updates ordinary notes and source references.",
            ),
            (
                "delegation-review-first",
                "Review changes first: BRN proposes; you approve.",
            ),
        ] {
            let choice = window.within("delegation-pending").find(id);
            assert_eq!(choice.role(), Some(gpui_kit::Role::Button), "{id}");
            assert_eq!(choice.label(), Some(text));
        }
    });
    update(cx, &f, |view, _, cx| {
        focus_tests::settings(view, SettingsSection::Delegation, cx)
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "delegation-choice"),
            "How should BRN update your notes?"
        );
        for id in ["delegation-maintain", "delegation-review-first"] {
            let radio = window.find(id);
            assert_eq!(radio.role(), Some(gpui_kit::Role::RadioButton), "{id}");
            assert_ne!(
                radio.checked(),
                Some(true),
                "nothing is chosen while pending"
            );
        }
        assert!(window.try_find("maintenance-setting").is_none());
    });
    click(cx, &f, "delegation-review-first");
    on_window(cx, &f, |window, _| {
        assert_eq!(window.find("delegation-review-first").checked(), Some(true));
        assert_eq!(window.find("delegation-maintain").checked(), Some(false));
        let switch = window.find("maintenance-setting");
        assert_eq!(switch.role(), Some(gpui_kit::Role::Switch));
        assert_eq!(switch.label(), Some("Maintenance: Running"));
    });
    let saved = Workspace::open(&f.data).unwrap().settings().unwrap();
    assert!(saved.delegation_chosen && saved.review_first && saved.maintenance);
}

#[gpui_kit::test]
fn the_helper_section_shows_its_tested_note_and_saves_with_settings(cx: &mut TestAppContext) {
    // Tall enough to show the whole Models section without scrolling.
    let f = fixture(cx, 1250., 1500., |_| {});
    let auth = f._root.path().join("synthetic-auth.json");
    let auth = auth.to_str().unwrap().to_owned();
    update(cx, &f, |view, window, cx| {
        focus_tests::settings(view, SettingsSection::Models, cx);
        view.auth_path
            .update(cx, |input, cx| input.set_value(auth.clone(), window, cx));
    });
    let canonical_before = Workspace::open_without_default_credentials(&f.data)
        .unwrap()
        .settings()
        .unwrap();
    let history_before = Workspace::open_without_default_credentials(&f.data)
        .unwrap()
        .history()
        .unwrap();
    on_window(cx, &f, |window, _| {
        let note = window.find("settings-helper-tested");
        assert!(note.visible());
        assert_eq!(note.label(), Some(brn_threads_app::HELPER_TESTED_NOTE));
        assert_eq!(window.find("effort-high").label(), Some("High"));
        assert!(window.find("helper-provider-off").visible());
        assert!(window.try_find("helper-effort-high").is_none());
        assert!(window.find("save-settings").visible());
        assert!(window.try_find("save-settings-reason").is_none());
    });
    click(cx, &f, "helper-provider-Codex");
    on_window(cx, &f, |window, _| {
        assert!(window.find("helper-effort-high").visible());
        assert!(window.find("settings-helper-tested").visible());
    });
    click(cx, &f, "helper-effort-low");
    click(cx, &f, "helper-provider-off");
    click(cx, &f, "helper-provider-Codex");
    click(cx, &f, "effort-high");
    let model_after_refusal = "offline-model-after-refusal";
    let relative_auth = "relative/synthetic-auth.json";
    update(cx, &f, |view, window, cx| {
        view.model.update(cx, |input, cx| {
            input.set_value(model_after_refusal, window, cx)
        });
        view.auth_path
            .update(cx, |input, cx| input.set_value(relative_auth, window, cx));
    });
    click(cx, &f, "save-settings");

    let after_refusal = Workspace::open_without_default_credentials(&f.data).unwrap();
    assert_eq!(after_refusal.settings().unwrap(), canonical_before);
    assert_eq!(after_refusal.history().unwrap(), history_before);
    update(cx, &f, |view, _, cx| {
        assert!(view.notice.text().contains("absolute paths"));
        assert_eq!(view.model.read(cx).value(), model_after_refusal);
        assert_eq!(view.auth_path.read(cx).value(), relative_auth);
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find("helper-effort-high").visible());
        assert!(window.find("effort-high").visible());
    });
    update(cx, &f, |view, window, cx| {
        view.auth_path
            .update(cx, |input, cx| input.set_value(auth.clone(), window, cx));
    });
    click(cx, &f, "save-settings");
    let saved = Workspace::open(&f.data).unwrap().settings().unwrap();
    assert_eq!(saved.provider, "offline-fixture");
    assert_eq!(saved.effort, "high");
    assert_eq!(saved.model, model_after_refusal);
    assert_eq!(saved.auth_file.as_deref(), Some(auth.as_str()));
    assert_eq!(
        saved.helper,
        Some(brn_threads_app::HelperSettings {
            provider: "Codex".into(),
            model: "gpt-6-luna".into(),
            effort: "high".into(),
            auth_file: Some(auth.clone()),
            credentials_dir: None,
        })
    );
    // While work runs, both Models buttons announce why they wait.
    let _sender = update(cx, &f, |view, _, cx| {
        let (sender, receiver) = mpsc::sync_channel(1);
        view.worker = Some(Worker::output(receiver, true));
        cx.notify();
        sender
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "save-settings"),
            "Save settings, unavailable: BRN is working"
        );
        assert!(window.find("save-settings-reason").visible());
        assert_eq!(
            label(window, "connect-copilot"),
            "Connect GitHub Copilot, unavailable: BRN is working"
        );
    });
}

#[gpui_kit::test]
fn copilot_connection_does_not_save_an_unsaved_effort_choice(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 1500., |_| {});
    let credentials = f
        ._root
        .path()
        .join("synthetic-copilot")
        .display()
        .to_string();
    update(cx, &f, |view, _, cx| {
        focus_tests::settings(view, SettingsSection::Models, cx);
    });
    click(cx, &f, "effort-high");

    assert_eq!(
        Workspace::open_without_default_credentials(&f.data)
            .unwrap()
            .settings()
            .unwrap()
            .effort,
        "medium"
    );

    update(cx, &f, |view, window, cx| {
        let (sender, receiver) = mpsc::sync_channel(2);
        view.worker = Some(Worker::connect(receiver, CancellationToken::new()));
        sender
            .send(WorkerEvent::CopilotConnected(credentials.clone()))
            .unwrap();
        sender
            .send(WorkerEvent::Finished(Ok(
                "Synthetic Copilot connection finished".into(),
            )))
            .unwrap();
        view.poll_worker(window, cx);
        assert_eq!(view.settings.provider, "copilot");
        assert_eq!(view.provider_form.provider, "copilot");
        assert_eq!(view.provider_form.effort, "medium");
    });

    let saved = Workspace::open_without_default_credentials(&f.data)
        .unwrap()
        .settings()
        .unwrap();
    assert_eq!(saved.provider, "copilot");
    assert_eq!(saved.credentials_dir.as_deref(), Some(credentials.as_str()));
    assert_eq!(saved.effort, "medium");
}

/// Recent chats' empty state names why it lists nothing.
fn recent_empty(cx: &mut TestAppContext, f: &Fixture) -> String {
    on_window(cx, f, |window, _| {
        assert!(window.find("threads-empty").visible());
        label(window, "threads-empty")
    })
}

#[gpui_kit::test]
fn recent_chats_says_no_chats_yet_only_without_chats(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |_| {});
    assert_eq!(recent_empty(cx, &f), "No chats yet · Start one with ⌘T.");
}

#[gpui_kit::test]
fn recent_chats_under_a_chat_that_needs_you_says_no_other_open_chats(cx: &mut TestAppContext) {
    let mut thread = String::new();
    let f = fixture(cx, 1250., 860., |workspace| {
        thread = workspace.new_thread("Delivery update").unwrap().id;
        needs(workspace, &thread, AttentionKind::Blocker, FAILURE, None);
    });
    on_window(cx, &f, |window, _| {
        assert!(window.find(format!("attention-{thread}")).visible());
    });
    assert_eq!(recent_empty(cx, &f), "No other open chats.");
}

#[gpui_kit::test]
fn recent_chats_with_only_archived_chats_points_to_archived(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |workspace| {
        let thread = workspace.new_thread("Old question").unwrap().id;
        workspace.resolve_thread(&thread).unwrap();
    });
    assert_eq!(
        recent_empty(cx, &f),
        "No open chats · Archived chats are in All chats."
    );
    click(cx, &f, "nav-all-chats");
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "resolved-count"), "Archived · 1");
    });
}

#[gpui_kit::test]
fn preparing_search_shows_a_spinner_with_its_text(cx: &mut TestAppContext) {
    let f = fixture(cx, 1250., 860., |workspace| {
        workspace.new_note("Pilot proposal", "Saved text").unwrap();
    });
    update(cx, &f, |view, _, cx| {
        focus_tests::solo(view, TileKind::Notes, cx);
        view.results.preparing = true;
    });
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "search-preparing"), "Preparing search…");
        // The local views sit in one compact row above the list.
        assert!(window.find("library-views").bounds().size.height <= px(40.));
        assert_eq!(label(window, "library-articles"), "Articles · 1");
        assert_eq!(label(window, "library-files"), "All files · 1");
        assert_eq!(label(window, "topics"), "Topics · Create Topics note");
    });
}

fn press(cx: &mut TestAppContext, f: &Fixture, key: &str) {
    on_window(cx, f, |window, cx| window.press(key, cx));
}
/// Tab until `id` has focus; false if it never does.
fn tab_to(cx: &mut TestAppContext, f: &Fixture, id: &str) -> bool {
    (0..80).any(|_| {
        press(cx, f, "tab");
        on_window(cx, f, |window, _| {
            window.try_find(id.to_owned()).and_then(|e| e.focused()) == Some(true)
        })
    })
}

/// Every Inbox item keeps a keyboard route: a note and its kept edit
/// through ⌘K, a review through its sidebar chat row and then Tab
/// into the chat's review card, and a change through ⌘K Recent changes and
/// its Undo buttons.
#[gpui_kit::test]
fn each_home_item_keeps_a_keyboard_route(cx: &mut TestAppContext) {
    let (mut kept, mut thread, mut operation) = (String::new(), String::new(), String::new());
    let f = fixture(cx, 1400., 1000., |workspace| {
        let note = workspace.new_note("Kept plan", "Saved text").unwrap();
        let edit = workspace.store.begin_edit(&note.id, note.version).unwrap();
        workspace
            .store
            .update_buffer(&edit.id, 1, "Kept text")
            .unwrap();
        kept = note.id;
        thread = workspace.new_thread("Review chat").unwrap().id;
        let survey = workspace
            .new_note("Survey", "Survey closes Thursday.")
            .unwrap();
        let RecordData::Note(mut data) = survey.data.clone() else {
            unreachable!()
        };
        data.markdown = "Survey closes Friday.".into();
        let op = workspace.store.allocate_operation("home-route").unwrap();
        workspace
            .store
            .prepare(
                &op,
                &ChangeRequest {
                    reason: "Review prepared changes".into(),
                    inputs: vec![],
                    writes: vec![Put {
                        id: survey.id.clone(),
                        expected_version: Some(survey.version),
                        archived: false,
                        data: RecordData::Note(data),
                    }],
                },
            )
            .unwrap();
        operation = op.as_str().to_owned();
        needs(
            workspace,
            &thread,
            AttentionKind::Review,
            "Review prepared changes",
            Some(operation.clone()),
        );
    });
    click(cx, &f, "nav-inbox");
    on_window(cx, &f, |window, _| {
        assert!(window.find("home-recover-0").visible());
        assert!(window.find(format!("home-candidate-{operation}")).visible());
    });
    // The note and its kept edit: ⌘K opens the note.
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, cx| window.input("Kept plan", cx));
    press(cx, &f, "enter");
    // Opening the note resumes its kept edit, as the Home row does.
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(kept.as_str()));
        assert!(
            view.writing
                .note
                .as_ref()
                .is_some_and(|open| open.session().is_some())
        );
        assert_eq!(view.writing.note_editor.read(cx).value(), "Kept text");
    });
    // The review: the chat that needs you leads Recent chats in the
    // sidebar; Tab past Projects reaches its row and ↵ opens the chat.
    update(cx, &f, |view, window, cx| view.focus_root(window, cx));
    assert!(tab_to(cx, &f, "nav-projects"), "Tab reaches Projects");
    press(cx, &f, "tab");
    press(cx, &f, "enter");
    cx.read(|cx| assert_eq!(f.view.read(cx).chat_thread(), Some(thread.as_str())));
    // The composer takes Tab for indentation, so continue from the
    // workspace into the chat's review card.
    update(cx, &f, |view, window, cx| view.focus_root(window, cx));
    on_window(cx, &f, |window, cx| {
        window.simulate_next_frame(cx);
    });
    let review = format!("thread-review-{operation}");
    assert!(tab_to(cx, &f, &review), "Tab reaches Show in note");
    press(cx, &f, "space");
    cx.read(|cx| assert!(f.view.read(cx).writing.review.is_some()));
    // The change: ⌘K opens Changes, where each change has an Undo button.
    update(cx, &f, |view, window, cx| view.keep_review(window, cx));
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, cx| window.input("Changes", cx));
    press(cx, &f, "enter");
    cx.read(|cx| assert_eq!(f.view.read(cx).layout.focus(), Some(TileKind::History)));
    let undo = cx.read(|cx| {
        let view = f.view.read(cx);
        let receipt = view
            .history
            .iter()
            .rev()
            .find(|receipt| receipt.writes.iter().any(|w| w.after.id == kept))
            .unwrap();
        format!("undo-{}", receipt.operation.as_str())
    });
    assert!(tab_to(cx, &f, &undo), "Tab reaches {undo}");
}

/// The saved Linux AT tree omitted these labeled rows entirely. Keyboard
/// activation must use the same open-chat callback without creating a Run.
#[gpui_kit::test]
fn chat_rows_are_named_controls_and_enter_space_select_without_run(cx: &mut TestAppContext) {
    for (key, attention) in [("enter", false), ("space", true)] {
        let mut chat = String::new();
        let f = fixture(cx, 1250., 1000., |workspace| {
            chat = workspace.new_thread("Keyboard chat Ω").unwrap().id;
            workspace
                .post_message(&chat, "Synthetic posted context")
                .unwrap();
            if attention {
                needs(
                    workspace,
                    &chat,
                    AttentionKind::Question,
                    "Owner input needed",
                    None,
                );
            }
        });
        let before = cx.read(|cx| f.view.read(cx).workspace.records().unwrap());
        let row = format!("{}-{chat}", if attention { "attention" } else { "thread" });
        on_window(cx, &f, |window, _| {
            let row = window.find(row.clone());
            assert_eq!(row.role(), Some(gpui_kit::Role::Button));
            assert!(row.label().unwrap().starts_with("Keyboard chat Ω"));
            assert_eq!(row.selected(), Some(false));
        });
        // Locate the preceding sidebar control by actual keyboard traversal.
        update(cx, &f, |view, window, cx| view.focus_root(window, cx));
        assert!(tab_to(cx, &f, "nav-projects"));
        press(cx, &f, "tab");
        press(cx, &f, key);
        cx.read(|cx| {
            let ui = f.view.read(cx);
            assert_eq!(ui.chat_thread(), Some(chat.as_str()));
            assert_eq!(ui.workspace.records().unwrap(), before);
            assert!(ui.worker.is_none());
        });
        on_window(cx, &f, |window, _| {
            assert_eq!(window.find(row).selected(), Some(true))
        });
        click(cx, &f, "nav-all-chats");
        on_window(cx, &f, |window, _| {
            let row = window.find(format!("all-thread-{chat}"));
            assert_eq!(row.role(), Some(gpui_kit::Role::Button));
            assert!(row.label().unwrap().starts_with("Keyboard chat Ω"));
        });
    }
}

/// Discriminate the preserved Linux Tasks-menu/chat-focus mismatch. This uses
/// the test platform's GPUI events, not AT-SPI or a native window acceptance run.
#[gpui_kit::test]
fn chat_focus_dispatch_with_tasks_panel_and_split_enter(cx: &mut TestAppContext) {
    use gpui_kit::test::TestEventExt;
    use std::{cell::RefCell, rc::Rc};

    for redraw in [false, true] {
        let mut chat = String::new();
        let f = fixture(cx, 1250., 1000., |workspace| {
            chat = workspace
                .new_thread("Synthetic outcomes conversation")
                .unwrap()
                .id;
            workspace
                .post_message(&chat, "Synthetic posted context")
                .unwrap();
        });
        update(cx, &f, |view, window, cx| {
            view.show_tile(TileKind::Actions, window, cx);
        });
        let before = cx.read(|cx| f.view.read(cx).workspace.records().unwrap());
        let history = cx.read(|cx| f.view.read(cx).workspace.history().unwrap());
        let events = Rc::new(RefCell::new(Vec::new()));
        let observed = events.clone();
        let _observer = cx.update(|cx| {
            cx.observe_keystrokes(move |event, window, cx| {
                observed.borrow_mut().push((
                    event.keystroke.key.clone(),
                    event.action.as_ref().map(|action| action.name().to_owned()),
                    format!("{:?}", event.context_stack),
                    format!("{:?}", window.focused(cx)),
                ));
            })
        });
        on_window(cx, &f, |window, _| {
            assert!(window.find("inbox-tasks").visible());
            assert!(window.find("nav-projects").visible());
            eprintln!(
                "diagnostic redraw={redraw}: a11y_active={}",
                window.is_a11y_active()
            );
        });
        // Keys only from the workspace root to the sidebar control before the
        // chat row; navigation never injects row focus.
        update(cx, &f, |view, window, cx| view.focus_root(window, cx));
        let row = format!("thread-{chat}");
        assert!(tab_to(cx, &f, "nav-projects"), "Tab reaches Projects");
        on_window(cx, &f, |window, _| {
            assert!(window.try_find("popup-menu").is_none());
        });
        let archived = on_window(cx, &f, |window, cx| window.focused(cx).unwrap());
        press(cx, &f, "tab");
        let focused = on_window(cx, &f, |window, cx| window.focused(cx).unwrap());
        // Kit cannot query the row's implicit focus binding. Observe GPUI's
        // actual handle without replacing it; the expected selection on key-up
        // below establishes which row receives activation.
        on_window(cx, &f, |window, cx| {
            assert!(!archived.is_focused(window));
            assert!(focused.is_focused(window));
            eprintln!(
                "diagnostic after row Tab: focus={:?}, contexts={:?}, row={:?}",
                window.focused(cx),
                window.context_stack(),
                window.find(row.clone())
            );
            assert!(!format!("{:?}", window.context_stack()).contains("Popover"));
        });
        if redraw {
            on_window(cx, &f, |window, cx| window.render_frame(cx));
        }
        events.borrow_mut().clear();
        on_window(cx, &f, |window, cx| {
            assert!(focused.is_focused(window));
            window.key_down("enter", false, cx);
        });
        eprintln!(
            "diagnostic Enter down redraw={redraw}: {:?}",
            events.borrow()
        );
        on_window(cx, &f, |window, _| {
            assert!(focused.is_focused(window));
            assert!(
                window.try_find("popup-menu").is_none(),
                "Enter down opened a menu"
            );
        });
        cx.read(|cx| assert!(f.view.read(cx).chat_thread().is_none()));
        let observed = events.borrow();
        let enter = observed
            .iter()
            .find(|event| event.0 == "enter")
            .expect("Enter observed");
        assert!(enter.1.is_none(), "unexpected action: {enter:?}");
        assert!(
            !enter.2.contains("Popover"),
            "unexpected popup ancestry: {enter:?}"
        );
        drop(observed);
        on_window(cx, &f, |window, cx| window.key_up("enter", cx));
        on_window(cx, &f, |window, _| {
            assert!(
                window.try_find("popup-menu").is_none(),
                "Enter up opened a menu"
            );
            assert_eq!(window.find(row.clone()).selected(), Some(true));
        });
        cx.read(|cx| {
            let ui = f.view.read(cx);
            assert_eq!(ui.chat_thread(), Some(chat.as_str()));
            assert_eq!(ui.workspace.records().unwrap(), before);
            assert_eq!(ui.workspace.history().unwrap(), history);
            assert!(
                before
                    .iter()
                    .all(|record| !matches!(record.data, RecordData::Run(_)))
            );
            assert!(ui.worker.is_none());
        });
    }
}

/// S5 Library (spec §6.4, AC-08/09/26): Articles is the default and leaves
/// out full imported references; All files lists them and retained sources
/// without a readable import; a source states its availability limit; a
/// row opens the same note for reading with the assistant closed; Knowledge
/// checks is in the Library menu; the launcher names type and scope and
/// never lists project-only documents.
#[gpui_kit::test]
fn library_articles_files_sources_checks_and_search_scope(cx: &mut TestAppContext) {
    let mut ids = (
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    );
    let f = fixture(cx, 1280., 900., |workspace| {
        let article = workspace
            .new_note(
                "Harbor delivery",
                "# Harbor delivery\n\nCurrent plan for the berth.",
            )
            .unwrap()
            .id;
        let source = |label: &str| {
            RecordData::Source(Source {
                chat: None,
                locator: format!("/nonexistent-brn-fixture/{label}"),
                label: label.into(),
                observed_at: "2026-11-01T09:00:00Z".into(),
                external_version: None,
                outcome: "converted".into(),
                gaps: vec![],
                report: None,
            })
        };
        let owner = HostAuthority::owner("fixture");
        let op = workspace
            .store
            .allocate_operation("s5-library-fixture")
            .unwrap();
        let (survey, loose, note) = (op.creation_id(0), op.creation_id(1), op.creation_id(2));
        let mut reference = Note::working("Harbor survey", "Survey text, kept faithfully.");
        reference.protected = true;
        reference.import = Some(ImportCoverage {
            source: survey.clone(),
            full_note: true,
            gaps: vec![],
            report: None,
        });
        let put = |id: &str, data: RecordData| Put {
            id: id.into(),
            expected_version: None,
            archived: false,
            data,
        };
        let request = ChangeRequest {
            reason: "Synthetic Library fixture".into(),
            writes: vec![
                put(&survey, source("harbor-survey.pdf")),
                put(&loose, source("harbor-tides.csv")),
                put(&note, RecordData::Note(reference)),
            ],
            inputs: vec![],
        };
        workspace
            .store
            .prepare_owner(&op, &request, &owner)
            .unwrap();
        assert!(matches!(
            workspace.store.apply(&op, &owner).unwrap(),
            ApplyOutcome::Applied(_)
        ));
        // One ordinary update committed under the standing delegation.
        workspace
            .choose_delegation(DelegationChoice::MaintainOrdinaryNotes)
            .unwrap();
        let current = workspace.store.record(&article).unwrap().unwrap();
        let RecordData::Note(mut updated) = current.data.clone() else {
            unreachable!()
        };
        updated.markdown.push_str("\nBerth two opens in May.\n");
        let op = workspace.store.allocate_operation("s5-delegated").unwrap();
        workspace
            .store
            .prepare(
                &op,
                &ChangeRequest {
                    reason: "Delegated maintenance fixture".into(),
                    writes: vec![Put {
                        id: article.clone(),
                        expected_version: Some(current.version),
                        archived: false,
                        data: RecordData::Note(updated),
                    }],
                    inputs: vec![],
                },
            )
            .unwrap();
        assert!(matches!(
            workspace
                .store
                .apply(&op, &HostAuthority::maintenance("fixture"))
                .unwrap(),
            ApplyOutcome::Applied(_)
        ));
        let receipt = workspace
            .project_command(
                "s5-project",
                ProjectCommand::Create {
                    title: "Harbor project".into(),
                },
            )
            .unwrap();
        let project = receipt.writes[0].after.id.clone();
        let doc = workspace
            .project_command(
                "s5-project-doc",
                ProjectCommand::CreateDocument {
                    project: project.clone(),
                    title: "Harbor private".into(),
                    markdown: "Only in the project.".into(),
                },
            )
            .unwrap()
            .writes
            .iter()
            .find(|w| matches!(w.after.data, RecordData::Note(_)))
            .unwrap()
            .after
            .id
            .clone();
        ids = (article, note, loose, doc, survey, project);
    });
    let (article, reference, loose, private, survey, project) = ids;
    click(cx, &f, "nav-library");
    on_window(cx, &f, |window, _| {
        assert_eq!(label(window, "library-articles"), "Articles · 1");
        assert_eq!(label(window, "library-files"), "All files · 3");
        assert_eq!(window.find("library-articles").selected(), Some(true));
        assert!(window.find(format!("note-{article}")).visible());
        let row = label(window, format!("note-{article}"));
        assert!(
            row.contains("Current plan for the berth."),
            "the row describes it: {row}"
        );
        assert!(
            row.contains("by delegated maintenance"),
            "the delegated update is named, not called reviewed: {row}"
        );
        assert!(window.try_find(format!("note-{reference}")).is_none());
        assert!(window.try_find(format!("note-{private}")).is_none());
        let footer = label(window, "library-footer");
        assert!(footer.contains("not personally reviewed"), "{footer}");
        assert!(!footer.contains("Approved"), "{footer}");
    });
    click(cx, &f, "library-files");
    on_window(cx, &f, |window, _| {
        let row = label(window, format!("note-{reference}"));
        assert!(row.contains("Full imported reference"), "{row}");
        assert!(row.contains("Protected"), "{row}");
        assert!(window.find(format!("source-{loose}")).visible());
        assert!(
            window.try_find(format!("source-{survey}")).is_none(),
            "a source with a readable import is reached through that import"
        );
        assert!(window.try_find(format!("note-{private}")).is_none());
    });
    click(cx, &f, format!("source-{loose}"));
    cx.read(|cx| {
        let notice = f.view.read(cx).notice.text().to_owned();
        assert!(
            notice.contains("BRN keeps only what it recorded"),
            "{notice}"
        );
    });
    click(cx, &f, format!("note-{reference}"));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(reference.as_str()));
        assert_eq!(
            view.layout.visible(),
            [TileKind::Note],
            "a Library article opens for reading with the assistant closed"
        );
        assert!(view.writing.note.is_reading(), "Library opens in Read");
    });
    // A kept unsaved edit is shown in Write when the row is opened again.
    cx.update_window(f.window, |_, window, cx| {
        f.view.update(cx, |view, cx| {
            view.set_note_reading(false, window, cx);
            view.writing.note_editor.update(cx, |editor, cx| {
                editor.set_value("Survey text, kept faithfully. Edited.", window, cx)
            });
            view.preserve_visible_note_input(cx);
        });
    })
    .unwrap();
    cx.run_until_parked();
    click(cx, &f, "nav-library");
    click(cx, &f, "library-files");
    click(cx, &f, format!("note-{reference}"));
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(reference.as_str()));
        assert!(
            !view.writing.note.is_reading(),
            "an unsaved edit stays in Write so it is seen first"
        );
    });
    click(cx, &f, "nav-library");
    click(cx, &f, "library-menu");
    assert_eq!(
        menu_rows(cx, &f),
        ["Knowledge checks", "Recent changes", "Create Topics note"]
    );
    choose(cx, &f, "Knowledge checks");
    on_window(cx, &f, |window, _| {
        assert!(window.find("library-checks-heading").visible());
        assert!(window.find("knowledge-run").visible());
    });
    click(cx, &f, "library-back");
    on_window(cx, &f, |window, _| {
        assert!(window.find("library-articles").visible());
    });
    press(cx, &f, "cmd-k");
    on_window(cx, &f, |window, cx| window.input("harbor", cx));
    on_window(cx, &f, |window, _| {
        let labels: Vec<String> = (0..12)
            .filter_map(|i| window.try_find(format!("launcher-item-{i}")))
            .filter_map(|row| row.label().map(str::to_owned))
            .collect();
        assert!(
            labels.contains(&"Article: Harbor delivery".to_owned()),
            "{labels:?}"
        );
        assert!(
            labels.contains(&"File: Harbor survey · full imported reference".to_owned()),
            "{labels:?}"
        );
        assert!(
            labels.iter().all(|label| !label.contains("Harbor private")),
            "project-only material stays out of global search: {labels:?}"
        );
        assert_eq!(
            label(window, "launcher-group-files"),
            "Files · Current knowledge · saved library"
        );
    });
    press(cx, &f, "enter");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(article.as_str()));
        assert!(
            view.writing.note.is_reading(),
            "a found article opens in Read"
        );
        assert_eq!(view.layout.visible(), [TileKind::Note]);
    });
    // Inside the project, its own documents are found and named as such.
    click(cx, &f, "nav-projects");
    click(cx, &f, format!("project-row-{project}"));
    click(cx, &f, "launcher");
    on_window(cx, &f, |window, cx| window.input("private", cx));
    on_window(cx, &f, |window, _| {
        assert_eq!(
            label(window, "launcher-item-0"),
            "Project document: Harbor private · only in Harbor project"
        );
        assert_eq!(
            label(window, "launcher-group-project"),
            "This project · Project only · not in Library"
        );
    });
    press(cx, &f, "enter");
    cx.read(|cx| {
        let view = f.view.read(cx);
        assert_eq!(view.open_note_id(), Some(private.as_str()));
        assert_eq!(view.note_writing_scope.as_deref(), Some(project.as_str()));
    });
}
