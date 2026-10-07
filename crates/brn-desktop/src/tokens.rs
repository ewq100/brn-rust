//! BRN design tokens: the single source for colour, type, spacing and size values.
//!
//! The design handbook (`docs/design`) generates its token tables and prototype
//! CSS from this file with `scripts/design-handbook.py`; keep each value a plain
//! literal on its own line so that generator can read it. Semantic roles:
//! cyan = focus, links, evidence and the primary action; amber = needs attention,
//! review or changed state; purple = AI-generated or provisional; green = approved,
//! saved or done; red = failure, destructive or overdue. Colour never carries
//! meaning alone: pair it with text or a symbol.

use crate::layout::Scheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub background: u32,
    pub panel: u32,
    pub paper: u32,
    pub text: u32,
    pub muted: u32,
    pub line: u32,
    pub active: u32,
    pub cyan: u32,
    pub amber: u32,
    pub purple: u32,
    pub green: u32,
    pub red: u32,
}

pub const DARK: Palette = Palette {
    background: 0x101619,
    panel: 0x151e22,
    paper: 0x141c20,
    text: 0xe4ecef,
    muted: 0xa6b5bd,
    line: 0x35434a,
    active: 0x20353b,
    cyan: 0x71d8e7,
    amber: 0xefc378,
    purple: 0xc4acf2,
    green: 0x9cdbad,
    red: 0xf2a2a2,
};

pub const LIGHT: Palette = Palette {
    background: 0xf5f7f7,
    panel: 0xedf1f1,
    paper: 0xffffff,
    text: 0x24363c,
    muted: 0x566970,
    line: 0xcbd6d9,
    active: 0xddedef,
    cyan: 0x006772,
    amber: 0x81510c,
    purple: 0x765299,
    green: 0x326342,
    red: 0xa3323a,
};

/// Interface chrome, labels and reading text: the platform UI sans.
pub const UI_FONT: &str = ".SystemUIFont";
/// Technical identity: paths, UUIDs, model ids, byte ranges, counts and editors.
pub const MONO_FONT: &str = "Menlo";

/// Type scale in points. Chrome stays compact; reading text is larger.
pub mod text {
    /// Section labels and badges.
    pub const CAPTION: f32 = 11.0;
    /// Monospace metadata lines.
    pub const META: f32 = 11.5;
    /// Default chrome, list rows and controls.
    pub const BODY: f32 = 13.0;
    /// Chat answers, questions and reading prose.
    pub const READING: f32 = 14.0;
    /// View titles in the document pane.
    pub const TITLE: f32 = 17.0;
    /// Welcome and empty-state headlines.
    pub const DISPLAY: f32 = 22.0;
}

/// Spacing steps in points (4-point rhythm).
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// Component sizes in points.
pub mod size {
    /// Navigation and list rows.
    pub const ROW: f32 = 28.0;
    /// Minimum status-strip height.
    pub const STATUS_STRIP: f32 = 26.0;
    /// Accent bar on callouts and selected rows.
    pub const ACCENT_BAR: f32 = 2.0;
    /// Corner radius: square panes and controls by approved direction.
    pub const RADIUS: u32 = 0;
    /// Readable line length for chat and reading panes.
    pub const READING_MAX_WIDTH: f32 = 760.0;
}

pub fn palette(scheme: Scheme) -> Palette {
    match scheme {
        Scheme::Dark => DARK,
        Scheme::Light => LIGHT,
    }
}

pub fn hex(value: u32) -> String {
    format!("#{value:06x}")
}

