//! Native Dashboard views over checked workflow observations and explicit commands.
use super::*;
use brn_workflow::dashboard::DashboardFilter;
use gpui_kit::{
    AnyElement, TestSupportExt,
    base::Disableable,
    component::{Selectable, WindowExt},
};

pub(super) struct DashboardPane {
    pub proof: Entity<EditorState>,
    pub scroll: ScrollHandle,
    pub attempt: Option<Uuid>,
}
impl DashboardPane {
    pub(super) fn new(window: &mut Window, cx: &mut Context<Desktop>) -> Self {
        Self {
            proof: cx.new(|cx| EditorState::new(window, cx).default_value("")),
            scroll: ScrollHandle::new(),
            attempt: None,
        }
    }
}
pub(super) fn proof_widget(editor: &Entity<EditorState>) -> Editor {
    Editor::new(editor)
        .h_full()
        .readonly(true)
        .aria_label("Complete Action and retained completion proof")
}
pub(super) fn proof_text(ai: &crate::ai::AiState, operation: Option<Uuid>) -> String {
    let mut text = ai.dashboard.selected.as_ref().map_or_else(
        || "No Action selected.".into(),
        |entry| {
            format!(
                "Complete Action and dependency observation:\n{}",
                serde_json::to_string_pretty(entry).expect("checked Action JSON")
            )
        },
    );
    let attempt = operation
        .and_then(|operation| {
            ai.dashboard
                .attempts
                .iter()
                .find(|attempt| attempt.request.operation_id == operation)
        })
        .or_else(|| ai.dashboard.attempts.last());
    if let Some(attempt) = attempt {
        text.push_str("\n\nExact retained completion request:\n");
        text.push_str(
            &serde_json::to_string_pretty(&attempt.request).expect("completion request JSON"),
        );
        if let Some(error) = &attempt.error {
            text.push_str(&format!(
                "\n\nCompletion response {:?}: {}",
                error.kind, error.message
            ));
        }
        if let Some(receipt) = &attempt.receipt {
            text.push_str("\n\nAcknowledged completion receipt:\n");
            text.push_str(&serde_json::to_string_pretty(receipt).expect("checked completion JSON"));
        }
    }
    text
}
fn copy(id: String, label: &str, text: String) -> Button {
    Button::new(id)
        .label(label.to_owned())
        .compact()
        .on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text.clone()))
        })
}
impl Desktop {
    pub(super) fn sync_dashboard_widgets(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let text = proof_text(self.ai.as_ref().unwrap(), self.dashboard.attempt);
        if self.dashboard.proof.read(cx).value().as_ref() != text {
            self.dashboard
                .proof
                .update(cx, |editor, cx| editor.set_value(text, window, cx));
        }
    }
    fn dashboard_blocked(&self) -> bool {
        !self.ai.as_ref().unwrap().ready
            || self.open_doc != Some(DocRef::Dashboard)
            || self.simple_transition.is_some()
            || self.closing.is_some()
            || self.closed
            || self.close_failed
    }
    fn dashboard_page(
        &mut self,
        filter: DashboardFilter,
        older: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dashboard_blocked() || window.has_active_dialog(cx) {
            return;
        }
        if let Some(command) = self.ai.as_mut().unwrap().refresh_dashboard(filter, older) {
            self.simple_send(command, cx);
            self.dashboard.scroll.set_offset(point(px(0.), px(0.)));
        }
        self.sync_dashboard_widgets(window, cx);
        cx.notify();
    }
    pub(super) fn open_complete_action(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.dashboard_blocked() || window.has_active_dialog(cx) {
            return;
        }
        let Some(capture) = self.ai.as_ref().unwrap().capture_action_completion() else {
            return;
        };
        let desktop = cx.entity().downgrade();
        window.open_dialog(cx, move |dialog, _, cx| {
            let current = desktop.upgrade();
            let disabled = current.as_ref().is_none_or(|desktop| {
                let this = desktop.read(cx);
                this.dashboard_blocked() || !this.ai.as_ref().unwrap().action_completion_capture_current(&capture)
            });
            let mut body = div().id("exact-completion-capture").test_support().flex().flex_col().gap_2()
                .child("Confirm that this exact identified real-world Action is complete. Its fields and immutable origin are captured below. Follow-up work is a new related Action.")
                .child(format!("Completion operation {}", capture.request().operation_id))
                .child(action_review::before_body(0, &capture.request().before));
            if disabled {
                body = body.child("The captured selection changed or current work prevents admission. Close this confirmation and inspect the current Action.");
            }
            if let Some(current) = &current {
                body = body.child(current.read(cx).ai.as_ref().unwrap().notice.clone());
            }
            let capture = capture.clone();
            let desktop = desktop.clone();
            dialog.title("Complete captured Action").w(px(840.)).child(body)
                .child(Button::new("confirm-exact-action-completion").label("Complete captured Action").disabled(disabled)
                    .on_click(move |_, window, cx| {
                        let mut admitted = false;
                        let _ = desktop.update(cx, |this, cx| {
                            if this.dashboard_blocked() { return; }
                            let Some(command) = this.ai.as_mut().unwrap().confirm_action_completion(&capture) else {
                                this.ai.as_mut().unwrap().notice = "Completion was not admitted; the captured selection is no longer current.".into();
                                cx.notify();
                                return;
                            };
                            let operation = command.0;
                            this.dashboard.attempt = Some(operation);
                            this.simple_send(command, cx);
                            admitted = this.ai.as_ref().unwrap().pending.contains_key(&operation);
                            this.sync_dashboard_widgets(window, cx);
                        });
                        if admitted { window.close_dialog(cx); }
                    }))
        });
    }
    pub(super) fn render_dashboard(&self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let state = &ai.dashboard;
        let blocked = self.dashboard_blocked();
        let loading = ai.dashboard_loading();
        let mut filters = div().flex().flex_wrap().gap_1();
        for (index, (label, filter)) in [
            ("Active", DashboardFilter::Active),
            ("Open", DashboardFilter::Open),
            ("Waiting", DashboardFilter::Waiting),
            ("Blocked", DashboardFilter::Blocked),
            ("Completed", DashboardFilter::Completed),
            ("Overdue", DashboardFilter::Overdue),
            ("Follow-up", DashboardFilter::FollowUp),
            ("All", DashboardFilter::All),
        ]
        .into_iter()
        .enumerate()
        {
            filters = filters.child(
                Button::new(format!("dashboard-filter-{index}"))
                    .label(label)
                    .compact()
                    .selected(state.filter == filter)
                    .disabled(blocked || loading)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.dashboard_page(filter, false, window, cx)
                    })),
            );
        }
        let filter = state.filter;
        let older = state
            .page
            .as_ref()
            .is_some_and(|page| page.next_before.is_some());
        let mut content = div()
            .id("dashboard-scroll")
            .track_scroll(&self.dashboard.scroll)
            .overflow_y_scroll()
            .vertical_scrollbar(&self.dashboard.scroll)
            .flex_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .gap_2()
            .p_2()
            .child("Dashboard · Actions")
            .child(filters)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(
                        Button::new("refresh-dashboard")
                            .label("Refresh today")
                            .compact()
                            .disabled(blocked || loading)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.dashboard_page(filter, false, window, cx)
                            })),
                    )
                    .child(
                        Button::new("older-dashboard")
                            .label("Older page")
                            .compact()
                            .disabled(blocked || loading || !older)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.dashboard_page(filter, true, window, cx)
                            })),
                    ),
            );
        if loading {
            content = content.child("Loading Dashboard…");
        }
        if let Some(error) = &state.error {
            content = content.child(format!("Dashboard: {error}"));
        }
        if let Some(page) = &state.page {
            let counts = &page.counts;
            let label = format!(
                "As of {} · Open {} · Waiting {} · Blocked {} · Completed {} · Overdue {} · Follow-up {}",
                page.as_of,
                counts.open,
                counts.waiting,
                counts.blocked,
                counts.completed,
                counts.overdue,
                counts.follow_up
            );
            content = content.child(div().id("dashboard-counts").test_support().aria_label(label.clone()).child(label))
                .child("Counts cover all retained Actions. Overdue starts the day after the due date; follow-up starts on its named day. Date signals may overlap. Dependencies are observations, not automatic state changes.");
            if page.entries.is_empty() {
                content = content.child("No Actions in this page.");
            }
            for entry in &page.entries {
                let id = entry.action.origin.id;
                let mut signals = Vec::new();
                if entry.overdue {
                    signals.push("overdue");
                }
                if entry.follow_up {
                    signals.push("follow-up");
                }
                if entry.dependency_blocked {
                    signals.push("unfinished/missing dependency");
                }
                content = content.child(
                    Button::new(format!("dashboard-action-{id}"))
                        .label(format!(
                            "{:?} · {} · {}",
                            entry.action.data.state,
                            compact_title(&entry.action.data.title),
                            signals.join(" · ")
                        ))
                        .selected(
                            state
                                .selected
                                .as_ref()
                                .is_some_and(|entry| entry.action.origin.id == id),
                        )
                        .disabled(blocked || loading)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if this.dashboard_blocked() || window.has_active_dialog(cx) {
                                return;
                            }
                            this.ai.as_mut().unwrap().select_dashboard_action(id);
                            this.sync_dashboard_widgets(window, cx);
                            cx.notify();
                        })),
                );
            }
        }
        if state.selected.is_some() {
            content = content.child(
                Button::new("complete-selected-action")
                    .label("Complete…")
                    .compact()
                    .disabled(blocked || !ai.action_completion_available())
                    .on_click(
                        cx.listener(|this, _, window, cx| this.open_complete_action(window, cx)),
                    ),
            );
        }
        if let Some(entry) = state.selected.as_ref().filter(|entry| {
            entry.action.data.state == brn_workflow::actions::ActionState::Completed
        }) {
            let id = entry.action.origin.id;
            content = content.child(
                Button::new("new-related-action")
                    .label("New related follow-up…")
                    .disabled(blocked || loading || ai.application_busy())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if this.dashboard_blocked()
                            || this.ai.as_ref().unwrap().dashboard_loading()
                            || window.has_active_dialog(cx)
                        {
                            return;
                        }
                        if this
                            .ai
                            .as_ref()
                            .unwrap()
                            .dashboard
                            .selected
                            .as_ref()
                            .is_some_and(|entry| {
                                entry.action.origin.id == id
                                    && entry.action.data.state
                                        == brn_workflow::actions::ActionState::Completed
                            })
                        {
                            this.simple_leave(simple::EditorTransition::ActionDraft(Some(id)), cx);
                        }
                    })),
            );
        }
        content = content
            .child(
                div()
                    .h(px(240.))
                    .flex_shrink_0()
                    .child(proof_widget(&self.dashboard.proof)),
            )
            .child(copy(
                "copy-dashboard-proof".into(),
                "Copy full displayed Action / completion proof",
                proof_text(ai, self.dashboard.attempt),
            ));
        if !state.attempts.is_empty() {
            content = content.child("Retained completion requests in this app session");
        }
        for attempt in &state.attempts {
            let operation = attempt.request.operation_id;
            let status = if attempt.receipt.is_some() {
                "Acknowledged Completed"
            } else if attempt.error.is_some() {
                "Response failed; outcome not inferred"
            } else {
                "Awaiting acknowledgement"
            };
            content = content.child(format!(
                "{} · Action {} · {status}",
                operation, attempt.request.before.origin.id
            ));
            if let Some(error) = &attempt.error {
                content = content.child(format!("{:?}: {}", error.kind, error.message));
            }
            content = content
                .child(
                    Button::new(format!("inspect-completion-{operation}"))
                        .label("Inspect retained request / receipt")
                        .compact()
                        .disabled(blocked)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if this.dashboard_blocked() || window.has_active_dialog(cx) {
                                return;
                            }
                            this.dashboard.attempt = Some(operation);
                            this.sync_dashboard_widgets(window, cx);
                            cx.notify();
                        })),
                )
                .child(copy(
                    format!("copy-completion-request-{operation}"),
                    "Copy exact completion request",
                    serde_json::to_string_pretty(&attempt.request)
                        .expect("completion request JSON"),
                ));
            if attempt.error.is_some() && attempt.receipt.is_none() {
                content = content.child(
                    Button::new(format!("retry-completion-{operation}"))
                        .label("Retry exact retained completion")
                        .compact()
                        .disabled(
                            blocked
                                || ai.application_busy()
                                || ai.active.is_some()
                                || ai.rewrite.is_some(),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if this.dashboard_blocked() || window.has_active_dialog(cx) {
                                return;
                            }
                            if let Some(command) =
                                this.ai.as_mut().unwrap().retry_action_completion(operation)
                            {
                                this.dashboard.attempt = Some(operation);
                                this.simple_send(command, cx);
                                this.sync_dashboard_widgets(window, cx);
                            }
                        })),
                );
            }
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(content.test_support())
            .into_any_element()
    }
}
