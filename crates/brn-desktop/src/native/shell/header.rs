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
        let vault = ai
            .vault_root
            .as_ref()
            .and_then(|root| root.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "No vault".into());
        let mut bar = div()
            .flex()
            .items_center()
            .gap(px(tokens::space::SM))
            .flex_1()
            .min_w(px(0.))
            .pr_2()
            .child(
                div()
                    .flex_shrink_0()
                    .font_family(tokens::MONO_FONT)
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .text_color(color(p.cyan))
                    .child("brn"),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_color(color(p.muted))
                    .child(vault),
            )
            .child(div().min_w(px(0.)).overflow_hidden().child(ui::badge(
                self.phase_status(),
                tone,
                p,
            )))
            .child(div().flex_1());
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
                    .label("Focus")
                    .ghost()
                    .small()
                    .tooltip("Hide both rails (⇧⌘↩)")
                    .selected(self.layout.focus)
                    .toggled(self.layout.focus)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_focus(cx))),
            )
            .child(
                Button::new("toggle-vault")
                    .label("Vault")
                    .ghost()
                    .small()
                    .tooltip("Show or hide the vault rail (⌥⌘0)")
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
    pub(super) fn render_status_line(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
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
        line
    }
}
