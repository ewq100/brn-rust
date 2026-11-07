//! Owner changes with checked Undo; conversation/runtime receipts remain
//! available in a collapsed group without execution-control Undo.
use super::shell::pill;
use super::*;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::tooltip::Tooltip;

use super::tiles::{PAGE_WIDTH, column};

fn group(id: impl Into<gpui_kit::ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div().id(id).flex().flex_col().gap(px(2.)).min_w_0()
}

impl ThreadsUi {
    /// A record that may need refresh, in owner terms on one line: its
    /// title, or for a chat Message its kind, never its stored payload (R3;
    /// a rewrite request's text is its saved JSON snapshot).
    fn refresh_name(&self, record: &Record) -> String {
        let name = match &record.data {
            // The request's note title was resolved when history refreshed.
            RecordData::Message(message) if message.role == WRITING_REQUEST_ROLE => {
                match self.refresh_names.get(&record.id) {
                    Some(note) => format!("Rewrite request · {note}"),
                    None => "Rewrite request".into(),
                }
            }
            RecordData::Message(_) => "Earlier chat message".into(),
            RecordData::Link(_) => "Related note and chat".into(),
            _ => title(record),
        };
        name.split_whitespace().collect::<Vec<_>>().join(" ")
    }
    /// One recorded change: its reason, what it touched and when, an Undo
    /// ghost, and for derived work a "May need refresh" sub-row.
    fn receipt_row(&self, receipt: &Receipt, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let id = receipt.operation.as_str().to_owned();
        let reason = lists::receipt_reason(&receipt.reason);
        let affected = lists::affected_titles(receipt);
        let stamp = receipt.committed_at.clone();
        let undo_label = if affected.is_empty() {
            format!("Undo: {reason}")
        } else {
            format!("Undo: {reason} · {affected}")
        };
        let undo_id = id.clone();
        let mut entry = div()
            .id(format!("receipt-{id}"))
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .px_2()
            .py_2()
            .min_w_0()
            .border_b_1()
            .border_color(rgb(p.hairline))
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .id(format!("receipt-reason-{id}"))
                                    .font_weight(gpui_kit::FontWeight::MEDIUM)
                                    .text_color(rgb(p.ink))
                                    .aria_label(reason.clone())
                                    .child(reason.clone())
                                    .test_support(),
                            )
                            .child(
                                div()
                                    .id(format!("receipt-facts-{id}"))
                                    .flex()
                                    .min_w_0()
                                    .gap_1()
                                    .text_size(SMALL_TEXT)
                                    .text_color(rgb(p.muted))
                                    .when(!affected.is_empty(), |facts| {
                                        facts.child(
                                            div()
                                                .min_w_0()
                                                .truncate()
                                                .child(format!("{affected} ·")),
                                        )
                                    })
                                    .child(div().flex_shrink_0().child(lists::human_time(&stamp)))
                                    .tooltip(move |window, cx| {
                                        Tooltip::new(stamp.clone()).build(window, cx)
                                    })
                                    .test_support(),
                            ),
                    )
                    .when(lists::receipt_offers_undo(receipt), |row| {
                        row.child(
                            button(format!("undo-{id}"))
                                .label("Undo")
                                .icon(IconName::Undo2)
                                .ghost()
                                .compact()
                                .h(components::tokens::COMPACT_CONTROL)
                                .flex_shrink_0()
                                .accessibility_label(undo_label.clone())
                                .tooltip(undo_label)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.undo_receipt(&undo_id, window, cx)
                                })),
                        )
                    }),
            );
        if !lists::is_background(receipt) && !receipt.needs_refresh.is_empty() {
            let mut refresh = div()
                .id(format!("receipt-refresh-{id}"))
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .px_2()
                .py_1()
                .rounded(components::tokens::RADIUS_CONTROL)
                .bg(rgb(p.warning_bg))
                .text_color(rgb(p.ink))
                .aria_label("May need refresh")
                .child(components::tone_mark(
                    format!("receipt-refresh-{id}-tone"),
                    Tone::Warning,
                ))
                .child(
                    div()
                        .font_weight(gpui_kit::FontWeight::MEDIUM)
                        .mr_1()
                        .child("May need refresh"),
                );
            refresh = refresh.child(
                div()
                    .id(format!("receipt-refresh-explanation-{id}"))
                    .w_full()
                    .text_size(SMALL_TEXT)
                    .child("This change may affect the saved work below. Review it against the current inputs before relying on it. A follow-up opens a chat; nothing updates automatically.")
                    .test_support(),
            );
            if let Ok(records) = self.workspace.refresh_records(&id) {
                for record in records {
                    let mut note =
                        matches!(record.data, RecordData::Note(_)).then(|| record.id.clone());
                    let mut chat = match &record.data {
                        RecordData::Message(message) => Some(message.thread.clone()),
                        RecordData::Thread(_) => Some(record.id.clone()),
                        _ => None,
                    };
                    if let RecordData::Link(link) = &record.data {
                        for endpoint in [&link.from, &link.to] {
                            match self
                                .records
                                .iter()
                                .find(|record| &record.id == endpoint)
                                .map(|record| &record.data)
                            {
                                Some(RecordData::Note(_)) => note = Some(endpoint.clone()),
                                Some(RecordData::Thread(_)) => chat = Some(endpoint.clone()),
                                _ => {}
                            }
                        }
                    }
                    let name = self.refresh_name(&record);
                    // One line that ellipsizes inside the panel (R3); the
                    // full name is the tooltip and the accessible name.
                    refresh = refresh.child(
                        pill(format!("refresh-record-{id}-{}", record.id), name.clone())
                            .max_w(gpui_kit::relative(1.))
                            .min_w_0()
                            .accessibility_label(format!("May need refresh: {name}"))
                            .tooltip(if note.is_some() {
                                format!("{name} · Open this note to review its inputs")
                            } else if chat.is_some() {
                                format!("{name} · Open this chat to review its current context")
                            } else {
                                format!("{name} · Open Tasks to review it")
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                if let Some(id) = &note {
                                    this.open_note(id, None, window, cx);
                                } else if let Some(id) = &chat {
                                    this.open_thread(id, window, cx);
                                } else {
                                    this.show_tile(TileKind::Actions, window, cx);
                                }
                            })),
                    );
                }
            }
            let receipt_id = id.clone();
            refresh = refresh.child(
                button(format!("refresh-followup-{id}"))
                    .label("Open refresh follow-up")
                    .outline()
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .tooltip(
                        "Open a chat that reviews the current inputs before anything is updated",
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        match this.workspace.refresh_followup(&receipt_id) {
                            Ok(thread) => {
                                this.refresh();
                                this.open_thread(&thread.id, window, cx);
                            }
                            Err(error) => {
                                this.fail(error.to_string());
                                cx.notify();
                            }
                        }
                    })),
            );
            entry = entry.child(refresh.test_support());
        }
        entry.test_support().into_any_element()
    }
    pub(super) fn history_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let (background, changes): (Vec<&Receipt>, Vec<&Receipt>) = self
            .history
            .iter()
            .rev()
            .partition(|r| lists::is_background(r));
        let mut view = column().child(super::inbox_page::lead(
            "history-lead",
            "Changes recorded in this workspace, newest first, by you and by BRN. Undo appears where it still applies.",
        ));
        if changes.is_empty() {
            view = view.child(components::empty_state(
                "history-empty",
                IconName::RotateCcwClock,
                "No changes yet",
                "Recorded changes and the same checked Undo used throughout your workspace.",
                None,
            ));
        } else {
            let mut list = super::inbox_page::page_group("history-changes")
                .role(gpui_kit::Role::Group)
                .aria_label(format!("Changes · {}", changes.len()))
                .child(super::inbox_page::group_heading(
                    "Changes",
                    Some(changes.len()),
                ));
            for receipt in changes {
                list = list.child(self.receipt_row(receipt, cx));
            }
            view = view.child(list);
        }
        // Complete receipts stay inspectable without suggesting Undo of a Run.
        if !background.is_empty() {
            let mut list = group("history-background").child(components::section_header(
                "Chat and runtime details",
                Some(background.len()),
                Some(self.disclosure("history-background", background.len(), cx)),
            ));
            if self.open_sections.contains("history-background") {
                for receipt in background {
                    list = list.child(self.receipt_row(receipt, cx));
                }
            }
            view = view.child(list.test_support());
        }
        super::inbox_page::page_frame(
            "history-content",
            PAGE_WIDTH,
            self.arrangement.primary < 720.,
            view,
        )
        .test_support()
        .into_any_element()
    }
}

