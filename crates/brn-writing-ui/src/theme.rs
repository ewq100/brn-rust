//! Appearance palettes. Presentation only: nothing here affects authority.
//!
//! `Light` and `Dark` are the BRN-UX-002 semantic token pairs
//! (`docs/product/ux-rebuild/BRN_Design_Tokens.json`). The five named
//! palettes are the earlier themes, kept as advanced choices. Every palette
//! fills the same fields so screens never branch on the palette identity.
use gpui_kit::{App, component::Theme, px, rgb};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeId {
    Light,
    Dark,
    #[default]
    Sammal,
    Laanemeri,
    Raba,
    Virmalised,
    Paber,
}
impl ThemeId {
    pub const ALL: [Self; 7] = [
        Self::Light,
        Self::Dark,
        Self::Sammal,
        Self::Laanemeri,
        Self::Raba,
        Self::Virmalised,
        Self::Paber,
    ];
    /// The earlier named palettes, offered under advanced appearance.
    pub const LEGACY: [Self; 5] = [
        Self::Sammal,
        Self::Laanemeri,
        Self::Raba,
        Self::Virmalised,
        Self::Paber,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Sammal => "sammal",
            Self::Laanemeri => "laanemeri",
            Self::Raba => "raba",
            Self::Virmalised => "virmalised",
            Self::Paber => "paber",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Sammal => "Sammal",
            Self::Laanemeri => "Läänemeri",
            Self::Raba => "Raba",
            Self::Virmalised => "Virmalised",
            Self::Paber => "Paber",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Light => "neutral light",
            Self::Dark => "neutral dark",
            Self::Sammal => "moss on cream",
            Self::Laanemeri => "Baltic blue-grey",
            Self::Raba => "dark bog peat · red",
            Self::Virmalised => "dark night · aurora green",
            Self::Paber => "black ink on paper",
        }
    }
    pub fn from_key(key: Option<&str>) -> Self {
        Self::ALL
            .into_iter()
            .find(|theme| Some(theme.key()) == key)
            .unwrap_or_default()
    }
    pub fn is_dark(self) -> bool {
        self.palette().dark
    }
    pub fn palette(self) -> Palette {
        let base = match self {
            Self::Light => return Palette::tokens(&LIGHT),
            Self::Dark => return Palette::tokens(&DARK),
            Self::Sammal => Base {
                paper: 0xf7f5ee,
                tile: 0xfffdf7,
                alt: 0xf1f5ed,
                border: 0x9cad91,
                ink: 0x283526,
                muted: 0x56634f,
                accent: 0x416438,
                primary_fg: 0xfffdf7,
                dark: false,
            },
            Self::Laanemeri => Base {
                paper: 0xeef2f5,
                tile: 0xfbfcfd,
                alt: 0xe6edf2,
                border: 0x9fb2c1,
                ink: 0x1d2b36,
                muted: 0x52646f,
                accent: 0x2f6690,
                primary_fg: 0xfbfcfd,
                dark: false,
            },
            // The owner's d9534b reached only 4.0:1 on tiles; lightened 15% to pass 4.5:1.
            Self::Raba => Base {
                paper: 0x1c1a16,
                tile: 0x24221c,
                alt: 0x2b2921,
                border: 0x4d4a3c,
                ink: 0xe6e0cc,
                muted: 0xa39e86,
                accent: 0xdf6d66,
                primary_fg: 0x24221c,
                dark: true,
            },
            Self::Virmalised => Base {
                paper: 0x0f1419,
                tile: 0x151b22,
                alt: 0x1b232c,
                border: 0x2f3b47,
                ink: 0xd6e2ea,
                muted: 0x8796a3,
                accent: 0x5fd6a4,
                primary_fg: 0x151b22,
                dark: true,
            },
            Self::Paber => Base {
                paper: 0xf2f1ed,
                tile: 0xffffff,
                alt: 0xeceae4,
                border: 0x8a8a85,
                ink: 0x111111,
                muted: 0x555550,
                accent: 0x111111,
                primary_fg: 0xffffff,
                dark: false,
            },
        };
        Palette::derive(base)
    }
}

