use super::*;
use crate::layout::Divider;
use gpui_kit::{KeyDownEvent, MouseDownEvent, MouseMoveEvent};

pub(in crate::native) struct DividerDrag {
    divider: Divider,
    grab_offset: f32,
    last_pointer_x: f32,
    initial: LayoutState,
}

impl DividerDrag {
    fn new(
        divider: Divider,
        pointer_x: f32,
        width: f32,
        resolved: &ResolvedLayout,
        state: &LayoutState,
    ) -> Self {
        // LayoutState::drag expects the left edge for History/Document, but
        // the right edge for Vault. Preserve where in the hit area we grabbed.
        let edge = match divider {
            Divider::History => state.history_w,
            Divider::Document => resolved.centre_x + resolved.doc_w,
            Divider::Vault => width - state.vault_w,
        };
        Self {
            divider,
            grab_offset: pointer_x - edge,
            last_pointer_x: pointer_x,
            initial: state.clone(),
        }
    }

    fn apply(
        &mut self,
        pointer_x: f32,
        width: f32,
        resolved: &ResolvedLayout,
        state: &mut LayoutState,
    ) {
        if pointer_x == self.last_pointer_x {
            return;
        }
        self.last_pointer_x = pointer_x;
        state.drag(self.divider, pointer_x - self.grab_offset, width, resolved);
    }

    fn changed(&self, state: &LayoutState) -> bool {
        self.initial != *state
    }
}

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
        let active = handle.is_focused(window)
            || self
                .dragging
                .as_ref()
                .is_some_and(|drag| drag.divider == divider);
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
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    let width = f32::from(window.viewport_size().width);
                    let resolved = this.layout.resolve(width, this.open_doc.is_some());
                    this.dragging = Some(DividerDrag::new(
                        divider,
                        f32::from(event.position.x),
                        width,
                        &resolved,
                        &this.layout,
                    ));
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
        let Some(drag) = self.dragging.as_mut() else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            self.end_divider_drag(cx);
            return;
        }
        let width = f32::from(window.viewport_size().width);
        let resolved = self.layout.resolve(width, self.open_doc.is_some());
        drag.apply(
            f32::from(event.position.x),
            width,
            &resolved,
            &mut self.layout,
        );
        cx.notify();
    }

    pub(in crate::native) fn end_divider_drag(&mut self, cx: &mut Context<Self>) {
        if let Some(drag) = self.dragging.take() {
            if drag.changed(&self.layout) {
                self.persist_layout(cx);
            } else {
                cx.notify();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grabbing_anywhere_in_each_divider_preserves_width_until_pointer_moves() {
        let width = 1400.;
        for divider in [Divider::History, Divider::Document, Divider::Vault] {
            for offset in [0., layout::DIVIDER / 2., layout::DIVIDER] {
                let mut state = LayoutState::default();
                let initial = state.clone();
                let resolved = state.resolve(width, true);
                let left = match divider {
                    Divider::History => state.history_w,
                    Divider::Document => resolved.centre_x + resolved.doc_w,
                    Divider::Vault => width - state.vault_w - layout::DIVIDER,
                };
                let pointer = left + offset;
                let mut drag = DividerDrag::new(divider, pointer, width, &resolved, &state);
                drag.apply(pointer, width, &resolved, &mut state);
                assert_eq!(state, initial);
                assert!(!drag.changed(&state));
                drag.apply(pointer + 10., width, &resolved, &mut state);
                assert!(drag.changed(&state));
                match divider {
                    Divider::History => assert_eq!(state.history_w, initial.history_w + 10.),
                    Divider::Vault => assert_eq!(state.vault_w, initial.vault_w - 10.),
                    Divider::Document => {
                        assert!(
                            (state.resolve(width, true).doc_w - resolved.doc_w - 10.).abs() < 0.001
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn click_or_clamped_drag_does_not_count_as_layout_change() {
        let width = 1400.;
        let mut state = LayoutState {
            history_w: layout::HISTORY.max,
            ..LayoutState::default()
        };
        let resolved = state.resolve(width, true);
        let pointer = state.history_w + 2.;
        let mut drag = DividerDrag::new(Divider::History, pointer, width, &resolved, &state);
        assert!(!drag.changed(&state));
        drag.apply(pointer + 10., width, &resolved, &mut state);
        assert!(!drag.changed(&state));
    }
}
