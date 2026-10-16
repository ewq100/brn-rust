//! Writing and conversation tiles: chat, note, in-place review, compare and
//! linked work. Every action calls the same checked services as before.
use super::shell::{item, pill};
use super::state::ReviewSummary;
use super::tiles::scroll;
use super::*;
use brn_writing_ui::writing::READING_MEASURE;
use gpui_kit::SharedString;
use gpui_kit::StyledText;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::menu::DropdownMenu;
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
            .writing
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
    pub(super) fn chat_card_stop(&self) -> Option<String> {
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
                        .track_focus(&self.archived_banner)
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