struct Base {
    paper: u32,
    tile: u32,
    alt: u32,
    border: u32,
    ink: u32,
    muted: u32,
    accent: u32,
    primary_fg: u32,
    dark: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub paper: u32,
    pub tile: u32,
    pub alt: u32,
    pub border: u32,
    pub ink: u32,
    pub muted: u32,
    pub accent: u32,
    pub primary_fg: u32,
    pub accent_hover: u32,
    pub accent_active: u32,
    pub hover: u32,
    pub active: u32,
    pub selection: u32,
    pub add: u32,
    pub delete: u32,
    pub comment: u32,
    pub comment_strong: u32,
    /// Separators and panel outlines; `border` stays for inputs and focus.
    pub hairline: u32,
    /// Tone text and icon colors; each passes 4.5:1 on tile, alt, paper and
    /// its own tint. `danger` also fills the destructive-dialog confirm.
    pub danger: u32,
    pub warning: u32,
    pub success: u32,
    pub info: u32,
    /// Banner and chip tints; text on them is `ink`.
    pub danger_bg: u32,
    pub warning_bg: u32,
    pub success_bg: u32,
    pub info_bg: u32,
    // BRN-UX-002 semantic roles without an earlier field. Earlier fields map
    // as: canvas = `tile`, text = `ink`, text_muted = `muted`, divider =
    // `hairline`, control_border = `border`, primary = `accent`, on_primary
    // = `primary_fg`, surface_subtle = `alt`, and the tone pairs to
    // `danger`/`danger_bg`, `warning`/`warning_bg`, `success`/`success_bg`.
    /// Navigation sidebar background.
    pub sidebar: u32,
    /// Menus, dialogs and popovers.
    pub surface_raised: u32,
    /// Input placeholder text (4.5:1 on inputs).
    pub placeholder: u32,
    /// Visible keyboard focus ring (3:1 against its surface).
    pub focus: u32,
    /// Link text.
    pub link: u32,
    pub dark: bool,
}

/// BRN-UX-002 geometry and type targets in logical pixels, shared by the
/// desktop shell and the writing surfaces. Starting values; usable width
/// policy and readability take precedence (spec §8, §10).
pub mod metrics {
    pub const SIDEBAR: f32 = 224.;
    pub const SIDEBAR_MIN: f32 = 208.;
    pub const SIDEBAR_MAX: f32 = 280.;
    pub const SIDEBAR_RAIL: f32 = 56.;
    pub const SIDEBAR_ROW: f32 = 36.;
    pub const HEADER: f32 = 56.;
    pub const CONVERSATION_WIDTH: f32 = 720.;
    pub const CONVERSATION_MIN_WIDTH: f32 = 640.;
    pub const DOCUMENT_WIDTH: f32 = 760.;
    pub const DOCUMENT_MIN_WIDTH: f32 = 680.;
    pub const DOCUMENT_REGION_MIN: f32 = 560.;
    pub const OUTLINE: f32 = 200.;
    pub const OUTLINE_MIN: f32 = 180.;
    pub const OUTLINE_MAX: f32 = 240.;
    pub const ASSISTANT: f32 = 320.;
    pub const ASSISTANT_MIN: f32 = 280.;
    pub const ASSISTANT_MAX: f32 = 400.;
    pub const PAGE_INSET: f32 = 32.;
    pub const COMPACT_PAGE_INSET: f32 = 20.;
    pub const CONTROL_HEIGHT: f32 = 32.;
    pub const ICON_TARGET: f32 = 30.;
    pub const DIVIDER: f32 = 1.;
    pub const RADIUS_CONTROL: f32 = 7.;
    pub const RADIUS_CARD: f32 = 10.;
    pub const RADIUS_BUBBLE: f32 = 18.;
    pub const RADIUS_COMPOSER: f32 = 18.;
    pub const COMPOSER_MIN_ROWS: usize = 2;
    pub const COMPOSER_MAX_ROWS: usize = 8;
    /// Type scale.
    pub const TEXT_METADATA: f32 = 11.;
    pub const TEXT_LABEL: f32 = 12.;
    pub const TEXT_SIDEBAR: f32 = 13.;
    pub const TEXT_BODY: f32 = 14.;
    pub const TEXT_DOCUMENT: f32 = 16.;
    pub const TEXT_SECTION: f32 = 18.;
    pub const TEXT_CONVERSATION_HEADING: f32 = 20.;
    pub const TEXT_ARTICLE_HEADING: f32 = 28.;
    pub const DOCUMENT_LINE_HEIGHT: f32 = 1.7;
    /// Markdown H1–H6 sizes in the reading view.
    pub const DOCUMENT_HEADINGS: [f32; 6] = [28., 22., 18., 16., 16., 16.];
    pub const DOCUMENT_HEADING_LINE_HEIGHT: f32 = 1.3;
    pub const DOCUMENT_PARAGRAPH_GAP: f32 = 12.;
    pub const DOCUMENT_HEADING_BEFORE: f32 = 24.;
    pub const DOCUMENT_HEADING_AFTER: f32 = 8.;
}

