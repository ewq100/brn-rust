//! Writing and conversation tiles: chat, note, in-place review, compare and
//! linked work. Every action calls the same checked services as before.
use super::components::AccessibleDropdownMenu;
use super::shell::{item, one_line, pill};
use super::state::ReviewSummary;
use super::tiles::scroll;
use super::*;
use gpui_kit::SharedString;
use gpui_kit::StyledText;
use gpui_kit::WeakEntity;
use gpui_kit::component::button::ButtonVariants;

impl ThreadsUi {
    pub(super) fn safe_message(&self, id: String, text: String) -> TextView {
        self.safe_message_for_thread(id, text, self.chat_thread())
    }
    pub(super) fn safe_message_for_thread(
        &self,
        id: String,
        text: String,
        thread: Option<&str>,
    ) -> TextView {
        safe_markdown(
            TextView::markdown(id, text).w_full(),
            self.assets_in_chat(thread),
        )
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
    /// The conversation's header actions (spec §6.1): the project where
    /// relevant, the per-conversation Activity disclosure, Linked work and
    /// one ⋯ menu. Beside a document (the assistant) the header is smaller:
    /// Activity and ⋯, whose menu then holds Linked work.
    pub(super) fn chat_controls(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let this = cx.entity().downgrade();
        let primary = self.layout.primary() == TileKind::Chat;
        let mut controls = vec![];
        let thread = self.chat_thread().map(str::to_owned);
        if primary
            && let Some(project) = thread
                .as_deref()
                .and_then(|chat| project_for_chat(&self.records, chat))
        {
            let (id, name) = (project.id.clone(), one_line(&title(project)));
            controls.push(
                pill("chat-project", short(&name))
                    .icon(IconName::Folder)
                    .min_w_0()
                    .flex_shrink(1.)
                    .accessibility_label(format!("Project {name}"))
                    .tooltip(format!("Open project {name}"))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.projects_trash = false;
                        this.selected_project = Some(id.clone());
                        this.show_tile(TileKind::Projects, window, cx)
                    }))
                    .into_any_element(),
            );
        }
        if let Some(id) = &thread {
            let open = self.conversation.activity_open;
            let last = self
                .last_work_status(id)
                .unwrap_or_else(|| "no reply recorded yet".into());
            controls.push(
                pill("chat-activity", "Activity")
                    .icon(IconName::ListChecks)
                    .selected(open)
                    .accessibility_label(format!(
                        "{} activity · {last}",
                        if open { "Hide" } else { "Show" }
                    ))
                    .tooltip(last)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.conversation.activity_open = !this.conversation.activity_open;
                        cx.notify();
                    }))
                    .into_any_element(),
            );
            if primary {
                let count = linked(&self.records, id).len();
                let shown = self.layout.support() == Some(TileKind::Linked)
                    && self.arrangement.support_visible();
                controls.push(
                    pill(
                        "chat-linked",
                        if count > 0 {
                            format!("Linked · {count}")
                        } else {
                            "Linked".into()
                        },
                    )
                    .icon(IconName::Link)
                    .selected(shown)
                    .tooltip("Notes, sources and tasks used in this chat")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if shown {
                            this.relayout(window, cx, |layout| layout.close(TileKind::Linked));
                        } else {
                            this.show_tile(TileKind::Linked, window, cx);
                        }
                    }))
                    .into_any_element(),
                );
            }
            if self.thread_resolved(id) {
                controls.push(
                    components::status_chip(
                        "chat-archived",
                        None,
                        Some(IconName::Archive),
                        "Archived",
                    )
                    .into_any_element(),
                );
            }
        }
        controls.push(self.chat_menu(&this, primary));
        controls
    }
    /// The conversation ⋯ menu: the note's chats, New chat, All chats,
    /// Capture useful outcomes, Move to project, Open note, Linked work,
    /// Quiet mode and Archive / Unarchive.
    fn chat_menu(&self, this: &WeakEntity<Self>, primary: bool) -> gpui_kit::AnyElement {
        let this = this.clone();
        let note = self
            .writing
            .note
            .as_ref()
            .map(|open| (open.record().id.clone(), title(open.record())));
        let threads: Vec<(String, String)> = note
            .as_ref()
            .and_then(|(id, _)| self.workspace.writing_threads(id).ok())
            .unwrap_or_default()
            .iter()
            .map(|record| (record.id.clone(), title(record)))
            .collect();
        let selected = self.chat_thread().map(str::to_owned);
        let resolved = selected
            .as_deref()
            .is_some_and(|id| self.thread_resolved(id));
        let chat_note = selected.as_deref().and_then(|id| self.thread_note(id));
        let busy = self.worker.is_some();
        let quiet = self.queue_state().quiet;
        components::overflow_menu("chat-menu", "Chat actions")
            .accessible_dropdown_menu(move |mut menu, _, _| {
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
                    menu = menu
                        .item(item(
                            &this,
                            "New chat for this note",
                            None,
                            false,
                            false,
                            |view, window, cx| view.new_writing_thread(window, cx),
                        ))
                        .separator();
                }
                menu = menu
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
                        |view, window, cx| view.show_tile(TileKind::AllThreads, window, cx),
                    ));
                let Some(chat) = selected.clone() else {
                    return menu;
                };
                menu = menu.separator();
                if !resolved {
                    menu = menu.item(item(
                        &this,
                        "Capture useful outcomes…",
                        None,
                        busy,
                        false,
                        |view, window, cx| view.capture_outcomes(window, cx),
                    ));
                }
                let moving = chat.clone();
                menu = menu.item(item(
                    &this,
                    "Move to project…",
                    None,
                    false,
                    false,
                    move |view, window, cx| view.move_chat_dialog(&moving, window, cx),
                ));
                if let Some(note) = chat_note.clone() {
                    menu = menu.item(item(
                        &this,
                        "Open note",
                        None,
                        false,
                        false,
                        move |view, window, cx| view.open_note(&note, None, window, cx),
                    ));
                }
                if !primary {
                    menu = menu.item(item(
                        &this,
                        "Linked work",
                        None,
                        false,
                        false,
                        |view, window, cx| view.show_tile(TileKind::Linked, window, cx),
                    ));
                }
                menu = menu
                    .item(item(
                        &this,
                        "Quiet mode",
                        None,
                        false,
                        quiet,
                        |view, _, cx| view.toggle_quiet(cx),
                    ))
                    .separator();
                let lifecycle = chat.clone();
                menu.item(item(
                    &this,
                    if resolved { "Unarchive" } else { "Archive" },
                    None,
                    false,
                    false,
                    move |view, _, cx| {
                        if resolved {
                            view.reopen_thread(&lifecycle, cx)
                        } else {
                            view.resolve_thread(&lifecycle, cx)
                        }
                    },
                ))
            })
            .into_any_element()
    }
    /// The conversation workspace: the transcript in its centred column,
    /// then, above the one composer, the activity disclosure (when open),
    /// the running line, a permission request, waiting messages and
    /// attachments. A new chat in the primary view is the start surface.
    pub(super) fn chat_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let primary = self.layout.primary() == TileKind::Chat;
        let chat = self.chat_thread().map(str::to_owned);
        if chat.is_none() && primary {
            return self.start_surface(cx);
        }
        let body = match &chat {
            Some(id) => self.conversation(id, cx),
            None => self.assistant_start(cx),
        };
        // A rewrite's progress belongs to its document, not the chat.
        let writing_run = self
            .worker
            .as_ref()
            .and_then(Worker::run_identity)
            .and_then(|run| self.workspace.store.record(&run.run_id).ok().flatten())
            .and_then(|record| match record.data {
                RecordData::Run(run) => run.instruction,
                _ => None,
            })
            .is_some_and(|id| self.workspace.writing_request(&id).is_ok());
        let activity = chat
            .as_deref()
            .filter(|_| self.conversation.activity_open)
            .map(|id| self.activity_panel(id, cx));
        let supports = [activity, self.queue_panel(cx), self.attachments_panel(cx)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let footer = super::chat::transcript_column(primary)
            .flex_shrink_0()
            .gap_2()
            .pt_1()
            .pb_3()
            .when(!supports.is_empty(), |footer| {
                // Supporting rows scroll inside a bounded region so the
                // transcript stays reachable in a small window.
                footer.child(
                    div()
                        .id("chat-supports")
                        .flex()
                        .flex_col()
                        .gap_2()
                        .max_h(px(260.))
                        .overflow_y_scroll()
                        .children(supports)
                        .test_support(),
                )
            })
            .children((!writing_run).then(|| self.run_status_line()).flatten())
            .children(self.mcp_permission_card(cx))
            .child(self.composer_tile(cx));
        div()
            .id("chat-tile")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(super::slash::bounds_probe(&self.slash.area_bounds))
            .child(div().flex_1().min_h_0().child(body))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_shrink_0()
                    .w_full()
                    .when(primary, |row| row.items_center())
                    .child(footer),
            )
            .test_support()
            .into_any_element()
    }
    /// The New chat start surface (§4.3): a short greeting, the composer
    /// and, while unchosen, the delegation choice. The composer's + menu
    /// holds attaching and new documents; no row repeats them. Existing
    /// manual work stays available before choosing.
    fn start_surface(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let (heading, hint) = match &self.chat {
            ChatTarget::NewForNote(note) => (
                format!(
                    "New chat about {}",
                    self.note_title(note)
                        .map_or_else(|| note.clone(), |title| short(&title))
                ),
                "This chat stays linked to the note.",
            ),
            _ => ("What are we working on?".to_owned(), "Ask anything."),
        };
        let column = super::chat::transcript_column(true)
            .gap_4()
            .py(components::tokens::SPACE_8)
            .child(
                div()
                    .id("chat-start")
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("chat-start-heading")
                            .text_size(px(theme::metrics::TEXT_ARTICLE_HEADING))
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .text_color(rgb(p.ink))
                            .aria_label(heading.clone())
                            .child(heading)
                            .test_support(),
                    )
                    .child(
                        div()
                            .text_size(UI_TEXT)
                            .text_color(rgb(p.muted))
                            .child(hint),
                    )
                    .test_support(),
            )
            .children(self.queue_panel(cx))
            .child(self.composer_tile(cx))
            .when(self.delegation_host() == Some(TileKind::Chat), |column| {
                column.child(self.working_knowledge(cx))
            });
        div()
            .id("chat-tile")
            .relative()
            .size_full()
            .child(super::slash::bounds_probe(&self.slash.area_bounds))
            .child(
                scroll("thread-conversation")
                    .items_center()
                    .justify_center()
                    .px_0()
                    .child(column)
                    .test_support(),
            )
            .test_support()
            .into_any_element()
    }
    /// A new chat beside a document: what it is, above the composer.
    fn assistant_start(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let (heading, body) = match &self.chat {
            ChatTarget::NewForNote(note) => (
                format!(
                    "New chat about {}",
                    self.note_title(note)
                        .map_or_else(|| note.clone(), |title| short(&title))
                ),
                "It stays linked to the note.".to_owned(),
            ),
            _ => (
                "New chat".to_owned(),
                "Ask below. Attach files for this chat, or let BRN use your saved notes."
                    .to_owned(),
            ),
        };
        let mut start = scroll("thread-conversation")
            .gap_3()
            .p_4()
            .child(components::empty_state(
                "chat-start",
                IconName::MessageSquare,
                heading,
                body,
                None,
            ));
        if self.delegation_host() == Some(TileKind::Chat) {
            start = start.child(self.working_knowledge(cx));
        }
        start.test_support().into_any_element()
    }
    pub(super) fn conversation_detail(
        &self,
        id: &str,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let Some(record) = self.records.iter().find(|r| r.id == id) else {
            return scroll("thread-conversation")
                .child(components::empty_state(
                    "thread-unavailable",
                    IconName::MessageSquare,
                    "Chat unavailable",
                    "Open another from Chats or ⌘K.",
                    None,
                ))
                .into_any_element();
        };
        let RecordData::Thread(thread) = &record.data else {
            return div().into_any_element();
        };
        let queue = self.queue_state();
        let messages = self.workspace.messages(id).unwrap_or_default();
        let path = self.continue_chat_path(&messages, id);
        let recovering = self.shows_recovery_card(id, &thread.state, &path);
        // Rewrite proposals belong to the Note.
        let cards: Vec<(usize, &Attention, Option<&Prepared>)> = thread
            .attention
            .iter()
            .enumerate()
            .filter_map(|(index, attention)| {
                let candidate = self
                    .candidates
                    .iter()
                    .find(|c| attention.record.as_deref() == Some(c.operation.as_str()));
                (!candidate.is_some_and(|c| self.writing_candidate_note(c).is_some()))
                    .then_some((index, attention, candidate))
            })
            .collect();
        // Only the newest card awaiting the owner is filled: the recovery
        // card below the others, else the last proposal.
        let lead = if recovering {
            None
        } else {
            cards
                .iter()
                .rev()
                .find_map(|(index, _, candidate)| candidate.map(|_| *index))
        };
        let mut conversation =
            super::chat::transcript_column(self.centered(TileKind::Chat)).gap_3();
        for (index, attention, candidate) in cards {
            conversation = conversation.child(self.attention_card(
                index,
                attention,
                candidate,
                lead == Some(index),
                cx,
            ));
        }
        if self.latest_writing_instruction(id).is_none()
            && let Some(stream) = self.stream_for(id)
        {
            conversation = conversation.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(metadata("Writing…"))
                    .child(self.safe_message(
                        format!(
                                "response-{}",
                                self.worker
                                    .as_ref()
                                    .and_then(Worker::run_identity).map(|run| run.run_id.as_str())
                                    .unwrap_or(id)
                            ),
                        stream.into(),
                    )),
            );
        }
        if recovering {
            conversation = conversation.child(self.recovery_card(id, &queue, &path, cx));
        }
        // Capture useful outcomes stays one visible step after the latest
        // exchange as well as in the ⋯ menu (proposals wait for review).
        if !messages.is_empty() && !self.thread_resolved(id) {
            conversation = conversation.child(
                div().flex().child(
                    pill("capture-outcomes", "Capture outcomes…")
                        .icon(IconName::ListChecks)
                        .disabled(self.worker.is_some())
                        .tooltip("Propose useful notes, decisions and follow-ups for review; nothing is saved automatically")
                        .on_click(cx.listener(|this, _, window, cx| this.capture_outcomes(window, cx))),
                ),
            );
        }
        scroll("thread-conversation")
            .py_4()
            .px_0()
            .when(self.transcript_centered(), |detail| detail.items_center())
            .child(conversation)
            .test_support()
            .into_any_element()
    }
    /// The transcript centers when the conversation is the primary view;
    /// as the assistant beside a document it sits left like the note.
    fn transcript_centered(&self) -> bool {
        self.centered(TileKind::Chat)
    }
    /// A needs-you card. A question (or a review whose proposal is gone) is a
    /// warning with the recorded reason; a review names its target notes.
    fn attention_card(
        &self,
        index: usize,
        attention: &Attention,
        candidate: Option<&Prepared>,
        lead: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let id = format!("attention-card-{index}");
        if let Some(candidate) = candidate {
            let summary = review_summary(candidate, &self.records);
            let label = review_label(&summary, &candidate.request.reason);
            return components::card(id, None)
                .aria_label(format!(
                    "Needs you · {label}{}",
                    if summary.protected {
                        " · Protected"
                    } else {
                        ""
                    }
                ))
                .child(self.proposal_card(candidate, &summary, lead, cx))
                .test_support()
                .into_any_element();
        }
        components::card(id.clone(), Some(Tone::Warning))
            .aria_label(format!("Needs you · {}", attention.reason))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(components::tone_mark(format!("{id}-tone"), Tone::Warning))
                    .child(
                        div()
                            .text_size(SMALL_TEXT)
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child("Needs you"),
                    ),
            )
            .child(div().min_w_0().child(attention.reason.clone()))
            .test_support()
            .into_any_element()
    }
    /// The chat's recovery card shows when Continue applies. A writing
    /// continuation never renders in chat; the Note's Retry saved rewrite
    /// owns it.
    fn shows_recovery_card(&self, thread: &str, state: &ThreadState, path: &ContinuePath) -> bool {
        !matches!(path, ContinuePath::RetryRewrite(_))
            && self
                .chat_runs
                .can_continue(thread, state, self.worker.is_some())
    }
    /// The chat's latest run when it is saved as Working and this window
    /// does not hold it: it may still run in another window or the CLI, so
    /// the recovery card offers Stop….
    fn saved_run_to_stop(&self, thread: &str) -> Option<String> {
        let latest = self.chat_runs.latest(thread)?;
        let local = self
            .worker
            .as_ref()
            .and_then(Worker::run_identity)
            .map(|run| run.run_id.as_str());
        (matches!(&latest.data, RecordData::Run(run) if run.state == RunState::Working)
            && local != Some(latest.id.as_str()))
        .then(|| latest.id.clone())
    }
    /// The run whose Stop… the visible Chat tile's recovery card renders, so
    /// the Note bar leaves that one control to the card (IDs stay unique).
    pub(super) fn chat_card_stop(&self) -> Option<String> {
        // Only a drawn Chat pane renders that card.
        self.panel_width(TileKind::Chat)?;
        let thread = self.chat_thread()?;
        let RecordData::Thread(data) = &self.records.iter().find(|r| r.id == thread)?.data else {
            return None;
        };
        let messages = self.workspace.messages(thread).unwrap_or_default();
        let path = self.continue_chat_path(&messages, thread);
        self.shows_recovery_card(thread, &data.state, &path)
            .then(|| self.saved_run_to_stop(thread))
            .flatten()
    }
    /// The open chat shows a recovery card whose Continue or Resume queue is
    /// the chat's filled control, so Send stays outline beside it.
    pub(super) fn recovery_card_leads(&self) -> bool {
        let Some(thread) = self.chat_thread() else {
            return false;
        };
        let Some(RecordData::Thread(data)) = self
            .records
            .iter()
            .find(|r| r.id == thread)
            .map(|r| &r.data)
        else {
            return false;
        };
        let messages = self.workspace.messages(thread).unwrap_or_default();
        let path = self.continue_chat_path(&messages, thread);
        !self.thread_resolved(thread) && self.shows_recovery_card(thread, &data.state, &path)
    }
    /// The recovery card: the recorded state and its reason in full, then
    /// what the button does and what stays paused. Its tone is the strongest
    /// line's. `continue-{run}` (or `continue-{chat}` without a run) is the
    /// primary, except in an archived chat, where it is unavailable until
    /// Unarchive. Stop… does not depend on Archive, so `chat_card_stop`
    /// stays in step with the card.
    fn recovery_card(
        &self,
        thread: &str,
        queue: &ChatControl,
        path: &ContinuePath,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let latest = self.chat_runs.latest(thread);
        let run = latest.and_then(|record| match &record.data {
            RecordData::Run(run) => Some(run),
            _ => None,
        });
        let owner_after = self.chat_runs.owner_message_after_latest(thread);
        // The chat's recorded notice for its latest run (U10), unless a
        // newer owner message makes line 1 "Not answered yet".
        let notice = latest
            .zip(run)
            .filter(|_| !owner_after)
            .and_then(|(record, run)| {
                let RecordData::Thread(chat) = &self.records.iter().find(|r| r.id == thread)?.data
                else {
                    return None;
                };
                terminal_notice(run, &record.id, &chat.attention).map(|(_, notice)| notice)
            });
        let mut card = recovery(
            path,
            self.pending_run_working(queue),
            run,
            owner_after,
            queue,
            thread,
        )
        .with_notice(notice);
        // Archive keeps the chat Open, but the service refuses a run there:
        // the composer banner's Unarchive is the next action.
        if self.thread_resolved(thread) {
            card = card.archived();
        }
        let tone = severity_tone(card.severity());
        let id = format!("recovery-card-{thread}");
        let stop = self
            .saved_run_to_stop(thread)
            .map(|run| self.stop_saved_run_button(&run, cx));
        let thread_id = thread.to_owned();
        let line = |suffix: &str, text: String| {
            div()
                .id(SharedString::from(format!("{id}-{suffix}")))
                .min_w_0()
                .aria_label(text.clone())
                .child(text)
                .test_support()
        };
        components::card(id.clone(), tone)
            .aria_label(card.label())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .children(tone.map(|tone| components::tone_mark(format!("{id}-tone"), tone)))
                    .child(
                        line("state", card.word.to_owned())
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD),
                    ),
            )
            // The recorded reason wraps in full, never truncated.
            .when_some(card.reason.clone(), |card, reason| {
                card.child(line("reason", reason))
            })
            .when_some(card.detail.clone(), |card, detail| {
                card.child(
                    line("detail", detail)
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(pal().muted)),
                )
            })
            .child(line("next", card.next.clone()))
            .when_some(card.waiting.clone(), |card, waiting| {
                card.child(line("waiting", waiting))
            })
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .justify_end()
                    .gap_1()
                    .children(stop)
                    .child({
                        let control = button(format!(
                            "continue-{}",
                            latest.map_or(thread, |record| record.id.as_str())
                        ))
                        .label(card.action)
                        .compact()
                        .h(components::tokens::COMPACT_CONTROL);
                        if card.available {
                            control
                                .primary()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.continue_run(thread_id.clone(), cx)
                                }))
                        } else {
                            // Unarchive (the composer banner) is the only
                            // filled control.
                            control
                                .outline()
                                .disabled(true)
                                .accessibility_label(format!(
                                    "{}, unavailable: unarchive to continue",
                                    card.action
                                ))
                                .tooltip("Unarchive to continue")
                        }
                    }),
            )
            .test_support()
            .into_any_element()
    }
    /// A compact preview of one exact candidate; Apply binds that candidate.
    /// It sits inside its attention card, which holds the only border. The
    /// heading names every target note with the total +a −r and marks a
    /// protected one; `lead` fills Apply/Approve for the newest proposal.
    pub(super) fn proposal_card(
        &self,
        candidate: &Prepared,
        summary: &ReviewSummary,
        lead: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let p = pal();
        let op = candidate.operation.as_str().to_owned();
        let protected = summary.protected;
        let (mut added, mut removed, mut preview) = (0, 0, None);
        for write in &candidate.request.writes {
            let RecordData::Note(proposed) = &write.data else {
                continue;
            };
            let current = self
                .workspace
                .store
                .note(&write.id)
                .ok()
                .flatten()
                .and_then(|record| match record.data {
                    RecordData::Note(note) => Some(note.markdown),
                    _ => None,
                });
            let diff = diff::preview(current.as_deref().unwrap_or(""), &proposed.markdown, 2);
            added += diff.added;
            removed += diff.removed;
            // The first note's lines preview the proposal.
            preview.get_or_insert(diff);
        }
        let label = review_label(summary, &candidate.request.reason);
        let counts = (!summary.note_titles.is_empty()).then(|| format!("+{added} −{removed}"));
        let announced = [
            Some(label.clone()),
            counts.clone(),
            protected.then(|| "Protected".to_owned()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
        let mut card = div()
            .id(format!("proposal-{op}"))
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .id(format!("proposal-heading-{op}"))
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_x_2()
                    .gap_y_1()
                    .aria_label(announced)
                    .child(
                        div()
                            .min_w_0()
                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                            .child(label),
                    )
                    .children(counts.map(|counts| {
                        div()
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .child(counts)
                    }))
                    .when(protected, |heading| {
                        heading.child(components::status_chip(
                            format!("proposal-protected-{op}"),
                            Some(Tone::Info),
                            Some(IconName::Lock),
                            "Protected",
                        ))
                    })
                    .test_support(),
            );
        for write in &candidate.request.writes {
            if let RecordData::Action(action) = &write.data {
                let facts = brn_threads_app::action_review_text(action);
                card = card.child(
                    div()
                        .id(format!("proposal-action-{op}-{}", write.id))
                        .aria_label(facts.clone())
                        .child(facts)
                        .test_support(),
                );
            }
        }
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
                    .text_size(UI_TEXT)
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
                .child({
                    let control = button(format!("card-apply-{op}"))
                        .label(if protected { "Approve" } else { "Apply" })
                        .compact()
                        .h(components::tokens::COMPACT_CONTROL)
                        .tooltip(if protected {
                            "Approving saves exactly this proposal, including a protected note"
                        } else {
                            "Applying saves exactly this proposal"
                        })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.card_apply(apply.clone(), window, cx)
                        }));
                    if lead {
                        control.primary()
                    } else {
                        control.outline()
                    }
                }),
        )
        .test_support()
        .into_any_element()
    }
    /// What each Note toolbar arrangement needs for the open note: each
    /// label's measured text plus its control's fixed chrome, as measured on
    /// the toolbar (Write 16 and Read 17 pt, Format ▾ and Outline 41, the
    /// status chip 38, the comment toggle 40 with a count or 24 without,
    /// Edit note and Connections 17), 50 pt of gaps, padding, spacer and
    /// borders, and 8 pt of slack. The full row's five 24 pt formatting
    /// buttons and their gaps (136 pt) replace Format ▾.
    pub(super) fn linked_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let mut context = scroll("thread-linked-work").gap_0().px_3().pb_3();
        let Some(id) = self.chat_thread() else {
            return context
                .child(components::empty_state(
                    "linked-empty",
                    IconName::Link,
                    "No chat open",
                    "Notes, sources and tasks used in a chat appear here.",
                    None,
                ))
                .test_support()
                .into_any_element();
        };
        let linked_ids = linked(&self.records, id);
        if linked_ids.is_empty() {
            context = context.child(components::empty_state(
                "linked-empty",
                IconName::Link,
                "Nothing linked yet",
                "Notes, sources and tasks used in this chat appear here.",
                None,
            ));
        }
        // Entries are separated by hairlines, never boxed.
        let entry = || {
            div()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .gap_1()
                .py_3()
                .min_w_0()
                .border_b_1()
                .border_color(rgb(p.hairline))
        };
        for record in self
            .records
            .iter()
            .filter(|r| !r.archived && linked_ids.contains(&r.id))
        {
            match &record.data {
                RecordData::Note(note) => {
                    let id = record.id.clone();
                    context = context.child(
                        entry()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .truncate()
                                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                            .child(note.title.clone()),
                                    )
                                    .when(note.protected, |line| {
                                        line.child(components::status_chip(
                                            format!("linked-note-protected-{id}"),
                                            Some(Tone::Info),
                                            Some(IconName::Lock),
                                            "Protected",
                                        ))
                                    })
                                    .child(components::status_chip(
                                        format!("linked-note-version-{id}"),
                                        None,
                                        None,
                                        format!("v{}", record.version),
                                    )),
                            )
                            .child(
                                self.safe_message(
                                    format!("linked-note-preview-{id}"),
                                    note.markdown.clone(),
                                )
                                .max_lines(8),
                            )
                            .child(div().child(
                                pill(format!("linked-note-{id}"), "Open note").on_click(
                                    cx.listener(move |this, _, window, cx| {
                                        this.open_note(&id, None, window, cx)
                                    }),
                                ),
                            )),
                    );
                }
                RecordData::Action(_) => context = context.child(self.action_row(record, true, cx)),
                RecordData::Source(source) => {
                    let locator = source.locator.clone();
                    let open_locator = locator.clone();
                    context = context.child(
                        entry()
                            .child(metadata("Source"))
                            .child(
                                div()
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .child(source.label.clone()),
                            )
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
                                        pill(format!("open-source-{}", record.id), "Open")
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
                                            "Copy reference",
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
                    let mut passage = entry()
                        .child(metadata("Passage"))
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
                            div().child(
                                pill(
                                    format!("comment-note-{}", record.id),
                                    format!("Open {}", short(&title(note))),
                                )
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.open_note(&id, None, window, cx)
                                    },
                                )),
                            ),
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

/// A recovery card's tone; neutral has none.
fn severity_tone(severity: Severity) -> Option<Tone> {
    match severity {
        Severity::Neutral => None,
        Severity::Warning => Some(Tone::Warning),
        Severity::Error => Some(Tone::Error),
    }
}
