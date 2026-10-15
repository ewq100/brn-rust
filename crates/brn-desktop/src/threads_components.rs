//! Shared presentation tokens and builders. Presentation only: every control
//! keeps its own element ID, accessible label and checked service call.
use super::theme::pal;
use super::{SMALL_TEXT, UI_FONT, UI_TEXT};
use gpui_kit::component::button::{Button, ButtonGroup, ButtonVariants};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::input::EditorState;
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
    reason = "the scale is complete; later slices adopt the rest"
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
    id: impl Into<ElementId>,
    full: SharedString,
    selected: bool,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    let p = pal();
    let tooltip = full.clone();
    div()
        .id(id)
        .flex()
        .flex_shrink_0()
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

fn row_title(title: SharedString, selected: bool) -> gpui_kit::Div {
    div()
        .flex_1()
        .min_w_0()
        .truncate()
        .when(selected, |title| title.font_weight(FontWeight::MEDIUM))
        .child(title)
}

/// Metadata is capped at 40 % and truncates; selected rows switch it to ink
/// because muted text is not readable on the selection tint.
fn row_meta(meta: SharedString, selected: bool) -> gpui_kit::Div {
    let p = pal();
    div()
        .flex_shrink_0()
        .max_w(relative(0.4))
        .min_w_0()
        .truncate()
        .text_size(SMALL_TEXT)
        .text_color(rgb(if selected { p.ink } else { p.muted }))
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

/// A 28 px list row. The title never yields to metadata; the full text is in
/// the tooltip and accessible label.
#[allow(dead_code, reason = "S6 moves the navigation lists onto it")]
pub(super) fn list_row(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    meta: impl Into<SharedString>,
    selected: bool,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let (title, meta) = (title.into(), meta.into());
    row_frame(id, row_text(&title, &[&meta]), selected)
        .items_center()
        .gap_2()
        .h(COMPACT_CONTROL)
        .child(row_title(title, selected))
        .when(!meta.is_empty(), |row| row.child(row_meta(meta, selected)))
        .test_support()
}

/// A 44 px row whose second line states why it needs attention.
#[allow(dead_code, reason = "S6 uses it for needs-you rows")]
pub(super) fn list_row_two_line(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    reason: impl Into<SharedString>,
    meta: impl Into<SharedString>,
    selected: bool,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let p = pal();
    let (title, reason, meta) = (title.into(), reason.into(), meta.into());
    row_frame(id, row_text(&title, &[&reason, &meta]), selected)
        .flex_col()
        .justify_center()
        .h(px(44.))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(row_title(title, selected))
                .when(!meta.is_empty(), |line| {
                    line.child(row_meta(meta, selected))
                }),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(SMALL_TEXT)
                .text_color(rgb(if selected { p.ink } else { p.muted }))
                .child(reason),
        )
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
/// dismisses itself. The tone icon is announced as `{id}-tone`.
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
                .id(SharedString::from(format!("{id}-tone")))
                .flex_shrink_0()
                .aria_label(tone.word())
                .child(tone_icon(tone, Size::Small))
                .test_support(),
        )
        .child(div().flex_1().min_w_0().child(text))
        .children(actions)
        .children(dismiss)
        .test_support()
}

/// A compact status chip on its tone's tint; `None` is neutral.
pub(super) fn status_chip(
    id: impl Into<ElementId>,
    tone: Option<Tone>,
    icon: Option<IconName>,
    label: impl Into<SharedString>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let p = pal();
    let label = label.into();
    let (bg, fg) = tone.map_or((p.alt, p.muted), |tone| (tone.tint(), tone.color()));
    div()
        .id(id)
        .flex_shrink_0()
        .aria_label(label.clone())
        .child(
            Tag::custom(rgb(bg).into(), rgb(fg).into(), rgb(p.hairline).into())
                .rounded(RADIUS_CHIP)
                .gap_1()
                .children(icon.map(|icon| Icon::new(icon).with_size(Size::XSmall)))
                .child(label),
        )
        .test_support()
}

/// The one row under a panel header, with a hairline below.
#[allow(dead_code, reason = "S3 moves the Note toolbar onto it")]
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
#[allow(dead_code, reason = "S3 builds the prose editors with it")]
pub(super) fn prose_editor_state(
    window: &mut Window,
    cx: &mut Context<EditorState>,
) -> EditorState {
    EditorState::new(window, cx).line_number(false)
}

/// What a dialog's confirm does; the dialog builder runs on every render.
#[allow(
    dead_code,
    reason = "S3, S5, S6 and S7 open their destructive dialogs with it"
)]
pub(super) type DialogAction = Rc<dyn Fn(&mut Window, &mut App)>;

/// A dialog for an irreversible step: the title states the consequence, the
/// body says whether it can be undone, and only a click (or Space on the
/// focused button) confirms. ↵ is inert. IDs: `{id}-body`, `{id}-cancel`,
/// `{id}-confirm`.
#[allow(
    dead_code,
    reason = "S3, S5, S6 and S7 open their destructive dialogs with it"
)]
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
            div()
                .flex()
                .justify_end()
                .gap_2()
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