/// One theme's semantic token values, as in `BRN_Design_Tokens.json`.
struct Tokens {
    canvas: u32,
    sidebar: u32,
    surface_subtle: u32,
    surface_raised: u32,
    hover: u32,
    text: u32,
    text_muted: u32,
    placeholder: u32,
    divider: u32,
    control_border: u32,
    primary: u32,
    on_primary: u32,
    focus: u32,
    link: u32,
    selection: u32,
    success_text: u32,
    success_surface: u32,
    warning_text: u32,
    warning_surface: u32,
    danger_text: u32,
    danger_surface: u32,
    dark: bool,
}
const LIGHT: Tokens = Tokens {
    canvas: 0xffffff,
    sidebar: 0xf5f5f3,
    surface_subtle: 0xfafaf9,
    surface_raised: 0xffffff,
    hover: 0xeaeae6,
    text: 0x242424,
    text_muted: 0x64645f,
    placeholder: 0x70706a,
    divider: 0xdeded9,
    control_border: 0x85857e,
    primary: 0x292a28,
    on_primary: 0xffffff,
    focus: 0x526a9c,
    link: 0x315ca0,
    selection: 0xd7e5fb,
    success_text: 0x346b54,
    success_surface: 0xedf5ee,
    warning_text: 0x7c5c22,
    warning_surface: 0xf9f3e7,
    danger_text: 0xa63535,
    danger_surface: 0xfceeee,
    dark: false,
};
const DARK: Tokens = Tokens {
    canvas: 0x202020,
    sidebar: 0x181818,
    surface_subtle: 0x252525,
    surface_raised: 0x2a2a28,
    hover: 0x30302e,
    text: 0xebebeb,
    text_muted: 0xb3b3ab,
    placeholder: 0xa6a69f,
    divider: 0x3e3e3a,
    control_border: 0x818179,
    primary: 0xeeeee9,
    on_primary: 0x232421,
    focus: 0x9fb7e5,
    link: 0xabc7f5,
    selection: 0x3b5071,
    success_text: 0xa4d1b9,
    success_surface: 0x263a2f,
    warning_text: 0xdec18a,
    warning_surface: 0x393226,
    danger_text: 0xf0b4b4,
    danger_surface: 0x412828,
    dark: true,
};

