//! Native chat presentation delegates durable control to the shared checked service.
use super::shell::{one_line, pill};
use super::*;
use gpui_kit::Focusable;
use gpui_kit::component::message_scroller::{MessageScroller, MessageScrollerState};
use gpui_kit::component::{Icon, Sizable, Size};

pub(super) struct ChatScroller {
    pub state: Entity<MessageScrollerState>,
    pub keys: Vec<String>,
    pub signature: String,
}
impl ThreadsUi {
    pub(super) fn observe_run_activity(&mut self, chat: &str, text: &str) {
        if self
            .run_activity
            .transition(RunActivityEvent::Observed { chat, text })
        {
            self.run_activity_scroll.scroll_to_bottom();
        }
    }
    pub(super) fn worker_status(&self, worker: &Worker) -> String {
        if worker.kind() == WorkerKind::Run && self.queue_state().quiet {
            if worker.cancel.is_cancelled() {
                "Stopping…".into()
            } else {
                "Working quietly…".into()
            }
        } else {
            worker.kind().status(&self.progress)
        }
    }
    /// The running line above the composer while this chat's reply works:
    /// the real worker status beside a spinner (Stop is the composer's).
    pub(super) fn run_status_line(&self) -> Option<gpui_kit::AnyElement> {
        let worker = self.worker.as_ref().filter(|_| self.running_here())?;
        let status = self.worker_status(worker);
        Some(
            div()
                .id("run-line")
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .font_family(UI_FONT)
                .text_size(SMALL_TEXT)
                .text_color(rgb(pal().muted))
                .aria_label(status.clone())
                .child(
                    Icon::new(IconName::LoaderCircle)
                        .with_size(Size::XSmall)
                        .flex_shrink_0()
                        .text_color(rgb(pal().info)),
                )
                .child(div().min_w_0().truncate().child(status))
                .test_support()
                .into_any_element(),
        )
    }
    /// The chat's last recorded run, in words: its state, provider, model
    /// and effort as the run saved them (a later selection never relabels
    /// it), and its saved progress.
    pub(super) fn last_work_status(&self, chat: &str) -> Option<String> {
        let record = latest_run(&self.records, &self.history, chat)?;
        let RecordData::Run(run) = &record.data else {
            return None;
        };
        let state = match run.state {
            RunState::Working if self.running_here() => "Working",
            RunState::Working => "Saved as working",
            RunState::Completed => "Completed",
            RunState::Failed => "Failed",
            RunState::Interrupted => "Stopped or interrupted",
            RunState::Cancelled => "Cancelled",
        };
        let model = super::shell::model_display(&run.provider, &run.model);
        let mut line = format!("Last reply: {state} · {model} · {} effort", run.effort);
        if !run.progress.trim().is_empty() {
            line.push_str(&format!(" · {}", one_line(&run.progress)));
        }
        Some(line)
    }
    /// The per-conversation activity disclosure (header Activity): the last
    /// recorded run, the routine steps of the latest trace (unless Quiet),
    /// loaded guidance and local MCP status. Answers, Stop, errors,
    /// permissions and reviews never live here.
    pub(super) fn activity_panel(
        &self,
        chat: &str,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let p = pal();
        let quiet = self.queue_state().quiet;
        let steps = !quiet && self.run_activity.has_steps_for(chat);
        let ended = (!self.running_here() && steps)
            .then(|| self.run_activity.status().to_owned())
            .filter(|status| !status.is_empty());
        let quiet_control = button("chat-quiet")
            .ghost()
            .compact()
            .h(components::tokens::COMPACT_CONTROL)
            .icon(if quiet { IconName::EyeOff } else { IconName::Eye })
            .label(if quiet { "Quiet on" } else { "Quiet off" })
            .accessibility_label(if quiet {
                "Quiet on · show activity"
            } else {
                "Quiet off · hide activity"
            })
            .selected(quiet)
            .tooltip(if quiet {
                "Quiet is on · click to show routine steps. Answers, Stop, errors and decisions stay visible."
            } else {
                "Quiet is off · click to hide routine steps. Answers, Stop, errors and decisions stay visible."
            })
            .on_click(cx.listener(|this, _, _, cx| this.toggle_quiet(cx)));
        let mut panel = div()
            .id("chat-activity-panel")
            .flex()
            .flex_col()
            .gap_2()
            .w_full()
            .px_3()
            .py_2()
            .max_h(px(220.))
            .rounded(px(theme::metrics::RADIUS_CARD))
            .border_1()
            .border_color(rgb(p.hairline))
            .bg(rgb(p.alt))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(subheading("Activity").flex_1())
                    .child(quiet_control)
                    .child(
                        components::icon_button(
                            "chat-activity-close",
                            IconName::Close,
                            "Hide activity",
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.conversation.activity_open = false;
                            cx.notify();
                        })),
                    ),
            );
        let last = self.last_work_status(chat);
        panel = panel.child(
            metadata(
                last.clone()
                    .unwrap_or_else(|| "No reply recorded yet".into()),
            )
            .id("chat-last-work")
            .aria_label(last.unwrap_or_else(|| "No reply recorded yet".into()))
            .test_support(),
        );
        if let Some(status) = ended {
            panel = panel.child(
                metadata(format!("Latest trace · {status}"))
                    .id("run-activity-status")
                    .test_support(),
            );
        }
        if steps {
            let mut list = div()
                .id("run-activity-steps")
                .flex()
                .flex_col()
                .gap_1()
                .max_h(px(120.))
                .flex_shrink_0()
                .track_scroll(&self.run_activity_scroll)
                .overflow_x_scroll()
                .overflow_y_scroll()
                .test_support();
            for (index, text) in self.run_activity.steps().iter().enumerate() {
                list = list.child(
                    metadata(text.clone())
                        .font_family(UI_FONT)
                        .text_size(SMALL_TEXT)
                        .id(format!("run-activity-step-{index}"))
                        .aria_label(text.clone())
                        .test_support(),
                );
            }
            panel = panel.child(
                div()
                    .id("run-activity")
                    .flex()
                    .flex_col()
                    .child(list)
                    .test_support(),
            );
        } else if quiet {
            panel = panel.child(metadata(
                "Quiet is on: routine steps are hidden. Answers, Stop, errors and decisions stay visible.",
            ));
        }
        panel
            .child(self.loaded_skill_details(chat))
            .child(self.mcp_detail(chat))
            .test_support()
            .into_any_element()
    }
    /// Quiet is a saved display preference; it changes no work.
    pub(super) fn toggle_quiet(&mut self, cx: &mut Context<Self>) {
        let quiet = !self.queue_state().quiet;
        if let Err(e) = self
            .workspace
            .choose_quiet(&format!("quiet:{}", uuid::Uuid::new_v4()), quiet)
        {
            self.fail(e.to_string());
        }
        cx.notify();
    }
    /// "Stop…" for a saved Working run this window does not hold: BRN can't
    /// tell whether another window or the CLI still runs it, so it confirms.
    pub(super) fn stop_saved_run_button(&self, run: &str, cx: &mut Context<Self>) -> Button {
        let run = run.to_owned();
        button(format!("stop-saved-run-{run}"))
            .label("Stop…")
            .compact()
            .outline()
            .h(components::tokens::COMPACT_CONTROL)
            .tooltip("Stop this reply; asks first")
            .on_click(cx.listener(move |this, _, window, cx| {
                this.confirm_stop_saved_run(run.clone(), window, cx)
            }))
    }
    pub(super) fn confirm_stop_saved_run(
        &mut self,
        run: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = cx.entity().downgrade();
        self.open_dialog(window, cx, move |dialog, _, _| {
            let (target, run) = (target.clone(), run.clone());
            components::consequence_dialog(
                dialog,
                "stop-saved-run",
                "Stop this reply?",
                "BRN can't tell whether another BRN window or the CLI is still running it. Stopping marks it stopped and pauses the queue; changes it tries to save after that are refused. You can Continue afterwards.",
                "Stop reply",
                std::rc::Rc::new(move |_, cx| {
                    let _ = target.update(cx, |view, cx| view.stop_saved_run(&run, cx));
                }),
            )
        });
    }
    /// The existing checked stop: pauses the queue, marks the Working run
    /// Interrupted and fences it, so later writes from whoever runs it are
    /// refused.
    fn stop_saved_run(&mut self, run: &str, cx: &mut Context<Self>) {
        match self.workspace.stop_chat_run(
            &format!("stop-saved:{}", uuid::Uuid::new_v4()),
            run,
            None,
        ) {
            Ok(_) => self
                .warn("Reply stopped. The queue is paused; Continue or Resume queue when ready."),
            Err(e) => self.fail(format!(
                "Could not stop this reply: {e}. It may still be running; try Stop… again."
            )),
        }
        self.refresh();
        cx.notify();
    }
    /// `resume_chat_queue`'s own check: the queue's pending item has a run
    /// saved as Working, which Resume queue stops before it resumes.
    pub(super) fn pending_run_working(&self, queue: &ChatControl) -> bool {
        queue
            .pending
            .as_ref()
            .and_then(|q| q.run.as_deref())
            .is_some_and(|run| {
                matches!(self.workspace.store.record(run), Ok(Some(Record { data: RecordData::Run(r), .. })) if r.state == RunState::Working)
            })
    }
    pub(super) fn assets_in_chat(&self, chat: Option<&str>) -> Arc<BTreeMap<String, Arc<Image>>> {
        #[cfg(all(test, feature = "native-test-support"))]
        WRITING_DISPLAY_LOADS.with(|loads| {
            let (assets, runs) = loads.get();
            loads.set((assets + 1, runs));
        });
        Arc::new(
            self.records
                .iter()
                .filter(|r| {
                    matches!(r.data, RecordData::Asset(_)) && record_in_chat(r, &self.records, chat)
                })
                .filter_map(|r| {
                    let key = format!("brn-asset://{}", r.id);
                    self.assets.get(&key).cloned().map(|image| (key, image))
                })
                .collect(),
        )
    }
    pub(super) fn pause_failed_worker(&mut self) {
        let key = format!("worker-failure:{}", uuid::Uuid::new_v4());
        let result = if let Some(run) = self.worker.as_ref().and_then(Worker::run_identity) {
            self.workspace.interrupt_chat_run(
                &key,
                &run.run_id,
                (!self.stream.is_empty()).then_some(self.stream.as_str()),
            )
        } else {
            self.workspace.recover_queue(&key)
        };
        if let Err(e) = result {
            self.fail(format!(
                "Could not preserve interruption state: {e}. Copy visible progress before closing."
            ));
        }
    }
    pub(super) fn queue_state(&self) -> ChatControl {
        self.workspace
            .chat_control()
            .ok()
            .and_then(|r| match r.data {
                RecordData::ChatControl(c) => Some(c),
                _ => None,
            })
            .unwrap_or_default()
    }
    pub(super) fn save_composer(&mut self, destination: ComposerDestination, text: String) {
        self.composer_drafts
            .preserve(destination.clone(), text.clone());
        let Ok(key) = serde_json::to_string(&destination) else {
            return;
        };
        let generation = self.draft_generations.entry(key.clone()).or_default();
        let Some(next) = generation.checked_add(1) else {
            self.fail("Composer recovery counter exhausted. Copy your text before closing.");
            return;
        };
        match self.workspace.store.save_chat_draft(&key, next, &text) {
            Ok(_) => *generation = next,
            Err(e) => self.fail(format!(
                "Composer recovery could not be saved: {e}. Keep this window open or copy your text."
            )),
        }
    }
    fn composer_chat(&mut self, prompt: &str) -> AppResult<String> {
        match self.chat.clone() {
            ChatTarget::Thread(id) => {
                if self.thread_resolved(&id) {
                    return Err(AppError::Invalid(
                        "Unarchive this chat before sending".into(),
                    ));
                }
                Ok(id)
            }
            ChatTarget::NewForNote(note) => self.ensure_writing_thread(&note),
            ChatTarget::New => self.workspace.new_thread(&short(prompt)).map(|r| r.id),
        }
    }
    pub(super) fn enqueue_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = self.composer.read(cx).value().to_string();
        if prompt.trim().is_empty() {
            return;
        }
        let destination = self.destination();
        let editing_item = self.queued_edit_for_current_chat().map(str::to_owned);
        let edited = editing_item.is_some();
        // Every waiting item ahead of the new one, in any chat.
        let ahead = self.queue_state().queue.len();
        let result = if let Some(item) = &editing_item {
            self.workspace
                .edit_queued(
                    &format!("edit-queue:{}", uuid::Uuid::new_v4()),
                    item,
                    &prompt,
                )
                .map(|_| None)
        } else {
            self.composer_chat(&prompt).and_then(|thread| {
                self.workspace
                    .queue_message(&format!("queue:{}", uuid::Uuid::new_v4()), &thread, &prompt)
                    .map(|_| Some(thread))
            })
        };
        match result {
            Ok(thread) => {
                // Clear once, after the service accepted it (CHAT-02); a
                // queued edit gives back the draft it set aside.
                let restored = if edited {
                    self.editing_queue = None;
                    self.conversation.take_stash(&destination)
                } else {
                    String::new()
                };
                self.save_composer(destination, restored.clone());
                self.composer
                    .update(cx, |e, cx| e.set_value(restored, window, cx));
                if let Some(thread) = thread {
                    self.chat = ChatTarget::Thread(thread);
                }
                self.refresh();
                let paused = self.queue_state().paused;
                let busy = self.worker.is_some();
                // First match wins: Busy and Paused hold together after Stop.
                if edited {
                    self.inform("Queued message updated.");
                } else if paused {
                    self.warn("Queued. The queue is paused; Resume queue to run it.");
                } else if busy && ahead > 0 {
                    self.inform(format!(
                        "Queued; runs after {ahead} waiting message{}.",
                        if ahead == 1 { "" } else { "s" }
                    ));
                } else if busy {
                    self.inform("Queued; runs after the current reply.");
                }
                // An idle, unpaused send never reaches this queue path: `ask`
                // starts it directly, and `start_run` clears an earlier notice.
                if !busy && !paused {
                    self.dispatch_next(cx);
                }
            }
            Err(e) => self.fail(format!(
                "Could not {}: {e}. Your text is preserved; {} again to retry.",
                if edited {
                    "save the queued message"
                } else {
                    "queue"
                },
                if edited { "Save queued" } else { "Queue" },
            )),
        }
        cx.notify();
    }
    pub(super) fn dispatch_next(&mut self, cx: &mut Context<Self>) {
        if self.worker.is_some() {
            return;
        }
        match self
            .workspace
            .dispatch_queued(&format!("dispatch:{}", uuid::Uuid::new_v4()))
        {
            Ok(Some(q)) => {
                self.start_run(q.text, q.thread, vec![], None, cx);
                if self.worker.is_none() {
                    let _ = self
                        .workspace
                        .pause_queue(&format!("start-failed:{}", uuid::Uuid::new_v4()));
                }
            }
            Ok(None) => {}
            Err(e) => {
                self.fail(e.to_string());
                let _ = self
                    .workspace
                    .pause_queue(&format!("dispatch-failed:{}", uuid::Uuid::new_v4()));
            }
        }
        self.refresh();
        cx.notify();
    }
    pub(super) fn resume_chat_queue(&mut self, cx: &mut Context<Self>) {
        if self.worker.is_some() {
            self.warn("Wait for current work to stop before Resume.");
            cx.notify();
            return;
        }
        if let Some(run) = self.queue_state().pending.and_then(|q| q.run)
            && matches!(self.workspace.store.record(&run), Ok(Some(Record { data: RecordData::Run(r), .. })) if r.state == RunState::Working)
            && let Err(e) = self.workspace.interrupt_chat_run(
                &format!("resume-stale:{}", uuid::Uuid::new_v4()),
                &run,
                None,
            )
        {
            self.fail(e.to_string());
            cx.notify();
            return;
        }
        match self
            .workspace
            .resume_queue(&format!("resume:{}", uuid::Uuid::new_v4()))
        {
            Ok(_) => self.dispatch_next(cx),
            Err(e) => self.fail(e.to_string()),
        }
        cx.notify();
    }
    pub(super) fn acknowledge_run(
        &mut self,
        terminal: Option<(String, u64)>,
        success: bool,
    ) -> bool {
        let pending = self.queue_state().pending;
        let local_run = self
            .worker
            .as_ref()
            .and_then(Worker::run_identity)
            .map(|run| &run.run_id);
        if let Some(q) = &pending
            && q.run.as_ref() != local_run
        {
            return false;
        }
        let Some((run, fence)) = terminal else {
            if !self
                .worker
                .as_ref()
                .is_some_and(|worker| worker.continuation().is_some())
            {
                self.pause_failed_worker();
            }
            return false;
        };
        if local_run != Some(&run) {
            return false;
        }
        if let Some(q) = pending {
            let Some(message) = q.message else {
                return false;
            };
            return self
                .workspace
                .finish_queued(
                    &format!("finish-queue:{run}:{fence}"),
                    &q.id,
                    &message,
                    &run,
                    fence,
                    success,
                )
                .is_ok()
                && success;
        }
        let valid = matches!(self.workspace.store.record(&run), Ok(Some(Record { data: RecordData::Run(r), .. })) if r.fence == fence && if success { r.state == RunState::Completed } else { r.state != RunState::Working });
        if valid && !success {
            let _ = self
                .workspace
                .pause_queue(&format!("failed-queue:{run}:{fence}"));
        }
        valid && success
    }
    /// The waiting-message panel above the composer (CHAT-03). It shows only
    /// when something waits; every item keeps Edit, Remove and keyboard
    /// Move up / Move down (no drag). A paused queue is a warning card with
    /// Resume queue; a running one is a quiet list.
    pub(super) fn queue_panel(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let c = self.queue_state();
        if c.pending.is_none() && c.queue.is_empty() {
            return None;
        }
        let here = self.chat_thread().map(str::to_owned);
        let waiting = format!("{} waiting", c.queue.len());
        // A short window keeps the queue to its one summary row; the
        // disclosure shows the items.
        let collapsed = self.short_window && !self.queue_expanded;
        let disclosure = self.short_window.then(|| {
            let (icon, label) = if collapsed {
                (IconName::ChevronRight, "Show waiting messages")
            } else {
                (IconName::ChevronDown, "Hide waiting messages")
            };
            components::icon_button("queue-disclosure", icon, label)
                .flex_shrink_0()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.queue_expanded = !this.queue_expanded;
                    cx.notify();
                }))
        });
        // The state leads and never truncates; in a narrow chat the count
        // gives way first. The full line is the tooltip and accessible name.
        let state = match (c.paused, self.short_window) {
            (true, true) => "Paused",
            (true, false) => "Queue paused",
            (false, _) => "Queue",
        };
        let summary = format!("{state} · {waiting}");
        let summary_row = |weight: gpui_kit::FontWeight| {
            let tooltip = summary.clone();
            div()
                .id("queue-summary")
                .flex()
                .flex_1()
                .min_w_0()
                .text_size(SMALL_TEXT)
                .aria_label(summary.clone())
                .tooltip(move |window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(tooltip.clone()).build(window, cx)
                })
                .child(
                    div()
                        .id("queue-summary-state")
                        .flex_shrink_0()
                        .font_weight(weight)
                        .child(state)
                        .test_support(),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .child(format!("\u{a0}· {waiting}")),
                )
                .test_support()
        };
        let busy = self.worker.is_some();
        let mut queue = if c.paused {
            components::card("queue-panel", Some(Tone::Warning))
                .gap_1()
                .p_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .children(disclosure)
                        .child(components::tone_mark("queue-panel-tone", Tone::Warning))
                        .child(summary_row(gpui_kit::FontWeight::MEDIUM))
                        .child(
                            button("queue-resume")
                                .label("Resume queue")
                                .compact()
                                .outline()
                                .h(components::tokens::COMPACT_CONTROL)
                                .flex_shrink_0()
                                .disabled(busy)
                                .when(busy, |button| {
                                    button
                                        .accessibility_label(
                                            "Resume queue, unavailable: wait for current work to stop",
                                        )
                                        .tooltip("Wait for current work to stop")
                                })
                                .when(!busy, |button| {
                                    button.tooltip("Run the waiting messages in order")
                                })
                                .on_click(cx.listener(|this, _, _, cx| this.resume_chat_queue(cx))),
                        ),
                )
        } else {
            div()
                .id("queue-panel")
                .flex()
                .flex_col()
                .gap_1()
                .px_3()
                .py_2()
                .rounded(px(theme::metrics::RADIUS_CARD))
                .border_1()
                .border_color(rgb(pal().hairline))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .children(disclosure)
                        .child(
                            summary_row(gpui_kit::FontWeight::NORMAL).text_color(rgb(pal().muted)),
                        ),
                )
        };
        let items: Vec<&QueuedMessage> = if collapsed {
            Vec::new()
        } else {
            c.pending.iter().chain(c.queue.iter()).collect()
        };
        for (index, q) in items.into_iter().enumerate() {
            let edit = q.clone();
            let remove = q.id.clone();
            let active = q.run.as_ref().is_some_and(|id| {
                self.worker
                    .as_ref()
                    .and_then(Worker::run_identity)
                    .map(|run| &run.run_id)
                    == Some(id)
            });
            // Only waiting items reorder: up goes before the previous one,
            // down before the one after next (or to the end).
            let position = c.queue.iter().position(|i| i.id == q.id);
            let up = position
                .and_then(|i| i.checked_sub(1))
                .map(|i| c.queue[i].id.clone());
            let down = position
                .filter(|i| i + 1 < c.queue.len())
                .map(|i| c.queue.get(i + 2).map(|next| next.id.clone()));
            let state = if active {
                "Running"
            } else if c.pending.as_ref().is_some_and(|p| p.id == q.id) {
                "Next"
            } else {
                "Waiting"
            };
            // Only items from other chats name their chat.
            let label = if here.as_deref() == Some(q.thread.as_str()) {
                format!("{state} · {}", short(&q.text))
            } else {
                format!(
                    "{state} · {} · {}",
                    short(&self.thread_title(&q.thread)),
                    short(&q.text)
                )
            };
            let reorder = |id: String, direction: &'static str, before: Option<String>| {
                let item = q.id.clone();
                components::icon_button(
                    id,
                    if direction == "up" {
                        IconName::ArrowUp
                    } else {
                        IconName::ArrowDown
                    },
                    if direction == "up" {
                        "Move up"
                    } else {
                        "Move down"
                    },
                )
                .flex_shrink_0()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Err(e) = this.workspace.move_queued(
                        &format!("move:{}", uuid::Uuid::new_v4()),
                        &item,
                        before.as_deref(),
                    ) {
                        this.fail(e.to_string());
                    }
                    this.refresh();
                    cx.notify();
                }))
            };
            queue = queue.child(
                div()
                    .id(format!("queue-card-{}", q.id))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_1()
                    .child(
                        div()
                            .id(format!("queue-label-{index}"))
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(SMALL_TEXT)
                            .text_color(rgb(pal().ink))
                            .aria_label(label.clone())
                            .child(label)
                            .test_support(),
                    )
                    .child(
                        pill(format!("queue-edit-{index}"), "Edit")
                            .flex_shrink_0()
                            .disabled(active)
                            .when(active, |button| {
                                let (announced, tooltip) = components::unavailable(
                                    "Edit",
                                    Some("this message is running"),
                                );
                                button.accessibility_label(announced).tooltip(tooltip)
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.begin_queued_edit(&edit, window, cx)
                            })),
                    )
                    .child(
                        pill(format!("queue-remove-{index}"), "Remove")
                            .flex_shrink_0()
                            .disabled(active)
                            .when(active, |button| {
                                let (announced, tooltip) = components::unavailable(
                                    "Remove",
                                    Some("this message is running; Stop it first"),
                                );
                                button.accessibility_label(announced).tooltip(tooltip)
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Err(e) = this.workspace.remove_queued(
                                    &format!("remove:{}", uuid::Uuid::new_v4()),
                                    &remove,
                                ) {
                                    this.fail(e.to_string());
                                }
                                this.refresh();
                                cx.notify();
                            })),
                    )
                    .when_some(up, |row, before| {
                        row.child(reorder(format!("queue-up-{index}"), "up", Some(before)))
                    })
                    .when_some(down, |row, before| {
                        row.child(reorder(format!("queue-down-{index}"), "down", before))
                    })
                    .test_support(),
            );
        }
        Some(queue.test_support().into_any_element())
    }
    /// Edit a waiting message in the composer. The draft the composer held
    /// for that chat is kept aside and returns after Save queued or Cancel
    /// edit, so editing never loses unsent text.
    pub(super) fn begin_queued_edit(
        &mut self,
        edit: &QueuedMessage,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_chat(ChatTarget::Thread(edit.thread.clone()), window, cx);
        let destination = self.destination();
        if self.queued_edit_for_current_chat().is_none() {
            let draft = self.composer.read(cx).value().to_string();
            self.conversation.stash(destination, draft);
        }
        self.editing_queue = Some(QueuedEdit {
            item: edit.id.clone(),
            thread: edit.thread.clone(),
        });
        self.composer
            .update(cx, |e, cx| e.set_value(edit.text.clone(), window, cx));
        self.composer.focus_handle(cx).focus(window, cx);
        cx.notify();
    }
    /// The chat's private attachments: coverage, Inspect and Keep in BRN,
    /// which names its destination before it promotes anything (§6.8).
    pub(super) fn attachments_panel(&self, cx: &mut Context<Self>) -> Option<gpui_kit::AnyElement> {
        let ChatTarget::Thread(chat) = &self.chat else {
            return None;
        };
        let mut panel = div().id("chat-attachments").flex().flex_col().gap_1();
        let mut any = false;
        for r in self.records.iter().filter(|r| {
            !r.archived && matches!(&r.data, RecordData::Note(n) if n.chat.as_ref() == Some(chat))
        }) {
            any = true;
            let open = r.id.clone();
            let keep = (r.id.clone(), r.version, one_line(&title(r)));
            let coverage = match &r.data {
                RecordData::Note(n) => n
                    .import
                    .as_ref()
                    .and_then(|i| i.report.as_ref())
                    .map(documents::report_summary)
                    .unwrap_or_else(|| "Coverage unchecked".into()),
                _ => String::new(),
            };
            panel = panel.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .rounded(px(theme::metrics::RADIUS_CONTROL))
                    .border_1()
                    .border_color(rgb(pal().hairline))
                    .child(
                        Icon::new(IconName::FileText)
                            .with_size(Size::Small)
                            .flex_shrink_0()
                            .text_color(rgb(pal().muted)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(SMALL_TEXT)
                            .child(format!("Attachment · {}", short(&title(r)))),
                    )
                    .child(
                        metadata(coverage.clone())
                            .flex_shrink_0()
                            .id(format!("attachment-coverage-{}", r.id))
                            .aria_label(coverage)
                            .test_support(),
                    )
                    .child(
                        pill(format!("attachment-open-{}", r.id), "Inspect")
                            .flex_shrink_0()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_note(&open, None, window, cx)
                            })),
                    )
                    .child(
                        pill(format!("attachment-keep-{}", r.id), "Keep in BRN…")
                            .flex_shrink_0()
                            .tooltip("Move this attachment into your saved library; asks first")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.confirm_keep_in_brn(keep.clone(), window, cx)
                            })),
                    ),
            );
        }
        any.then(|| panel.test_support().into_any_element())
    }
    /// Keep in BRN names the retained note and its destination before the
    /// version-bound owner change; Undo stays in Recent changes.
    pub(super) fn confirm_keep_in_brn(
        &mut self,
        (note, version, name): (String, u64, String),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = cx.entity().downgrade();
        let body = format!(
            "\u{201c}{}\u{201d} moves from this chat into your saved library, where general chats and search can use it. It keeps its content, figures, protection and source. You can undo this in Recent changes.",
            short(&name)
        );
        self.open_dialog(window, cx, move |dialog, _, _| {
            let (target, note) = (target.clone(), note.clone());
            dialog
                .title("Keep in BRN?")
                .on_ok(|_, _, _| false)
                .child(
                    div()
                        .id("keep-in-brn-body")
                        .flex()
                        .flex_col()
                        .gap_2()
                        .text_size(UI_TEXT)
                        .aria_label(body.clone())
                        .child(metadata("Destination: Saved library"))
                        .child(body.clone())
                        .test_support(),
                )
                .footer(
                    components::dialog_footer()
                        .child(
                            button("keep-in-brn-cancel")
                                .label("Cancel")
                                .outline()
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        )
                        .child(
                            button("keep-in-brn-confirm")
                                .label("Keep in BRN")
                                .primary()
                                .on_click(move |_, window, cx| {
                                    let note = note.clone();
                                    let _ = target.update(cx, |this, cx| {
                                        this.keep_in_brn(&note, version, cx)
                                    });
                                    window.close_dialog(cx);
                                }),
                        ),
                )
        });
    }
    fn keep_in_brn(&mut self, note: &str, version: u64, cx: &mut Context<Self>) {
        match self
            .workspace
            .keep_in_brn(&format!("keep:{}", uuid::Uuid::new_v4()), note, version)
        {
            Ok(outcome) => self.report_outcome(&outcome),
            Err(e) => self.fail(e.to_string()),
        }
        self.refresh();
        cx.notify();
    }
    pub(super) fn attach_chat_path(
        &mut self,
        paths: &[PathBuf],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if paths.len() != 1 {
            self.warn("Attach one supported document at a time.");
            cx.notify();
            return;
        }
        if self.worker.is_some() {
            self.warn("Finish or stop work before attaching a file.");
            cx.notify();
            return;
        }
        let text = self.composer.read(cx).value().to_string();
        match self.composer_chat(if text.trim().is_empty() {
            "New chat"
        } else {
            &text
        }) {
            Ok(chat) => {
                self.set_chat(ChatTarget::Thread(chat.clone()), window, cx);
                self.composer
                    .update(cx, |e, cx| e.set_value(text, window, cx));
                self.convert_file(
                    None,
                    paths[0].clone(),
                    ImportIntent::FullNote,
                    Some(chat),
                    cx,
                );
            }
            Err(e) => {
                self.fail(e.to_string());
                cx.notify();
            }
        }
    }
    pub(super) fn choose_chat_attachment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.worker.is_some() {
            self.warn("Finish or stop work before attaching a file.");
            cx.notify();
            return;
        }
        let text = self.composer.read(cx).value().to_string();
        match self.composer_chat(if text.trim().is_empty() {
            "New chat"
        } else {
            &text
        }) {
            Ok(id) => {
                self.set_chat(ChatTarget::Thread(id.clone()), window, cx);
                self.composer
                    .update(cx, |e, cx| e.set_value(text, window, cx));
                self.choose_import(ImportIntent::FullNote, Some(id), window, cx);
            }
            Err(e) => {
                self.fail(e.to_string());
                cx.notify();
            }
        }
    }
    pub(super) fn conversation(&self, id: &str, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let messages = self.workspace.messages(id).unwrap_or_default().into_iter().filter(|record| {
            !matches!(&record.data, RecordData::Message(message) if matches!(message.role.as_str(), "writing-request" | "writing-addressed"))
        }).collect::<Vec<_>>();
        let mut keys = messages.iter().map(|r| r.id.clone()).collect::<Vec<_>>();
        keys.push(format!("details-{id}"));
        let state = {
            let mut states = self.scrollers.borrow_mut();
            let slot = states.entry(id.into()).or_insert_with(|| ChatScroller {
                state: cx.new(|cx| MessageScrollerState::new(0, cx)),
                keys: vec![],
                signature: String::new(),
            });
            if slot.keys != keys {
                let same = slot
                    .keys
                    .iter()
                    .zip(&keys)
                    .take_while(|(a, b)| a == b)
                    .count();
                let old_len = slot.keys.len();
                let new_len = keys.len();
                slot.state.update(cx, |s, cx| {
                    s.splice(same..old_len, new_len - same, cx);
                });
                slot.keys = keys;
            }
            // Streaming changes the last row's height, preserving a scrolled-up anchor.
            let signature = format!(
                "{}:{:?}",
                self.stream,
                self.records.iter().find(|r| r.id == id).map(|r| r.version),
            );
            if signature != slot.signature {
                slot.state.update(cx, |s, cx| {
                    let count = s.item_count();
                    if count > 0 {
                        s.remeasure_items(count - 1..count, cx);
                    }
                });
                slot.signature = signature;
            }
            // A search hit opened this chat: show its Message once.
            let reveal = self
                .reveal_message
                .borrow()
                .clone()
                .filter(|(chat, _)| chat == id);
            if let Some((_, message)) = reveal {
                if let Some(index) = messages.iter().position(|record| record.id == message) {
                    slot.state.update(cx, |s, cx| {
                        s.scroll_to_item(index, cx);
                    });
                }
                self.reveal_message.borrow_mut().take();
            }
            slot.state.clone()
        };
        let weak = cx.entity().downgrade();
        let chat = id.to_owned();
        MessageScroller::new(format!("conversation-{id}"), state, move |index, _, cx| {
            weak.update(cx, |this, cx| {
                if let Some(message) = messages.get(index) {
                    this.message_row(message, cx)
                } else {
                    this.conversation_detail(&chat, cx)
                }
            })
            .unwrap_or_else(|_| div().into_any_element())
        })
        // The kit's jump control is icon-only; name it for assistive tech.
        .with_jump_button_renderer(|button| button.accessibility_label("Jump to latest"))
        .into_any_element()
    }
    fn message_row(&self, record: &Record, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let RecordData::Message(m) = &record.data else {
            return div().into_any_element();
        };
        let p = pal();
        let key = self
            .history
            .iter()
            .find(|r| r.writes.iter().any(|w| w.after.id == record.id))
            .and_then(|r| {
                r.writes.iter().find_map(|w| match w.after.data {
                    RecordData::Run(_) => Some(format!("response-{}", w.after.id)),
                    _ => None,
                })
            })
            .unwrap_or_else(|| format!("message-{}", record.id));
        // One readable column (CONVERSATION_WIDTH of text plus gutters),
        // centred when the conversation is the primary view.
        let mut column = transcript_column(self.centered(TileKind::Chat))
            .gap_1()
            .py_2();
        match m.role.as_str() {
            // The owner's words sit in a soft bubble at the column's end.
            "owner" => {
                column = column.items_end().child(
                    div()
                        .id(format!("owner-bubble-{}", record.id))
                        .max_w(gpui_kit::relative(0.85))
                        .min_w_0()
                        .px_4()
                        .py_2()
                        .rounded(px(theme::metrics::RADIUS_BUBBLE))
                        .bg(rgb(bubble_fill()))
                        .text_color(rgb(p.ink))
                        .aria_label("You")
                        .child(self.safe_message(key, m.text.clone()))
                        .test_support(),
                );
            }
            // BRN's answer is flat prose through the maintained renderer,
            // with its actions below it (visible, never hover-only).
            "assistant" | "assistant-partial" => {
                let copy = m.text.clone();
                let decision = record.clone();
                let save_decision = m.role == "assistant"
                    && !m.text.trim().is_empty()
                    && !self.thread_resolved(&m.thread);
                if m.role == "assistant-partial" {
                    column = column.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(components::tone_mark(
                                format!("partial-{}", record.id),
                                Tone::Warning,
                            ))
                            .child(metadata("Partial reply · interrupted before it finished")),
                    );
                }
                column = column.child(self.safe_message(key, m.text.clone())).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .h(components::tokens::ICON_CONTROL)
                        .child(
                            components::icon_button(
                                format!("answer-copy-{}", record.id),
                                IconName::Copy,
                                "Copy reply",
                            )
                            .flex_shrink_0()
                            .on_click(cx.listener(
                                move |_, _, _, cx| {
                                    cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                                        copy.clone(),
                                    ))
                                },
                            )),
                        )
                        .when(save_decision, |controls| {
                            controls.child(
                                pill(
                                    format!("answer-decision-{}", record.id),
                                    "Save as decision…",
                                )
                                .h(components::tokens::ICON_CONTROL)
                                .text_size(SMALL_TEXT)
                                .flex_shrink_0()
                                .tooltip(
                                    "Review and save this reply as a protected decision in BRN",
                                )
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.decision_dialog(decision.clone(), window, cx)
                                    },
                                )),
                            )
                        }),
                );
            }
            CAPTURE_REQUEST_ROLE => {
                column = column.child(metadata(
                    "You asked for Capture outcomes · proposals wait for review; the chat stays open",
                ));
            }
            "writing-request" => {
                column = column.child(metadata("You · Rewrite with AI · saved context"));
            }
            _ => {}
        }
        div()
            .id(format!("chat-message-{}", record.id))
            .flex()
            .flex_col()
            .w_full()
            .when(self.centered(TileKind::Chat), |row| row.items_center())
            .child(column)
            .test_support()
            .into_any_element()
    }
}

