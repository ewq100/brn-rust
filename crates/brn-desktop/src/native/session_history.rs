//! Explicit reversible session organization; no navigation or inference side effects.
use super::*;
use brn_workflow::conversations::{ConversationFilter, ConversationState};
use gpui_kit::{AnyElement, TestSupportExt, base::Disableable, component::Selectable};

impl Desktop {
    pub(super) fn render_session_controls(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let ai = self.ai.as_ref().unwrap();
        let frozen = !ai.ready || ai.session_change_pending();
        let mut controls = div()
            .id("session-lifecycle-controls")
            .flex()
            .flex_col()
            .gap_1();
        let mut filters = div().flex().gap_1();
        for (id, label, filter) in [
            ("sessions-active", "Active", ConversationFilter::Active),
            (
                "sessions-archived",
                "Archived",
                ConversationFilter::Archived,
            ),
        ] {
            filters = filters.child(
                Button::new(id)
                    .label(label)
                    .selected(ai.session_history.filter == filter)
                    .disabled(frozen)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(command) =
                            this.ai.as_mut().unwrap().select_session_filter(filter)
                        {
                            this.simple_send(command, cx);
                        }
                    })),
            );
        }
        filters = filters.child(
            Button::new("refresh-session-history")
                .label("↻")
                .tooltip("Refresh history")
                .disabled(frozen)
                .on_click(cx.listener(|this, _, _, cx| {
                    if this
                        .ai
                        .as_ref()
                        .is_some_and(|ai| ai.ready && !ai.session_change_pending())
                    {
                        let command = this.ai.as_mut().unwrap().refresh_session_summaries();
                        this.simple_send(command, cx);
                    }
                })),
        );
        controls = controls.child(filters);
        if ai.conversation.is_some() {
            let lifecycle = ai.selected_session_lifecycle();
            let archived =
                lifecycle.is_some_and(|value| value.state == ConversationState::Archived);
            let label = if ai.session_change_pending() {
                "Changing session state…"
            } else if archived {
                "Archived — restore to continue"
            } else if lifecycle.is_some() {
                "Active session"
            } else {
                "Session state unavailable; select or refresh history"
            };
            controls = controls.child(
                div()
                    .id("selected-session-lifecycle")
                    .aria_label(label)
                    .text_xs()
                    .child(label)
                    .test_support(),
            );
            let (id, label, target) = if archived {
                (
                    "restore-session",
                    "Restore session",
                    ConversationState::Active,
                )
            } else {
                (
                    "archive-session",
                    "Archive session",
                    ConversationState::Archived,
                )
            };
            controls = controls.child(
                Button::new(id)
                    .label(label)
                    .disabled(!ai.can_change_session_lifecycle())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(command) = this
                            .ai
                            .as_mut()
                            .unwrap()
                            .set_selected_session_lifecycle(target)
                        {
                            this.simple_send(command, cx);
                        }
                    })),
            );
        }
        if let Some(error) = &ai.session_history.error {
            controls = controls.child(
                div()
                    .id("session-lifecycle-error")
                    .aria_label(error.clone())
                    .text_xs()
                    .child(error.clone())
                    .test_support(),
            );
        }
        controls.into_any_element()
    }
}