impl Palette {
    /// A BRN-UX-002 token pair. Roles the token file does not name (pressed
    /// fills, comment highlights, info tone) derive from the named tokens.
    fn tokens(t: &Tokens) -> Self {
        let toward = if t.dark { 0xffffff } else { 0x000000 };
        let primary_shift = |amount: f32| {
            // A near-black primary lightens on hover; a near-white one darkens.
            mix(t.primary, if t.dark { 0x000000 } else { 0xffffff }, amount)
        };
        Self {
            paper: t.canvas,
            tile: t.canvas,
            alt: t.surface_subtle,
            border: t.control_border,
            ink: t.text,
            muted: t.text_muted,
            accent: t.primary,
            primary_fg: t.on_primary,
            accent_hover: primary_shift(0.12),
            accent_active: primary_shift(0.2),
            hover: t.hover,
            active: mix(t.hover, toward, 0.06),
            selection: t.selection,
            add: t.success_surface,
            delete: t.danger_surface,
            comment: mix(t.canvas, 0xf2c14e, if t.dark { 0.22 } else { 0.30 }),
            comment_strong: mix(t.canvas, 0xf2c14e, if t.dark { 0.34 } else { 0.55 }),
            hairline: t.divider,
            danger: t.danger_text,
            warning: t.warning_text,
            success: t.success_text,
            info: t.link,
            danger_bg: t.danger_surface,
            warning_bg: t.warning_surface,
            success_bg: t.success_surface,
            info_bg: mix(t.canvas, t.link, if t.dark { 0.18 } else { 0.10 }),
            sidebar: t.sidebar,
            surface_raised: t.surface_raised,
            placeholder: t.placeholder,
            focus: t.focus,
            link: t.link,
            dark: t.dark,
        }
    }
    fn derive(base: Base) -> Self {
        let toward = if base.dark { 0xffffff } else { 0x000000 };
        // Tones share one hue per role across themes; only light or dark differs.
        let tone = |light: u32, dark: u32| if base.dark { dark } else { light };
        let tint = |color: u32| mix(base.tile, color, if base.dark { 0.22 } else { 0.14 });
        let (danger, warning, success, info) = (
            tone(0xb63629, 0xf3938b),
            tone(0x8b5d00, 0xe3b341),
            tone(0x2a7349, 0x6fcf97),
            tone(0x2f6690, 0x79b8ff),
        );
        Self {
            paper: base.paper,
            tile: base.tile,
            alt: base.alt,
            border: base.border,
            ink: base.ink,
            muted: base.muted,
            accent: base.accent,
            primary_fg: base.primary_fg,
            accent_hover: mix(base.accent, toward, 0.14),
            accent_active: mix(base.accent, toward, 0.26),
            hover: mix(base.tile, base.accent, 0.10),
            active: mix(base.tile, base.accent, 0.18),
            selection: mix(base.tile, base.accent, if base.dark { 0.32 } else { 0.22 }),
            add: mix(base.tile, 0x2e9e5b, if base.dark { 0.30 } else { 0.20 }),
            delete: mix(base.tile, 0xd04437, if base.dark { 0.30 } else { 0.18 }),
            comment: mix(base.tile, 0xf2c14e, if base.dark { 0.28 } else { 0.42 }),
            comment_strong: mix(base.tile, 0xf2c14e, if base.dark { 0.38 } else { 0.70 }),
            hairline: mix(base.tile, base.border, 0.45),
            danger,
            warning,
            success,
            info,
            danger_bg: tint(danger),
            warning_bg: tint(warning),
            success_bg: tint(success),
            info_bg: tint(info),
            sidebar: base.alt,
            surface_raised: base.tile,
            placeholder: base.muted,
            focus: base.accent,
            link: base.accent,
            dark: base.dark,
        }
    }
}

