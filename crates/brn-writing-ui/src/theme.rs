//! Five appearance themes. Presentation only: nothing here affects authority.
use gpui_kit::{App, component::Theme, px, rgb};
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeId {
    #[default]
    Sammal,
    Laanemeri,
    Raba,
    Virmalised,
    Paber,
}
impl ThemeId {
    pub const ALL: [Self; 5] = [
        Self::Sammal,
        Self::Laanemeri,
        Self::Raba,
        Self::Virmalised,
        Self::Paber,
    ];
    pub fn key(self) -> &'static str {
        match self {
            Self::Sammal => "sammal",
            Self::Laanemeri => "laanemeri",
            Self::Raba => "raba",
            Self::Virmalised => "virmalised",
            Self::Paber => "paber",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Sammal => "Sammal",
            Self::Laanemeri => "Läänemeri",
            Self::Raba => "Raba",
            Self::Virmalised => "Virmalised",
            Self::Paber => "Paber",
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Sammal => "moss on cream · default",
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
    pub fn palette(self) -> Palette {
        let base = match self {
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
    pub dark: bool,
}
impl Palette {
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
        colors.ring = rgb(p.accent).into();
        colors.caret = rgb(p.accent).into();
        colors.selection = rgb(p.selection).into();
        colors.link = rgb(p.accent).into();
        colors.link_hover = rgb(p.accent_hover).into();
        colors.link_active = rgb(p.accent_active).into();
        colors.popover = rgb(p.tile).into();
        colors.popover_foreground = rgb(p.ink).into();
        colors.title_bar = rgb(p.alt).into();
        colors.title_bar_border = rgb(p.hairline).into();
        colors.window_border = rgb(p.border).into();
        colors.scrollbar = rgb(p.paper).into();
        colors.sidebar = rgb(p.alt).into();
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
