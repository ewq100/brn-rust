//! Shared presentation tokens and builders. Presentation only: every control
//! keeps its own element ID, accessible label and checked service call.
use super::theme::pal;
use super::{SMALL_TEXT, UI_FONT, UI_TEXT};
pub(super) use brn_writing_ui::components::*;
use gpui_kit::component::button::Button;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Sizable, Size};
use gpui_kit::{
    Action, AnyElement, ElementId, FontWeight, SharedString, TestSupportExt, div, prelude::*, px,
    relative, rgb,
};
use tokens::*;

/// The complete Lucide catalog; BRN composes the icons it adds beyond Kit's
/// default set in `launch`.
/// An icon control whose tooltip shows the action's key binding.
pub(super) fn icon_button_with_action(
    id: impl Into<ElementId>,
    icon: IconName,
    label: impl Into<SharedString>,
    action: &dyn Action,
) -> Button {
    let label = label.into();
    icon_button(id, icon, label.clone()).tooltip_with_action(label, action, None)
}

/// A mode switch. Each segment keeps its own ID, handler and tooltip because
/// the group has no click handler of its own; Kit reports the selected
/// segment as pressed.
/// The ⋯ trigger; callers chain `.dropdown_menu(..)`.
pub(super) fn overflow_menu(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    icon_button(id, IconName::Ellipsis, label)
}

/// A quiet, sentence-case group label with an optional count and trailing
/// control.
pub(super) fn section_header(
    label: impl Into<SharedString>,
    count: Option<usize>,
    trailing: Option<AnyElement>,
) -> gpui_kit::Div {
    let label = label.into();
    let text: SharedString = match count {
        Some(count) => format!("{label} · {count}").into(),
        None => label,
    };
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .pt(SPACE_4)
        .pb(SPACE_1)
        .font_family(UI_FONT)
        .text_size(SMALL_TEXT)
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(pal().muted))
        .child(div().flex_1().min_w_0().truncate().child(text))
        .children(trailing)
}

fn row_frame(
    id: SharedString,
    full: SharedString,
    tooltip: SharedString,
    selected: bool,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let p = pal();
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
        .min_w_0()
        .px_2()
        .rounded(RADIUS_CONTROL)
        .font_family(UI_FONT)
        .text_size(UI_TEXT)
        .text_color(rgb(p.ink))
        .cursor_pointer()
        .when(selected, |row| row.bg(rgb(p.selection)))
        .when(!selected, |row| {
            row.hover(|style| style.bg(rgb(pal().hover)))
        })
        .aria_label(full)
        .aria_selected(selected)
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
}

/// One line that truncates with an ellipsis; `{row}-title` and
/// `{row}-reason` report their bounds to tests.
fn row_line(id: SharedString, text: SharedString) -> gpui_kit::Stateful<gpui_kit::Div> {
    div().id(id).min_w_0().truncate().child(text)
}

fn row_title(row: &SharedString, title: SharedString, selected: bool) -> AnyElement {
    row_line(format!("{row}-title").into(), title)
        .flex_1()
        .when(selected, |title| title.font_weight(FontWeight::MEDIUM))
        .test_support()
        .into_any_element()
}

/// A row's secondary text color: muted is not readable on the selection
/// tint, so a selected row's metadata and reason use ink.
pub(super) fn row_meta_color(selected: bool) -> u32 {
    let p = pal();
    if selected { p.ink } else { p.muted }
}

/// Metadata is capped at 40 % and truncates.
fn row_meta(meta: SharedString, selected: bool) -> gpui_kit::Div {
    div()
        .flex_shrink_0()
        .max_w(relative(0.4))
        .min_w_0()
        .truncate()
        .text_size(SMALL_TEXT)
        .text_color(rgb(row_meta_color(selected)))
        .child(meta)
}