/// One transcript row's column: the conversation measure with gutters,
/// narrower gutters beside a document.
pub(super) fn transcript_column(primary: bool) -> gpui_kit::Div {
    let gutter = if primary {
        components::tokens::SPACE_6
    } else {
        components::tokens::SPACE_4
    };
    div()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .max_w(px(theme::metrics::CONVERSATION_WIDTH) + gutter * 2.)
        .px(gutter)
}

/// The owner bubble's soft fill: one step off the canvas in either theme.
fn bubble_fill() -> u32 {
    let p = pal();
    if p.dark { p.surface_raised } else { p.sidebar }
}

/// Conversation presentation state the host keeps per window session.
#[derive(Default)]
pub(super) struct ConversationView {
    /// The header's Activity disclosure is open.
    pub activity_open: bool,
    /// The composer was empty at its last change (re-render on a flip).
    pub composer_empty: bool,
    /// The draft a queued edit set aside, with its destination.
    stash: Option<(ComposerDestination, String)>,
}
impl ConversationView {
    pub(super) fn stash(&mut self, destination: ComposerDestination, text: String) {
        self.stash = Some((destination, text));
    }
    /// The stashed draft for `destination`, or empty; another
    /// destination's stash stays.
    pub(super) fn take_stash(&mut self, destination: &ComposerDestination) -> String {
        match self.stash.take() {
            Some((saved, text)) if &saved == destination => text,
            other => {
                self.stash = other;
                String::new()
            }
        }
    }
}
