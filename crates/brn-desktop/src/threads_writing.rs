//! Writing and conversation tiles: chat, note, in-place review, compare and
//! linked work. Every action calls the same checked services as before.
use super::shell::{item, pill};
use super::state::ReviewSummary;
use super::tiles::scroll;
use super::workbench::{RunOutcome, WritingRow};
use super::*;
use gpui_kit::SharedString;
use gpui_kit::StyledText;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::{DropdownMenu, PopupMenu};
use gpui_kit::component::{Icon, Sizable, Size};

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
    pub(super) fn chat_controls(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let this = cx.entity().downgrade();
        let mut controls = vec![];
        if let ChatTarget::Thread(id) = &self.chat {
            let id = id.clone();
            // An archived chat's only Unarchive is in the composer banner.
            controls.push(if self.thread_resolved(&id) {
                components::status_chip("chat-archived", None, Some(IconName::Archive), "Archived")
                    .into_any_element()
            } else {
                const ARCHIVE: &str = "Archive this chat. Stops work and pauses the queue.";
                pill("resolve-thread", "Archive")
                    .accessibility_label(ARCHIVE)
                    .tooltip(ARCHIVE)
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
            components::icon_button("thread-menu", IconName::ChevronDown, "Chats for this note")
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
                        "New chat for this note",
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
                        |view, window, cx| view.show_tile(TileKind::AllThreads, window, cx),
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
                // A new chat's empty state: what it is, and what the
                // composer below can do.
                let (title, body) = match target {
                    ChatTarget::NewForNote(note) => (
                        format!(
                            "New chat about {}",
                            self.note_rows
                                .iter()
                                .find(|row| &row.id == note)
                                .map_or_else(|| note.clone(), |row| short(&row.title))
                        ),
                        "It stays linked to the note.".to_owned(),
                    ),
                    _ => (
                        "New chat".to_owned(),
                        "Ask below. Attach files for this chat, or let BRN use your saved notes."
                            .to_owned(),
                    ),
                };
                let mut start =
                    scroll("thread-conversation")
                        .gap_3()
                        .p_4()
                        .child(components::empty_state(
                            "chat-start",
                            IconName::MessageSquare,
                            title,
                            body,
                            None,
                        ));
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
                if self
                    .worker
                    .as_ref()
                    .and_then(|w| w.run_id.as_ref())
                    .and_then(|id| self.workspace.store.record(id).ok().flatten())
                    .and_then(|r| match r.data {
                        RecordData::Run(run) => run.instruction,
                        _ => None,
                    })
                    .is_some_and(|id| self.workspace.writing_request(&id).is_ok())
                {
                    None
                } else {
                    Some(self.live_activity(true, cx))
                }
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
            .test_support()
            .into_any_element()
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
        let path = continue_path(&queue, &messages, id);
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
        let mut conversation = transcript_column().gap_3();
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
        if recovering {
            conversation = conversation.child(self.recovery_card(id, &queue, &path, cx));
        }
        scroll("thread-conversation")
            .py_4()
            .px_0()
            .when(self.transcript_centered(), |detail| detail.items_center())
            .child(conversation)
            .test_support()
            .into_any_element()
    }
    /// The transcript centers in a maximized chat (and Solo), otherwise it
    /// sits left like the note.
    fn transcript_centered(&self) -> bool {
        self.layout.maximized() == Some(TileKind::Chat)
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
            && can_continue_thread(
                &self.records,
                &self.history,
                thread,
                state,
                self.worker.is_some(),
            )
    }
    /// The chat's latest run when it is saved as Working and this window
    /// does not hold it: it may still run in another window or the CLI, so
    /// the recovery card offers Stop….
    fn saved_run_to_stop(&self, thread: &str) -> Option<String> {
        let latest = latest_run(&self.records, &self.history, thread)?;
        let local = self.worker.as_ref().and_then(|w| w.run_id.as_deref());
        (matches!(&latest.data, RecordData::Run(run) if run.state == RunState::Working)
            && local != Some(latest.id.as_str()))
        .then(|| latest.id.clone())
    }
    /// The run whose Stop… the visible Chat tile's recovery card renders, so
    /// the Note bar leaves that one control to the card (IDs stay unique).
    fn chat_card_stop(&self) -> Option<String> {
        if !self
            .layout
            .visible()
            .iter()
            .any(|tile| tile.kind == TileKind::Chat)
        {
            return None;
        }
        let thread = self.chat_thread()?;
        let RecordData::Thread(data) = &self.records.iter().find(|r| r.id == thread)?.data else {
            return None;
        };
        let messages = self.workspace.messages(thread).unwrap_or_default();
        let path = continue_path(&self.queue_state(), &messages, thread);
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
        let path = continue_path(&self.queue_state(), &messages, thread);
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
        let latest = latest_run(&self.records, &self.history, thread);
        let run = latest.and_then(|record| match &record.data {
            RecordData::Run(run) => Some(run),
            _ => None,
        });
        let owner_after = owner_message_after_latest_run(&self.records, &self.history, thread);
        let mut card = recovery(
            path,
            self.pending_run_working(queue),
            run,
            owner_after,
            queue,
            thread,
        );
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
    fn composer_tile(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let this = cx.entity().downgrade();
        let busy = self.worker.is_some();
        let archived = self.chat_thread().filter(|id| self.thread_resolved(id));
        let quiet = self.queue_state().quiet;
        let model_this = this.clone();
        let effort_this = this.clone();
        let add_this = this.clone();
        let quiet_control = button("chat-quiet")
                .ghost()
                .icon(if quiet { gpui_kit::component::IconName::EyeOff } else { gpui_kit::component::IconName::Eye })
                .w(px(28.)).h(px(28.))
                .accessibility_label(if quiet { "Quiet on · show activity" } else { "Quiet off · hide activity" })
                .toggled(quiet)
                .selected(quiet)
                .tooltip(if quiet {
                    "Quiet is on · click to show activity. Answers, Stop, errors and decisions stay visible."
                } else {
                    "Quiet is off · click to hide routine activity. Answers, Stop, errors and decisions stay visible."
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    let quiet = !this.queue_state().quiet;
                    if let Err(e) = this
                        .workspace
                        .choose_quiet(&format!("quiet:{}", uuid::Uuid::new_v4()), quiet)
                    {
                        this.fail(e.to_string());
                    }
                    cx.notify();
                }));
        let can_steer = self.can_stop_and_send();
        // Below 420 pt the model control also names the effort (the model
        // name truncates first) and its menu holds the effort choices.
        let narrow = self.chat_narrow;
        let model = self.settings.model.clone();
        let effort = self.settings.effort.clone();
        let model_control = if narrow {
            button("model-pill")
                .compact()
                .ghost()
                .h(components::tokens::COMPACT_CONTROL)
                .flex_shrink(1.)
                .min_w_0()
                .max_w(px(160.))
                .dropdown_caret(true)
                .accessibility_label(format!("Model {model}, effort {effort}"))
                .tooltip("Provider, model and reasoning effort · changes Settings · no fallback")
                .child(
                    div()
                        .flex()
                        .min_w_0()
                        .child(div().min_w_0().truncate().child(model))
                        .child(
                            div()
                                .flex_shrink_0()
                                .whitespace_nowrap()
                                .child(format!("\u{a0}· {effort}")),
                        ),
                )
                .dropdown_menu(move |menu, _, cx| match model_this.upgrade() {
                    Some(view) => {
                        let view = view.read(cx);
                        view.effort_section(
                            view.model_menu(menu, &model_this).separator(),
                            &model_this,
                        )
                    }
                    None => menu,
                })
        } else {
            pill(
                "model-pill",
                super::shell::model_display(&self.settings.provider, &model),
            )
            .dropdown_caret(true)
            .tooltip("Provider and model · changes Settings · no fallback")
            .dropdown_menu(move |menu, _, cx| match model_this.upgrade() {
                Some(view) => view.read(cx).model_menu(menu, &model_this),
                None => menu,
            })
        };
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
            .px_2()
            .py_1()
            .bg(rgb(p.alt))
            .border_t_1()
            .border_color(rgb(p.hairline))
            // The archived chat's one banner holds its only Unarchive.
            .when_some(archived, |composer, chat| {
                let chat = chat.to_owned();
                composer.child(
                    div()
                        .id("composer-destination")
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .rounded(components::tokens::RADIUS_CONTROL)
                        .border_1()
                        .border_color(rgb(p.hairline))
                        .bg(rgb(p.tile))
                        .font_family(UI_FONT)
                        .text_size(UI_TEXT)
                        .text_color(rgb(p.ink))
                        .aria_label("Archived. Unarchive to reply.")
                        .child(
                            Icon::new(IconName::Archive)
                                .with_size(Size::Small)
                                .flex_shrink_0()
                                .text_color(rgb(p.muted)),
                        )
                        .child(div().flex_1().min_w_0().child("Archived. Unarchive to reply."))
                        .child(
                            // The archived chat's next action; Send waits.
                            button("reopen-thread")
                                .label("Unarchive")
                                .compact()
                                .primary()
                                .h(components::tokens::COMPACT_CONTROL)
                                .tooltip("Unarchive this chat. The queue stays paused until you resume it.")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.reopen_thread(&chat, cx)
                                })),
                        )
                        .test_support(),
                )
            })
            .child(self.composer_controls(cx))
            .child(
                div().h(px(64.)).child(
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
                        components::icon_button("composer-add", IconName::Plus, "Add information")
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
                    .child(model_control)
                    .when(!narrow, |row| {
                        row.child(
                            pill("effort-pill", self.settings.effort.clone())
                                .dropdown_caret(true)
                                .tooltip("Reasoning effort")
                                .dropdown_menu(move |menu, _, cx| match effort_this.upgrade() {
                                    Some(view) => view.read(cx).effort_menu(menu, &effort_this),
                                    None => menu,
                                }),
                        )
                    })
                    .child(quiet_control)
                    .when(can_steer, |row| {
                        row.child(
                            pill("chat-steer", "Steer")
                                .tooltip("Stop the current reply and start fresh with this message.")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.stop_and_send(window, cx)
                                })),
                        )
                    })
                    .child({
                        // While work runs ↵ queues, so Send reads Queue. In an
                        // archived chat Unarchive is the filled control, and
                        // beside a recovery card its Continue or Resume queue
                        // is: one filled control per panel.
                        let send = button("ask-delegate");
                        let send = if archived.is_some() || self.recovery_card_leads() {
                            send.outline()
                        } else {
                            send.primary()
                        };
                        send.label(if self.editing_queue.is_some() {
                            "Save queued"
                        } else if busy {
                            "Queue"
                        } else {
                            "Send"
                        })
                        .compact()
                        .h(components::tokens::COMPACT_CONTROL)
                        .tooltip(if self.editing_queue.is_some() {
                            "Save the queued message ↵ · Shift-↵ adds a line"
                        } else if busy {
                            "Add this message to the queue ↵"
                        } else {
                            "Send ↵ · Shift-↵ adds a line"
                        })
                        .disabled(false)
                        .on_click(cx.listener(|this, _, window, cx| this.ask(window, cx)))
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
    pub(super) fn note_toolbar_needs(&self, window: &Window) -> layout::NoteNeeds {
        let Some(open) = &self.note else {
            return layout::NoteNeeds::WIDEST;
        };
        let RecordData::Note(note) = &open.record.data else {
            return layout::NoteNeeds::WIDEST;
        };
        let text = |label: &str, size| text_width(window, label, size);
        let kind = if note.protected {
            "Protected"
        } else {
            "Working"
        };
        let chip = text(&format!("{kind} · v{}", open.record.version), SMALL_TEXT) + 38.;
        let pending = self.pending_comments(&open.record.id);
        let toggle = if pending > 0 {
            text(&pending.to_string(), UI_TEXT) + 40.
        } else {
            24.
        };
        let format = text("Format", UI_TEXT) + 41.;
        let edit = text("Edit note", UI_TEXT) + 17.;
        let connections = text("Connections", UI_TEXT) + 17.;
        let format_menu = 50.
            + 8.
            + text("Write", UI_TEXT)
            + 16.
            + text("Read", UI_TEXT)
            + 17.
            + format
            + text("Outline", UI_TEXT)
            + 41.
            + chip
            + toggle
            + edit
            + connections;
        layout::NoteNeeds {
            full: format_menu - format + 136.,
            format_menu,
            compact: format_menu - edit - connections - 8.,
        }
    }
    /// The Note toolbar, one row: Write | Read, formatting and Outline on the
    /// left; the status chip, comments, Edit note and Connections on the
    /// right. `note_fit` folds controls into menus as the panel narrows.
    pub(super) fn note_controls(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let Some(open) = &self.note else {
            return vec![];
        };
        let RecordData::Note(note) = &open.record.data else {
            return vec![];
        };
        // During review the review bar replaces the toolbar; its Proposal
        // label and Protected chip state the review.
        if self.review.is_some() {
            return vec![];
        }
        let fit = self.note_fit;
        let mut controls = self.document_controls(fit, cx);
        controls.push(div().flex_1().min_w(px(4.)).into_any_element());
        // Protection and version stay visible in every state, dirty included.
        let (tone, icon, kind) = if note.protected {
            (Some(Tone::Info), IconName::Lock, "Protected")
        } else {
            (None, IconName::FileText, "Working")
        };
        let version = format!("v{}", open.record.version);
        let status = format!("{kind} · {version}");
        controls.push(
            if fit.icons() {
                components::short_status_chip("note-status-chip", tone, Some(icon), version, status)
            } else {
                components::status_chip("note-status-chip", tone, Some(icon), status)
            }
            .into_any_element(),
        );
        controls.extend(self.comments_toggle(cx));
        if !fit.actions_in_menu() {
            let blocked = self.edit_note_blocked();
            controls.push(
                pill("edit-note-context", "Edit note")
                    .disabled(blocked.is_some())
                    .accessibility_label(match blocked {
                        Some(reason) => format!("Edit note, unavailable: {reason}"),
                        None => "Edit note".into(),
                    })
                    .tooltip(match blocked {
                        Some(reason) => format!("Edit note is unavailable: {reason}"),
                        None => "Give BRN an instruction for this saved note".into(),
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.edit_note_dialog(window, cx)))
                    .into_any_element(),
            );
            controls.push(
                pill("note-connections", "Connections")
                    .tooltip("Linked notes and sources of this note")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_tile(TileKind::Connections, window, cx)
                    }))
                    .into_any_element(),
            );
        }
        controls
    }
    /// The Note ⋯'s own items: the review's during review, otherwise the
    /// open note's. The panel header renders the ⋯ (`note-menu`) and adds its
    /// Panel section.
    pub(super) fn note_menu_section(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<Box<dyn Fn(PopupMenu) -> PopupMenu + 'static>> {
        if let Some(review) = &self.review {
            return Some(Box::new(self.review_menu_section(review, cx)));
        }
        let open = self.note.as_ref()?;
        let RecordData::Note(note) = &open.record.data else {
            return None;
        };
        let this = cx.entity().downgrade();
        let dirty = open.session.is_some();
        let busy = self.worker.is_some();
        let imported = note.import.is_some();
        let protected = note.protected;
        // A narrow Note keeps Edit note and Connections here instead.
        let collapsed = self.note_fit.actions_in_menu();
        let edit_blocked = self.edit_note_blocked();
        Some(Box::new(move |menu: PopupMenu| {
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
                ));
            if collapsed {
                menu = menu
                    .item(item(
                        &this,
                        match edit_blocked {
                            Some(reason) => format!("Edit note… · {reason}"),
                            None => "Edit note…".into(),
                        },
                        None,
                        edit_blocked.is_some(),
                        false,
                        |view, window, cx| view.edit_note_dialog(window, cx),
                    ))
                    .item(item(
                        &this,
                        "Connections",
                        None,
                        false,
                        false,
                        |view, window, cx| view.show_tile(TileKind::Connections, window, cx),
                    ));
            }
            let mut menu = menu
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
            // The destructive item comes last and confirms in a dialog.
            menu.separator()
                .item(item(
                    &this,
                    "New chat for this note",
                    None,
                    false,
                    false,
                    |view, window, cx| {
                        view.new_writing_thread(window, cx);
                    },
                ))
                .separator()
                .item(item(
                    &this,
                    "Discard edit…",
                    None,
                    !dirty,
                    false,
                    |view, window, cx| view.confirm_discard(window, cx),
                ))
        }))
    }
    /// The Note ⋯ during review: the side-by-side and editing routes and the
    /// saved inputs, then Keep for later and Dismiss when the review bar is
    /// too narrow to show them.
    fn review_menu_section(
        &self,
        review: &Review,
        cx: &mut Context<Self>,
    ) -> impl Fn(PopupMenu) -> PopupMenu + 'static {
        let this = cx.entity().downgrade();
        let note = review.note.is_some();
        let editing = self.review_editing;
        let inputs = self.review_inputs || !note;
        let collapsed = self.review_fit.actions_in_menu();
        move |menu: PopupMenu| {
            let menu = menu
                .item(item(
                    &this,
                    "Compare side by side",
                    None,
                    !note,
                    false,
                    |view, window, cx| view.show_tile(TileKind::Compare, window, cx),
                ))
                .item(item(
                    &this,
                    "Edit final text",
                    None,
                    !note || editing,
                    false,
                    |view, window, cx| view.start_review_edit(window, cx),
                ))
                // A proposal without a note always shows its inputs.
                .item(item(&this, "Inputs", None, !note, inputs, |view, _, cx| {
                    view.review_inputs = !view.review_inputs;
                    cx.notify();
                }));
            if !collapsed {
                return menu;
            }
            menu.separator()
                .item(item(
                    &this,
                    "Keep for later",
                    None,
                    false,
                    false,
                    |view, window, cx| view.keep_review(window, cx),
                ))
                .item(item(
                    &this,
                    "Dismiss",
                    None,
                    false,
                    false,
                    |view, window, cx| view.dismiss_review(window, cx),
                ))
        }
    }
    /// Discarding closes the edit with no Undo, so it always confirms; ↵ is
    /// inert and only the danger button discards.
    pub(super) fn confirm_discard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(session) = self.note.as_ref().and_then(|open| open.session.clone()) else {
            return;
        };
        let body = format!(
            "Your changes since v{} will be lost. This can't be undone.",
            session.base_version
        );
        let target = cx.entity().downgrade();
        self.open_dialog(window, cx, move |dialog, _, _| {
            let target = target.clone();
            components::consequence_dialog(
                dialog,
                "discard-note",
                "Discard your unsaved edit?",
                body.clone(),
                "Discard",
                std::rc::Rc::new(move |window, cx| {
                    let _ = target.update(cx, |view, cx| view.discard(window, cx));
                }),
            )
        });
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
            return div()
                .id("note-empty-state")
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(components::empty_state(
                    "note-empty",
                    IconName::FileText,
                    "No note open",
                    "Open one from Notes or ⌘K.",
                    Some(
                        button("new-note")
                            .label("New note")
                            .outline()
                            .tooltip("New note (⌘N)")
                            .on_click(cx.listener(|this, _, window, cx| this.new_note(window, cx))),
                    ),
                ))
                .into_any_element();
        };
        let _ = window;
        let dirty = open.session.is_some();
        let blocked = self.recovery_blocked();
        let mut content = div()
            .id("note-tile")
            .size_full()
            .flex()
            .flex_col()
            .min_h_0();
        // Banners sit between the toolbar and the text and never move into a
        // menu; hairlines separate adjacent ones. In a short Note the
        // saved-version preview gives up height first; past that the banners
        // after the first scroll inside their own box, which keeps
        // BANNER_FLOOR, so no banner is painted over the text, the text
        // keeps its minimum and the writing bar stays in the Note.
        let mut banners = self.note_banners(cx).into_iter();
        let banner_shown = banners.len() > 0;
        if let Some(lead) = banners.next() {
            // The first banner (a kept edit, or the stale edit with its
            // Save as new note and Open current version) never shrinks, so
            // its actions stay in view. The rest share a box below it that
            // keeps its first line (the saved version's label, or the next
            // banner's opening line) and scrolls.
            // `note-banners` opens the stack with the lead banner;
            // `note-banners-more` is the shared box below it.
            let rest: Vec<_> = banners.collect();
            let more = !rest.is_empty();
            content = content
                .child(
                    div()
                        .id("note-banners")
                        .flex()
                        .flex_col()
                        .flex_shrink(1.)
                        .min_h(components::BANNER_PINNED_MIN)
                        .bg(rgb(p.hairline))
                        .border_b_1()
                        .border_color(rgb(p.hairline))
                        .child(lead)
                        .test_support(),
                )
                // The saved version and later banners give way first.
                .when(more, |content| {
                    content.child(
                        div()
                            .id("note-banners-more")
                            .flex()
                            .flex_col()
                            .flex_shrink(YIELD_FIRST)
                            .min_h(BANNER_FLOOR)
                            .overflow_y_scroll()
                            .gap(px(1.))
                            .bg(rgb(p.hairline))
                            .border_b_1()
                            .border_color(rgb(p.hairline))
                            .children(rest)
                            .test_support(),
                    )
                });
        }
        if let Some((_, quote)) = &self.selection
            && !blocked
        {
            let mut strip = div()
                .id("selection-toolbar")
                .flex()
                .flex_wrap()
                .flex_shrink_0()
                .items_center()
                .gap_1()
                .min_h(components::tokens::TOOLBAR)
                .px_3()
                .py_1()
                .bg(rgb(p.alt))
                .border_b_1()
                .border_color(rgb(p.hairline))
                .font_family(UI_FONT)
                .text_size(SMALL_TEXT)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(rgb(p.muted))
                        .child(format!("“{}”", short(quote))),
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
                let reason =
                    "Save and comment saves your edit first, then comments on this passage.";
                strip = strip.child(
                    div()
                        .id("selection-comment-reason")
                        .w_full()
                        .text_color(rgb(p.muted))
                        .aria_label(reason)
                        .child(reason)
                        .test_support(),
                );
            }
            content = content.child(strip.test_support());
        }
        let editor = if self.note_reading {
            super::document::readable_document(&self.note_read_view, self.note_assets())
                .into_any_element()
        } else {
            prose_editor(&self.note_editor)
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
                    .menu("Select all", Box::new(gpui_kit::base::input::SelectAll))
                })
                .into_any_element()
        };
        let centered = self.layout.maximized() == Some(TileKind::Note);
        // The text keeps a few lines however many banners a short Note shows,
        // and 120 px beside an open comments section. The section asks for
        // its floor after the reply below gives way, and gives way first,
        // before the text. Banners are never hidden to meet that 120 px:
        // with a banner shown the text still asks for 120 px and gives way
        // last, below it only when the rows that never hide (the banners'
        // actions, a failed run's reason, the reply's label and the whole
        // writing bar) leave less, down to its few lines.
        let stacked = self.comments_stacked();
        let text_floor = text_min(stacked && !banner_shown);
        let text_prefer = text_min(stacked);
        // A shrink factor of 1 beside the section's YIELD_FIRST: the
        // section gives way first, and once it is at its floor the text
        // takes up the whole remaining overflow (a factor below 1 would
        // leave part of it painted over the rows below).
        let mut body = div()
            .flex()
            .flex_grow(1.)
            .flex_basis(if stacked { text_prefer } else { px(0.) })
            .flex_shrink(1.)
            .min_h(text_floor)
            .child(
                div()
                    .id("note-body")
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .when(centered, |body| body.justify_center())
                    .child(
                        reading_column()
                            .id("note-column")
                            .child(editor)
                            .test_support(),
                    )
                    .on_mouse_up(
                        gpui_kit::MouseButton::Left,
                        cx.listener(|this, _, _, cx| this.track_selection(cx)),
                    )
                    .on_key_up(cx.listener(|this, _, _, cx| this.track_selection(cx)))
                    .test_support(),
            );
        if self.comments_beside() {
            body = body.child(self.margin_column(cx));
        }
        // Below the text: comments, then the run's status and reply, then the
        // writing bar. An open draft keeps its whole input: the area's floor
        // holds the text box plus the draft, and the rows around give way
        // first.
        let (prefer, min) = if stacked {
            let floor = self.stacked_floor();
            let min = if self.comment_drafting() {
                text_floor + floor
            } else {
                text_floor
            };
            (text_prefer + floor, min)
        } else {
            (px(0.), text_floor)
        };
        content
            .child(self.text_and_comments(body, prefer, min, stacked, cx))
            .child(self.writing_activity(cx))
            .child(self.writing_run_status())
            .child(self.writing_proposals(cx))
            .child(self.writing_result())
            .children(self.note_bar(cx))
            .test_support()
            .into_any_element()
    }
    /// The Note's banners, top to bottom: a kept edit, the edit's stale base
    /// (with the saved version) or a newer saved version, and the import
    /// summary with its coverage details.
    fn note_banners(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let p = pal();
        let mut banners: Vec<gpui_kit::AnyElement> = vec![];
        // The first banner leads the stack: its actions never leave view.
        let banner = |lead: bool,
                      id: &'static str,
                      tone: Tone,
                      text: SharedString,
                      actions: Vec<gpui_kit::AnyElement>|
         -> gpui_kit::AnyElement {
            if lead {
                components::banner_pinned(id, tone, text, actions).into_any_element()
            } else {
                components::banner(id, tone, text, actions, None).into_any_element()
            }
        };
        let Some(open) = &self.note else {
            return banners;
        };
        let RecordData::Note(note) = &open.record.data else {
            return banners;
        };
        if self.recovery_blocked()
            && let Some(buffer) = self.buffers.iter().find(|s| s.note == open.record.id)
        {
            let (buffer, id) = (buffer.clone(), buffer.note.clone());
            banners.push(banner(
                banners.is_empty(),
                "note-recover-banner",
                Tone::Warning,
                "An unsaved edit was kept. This note is read-only until you recover it.".into(),
                // The kept edit blocks the note, so recovering it is the
                // panel's one filled primary.
                vec![
                    button("recover-note-edit")
                        .label("Recover edit")
                        .compact()
                        .primary()
                        .h(components::tokens::COMPACT_CONTROL)
                        .tooltip("Reopen the kept edit so you can save or discard it")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_note(&id, Some(buffer.clone()), window, cx)
                        }))
                        .into_any_element(),
                ],
            ));
        }
        let id = open.record.id.clone();
        let current = move |cx: &mut Context<Self>| {
            let id = id.clone();
            banner_button("show-current-note", "Open current version")
                .on_click(
                    cx.listener(move |this, _, window, cx| this.open_note(&id, None, window, cx)),
                )
                .into_any_element()
        };
        // An open edit usually makes both version facts true; the stale-edit
        // banner states both, so Open current version renders once.
        if self.edit_is_stale() {
            banners.push(banner(
                banners.is_empty(),
                "note-stale-banner",
                Tone::Warning,
                "This note changed since your edit began. Your text is kept. Compare it with the saved version below, or save your text as a new note.".into(),
                vec![
                    button("save-buffer-copy")
                        .label("Save as new note")
                        .compact()
                        .primary()
                        .h(components::tokens::COMPACT_CONTROL)
                        .tooltip("Save your text as a separate working note")
                        .on_click(cx.listener(|this, _, window, cx| this.save_copy(window, cx)))
                        .into_any_element(),
                    current(cx),
                ],
            ));
            banners.push(
                div()
                    .id("note-stale-preview")
                    .flex()
                    .flex_col()
                    .h(px(204.))
                    .min_h(px(84.))
                    .flex_shrink(1.)
                    .bg(rgb(p.alt))
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .font_family(UI_FONT)
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .border_b_1()
                            .border_color(rgb(p.hairline))
                            .child("Saved version"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .px_3()
                            .child(markdown(&self.current_view, self.note_assets())),
                    )
                    .test_support()
                    .into_any_element(),
            );
        } else if self
            .records
            .iter()
            .any(|r| r.id == open.record.id && r.version != open.record.version)
        {
            banners.push(banner(
                banners.is_empty(),
                "note-newer-banner",
                Tone::Warning,
                "A newer version of this note is saved. Your visible text is kept.".into(),
                vec![current(cx)],
            ));
        }
        if let Some(coverage) = &note.import {
            let intent = if coverage.full_note {
                "Full imported reference"
            } else {
                "Useful information"
            };
            // Info only when the report records full coverage.
            let (summary, details, checked) = if let Some(report) = &coverage.report {
                (
                    report.summary(),
                    report
                        .issues
                        .iter()
                        .map(ImportIssue::display)
                        .collect::<Vec<_>>(),
                    report.status == ImportStatus::Complete
                        && report.issues.iter().all(|issue| {
                            !matches!(
                                issue.kind,
                                ImportIssueKind::Omission | ImportIssueKind::Unverified
                            )
                        }),
                )
            } else {
                (
                    if coverage.gaps.is_empty() {
                        "Coverage not recorded"
                    } else {
                        "Partial / coverage requires inspection"
                    },
                    coverage.gaps.clone(),
                    false,
                )
            };
            let mut actions = vec![];
            if let Some(record) = self.records.iter().find(|r| r.id == coverage.source)
                && let RecordData::Source(source) = &record.data
            {
                let locator = source.locator.clone();
                actions.push(
                    banner_button("open-import-original", "Open original")
                        .on_click(move |_, _, cx| {
                            if std::path::Path::new(&locator).is_absolute() {
                                cx.open_with_system(std::path::Path::new(&locator));
                            } else if locator.starts_with("https://")
                                || locator.starts_with("http://")
                            {
                                cx.open_url(&locator);
                            }
                        })
                        .into_any_element(),
                );
            }
            if !details.is_empty() {
                actions.push(
                    pill(
                        "toggle-coverage",
                        format!("Coverage details ({})", details.len()),
                    )
                    .selected(self.show_coverage)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.show_coverage = !this.show_coverage;
                        cx.notify();
                    }))
                    .into_any_element(),
                );
            }
            banners.push(banner(
                banners.is_empty(),
                "note-import-summary",
                if checked { Tone::Info } else { Tone::Warning },
                format!("{intent} · {summary}").into(),
                actions,
            ));
            if self.show_coverage && !details.is_empty() {
                // Provenance may contain long paths and many qualifications.
                // Keep every issue scrollable without consuming the note body.
                banners.push(
                    div()
                        .id("note-coverage-details")
                        .w_full()
                        .max_h(px(120.))
                        .flex_shrink_0()
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .bg(rgb(p.alt))
                        .px_3()
                        .py_2()
                        .font_family(UI_FONT)
                        .child(
                            div()
                                .text_size(SMALL_TEXT)
                                .text_color(rgb(p.muted))
                                .child(format!("Coverage details ({})", details.len())),
                        )
                        .children(details.into_iter().enumerate().map(|(index, detail)| {
                            div()
                                .id(format!("note-coverage-detail-{index}"))
                                .flex_shrink_0()
                                .text_size(UI_TEXT)
                                .child(detail)
                                .test_support()
                        }))
                        .test_support()
                        .into_any_element(),
                );
            }
        }
        banners
    }
    /// The note whose comments the Note shows: the open note, unless a
    /// review shows another note's text.
    fn comments_note(&self) -> Option<&OpenNote> {
        let open = self.note.as_ref()?;
        match &self.review {
            Some(review) if review.note.as_deref() != Some(open.record.id.as_str()) => None,
            _ => Some(open),
        }
    }
    /// A draft, a comment or a load error to show for that note.
    fn comments_have_content(&self) -> bool {
        self.comments_note().is_some_and(|open| {
            self.comment_anchor
                .as_ref()
                .is_some_and(|(note, ..)| note == &open.record.id)
                || self
                    .workspace
                    .writing_comments(&open.record.id)
                    .map_or(true, |comments| !comments.is_empty())
        })
    }
    /// Whether the comments are open where they sit now. Beside the text
    /// they follow the note until the toggle decides; the stacked section
    /// stays collapsed until opened.
    pub(super) fn comments_open(&self) -> bool {
        if self.margin_mode == layout::MarginMode::Stacked {
            self.comments_expanded
        } else {
            self.margin.unwrap_or_else(|| self.comments_have_content())
        }
    }
    /// The margin renders beside the text.
    pub(super) fn comments_beside(&self) -> bool {
        self.margin_mode != layout::MarginMode::Stacked
            && self.comments_note().is_some()
            && self.comments_open()
    }
    /// The in-tile section renders below the text.
    pub(super) fn comments_stacked(&self) -> bool {
        self.margin_mode == layout::MarginMode::Stacked
            && self.comments_note().is_some()
            && self.comments_expanded
    }
    /// The open stacked section's floor: the label and a card's first
    /// line, or the whole draft while one is open.
    fn stacked_floor(&self) -> gpui_kit::Pixels {
        if self.comment_drafting() {
            STACKED_DRAFT_FLOOR
        } else {
            COMMENTS_FLOOR
        }
    }
    /// `toggle-margin`'s one handler, in the toolbar and in the review bar.
    pub(super) fn toggle_comments(&mut self, cx: &mut Context<Self>) {
        let open = !self.comments_open();
        if self.margin_mode == layout::MarginMode::Stacked {
            self.comments_expanded = open;
        } else {
            self.margin = Some(open);
        }
        cx.notify();
    }
    /// Escape inside the stacked section collapses it, keeps any draft and
    /// returns focus to the text above.
    fn collapse_comments(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.comments_expanded = false;
        if !self.review_editing
            && self
                .review
                .as_ref()
                .is_some_and(|review| review.note.is_some())
        {
            self.tile_reveal = None;
            self.focus_root(window, cx);
            if self.review_reading {
                self.candidate_view
                    .read(cx)
                    .focus_handle()
                    .clone()
                    .focus(window, cx);
            } else {
                self.diff_editor
                    .update(cx, |editor, cx| editor.focus(window, cx));
            }
        } else {
            self.focus_visible_editor(window, cx);
        }
        cx.notify();
    }
    /// The comment toggle with its open count: the same control, handler and
    /// state in the Note toolbar and the review bar, which never render
    /// together.
    fn comments_toggle(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let open = self.comments_note()?;
        let pending = self.pending_comments(&open.record.id);
        let label = if pending > 0 {
            format!("Comments · {pending}")
        } else {
            "Comments".into()
        };
        let shown = self.comments_open();
        let reviewing = self.review.is_some();
        Some(
            if pending > 0 {
                pill("toggle-margin", pending.to_string())
                    .icon(IconName::MessageSquare)
                    .accessibility_label(label)
            } else {
                components::icon_button("toggle-margin", IconName::MessageSquare, label)
            }
            .selected(shown)
            .tooltip(match (shown, reviewing) {
                (true, true) => "Hide comments",
                (false, true) => "Show comments",
                (true, false) => "Hide comments · select text to comment (⌘/)",
                (false, false) => "Show comments · select text to comment (⌘/)",
            })
            .on_click(cx.listener(|this, _, _, cx| this.toggle_comments(cx)))
            .into_any_element(),
        )
    }
    fn pending_comments(&self, note: &str) -> usize {
        self.workspace
            .writing_comments(note)
            .map(|comments| comments.iter().filter(|entry| !entry.addressed).count())
            .unwrap_or_default()
    }
    /// "Comments · N", 12 px, above the list in both placements.
    fn comments_label(&self, note: &str) -> gpui_kit::AnyElement {
        let pending = self.pending_comments(note);
        div()
            .flex_shrink_0()
            .font_family(UI_FONT)
            .text_size(SMALL_TEXT)
            .font_weight(gpui_kit::FontWeight::MEDIUM)
            .text_color(rgb(pal().muted))
            .child(if pending > 0 {
                format!("Comments · {pending}")
            } else {
                "Comments".into()
            })
            .into_any_element()
    }
    /// Text above, comments below. The area asks for `prefer` (the text's
    /// 120 px plus the open section's floor) and gives way only after the
    /// rows around it reach their minimums (the reply in Write, the details
    /// in review), down to `min`. Inside, the section (at most 40 % of the
    /// area, clipped) shrinks before the text.
    fn text_and_comments(
        &self,
        text: impl IntoElement,
        prefer: gpui_kit::Pixels,
        min: gpui_kit::Pixels,
        stacked: bool,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        div()
            .id("text-and-comments")
            .flex()
            .flex_col()
            .flex_grow(1.)
            .flex_basis(prefer)
            .flex_shrink(YIELD_LAST)
            .min_h(min)
            .child(text)
            .when(stacked, |area| area.child(self.comments_section(cx)))
            .test_support()
            .into_any_element()
    }
    /// Wide and Narrow: the margin beside the text, at the mode's width.
    fn margin_column(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(open) = self.comments_note() else {
            return div().into_any_element();
        };
        div()
            .id("writing-feedback")
            .w(px(self.margin_mode.width()))
            .flex_shrink_0()
            .h_full()
            .overflow_y_scroll()
            .track_scroll(&self.comments_scroll)
            .flex()
            .flex_col()
            .gap_2()
            .px_2()
            .py_2()
            .border_l_1()
            .border_color(rgb(pal().hairline))
            .child(self.comments_label(&open.record.id))
            .children(self.comment_items(true, cx))
            .test_support()
            .into_any_element()
    }
    /// Stacked: the docked section below the text, with the same list,
    /// cards, draft and IDs as the margin. Escape inside collapses it.
    fn comments_section(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let Some(open) = self.comments_note() else {
            return div().into_any_element();
        };
        // Clipped, so a squeezed section reads as its one-line label row
        // and never paints over the rows below. An open draft sits above
        // the list and keeps the section at least as tall as itself, so
        // its input is never cut off; the cards scroll below it.
        let drafting = self.comment_drafting();
        // Beside the text's 120 px ask the section gives way first.
        div()
            .id("comments-section")
            .flex()
            .flex_col()
            .flex_shrink(YIELD_FIRST)
            .min_h(if drafting {
                STACKED_DRAFT_FLOOR
            } else {
                px(0.)
            })
            .max_h(gpui_kit::relative(0.4))
            .overflow_hidden()
            .border_t_1()
            .border_color(rgb(pal().hairline))
            .on_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
                    if event.keystroke.key == "escape" {
                        this.collapse_comments(window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .child(
                div()
                    .id("comments-section-label")
                    .flex_shrink_0()
                    .px_3()
                    .py_1()
                    .child(self.comments_label(&open.record.id))
                    .test_support(),
            )
            .children(
                self.comment_draft(cx)
                    .map(|draft| div().flex_shrink_0().px_3().pb_2().child(draft)),
            )
            .child(
                div()
                    .id("writing-feedback")
                    .flex()
                    .flex_col()
                    .gap_2()
                    .flex_shrink(1.)
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.comments_scroll)
                    .px_3()
                    .pb_2()
                    .children(self.comment_items(false, cx))
                    .test_support(),
            )
            .test_support()
            .into_any_element()
    }
    /// The draft, a load error or the empty hint, then one card per comment.
    /// Only one placement renders at a time, so the IDs stay unique. The
    /// stacked section renders the draft itself, above its scroller.
    fn comment_items(&self, with_draft: bool, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let mut items = vec![];
        let Some(open) = self.comments_note() else {
            return items;
        };
        let note_id = open.record.id.clone();
        let drafting = self.comment_drafting();
        if with_draft {
            items.extend(self.comment_draft(cx));
        }
        let (comments, feedback_error) = match self.workspace.writing_comments(&note_id) {
            Ok(comments) => (comments, None),
            Err(error) => (
                vec![],
                Some(format!(
                    "Couldn't load comments: {error}. Reload saved note to try again."
                )),
            ),
        };
        self.comment_cards(items, comments, feedback_error, drafting, cx)
    }
    /// The passage draft: its passage, Cancel and Add comment, then the
    /// input. The stacked section keeps all of it in view.
    fn comment_draft(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let p = pal();
        let open = self.comments_note()?;
        let dirty = open.session.is_some();
        let (_, _, _, quote) = self
            .comment_anchor
            .as_ref()
            .filter(|(anchor_note, ..)| anchor_note == &open.record.id)?;
        Some(
            div()
                .id("comment-composer")
                .flex()
                .flex_col()
                .flex_shrink_0()
                .gap_1()
                .p_2()
                .rounded_md()
                .border_l_2()
                .border_color(rgb(p.accent))
                .bg(rgb(p.tile))
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(80.))
                                .truncate()
                                .text_size(SMALL_TEXT)
                                .text_color(rgb(p.muted))
                                .italic()
                                .child(format!("“{}”", short(quote))),
                        )
                        .child(pill("cancel-comment", "Cancel").on_click(cx.listener(
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
                        .child(
                            // Outline: the panel's one filled primary
                            // stays in the writing or review bar.
                            button("post-comment")
                                .label("Add comment")
                                .compact()
                                .h(components::tokens::COMPACT_CONTROL)
                                .outline()
                                .disabled(dirty)
                                .when(dirty, |post| {
                                    post.accessibility_label(
                                        "Add comment, unavailable: Save your edit first",
                                    )
                                })
                                .tooltip(if dirty {
                                    "Save your edit first; comments anchor to the saved note"
                                } else {
                                    "Add this comment to the note's feedback"
                                })
                                .on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.post_comment(window, cx)
                                    }),
                                ),
                        ),
                )
                .child(
                    div()
                        .id("comment-input-box")
                        .h(px(if self.margin_mode == layout::MarginMode::Stacked {
                            STACKED_DRAFT_INPUT
                        } else {
                            90.
                        }))
                        .child(
                            prose_editor(&self.comment_input)
                                .h_full()
                                .aria_label("Comment"),
                        )
                        .test_support(),
                )
                .test_support()
                .into_any_element(),
        )
    }
    /// A passage draft is open for the note the comments show.
    fn comment_drafting(&self) -> bool {
        self.comments_note().is_some_and(|open| {
            self.comment_anchor
                .as_ref()
                .is_some_and(|(anchor_note, ..)| anchor_note == &open.record.id)
        })
    }
    /// The load error or the empty hint, then one card per comment.
    fn comment_cards(
        &self,
        mut items: Vec<gpui_kit::AnyElement>,
        mut comments: Vec<WritingComment>,
        feedback_error: Option<String>,
        drafting: bool,
        cx: &mut Context<Self>,
    ) -> Vec<gpui_kit::AnyElement> {
        let p = pal();
        let Some(open) = self.comments_note() else {
            return items;
        };
        let RecordData::Note(note) = &open.record.data else {
            return items;
        };
        let note_id = open.record.id.clone();
        let dirty = open.session.is_some();
        let failed = feedback_error.is_some();
        if let Some(error) = feedback_error {
            items.push(
                components::status_line("comments-error", Some(Tone::Error), error)
                    .into_any_element(),
            );
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
        if comments.is_empty() && !drafting && !failed {
            items.push(
                components::empty_state(
                    "margin-empty",
                    IconName::MessageSquare,
                    "No comments yet",
                    "Select a passage and press ⌘/.",
                    None,
                )
                .into_any_element(),
            );
        }
        for entry in &comments {
            let RecordData::Comment(comment) = &entry.record.data else {
                continue;
            };
            let id = entry.record.id.clone();
            let location = comment_location(comment, open.record.version, &note.markdown);
            let checked = !entry.addressed && !self.excluded_comments.contains(&id);
            // An unlocated comment is kept as written; the owner excludes it
            // or comments on the passage again to reattach the feedback.
            let unlocated = unlocated_reason(comment, open.record.version);
            let status = if entry.addressed {
                "Addressed · history kept".to_owned()
            } else if location.is_none() {
                unlocated.status.clone()
            } else if !checked {
                "Excluded from rewrite".into()
            } else {
                String::new()
            };
            let quote_label = format!(
                "“{}{}”",
                comment.quote.chars().take(28).collect::<String>(),
                if comment.quote.chars().count() > 28 {
                    "…"
                } else {
                    ""
                }
            );
            // A disabled Show passage keeps its quote in the announced name,
            // so each comment's control stays distinct, and states why.
            let disabled_names = |why: &str| {
                (
                    components::unavailable(&format!("Show passage {quote_label}"), Some(why)).0,
                    components::unavailable("Show passage", Some(why)).1,
                )
            };
            let quote = if let Some(range) = location.clone() {
                let blocked = if dirty {
                    Some("save your edit first")
                } else if self.review.is_some() {
                    Some("finish the review first")
                } else {
                    None
                };
                let reveal = button(format!("reveal-comment-{id}"))
                    .label(quote_label.clone())
                    .ghost()
                    .compact()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .disabled(blocked.is_some())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.reveal(range.clone(), window, cx)
                    }));
                match blocked {
                    Some(why) => {
                        let (announced, tooltip) = disabled_names(why);
                        reveal.accessibility_label(announced).tooltip(tooltip)
                    }
                    None => reveal.tooltip("Show passage in note"),
                }
            } else {
                let (announced, tooltip) = disabled_names(&unlocated.reason);
                button(format!("comment-quote-{id}"))
                    .label(quote_label.clone())
                    .ghost()
                    .compact()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .disabled(true)
                    .accessibility_label(announced)
                    .tooltip(tooltip)
            };
            let mut actions = div().flex().flex_wrap().gap_1();
            if !entry.addressed {
                let toggle = id.clone();
                actions = actions.child(
                    pill(
                        format!("include-comment-{id}"),
                        if checked { "Included" } else { "Include" },
                    )
                    .when(checked, |toggle| toggle.icon(IconName::Check))
                    .selected(checked)
                    .tooltip(if location.is_none() && !checked {
                        components::unavailable("Include", Some(&unlocated.reason)).1
                    } else if location.is_none() {
                        format!("Exclude this comment; {}", unlocated.reason)
                    } else if checked {
                        "Exclude from the next rewrite".into()
                    } else {
                        "Include in the next rewrite".into()
                    })
                    .when(!checked && location.is_none(), |toggle| {
                        toggle.accessibility_label(
                            components::unavailable("Include", Some(&unlocated.reason)).0,
                        )
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
            if let Some((request, operation, snapshot)) = &applied
                && !entry.addressed
                && snapshot.comments.iter().any(|captured| captured.id == id)
            {
                let (request, operation, record) = (request.clone(), operation.clone(), id.clone());
                actions = actions.child(
                    pill(format!("address-comment-{id}"), "Mark addressed").on_click(cx.listener(
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
            // Body first; the passage and the card's controls share a row
            // that wraps in a narrow margin. Cards keep only the left rule.
            let card = div()
                .id(format!("comment-card-{id}"))
                .flex()
                .flex_col()
                .gap_1()
                .p_2()
                .rounded_md()
                .border_l_2()
                .border_color(rgb(p.border))
                .bg(rgb(if entry.addressed { p.alt } else { p.tile }))
                .child(div().text_size(UI_TEXT).child(comment.body.clone()))
                .when(!status.is_empty(), |card| {
                    card.child(
                        div()
                            .id(format!("comment-status-{id}"))
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(p.muted))
                            .aria_label(status.clone())
                            .child(status)
                            .test_support(),
                    )
                })
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_center()
                        .gap_1()
                        .child(quote)
                        .child(div().flex_1())
                        .child(actions),
                );
            items.push(card.test_support().into_any_element());
        }
        items
    }
    /// The expanded saved inputs, when shown, then the writing bar.
    fn note_bar(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let p = pal();
        let Some(open) = &self.note else {
            return vec![];
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
            .font_family(UI_FONT)
            .text_size(UI_TEXT)
            .bg(rgb(p.alt))
            .border_t_1()
            .border_color(rgb(p.hairline));
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
                    .filter(|snapshot| {
                        snapshot.note.id == note_id && self.latest_rewrite_applied(&note_id)
                    })
                    .map(|snapshot| (request.clone(), operation.clone(), snapshot))
            });
        let selected = self.writing_threads.get(&note_id).cloned();
        let latest_request = self.latest_note_writing_request(&note_id);
        // The expanded saved rewrite inputs sit above the bar, outside it:
        // a short Note scrolls them down to their heading line before the
        // text gives up height or the bar is cut off.
        let mut saved_inputs = None;
        if let Some(record) = &latest_request
            && self.expanded_requests.contains(&record.id)
            && let Ok(snapshot) = self.workspace.writing_request(&record.id)
        {
            let mut disclosure = div()
                .id("writing-requests")
                .flex_shrink(1.)
                .min_h(px(40.))
                .max_h(px(220.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap_1()
                .px_3()
                .py_1()
                .bg(rgb(p.alt))
                .border_t_1()
                .border_color(rgb(p.hairline))
                .font_family(UI_FONT)
                .text_size(PROSE_TEXT)
                .child(subheading("Saved rewrite inputs"))
                .child(format!(
                    "Saved note v{} · {} comments · {} posted chat messages. Later additions stay pending.",
                    snapshot.note.version,
                    snapshot.comments.len(),
                    snapshot.messages.len()
                ));
            if let RecordData::Note(captured) = &snapshot.note.data {
                disclosure = disclosure.child(subheading("Captured saved note")).child(
                    self.safe_message_for_thread(
                        format!("captured-note-{}", record.id),
                        captured.markdown.clone(),
                        Some(&snapshot.thread),
                    ),
                );
            }
            if !snapshot.general_feedback.is_empty() {
                disclosure = disclosure
                    .child(subheading("Overall feedback"))
                    .child(snapshot.general_feedback.clone());
            }
            disclosure = disclosure.child(subheading("Captured chat"));
            for message in &snapshot.messages {
                if let RecordData::Message(body) = &message.data {
                    disclosure = disclosure
                        .child(if body.role == "owner" { "You" } else { "BRN" })
                        .child(self.safe_message_for_thread(
                            format!("captured-chat-{}", message.id),
                            body.text.clone(),
                            Some(&snapshot.thread),
                        ));
                }
            }
            saved_inputs = Some(disclosure.test_support());
        }
        bar = bar.child(self.overall_feedback());
        // The first matching row picks the one filled primary; controls of
        // lower matching rows still render once, outline or ghost.
        let busy = self.worker.is_some();
        let proposals = self
            .candidates
            .iter()
            .any(|candidate| self.writing_candidate_note(candidate).as_deref() == Some(&note_id));
        let Some(row) = self.writing_bar_row() else {
            return vec![];
        };
        let mut info: Vec<gpui_kit::AnyElement> = vec![];
        let mut secondary: Vec<gpui_kit::AnyElement> = vec![];
        let mut primary: Option<gpui_kit::AnyElement> = None;
        let mut reason: Option<&'static str> = None;
        if dirty {
            info.push(
                components::status_chip(
                    "note-unsaved-chip",
                    Some(Tone::Warning),
                    None,
                    "Unsaved edit · recoverable",
                )
                .into_any_element(),
            );
            secondary.push(
                pill("discard-note", "Discard…")
                    .tooltip("Discard your unsaved edit; asks first")
                    .on_click(cx.listener(|this, _, window, cx| this.confirm_discard(window, cx)))
                    .into_any_element(),
            );
            // While the note changed since the edit began, Save as new note
            // is the stale-edit banner's primary and renders only there.
            if row != WritingRow::DirtyStale {
                secondary.push(
                    pill("save-buffer-copy", "Save as new note")
                        .tooltip("Save your text as a separate working note")
                        .on_click(cx.listener(|this, _, window, cx| this.save_copy(window, cx)))
                        .into_any_element(),
                );
            }
            let save = button("save-note")
                .label("Save ⌘S")
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .tooltip("Save this edit (⌘S)")
                .on_click(cx.listener(|this, _, window, cx| this.save(window, cx)));
            if row == WritingRow::Dirty {
                primary = Some(save.primary().into_any_element());
            } else {
                secondary.push(save.ghost().into_any_element());
            }
        }
        if let Some((request, operation, snapshot)) = &applied {
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
            info.push(
                components::status_chip(
                    "applied-state",
                    Some(Tone::Success),
                    Some(IconName::CircleCheck),
                    format!(
                        "Applied · v{}{}",
                        open.record.version,
                        if open_ids.is_empty() && !snapshot.comments.is_empty() {
                            format!(
                                " · {} addressed",
                                plural(snapshot.comments.len(), "comment")
                            )
                        } else {
                            String::new()
                        }
                    ),
                )
                .into_any_element(),
            );
            info.push(
                components::icon_button("clear-applied", IconName::Close, "Hide")
                    .tooltip("Hide; Changes keeps it and its Undo")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.applied_rewrite.remove(&clear);
                        cx.notify();
                    }))
                    .into_any_element(),
            );
            // As on the base, an open edit hides Undo; Changes keeps it.
            if !dirty {
                secondary.push(
                    pill("undo-writing-apply", "Undo")
                        .tooltip("Undo the applied rewrite; Changes keeps it")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.undo_receipt(&undo, window, cx)
                        }))
                        .into_any_element(),
                );
            }
            let again_blocked = if busy {
                Some("BRN is working")
            } else if dirty {
                Some("save your edit first")
            } else {
                None
            };
            secondary.push(
                pill("rewrite-again", "Rewrite again")
                    .disabled(again_blocked.is_some())
                    .when_some(again_blocked, |button, why| {
                        button
                            .accessibility_label(format!("Rewrite again, unavailable: {why}"))
                            .tooltip(format!("Rewrite again is unavailable: {why}"))
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.rewrite_note(cx)))
                    .into_any_element(),
            );
            if !open_ids.is_empty() {
                let mark = button("confirm-addressed")
                    .label(format!(
                        "Mark {} addressed",
                        plural(open_ids.len(), "comment")
                    ))
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.confirm_addressed(&request, &operation_mark, &open_ids, cx)
                    }));
                if row == WritingRow::Applied {
                    primary = Some(mark.primary().into_any_element());
                } else {
                    secondary.push(mark.outline().into_any_element());
                }
            }
        } else {
            let mut included = 0;
            let mut unresolved_included = false;
            if let RecordData::Note(note) = &open.record.data {
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
            let scope = format!(
                "Whole note · {} · {}",
                plural(included, "comment"),
                plural(messages, "saved context message")
            );
            info.push(
                // The scope shares the actions' row and truncates (its full
                // text is in the tooltip) instead of taking a row of its own
                // in a narrow Note.
                div()
                    .id("rewrite-scope")
                    .flex_1()
                    .min_w(px(120.))
                    .truncate()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .aria_label(scope.clone())
                    .tooltip({
                        let scope = SharedString::from(scope.clone());
                        move |window, cx| {
                            gpui_kit::component::tooltip::Tooltip::new(scope.clone())
                                .build(window, cx)
                        }
                    })
                    .child(scope)
                    .test_support()
                    .into_any_element(),
            );
            // Why a rewrite can't start now; shown inline when it blocks the
            // primary, and always in the control's tooltip and label.
            let recovering = row == WritingRow::Recover;
            let why = if recovering {
                Some("Recover the kept edit first")
            } else if busy {
                Some("BRN is working")
            } else if feedback_error {
                Some("Feedback couldn't load")
            } else if unresolved_included {
                Some("Exclude or re-select unlocated comments")
            } else if dirty {
                Some("Save your edit first")
            } else {
                None
            };
            if let Some(record) = latest_request {
                let toggle = record.id.clone();
                let retry = record.id.clone();
                let expanded = self.expanded_requests.contains(&record.id);
                secondary.push(
                    components::icon_button(
                        "inspect-rewrite-inputs",
                        IconName::Info,
                        if expanded {
                            "Hide saved inputs"
                        } else {
                            "Saved inputs"
                        },
                    )
                    .selected(expanded)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.expanded_requests.remove(&toggle) {
                            this.expanded_requests.insert(toggle.clone());
                        }
                        cx.notify();
                    }))
                    .into_any_element(),
                );
                let retry_blocked = if recovering {
                    Some("Recover the kept edit first")
                } else if busy {
                    Some("BRN is working")
                } else if proposals {
                    Some("Review or dismiss the ready rewrite first")
                } else if dirty {
                    Some("Save your edit first")
                } else {
                    None
                };
                let retry = button("retry-rewrite")
                    .label("Retry saved rewrite")
                    .compact()
                    .h(components::tokens::COMPACT_CONTROL)
                    .disabled(retry_blocked.is_some())
                    .accessibility_label(match retry_blocked {
                        Some(why) => format!("Retry saved rewrite, unavailable: {why}"),
                        None => "Retry saved rewrite".into(),
                    })
                    .tooltip(match retry_blocked {
                        Some(why) => format!("Retry is unavailable: {why}"),
                        None => "Run the rewrite again with the same saved inputs".into(),
                    })
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.retry_rewrite(retry.clone(), cx)),
                    );
                if row == WritingRow::Retry {
                    reason = retry_blocked;
                    primary = Some(retry.primary().into_any_element());
                } else {
                    secondary.push(retry.ghost().into_any_element());
                }
            }
            // As on the base, an open edit hides Undo; Changes keeps it.
            if let Some((_, operation)) = self.applied_rewrite.get(&note_id)
                && !dirty
            {
                let undo = operation.clone();
                secondary.push(
                    pill("undo-writing-apply", "Undo previous rewrite")
                        .tooltip("Undo the last applied rewrite; Changes keeps it")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.undo_receipt(&undo, window, cx)
                        }))
                        .into_any_element(),
                );
            }
            let rewrite = button("rewrite-note")
                .label("Rewrite with AI ⌘⇧R")
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .disabled(why.is_some())
                .accessibility_label(match why {
                    Some(why) => format!("Rewrite with AI, unavailable: {why}"),
                    None => "Rewrite with AI ⌘⇧R".into(),
                })
                .tooltip(
                    "Captures this note, included comments, overall feedback and saved context",
                )
                .on_click(cx.listener(|this, _, _, cx| this.rewrite_note(cx)));
            if matches!(row, WritingRow::Finished | WritingRow::Idle) {
                reason = why;
                primary = Some(rewrite.primary().into_any_element());
            } else {
                secondary.push(rewrite.ghost().into_any_element());
            }
            // A rewrite saved as Working that this window does not hold may
            // still run elsewhere: Stop… asks first. When the visible chat's
            // recovery card already offers Stop… for this run, it is there.
            if let Some(RunOutcome::Elsewhere(run)) = self.writing_run_outcome()
                && self.worker.as_ref().and_then(|w| w.run_id.as_deref()) != Some(run.as_str())
                && self.chat_card_stop().as_deref() != Some(run.as_str())
            {
                secondary.push(self.stop_saved_run_button(&run, cx).into_any_element());
            }
        }
        let line = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .children(info)
            .child(div().flex_1())
            .children(secondary)
            .when_some(reason, |line, why| {
                line.child(
                    div()
                        .id("writing-blocked-reason")
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .child(why)
                        .test_support(),
                )
            })
            .children(primary);
        // The inputs give way down to their heading; the bar never does.
        saved_inputs
            .map(|inputs| inputs.into_any_element())
            .into_iter()
            .chain([bar.child(line).test_support().into_any_element()])
            .collect()
    }
    fn review_body(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let p = pal();
        let Some(review) = &self.review else {
            return div().into_any_element();
        };
        let fit = self.review_fit;
        let writes = &review.candidate.request.writes;
        let shape = ProposalShape::of(writes);
        // Approval wording whenever the proposal writes a protected note.
        let protected = writes.iter().any(|write| {
            matches!(&write.data, RecordData::Note(note) if note.protected)
                || self.records.iter().any(|record| {
                    record.id == write.id
                        && matches!(&record.data, RecordData::Note(note) if note.protected)
                })
        });
        let counts = review.note.as_ref().map(|id| {
            let current = self
                .workspace
                .store
                .note(id)
                .ok()
                .flatten()
                .and_then(|record| match record.data {
                    RecordData::Note(note) => Some(note.markdown),
                    _ => None,
                })
                .unwrap_or_default();
            let tracked = diff::tracked(&current, &review.original);
            format!("+{} −{}", tracked.added, tracked.removed)
        });
        let editing = self.review_editing;
        let edited = editing || review.edited;
        let grouped = review.groups.groups.len() > 1;
        let partial = review.selected.len() != review.groups.groups.len();
        let snapshot = review
            .candidate
            .request
            .inputs
            .iter()
            .find_map(|input| self.workspace.writing_request(&input.record).ok());
        let open_comments = snapshot.as_ref().map_or(0, |snapshot| {
            let addressed: BTreeSet<String> = self
                .workspace
                .writing_comments(&snapshot.note.id)
                .map(|entries| {
                    entries
                        .into_iter()
                        .filter(|entry| entry.addressed)
                        .map(|entry| entry.record.id)
                        .collect()
                })
                .unwrap_or_default();
            snapshot
                .comments
                .iter()
                .filter(|comment| !addressed.contains(&comment.id))
                .count()
        });
        let footer = review_footer(shape, protected, open_comments);

        // The Proposal label plus the Protected chip states the review. A
        // narrower bar keeps the counts, then the shape, in the label's
        // tooltip and accessible name.
        let described = match &counts {
            Some(counts) => format!("Proposal · {} · {counts}", shape.label()),
            None => format!("Proposal · {}", shape.label()),
        };
        let shown_counts = counts.clone().filter(|_| !fit.actions_in_menu());
        let folded = fit.icons() || (counts.is_some() && shown_counts.is_none());
        let label = div()
            .id("review-proposal")
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap_1()
            .aria_label(described.clone())
            .when(folded, |label| {
                let tip = SharedString::from(described.clone());
                label.tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx)
                })
            })
            .child(
                div()
                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                    .child("Proposal"),
            )
            .when(!fit.icons(), |label| {
                label.child(
                    div()
                        .text_color(rgb(p.muted))
                        .child(format!("· {}", shape.label())),
                )
            })
            .when_some(shown_counts, |label, counts| {
                label.child(div().text_color(rgb(p.muted)).child(counts))
            })
            .test_support();
        let protected_chip = protected.then(|| {
            if fit.icons() {
                components::short_status_chip(
                    "review-protected",
                    Some(Tone::Info),
                    Some(IconName::Lock),
                    "",
                    "Protected",
                )
            } else {
                components::status_chip(
                    "review-protected",
                    Some(Tone::Info),
                    Some(IconName::Lock),
                    "Protected",
                )
            }
        });
        // Changes | Result: the old toggle's ID stays on Result.
        let mode = review.note.is_some().then(|| {
            let why = edited.then_some("Finish or cancel the edit first");
            components::segmented(
                "review-mode",
                vec![
                    button("review-show-changes")
                        .label("Changes")
                        .selected(!self.review_reading)
                        .disabled(edited)
                        .when_some(why, |segment, why| {
                            segment.accessibility_label(format!("Changes, unavailable: {why}"))
                        })
                        .tooltip(why.unwrap_or("Show the tracked changes"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.set_review_reading(false, window, cx)
                        })),
                    button("review-read-result")
                        .label("Result")
                        .selected(self.review_reading)
                        .disabled(edited)
                        .when_some(why, |segment, why| {
                            segment.accessibility_label(format!("Result, unavailable: {why}"))
                        })
                        .tooltip(why.unwrap_or("Read the proposed text, formatted"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.set_review_reading(true, window, cx)
                        })),
                ],
            )
        });
        // A proposal without note text names its reason here; the details
        // below list what it writes.
        let middle = if review.note.is_none() {
            let reason = review.candidate.request.reason.clone();
            let tip = SharedString::from(reason.clone());
            div()
                .id("review-reason")
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(rgb(p.muted))
                .aria_label(reason.clone())
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx)
                })
                .child(reason)
                .test_support()
                .into_any_element()
        } else {
            div().flex_1().into_any_element()
        };
        let mut secondary: Vec<gpui_kit::AnyElement> = vec![];
        if editing {
            secondary.push(
                pill("cancel-final", "Cancel edit")
                    .tooltip("Discard your edit to the final text and show the proposal again")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.cancel_review_edit(window, cx)),
                    )
                    .into_any_element(),
            );
        }
        if !fit.actions_in_menu() {
            secondary.push(
                pill("close-review", "Keep for later")
                    .tooltip("Close the review; the proposal stays ready in the writing bar")
                    .on_click(cx.listener(|this, _, window, cx| this.keep_review(window, cx)))
                    .into_any_element(),
            );
            secondary.push(
                pill("dismiss-proposal", "Dismiss")
                    .tooltip("Dismiss this proposal; the note stays unchanged")
                    .on_click(cx.listener(|this, _, window, cx| this.dismiss_review(window, cx)))
                    .into_any_element(),
            );
        }
        // The one primary, by state; blocked controls stay in place with
        // their reason.
        let full = shape.primary(protected);
        let apply_blocked = if edited {
            Some("Finish or cancel the edit first")
        } else if partial {
            Some("Prepare your selection first")
        } else {
            None
        };
        // The full label shows only where Apply leads and the bar has room;
        // it stays the tooltip and accessible name.
        let apply_leads = !editing && !(grouped && partial);
        let apply = button("apply-final")
            .label(if fit.icons() || !apply_leads {
                if protected { "Approve" } else { "Apply" }.to_owned()
            } else {
                full.clone()
            })
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .disabled(apply_blocked.is_some())
            .accessibility_label(match apply_blocked {
                Some(why) => format!("{full}, unavailable: {why}"),
                None => full.clone(),
            })
            .tooltip(match apply_blocked {
                Some(why) => format!("{full} is unavailable: {why}"),
                None => format!("{full} · {}", shape.binding(protected)),
            })
            .on_click(cx.listener(|this, _, window, cx| this.apply_review(window, cx)));
        let select_blocked = if edited {
            Some("Finish or cancel the edit first")
        } else if review.selected.is_empty() {
            Some("Include at least one group")
        } else if !partial {
            Some("Omit a group first")
        } else {
            None
        };
        let prepare_selected = button("prepare-selected")
            .label("Prepare selection")
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .disabled(select_blocked.is_some())
            .accessibility_label(match select_blocked {
                Some(why) => format!("Prepare selection, unavailable: {why}"),
                None => "Prepare selection".into(),
            })
            .tooltip(select_blocked.unwrap_or("Review only the included groups as a new proposal"))
            .on_click(cx.listener(|this, _, window, cx| this.prepare_review_selection(window, cx)));
        // A Prepare selection that waits sits in the bar only when the full
        // bar has room; otherwise it moves beside the group toggles, where
        // its reason applies, so the bar keeps one row with the primary
        // last.
        let mut waiting_selection = None;
        let mut prepare_selected = Some(prepare_selected);
        if grouped && (editing || !partial) {
            let waiting = prepare_selected.take().map(|control| control.ghost());
            if fit.actions_in_menu() {
                waiting_selection = waiting;
            } else {
                secondary.extend(waiting.map(IntoElement::into_any_element));
            }
        }
        let primary = if editing {
            secondary.push(apply.outline().into_any_element());
            button("prepare-final")
                .label("Use this text")
                .compact()
                .h(components::tokens::COMPACT_CONTROL)
                .disabled(!review.edited)
                .accessibility_label(if review.edited {
                    "Use this text"
                } else {
                    "Use this text, unavailable: Edit the text first"
                })
                .tooltip(if review.edited {
                    "Review your text as a new proposal"
                } else {
                    "Edit the text first"
                })
                .on_click(cx.listener(|this, _, window, cx| this.prepare_review_edit(window, cx)))
                .primary()
        } else if let Some(selection) = prepare_selected.filter(|_| grouped && partial) {
            secondary.push(apply.outline().into_any_element());
            selection.primary()
        } else {
            apply.primary()
        };
        // One row; a crowded state wraps rather than clipping a control.
        let bar = components::toolbar("review-bar")
            .h_auto()
            .min_h(components::tokens::TOOLBAR)
            .flex_wrap()
            .py(px(3.5))
            .font_family(UI_FONT)
            .text_size(SMALL_TEXT)
            .child(label)
            .children(protected_chip)
            .children(mode)
            .child(middle)
            .children(secondary)
            .children(self.comments_toggle(cx))
            .child(primary);
        let blocked_reason = if editing {
            Some(
                "Finish or cancel the edit first. Use this text reviews your text as a new proposal.",
            )
        } else if grouped && review.selected.is_empty() {
            Some("Include at least one group, then prepare your selection.")
        } else if grouped && partial {
            Some(
                "Prepare your selection first. Only the included groups become a new proposal to review.",
            )
        } else {
            None
        };
        // The details scroll; with several rows they give way first, down to
        // a line or two, so the review text and the footer keep their room.
        let note_writes = writes
            .iter()
            .filter(|write| matches!(write.data, RecordData::Note(_)))
            .count();
        let tall = self.review_inputs || review.note.is_none() || grouped || note_writes > 1;
        let mut details = div()
            .id("review-details")
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .py_1()
            .flex_shrink_0()
            .when(tall, |details| details.flex_shrink(1.).min_h(px(44.)))
            .text_size(UI_TEXT)
            .max_h(px(220.))
            .overflow_y_scroll()
            .when_some(blocked_reason, |details, why| {
                details.child(
                    div()
                        .id("review-blocked-reason")
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .aria_label(why)
                        .child(why)
                        .test_support(),
                )
            });
        if self.review_inputs || review.note.is_none() {
            // The bar names the reason when the proposal has no note text.
            if review.note.is_some() {
                details = details.child(review.candidate.request.reason.clone());
            }
            for write in &review.candidate.request.writes {
                details = details.child(metadata(match &write.data {
                    RecordData::Note(n) => format!(
                        "Note: {}{}",
                        n.title,
                        if n.protected { " · protected" } else { "" }
                    ),
                    RecordData::Action(a) => format!("Task: {} · {:?}", a.description, a.state),
                    RecordData::Thread(t) => format!("Chat: {}", t.title),
                    RecordData::Message(m) => format!("Message: {}", short(&m.text)),
                    RecordData::Link(_) => "Connect related work".into(),
                    RecordData::Comment(c) => format!("Comment: {}", short(&c.body)),
                    RecordData::Source(s) => format!("Source: {}", s.label),
                    RecordData::Asset(_) => "Retain a meaningful figure".into(),
                    RecordData::Run(_) => "Save work progress".into(),
                    RecordData::Settings(_) => "Change workspace settings".into(),
                    RecordData::ChatControl(_) => "Change chat controls".into(),
                    RecordData::Project(_) => "Change project configuration".into(),
                }));
            }
            if let Some(snapshot) = &snapshot {
                details = details.child(metadata(format!(
                    "Inputs: saved v{} · {} · {}",
                    snapshot.note.version,
                    plural(snapshot.comments.len(), "comment"),
                    plural(snapshot.messages.len(), "chat message")
                )));
                if !snapshot.general_feedback.is_empty() {
                    details = details
                        .child(subheading("Overall feedback"))
                        .child(snapshot.general_feedback.clone());
                }
            }
            details = details.child(metadata(
                "A note that changed since, or has an open edit, stays unchanged until that is resolved.",
            ));
        }
        if grouped {
            details = details.child(metadata("Include or omit whole notes and tasks."));
            let mut groups = div().flex().flex_wrap().gap_1();
            for group in &review.groups.groups {
                let id = group.id.clone();
                let included = review.selected.contains(&id);
                let label = format!(
                    "{} {}: {}",
                    if included { "Include" } else { "Omit" },
                    if group.kind == "Action" {
                        "task"
                    } else {
                        "note"
                    },
                    short(&group.title)
                );
                let blocked = if !review.groups.ungrouped_writes.is_empty() {
                    Some("These changes can't be separated")
                } else if edited {
                    Some("Finish or cancel the edit first")
                } else {
                    None
                };
                groups = groups.child(
                    pill(format!("review-group-{id}"), label.clone())
                        .selected(included)
                        .disabled(blocked.is_some())
                        .when_some(blocked, |toggle, why| {
                            toggle
                                .accessibility_label(format!("{label}, unavailable: {why}"))
                                .tooltip(why)
                        })
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
            details = details.child(groups.children(waiting_selection));
            if !review.groups.ungrouped_writes.is_empty() {
                let line = "These changes can't be separated. Apply all or ask for a new rewrite.";
                details = details.child(
                    div()
                        .id("review-inseparable")
                        .aria_label(line)
                        .child(line)
                        .test_support(),
                );
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
            // "Showing: A · B", the shown note selected.
            let mut members = div()
                .id("review-members")
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(p.muted))
                        .child("Showing:"),
                );
            for (index, (id, name)) in notes.into_iter().enumerate() {
                let selected = review.note.as_ref() == Some(&id);
                if index > 0 {
                    members = members.child(div().text_color(rgb(p.muted)).child("·"));
                }
                members = members.child(
                    pill(format!("review-member-{id}"), short(&name))
                        .tooltip(format!("Show {name}"))
                        .accessibility_label(format!("Show {name}"))
                        .selected(selected)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select_review_note(id.clone(), window, cx)
                        })),
                );
            }
            details = details.child(members.test_support());
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
        let centered = self.layout.maximized() == Some(TileKind::Note);
        // The review text, then comments when they're stacked, then the
        // footer that names exactly what Apply saves; it never folds and
        // never moves. The text asks for 120 px (plus the open section's
        // floor) after the details give way, and yields to a few lines
        // before the footer would.
        let stacked = self.comments_stacked();
        let text = review.note.is_some();
        let (prefer, min) = match (text, stacked) {
            (false, _) => (px(0.), px(0.)),
            (true, false) => (text_min(true), text_min(false)),
            (true, true) => (text_min(true) + self.stacked_floor(), text_min(false)),
        };
        let mut document = div()
            .id("review-body")
            .flex()
            .flex_grow(1.)
            .flex_basis(if text { text_min(true) } else { px(0.) })
            // Inside the area the open section gives way before the text.
            .flex_shrink(YIELD_LAST)
            .min_h(min)
            .min_w_0()
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .when(centered, |text| text.justify_center())
                    .child(reading_column().child(body)),
            );
        if self.comments_beside() {
            document = document.child(self.margin_column(cx));
        }
        div()
            .id("review-panel")
            .size_full()
            .flex()
            .flex_col()
            .min_h_0()
            .child(bar.test_support())
            .child(details.test_support())
            .child(self.text_and_comments(document.test_support(), prefer, min, stacked, cx))
            .child(self.writing_activity(cx))
            .child(
                div()
                    .id("note-bar")
                    .flex_shrink_0()
                    .px_3()
                    .py_2()
                    .font_family(UI_FONT)
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.ink))
                    .bg(rgb(p.alt))
                    .border_t_1()
                    .border_color(rgb(p.hairline))
                    .aria_label(footer.clone())
                    .child(footer)
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
                .child(components::empty_state(
                    "compare-empty",
                    IconName::Columns2,
                    "Nothing to compare",
                    "Compare shows a whole-note proposal beside the saved note.",
                    None,
                ))
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
                    .child(section(format!("Current · v{version}")))
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
                    .child(section("Proposal · exact text under review"))
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