/// A gpui-component `ThemeConfig` document for the scheme. Installing it through
/// `Theme::change` resolves component colours, cached tokens and editor colours together.
pub fn theme_config_json(scheme: Scheme) -> serde_json::Value {
    let p = palette(scheme);
    let (name, mode) = match scheme {
        Scheme::Dark => ("BRN Dark", "dark"),
        Scheme::Light => ("BRN Light", "light"),
    };
    serde_json::json!({
        "is_default": false,
        "name": name,
        "mode": mode,
        "radius": size::RADIUS,
        "radius.lg": size::RADIUS,
        "shadow": false,
        "font.family": UI_FONT,
        "font.size": text::BODY,
        "mono_font.family": MONO_FONT,
        "colors": {
            "background": hex(p.background),
            "foreground": hex(p.text),
            "border": hex(p.line),
            "input.border": hex(p.line),
            "accent.background": hex(p.active),
            "accent.foreground": hex(p.text),
            "muted.background": hex(p.panel),
            "muted.foreground": hex(p.muted),
            "ring": hex(p.cyan),
            "caret": hex(p.cyan),
            "link": hex(p.cyan),
            "link.hover": hex(p.cyan),
            "link.active": hex(p.cyan),
            "selection.background": hex(p.active),
            "primary.background": hex(p.cyan),
            "primary.foreground": hex(p.background),
            "primary.hover.background": hex(p.cyan),
            "primary.active.background": hex(p.cyan),
            "secondary.background": hex(p.paper),
            "secondary.foreground": hex(p.text),
            "secondary.hover.background": hex(p.active),
            "secondary.active.background": hex(p.active),
            "button.background": hex(p.paper),
            "button.foreground": hex(p.text),
            "button.hover.background": hex(p.active),
            "button.active.background": hex(p.active),
            "list.background": hex(p.panel),
            "list.hover.background": hex(p.active),
            "list.active.background": hex(p.active),
            "list.active.border": hex(p.cyan),
            "popover.background": hex(p.panel),
            "popover.foreground": hex(p.text),
            "sidebar.background": hex(p.panel),
            "sidebar.foreground": hex(p.text),
            "sidebar.border": hex(p.line),
            "title_bar.background": hex(p.panel),
            "title_bar.border": hex(p.line),
            "tab.background": hex(p.panel),
            "tab.foreground": hex(p.muted),
            "tab.active.background": hex(p.paper),
            "tab.active.foreground": hex(p.cyan),
            "tab_bar.background": hex(p.panel),
            "success.background": hex(p.green),
            "danger.background": hex(p.red),
            "danger.foreground": hex(p.background),
            "base.red": hex(p.red),
            "warning.background": hex(p.amber),
            "info.background": hex(p.cyan),
            "base.cyan": hex(p.cyan),
            "base.green": hex(p.green),
            "base.yellow": hex(p.amber),
            "base.magenta": hex(p.purple),
            "window.border": hex(p.line)
        },
        "highlight": {
            "editor.background": hex(p.paper),
            "editor.foreground": hex(p.text),
            "editor.active_line.background": hex(p.active),
            "editor.line_number": hex(p.muted)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Scheme;

    #[test]
    fn dark_palette_matches_the_handoff() {
        assert_eq!(
            DARK,
            Palette {
                background: 0x101619,
                panel: 0x151e22,
                paper: 0x141c20,
                text: 0xe4ecef,
                muted: 0xa6b5bd,
                line: 0x35434a,
                active: 0x20353b,
                cyan: 0x71d8e7,
                amber: 0xefc378,
                purple: 0xc4acf2,
                green: 0x9cdbad,
                red: 0xf2a2a2,
            }
        );
    }

    #[test]
    fn light_palette_matches_the_handoff() {
        assert_eq!(
            LIGHT,
            Palette {
                background: 0xf5f7f7,
                panel: 0xedf1f1,
                paper: 0xffffff,
                text: 0x24363c,
                muted: 0x566970,
                line: 0xcbd6d9,
                active: 0xddedef,
                cyan: 0x006772,
                amber: 0x81510c,
                purple: 0x765299,
                green: 0x326342,
                red: 0xa3323a,
            }
        );
    }

    #[test]
    fn palette_follows_the_scheme() {
        assert_eq!(palette(Scheme::Dark), DARK);
        assert_eq!(palette(Scheme::Light), LIGHT);
    }

    #[test]
    fn hex_formats_six_lowercase_digits() {
        assert_eq!(hex(0x006772), "#006772");
        assert_eq!(hex(0xE4ECEF), "#e4ecef");
    }

    #[test]
    fn theme_config_covers_component_and_editor_colours() {
        let dark = theme_config_json(Scheme::Dark);
        assert_eq!(dark["mode"], "dark");
        assert_eq!(dark["radius"], 0);
        assert_eq!(dark["colors"]["background"], "#101619");
        assert_eq!(dark["colors"]["button.background"], "#141c20");
        assert_eq!(dark["colors"]["ring"], "#71d8e7");
        assert_eq!(dark["colors"]["base.magenta"], "#c4acf2");
        assert_eq!(dark["colors"]["danger.background"], "#f2a2a2");
        assert_eq!(dark["font.family"], UI_FONT);
        assert_eq!(dark["mono_font.family"], MONO_FONT);
        assert_eq!(dark["highlight"]["editor.background"], "#141c20");
        let light = theme_config_json(Scheme::Light);
        assert_eq!(light["mode"], "light");
        assert_eq!(light["colors"]["foreground"], "#24363c");
        assert_eq!(light["highlight"]["editor.foreground"], "#24363c");
    }
}
