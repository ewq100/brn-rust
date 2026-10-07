//! Shared BRN view components.
//!
//! Guidance, states and screenshots: `docs/design/src/components.md`. Values come
//! from [`crate::tokens`]; do not hard-code colours or font sizes in views.
//! Every tone is paired with text, so meaning never depends on colour alone.

use super::theme::color;
use crate::tokens::{self, Palette, size, space, text};
use gpui_kit::{
    AnyElement, Div, ElementId, Hsla, SharedString,
    assets::IconName,
    component::{
        Icon, Sizable,
        button::{Button, ButtonVariants},
    },
    div,
    prelude::*,
    px,
};

/// Semantic tone. Each maps to exactly one palette role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Ordinary metadata and inactive state.
    Neutral,
    /// Focus, links, evidence and the primary action.
    Info,
    /// Needs attention: review, changed, waiting, follow-up.
    Attention,
    /// AI-generated or provisional content.
    Ai,
    /// Approved, saved, completed or healthy.
    Success,
    /// Failure, destructive or overdue.
    Danger,
}

impl Tone {
    pub fn value(self, p: Palette) -> u32 {
        match self {
            Tone::Neutral => p.muted,
            Tone::Info => p.cyan,
            Tone::Attention => p.amber,
            Tone::Ai => p.purple,
            Tone::Success => p.green,
            Tone::Danger => p.red,
        }
    }

    pub fn hsla(self, p: Palette) -> Hsla {
        color(self.value(p))
    }
}

/// Uppercase caption that names a group of rows or controls.
pub fn section_label(label: impl Into<SharedString>, p: Palette) -> Div {
    let label: SharedString = label.into();
    div()
        .flex_shrink_0()
        .pt(px(space::MD))
        .pb(px(space::XS))
        .px(px(space::SM))
        .text_size(px(text::CAPTION))
        .font_weight(gpui_kit::FontWeight::SEMIBOLD)
        .text_color(color(p.muted))
        .child(label.to_uppercase())
}

/// Monospace metadata: paths, ids, model names, counts.
pub fn meta(value: impl Into<SharedString>, p: Palette) -> Div {
    div()
        .font_family(tokens::MONO_FONT)
        .text_size(px(text::META))
        .text_color(color(p.muted))
        .child(value.into())
}

/// Muted explanatory prose in the UI font.
pub fn hint(value: impl Into<SharedString>, p: Palette) -> Div {
    div()
        .text_size(px(text::BODY - 1.0))
        .line_height(px(18.))
        .text_color(color(p.muted))
        .child(value.into())
}

/// Compact state label: tinted text with a tinted field. Always text, never a dot alone.
pub fn badge(label: impl Into<SharedString>, tone: Tone, p: Palette) -> Div {
    let ink = tone.hsla(p);
    div()
        .flex_shrink_0()
        .flex()
        .items_center()
        .h(px(18.))
        .px(px(6.))
        .bg(ink.opacity(0.14))
        .border_1()
        .border_color(ink.opacity(0.45))
        .text_color(ink)
        .font_family(tokens::MONO_FONT)
        .text_size(px(text::CAPTION - 0.5))
        .whitespace_nowrap()
        .child(label.into())
}

/// A trust, safety or status explanation with a tone bar. Use for consequences
/// ("knowledge unchanged until approval"), not for ordinary help text.
pub fn callout(tone: Tone, body: impl IntoElement, p: Palette) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .bg(tone.hsla(p).opacity(0.07))
        .child(
            div()
                .w(px(size::ACCENT_BAR))
                .flex_shrink_0()
                .bg(tone.hsla(p)),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .px(px(space::MD))
                .py(px(space::SM))
                .text_size(px(text::BODY - 1.0))
                .line_height(px(18.))
                .text_color(color(p.text))
                .child(body),
        )
}

/// Title row for a document-pane view: optional kind badge, title, identity line.
/// Append toolbar buttons with `.child(...)`; they align to the right.
pub fn view_header(
    title: impl Into<SharedString>,
    kind: Option<(&str, Tone)>,
    identity: Option<String>,
    p: Palette,
) -> Div {
    let mut heading = div().flex().flex_col().flex_1().min_w(px(0.)).gap(px(2.));
    let mut title_row = div().flex().items_center().gap(px(space::SM)).min_w(px(0.));
    if let Some((kind, tone)) = kind {
        title_row = title_row.child(badge(kind.to_owned(), tone, p));
    }
    title_row = title_row.child(
        div()
            .min_w(px(0.))
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(text::TITLE))
            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
            .text_color(color(p.text))
            .child(title.into()),
    );
    heading = heading.child(title_row);
    if let Some(identity) = identity {
        heading = heading.child(
            meta(identity, p)
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis(),
        );
    }
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .gap(px(space::SM))
        .px(px(space::LG))
        .py(px(space::MD))
        .border_b_1()
        .border_color(color(p.line))
        .child(heading)
}

