//! Shared writing and shell presentation primitives; no service or authority.
use crate::native::{SMALL_TEXT, UI_FONT, UI_TEXT, button};
use crate::theme::pal;
/// The complete Lucide catalog; BRN composes the icons it adds beyond Kit's
/// default set in the desktop `launch`.
pub use gpui_kit::assets::IconName;
use gpui_kit::component::button::{Button, ButtonGroup, ButtonVariants};
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::input::EditorState;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable, Size, WindowExt};
use gpui_kit::{
    AnyElement, App, Context, ElementId, FontWeight, SharedString, TestSupportExt, Window, div,
    prelude::*, px, rgb,
};
use std::rc::Rc;

/// Spacing scale, bar heights, control heights and radii.
#[allow(
    dead_code,
    reason = "the spacing scale stays complete; 12 and 32 px have no user yet"
)]
pub mod tokens {
    use gpui_kit::{Pixels, px};
    pub const SPACE_1: Pixels = px(4.);
    pub const SPACE_2: Pixels = px(8.);
    pub const SPACE_3: Pixels = px(12.);
    pub const SPACE_4: Pixels = px(16.);
    pub const SPACE_6: Pixels = px(24.);
    pub const SPACE_8: Pixels = px(32.);
    pub const TITLE_BAR: Pixels = px(38.);
    pub const NAV_BAR: Pixels = px(32.);
    pub const PANEL_HEADER: Pixels = px(36.);
    pub const TOOLBAR: Pixels = px(36.);
    pub const STATUS_BAR: Pixels = px(28.);
    pub const ICON_CONTROL: Pixels = px(24.);
    pub const COMPACT_CONTROL: Pixels = px(28.);
    pub const FORM_CONTROL: Pixels = px(32.);
    pub const RADIUS_CHIP: Pixels = px(4.);
    pub const RADIUS_CONTROL: Pixels = px(6.);
    pub const RADIUS_PANEL: Pixels = px(8.);
}
use tokens::*;

/// A status tone: icon, word and tint, never color alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Info,
    Success,
    Warning,
    Error,
}
impl Tone {
    /// The announced word; the visible text carries the meaning.
    pub fn word(self) -> &'static str {
        match self {
            Self::Info => "Info",
            Self::Success => "Success",
            Self::Warning => "Warning",
            Self::Error => "Error",
        }
    }
    pub fn icon(self) -> IconName {
        match self {
            Self::Info => IconName::Info,
            Self::Success => IconName::CircleCheck,
            Self::Warning => IconName::TriangleAlert,
            Self::Error => IconName::CircleAlert,
        }
    }
    /// Text and icon color.
    pub fn color(self) -> u32 {
        let p = pal();
        match self {
            Self::Info => p.info,
            Self::Success => p.success,
            Self::Warning => p.warning,
            Self::Error => p.danger,
        }
    }
    /// Banner and chip tint.
    pub fn tint(self) -> u32 {
        let p = pal();
        match self {
            Self::Info => p.info_bg,
            Self::Success => p.success_bg,
            Self::Warning => p.warning_bg,
            Self::Error => p.danger_bg,
        }
    }
}

pub fn tone_icon(tone: Tone, size: impl Into<Size>) -> Icon {
    Icon::new(tone.icon())
        .with_size(size)
        .text_color(rgb(tone.color()))
}

