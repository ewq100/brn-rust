//! Applies the BRN handoff palette to the gpui-component theme.

use crate::layout::{Appearance, Scheme};
use crate::tokens;
use gpui_kit::{
    App, Hsla, Window, WindowAppearance,
    component::{Theme, ThemeConfig},
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
    // gpui-component 0.6.6 requires a syntax map even without syntax overrides.
    config["highlight"]["syntax"] = serde_json::json!({});
    serde_json::from_value(config).map_err(|error| {
        format!("Theme tokens could not be applied ({error}); using the default theme.")
    })
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
        theme.font_family = tokens::CHROME_FONT.into();
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
}