#[cfg(all(test, feature = "native-test-support"))]
mod tests {
    use super::*;
    use gpui_kit::{TestAppContext, test::TestWindowExt};

    struct Fixture {
        _root: tempfile::TempDir,
        view: Entity<ThreadsUi>,
        window: gpui_kit::AnyWindowHandle,
        note: Record,
        chat: String,
        reply: String,
        edit: Receipt,
        terminal: Receipt,
        derived: String,
    }
    fn fixture(cx: &mut TestAppContext) -> Fixture {
        cx.update(gpui_kit::init);
        let root = tempfile::Builder::new()
            .prefix("brn-nu-history-ui-")
            .tempdir_in("/private/tmp")
            .unwrap();
        let data = root.path().join("data");
        let mut workspace = Workspace::open(&data).unwrap();
        focus_tests::configure_offline_fixture(&mut workspace);
        let note = workspace.new_note("Brief", "Original text").unwrap();
        let chat = workspace.new_thread("Edit the brief").unwrap().id;
        let start = workspace.store.allocate_operation("start").unwrap();
        let run_id = start.creation_id(0);
        let settings = workspace.settings().unwrap();
        let mut run = Run {
            instruction: None,
            queue_claim: None,
            thread: chat.clone(),
            provider: settings.provider,
            model: settings.model,
            guide_identity: "fixture".into(),
            budget: 3,
            effort: settings.effort,
            loaded_skills: vec![],
            loaded_skill_details: vec![],
            fence: 1,
            state: RunState::Working,
            progress: String::new(),
            operations: vec![],
            sources: vec![],
            web_sources: vec![],
        };
        let apply = |workspace: &mut Workspace, key: &str, reason: &str, writes: Vec<Put>| {
            let ApplyOutcome::Applied(receipt) = workspace
                .owner_change(
                    key,
                    &ChangeRequest {
                        reason: reason.into(),
                        writes,
                        inputs: vec![],
                    },
                )
                .unwrap()
            else {
                panic!("fixture must apply")
            };
            receipt
        };
        apply(
            &mut workspace,
            "start",
            "Start a fresh invocation",
            vec![Put {
                id: run_id.clone(),
                expected_version: None,
                archived: false,
                data: RecordData::Run(run.clone()),
            }],
        );
        let mut changed = note.data.clone();
        if let RecordData::Note(note) = &mut changed {
            note.markdown = "Original text\nReminder".into();
        }
        let edit = apply(
            &mut workspace,
            "edit",
            "Added reminder to Brief",
            vec![Put {
                id: note.id.clone(),
                expected_version: Some(note.version),
                archived: false,
                data: changed,
            }],
        );
        run.operations.push(edit.operation.clone());
        run.state = RunState::Failed;
        run.fence += 1;
        let terminal_op = workspace.store.allocate_operation("terminal").unwrap();
        let reply = terminal_op.creation_id(0);
        let chat_record = workspace.store.record(&chat).unwrap().unwrap();
        let RecordData::Thread(mut thread) = chat_record.data else {
            unreachable!()
        };
        thread.attention = vec![
            Attention {
                kind: AttentionKind::Blocker,
                reason: "Run failed: synthetic offline provider".into(),
                record: Some(run_id.clone()),
            },
            Attention {
                kind: AttentionKind::Question,
                reason: "Which date?".into(),
                record: Some(run_id.clone()),
            },
        ];
        let terminal = apply(
            &mut workspace,
            "terminal",
            "Record terminal run result",
            vec![
                Put {
                    id: run_id,
                    expected_version: Some(1),
                    archived: false,
                    data: RecordData::Run(run),
                },
                Put {
                    id: reply.clone(),
                    expected_version: None,
                    archived: false,
                    data: RecordData::Message(Message {
                        thread: chat.clone(),
                        role: "assistant".into(),
                        text: "I added the reminder".into(),
                    }),
                },
                Put {
                    id: chat.clone(),
                    expected_version: Some(chat_record.version),
                    archived: false,
                    data: RecordData::Thread(thread),
                },
            ],
        );
        let derived_op = workspace
            .store
            .allocate_operation("derived-message")
            .unwrap();
        let derived = derived_op.creation_id(0);
        let ApplyOutcome::Applied(_) = workspace
            .owner_change(
                "derived-message",
                &ChangeRequest {
                    reason: "Saved summary".into(),
                    inputs: vec![RevisionInput {
                        record: note.id.clone(),
                        version: note.version + 1,
                        expectation: InputExpectation::Current,
                    }],
                    writes: vec![Put {
                        id: derived.clone(),
                        expected_version: None,
                        archived: false,
                        data: RecordData::Message(Message {
                            thread: chat.clone(),
                            role: "assistant".into(),
                            text: "Earlier summary".into(),
                        }),
                    }],
                },
            )
            .unwrap()
        else {
            panic!("derived message must apply")
        };
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
                        ui.open_thread(&chat, window, cx);
                        ui.show_tile(TileKind::History, window, cx);
                        ui
                    })
                },
            )
            .unwrap()
        });
        cx.run_until_parked();
        Fixture {
            _root: root,
            window,
            view,
            note,
            chat,
            reply,
            edit,
            terminal,
            derived,
        }
    }
    fn on_window(cx: &mut TestAppContext, f: &Fixture, action: impl FnOnce(&mut Window, &mut App)) {
        cx.update_window(f.window, |_, window, cx| action(window, cx))
            .unwrap();
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn recent_changes_hides_runtime_undo_and_marks_the_preserved_reply(cx: &mut TestAppContext) {
        let f = fixture(cx);
        on_window(cx, &f, |window, cx| {
            assert!(
                window
                    .find(format!("receipt-{}", f.edit.operation.as_str()))
                    .visible()
            );
            assert!(
                window
                    .try_find(format!("receipt-{}", f.terminal.operation.as_str()))
                    .is_none()
            );
            f.view.update(cx, |ui, cx| {
                ui.open_sections.insert("history-background".into());
                cx.notify();
            });
        });
        on_window(cx, &f, |window, cx| {
            assert!(
                window
                    .find(format!("receipt-{}", f.terminal.operation.as_str()))
                    .visible()
            );
            assert!(
                window
                    .try_find(format!("undo-{}", f.terminal.operation.as_str()))
                    .is_none()
            );
            window.click(format!("undo-{}", f.edit.operation.as_str()), cx);
        });
        on_window(cx, &f, |window, cx| {
            let undo = f
                .view
                .read(cx)
                .history
                .last()
                .unwrap()
                .operation
                .as_str()
                .to_owned();
            assert!(
                window
                    .find(format!("receipt-refresh-explanation-{undo}"))
                    .visible()
            );
            window.click(format!("refresh-record-{undo}-{}", f.derived), cx);
        });
        on_window(cx, &f, |window, cx| {
            assert_eq!(
                window.find(format!("answer-undone-{}", f.reply)).label(),
                Some("Change undone")
            );
            let ui = f.view.read(cx);
            assert!(
                matches!(ui.workspace.store.record(&f.note.id).unwrap().unwrap().data, RecordData::Note(n) if n.markdown == "Original text")
            );
            assert!(ui.workspace.messages(&f.chat).unwrap().iter().any(|record| matches!(&record.data, RecordData::Message(m) if m.text == "I added the reminder")));
            assert!(
                ui.workspace
                    .history()
                    .unwrap()
                    .iter()
                    .any(|receipt| receipt.operation == f.terminal.operation),
                "full receipt history stays intact"
            );
        });
    }

    #[gpui_kit::test]
    fn inbox_acknowledges_only_the_exact_failed_run_notice(cx: &mut TestAppContext) {
        let f = fixture(cx);
        on_window(cx, &f, |window, cx| {
            f.view.update(cx, |ui, cx| {
                ui.show_tile(TileKind::Overview, window, cx);
                assert_eq!(ui.for_you_count(), 2);
            })
        });
        on_window(cx, &f, |window, cx| {
            window.click(format!("for-you-chat-{}-0-acknowledge", f.chat), cx);
        });
        on_window(cx, &f, |window, cx| {
            let ui = f.view.read(cx);
            assert_eq!(
                ui.for_you_count(),
                1,
                "the unrelated question still needs an answer"
            );
            assert!(
                ui.for_you()
                    .iter()
                    .all(|item| item.need != inbox_page::Need::Failed)
            );
            assert!(
                window
                    .try_find(format!("for-you-chat-{}-0-acknowledge", f.chat))
                    .is_none()
            );
            assert_eq!(
                ui.workspace.history().unwrap().last().unwrap().reason,
                "Acknowledged run notice"
            );
        });
    }
}