/// A one-line label's width in the UI font, as the text system lays it out.
fn text_width(window: &Window, text: &str, size: gpui_kit::Pixels) -> f32 {
    let mut style = window.text_style();
    style.font_family = UI_FONT.into();
    let run = style.to_run(text.len());
    f32::from(
        window
            .text_system()
            .shape_line(text.to_owned().into(), size, &[run], None)
            .width,
    )
}

/// The widest line of note or review text; wider panels keep 24 px gutters
/// and leave the rest as margin.
const READING_MEASURE: f32 = 680.;

/// The note and review text column: at most 680 px of text with 24 px
/// gutters, filling a narrower panel. Long words and URLs wrap.
fn reading_column() -> gpui_kit::Div {
    div()
        .flex_1()
        .min_w_0()
        .h_full()
        .max_w(px(READING_MEASURE) + components::tokens::SPACE_6 * 2.)
        .px(components::tokens::SPACE_6)
        .py_2()
}

/// One transcript row's column: the reading measure with 24 px gutters.
fn transcript_column() -> gpui_kit::Div {
    div()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .max_w(px(READING_MEASURE) + components::tokens::SPACE_6 * 2.)
        .px(components::tokens::SPACE_6)
}

/// A recovery card's tone; neutral has none.
fn severity_tone(severity: Severity) -> Option<Tone> {
    match severity {
        Severity::Neutral => None,
        Severity::Warning => Some(Tone::Warning),
        Severity::Error => Some(Tone::Error),
    }
}