/// A 24 px ghost icon control. The label is announced and shown as the
/// tooltip; callers may replace the tooltip with one naming a shortcut.
pub fn icon_button(
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

/// A mode switch. Each segment keeps its own ID, handler and tooltip because
/// the group has no click handler of its own; Kit reports the selected
/// segment as pressed.
pub fn segmented(id: impl Into<ElementId>, buttons: Vec<Button>) -> ButtonGroup {
    ButtonGroup::new(id).children(
        buttons
            .into_iter()
            .map(|button| button.compact().ghost().h(COMPACT_CONTROL)),
    )
}

/// A quiet, sentence-case group label with an optional count and trailing
/// control.
pub fn section_header(
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

pub fn row_text(title: &SharedString, more: &[&SharedString]) -> SharedString {
    std::iter::once(title)
        .chain(more.iter().copied())
        .filter(|text| !text.is_empty())
        .map(SharedString::as_ref)
        .collect::<Vec<_>>()
        .join(" · ")
        .into()
}

/// A quiet empty state: icon, optional title, one sentence and at most one
/// action. Other routes belong in the body text.
pub fn empty_state(
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
pub fn banner(
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

/// A banner that leads a stack in a short panel: its actions sit on their
/// own row and never leave view; when its parent is short the text keeps
/// its first line and scrolls above them (`{id}-text`).
pub fn banner_pinned(
    id: impl Into<SharedString>,
    tone: Tone,
    text: impl Into<SharedString>,
    actions: Vec<AnyElement>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    let (id, text) = (id.into(), text.into());
    div()
        .id(id.clone())
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .gap_1()
        .px_3()
        .py(px(6.))
        .font_family(UI_FONT)
        .text_size(UI_TEXT)
        .bg(rgb(tone.tint()))
        .text_color(rgb(pal().ink))
        .aria_label(text.clone())
        .child(
            div()
                .id(SharedString::from(format!("{id}-text")))
                .flex()
                .items_start()
                .gap_2()
                .min_w_0()
                .flex_shrink(1.)
                .min_h(BANNER_LINE)
                .overflow_y_scroll()
                .child(
                    div()
                        .id(SharedString::from(format!("{id}-tone")))
                        .flex_shrink_0()
                        .mt(px(2.))
                        .aria_label(tone.word())
                        .child(tone_icon(tone, Size::Small))
                        .test_support(),
                )
                .child(div().min_w_0().child(text))
                .test_support(),
        )
        .when(!actions.is_empty(), |banner| {
            banner.child(
                div()
                    .flex()
                    .flex_wrap()
                    .flex_shrink_0()
                    .gap_1()
                    .children(actions),
            )
        })
        .test_support()
}

/// One line of 14 px banner text.
pub const BANNER_LINE: gpui_kit::Pixels = px(20.);
/// A pinned banner's least height: its padding, one line of text and its
/// row of 28 px actions.
pub const BANNER_PINNED_MIN: gpui_kit::Pixels = px(6. + 20. + 4. + 28. + 6.);

/// A status sentence that wraps in full, never truncated: the tone icon and
/// text in the tone's color, or muted text when neutral (`None`). The tone is
/// announced as `{id}-tone`.
pub fn status_line(
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

/// A compact status chip on its tone's tint; `None` is neutral.
pub fn status_chip(
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
pub fn short_status_chip(
    id: impl Into<ElementId>,
    tone: Option<Tone>,
    icon: Option<IconName>,
    text: impl Into<SharedString>,
    label: impl Into<SharedString>,
) -> gpui_kit::base::ObservedElement<gpui_kit::Stateful<gpui_kit::Div>> {
    chip(id, tone, icon, text.into(), label.into())
}

/// A status chip with separate visible text and a full accessible label.
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
pub fn toolbar(id: impl Into<ElementId>) -> gpui_kit::Stateful<gpui_kit::Div> {
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
pub fn prose_editor_state(window: &mut Window, cx: &mut Context<EditorState>) -> EditorState {
    EditorState::new(window, cx).line_number(false)
}

/// A disabled control's accessible label and tooltip, both naming why:
/// ("Save, unavailable: BRN is working", "Save is unavailable: BRN is
/// working"). With no reason both are the label.
pub fn unavailable(label: &str, why: Option<&str>) -> (String, String) {
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
pub const DIALOG_CLEARANCE: f32 = 96.;
/// Kit's dialog surface around the body: 16 px padding above and below,
/// the title row and its gap, and the footer's row and gap.
const DIALOG_CHROME: f32 = 128.;

/// A dialog's body: it scrolls inside what is left of window − 96 after the
/// title and the footer, so the footer's buttons stay visible in a short
/// window. Put the content in one child.
pub fn dialog_body(id: impl Into<ElementId>, window: &Window) -> gpui_kit::Stateful<gpui_kit::Div> {
    let height = window.viewport_size().height;
    let cap = (height - px(DIALOG_CLEARANCE + DIALOG_CHROME)).max(px(96.));
    div().id(id).w_full().max_h(cap).overflow_y_scroll()
}

/// A dialog's footer row, right-aligned: Cancel first, then the confirm.
pub fn dialog_footer() -> gpui_kit::Div {
    div().flex().justify_end().gap_2()
}

/// What a dialog's confirm does; the dialog builder runs on every render.
pub type DialogAction = Rc<dyn Fn(&mut Window, &mut App)>;

/// A dialog for an irreversible step: the title states the consequence, the
/// body says whether it can be undone, and only a click (or Space on the
/// focused button) confirms. ↵ is inert. IDs: `{id}-body`, `{id}-cancel`,
/// `{id}-confirm`.
pub fn consequence_dialog(
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
                    button(SharedString::from(format!("{id}-cancel")))
                        .label("Cancel")
                        .outline()
                        .on_click(|_, window, cx| window.close_dialog(cx)),
                )
                .child(
                    button(SharedString::from(format!("{id}-confirm")))
                        .label(confirm.into())
                        .danger()
                        .on_click(move |_, window, cx| {
                            on_confirm(window, cx);
                            window.close_dialog(cx);
                        }),
                ),
        )
}
