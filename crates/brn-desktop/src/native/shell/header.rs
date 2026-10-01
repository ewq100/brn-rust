use super::*;

impl Desktop {
    pub(super) fn render_header(
        &mut self,
        resolved: &ResolvedLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = self.palette();
        let tone = match self.phase {
            Phase::Idle => p.green,
            Phase::Failed(_) => p.amber,
            Phase::Opening => p.muted,
            Phase::Running { .. } | Phase::Cancelling { .. } => p.cyan,
        };
        let mut bar = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_1()
            .min_w(px(0.))
            .pr_2()
            .child(div().flex_shrink_0().child("brn"))
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(color(p.muted))
                    .child("/ workspace"),
            )
            .child(
                div()
                    .min_w(px(0.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(color(tone))
                    .child(self.phase_status()),
            )
            .child(div().flex_1());
        if matches!(self.phase, Phase::Running { .. }) {
            bar = bar.child(
                Button::new("cancel")
                    .label("Cancel")
                    .compact()
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_running(cx))),
            );
        }
        bar = bar
            .child(
                Button::new("toggle-focus")
                    .label("Focus")
                    .compact()
                    .selected(self.layout.focus)
                    .toggled(self.layout.focus)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_focus(cx))),
            )
            .child(
                Button::new("toggle-vault")
                    .label("Vault")
                    .compact()
                    .selected(resolved.vault == RailDisplay::Open)
                    .toggled(resolved.vault == RailDisplay::Open)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_rail(Rail::Vault, cx))),
            );
        if resolved.history != RailDisplay::Open {
            bar = bar.child(
                Button::new("header-settings")
                    .label("⚙")
                    .compact()
                    .tooltip("Settings (⌘,)")
                    .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx))),
            );
        }
        TitleBar::new().child(bar)
    }
    pub(super) fn render_status_line(&self) -> impl IntoElement {
        let p = self.palette();
        let mut line = div()
            .id("status-line")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .max_h(px(96.))
            .overflow_y_scroll()
            .px_3()
            .py_1()
            .gap_1()
            .border_b_1()
            .border_color(color(p.line))
            .bg(color(p.paper))
            .text_color(color(p.muted))
            .child(self.message.clone());
        if matches!(self.phase, Phase::Failed(_)) {
            line = line.child("Workspace unavailable. Review the error above, correct the workspace, then relaunch.");
        }
        if !self.progress.is_empty()
            && matches!(self.phase, Phase::Running { .. } | Phase::Cancelling { .. })
        {
            line = line.child(format!("Progress: {}", self.progress));
        }
        if let Some(note) = &self.layout_note {
            line = line.child(note.clone());
        }
        line
    }
}
