//! Shared presentation tokens and builders. Presentation only: every control
//! keeps its own element ID, accessible label and checked service call.
use super::theme::pal;
use super::{SMALL_TEXT, UI_FONT, UI_TEXT};
use gpui_kit::component::button::{Button, ButtonGroup, ButtonVariants};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::input::EditorState;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable, Size, WindowExt};
use gpui_kit::{
    Action, AnyElement, App, Context, ElementId, FontWeight, SharedString, TestSupportExt, Window,
    div, prelude::*, px, relative, rgb,
};
use std::rc::Rc;

/// The complete Lucide catalog; BRN composes the icons it adds beyond Kit's
/// default set in `launch`.
pub(super) use gpui_kit::assets::IconName;

/// Spacing scale, bar heights, control heights and radii.
#[allow(
    dead_code,
    reason = "the spacing scale stays complete; 12 and 32 px have no user yet"
)]
pub(super) mod tokens {
    use gpui_kit::{Pixels, px};
    pub(in super::super) const SPACE_1: Pixels = px(4.);
    pub(in super::super) const SPACE_2: Pixels = px(8.);
    pub(in super::super) const SPACE_3: Pixels = px(12.);
    pub(in super::super) const SPACE_4: Pixels = px(16.);
    pub(in super::super) const SPACE_6: Pixels = px(24.);
    pub(in super::super) const SPACE_8: Pixels = px(32.);
    pub(in super::super) const TITLE_BAR: Pixels = px(38.);
    pub(in super::super) const NAV_BAR: Pixels = px(32.);
    pub(in super::super) const PANEL_HEADER: Pixels = px(36.);
    pub(in super::super) const TOOLBAR: Pixels = px(36.);
    pub(in super::super) const STATUS_BAR: Pixels = px(28.);
    pub(in super::super) const ICON_CONTROL: Pixels = px(24.);
    pub(in super::super) const COMPACT_CONTROL: Pixels = px(28.);
    pub(in super::super) const FORM_CONTROL: Pixels = px(32.);
    pub(in super::super) const RADIUS_CHIP: Pixels = px(4.);
    pub(in super::super) const RADIUS_CONTROL: Pixels = px(6.);
    pub(in super::super) const RADIUS_PANEL: Pixels = px(8.);
}
use tokens::*;

/// A status tone: icon, word and tint, never color alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Tone {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}
impl Tone {
    /// The announced word; the visible text carries the meaning.
    pub(super) fn word(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }
    fn icon(self) -> IconName {
        match self {
            Self::Info => IconName::Info,
            Self::Success => IconName::CircleCheck,
            Self::Warning => IconName::TriangleAlert,
            Self::Error => IconName::CircleAlert,
        }
    }
    /// Text and icon color.
    fn color(self) -> u32 {
        let p = pal();
        match self {
            Self::Info => p.info,
            Self::Success => p.success,
            Self::Warning => p.warning,
            Self::Error => p.danger,
        }
    }
    /// Banner and chip tint.
    fn tint(self) -> u32 {
        let p = pal();
        match self {
            Self::Info => p.info_bg,
            Self::Success => p.success_bg,
            Self::Warning => p.warning_bg,
            Self::Error => p.danger_bg,
        }
    }
}

fn tone_icon(tone: Tone, size: impl Into<Size>) -> Icon {
    Icon::new(tone.icon())
        .with_size(size)
        .text_color(rgb(tone.color()))
}

/// A 24 px ghost icon control. The label is announced and shown as the
/// tooltip; callers may replace the tooltip with one naming a shortcut.
pub(super) fn icon_button(
    id: impl Into<ElementId>,
    icon: IconName,
    label: impl Into<SharedString>,
) -> Button {
    let label = label.into();
    Button::new(id)
        .icon(icon)
        .ghost()
        .with_size(Size::Small)
        .accessibility_label(label.clone())
        .tooltip(label)
}

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
pub(super) fn segmented(id: impl Into<ElementId>, buttons: Vec<Button>) -> ButtonGroup {
    ButtonGroup::new(id).children(
        buttons
            .into_iter()
            .map(|button| button.compact().ghost().h(COMPACT_CONTROL)),
    )
}

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

