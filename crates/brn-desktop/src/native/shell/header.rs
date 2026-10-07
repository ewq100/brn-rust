use super::*;

impl Desktop {
    pub(super) fn render_header(
        &mut self,
        resolved: &ResolvedLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        use super::super::ui::{self, Tone};
        let p = self.palette();
        let ai = self.ai.as_ref().unwrap();
        let running = ai.active.is_some() || ai.rewrite.is_some();
        let tone = if running || self.closing.is_some() {
            Tone::Ai
        } else if ai.startup_failed {
            Tone::Danger
        } else if ai.ready {
            Tone::Success
        } else {
            Tone::Neutral
        };
        let running = running || self.closing.is_some();
        let mut bar = div()
            .flex()
            .items_center()
            .gap(px(tokens::space::SM))
            .flex_1()
            .min_w(px(0.))
            .pr_2()
            .child(
                Button::new("toggle-history")
                    .icon(gpui_kit::assets::IconName::PanelLeft)
                    .ghost()
                    .small()
                    .tooltip("Show or hide the sidebar (⌘0)")
                    .selected(resolved.history == RailDisplay::Open)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_rail(Rail::History, cx))),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .font_family(tokens::MONO_FONT)
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .text_color(color(p.cyan))
                    .child("brn"),
            );
        // Ready is the normal state; only other phases earn a badge.
        if tone != Tone::Success {
            bar = bar.child(div().min_w(px(0.)).overflow_hidden().child(ui::badge(
                self.phase_status(),
                tone,
                p,
            )));
        }
        bar = bar.child(div().flex_1());
        if running {
            bar = bar.child(
                Button::new("cancel")
                    .label("Stop")
                    .danger()
                    .small()
                    .tooltip("Stop the running request (⌘.)")
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_running(cx))),
            );
        }
        bar = bar
            .child(
                Button::new("toggle-focus")
                    .icon(gpui_kit::assets::IconName::Maximize)
                    .ghost()
                    .small()
                    .tooltip("Focus: hide both sidebars (⇧⌘↩)")
                    .selected(self.layout.focus)
                    .toggled(self.layout.focus)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_focus(cx))),
            )
            .child(
                Button::new("toggle-vault")
                    .icon(gpui_kit::assets::IconName::PanelRight)
                    .ghost()
                    .small()
                    .tooltip("Vault: browse notes, sources and history (⌥⌘0)")
                    .selected(resolved.vault == RailDisplay::Open)
                    .toggled(resolved.vault == RailDisplay::Open)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_rail(Rail::Vault, cx))),
            );
        if resolved.history != RailDisplay::Open {
            bar = bar.child(
                Button::new("header-settings")
                    .icon(gpui_kit::assets::IconName::Settings)
                    .ghost()
                    .small()
                    .tooltip("Settings (⌘,)")
                    .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx))),
            );
        }
        TitleBar::new().child(bar)
    }
    pub(super) fn render_status_line(&self, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette();
        let ai = self.ai.as_ref();
        let notices = self.layout_note.is_some()
            || ai.is_some_and(|ai| ai.startup_failed || ai.restored.is_some())
            || self.close_failed;
        // Routine readiness needs no strip; keep the chrome quiet.
        if !notices
            && matches!(
                self.message.as_str(),
                "" | "Workspace ready." | "Opening workspace…"
            )
        {
            return div().id("status-line").into_any_element();
        }
        let mut line = div()
            .id("status-line")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .min_h(px(tokens::size::STATUS_STRIP))
            .max_h(px(96.))
            .overflow_y_scroll()
            .justify_center()
            .px_3()
            .py_1()
            .gap_1()
            .border_t_1()
            .border_color(color(p.line))
            .bg(color(p.panel))
            .text_size(px(tokens::text::CAPTION + 0.5))
            .text_color(color(p.muted))
            .child(self.message.clone());
        if self.ai.as_ref().is_some_and(|ai| ai.startup_failed) {
            line = line.child("Workspace unavailable. Review the error above, correct the workspace, then relaunch.");
        }
        if let Some(note) = &self.layout_note {
            line = line.child(note.clone());
        }
        if let Some(ai) = &self.ai {
            if let Some(backup) = &ai.restored {
                line = line.child(format!("Restored user work from {}", backup.display()));
            }
            if self.close_failed {
                line = line.child("Local work is stopped. Copy any unsaved partial text before explicitly closing.");
                line = line.child(
                    Button::new("confirm-unsaved-close")
                        .label("Confirm close without saving partial answer")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.closed = true;
                            this.close_failed = false;
                            window.remove_window();
                            cx.notify();
                        })),
                );
            }
        }
        line.into_any_element()
    }
}