/// Linear sRGB channel mix; `amount` 0 keeps `from`, 1 becomes `to`.
pub fn mix(from: u32, to: u32, amount: f32) -> u32 {
    let channel = |shift: u32| {
        let a = ((from >> shift) & 0xff) as f32;
        let b = ((to >> shift) & 0xff) as f32;
        ((a + (b - a) * amount).round().clamp(0., 255.) as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// WCAG 2 contrast ratio.
#[cfg(test)]
pub fn contrast(a: u32, b: u32) -> f32 {
    fn luminance(color: u32) -> f32 {
        let linear = |shift: u32| {
            let value = ((color >> shift) & 0xff) as f32 / 255.;
            if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0)
    }
    let (a, b) = (luminance(a), luminance(b));
    let (hi, lo) = if a > b { (a, b) } else { (b, a) };
    (hi + 0.05) / (lo + 0.05)
}

thread_local! {
    static CURRENT: Cell<ThemeId> = const { Cell::new(ThemeId::Sammal) };
}

/// Colors of the theme applied in this UI thread.
pub fn pal() -> Palette {
    CURRENT.with(Cell::get).palette()
}

/// Apply one palette to toolkit controls; they keep their own focus, selected
/// and disabled behavior.
pub fn apply(theme: ThemeId, cx: &mut App) {
    CURRENT.with(|current| current.set(theme));
    let p = theme.palette();
    // Switch the toolkit's light/dark mode first: it reloads that mode's
    // registered theme (code highlighting, Base scrollbars), which would
    // replace colors edited in the same update.
    let mode = if p.dark {
        gpui_kit::component::ThemeMode::Dark
    } else {
        gpui_kit::component::ThemeMode::Light
    };
    if Theme::global(cx).mode != mode {
        Theme::change(mode, None, cx);
    }
    Theme::update(cx, |theme| {
        theme.font_family = crate::native::UI_FONT.into();
        theme.font_size = crate::native::PROSE_TEXT;
        theme.mono_font_family = crate::native::MONO.into();
        theme.mono_font_size = crate::native::UI_TEXT;
        theme.radius = px(6.);
        theme.radius_lg = px(8.);
        theme.shadow = false;
        let colors = &mut theme.colors;
        colors.background = rgb(p.tile).into();
        colors.foreground = rgb(p.ink).into();
        colors.border = rgb(p.border).into();
        colors.input = rgb(p.border).into();
        colors.button = rgb(p.tile).into();
        colors.button_foreground = rgb(p.ink).into();
        colors.button_hover = rgb(p.hover).into();
        colors.button_active = rgb(p.active).into();
        // Ghost buttons use secondary ink and the secondary active fill.
        colors.secondary = rgb(p.alt).into();
        colors.secondary_foreground = rgb(p.ink).into();
        colors.secondary_hover = rgb(p.hover).into();
        colors.secondary_active = rgb(p.active).into();
        colors.button_secondary = colors.secondary;
        colors.button_secondary_foreground = colors.secondary_foreground;
        colors.button_secondary_hover = colors.secondary_hover;
        colors.button_secondary_active = colors.secondary_active;
        colors.primary = rgb(p.accent).into();
        colors.primary_foreground = rgb(p.primary_fg).into();
        colors.primary_hover = rgb(p.accent_hover).into();
        colors.primary_active = rgb(p.accent_active).into();
        colors.button_primary = colors.primary;
        colors.button_primary_foreground = colors.primary_foreground;
        colors.button_primary_hover = colors.primary_hover;
        colors.button_primary_active = colors.primary_active;
        // Tone channels feed Button::danger() and Tag; they shade like the accent.
        let toward = if p.dark { 0xffffff } else { 0x000000 };
        let tone = |color: u32| -> [gpui_kit::Hsla; 4] {
            [
                rgb(color).into(),
                rgb(p.tile).into(),
                rgb(mix(color, toward, 0.14)).into(),
                rgb(mix(color, toward, 0.26)).into(),
            ]
        };
        [
            colors.danger,
            colors.danger_foreground,
            colors.danger_hover,
            colors.danger_active,
        ] = tone(p.danger);
        [
            colors.button_danger,
            colors.button_danger_foreground,
            colors.button_danger_hover,
            colors.button_danger_active,
        ] = tone(p.danger);
        [
            colors.warning,
            colors.warning_foreground,
            colors.warning_hover,
            colors.warning_active,
        ] = tone(p.warning);
        [
            colors.button_warning,
            colors.button_warning_foreground,
            colors.button_warning_hover,
            colors.button_warning_active,
        ] = tone(p.warning);
        [
            colors.success,
            colors.success_foreground,
            colors.success_hover,
            colors.success_active,
        ] = tone(p.success);
        [
            colors.button_success,
            colors.button_success_foreground,
            colors.button_success_hover,
            colors.button_success_active,
        ] = tone(p.success);
        [
            colors.info,
            colors.info_foreground,
            colors.info_hover,
            colors.info_active,
        ] = tone(p.info);
        [
            colors.button_info,
            colors.button_info_foreground,
            colors.button_info_hover,
            colors.button_info_active,
        ] = tone(p.info);
        colors.accent = rgb(p.active).into();
        colors.accent_foreground = rgb(p.ink).into();
        colors.muted = rgb(p.alt).into();
        colors.muted_foreground = rgb(p.muted).into();
        colors.ring = rgb(p.focus).into();
        colors.caret = rgb(p.accent).into();
        colors.selection = rgb(p.selection).into();
        colors.link = rgb(p.link).into();
        colors.link_hover = rgb(mix(p.link, toward, 0.14)).into();
        colors.link_active = rgb(mix(p.link, toward, 0.26)).into();
        colors.popover = rgb(p.surface_raised).into();
        colors.popover_foreground = rgb(p.ink).into();
        colors.title_bar = rgb(p.sidebar).into();
        colors.title_bar_border = rgb(p.hairline).into();
        colors.window_border = rgb(p.border).into();
        colors.scrollbar = rgb(p.paper).into();
        colors.sidebar = rgb(p.sidebar).into();
        colors.sidebar_foreground = rgb(p.ink).into();
        colors.sidebar_border = rgb(p.hairline).into();
        colors.scrollbar_thumb = rgb(p.border).into();
        colors.scrollbar_thumb_hover = rgb(p.muted).into();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_keeps_readable_text_controls_and_tints() {
        for theme in ThemeId::ALL {
            let p = theme.palette();
            for (name, fg) in [("ink", p.ink), ("muted", p.muted), ("accent", p.accent)] {
                for (surface, bg) in [("tile", p.tile), ("alt", p.alt), ("paper", p.paper)] {
                    let ratio = contrast(fg, bg);
                    assert!(
                        ratio >= 4.5,
                        "{} {name} on {surface}: {ratio:.2}",
                        theme.label()
                    );
                }
            }
            assert!(contrast(p.primary_fg, p.accent) >= 4.5, "{}", theme.label());
            assert!(
                contrast(p.primary_fg, p.accent_hover) >= 4.5,
                "{}",
                theme.label()
            );
            for (name, tint) in [
                ("hover", p.hover),
                ("selection", p.selection),
                ("add", p.add),
                ("delete", p.delete),
                ("comment", p.comment),
                ("comment-strong", p.comment_strong),
            ] {
                assert!(
                    contrast(p.ink, tint) >= 4.5,
                    "{} ink on {name}: {:.2}",
                    theme.label(),
                    contrast(p.ink, tint)
                );
                assert_ne!(tint, p.tile, "{} {name} is visible", theme.label());
            }
        }
    }

    #[test]
    fn every_theme_keeps_tones_tints_and_the_danger_fill_readable() {
        let toward = |p: &Palette| if p.dark { 0xffffff } else { 0x000000 };
        let at_least = |theme: ThemeId, what: &str, fg: u32, bg: u32| {
            let ratio = contrast(fg, bg);
            assert!(ratio >= 4.5, "{} {what}: {ratio:.2}", theme.label());
        };
        for theme in ThemeId::ALL {
            let p = theme.palette();
            // The destructive-dialog confirm: tile text on the danger fill and its hover.
            at_least(theme, "tile on danger", p.tile, p.danger);
            at_least(
                theme,
                "tile on danger hover",
                p.tile,
                mix(p.danger, toward(&p), 0.14),
            );
            let tones = [
                ("danger", p.danger, p.danger_bg),
                ("warning", p.warning, p.warning_bg),
                ("success", p.success, p.success_bg),
                ("info", p.info, p.info_bg),
            ];
            for (name, tone, tint) in tones {
                for (surface, bg) in [
                    ("tile", p.tile),
                    ("alt", p.alt),
                    ("paper", p.paper),
                    ("own tint", tint),
                ] {
                    at_least(theme, &format!("{name} on {surface}"), tone, bg);
                }
                at_least(theme, &format!("ink on {name} tint"), p.ink, tint);
                assert_ne!(tint, p.tile, "{} {name} tint is visible", theme.label());
            }
            for (surface, bg) in [("selection", p.selection), ("hover", p.hover)] {
                at_least(theme, &format!("ink on {surface}"), p.ink, bg);
            }
            for (surface, bg) in [
                ("tile", p.tile),
                ("alt", p.alt),
                ("paper", p.paper),
                ("hover", p.hover),
            ] {
                at_least(theme, &format!("muted on {surface}"), p.muted, bg);
            }
            assert!(
                p.hairline != p.tile && p.hairline != p.border,
                "{} hairline sits between tile and border",
                theme.label()
            );
        }
    }

    /// BRN-UX-002 §9.2: text, metadata, placeholders and links reach 4.5:1
    /// on the surfaces they sit on; focus and control borders reach 3:1.
    #[test]
    fn light_and_dark_meet_the_measured_contrast_targets() {
        for theme in [ThemeId::Light, ThemeId::Dark] {
            let p = theme.palette();
            let surfaces = [
                ("canvas", p.tile),
                ("sidebar", p.sidebar),
                ("subtle", p.alt),
                ("raised", p.surface_raised),
                ("hover", p.hover),
            ];
            // Placeholders sit on input fills only, not on hovered rows.
            for bg in [p.tile, p.alt, p.surface_raised] {
                let ratio = contrast(p.placeholder, bg);
                assert!(ratio >= 4.5, "{} placeholder: {ratio:.2}", theme.label());
            }
            for (surface, bg) in surfaces {
                for (name, fg, minimum) in [
                    ("text", p.ink, 4.5),
                    ("muted", p.muted, 4.5),
                    ("link", p.link, 4.5),
                    ("focus", p.focus, 3.0),
                    ("control border", p.border, 3.0),
                ] {
                    let ratio = contrast(fg, bg);
                    assert!(
                        ratio >= minimum,
                        "{} {name} on {surface}: {ratio:.2}",
                        theme.label()
                    );
                }
            }
            assert!(contrast(p.primary_fg, p.accent) >= 4.5);
            assert!(contrast(p.primary_fg, p.accent_active) >= 4.5);
            assert_ne!(p.sidebar, p.tile, "{} sidebar is distinct", theme.label());
            assert_eq!(theme.is_dark(), theme == ThemeId::Dark);
        }
    }

    #[test]
    fn keys_round_trip_and_unknown_values_use_the_default() {
        for theme in ThemeId::ALL {
            assert_eq!(ThemeId::from_key(Some(theme.key())), theme);
        }
        assert_eq!(ThemeId::from_key(None), ThemeId::Sammal);
        assert_eq!(ThemeId::from_key(Some("neon")), ThemeId::Sammal);
        assert_eq!(ThemeId::Raba.palette().accent, 0xdf6d66);
        assert_eq!(ThemeId::Sammal.palette().accent, 0x416438);
    }
}