fn row_text(title: &SharedString, more: &[&SharedString]) -> SharedString {
    std::iter::once(title)
        .chain(more.iter().copied())
        .filter(|text| !text.is_empty())
        .map(SharedString::as_ref)
        .collect::<Vec<_>>()
        .join(" · ")
        .into()
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
pub(super) fn empty_state(
    id: impl Into<ElementId>,
    icon: IconName,
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    action: Option<Button>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let p = pal();
    let (title, body) = (title.into(), body.into());
    div()
        .id(id)
        .flex()
        .flex_col()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .px_3()
        .py_4()
        .font_family(UI_FONT)
        .text_center()
        .aria_label(row_text(&title, &[&body]))
        .child(
            Icon::new(icon)
                .with_size(Size::Size(px(20.)))
                .text_color(rgb(p.muted)),
        )
        .when(!title.is_empty(), |state| {
            state.child(
                div()
                    .text_size(UI_TEXT)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(p.ink))
                    .child(title),
            )
        })
        .when(!body.is_empty(), |state| {
            state.child(
                div()
                    .text_size(SMALL_TEXT)
                    .text_color(rgb(p.muted))
                    .child(body),
            )
        })
        .children(action)
        .test_support()
}

/// BRN's own banner row (Kit's Alert cannot hold actions). It never
/// dismisses itself. The tone icon is announced as `{id}-tone`. Actions sit
/// beside a short text and wrap below a long one, so the text never narrows
/// to a column beside them.
pub(super) fn banner(
    id: impl Into<SharedString>,
    tone: Tone,
    text: impl Into<SharedString>,
    actions: Vec<AnyElement>,
    dismiss: Option<Button>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let (id, text) = (id.into(), text.into());
    div()
        .id(id.clone())
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap_2()
        .px_3()
        .py(px(6.))
        .font_family(UI_FONT)
        .text_size(UI_TEXT)
        .bg(rgb(tone.tint()))
        .text_color(rgb(pal().ink))
        .aria_label(text.clone())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_x_3()
                .gap_y_1()
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap_2()
                        .min_w_0()
                        .child(
                            div()
                                .id(SharedString::from(format!("{id}-tone")))
                                .flex_shrink_0()
                                .mt(px(2.))
                                .aria_label(tone.word())
                                .child(tone_icon(tone, Size::Small))
                                .test_support(),
                        )
                        .child(div().min_w_0().child(text)),
                )
                .when(!actions.is_empty(), |line| {
                    line.child(div().flex().flex_wrap().gap_1().children(actions))
                }),
        )
        .children(dismiss)
        .test_support()
}

/// A status sentence that wraps in full, never truncated: the tone icon and
/// text in the tone's color, or muted text when neutral (`None`). The tone is
/// announced as `{id}-tone`.
pub(super) fn status_line(
    id: impl Into<SharedString>,
    tone: Option<Tone>,
    text: impl Into<SharedString>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let (id, text) = (id.into(), text.into());
    div()
        .id(id.clone())
        .flex()
        .items_start()
        .gap_2()
        .min_w_0()
        .font_family(UI_FONT)
        .text_size(UI_TEXT)
        .text_color(rgb(tone.map_or(pal().muted, Tone::color)))
        .aria_label(text.clone())
        .children(tone.map(|tone| {
            div()
                .id(SharedString::from(format!("{id}-tone")))
                .flex_shrink_0()
                .mt(px(2.))
                .aria_label(tone.word())
                .child(tone_icon(tone, Size::Small))
                .test_support()
        }))
        .child(div().flex_1().min_w_0().child(text))
        .test_support()
}

/// An inline card for something the owner must act on: radius 6, one
/// hairline border, the tone's tint (`alt` when neutral) and `ink` text.
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
pub(super) fn status_chip(
    id: impl Into<ElementId>,
    tone: Option<Tone>,
    icon: Option<IconName>,
    label: impl Into<SharedString>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let label = label.into();
    chip(id, tone, icon, label.clone(), label)
}

/// A status chip that shows less than it says: the visible `text` (for
/// example "v3"), with the full `label` announced and in the tooltip.
pub(super) fn short_status_chip(
    id: impl Into<ElementId>,
    tone: Option<Tone>,
    icon: Option<IconName>,
    text: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    chip(id, tone, icon, text.into(), label.into())
}

