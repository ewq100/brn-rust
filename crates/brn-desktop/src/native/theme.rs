//! Applies the BRN handoff palette to the gpui-component theme.

use crate::layout::{Appearance, Scheme};
use crate::tokens;
use gpui_kit::{
    App, Hsla, Window, WindowAppearance,
    component::{Theme, ThemeConfig, highlighter::HighlightTheme},
    rgb,
};
use std::rc::Rc;

pub fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

pub fn system_dark(window: &Window) -> bool {
    is_dark(window.appearance())
}

pub fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

pub fn config(scheme: Scheme) -> Result<ThemeConfig, String> {
    let mut config = tokens::theme_config_json(scheme);
    config.as_object_mut().unwrap().remove("highlight");
    let mut config: ThemeConfig = serde_json::from_value(config).map_err(|error| {
        format!("Theme tokens could not be applied ({error}); using the default theme.")
    })?;
    // Theme::change replaces the entire highlight style, so keep the toolkit's
    // scheme-matched syntax and override only the handoff editor colours.
    let mut highlight = match scheme {
        Scheme::Dark => HighlightTheme::default_dark(),
        Scheme::Light => HighlightTheme::default_light(),
    }
    .style
    .clone();
    let p = tokens::palette(scheme);
    highlight.editor_background = Some(color(p.paper));
    highlight.editor_foreground = Some(color(p.text));
    highlight.editor_active_line = Some(color(p.active));
    highlight.editor_line_number = Some(color(p.muted));
    config.highlight = Some(highlight);
    Ok(config)
}

/// Installs the handoff light and dark configs, then lets `Theme::change` resolve
/// component colours, cached tokens, editor highlight colours and the Base projection together.
pub fn apply(appearance: Appearance, window: &mut Window, cx: &mut App) -> Result<(), String> {
    let dark = Rc::new(config(Scheme::Dark)?);
    let light = Rc::new(config(Scheme::Light)?);
    {
        let theme = Theme::global_mut(cx);
        theme.dark_theme = dark;
        theme.light_theme = light;
        theme.font_family = tokens::UI_FONT.into();
        theme.mono_font_family = tokens::MONO_FONT.into();
    }
    let mode = match appearance.scheme(system_dark(window)) {
        Scheme::Dark => gpui_kit::component::ThemeMode::Dark,
        Scheme::Light => gpui_kit::component::ThemeMode::Light,
    };
    Theme::change(mode, Some(window), cx);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_dark_window_appearances_count_as_dark() {
        assert!(is_dark(WindowAppearance::Dark));
        assert!(is_dark(WindowAppearance::VibrantDark));
        assert!(!is_dark(WindowAppearance::Light));
        assert!(!is_dark(WindowAppearance::VibrantLight));
    }

    #[test]
    fn handoff_theme_configs_deserialize_for_both_schemes() {
        let dark = config(Scheme::Dark).unwrap();
        assert!(dark.mode.is_dark());
        assert_eq!(dark.radius, Some(0));
        assert!(dark.highlight.is_some());
        assert!(!config(Scheme::Light).unwrap().mode.is_dark());
    }

    #[test]
    fn handoff_editors_retain_syntax_colours_for_both_schemes() {
        for scheme in [Scheme::Dark, Scheme::Light] {
            let highlight = config(scheme).unwrap().highlight.unwrap();
            let syntax = serde_json::to_value(&highlight.syntax).unwrap();
            assert!(
                syntax
                    .as_object()
                    .unwrap()
                    .values()
                    .any(|style| !style.is_null())
            );
            assert_eq!(
                highlight.editor_background,
                Some(color(tokens::palette(scheme).paper))
            );
            assert_eq!(
                highlight.editor_foreground,
                Some(color(tokens::palette(scheme).text))
            );
            assert_eq!(
                highlight.editor_active_line,
                Some(color(tokens::palette(scheme).active))
            );
            assert_eq!(
                highlight.editor_line_number,
                Some(color(tokens::palette(scheme).muted))
            );
            let default = match scheme {
                Scheme::Dark => HighlightTheme::default_dark(),
                Scheme::Light => HighlightTheme::default_light(),
            };
            assert_eq!(highlight.syntax, default.style.syntax);
        }
    }
}
