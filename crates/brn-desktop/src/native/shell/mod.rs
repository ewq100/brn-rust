//! Workspace shell: region composition and layout preference changes.

mod centre;
pub(super) mod divider;
mod header;
mod history_rail;
mod settings;
mod vault_rail;

use super::theme::{self, color};
use super::*;
use crate::layout::{Appearance, CentreMode, Rail, RailDisplay};
use crate::tokens::{self, Palette};
use gpui_kit::{AnyElement, MouseButton, component::Selectable, relative};

pub(super) fn approval_tag(approval: Approval) -> &'static str {
    match approval {
        Approval::Approved => "✓ approved",
        Approval::Draft => "○ not approved",
        Approval::Withdrawn => "– withdrawn",
    }
}

fn section_label(text: &str, p: Palette) -> impl IntoElement {
    div()
        .pt_2()
        .text_size(px(10.))
        .text_color(color(p.muted))
        .child(text.to_uppercase())
}

impl Desktop {
    pub(super) fn palette(&self) -> Palette {
        tokens::palette(self.layout.appearance.scheme(self.system_dark))
    }

    pub(super) fn persist_layout(&mut self, cx: &mut Context<Self>) {
        match layout::save(&self.path, &self.layout) {
            Ok(()) => self.layout_note = None,
            Err(error) => {
                self.message = format!("Layout preferences were not saved: {error}");
            }
        }
        cx.notify();
    }

    pub(super) fn toggle_rail(&mut self, rail: Rail, cx: &mut Context<Self>) {
        let shown = match rail {
            Rail::History => self.resolved.history,
            Rail::Vault => self.resolved.vault,
        };
        if self.layout.toggle_rail(rail, shown) {
            self.persist_layout(cx);
        } else {
            let name = match rail {
                Rail::History => "history",
                Rail::Vault => "the vault",
            };
            self.message = format!("Widen the window to show {name}.");
            cx.notify();
        }
    }

    pub(super) fn toggle_focus(&mut self, cx: &mut Context<Self>) {
        self.layout.toggle_focus();
        self.persist_layout(cx);
    }

    fn set_appearance(
        &mut self,
        appearance: Appearance,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.layout.appearance = appearance;
        if let Err(error) = theme::apply(appearance, window, cx) {
            self.message = error;
        }
        self.persist_layout(cx);
    }

    pub(super) fn render_shell(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if std::mem::take(&mut self.focus_composer) {
            let handle = self.query.read(cx).focus_handle(cx);
            handle.focus(window, cx);
        }
        let width = f32::from(window.viewport_size().width);
        self.resolved = self.layout.resolve(width, self.open_doc.is_some());
        let resolved = self.resolved;
        let p = self.palette();
        if std::mem::take(&mut self.settings_requested) {
            let desktop = cx.entity().downgrade();
            window.defer(cx, move |window, cx| {
                if let Some(desktop) = desktop.upgrade() {
                    desktop.update(cx, |this, cx| this.open_settings(window, cx));
                }
            });
        }
        let mut row = div()
            .flex()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .child(self.render_rail_slot(Rail::History, resolved.history, cx));
        if resolved.history == RailDisplay::Open {
            row = row.child(self.render_divider(crate::layout::Divider::History, window, cx));
        }
        row = row.child(self.render_centre(&resolved, window, cx));
        if resolved.vault == RailDisplay::Open {
            row = row.child(self.render_divider(crate::layout::Divider::Vault, window, cx));
        }
        row = row.child(self.render_rail_slot(Rail::Vault, resolved.vault, cx));
        div()
            .id("brn-desktop")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(color(p.background))
            .text_color(color(p.text))
            .font_family(tokens::CHROME_FONT)
            .text_size(px(12.))
            .on_mouse_move(
                cx.listener(|this, event: &gpui_kit::MouseMoveEvent, window, cx| {
                    this.drag_divider(event, window, cx)
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.end_divider_drag(cx)),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.end_divider_drag(cx)),
            )
            .child(self.render_header(&resolved, cx))
            .child(self.render_status_line())
            .child(row)
    }

    fn render_rail_slot(
        &mut self,
        rail: Rail,
        display: RailDisplay,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = self.palette();
        match display {
            RailDisplay::Hidden => div().into_any_element(),
            RailDisplay::Open => match rail {
                Rail::History => self.render_history_rail(cx).into_any_element(),
                Rail::Vault => self.render_vault_rail(cx).into_any_element(),
            },
            RailDisplay::Collapsed | RailDisplay::AutoCollapsed => {
                let auto = display == RailDisplay::AutoCollapsed;
                let (id, label, tip) = match (rail, auto) {
                    (Rail::History, false) => ("expand-history", "›", "Show history (⌘0)"),
                    (Rail::History, true) => (
                        "expand-history",
                        "›",
                        "History is hidden to fit the window. Widen the window to show it.",
                    ),
                    (Rail::Vault, false) => ("expand-vault", "‹", "Show vault (⌥⌘0)"),
                    (Rail::Vault, true) => (
                        "expand-vault",
                        "‹",
                        "Vault is hidden to fit the window. Widen the window to show it.",
                    ),
                };
                let slot = div()
                    .w(px(layout::COLLAPSED_RAIL))
                    .flex_shrink_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .pt_2()
                    .bg(color(p.panel))
                    .border_color(color(if auto { p.amber } else { p.line }));
                let slot = match rail {
                    Rail::History => slot.border_r_1(),
                    Rail::Vault => slot.border_l_1(),
                };
                slot.child(
                    Button::new(id)
                        .label(label)
                        .compact()
                        .tooltip(tip)
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_rail(rail, cx))),
                )
                .into_any_element()
            }
        }
    }
}
