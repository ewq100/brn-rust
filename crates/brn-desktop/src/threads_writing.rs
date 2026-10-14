//! Writing and conversation tiles: chat, note, in-place review, compare and
//! linked work. Every action calls the same checked services as before.
use super::shell::{item, pill};
use super::tiles::scroll;
use super::*;
use gpui_kit::StyledText;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::DropdownMenu;

impl ThreadsUi {
    pub(super) fn safe_message(&self, id: String, text: String) -> TextView {
        let assets = self.assets_in_chat(self.chat_thread());
        TextView::markdown(id, text)
            .w_full()
            .markdown_extensions(
                MarkdownExtensions::default()
                    .plugin(LiteralHtml { block: true })
                    .plugin(LiteralHtml { block: false })
                    .parser_revision(1),
            )
            .image_source(move |uri| {
                assets
                    .get(uri.as_ref())
                    .cloned()
                    .unwrap_or_else(|| Arc::new(Image::from_bytes(ImageFormat::Png, Vec::new())))
                    .into()
            })
            .on_link_click(|url, _, _, cx| {
                if url.starts_with("https://") || url.starts_with("http://") {
                    cx.open_url(url);
                }
                cx.stop_propagation();
            })
    }
    pub(super) fn thread_title(&self, id: &str) -> String {
        self.thread_rows
            .iter()
            .find(|row| row.id == id)
            .map_or_else(|| id.to_owned(), |row| row.title.clone())
    }
    pub(super) fn thread_resolved(&self, id: &str) -> bool {
        self.thread_rows
            .iter()
            .any(|row| row.id == id && row.resolved)
    }
    pub(super) fn chat_controls(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let this = cx.entity().downgrade();
        let mut controls = vec![];
        if let ChatTarget::Thread(id) = &self.chat {
            let id = id.clone();
            controls.push(if self.thread_resolved(&id) {
                pill("reopen-thread", "Unarchive")
                    .tooltip("Unarchive this chat")
                    .on_click(cx.listener(move |this, _, _, cx| this.reopen_thread(&id, cx)))
                    .into_any_element()
            } else {
                pill("resolve-thread", "Archive")
                    .tooltip("Archive this chat; stop work and pause the saved queue")
                    .on_click(cx.listener(move |this, _, _, cx| this.resolve_thread(&id, cx)))
                    .into_any_element()
            });
        }
        let note = self
            .note
            .as_ref()
            .map(|open| (open.record.id.clone(), title(&open.record)));
        let threads: Vec<(String, String)> = note
            .as_ref()
            .and_then(|(id, _)| self.workspace.writing_threads(id).ok())
            .unwrap_or_default()
            .iter()
            .map(|record| (record.id.clone(), title(record)))
            .collect();
        let selected = self.chat_thread().map(str::to_owned);
        controls.push(
            pill("thread-menu", "▾")
                .tooltip("Chats for this note")
                .dropdown_menu(move |mut menu, _, _| {
                    if let Some((note_id, note_title)) = &note {
                        menu = menu.label(format!("Chats for {}", short(note_title)));
                        for (thread, label) in &threads {
                            let (thread, note_id) = (thread.clone(), note_id.clone());
                            menu = menu.item(item(
                                &this,
                                short(label),
                                None,
                                false,
                                selected.as_deref() == Some(thread.as_str()),
                                move |view, window, cx| {
                                    view.select_writing_thread(
                                        note_id.clone(),
                                        thread.clone(),
                                        window,
                                        cx,
                                    )
                                },
                            ));
                        }
                    }
                    menu.item(item(
                        &this,
                        "＋ New chat for this note",
                        None,
                        note.is_none(),
                        false,
                        |view, window, cx| view.new_writing_thread(window, cx),
                    ))
                    .separator()
                    .item(item(
                        &this,
                        "New chat",
                        Some(Box::new(NewThread)),
                        false,
                        false,
                        |view, window, cx| view.new_thread(window, cx),
                    ))
                    .item(item(
                        &this,
                        "All chats…",
                        None,
                        false,
                        false,
                        |view, _, cx| view.show_tile(TileKind::AllThreads, cx),
                    ))
                })
                .into_any_element(),
        );
        controls
    }
    pub(super) fn chat_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let body = match &self.chat {
            ChatTarget::Thread(id) => self.conversation(id, cx),
            target => {
                let mut start = scroll("thread-conversation")
                    .gap_3()
                    .p_4()
                    .child(metadata(match target {
                        ChatTarget::NewForNote(note) => format!(
                            "New chat about {}. It stays linked to the note.",
                            self.note_rows
                                .iter()
                                .find(|row| &row.id == note)
                                .map_or_else(|| note.clone(), |row| row.title.clone())
                        ),
                        _ => "Start a chat with BRN. Attach files for this conversation, or use your saved knowledge.".into(),
                    }));
                if self.delegation_host() == Some(TileKind::Chat) {
                    start = start.child(self.working_knowledge(cx));
                }
                start.test_support().into_any_element()
            }
        };
        let activity = self.chat_thread().and_then(|id| {
            if self
                .worker
                .as_ref()
                .is_some_and(|w| w.thread.as_deref() == Some(id))
            {
                Some(self.live_activity(true, cx))
            } else if !self.queue_state().quiet
                && self.run_activity_chat.as_deref() == Some(id)
                && !self.run_activity_labels.is_empty()
            {
                Some(self.live_activity(false, cx))
            } else {
                None
            }
        });
        div()
            .id("chat-tile")
            .size_full()
            .flex()
            .flex_col()
            .child(div().flex_1().min_h_0().child(body))
            // Keep the activity scroller outside MessageScroller's wheel mask.
            .when_some(activity, |tile, activity| {
                tile.child(div().flex_shrink_0().p_2().child(activity))
            })
            .child(self.composer_tile(cx))
            .into_any_element()
    }
    pub(super) fn conversation_detail(
        &self,
        id: &str,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let p = pal();
        let Some(record) = self.records.iter().find(|r| r.id == id) else {
            return scroll("thread-conversation")
                .child(empty(
                    "thread-unavailable",
                    "This chat is unavailable. Open another chat from the sidebar.",
                ))
                .into_any_element();
        };
        let RecordData::Thread(thread) = &record.data else {
            return div().into_any_element();
        };
        let mut conversation = scroll("thread-conversation").gap_3().p_4();
        let mut cards = BTreeSet::new();
        for (index, attention) in thread.attention.iter().enumerate() {
            let candidate = self
                .candidates
                .iter()
                .find(|c| attention.record.as_deref() == Some(c.operation.as_str()));
            let mut card = div()
                .id(format!("attention-card-{index}"))
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .border_1()
                .border_color(rgb(p.accent))
                .bg(rgb(p.alt))
                .child(
                    div()
                        .font_family(MONO)
                        .text_xs()
                        .text_color(rgb(p.accent))
                        .child(format!("brn needs you · {:?}", attention.kind).to_lowercase()),
                )
                .child(attention.reason.clone());
            if let Some(candidate) = candidate {
                cards.insert(candidate.operation.as_str().to_owned());
                card = card.child(self.proposal_card(candidate, cx));
            }
            conversation = conversation.child(card.test_support());
        }
        // Rewrite proposals for this thread appear even without an attention reason.
        for candidate in self.candidates.iter().filter(|candidate| {
            !cards.contains(candidate.operation.as_str())
                && candidate.request.inputs.iter().any(|input| {
                    self.workspace
                        .writing_request(&input.record)
                        .is_ok_and(|snapshot| snapshot.thread == id)
                })
        }) {
            conversation = conversation.child(self.proposal_card(candidate, cx));
        }
        if let Some(stream) = self.stream_for(id) {
            conversation = conversation.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(metadata("BRN · writing…"))
                    .child(self.safe_message(
                        format!(
                                "response-{}",
                                self.worker
                                    .as_ref()
                                    .and_then(|w| w.run_id.as_deref())
                                    .unwrap_or(id)
                            ),
                        stream.into(),
                    )),
            );
        }
        if can_continue_thread(
            &self.records,
            &self.history,
            id,
            &thread.state,
            self.worker.is_some(),
        ) {
            let latest = latest_run(&self.records, &self.history, id);
            let run = latest.and_then(|record| match &record.data {
                RecordData::Run(run) => Some(run),
                _ => None,
            });
            let thread_id = id.to_owned();
            let status = run.map_or_else(
                || "Saved instruction is ready to continue".into(),
                |run| {
                    if run.state == RunState::Working {
                        "Saved run status: Working · current activity is unconfirmed".into()
                    } else {
                        format!("{:?}", run.state)
                    }
                },
            );
            conversation = conversation.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .font_family(MONO)
                    .text_xs()
                    .text_color(rgb(p.muted))
                    .child(format!("■ {status}"))
                    .child(run.map_or_else(String::new, |run| run.progress.clone()))
                    .child(run.map_or(
                        "Continue starts fresh with the saved conversation and current notes.",
                        |run| continuation_notice(&run.state),
                    ))
                    .child(
                        button(format!(
                            "continue-{}",
                            latest.map_or(id, |record| record.id.as_str())
                        ))
                        .label(
                            if self.queue_state().pending.is_some_and(|q| q.thread == id) {
                                "Resume queue with current context"
                            } else {
                                "Continue with current context"
                            },
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.continue_run(thread_id.clone(), cx)
                        })),
                    ),
            );
        }
        if record.archived || thread.state == ThreadState::Resolved {
            let thread_id = id.to_owned();
            conversation = conversation
                .child(metadata("Archived chat · saved history remains available"))
                .child(button("chat-unarchive").label("Unarchive").on_click(
                    cx.listener(move |this, _, _, cx| this.reopen_thread(&thread_id, cx)),
                ));
        }
        conversation.test_support().into_any_element()
    }
    /// A compact preview of one exact candidate; Apply binds that candidate.
    pub(super) fn proposal_card(
        &self,
        candidate: &Prepared,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let p = pal();
        let op = candidate.operation.as_str().to_owned();
        let note = candidate
            .request
            .writes
            .iter()
            .find_map(|write| match &write.data {
                RecordData::Note(note) => Some((write.id.clone(), note)),
                _ => None,
            });
        let (heading_text, preview, protected) =
            match &note {
                Some((id, proposed)) => {
                    let current =
                        self.workspace.store.note(id).ok().flatten().and_then(
                            |record| match record.data {
                                RecordData::Note(note) => Some((note.markdown, note.protected)),
                                _ => None,
                            },
                        );
                    let protected = current.as_ref().is_some_and(|(_, protected)| *protected)
                        || proposed.protected;
                    let preview = diff::preview(
                        current.as_ref().map_or("", |(text, _)| text.as_str()),
                        &proposed.markdown,
                        2,
                    );
                    (
                        format!(
                            "proposal · {}  +{} −{}",
                            short(&proposed.title),
                            preview.added,
                            preview.removed
                        ),
                        Some(preview),
                        protected,
                    )
                }
                None => (
                    format!(
                        "proposal · {} changes · {}",
                        candidate.request.writes.len(),
                        short(&candidate.request.reason)
                    ),
                    None,
                    false,
                ),
            };
        let mut card = div()
            .id(format!("proposal-{op}"))
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .border_1()
            .border_color(rgb(p.border))
            .bg(rgb(p.tile))
            .child(
                div()
                    .font_family(MONO)
                    .text_xs()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child(heading_text),
            );
        if let Some(preview) = preview
            && !preview.text.is_empty()
        {
            let highlights = preview.marks.iter().map(|(range, mark)| {
                let style = match mark {
                    diff::Mark::Deleted => gpui_kit::HighlightStyle {
                        background_color: Some(rgb(p.delete).into()),
                        strikethrough: Some(gpui_kit::StrikethroughStyle {
                            thickness: px(1.),
                            color: Some(rgb(p.ink).into()),
                        }),
                        ..Default::default()
                    },
                    diff::Mark::Inserted => gpui_kit::HighlightStyle {
                        background_color: Some(rgb(p.add).into()),
                        ..Default::default()
                    },
                };
                (range.clone(), style)
            });
            card = card.child(
                div()
                    .text_sm()
                    .max_h(px(140.))
                    .overflow_hidden()
                    .child(StyledText::new(preview.text.clone()).with_highlights(highlights)),
            );
        }
        let show = candidate.clone();
        let dismiss = candidate.clone();
        let apply = candidate.clone();
        card.child(
            div()
                .flex()
                .flex_wrap()
                .gap_1()
                .child(
                    pill(format!("thread-review-{op}"), "Show in note").on_click(cx.listener(
                        move |this, _, window, cx| this.open_review(show.clone(), window, cx),
                    )),
                )
                .child(div().flex_1())
                .child(
                    pill(format!("card-dismiss-{op}"), "Dismiss").on_click(cx.listener(
                        move |this, _, window, cx| this.card_dismiss(dismiss.clone(), window, cx),
                    )),
                )
                .child(
                    pill(
                        format!("card-apply-{op}"),
                        if protected { "Approve" } else { "Apply" },
                    )
                    .primary()
                    .tooltip("Applies this exact proposal")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.card_apply(apply.clone(), window, cx)
                    })),
                ),
        )
        .test_support()
        .into_any_element()
    }
    fn composer_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let busy = self.worker.is_some();
        let destination = match &self.chat {
            ChatTarget::Thread(id) if self.thread_resolved(id) => format!(
                "Archived · {} · Unarchive to reply",
                short(&self.thread_title(id))
            ),
            ChatTarget::Thread(id) => format!("Reply to {}", short(&self.thread_title(id))),
            ChatTarget::NewForNote(note) => format!(
                "New chat about {}",
                self.note_rows
                    .iter()
                    .find(|row| &row.id == note)
                    .map_or_else(|| note.clone(), |row| short(&row.title))
            ),
            ChatTarget::New => "New standalone chat".into(),
        };
        let model_this = this.clone();
        let effort_this = this.clone();
        let add_this = this.clone();
        let can_steer = self.can_stop_and_send();
        div()
            .id("composer-tile")
            .on_drop(
                cx.listener(|this, paths: &gpui_kit::ExternalPaths, window, cx| {
                    this.attach_chat_path(paths.paths(), window, cx)
                }),
            )
            .capture_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "v"
                        && event.keystroke.modifiers.platform
                        && let Some(item) = cx.read_from_clipboard()
                    {
                        let paths = item
                            .entries()
                            .iter()
                            .filter_map(|entry| match entry {
                                gpui_kit::ClipboardEntry::ExternalPaths(paths) => {
                                    Some(paths.paths())
                                }
                                _ => None,
                            })
                            .flatten()
                            .cloned()
                            .collect::<Vec<_>>();
                        if !paths.is_empty() {
                            cx.stop_propagation();
                            this.attach_chat_path(&paths, window, cx);
                        }
                    }
                }),
            )
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .p_2()
            .bg(rgb(p.alt))
            .border_t_1()
            .border_color(rgb(p.border))
            .child(
                div()
                    .id("composer-destination")
                    .font_family(MONO)
                    .text_xs()
                    .text_color(rgb(p.muted))
                    .aria_label(destination.clone())
                    .child(destination)
                    .test_support(),
            )
            .child(self.composer_controls(cx))
            .child(
                pill(
                    "chat-quiet",
                    if self.queue_state().quiet {
                        "Quiet · on · answers only"
                    } else {
                        "Quiet · off · show activity"
                    },
                )
                .selected(self.queue_state().quiet)
                .tooltip(if self.queue_state().quiet {
                    "Hide routine activity throughout the window. Answers, Stop, errors and decisions stay visible."
                } else {
                    "Show recent activity while BRN works."
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    let quiet = !this.queue_state().quiet;
                    if let Err(e) = this
                        .workspace
                        .choose_quiet(&format!("quiet:{}", uuid::Uuid::new_v4()), quiet)
                    {
                        this.notice = e.to_string();
                    }
                    cx.notify();
                })),
            )
            .child(
                div().h(px(84.)).child(
                    prose_editor(&self.composer)
                        .h_full()
                        .aria_label("Ask or delegate"),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_1()
                    .child(
                        pill("composer-add", "＋")
                            .tooltip("Add information")
                            .dropdown_menu(move |menu, _, _| {
                                menu.item(item(
                                    &add_this,
                                    "Attach file to chat…",
                                    None,
                                    busy,
                                    false,
                                    |view, window, cx| view.choose_chat_attachment(window, cx),
                                ))
                                .item(item(
                                    &add_this,
                                    "Add useful information…",
                                    None,
                                    busy,
                                    false,
                                    |view, window, cx| {
                                        view.choose_import(
                                            ImportIntent::UsefulInformation,
                                            window,
                                            cx,
                                        )
                                    },
                                ))
                                .item(item(
                                    &add_this,
                                    "Import full note…",
                                    None,
                                    busy,
                                    false,
                                    |view, window, cx| {
                                        view.choose_import(ImportIntent::FullNote, window, cx)
                                    },
                                ))
                            }),
                    )
                    .child(div().flex_1())
                    .child(
                        pill(
                            "model-pill",
                            format!(
                                "{} · {} ▾",
                                super::shell::provider_key(&self.settings.provider),
                                self.settings.model
                            ),
                        )
                        .tooltip("Provider and model · changes Settings · no fallback")
                        .dropdown_menu(move |menu, _, cx| match model_this.upgrade() {
                            Some(view) => view.read(cx).model_menu(menu, &model_this),
                            None => menu,
                        }),
                    )
                    .child(
                        pill("effort-pill", format!("{} ▾", self.settings.effort))
                            .tooltip("Reasoning effort")
                            .dropdown_menu(move |menu, _, cx| match effort_this.upgrade() {
                                Some(view) => view.read(cx).effort_menu(menu, &effort_this),
                                None => menu,
                            }),
                    )
                    .when(can_steer, |row| {
                        row.child(
                            pill("chat-steer", "Steer")
                                .tooltip(
                                    "Stop this invocation and start fresh with this instruction",
                                )
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.stop_and_send(window, cx)
                                })),
                        )
                    })
                    .child(
                        button("ask-delegate")
                            .label(if self.editing_queue.is_some() {
                                "Save queued"
                            } else if busy {
                                "Queue ↵"
                            } else {
                                "Send ↵"
                            })
                            .primary()
                            .compact()
                            .tooltip("Enter sends or queues · Shift-Enter adds a line")
                            .disabled(false)
                            .on_click(cx.listener(|this, _, window, cx| this.ask(window, cx))),
                    ),
            )
            .test_support()
            .into_any_element()
    }
    pub(super) fn note_controls(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let p = pal();
        let Some(open) = &self.note else {
            return vec![];
        };
        let RecordData::Note(note) = &open.record.data else {
            return vec![];
        };
        if self.review.is_some() {
            return vec![
                div()
                    .px_1()
                    .bg(rgb(p.accent))
                    .text_color(rgb(p.primary_fg))
                    .child("reviewing")
                    .into_any_element(),
            ];
        }
        let this = cx.entity().downgrade();
        let dirty = open.session.is_some();
        let busy = self.worker.is_some();
        let imported = note.import.is_some();
        let protected = note.protected;
        let pending = self
            .workspace
            .writing_comments(&open.record.id)
            .map(|comments| comments.iter().filter(|entry| !entry.addressed).count())
            .unwrap_or_default();
        let status = format!(
            "{} v{}",
            if protected { "protected" } else { "working" },
            open.record.version
        );
        let mut controls = vec![
            div()
                .id("note-status-chip")
                .px_1()
                .border_1()
                .border_color(rgb(if protected { p.accent } else { p.border }))
                .aria_label(status.clone())
                .child(status)
                .test_support()
                .into_any_element(),
        ];
        if dirty {
            controls.push(
                div()
                    .id("note-unsaved-chip")
                    .px_1()
                    .bg(rgb(p.comment))
                    .child("● unsaved")
                    .test_support()
                    .into_any_element(),
            );
        }
        controls.push(
            pill(
                "toggle-margin",
                if pending > 0 {
                    format!("💬 {pending}")
                } else {
                    "💬".into()
                },
            )
            .selected(self.margin)
            .tooltip("Comments in the margin")
            .on_click(cx.listener(|this, _, _, cx| {
                this.margin = !this.margin;
                cx.notify();
            }))
            .into_any_element(),
        );
        controls.push(
            pill("edit-note-context", "Edit note")
                .disabled(!self.can_edit_note_context())
                .tooltip("Give BRN an instruction for this saved note")
                .on_click(cx.listener(|this, _, window, cx| this.edit_note_dialog(window, cx)))
                .into_any_element(),
        );
        controls.push(
            pill("note-connections", "Connections")
                .tooltip("Linked notes and sources of this note")
                .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::Connections, cx)))
                .into_any_element(),
        );
        controls.push(
            pill("note-menu", "⋯")
                .tooltip("Note actions")
                .dropdown_menu(move |menu, _, _| {
                    let mut menu = menu
                        .item(item(
                            &this,
                            "Save",
                            Some(Box::new(Save)),
                            !dirty,
                            false,
                            |view, window, cx| view.save(window, cx),
                        ))
                        .item(item(
                            &this,
                            "Discard edit",
                            None,
                            !dirty,
                            false,
                            |view, window, cx| view.discard(window, cx),
                        ))
                        .item(item(
                            &this,
                            "Reload saved note",
                            None,
                            dirty,
                            false,
                            |view, window, cx| {
                                if let Some(id) = view.open_note_id().map(str::to_owned) {
                                    view.open_note(&id, None, window, cx);
                                }
                            },
                        ))
                        .item(item(
                            &this,
                            "Save text as a new note",
                            None,
                            !dirty,
                            false,
                            |view, window, cx| view.save_copy(window, cx),
                        ))
                        .separator()
                        .item(item(
                            &this,
                            if dirty {
                                "Comment on selection · save first"
                            } else {
                                "Comment on selection"
                            },
                            Some(Box::new(CommentSelection)),
                            dirty,
                            false,
                            |view, window, cx| view.comment_on_selection(window, cx),
                        ))
                        .item(item(
                            &this,
                            "Rewrite with AI",
                            Some(Box::new(Rewrite)),
                            dirty || busy,
                            false,
                            |view, _, cx| view.rewrite_note(cx),
                        ))
                        .separator()
                        .item(item(
                            &this,
                            if protected {
                                "Make working note"
                            } else {
                                "Protect note"
                            },
                            None,
                            dirty,
                            false,
                            |view, window, cx| view.toggle_protection(window, cx),
                        ))
                        .item(item(
                            &this,
                            "Export Markdown snapshot…",
                            None,
                            busy,
                            false,
                            |view, window, cx| view.choose_output(false, window, cx),
                        ));
                    if imported {
                        menu = menu.item(item(
                            &this,
                            "Coverage details",
                            None,
                            false,
                            false,
                            |view, _, cx| {
                                view.show_coverage = !view.show_coverage;
                                cx.notify();
                            },
                        ));
                    }
                    menu.separator().item(item(
                        &this,
                        "New chat for this note",
                        None,
                        false,
                        false,
                        |view, window, cx| {
                            view.new_writing_thread(window, cx);
                            view.layout.open(TileKind::Chat);
                        },
                    ))
                })
                .into_any_element(),
        );
        controls
    }
    pub(super) fn note_tile(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        if self.review.is_some() {
            return self.review_body(cx);
        }
        self.sync_read_text(cx);
        self.sync_read_marks(cx);
        let p = pal();
        let Some(open) = &self.note else {
            return scroll("note-empty-state")
                .p_4()
                .gap_3()
                .child(empty(
                    "note-empty",
                    "Pick a note from the notes tile or ⌘K, or create one with ⌘N.",
                ))
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(button("note-open-notes").label("Open notes").on_click(
                            cx.listener(|this, _, _, cx| this.show_tile(TileKind::Notes, cx)),
                        ))
                        .child(button("new-note").label("New note ⌘N").on_click(
                            cx.listener(|this, _, window, cx| this.new_note(window, cx)),
                        )),
                )
                .into_any_element();
        };
        let _ = window;
        let RecordData::Note(note) = &open.record.data else {
            return div().into_any_element();
        };
        let dirty = open.session.is_some();
        let blocked = self.recovery_blocked();
        let mut content = div()
            .id("note-tile")
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(self.document_toolbar(cx));
        let mut banners = div()
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .pt_2()
            .flex_shrink_0();
        if let Some(current) = self.records.iter().find(|r| r.id == open.record.id)
            && current.version != open.record.version
        {
            let id = open.record.id.clone();
            banners = banners.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .child(
                        "This note has a newer saved version. Your visible text remains preserved.",
                    )
                    .child(
                        pill("show-current-note", "Open current saved note").on_click(cx.listener(
                            move |this, _, window, cx| this.open_note(&id, None, window, cx),
                        )),
                    ),
            );
        }
        if blocked && let Some(buffer) = self.buffers.iter().find(|s| s.note == open.record.id) {
            let buffer = buffer.clone();
            let id = buffer.note.clone();
            banners = banners.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .child("A recoverable edit exists for this note. The text is read-only until you recover it.")
                    .child(
                        pill("recover-note-edit", "Recover saved edit").on_click(cx.listener(
                            move |this, _, window, cx| this.open_note(&id, Some(buffer.clone()), window, cx),
                        )),
                    ),
            );
        }
        if let Some(coverage) = &note.import {
            let intent = if coverage.full_note {
                "Full imported reference"
            } else {
                "Useful information"
            };
            let (summary, details) = if let Some(report) = &coverage.report {
                (
                    report.summary(),
                    report
                        .issues
                        .iter()
                        .map(ImportIssue::display)
                        .collect::<Vec<_>>(),
                )
            } else {
                (
                    if coverage.gaps.is_empty() {
                        "Coverage not recorded"
                    } else {
                        "Partial / coverage requires inspection"
                    },
                    coverage.gaps.clone(),
                )
            };
            let mut line = div().flex().flex_wrap().items_center().gap_2().child(
                div()
                    .id("note-import-summary")
                    .text_xs()
                    .font_family(MONO)
                    .text_color(rgb(p.muted))
                    .child(format!("{intent} · {summary}"))
                    .test_support(),
            );
            if let Some(record) = self.records.iter().find(|r| r.id == coverage.source)
                && let RecordData::Source(source) = &record.data
            {
                let locator = source.locator.clone();
                line = line.child(pill("open-import-original", "open original").on_click(
                    move |_, _, cx| {
                        if std::path::Path::new(&locator).is_absolute() {
                            cx.open_with_system(std::path::Path::new(&locator));
                        } else if locator.starts_with("https://") || locator.starts_with("http://")
                        {
                            cx.open_url(&locator);
                        }
                    },
                ));
            }
            if !details.is_empty() {
                line = line.child(
                    pill(
                        "toggle-coverage",
                        format!("coverage details ({})", details.len()),
                    )
                    .selected(self.show_coverage)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_coverage = !this.show_coverage;
                        cx.notify();
                    })),
                );
            }
            banners = banners.child(line);
            if self.show_coverage && !details.is_empty() {
                // Provenance may contain long paths and many qualifications.
                // Keep every issue scrollable without consuming the note body.
                banners = banners.child(
                    div()
                        .id("note-coverage-details")
                        .w_full()
                        .max_h(px(120.))
                        .flex_shrink_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .border_1()
                        .border_color(rgb(p.border))
                        .bg(rgb(p.alt))
                        .p_2()
                        .child(
                            div()
                                .text_sm()
                                .child(format!("Coverage details ({})", details.len())),
                        )
                        .children(details.into_iter().enumerate().map(|(index, detail)| {
                            div()
                                .id(format!("note-coverage-detail-{index}"))
                                .flex_shrink_0()
                                .text_sm()
                                .child(detail)
                                .test_support()
                        }))
                        .test_support(),
                );
            }
        }
        if let Some(session) = &open.session
            && self
                .records
                .iter()
                .any(|r| r.id == session.note && r.version != session.base_version)
        {
            banners = banners.child(
                panel()
                    .child("The current note changed since this edit began. Your text is preserved; compare, then save your text as a new note if needed.")
                    .child(div().h(px(180.)).child(markdown(&self.current_view, self.note_assets()))),
            );
        }
        content = content.child(banners);
        if let Some((_, quote)) = &self.selection
            && !blocked
        {
            let mut strip = div()
                .id("selection-toolbar")
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .mx_3()
                .mt_1()
                .px_2()
                .py_1()
                .bg(rgb(p.alt))
                .border_1()
                .border_color(rgb(p.border))
                .text_xs()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(format!("❝ {}", short(quote))),
                )
                .child(
                    pill(
                        "selection-comment",
                        if dirty {
                            "Save and comment"
                        } else {
                            "Comment ⌘/"
                        },
                    )
                    .tooltip(if dirty {
                        "Save this edit, then comment on the same passage"
                    } else {
                        "Comment on this passage"
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this
                            .note
                            .as_ref()
                            .is_some_and(|note| note.session.is_some())
                        {
                            this.save_and_comment(window, cx)
                        } else {
                            this.comment_on_selection(window, cx)
                        }
                    })),
                )
                .child(pill("selection-ask", "Ask in chat").on_click(
                    cx.listener(|this, _, window, cx| this.ask_about_selection(window, cx)),
                ));
            if dirty {
                strip = strip.child(
                    div()
                        .id("selection-comment-reason")
                        .w_full()
                        .text_color(rgb(p.muted))
                        .child("Your edit is recoverable. Save and comment commits it before anchoring feedback to this passage.")
                        .test_support(),
                );
            }
            content = content.child(strip.test_support());
        }
        let text = self.note_editor.read(cx).value().to_string();
        let mut nav = div()
            .flex()
            .flex_wrap()
            .gap_1()
            .px_3()
            .pt_1()
            .flex_shrink_0();
        for (i, (title, range)) in sections(&text).into_iter().enumerate().take(24) {
            nav = nav.child(
                pill(
                    format!("section-{i}"),
                    short(
                        title
                            .strip_prefix("**")
                            .and_then(|value| value.strip_suffix("**"))
                            .unwrap_or(&title),
                    ),
                )
                .on_click(
                    cx.listener(move |this, _, window, cx| this.reveal(range.clone(), window, cx)),
                ),
            );
        }
        if !self.note_reading {
            content = content.child(nav);
        }
        let editor = prose_editor(&self.note_editor)
            .h_full()
            .readonly(blocked)
            .aria_label("Note text")
            .context_menu(move |menu, _, _| {
                menu.menu_with_disabled(
                    if dirty {
                        "Comment on selection · save first"
                    } else {
                        "Comment on selection"
                    },
                    dirty,
                    Box::new(CommentSelection),
                )
                .menu("Ask in chat", Box::new(AskAboutSelection))
                .separator()
                .menu_with_disabled("Cut", blocked, Box::new(gpui_kit::base::input::Cut))
                .menu("Copy", Box::new(gpui_kit::base::input::Copy))
                .menu_with_disabled("Paste", blocked, Box::new(gpui_kit::base::input::Paste))
                .separator()
                .menu("Select All", Box::new(gpui_kit::base::input::SelectAll))
            })
            .into_any_element();
        let editor = if self.note_reading {
            super::document::readable_document(&self.note_read_view, self.note_assets())
                .into_any_element()
        } else {
            editor
        };
        let mut body = div().flex().flex_1().min_h_0().gap_2().p_2().child(
            div()
                .id("note-body")
                .flex_1()
                .min_w_0()
                .min_h_0()
                .px_3()
                .py_2()
                .child(editor)
                .on_mouse_up(
                    gpui_kit::MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.track_selection(cx)),
                )
                .on_key_up(cx.listener(|this, _, _, cx| this.track_selection(cx)))
                .test_support(),
        );
        if self.margin {
            body = body.child(self.margin_column(cx));
        }
        content
            .child(body)
            .child(self.note_bar(cx))
            .test_support()
            .into_any_element()
    }
    fn margin_column(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let Some(open) = &self.note else {
            return div().into_any_element();
        };
        let RecordData::Note(note) = &open.record.data else {
            return div().into_any_element();
        };
        let note_id = open.record.id.clone();
        let dirty = open.session.is_some();
        let mut margin = div()
            .id("writing-feedback")
            .w(px(250.))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_3()
            .pl_2()
            .border_l_1()
            .border_color(rgb(p.border))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .child("Passage comments"),
                    )
                    .child(metadata("Feedback")),
            );
        if let Some((anchor_note, version, _, quote)) = &self.comment_anchor
            && anchor_note == &note_id
        {
            margin = margin.child(
                div()
                    .id("comment-composer")
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .border_1()
                    .border_color(rgb(p.accent))
                    .bg(rgb(p.tile))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(p.muted))
                            .child(format!("anchors to saved v{version}")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .italic()
                            .child(format!("❝ {}", short(quote))),
                    )
                    .child(
                        div().h(px(76.)).child(
                            prose_editor(&self.comment_input)
                                .h_full()
                                .aria_label("Passage feedback"),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(pill("cancel-comment", "cancel").on_click(cx.listener(
                                |this, _, window, cx| {
                                    this.comment_anchor = None;
                                    this.comment_operation = None;
                                    this.comment_input
                                        .update(cx, |e, cx| e.set_value("", window, cx));
                                    this.sync_note_marks(cx);
                                    this.focus_visible_editor(window, cx);
                                    cx.notify();
                                },
                            )))
                            .child(div().flex_1())
                            .child(
                                button("post-comment")
                                    .label("Add comment")
                                    .compact()
                                    .primary()
                                    .disabled(dirty)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.post_comment(window, cx)
                                    })),
                            ),
                    )
                    .test_support(),
            );
        }
        let (mut comments, feedback_error) = match self.workspace.writing_comments(&note_id) {
            Ok(comments) => (comments, None),
            Err(error) => (
                vec![],
                Some(format!(
                    "Could not load pending feedback: {error}. Reload saved note to retry."
                )),
            ),
        };
        if let Some(error) = feedback_error {
            margin = margin.child(error);
        }
        comments.sort_by_key(|entry| {
            let start = match &entry.record.data {
                RecordData::Comment(comment) => {
                    comment_location(comment, open.record.version, &note.markdown)
                        .map_or(usize::MAX, |range| range.start)
                }
                _ => usize::MAX,
            };
            (entry.addressed, start, entry.record.id.clone())
        });
        let applied = self
            .applied_rewrite
            .get(&note_id)
            .and_then(|(request, operation)| {
                self.workspace
                    .writing_request(request)
                    .ok()
                    .filter(|snapshot| snapshot.note.id == note_id)
                    .map(|snapshot| (request.clone(), operation.clone(), snapshot))
            });
        if comments.is_empty() && self.comment_anchor.is_none() {
            margin = margin.child(empty(
                "margin-empty",
                "Select text and press ⌘/ to comment. Comments stay pending until a rewrite is applied and you mark them addressed.",
            ));
        }
        for entry in &comments {
            let RecordData::Comment(comment) = &entry.record.data else {
                continue;
            };
            let id = entry.record.id.clone();
            let location = comment_location(comment, open.record.version, &note.markdown);
            let checked = !entry.addressed && !self.excluded_comments.contains(&id);
            let status = if entry.addressed {
                "addressed · history kept".to_owned()
            } else if location.is_none() {
                format!(
                    "not located in v{} · kept, not re-anchored",
                    open.record.version
                )
            } else if checked {
                "Included in next rewrite".into()
            } else {
                "Excluded from next rewrite".into()
            };
            let card = div()
                .id(format!("comment-card-{id}"))
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(if location.is_some() && !entry.addressed {
                    p.border
                } else {
                    p.alt
                }))
                .bg(rgb(if entry.addressed { p.alt } else { p.tile }))
                .child(div().text_xs().text_color(rgb(p.muted)).child(status))
                .child(div().text_sm().child(comment.body.clone()))
                .child(
                    div()
                        .text_xs()
                        .italic()
                        .text_color(rgb(p.muted))
                        .child(format!("❝ {}", short(&comment.quote))),
                );
            let mut actions = div().flex().flex_wrap().gap_1();
            if !entry.addressed {
                let toggle = id.clone();
                actions = actions.child(
                    pill(
                        format!("include-comment-{id}"),
                        if checked { "Included" } else { "Include" },
                    )
                    .tooltip(if checked {
                        "Exclude from the next rewrite"
                    } else {
                        "Include in the next rewrite"
                    })
                    .disabled(!checked && location.is_none())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.excluded_comments.remove(&toggle) {
                            this.excluded_comments.insert(toggle.clone());
                        }
                        cx.notify();
                    })),
                );
            }
            if let Some(range) = location {
                actions = actions.child(
                    pill(format!("reveal-comment-{id}"), "show passage")
                        .disabled(dirty || self.review.is_some())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.reveal(range.clone(), window, cx)
                        })),
                );
            }
            if let Some((request, operation, snapshot)) = &applied
                && !entry.addressed
                && snapshot.comments.iter().any(|captured| captured.id == id)
            {
                let (request, operation, record) = (request.clone(), operation.clone(), id.clone());
                actions = actions.child(
                    pill(format!("address-comment-{id}"), "mark addressed").on_click(cx.listener(
                        move |this, _, _, cx| {
                            this.confirm_addressed(
                                &request,
                                &operation,
                                std::slice::from_ref(&record),
                                cx,
                            )
                        },
                    )),
                );
            }
            margin = margin.child(card.child(actions).test_support());
        }
        margin.test_support().into_any_element()
    }
    fn note_bar(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let Some(open) = &self.note else {
            return div().into_any_element();
        };
        let note_id = open.record.id.clone();
        let dirty = open.session.is_some();
        let mut bar = div()
            .id("note-bar")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .px_3()
            .py_1()
            .font_family(".SystemUIFont")
            .text_sm()
            .bg(rgb(p.alt))
            .border_t_1()
            .border_color(rgb(p.border));
        let comments = self.workspace.writing_comments(&note_id);
        let feedback_error = comments.is_err();
        let comments = comments.unwrap_or_default();
        let applied = self
            .applied_rewrite
            .get(&note_id)
            .and_then(|(request, operation)| {
                self.workspace
                    .writing_request(request)
                    .ok()
                    .filter(|snapshot| snapshot.note.id == note_id)
                    .map(|snapshot| (request.clone(), operation.clone(), snapshot))
            });
        let selected = self.writing_threads.get(&note_id).cloned();
        // Host request Messages are durable; the most recent is discoverable after restart.
        let latest_request = selected
            .as_ref()
            .and_then(|thread| self.workspace.messages(thread).ok())
            .and_then(|messages| {
                messages.into_iter().rev().find(|record| {
                    matches!(&record.data, RecordData::Message(message) if message.role == "writing-request")
                        && self
                            .workspace
                            .writing_request(&record.id)
                            .is_ok_and(|snapshot| snapshot.note.id == note_id)
                })
            });
        if let Some(record) = &latest_request
            && self.expanded_requests.contains(&record.id)
            && let Ok(snapshot) = self.workspace.writing_request(&record.id)
        {
            let mut disclosure = div()
                .id("writing-requests")
                .max_h(px(220.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_1()
                .font_family(".SystemUIFont")
                .text_sm()
                .child(heading("Saved rewrite inputs"))
                .child(format!(
                    "Saved note v{} · {} comments · {} posted chat messages. Later additions stay pending.",
                    snapshot.note.version,
                    snapshot.comments.len(),
                    snapshot.messages.len()
                ));
            if let RecordData::Note(captured) = &snapshot.note.data {
                disclosure =
                    disclosure
                        .child(heading("Captured saved note"))
                        .child(self.safe_message(
                            format!("captured-note-{}", record.id),
                            captured.markdown.clone(),
                        ));
            }
            disclosure = disclosure.child(heading("Captured chat"));
            for message in &snapshot.messages {
                if let RecordData::Message(body) = &message.data {
                    disclosure = disclosure
                        .child(if body.role == "owner" { "You" } else { "BRN" })
                        .child(self.safe_message(
                            format!("captured-chat-{}", message.id),
                            body.text.clone(),
                        ));
                }
            }
            bar = bar.child(disclosure);
        }
        let mut line = div().flex().flex_wrap().items_center().gap_1();
        if dirty {
            line = line
                .child(
                    div()
                        .text_color(rgb(p.accent))
                        .child("● unsaved edit · recoverable"),
                )
                .child(
                    pill("save-note", "save ⌘S")
                        .primary()
                        .on_click(cx.listener(|this, _, window, cx| this.save(window, cx))),
                )
                .child(
                    pill("discard-note", "discard")
                        .on_click(cx.listener(|this, _, window, cx| this.discard(window, cx))),
                )
                .child(
                    pill("save-buffer-copy", "save as new note")
                        .on_click(cx.listener(|this, _, window, cx| this.save_copy(window, cx))),
                );
        } else if let Some((request, operation, snapshot)) = &applied {
            let open_ids: Vec<String> = snapshot
                .comments
                .iter()
                .filter(|captured| {
                    !comments
                        .iter()
                        .any(|entry| entry.record.id == captured.id && entry.addressed)
                })
                .map(|record| record.id.clone())
                .collect();
            let (request, operation_mark, undo) =
                (request.clone(), operation.clone(), operation.clone());
            let clear = note_id.clone();
            line = line
                .child(
                    div()
                        .id("applied-state")
                        .child(format!(
                            "✓ applied · v{}{}",
                            open.record.version,
                            if open_ids.is_empty() && !snapshot.comments.is_empty() {
                                format!(" · {} comments addressed", snapshot.comments.len())
                            } else {
                                String::new()
                            }
                        ))
                        .test_support(),
                )
                .child(div().flex_1())
                .child(
                    pill(
                        "confirm-addressed",
                        format!("mark {} comments addressed", open_ids.len()),
                    )
                    .disabled(open_ids.is_empty())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.confirm_addressed(&request, &operation_mark, &open_ids, cx)
                    })),
                )
                .child(
                    pill("undo-writing-apply", "undo")
                        .tooltip("Undo the applied rewrite (recorded receipt)")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.undo_receipt(&undo, window, cx)
                        })),
                )
                .child(
                    pill("clear-applied", "✕")
                        .tooltip("Hide; What changed keeps the receipt and its Undo")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.applied_rewrite.remove(&clear);
                            cx.notify();
                        })),
                );
        } else {
            let mut included = 0;
            let mut unresolved_included = false;
            if let Some(open) = &self.note
                && let RecordData::Note(note) = &open.record.data
            {
                for entry in &comments {
                    if let RecordData::Comment(comment) = &entry.record.data
                        && !entry.addressed
                        && !self.excluded_comments.contains(&entry.record.id)
                    {
                        included += 1;
                        unresolved_included |=
                            comment_location(comment, open.record.version, &note.markdown)
                                .is_none();
                    }
                }
            }
            let messages = selected
                .as_ref()
                .and_then(|thread| self.workspace.messages(thread).ok())
                .map(|messages| {
                    messages
                        .iter()
                        .filter(|record| matches!(&record.data, RecordData::Message(m) if matches!(m.role.as_str(), "owner" | "assistant")))
                        .count()
                })
                .unwrap_or_default();
            let why = if self.worker.is_some() {
                "BRN is working"
            } else if feedback_error {
                "feedback could not load"
            } else if unresolved_included {
                "exclude or re-select unlocated comments"
            } else {
                ""
            };
            line = line.child(
                div()
                    .id("rewrite-scope")
                    .child(format!(
                        "Whole note · {included} comment{} · {messages} chat message{}",
                        if included == 1 { "" } else { "s" },
                        if messages == 1 { "" } else { "s" }
                    ))
                    .test_support(),
            );
            if let Some(record) = latest_request {
                let toggle = record.id.clone();
                let retry = record.id.clone();
                line = line
                    .child(
                        pill(
                            "inspect-rewrite-inputs",
                            if self.expanded_requests.contains(&record.id) {
                                "hide saved inputs"
                            } else {
                                "saved inputs ⓘ"
                            },
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !this.expanded_requests.remove(&toggle) {
                                this.expanded_requests.insert(toggle.clone());
                            }
                            cx.notify();
                        })),
                    )
                    .child(
                        pill("retry-rewrite", "retry with these inputs")
                            .disabled(self.worker.is_some())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.retry_rewrite(retry.clone(), cx)
                            })),
                    );
            }
            line = line.child(div().flex_1());
            if !why.is_empty() {
                line = line.child(div().text_color(rgb(p.muted)).child(why));
            }
            line = line.child(
                button("rewrite-note")
                    .label("Rewrite with AI ⌘⇧R")
                    .compact()
                    .primary()
                    .tooltip("Captures the saved note, included comments and this note's chat")
                    .disabled(!why.is_empty())
                    .on_click(cx.listener(|this, _, _, cx| this.rewrite_note(cx))),
            );
        }
        bar.child(line).test_support().into_any_element()
    }
    fn review_body(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let Some(review) = &self.review else {
            return div().into_any_element();
        };
        let protected = review.note.as_ref().is_some_and(|id| {
            self.records.iter().any(|record| {
                &record.id == id && matches!(&record.data, RecordData::Note(note) if note.protected)
            })
        });
        let counts = if review.note.is_some() {
            let current = review
                .note
                .as_ref()
                .and_then(|id| self.workspace.store.note(id).ok().flatten())
                .and_then(|record| match record.data {
                    RecordData::Note(note) => Some(note.markdown),
                    _ => None,
                })
                .unwrap_or_default();
            let tracked = diff::tracked(&current, &review.original);
            format!("whole note · +{} −{}", tracked.added, tracked.removed)
        } else {
            format!("{} changes", review.candidate.request.writes.len())
        };
        let partial = review.selected.len() != review.groups.groups.len();
        let mut bar = div()
            .id("review-bar")
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .px_3()
            .py_1()
            .flex_shrink_0()
            .font_family(MONO)
            .text_xs()
            .bg(rgb(p.alt))
            .border_b_1()
            .border_color(rgb(p.border))
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .child("proposal"),
            )
            .child(div().text_color(rgb(p.muted)).child(counts))
            .child(
                pill("review-inputs", "ⓘ inputs")
                    .selected(self.review_inputs)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.review_inputs = !this.review_inputs;
                        cx.notify();
                    })),
            )
            .child(
                pill("review-compare", "compare")
                    .tooltip("Open a side-by-side tile")
                    .on_click(cx.listener(|this, _, _, cx| this.show_tile(TileKind::Compare, cx))),
            );
        if review.note.is_some() {
            bar = bar.child(
                pill(
                    "review-read-result",
                    if self.review_reading {
                        "Show changes"
                    } else {
                        "Read result"
                    },
                )
                .disabled(self.review_editing)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.set_review_reading(!this.review_reading, window, cx)
                })),
            );
            bar = if self.review_editing {
                bar.child(
                    pill("prepare-final", "use this text")
                        .disabled(!review.edited)
                        .tooltip("Creates a new candidate from your final text")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.prepare_review_edit(window, cx)),
                        ),
                )
                .child(pill("cancel-final", "cancel edit").on_click(
                    cx.listener(|this, _, window, cx| this.cancel_review_edit(window, cx)),
                ))
            } else {
                bar.child(
                    pill("review-edit", "edit final text")
                        .tooltip("Edit the final text; this creates a new candidate")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.start_review_edit(window, cx)),
                        ),
                )
            };
        }
        bar = bar
            .child(div().flex_1())
            .child(
                pill("close-review", "keep for later")
                    .on_click(cx.listener(|this, _, window, cx| this.keep_review(window, cx))),
            )
            .child(
                pill("dismiss-proposal", "dismiss")
                    .on_click(cx.listener(|this, _, window, cx| this.dismiss_review(window, cx))),
            )
            .child(
                button("apply-final")
                    .label(if protected {
                        "Approve whole note"
                    } else {
                        "Apply whole note"
                    })
                    .compact()
                    .primary()
                    .tooltip("Approval binds exactly this displayed candidate")
                    .disabled(review.edited || partial)
                    .on_click(cx.listener(|this, _, window, cx| this.apply_review(window, cx))),
            );
        let mut details = div()
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .py_1()
            .flex_shrink_0()
            .text_sm();
        if self.review_inputs || review.note.is_none() {
            details = details.child(review.candidate.request.reason.clone());
            for write in &review.candidate.request.writes {
                details = details.child(metadata(match &write.data {
                    RecordData::Note(n) => format!(
                        "Note: {}{}",
                        n.title,
                        if n.protected { " · protected" } else { "" }
                    ),
                    RecordData::Action(a) => format!("Action: {} · {:?}", a.description, a.state),
                    RecordData::Thread(t) => format!("Chat: {}", t.title),
                    RecordData::Message(m) => format!("Message: {}", short(&m.text)),
                    RecordData::Link(_) => "Connect related work".into(),
                    RecordData::Comment(c) => format!("Comment: {}", short(&c.body)),
                    RecordData::Source(s) => format!("Source: {}", s.label),
                    RecordData::Asset(_) => "Retain a meaningful figure".into(),
                    RecordData::Run(_) => "Record work progress".into(),
                    RecordData::Settings(_) => "Change workspace settings".into(),
                    RecordData::ChatControl(_) => "Change chat controls".into(),
                }));
            }
            if let Some(snapshot) = review
                .candidate
                .request
                .inputs
                .iter()
                .find_map(|input| self.workspace.writing_request(&input.record).ok())
            {
                details = details.child(metadata(format!(
                    "Inputs: saved v{} · {} comments · {} chat messages",
                    snapshot.note.version,
                    snapshot.comments.len(),
                    snapshot.messages.len()
                )));
            }
            details = details.child(metadata("Applying saves this exact proposal. Changed or actively edited notes remain unchanged until the issue is resolved. No per-change acceptance."));
        }
        if review.groups.groups.len() > 1 {
            details = details.child("Choose complete notes and Actions. Prepare your selection to review a new immutable proposal.");
            let mut groups = div().flex().flex_wrap().gap_1();
            for group in &review.groups.groups {
                let id = group.id.clone();
                let included = review.selected.contains(&id);
                groups = groups.child(
                    pill(
                        format!("review-group-{id}"),
                        format!(
                            "{} {}: {}",
                            if included { "Include" } else { "Omit" },
                            group.kind,
                            short(&group.title)
                        ),
                    )
                    .selected(included)
                    .disabled(review.edited || !review.groups.ungrouped_writes.is_empty())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(review) = &mut this.review
                            && !review.selected.remove(&id)
                        {
                            review.selected.insert(id.clone());
                        }
                        cx.notify();
                    })),
                );
                if !group.required_groups.is_empty() {
                    let required = group
                        .required_groups
                        .iter()
                        .filter_map(|id| review.groups.groups.iter().find(|g| &g.id == id))
                        .map(|g| g.title.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    groups = groups.child(metadata(format!("Requires: {required}")));
                }
            }
            details =
                details.child(groups).child(
                    pill("prepare-selected", "prepare selected groups")
                        .disabled(review.edited || review.selected.is_empty() || !partial)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.prepare_review_selection(window, cx)
                        })),
                );
            if !review.groups.ungrouped_writes.is_empty() {
                details = details.child("This proposal has shared changes that cannot be separated safely. Keep the complete proposal or request a rewrite.");
            }
        }
        let notes: Vec<_> = review
            .candidate
            .request
            .writes
            .iter()
            .filter_map(|write| match &write.data {
                RecordData::Note(note) => Some((write.id.clone(), note.title.clone())),
                _ => None,
            })
            .collect();
        if notes.len() > 1 {
            let mut members = div().flex().flex_wrap().gap_1();
            for (id, name) in notes {
                let selected = review.note.as_ref() == Some(&id);
                members = members.child(
                    pill(
                        format!("review-member-{id}"),
                        format!("show {}", short(&name)),
                    )
                    .selected(selected)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_review_note(id.clone(), window, cx)
                    })),
                );
            }
            details = details.child(members);
        }
        let body = if review.note.is_none() {
            div().into_any_element()
        } else if self.review_editing {
            prose_editor(&self.review_editor)
                .h_full()
                .aria_label("Final proposed note text")
                .into_any_element()
        } else if self.review_reading {
            super::document::readable_document(&self.candidate_view, self.note_assets())
                .into_any_element()
        } else {
            prose_editor(&self.diff_editor)
                .h_full()
                .readonly(true)
                .aria_label("Proposed changes")
                .into_any_element()
        };
        let reason = format!(
            "reviewing {} · approval binds this exact text{}",
            if self.review_editing {
                "your final text · use it to create a new candidate"
            } else {
                "the candidate"
            },
            review
                .candidate
                .request
                .inputs
                .iter()
                .find_map(|input| self.workspace.writing_request(&input.record).ok())
                .map(|snapshot| format!(
                    " · {} captured comments stay pending until you mark them",
                    snapshot.comments.len()
                ))
                .unwrap_or_default()
        );
        let mut document = div()
            .flex()
            .size_full()
            .min_h_0()
            .min_w_0()
            .child(div().flex_1().min_w_0().h_full().child(body));
        if self.margin
            && self
                .note
                .as_ref()
                .is_some_and(|open| review.note.as_deref() == Some(open.record.id.as_str()))
        {
            document = document.child(self.margin_column(cx));
        }
        div()
            .id("review-panel")
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(bar.test_support())
            .child(details)
            .child(
                div()
                    .id("review-body")
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .child(document)
                    .test_support(),
            )
            .child(
                div()
                    .id("note-bar")
                    .px_3()
                    .py_1()
                    .flex_shrink_0()
                    .font_family(MONO)
                    .text_xs()
                    .bg(rgb(p.alt))
                    .border_t_1()
                    .border_color(rgb(p.border))
                    .child(reason)
                    .test_support(),
            )
            .test_support()
            .into_any_element()
    }
    pub(super) fn compare_tile(&self, _: &mut Context<Self>) -> gpui_kit::AnyElement {
        if self
            .review
            .as_ref()
            .is_none_or(|review| review.note.is_none())
        {
            return scroll("compare-content")
                .child(empty("compare-empty", "No whole-note proposal to compare."))
                .into_any_element();
        }
        let version = self
            .review
            .as_ref()
            .and_then(|review| review.note.as_ref())
            .and_then(|id| self.records.iter().find(|record| &record.id == id))
            .map_or(0, |record| record.version);
        div()
            .id("compare-content")
            .size_full()
            .flex()
            .gap_2()
            .p_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(section(format!("CURRENT · v{version}")))
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(markdown(&self.current_view, self.note_assets())),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(section("CANDIDATE · exact text under review"))
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .child(markdown(&self.candidate_view, self.note_assets())),
                    ),
            )
            .test_support()
            .into_any_element()
    }
    pub(super) fn linked_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let mut context = scroll("thread-linked-work").gap_3().p_3();
        let Some(id) = self.chat_thread() else {
            return context
                .child(empty(
                    "linked-empty",
                    "Notes, sources and Actions used in a chat appear here.",
                ))
                .test_support()
                .into_any_element();
        };
        let linked_ids = linked(&self.records, id);
        if linked_ids.is_empty() {
            context = context.child(empty(
                "linked-empty",
                "Notes, sources and Actions used in this chat appear here.",
            ));
        }
        for record in self
            .records
            .iter()
            .filter(|r| !r.archived && linked_ids.contains(&r.id))
        {
            match &record.data {
                RecordData::Note(note) => {
                    let id = record.id.clone();
                    context = context.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .p_2()
                            .border_1()
                            .border_color(rgb(p.border))
                            .child(section(format!(
                                "NOTE · v{}{}",
                                record.version,
                                if note.protected { " · protected" } else { "" }
                            )))
                            .child(
                                div()
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .child(note.title.clone()),
                            )
                            .child(
                                self.safe_message(
                                    format!("linked-note-preview-{id}"),
                                    note.markdown.clone(),
                                )
                                .max_lines(8),
                            )
                            .child(
                                pill(format!("linked-note-{id}"), "open in note tile").on_click(
                                    cx.listener(move |this, _, window, cx| {
                                        this.open_note(&id, None, window, cx)
                                    }),
                                ),
                            ),
                    );
                }
                RecordData::Action(_) => context = context.child(self.action_row(record, cx)),
                RecordData::Source(source) => {
                    let locator = source.locator.clone();
                    let open_locator = locator.clone();
                    context = context.child(
                        panel()
                            .child(section("SOURCE"))
                            .child(source.label.clone())
                            .child(metadata(short(&source.locator)))
                            .child(source.outcome.clone())
                            .children(
                                source
                                    .report
                                    .as_ref()
                                    .map(|report| {
                                        report
                                            .issues
                                            .iter()
                                            .map(ImportIssue::display)
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_else(|| source.gaps.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .child(
                                        pill(format!("open-source-{}", record.id), "open")
                                            .on_click(move |_, _, cx| {
                                                if std::path::Path::new(&open_locator).is_absolute()
                                                {
                                                    cx.open_with_system(std::path::Path::new(
                                                        &open_locator,
                                                    ));
                                                } else if open_locator.starts_with("https://")
                                                    || open_locator.starts_with("http://")
                                                    || open_locator.starts_with("outlook:")
                                                {
                                                    cx.open_url(&open_locator);
                                                }
                                            }),
                                    )
                                    .child(
                                        pill(
                                            format!("copy-source-{}", record.id),
                                            "copy reference",
                                        )
                                        .on_click(
                                            move |_, _, cx| {
                                                cx.write_to_clipboard(
                                                    gpui_kit::ClipboardItem::new_string(
                                                        locator.clone(),
                                                    ),
                                                )
                                            },
                                        ),
                                    ),
                            ),
                    );
                }
                RecordData::Comment(comment) => {
                    let note = self.records.iter().find(|r| r.id == comment.note);
                    let location = note.and_then(|r| match &r.data {
                        RecordData::Note(n) => comment_location(comment, r.version, &n.markdown),
                        _ => None,
                    });
                    let mut passage = panel()
                        .child(section("PASSAGE"))
                        .child(format!("❝ {}", comment.quote))
                        .child(comment.body.clone())
                        .child(metadata(if location.is_some() {
                            "Located in the current note"
                        } else {
                            "Location unresolved · preserved from an earlier note version"
                        }));
                    if let Some(note) = note {
                        let id = note.id.clone();
                        passage = passage.child(
                            pill(
                                format!("comment-note-{}", record.id),
                                format!("open {}", short(&title(note))),
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| this.open_note(&id, None, window, cx),
                            )),
                        );
                    }
                    context = context.child(passage);
                }
                _ => {}
            }
        }
        context.test_support().into_any_element()
    }
}
