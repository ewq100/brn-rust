//! Window layout preferences and pure layout resolution for the desktop shell.

use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidthBounds {
    pub default: f32,
    pub min: f32,
    pub max: f32,
}

impl WidthBounds {
    pub fn clamp(self, value: f32) -> f32 {
        if value.is_finite() {
            value.clamp(self.min, self.max)
        } else {
            self.default
        }
    }
}

pub const HISTORY: WidthBounds = WidthBounds {
    default: 220.0,
    min: 180.0,
    max: 320.0,
};
pub const VAULT: WidthBounds = WidthBounds {
    default: 260.0,
    min: 180.0,
    max: 360.0,
};
pub const DOC_MIN: f32 = 360.0;
pub const CHAT_MIN: f32 = 260.0;
pub const DOC_SHARE_DEFAULT: f32 = 0.58;
pub const DOC_SHARE_MIN: f32 = 0.2;
pub const DOC_SHARE_MAX: f32 = 0.8;
pub const COLLAPSED_RAIL: f32 = 28.0;
pub const DIVIDER: f32 = 5.0;
pub const WINDOW_MIN: f32 = 480.0;
pub const KEY_STEP: f32 = 8.0;
pub const KEY_STEP_LARGE: f32 = 32.0;
const SPLIT_MIN: f32 = DOC_MIN + DIVIDER + CHAT_MIN;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Dark,
    Light,
}