/// A list row: 28 px with one line, or 44 px with a reason under the title.
/// Each line truncates with an ellipsis and the title never yields to the
/// metadata or chip; the full text is the accessible label and, unless
/// replaced, the tooltip.
pub(super) struct ListRow {
    id: SharedString,
    title: SharedString,
    reason: Option<SharedString>,
    meta: SharedString,
    chip: Option<(SharedString, AnyElement)>,
    tooltip: Option<SharedString>,
    selected: bool,
}
impl ListRow {
    pub(super) fn new(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            reason: None,
            meta: SharedString::default(),
            chip: None,
            tooltip: None,
            selected: false,
        }
    }
    /// A second line saying why the row needs attention.
    pub(super) fn reason(mut self, reason: impl Into<SharedString>) -> Self {
        self.reason = Some(reason.into());
        self
    }
    pub(super) fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = meta.into();
        self
    }
    /// A trailing status chip; `announced` joins the accessible label.
    pub(super) fn chip(
        mut self,
        announced: impl Into<SharedString>,
        chip: impl IntoElement,
    ) -> Self {
        self.chip = Some((announced.into(), chip.into_any_element()));
        self
    }
    /// Replaces the tooltip, which otherwise repeats the full text.
    pub(super) fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
    pub(super) fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    pub(super) fn build(
        self,
    ) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
        let Self {
            id,
            title,
            reason,
            meta,
            chip,
            tooltip,
            selected,
        } = self;
        let empty = SharedString::default();
        let announced = chip.as_ref().map_or(&empty, |(announced, _)| announced);
        let full = row_text(
            &title,
            &[reason.as_ref().unwrap_or(&empty), &meta, announced],
        );
        let frame = row_frame(id.clone(), full.clone(), tooltip.unwrap_or(full), selected);
        let first = div()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .child(row_title(&id, title, selected))
            .when(!meta.is_empty(), |line| {
                line.child(row_meta(meta, selected))
            })
            .children(chip.map(|(_, chip)| chip));
        match reason {
            None => frame
                .items_center()
                .h(COMPACT_CONTROL)
                .child(first.flex_1())
                .test_support(),
            Some(reason) => frame
                .flex_col()
                .justify_center()
                .h(px(44.))
                .child(first)
                .child(
                    row_line(format!("{id}-reason").into(), reason)
                        .text_size(SMALL_TEXT)
                        .text_color(rgb(row_meta_color(selected)))
                        .test_support(),
                )
                .test_support(),
        }
    }
}

/// A 28 px list row (`ListRow` with metadata).
pub(super) fn list_row(
    id: impl Into<SharedString>,
    title: impl Into<SharedString>,
    meta: impl Into<SharedString>,
    selected: bool,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    ListRow::new(id, title)
        .meta(meta)
        .selected(selected)
        .build()
}

/// A 44 px row whose second line states why it needs attention.
#[allow(dead_code, reason = "callers with a chip use ListRow directly")]
pub(super) fn list_row_two_line(
    id: impl Into<SharedString>,
    title: impl Into<SharedString>,
    reason: impl Into<SharedString>,
    meta: impl Into<SharedString>,
    selected: bool,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    ListRow::new(id, title)
        .reason(reason)
        .meta(meta)
        .selected(selected)
        .build()
}

/// A loading line: a spinner beside its text, announced as the text.
pub(super) fn loading(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let text = text.into();
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .font_family(UI_FONT)
        .text_size(SMALL_TEXT)
        .text_color(rgb(pal().muted))
        .aria_label(text.clone())
        .child(
            Spinner::new()
                .with_size(Size::XSmall)
                .color(rgb(pal().muted).into()),
        )
        .child(text)
        .test_support()
}

/// A quiet empty state: icon, optional title, one sentence and at most one
/// action. Other routes belong in the body text.
/// Cards never nest a bordered card.
pub(super) fn card(
    id: impl Into<ElementId>,
    tone: Option<Tone>,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let p = pal();
    div()
        .id(id)
        .flex()
        .flex_col()
        .flex_shrink_0()
        .gap_2()
        .p_3()
        .rounded(RADIUS_CONTROL)
        .border_1()
        .border_color(rgb(p.hairline))
        .bg(rgb(tone.map_or(p.alt, Tone::tint)))
        .font_family(UI_FONT)
        .text_size(UI_TEXT)
        .text_color(rgb(p.ink))
}

/// A tone's icon in its color, announced by the tone's word as `id`.
pub(super) fn tone_mark(
    id: impl Into<SharedString>,
    tone: Tone,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    div()
        .id(id.into())
        .flex_shrink_0()
        .aria_label(tone.word())
        .child(tone_icon(tone, Size::Small))
        .test_support()
}

/// A compact status chip on its tone's tint; `None` is neutral.
/// It stays in the tree while empty, so its ID is always addressable.
pub(super) fn field_error(
    id: impl Into<ElementId>,
    text: impl Into<SharedString>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let text = text.into();
    div()
        .id(id)
        .flex()
        .items_start()
        .gap_1()
        .font_family(UI_FONT)
        .text_size(SMALL_TEXT)
        .text_color(rgb(pal().danger))
        .aria_label(text.clone())
        .when(!text.is_empty(), |line| {
            line.child(tone_icon(Tone::Error, Size::XSmall).mt(px(2.)))
                .child(div().flex_1().min_w_0().child(text))
        })
        .test_support()
}

/// A multi-line form's ⌘↵: it submits from any field without adding a
/// newline (the capture runs before the editor's own handler), and from a
/// focused button through `Ask`. Apply it to the body and to the footer.
pub(super) fn form_keys(root: gpui_kit::Div, submit: DialogAction) -> gpui_kit::Div {
    let on_enter = submit.clone();
    root.capture_action(
        move |action: &gpui_kit::component::input::Enter, window, cx| {
            if action.secondary && !action.shift {
                on_enter(window, cx);
                cx.stop_propagation();
            }
        },
    )
    .on_action(move |_: &super::Ask, window, cx| submit(window, cx))
}
