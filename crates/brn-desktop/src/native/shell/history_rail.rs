use super::*;

impl Desktop {
    pub(super) fn render_history_rail(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let open_draft = self
            .draft_state
            .as_ref()
            .filter(|_| self.open_doc == Some(DocRef::Draft))
            .map(|state| state.id());
        let mut list = div()
            .id("history-rail-list")
            .track_scroll(&self.history_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_1()
            .p_2()
            .child(
                Button::new("new-session")
                    .label("+ New chat")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| this.choose_session(None, cx))),
            )
            .child(section_label("Conversations", p));
        for session in &self.sessions {
            let id = session.id;
            list = list.child(
                Button::new(format!("session-{id}"))
                    .label(format!(
                        "{}… · {} turns{}",
                        &id.to_string()[..8],
                        session.turns,
                        if session.has_thread {
                            " · provider linked"
                        } else {
                            " · recovery needed"
                        }
                    ))
                    .selected(self.selected_session == Some(id))
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_session(Some(id), cx))),
            );
        }
        list = list.child(section_label("Drafts", p));
        for draft in &self.drafts {
            let id = draft.id;
            list = list.child(
                Button::new(format!("history-draft-{id}"))
                    .label(compact_title(&draft.title))
                    .selected(open_draft == Some(id))
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(move |this, _, _, cx| this.open_draft_from_list(id, cx))),
            );
        }
        div()
            .w(px(self.layout.history_w))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(color(p.panel))
            .child(list)
            .child(
                div()
                    .flex_shrink_0()
                    .border_t_1()
                    .border_color(color(p.line))
                    .p_2()
                    .child(Button::new("settings-footer").label("⚙ Settings").on_click(
                        cx.listener(|this, _, window, cx| this.open_settings(window, cx)),
                    )),
            )
    }
}