/// Wrapping row of actions that keeps buttons at their natural width.
pub fn toolbar() -> Div {
    div()
        .flex()
        .flex_wrap()
        .flex_shrink_0()
        .items_center()
        .gap(px(space::SM))
}

/// Empty or not-yet-loaded content: a short headline and what to do next.
pub fn empty_state(
    title: impl Into<SharedString>,
    body: impl Into<SharedString>,
    p: Palette,
) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .py(px(space::LG))
        .child(
            div()
                .text_size(px(text::BODY))
                .font_weight(gpui_kit::FontWeight::MEDIUM)
                .text_color(color(p.text))
                .child(title.into()),
        )
        .child(hint(body, p))
}

/// Left-aligned navigation row with an icon and an optional count or state.
/// Keeps the stable element id so routing and tests are unchanged.
pub fn nav_row(
    id: impl Into<ElementId>,
    icon: IconName,
    label: impl Into<SharedString>,
    trailing: Option<(String, Tone)>,
    p: Palette,
) -> Button {
    let label: SharedString = label.into();
    let mut row = div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .items_center()
        .gap(px(space::SM))
        .text_size(px(text::BODY))
        .child(Icon::from(icon).small().text_color(color(p.muted)))
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .overflow_hidden()
                .text_ellipsis()
                .child(label.clone()),
        );
    if let Some((value, tone)) = trailing {
        row = row.child(
            div()
                .flex_shrink_0()
                .font_family(tokens::MONO_FONT)
                .text_size(px(text::CAPTION))
                .text_color(tone.hsla(p))
                .child(value),
        );
    }
    Button::new(id)
        .ghost()
        .w_full()
        .h(px(size::ROW))
        .accessibility_label(label)
        .child(row)
}

/// Left-aligned list row: title plus an optional muted detail line.
pub fn list_row(
    id: impl Into<ElementId>,
    title: impl Into<SharedString>,
    detail: Option<String>,
    trailing: Option<AnyElement>,
    p: Palette,
) -> Button {
    let title: SharedString = title.into();
    let height = if detail.is_some() { 40. } else { size::ROW };
    let mut column = div().flex_1().min_w(px(0.)).flex().flex_col().child(
        div()
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(text::BODY))
            .child(title.clone()),
    );
    if let Some(detail) = detail {
        column = column.child(
            div()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis()
                .text_size(px(text::CAPTION))
                .text_color(color(p.muted))
                .child(detail),
        );
    }
    let mut row = div()
        .flex_1()
        .min_w(px(0.))
        .flex()
        .items_center()
        .gap(px(space::SM))
        .child(column);
    if let Some(trailing) = trailing {
        row = row.child(trailing);
    }
    Button::new(id)
        .ghost()
        .w_full()
        .h(px(height))
        .accessibility_label(title)
        .child(row)
}

/// Small secondary command, for toolbars and row actions.
pub fn quiet(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button::new(id).ghost().small().label(label)
}

/// Standard secondary command.
pub fn secondary(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button::new(id).outline().small().label(label)
}

/// The one main command of a region (Ask, Save, Approve...).
pub fn primary(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Button {
    Button::new(id).primary().small().label(label)
}

/// `YYYY-MM-DD HH:MM UTC` for a Unix-millisecond timestamp. Recorded times are
/// shown in UTC so that every client renders the same exact value.
pub fn utc_time(ms: u64) -> String {
    let secs = ms / 1000;
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil-from-days (Howard Hinnant), valid for the Unix era.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        rem / 3600,
        (rem % 3600) / 60
    )
}

#[cfg(test)]
mod tests {
    use super::utc_time;

    #[test]
    fn utc_time_formats_civil_dates() {
        assert_eq!(utc_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(utc_time(1_791_373_193_125), "2026-10-07 11:39 UTC");
        assert_eq!(utc_time(951_782_400_000), "2000-02-29 00:00 UTC");
    }
}
