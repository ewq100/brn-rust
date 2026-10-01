use super::*;
use crate::layout::Divider;
use gpui_kit::{KeyDownEvent, MouseMoveEvent};

fn index(divider: Divider) -> usize {
    match divider {
        Divider::History => 0,
        Divider::Document => 1,
        Divider::Vault => 2,
    }
}

impl Desktop {
    pub(super) fn render_divider(
        &mut self,
        divider: Divider,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = self.palette();
        let handle = self.divider_focus[index(divider)].clone();
        let active = handle.is_focused(window) || self.dragging == Some(divider);
        let id = match divider {
            Divider::History => "divider-history",
            Divider::Document => "divider-document",
            Divider::Vault => "divider-vault",
        };
        div()
            .id(id)
            .w(px(layout::DIVIDER))
            .h_full()
            .flex_shrink_0()
            .cursor_col_resize()
            .bg(color(if active { p.cyan } else { p.line }))
            .track_focus(&handle)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window, cx| {
                    this.dragging = Some(divider);
                    this.divider_focus[index(divider)].focus(window, cx);
                    cx.notify();
                }),
            )
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                let step = if event.keystroke.modifiers.shift {
                    layout::KEY_STEP_LARGE
                } else {
                    layout::KEY_STEP
                };
                let delta = match event.keystroke.key.as_str() {
                    "left" => -step,
                    "right" => step,
                    _ => return,
                };
                let resolved = this.resolved;
                this.layout.nudge(divider, delta, &resolved);
                this.persist_layout(cx);
                cx.stop_propagation();
            }))
    }

    pub(super) fn drag_divider(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(divider) = self.dragging else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            self.end_divider_drag(cx);
            return;
        }
        let width = f32::from(window.viewport_size().width);
        let resolved = self.layout.resolve(width, self.open_doc.is_some());
        self.layout
            .drag(divider, f32::from(event.position.x), width, &resolved);
        cx.notify();
    }

    pub(in crate::native) fn end_divider_drag(&mut self, cx: &mut Context<Self>) {
        if self.dragging.take().is_some() {
            self.persist_layout(cx);
        }
    }
}