/// A banner's secondary action: a 28 px outline button.
fn banner_button(id: impl Into<gpui_kit::ElementId>, label: &'static str) -> Button {
    button(id)
        .label(label)
        .compact()
        .outline()
        .h(components::tokens::COMPACT_CONTROL)
}

/// "1 comment", "3 comments".
fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}
/// The text's few lines, or the 120 px it keeps beside an open comments
/// section. With a banner shown it asks for the 120 px and keeps the few
/// lines as its floor.
fn text_min(stacked: bool) -> gpui_kit::Pixels {
    px(if stacked { 120. } else { 48. })
}

/// The open comments section's floor: its label and the first card's
/// first line.
pub(super) const COMMENTS_FLOOR: gpui_kit::Pixels = px(64.);

/// The stacked draft's input height.
const STACKED_DRAFT_INPUT: f32 = 56.;

/// The open section's floor while a draft is open: the label row (26),
/// the draft (8 + 28 control row + 4 + input + 8) with its 8 px below,
/// and the 1 px rule above.
pub(super) const STACKED_DRAFT_FLOOR: gpui_kit::Pixels =
    px(26. + 8. + 28. + 4. + STACKED_DRAFT_INPUT + 8. + 8. + 1.);

/// The floor of the box that holds the banners after the first one in a
/// short Note: its first line (the saved version's label, or the next
/// banner's opening line) stays in view and the rest scrolls. The first
/// banner's actions are always in view above it.
const BANNER_FLOOR: gpui_kit::Pixels = px(28.);