fn chip(
    id: impl Into<ElementId>,
    tone: Option<Tone>,
    icon: Option<IconName>,
    text: SharedString,
    label: SharedString,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let p = pal();
    let (bg, fg) = tone.map_or((p.alt, p.muted), |tone| (tone.tint(), tone.color()));
    let tooltip = (text != label).then(|| label.clone());
    div()
        .id(id)
        .flex_shrink_0()
        .aria_label(label)
        .when_some(tooltip, |chip, tooltip| {
            chip.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
        })
        .child(
            Tag::custom(rgb(bg).into(), rgb(fg).into(), rgb(p.hairline).into())
                .rounded(RADIUS_CHIP)
                .gap_1()
                .children(icon.map(|icon| Icon::new(icon).with_size(Size::XSmall)))
                // An icon-only chip has no text child, so no trailing gap.
                .when(!text.is_empty(), |tag| tag.child(text)),
        )
        .test_support()
}

/// The one row under a panel header, with a hairline below.
pub(super) fn toolbar(id: impl Into<ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap_1()
        .h(TOOLBAR)
        .px_2()
        .border_b_1()
        .border_color(rgb(pal().hairline))
}

/// Prose editors have no line-number gutter. Call inside `cx.new(|cx| ..)`.
pub(super) fn prose_editor_state(
    window: &mut Window,
    cx: &mut Context<EditorState>,
) -> EditorState {
    EditorState::new(window, cx).line_number(false)
}

/// A disabled control's accessible label and tooltip, both naming why:
/// ("Save, unavailable: BRN is working", "Save is unavailable: BRN is
/// working"). With no reason both are the label.
pub(super) fn unavailable(label: &str, why: Option<&str>) -> (String, String) {
    match why {
        Some(why) => (
            format!("{label}, unavailable: {why}"),
            format!("{label} is unavailable: {why}"),
        ),
        None => (label.to_owned(), label.to_owned()),
    }
}

/// Dialogs stay this much shorter than the window, so the window around
/// them stays visible (plan.md, Overlays).
pub(super) const DIALOG_CLEARANCE: f32 = 96.;
/// Kit's dialog surface around the body: 16 px padding above and below,
/// the title row and its gap, and the footer's row and gap.
const DIALOG_CHROME: f32 = 128.;

/// A dialog's body: it scrolls inside what is left of window − 96 after the
/// title and the footer, so the footer's buttons stay visible in a short
/// window. Put the content in one child.
pub(super) fn dialog_body(
    id: impl Into<ElementId>,
    window: &Window,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let height = window.viewport_size().height;
    let cap = (height - px(DIALOG_CLEARANCE + DIALOG_CHROME)).max(px(96.));
    div().id(id).w_full().max_h(cap).overflow_y_scroll()
}

/// A dialog's footer row, right-aligned: Cancel first, then the confirm.
pub(super) fn dialog_footer() -> gpui_kit::Div {
    div().flex().justify_end().gap_2()
}

/// A dialog's error line under its fields: the error icon and what to do.
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

/// What a dialog's confirm does; the dialog builder runs on every render.
pub(super) type DialogAction = Rc<dyn Fn(&mut Window, &mut App)>;

/// A dialog for an irreversible step: the title states the consequence, the
/// body says whether it can be undone, and only a click (or Space on the
/// focused button) confirms. ↵ is inert. IDs: `{id}-body`, `{id}-cancel`,
/// `{id}-confirm`.
pub(super) fn consequence_dialog(
    dialog: Dialog,
    id: &str,
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    confirm: impl Into<SharedString>,
    on_confirm: DialogAction,
) -> Dialog {
    let body = body.into();
    dialog
        .title(title.into())
        .on_ok(|_, _, _| false)
        .child(
            div()
                .id(SharedString::from(format!("{id}-body")))
                .text_size(UI_TEXT)
                .aria_label(body.clone())
                .child(body)
                .test_support(),
        )
        .footer(
            dialog_footer()
                .child(
                    super::button(SharedString::from(format!("{id}-cancel")))
                        .label("Cancel")
                        .outline()
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                )
                .child(
                    super::button(SharedString::from(format!("{id}-confirm")))
                        .label(confirm.into())
                        .danger()
                        .on_click(move |_, window, cx| {
                            on_confirm(window, cx);
                            window.close_dialog(cx);
                        }),
                ),
        )
}