impl Appearance {
    pub fn scheme(self, system_dark: bool) -> Scheme {
        match self {
            Self::System if system_dark => Scheme::Dark,
            Self::System | Self::Light => Scheme::Light,
            Self::Dark => Scheme::Dark,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rail {
    History,
    Vault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Divider {
    History,
    Document,
    Vault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RailDisplay {
    Open,
    Collapsed,
    AutoCollapsed,
    Hidden,
}

impl RailDisplay {
    fn occupied(self, open_width: f32) -> f32 {
        match self {
            // An open rail is always rendered with its divider beside it.
            Self::Open => open_width + DIVIDER,
            Self::Collapsed | Self::AutoCollapsed => COLLAPSED_RAIL,
            Self::Hidden => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CentreMode {
    ChatOnly,
    Split,
    Tabs,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedLayout {
    pub history: RailDisplay,
    pub vault: RailDisplay,
    pub history_w: f32,
    pub vault_w: f32,
    pub centre_x: f32,
    pub centre_w: f32,
    pub mode: CentreMode,
    pub doc_w: f32,
    pub chat_w: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutState {
    pub history_w: f32,
    pub vault_w: f32,
    pub doc_share: f32,
    pub history_collapsed: bool,
    pub vault_collapsed: bool,
    pub focus: bool,
    pub appearance: Appearance,
}

impl Default for LayoutState {
    fn default() -> Self {
        Self {
            history_w: HISTORY.default,
            vault_w: VAULT.default,
            doc_share: DOC_SHARE_DEFAULT,
            history_collapsed: false,
            vault_collapsed: false,
            focus: false,
            appearance: Appearance::System,
        }
    }
}

impl LayoutState {
    pub fn sanitized(mut self) -> Self {
        self.history_w = HISTORY.clamp(self.history_w);
        self.vault_w = VAULT.clamp(self.vault_w);
        self.doc_share = if self.doc_share.is_finite() {
            self.doc_share.clamp(DOC_SHARE_MIN, DOC_SHARE_MAX)
        } else {
            DOC_SHARE_DEFAULT
        };
        self
    }

    /// Resolves display state for a window width. Never mutates saved preferences.
    pub fn resolve(&self, window_width: f32, has_document: bool) -> ResolvedLayout {
        let width = if window_width.is_finite() {
            window_width.max(0.0)
        } else {
            0.0
        };
        let centre_min = if has_document { SPLIT_MIN } else { CHAT_MIN };
        let initial = |collapsed: bool| {
            if self.focus {
                RailDisplay::Hidden
            } else if collapsed {
                RailDisplay::Collapsed
            } else {
                RailDisplay::Open
            }
        };
        let mut history = initial(self.history_collapsed);
        let mut vault = initial(self.vault_collapsed);
        let centre = |h: RailDisplay, v: RailDisplay| {
            width - h.occupied(self.history_w) - v.occupied(self.vault_w)
        };
        if centre(history, vault) < centre_min && vault == RailDisplay::Open {
            vault = RailDisplay::AutoCollapsed;
        }
        if centre(history, vault) < centre_min && history == RailDisplay::Open {
            history = RailDisplay::AutoCollapsed;
        }
        let centre_w = centre(history, vault).max(0.0);
        let mode = if !has_document {
            CentreMode::ChatOnly
        } else if centre_w < SPLIT_MIN {
            CentreMode::Tabs
        } else {
            CentreMode::Split
        };
        let (doc_w, chat_w) = match mode {
            CentreMode::Split => {
                let usable = centre_w - DIVIDER;
                let doc = (usable * self.doc_share).clamp(DOC_MIN, usable - CHAT_MIN);
                (doc, usable - doc)
            }
            CentreMode::Tabs => (centre_w, centre_w),
            CentreMode::ChatOnly => (0.0, centre_w),
        };
        ResolvedLayout {
            history,
            vault,
            history_w: history.occupied(self.history_w),
            vault_w: vault.occupied(self.vault_w),
            centre_x: history.occupied(self.history_w),
            centre_w,
            mode,
            doc_w,
            chat_w,
        }
    }

    /// Returns false when the rail is auto-collapsed and only a wider window can show it.
    pub fn toggle_rail(&mut self, rail: Rail, shown: RailDisplay) -> bool {
        let collapsed = match rail {
            Rail::History => &mut self.history_collapsed,
            Rail::Vault => &mut self.vault_collapsed,
        };
        if self.focus {
            *collapsed = false;
            self.focus = false;
            return true;
        }
        if shown == RailDisplay::AutoCollapsed {
            return false;
        }
        *collapsed = !*collapsed;
        true
    }

    pub fn toggle_focus(&mut self) {
        self.focus = !self.focus;
    }

    pub fn resize_rail(&mut self, rail: Rail, delta: f32) {
        match rail {
            Rail::History => self.history_w = HISTORY.clamp(self.history_w + delta),
            Rail::Vault => self.vault_w = VAULT.clamp(self.vault_w + delta),
        }
    }

    pub fn drag(
        &mut self,
        divider: Divider,
        pointer_x: f32,
        window_width: f32,
        resolved: &ResolvedLayout,
    ) {
        if !pointer_x.is_finite() || !window_width.is_finite() {
            return;
        }
        match divider {
            Divider::History => self.history_w = HISTORY.clamp(pointer_x),
            Divider::Vault => self.vault_w = VAULT.clamp(window_width - pointer_x),
            Divider::Document => self.set_doc_width(pointer_x - resolved.centre_x, resolved),
        }
    }

    /// Positive delta moves the divider right.
    pub fn nudge(&mut self, divider: Divider, delta: f32, resolved: &ResolvedLayout) {
        match divider {
            Divider::History => self.resize_rail(Rail::History, delta),
            Divider::Vault => self.resize_rail(Rail::Vault, -delta),
            Divider::Document => self.set_doc_width(resolved.doc_w + delta, resolved),
        }
    }

    pub fn reset_layout(&mut self) {
        *self = Self {
            appearance: self.appearance,
            ..Self::default()
        };
    }

    fn set_doc_width(&mut self, doc_w: f32, resolved: &ResolvedLayout) {
        if resolved.mode != CentreMode::Split || !doc_w.is_finite() {
            return;
        }
        let usable = resolved.centre_w - DIVIDER;
        let doc = doc_w.clamp(DOC_MIN, usable - CHAT_MIN);
        self.doc_share = (doc / usable).clamp(DOC_SHARE_MIN, DOC_SHARE_MAX);
    }
}

pub const FILE_NAME: &str = "layout.json";
pub const FILE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct LayoutFile {
    version: u32,
    layout: LayoutState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Loaded {
    Missing,
    Restored,
    /// Defaults are in use; the note explains why. The file is left untouched.
    Reset(String),
}

pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

pub fn load(data_dir: &Path) -> (LayoutState, Loaded) {
    let reset = |note: String| (LayoutState::default(), Loaded::Reset(note));
    let corrupt = || reset("Layout preferences were corrupt; using default layout.".into());
    let bytes = match std::fs::read(path(data_dir)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (LayoutState::default(), Loaded::Missing);
        }
        Err(error) => {
            return reset(format!(
                "Layout preferences were unreadable ({error}); using default layout."
            ));
        }
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return corrupt();
    };
    match value.get("version").and_then(serde_json::Value::as_u64) {
        Some(version) if version == u64::from(FILE_VERSION) => {}
        Some(version) => {
            return reset(format!(
                "Layout preferences version {version} is not supported; using default layout."
            ));
        }
        None => return corrupt(),
    }
    match serde_json::from_value::<LayoutFile>(value) {
        Ok(file) => (file.layout.sanitized(), Loaded::Restored),
        Err(_) => corrupt(),
    }
}

/// Writes atomically: a synced temporary file is renamed over `layout.json`.
pub fn save(data_dir: &Path, layout: &LayoutState) -> std::io::Result<()> {
    let file = LayoutFile {
        version: FILE_VERSION,
        layout: layout.clone().sanitized(),
    };
    let bytes = serde_json::to_vec_pretty(&file).map_err(std::io::Error::other)?;
    let temp = data_dir.join(format!("{FILE_NAME}.tmp"));
    let mut handle = std::fs::File::create(&temp)?;
    handle.write_all(&bytes)?;
    handle.sync_all()?;
    std::fs::rename(&temp, path(data_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn defaults_match_the_spec() {
        let state = LayoutState::default();
        assert_eq!(state.history_w, 220.0);
        assert_eq!(state.vault_w, 260.0);
        assert_eq!(state.doc_share, 0.58);
        assert!(!state.history_collapsed && !state.vault_collapsed && !state.focus);
        assert_eq!(state.appearance, Appearance::System);
    }

    #[test]
    fn wide_window_with_document_splits_and_keeps_rails() {
        let r = LayoutState::default().resolve(1400.0, true);
        assert_eq!((r.history, r.vault), (RailDisplay::Open, RailDisplay::Open));
        assert_eq!(r.mode, CentreMode::Split);
        assert!(close(r.centre_x, 220.0 + DIVIDER));
        assert!(close(r.centre_w, 1400.0 - 220.0 - 260.0 - 2.0 * DIVIDER));
        assert!(close(r.doc_w, (r.centre_w - DIVIDER) * 0.58));
        assert!(close(r.doc_w + DIVIDER + r.chat_w, r.centre_w));
    }

    #[test]
    fn vault_collapses_before_history() {
        let r = LayoutState::default().resolve(1000.0, true);
        assert_eq!(r.vault, RailDisplay::AutoCollapsed);
        assert_eq!(r.history, RailDisplay::Open);
        assert_eq!(r.mode, CentreMode::Split);
        assert!(close(r.centre_w, 1000.0 - 220.0 - DIVIDER - COLLAPSED_RAIL));
    }

    #[test]
    fn narrow_window_collapses_both_then_uses_tabs() {
        let r = LayoutState::default().resolve(600.0, true);
        assert_eq!(
            (r.history, r.vault),
            (RailDisplay::AutoCollapsed, RailDisplay::AutoCollapsed)
        );
        assert_eq!(r.mode, CentreMode::Tabs);
        assert!(close(r.centre_w, 600.0 - 2.0 * COLLAPSED_RAIL));
        assert!(close(r.doc_w, r.centre_w) && close(r.chat_w, r.centre_w));
    }

    #[test]
    fn chat_first_needs_only_the_chat_minimum() {
        let r = LayoutState::default().resolve(600.0, false);
        assert_eq!(r.mode, CentreMode::ChatOnly);
        assert_eq!(r.vault, RailDisplay::AutoCollapsed);
        assert_eq!(r.history, RailDisplay::Open);
        assert!(close(r.chat_w, 600.0 - 220.0 - DIVIDER - COLLAPSED_RAIL));
        assert_eq!(r.doc_w, 0.0);
    }

    #[test]
    fn boundary_widths_include_rail_dividers() {
        let exact = 220.0 + DIVIDER + 260.0 + DIVIDER + DOC_MIN + DIVIDER + CHAT_MIN;
        let r = LayoutState::default().resolve(exact, true);
        assert_eq!(
            (r.history, r.vault, r.mode),
            (RailDisplay::Open, RailDisplay::Open, CentreMode::Split)
        );
        assert!(close(r.doc_w, DOC_MIN) && close(r.chat_w, CHAT_MIN));
        assert!(close(r.history_w + r.centre_w + r.vault_w, exact));
        let r = LayoutState::default().resolve(exact - 1.0, true);
        assert_eq!(r.vault, RailDisplay::AutoCollapsed);
    }

    #[test]
    fn auto_collapse_never_changes_preferences() {
        let state = LayoutState::default();
        let before = state.clone();
        let _ = state.resolve(520.0, true);
        assert_eq!(state, before);
        let wide = state.resolve(1400.0, true);
        assert_eq!(
            (wide.history, wide.vault),
            (RailDisplay::Open, RailDisplay::Open)
        );
    }

    #[test]
    fn user_collapse_is_reported_as_collapsed_not_auto() {
        let state = LayoutState {
            history_collapsed: true,
            ..LayoutState::default()
        };
        let r = state.resolve(1400.0, true);
        assert_eq!(r.history, RailDisplay::Collapsed);
        assert!(close(r.centre_x, COLLAPSED_RAIL));
    }

    #[test]
    fn focus_hides_rails_and_restores_them() {
        let mut state = LayoutState {
            history_w: 250.0,
            ..LayoutState::default()
        };
        state.toggle_focus();
        let r = state.resolve(1400.0, true);
        assert_eq!(
            (r.history, r.vault),
            (RailDisplay::Hidden, RailDisplay::Hidden)
        );
        assert!(close(r.centre_w, 1400.0) && close(r.centre_x, 0.0));
        state.toggle_focus();
        let r = state.resolve(1400.0, true);
        assert_eq!((r.history, r.vault), (RailDisplay::Open, RailDisplay::Open));
        assert_eq!(state.history_w, 250.0);
    }

    #[test]
    fn toggling_a_rail_in_focus_exits_focus_and_shows_it() {
        let mut state = LayoutState {
            focus: true,
            history_collapsed: true,
            ..LayoutState::default()
        };
        assert!(state.toggle_rail(Rail::History, RailDisplay::Hidden));
        assert!(!state.focus && !state.history_collapsed);
    }

    #[test]
    fn toggling_flips_the_preference() {
        let mut state = LayoutState::default();
        assert!(state.toggle_rail(Rail::Vault, RailDisplay::Open));
        assert!(state.vault_collapsed);
        assert!(state.toggle_rail(Rail::Vault, RailDisplay::Collapsed));
        assert!(!state.vault_collapsed);
    }

    #[test]
    fn toggling_an_auto_collapsed_rail_reports_width_needed() {
        let mut state = LayoutState::default();
        assert!(!state.toggle_rail(Rail::Vault, RailDisplay::AutoCollapsed));
        assert_eq!(state, LayoutState::default());
    }

    #[test]
    fn split_respects_minimums_for_extreme_shares() {
        let state = LayoutState {
            doc_share: DOC_SHARE_MAX,
            ..LayoutState::default()
        };
        let r = state.resolve(220.0 + 260.0 + 2.0 * DIVIDER + 640.0, true);
        assert_eq!(r.mode, CentreMode::Split);
        assert!(r.chat_w >= CHAT_MIN - 0.01);
        let state = LayoutState {
            doc_share: DOC_SHARE_MIN,
            ..LayoutState::default()
        };
        let r = state.resolve(1400.0, true);
        assert!(r.doc_w >= DOC_MIN - 0.01);
    }

    #[test]
    fn dragging_rail_dividers_clamps_widths() {
        let mut state = LayoutState::default();
        let r = state.resolve(1400.0, true);
        state.drag(Divider::History, 1000.0, 1400.0, &r);
        assert_eq!(state.history_w, HISTORY.max);
        state.drag(Divider::History, 10.0, 1400.0, &r);
        assert_eq!(state.history_w, HISTORY.min);
        state.drag(Divider::Vault, 1200.0, 1400.0, &r);
        assert_eq!(state.vault_w, 200.0);
        state.drag(Divider::Vault, f32::NAN, 1400.0, &r);
        assert_eq!(state.vault_w, 200.0);
    }

    #[test]
    fn dragging_the_document_divider_sets_the_share() {
        let mut state = LayoutState::default();
        let r = state.resolve(1400.0, true);
        state.drag(Divider::Document, r.centre_x + 400.0, 1400.0, &r);
        assert!(close(state.doc_share, 400.0 / (r.centre_w - DIVIDER)));
        let after = state.resolve(1400.0, true);
        assert!(close(after.doc_w, 400.0));
    }

    #[test]
    fn keyboard_nudges_move_dividers_in_pointer_direction() {
        let mut state = LayoutState::default();
        let r = state.resolve(1400.0, true);
        state.nudge(Divider::History, KEY_STEP, &r);
        assert_eq!(state.history_w, 228.0);
        state.nudge(Divider::Vault, KEY_STEP, &r);
        assert_eq!(state.vault_w, 252.0);
        let r = state.resolve(1400.0, true);
        let before = r.doc_w;
        state.nudge(Divider::Document, -KEY_STEP_LARGE, &r);
        assert!(close(
            state.resolve(1400.0, true).doc_w,
            before - KEY_STEP_LARGE
        ));
    }

    #[test]
    fn document_nudge_is_ignored_outside_split() {
        let mut state = LayoutState::default();
        let r = state.resolve(600.0, true);
        state.nudge(Divider::Document, KEY_STEP, &r);
        assert_eq!(state.doc_share, DOC_SHARE_DEFAULT);
    }

    #[test]
    fn resize_rail_steps_and_clamps() {
        let mut state = LayoutState::default();
        state.resize_rail(Rail::Vault, KEY_STEP);
        assert_eq!(state.vault_w, 268.0);
        state.resize_rail(Rail::History, -1000.0);
        assert_eq!(state.history_w, HISTORY.min);
    }

    #[test]
    fn sanitized_clamps_and_replaces_non_finite_values() {
        let state = LayoutState {
            history_w: 5000.0,
            vault_w: f32::NAN,
            doc_share: 2.0,
            ..LayoutState::default()
        }
        .sanitized();
        assert_eq!(state.history_w, HISTORY.max);
        assert_eq!(state.vault_w, VAULT.default);
        assert_eq!(state.doc_share, DOC_SHARE_MAX);
    }

    #[test]
    fn reset_layout_keeps_appearance() {
        let mut state = LayoutState {
            history_w: 300.0,
            focus: true,
            vault_collapsed: true,
            appearance: Appearance::Light,
            ..LayoutState::default()
        };
        state.reset_layout();
        assert_eq!(
            state,
            LayoutState {
                appearance: Appearance::Light,
                ..LayoutState::default()
            }
        );
    }

    #[test]
    fn appearance_resolves_against_the_system() {
        assert_eq!(Appearance::System.scheme(true), Scheme::Dark);
        assert_eq!(Appearance::System.scheme(false), Scheme::Light);
        assert_eq!(Appearance::Dark.scheme(false), Scheme::Dark);
        assert_eq!(Appearance::Light.scheme(true), Scheme::Light);
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("brn-layout-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_file_loads_defaults_silently() {
        let dir = scratch("missing");
        assert_eq!(load(&dir), (LayoutState::default(), Loaded::Missing));
    }

    #[test]
    fn saved_layout_round_trips_without_leaving_a_temp_file() {
        let dir = scratch("round-trip");
        let state = LayoutState {
            history_w: 250.0,
            vault_collapsed: true,
            focus: true,
            appearance: Appearance::Dark,
            ..LayoutState::default()
        };
        save(&dir, &state).unwrap();
        assert_eq!(load(&dir), (state, Loaded::Restored));
        assert!(!dir.join(format!("{FILE_NAME}.tmp")).exists());
        let text = std::fs::read_to_string(path(&dir)).unwrap();
        assert!(text.contains("\"version\": 1"));
    }

    #[test]
    fn corrupt_file_resets_with_a_note_and_is_not_rewritten() {
        let dir = scratch("corrupt");
        std::fs::write(path(&dir), b"{not json").unwrap();
        let (state, loaded) = load(&dir);
        assert_eq!(state, LayoutState::default());
        assert!(matches!(loaded, Loaded::Reset(ref note) if note.contains("corrupt")));
        assert_eq!(std::fs::read(path(&dir)).unwrap(), b"{not json");
    }

    #[test]
    fn unknown_version_resets_with_a_note() {
        let dir = scratch("version");
        std::fs::write(path(&dir), br#"{"version":2,"layout":{}}"#).unwrap();
        let (state, loaded) = load(&dir);
        assert_eq!(state, LayoutState::default());
        assert!(matches!(loaded, Loaded::Reset(ref note) if note.contains("version 2")));
    }

    #[test]
    fn partial_or_out_of_range_fields_are_defaulted_and_clamped() {
        let dir = scratch("partial");
        std::fs::write(
            path(&dir),
            br#"{"version":1,"layout":{"history_w":9999,"appearance":"light"}}"#,
        )
        .unwrap();
        let (state, loaded) = load(&dir);
        assert_eq!(loaded, Loaded::Restored);
        assert_eq!(state.history_w, HISTORY.max);
        assert_eq!(state.vault_w, VAULT.default);
        assert_eq!(state.appearance, Appearance::Light);
    }
}