/// The shrink factor of the review or note text area: next to the reply's
/// and the details' factor of 1, it gives way only once they reach their
/// minimums.
const YIELD_LAST: f32 = 0.01;

/// The shrink factor of rows that give way before every other one: a
/// large factor on the yielding row rather than a small one on the rest,
/// so the rest still take up the remaining overflow once it reached its
/// floor.
const YIELD_FIRST: f32 = 1000.;

/// What a proposal writes. It names the review bar's label and primary and
/// the footer's binding sentence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ProposalShape {
    /// One note and nothing else.
    Note,
    /// Several notes and nothing else.
    Notes(usize),
    /// Anything else, counted as changes.
    Changes(usize),
}
impl ProposalShape {
    pub(super) fn of(writes: &[Put]) -> Self {
        let notes = writes
            .iter()
            .filter(|write| matches!(write.data, RecordData::Note(_)))
            .count();
        match (notes, writes.len()) {
            (1, 1) => Self::Note,
            (notes, total) if notes == total && notes > 1 => Self::Notes(notes),
            (_, total) => Self::Changes(total),
        }
    }
    pub(super) fn label(self) -> String {
        match self {
            Self::Note => "whole note".into(),
            Self::Notes(count) => plural(count, "note"),
            Self::Changes(count) => plural(count, "change"),
        }
    }
    /// "Apply whole note", or "Approve …" when a protected note is written.
    pub(super) fn primary(self, protected: bool) -> String {
        format!(
            "{} {}",
            if protected { "Approve" } else { "Apply" },
            self.label()
        )
    }
    /// The sentence that says exactly what Apply saves.
    pub(super) fn binding(self, protected: bool) -> String {
        match (self, protected) {
            (Self::Note, false) => "Applying saves exactly this text.".into(),
            (Self::Note, true) => "Approving saves exactly this text to a protected note.".into(),
            (Self::Notes(count), false) => format!("Applying saves exactly these {count} notes."),
            (Self::Notes(count), true) => {
                format!("Approving saves exactly these {count} notes, including protected notes.")
            }
            (Self::Changes(1), false) => "Applying saves exactly this change.".into(),
            (Self::Changes(1), true) => {
                "Approving saves exactly this change to a protected note.".into()
            }
            (Self::Changes(count), false) => {
                format!("Applying saves exactly these {count} changes.")
            }
            (Self::Changes(count), true) => format!(
                "Approving saves exactly these {count} changes, including a protected note."
            ),
        }
    }
}

/// The review footer: the binding sentence first, then the comments that
/// stay open (omitted when none).
pub(super) fn review_footer(shape: ProposalShape, protected: bool, open: usize) -> String {
    let binding = shape.binding(protected);
    match open {
        0 => binding,
        1 => format!("{binding} 1 comment stays open until you mark it addressed."),
        open => format!("{binding} {open} comments stay open until you mark them addressed."),
    }
}
