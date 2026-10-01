# Workspace Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Follow the current user's delegation preference; inline execution is the repository default. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the single-column page switcher in `brn-desktop` with the handoff's combined workspace. History is on the left, the document and chat share the centre, and the vault is on the right, all in the refined-terminal design. Existing workflow behaviour does not change.

**Architecture:** A GPUI-free `layout` module owns preferences, pure layout resolution and `layout.json` persistence. A GPUI-free `tokens` module owns the handoff palette. Both are unit-tested under default features. `native.rs` becomes `native/mod.rs`. Region render code lives in `native/shell/*` as `impl Desktop` blocks, so moved code keeps using `self.` unchanged. `Desktop` stays the single owner of the worker, `Phase`, generations and dirty guards.

**Tech Stack:** Rust 1.98.1 (pinned), GPUI Kit 0.6.6 / gpui-component 0.6.6 (`TitleBar`, `Theme`, `Button`, `Editor`, `Input`), serde + serde_json.

**Spec:** [Workspace shell design](../../../superpowers/specs/2026-10-01-workspace-shell-design.md). Read it together with this plan. Handoff reference: `/Users/evokessler/repos/brn/brn-ui-handoff` (read-only).

## Global Constraints

- Baseline: `main` at `9af18f5`. The written specification was approved on 1 October 2026. This is planning only; implementation has not started.
- Execute in isolation using `using-git-worktrees` at execution time. Preserve unrelated changes, including the active `markdown-note-editing` work.
- No new workflow, store, CLI or provider capability. UI calls go through the existing `Desktop::submit`/worker paths only.
- Move workflow calls and messages; do not rewrite them. Preserve dirty-state guards (`draft_guard`, `close_guard`), generation-aware handling, the exact-quote comment flows, and ⌘Q and close-window guards.
- Region defaults, minimums and maximums in points: history 220/180/320, vault 260/180/360, document 58% of the centre with a 360 minimum, chat 260 minimum, collapsed rail 28, divider 5. The window opens at 1100×800 with a 480×480 minimum.
- Rails auto-collapse (vault first, then history). The centre falls back to Document | Chat tabs below the document+divider+chat minimum, but only when a document is open. Auto-collapse never overwrites the saved preference.
- `layout.json` (`{"version": 1, "layout": {...}}`) lives in the explicit data directory:
  - Missing: defaults, silently.
  - Corrupt or unknown version: defaults plus a status note. Do not rewrite until the next user layout change.
  - Never store credentials or content. Unsent composer text is not persisted.
- Appearance is System (default), Dark or Light. Use exactly the handoff dark and light token values. Every status colour is paired with text.
- Shortcuts:

  | Shortcut | Action |
  | --- | --- |
  | ⌘0 | History |
  | ⌥⌘0 | Vault |
  | ⇧⌘↩ | Focus |
  | ⌘L | Composer |
  | ⌘N | New chat |
  | ⌘, | Settings |
  | ⌘. | Cancel |
  | Esc | Close dialog |

  Dividers move 8 pt with ←/→ and 32 pt with ⇧.
- Hide every handoff element without a backend. Do not ship fixture data as connected behaviour.
- Open rails are budgeted with their 5 pt divider in `LayoutState::resolve`. Rendered widths must equal resolved widths.
- Visible selection uses `Selectable::selected`. `Button::toggled` is accessibility metadata only in the pinned toolkit, so use it in addition for genuine toggles (Focus, Vault).
- Each scrollable region owns a persistent `ScrollHandle` in `Desktop`. Draft and source documents use separate handles.
- If a GPUI Kit signature named here differs slightly from the pinned crate, adapt the call to compile without changing the described behaviour, and note it in evidence.
- Use disposable explicit data directories for native checks. No live provider calls, model downloads, original-vault access, merge or release.
- Respect the pinned toolchain and lockfiles. Only `crates/brn-desktop/Cargo.toml` and the `brn-desktop` entry in `Cargo.lock` may change.
- Every commit includes `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.

## File ownership and task order

| Files | Responsibility | Task |
| --- | --- | --- |
| `crates/brn-desktop/Cargo.toml`, `Cargo.lock`, `src/main.rs`, `src/layout.rs` | Layout preferences, pure resolution and mutations | 1 |
| `src/layout.rs` (persistence), `src/tokens.rs` | `layout.json` load/save, palette tokens | 2 |
| `src/native/mod.rs` (moved from `src/native.rs`), `src/native/theme.rs` | Theme application, appearance observation, titlebar window options | 3 |
| `src/native/shell/{mod,header,history_rail,centre,vault_rail,settings}.rs`, `src/native/mod.rs` | Region rendering, content moves, collapse, Focus, tabs, settings dialog | 4 |
| `src/native/shell/divider.rs`, `src/native/mod.rs` | Drag/keyboard dividers, actions, menus, shortcuts | 5 |
| `docs/…`, `crates/brn-desktop/README.md` | Backlog, decision record, architecture/status/evidence | 6 |

Tasks run in order. Each ends with a commit that builds and passes its checks.

---

### Task 1: Layout model (GPUI-free)

**Files:**
- Modify: `crates/brn-desktop/Cargo.toml`
- Modify: `Cargo.lock` (only the `brn-desktop` dependency list)
- Modify: `crates/brn-desktop/src/main.rs:6-11`
- Create: `crates/brn-desktop/src/layout.rs`

**Interfaces:**
- Consumes: none.
- Produces (all `pub` in `crate::layout`):
  - `struct WidthBounds { default: f32, min: f32, max: f32 }` with `fn clamp(self, f32) -> f32`
  - consts `HISTORY`, `VAULT: WidthBounds`
  - `f32` consts `DOC_MIN`, `CHAT_MIN`, `DOC_SHARE_DEFAULT`, `DOC_SHARE_MIN`, `DOC_SHARE_MAX`, `COLLAPSED_RAIL`, `DIVIDER`, `WINDOW_MIN`, `KEY_STEP`, `KEY_STEP_LARGE`
  - `enum Appearance { System, Dark, Light }` with `fn scheme(self, system_dark: bool) -> Scheme`
  - `enum Scheme { Dark, Light }`, `enum Rail { History, Vault }`, `enum Divider { History, Document, Vault }`
  - `enum RailDisplay { Open, Collapsed, AutoCollapsed, Hidden }`, `enum CentreMode { ChatOnly, Split, Tabs }`
  - `struct ResolvedLayout { history, vault: RailDisplay, history_w, vault_w, centre_x, centre_w, doc_w, chat_w: f32, mode: CentreMode }`. Here `history_w`/`vault_w` are the space occupied: an open rail's width plus its 5 pt divider, or `COLLAPSED_RAIL`, or 0.
  - `struct LayoutState { history_w, vault_w, doc_share: f32, history_collapsed, vault_collapsed, focus: bool, appearance: Appearance }` with:
    - `Default`
    - `fn sanitized(self) -> Self`
    - `fn resolve(&self, window_width: f32, has_document: bool) -> ResolvedLayout`
    - `fn toggle_rail(&mut self, Rail, shown: RailDisplay) -> bool`
    - `fn toggle_focus(&mut self)`
    - `fn resize_rail(&mut self, Rail, delta: f32)`
    - `fn drag(&mut self, Divider, pointer_x: f32, window_width: f32, &ResolvedLayout)`
    - `fn nudge(&mut self, Divider, delta: f32, &ResolvedLayout)`
    - `fn reset_layout(&mut self)`

- [ ] **Step 1: Make serde available under default features**

In `crates/brn-desktop/Cargo.toml`, remove `"dep:serde_json"` from the `native-ui` feature list and make the dependencies unconditional:

```toml
[features]
default = []
native-ui = ["dep:gpui-kit", "dep:brn-workflow", "dep:uuid", "dep:sha2"]
native-retrieval = ["native-ui", "brn-workflow/native-retrieval"]

[dependencies]
brn-core = { path = "../brn-core" }
gpui-kit = { version = "=0.6.6", optional = true }
brn-workflow = { path = "../brn-workflow", optional = true }
uuid = { version = "1", features = ["v4"], optional = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = { version = "0.10", optional = true }
```

Run: `cargo +1.98.1 check -p brn-desktop --offline`
Expected: success. `git diff Cargo.lock` shows only `"serde",` added to the `brn-desktop` dependency list. If anything else changes in `Cargo.lock`, stop and report.

- [ ] **Step 2: Register the module under default features**

In `crates/brn-desktop/src/main.rs`, after the existing `use` lines and before `#[cfg(feature = "native-ui")] mod comments;`, add:

```rust
// Temporarily unconditional: native code consumes this module gradually (Tasks 3–5).
// Task 5 Step 5 narrows it to non-native builds.
#[allow(dead_code)]
mod layout;
```

- [ ] **Step 3: Write the failing tests**

Create `crates/brn-desktop/src/layout.rs` containing only the test module for now:

```rust
//! Window layout preferences and pure layout resolution for the desktop shell.

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
        assert_eq!((r.history, r.vault), (RailDisplay::AutoCollapsed, RailDisplay::AutoCollapsed));
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
        assert_eq!((r.history, r.vault, r.mode), (RailDisplay::Open, RailDisplay::Open, CentreMode::Split));
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
        assert_eq!((wide.history, wide.vault), (RailDisplay::Open, RailDisplay::Open));
    }

    #[test]
    fn user_collapse_is_reported_as_collapsed_not_auto() {
        let state = LayoutState { history_collapsed: true, ..LayoutState::default() };
        let r = state.resolve(1400.0, true);
        assert_eq!(r.history, RailDisplay::Collapsed);
        assert!(close(r.centre_x, COLLAPSED_RAIL));
    }

    #[test]
    fn focus_hides_rails_and_restores_them() {
        let mut state = LayoutState { history_w: 250.0, ..LayoutState::default() };
        state.toggle_focus();
        let r = state.resolve(1400.0, true);
        assert_eq!((r.history, r.vault), (RailDisplay::Hidden, RailDisplay::Hidden));
        assert!(close(r.centre_w, 1400.0) && close(r.centre_x, 0.0));
        state.toggle_focus();
        let r = state.resolve(1400.0, true);
        assert_eq!((r.history, r.vault), (RailDisplay::Open, RailDisplay::Open));
        assert_eq!(state.history_w, 250.0);
    }

    #[test]
    fn toggling_a_rail_in_focus_exits_focus_and_shows_it() {
        let mut state = LayoutState { focus: true, history_collapsed: true, ..LayoutState::default() };
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
        let state = LayoutState { doc_share: DOC_SHARE_MAX, ..LayoutState::default() };
        let r = state.resolve(220.0 + 260.0 + 2.0 * DIVIDER + 640.0, true);
        assert_eq!(r.mode, CentreMode::Split);
        assert!(r.chat_w >= CHAT_MIN - 0.01);
        let state = LayoutState { doc_share: DOC_SHARE_MIN, ..LayoutState::default() };
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
        assert!(close(state.resolve(1400.0, true).doc_w, before - KEY_STEP_LARGE));
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
        assert_eq!(state, LayoutState { appearance: Appearance::Light, ..LayoutState::default() });
    }

    #[test]
    fn appearance_resolves_against_the_system() {
        assert_eq!(Appearance::System.scheme(true), Scheme::Dark);
        assert_eq!(Appearance::System.scheme(false), Scheme::Light);
        assert_eq!(Appearance::Dark.scheme(false), Scheme::Dark);
        assert_eq!(Appearance::Light.scheme(true), Scheme::Light);
    }
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo +1.98.1 test -p brn-desktop --locked --offline layout::`
Expected: compile errors for `LayoutState`, `Appearance` and the other missing items.

- [ ] **Step 5: Implement the module**

Insert above the `#[cfg(test)]` block in `layout.rs`:

```rust
use serde::{Deserialize, Serialize};

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

pub const HISTORY: WidthBounds = WidthBounds { default: 220.0, min: 180.0, max: 320.0 };
pub const VAULT: WidthBounds = WidthBounds { default: 260.0, min: 180.0, max: 360.0 };
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
        let width = if window_width.is_finite() { window_width.max(0.0) } else { 0.0 };
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

    pub fn drag(&mut self, divider: Divider, pointer_x: f32, window_width: f32, resolved: &ResolvedLayout) {
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
        *self = Self { appearance: self.appearance, ..Self::default() };
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
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo +1.98.1 test -p brn-desktop --locked --offline layout::`
Expected: 21 tests pass.

Then run: `cargo +1.98.1 fmt --all -- --check && cargo +1.98.1 clippy -p brn-desktop --all-targets --locked --offline -- -D warnings && cargo +1.98.1 clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings`
Expected: success. If `fmt` reports differences, run `cargo +1.98.1 fmt --all` and re-check.

- [ ] **Step 7: Commit**

```bash
git add crates/brn-desktop/Cargo.toml Cargo.lock crates/brn-desktop/src/main.rs crates/brn-desktop/src/layout.rs
git commit -m "desktop: add GPUI-free workspace layout model

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Layout persistence and palette tokens (GPUI-free)

**Files:**
- Modify: `crates/brn-desktop/src/layout.rs`
- Create: `crates/brn-desktop/src/tokens.rs`
- Modify: `crates/brn-desktop/src/main.rs` (module list)

**Interfaces:**
- Consumes: `LayoutState`, `LayoutState::sanitized`, `Scheme` (Task 1).
- Produces:
  - `crate::layout::{FILE_NAME: &str, FILE_VERSION: u32}`
  - `enum Loaded { Missing, Restored, Reset(String) }`
  - `fn path(data_dir: &Path) -> PathBuf`
  - `fn load(data_dir: &Path) -> (LayoutState, Loaded)`
  - `fn save(data_dir: &Path, layout: &LayoutState) -> std::io::Result<()>`
  - `crate::tokens::{Palette { background, panel, paper, text, muted, line, active, cyan, amber, purple, green: u32 }, DARK, LIGHT, fn palette(Scheme) -> Palette, fn hex(u32) -> String, fn theme_config_json(Scheme) -> serde_json::Value, CHROME_FONT, READING_FONT}`

- [ ] **Step 1: Write the failing persistence tests**

Append inside `mod tests` in `layout.rs`:

```rust
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
        std::fs::write(path(&dir), br#"{"version":1,"layout":{"history_w":9999,"appearance":"light"}}"#).unwrap();
        let (state, loaded) = load(&dir);
        assert_eq!(loaded, Loaded::Restored);
        assert_eq!(state.history_w, HISTORY.max);
        assert_eq!(state.vault_w, VAULT.default);
        assert_eq!(state.appearance, Appearance::Light);
    }
```

- [ ] **Step 2: Write the failing token tests**

Create `crates/brn-desktop/src/tokens.rs` with only:

```rust
//! BRN handoff colour and font tokens for the refined-terminal direction.

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
        assert_eq!(dark["highlight"]["editor.background"], "#141c20");
        let light = theme_config_json(Scheme::Light);
        assert_eq!(light["mode"], "light");
        assert_eq!(light["colors"]["foreground"], "#24363c");
        assert_eq!(light["highlight"]["editor.foreground"], "#24363c");
    }
}
```

In `main.rs`, below `mod layout;`, add:

```rust
// Temporarily unconditional; Task 5 Step 5 narrows it to non-native builds.
#[allow(dead_code)]
mod tokens;
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo +1.98.1 test -p brn-desktop --locked --offline -- layout:: tokens::`
Expected: compile errors for `load`, `save`, `path`, `Loaded`, `FILE_NAME`, `Palette`, `DARK`, `LIGHT`, `palette`, `hex` and `theme_config_json`.

- [ ] **Step 4: Implement persistence**

In `layout.rs`, extend the imports and add after the `impl LayoutState` block:

```rust
use std::io::Write;
use std::path::{Path, PathBuf};

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
        Err(error) => return reset(format!("Layout preferences were unreadable ({error}); using default layout.")),
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return corrupt();
    };
    match value.get("version").and_then(serde_json::Value::as_u64) {
        Some(version) if version == u64::from(FILE_VERSION) => {}
        Some(version) => {
            return reset(format!("Layout preferences version {version} is not supported; using default layout."));
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
    let file = LayoutFile { version: FILE_VERSION, layout: layout.clone().sanitized() };
    let bytes = serde_json::to_vec_pretty(&file).map_err(std::io::Error::other)?;
    let temp = data_dir.join(format!("{FILE_NAME}.tmp"));
    let mut handle = std::fs::File::create(&temp)?;
    handle.write_all(&bytes)?;
    handle.sync_all()?;
    std::fs::rename(&temp, path(data_dir))
}
```

Keep all `use` lines together at the top of the file.

- [ ] **Step 5: Implement tokens**

Insert above the test module in `tokens.rs`:

```rust
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
};

/// Interface chrome, paths, statuses and controls.
pub const CHROME_FONT: &str = "Menlo";
/// Editorial reading surface.
pub const READING_FONT: &str = "Georgia";

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
        "radius": 0,
        "radius.lg": 0,
        "shadow": false,
        "mono_font.family": CHROME_FONT,
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
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo +1.98.1 test -p brn-desktop --locked --offline -- layout:: tokens::`
Expected: 31 tests pass (21 layout, 5 persistence, 5 tokens).

Run the same `fmt`/`clippy` (default and `native-ui`) commands as Task 1 Step 6. Expected: success.

- [ ] **Step 7: Commit**

```bash
git add crates/brn-desktop/src/layout.rs crates/brn-desktop/src/tokens.rs crates/brn-desktop/src/main.rs
git commit -m "desktop: persist layout preferences and add handoff tokens

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 3: Native theme, titlebar and appearance

**Files:**
- Move: `crates/brn-desktop/src/native.rs` → `crates/brn-desktop/src/native/mod.rs`
- Create: `crates/brn-desktop/src/native/theme.rs`
- Modify: `crates/brn-desktop/src/native/mod.rs` (imports, `Desktop` fields, `Desktop::new`, `render` root, `run`)

**Interfaces:**
- Consumes: `layout::{load, Loaded, LayoutState, Appearance, Scheme, WINDOW_MIN}`, `tokens::{palette, Palette, CHROME_FONT}`.
- Produces:
  - `native::theme::{fn is_dark(WindowAppearance) -> bool, fn system_dark(&Window) -> bool, fn color(u32) -> Hsla, fn config(Scheme) -> Result<ThemeConfig, String>, fn apply(Appearance, &mut Window, &mut App) -> Result<(), String>}`
  - new `Desktop` fields: `layout: LayoutState`, `system_dark: bool`, `layout_note: Option<String>`

- [ ] **Step 1: Move the module without changes**

```bash
mkdir -p crates/brn-desktop/src/native
git mv crates/brn-desktop/src/native.rs crates/brn-desktop/src/native/mod.rs
cargo +1.98.1 build -p brn-desktop --features native-ui --locked --offline
```

Expected: build succeeds. `mod native;` in `main.rs` resolves `native/mod.rs` unchanged.

- [ ] **Step 2: Write the failing theme test**

Create `crates/brn-desktop/src/native/theme.rs`:

```rust
//! Applies the BRN handoff palette to the gpui-component theme.

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
```

Add `mod theme;` at the top of `native/mod.rs`, below the existing `use` block.

Run: `cargo +1.98.1 test -p brn-desktop --features native-ui --locked --offline theme::`
Expected: compile errors, `is_dark` and `config` not found.

- [ ] **Step 3: Implement the theme module**

Insert above the test module in `native/theme.rs`:

```rust
use crate::layout::{Appearance, Scheme};
use crate::tokens;
use gpui_kit::{
    App, Hsla, Window, WindowAppearance,
    component::{Theme, ThemeConfig},
    rgb,
};
use std::rc::Rc;

pub fn is_dark(appearance: WindowAppearance) -> bool {
    matches!(appearance, WindowAppearance::Dark | WindowAppearance::VibrantDark)
}

pub fn system_dark(window: &Window) -> bool {
    is_dark(window.appearance())
}

pub fn color(hex: u32) -> Hsla {
    rgb(hex).into()
}

pub fn config(scheme: Scheme) -> Result<ThemeConfig, String> {
    serde_json::from_value(tokens::theme_config_json(scheme))
        .map_err(|error| format!("Theme tokens could not be applied ({error}); using the default theme."))
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
```

`Theme::change` calls `apply_config` for the selected config and rebuilds the Base theme, so no field is written after it. If `ThemeConfig` is not re-exported at `gpui_kit::component`, import it from `gpui_kit::component::theme`.

Run: `cargo +1.98.1 test -p brn-desktop --features native-ui --locked --offline theme::`
Expected: PASS.

- [ ] **Step 4: Load layout and follow appearance in `Desktop`**

In `native/mod.rs`:

1. Add the imports:

   ```rust
   use crate::layout::{self, LayoutState, Loaded};
   ```

   Also add `component::TitleBar` to the existing `gpui_kit::component::{...}` import list.

2. Add these fields to `struct Desktop`, after `config: Config,`:

   ```rust
       layout: LayoutState,
       system_dark: bool,
       layout_note: Option<String>,
   ```

3. In `Desktop::new`, before `Self {`, add:

   ```rust
           let (layout, loaded) = layout::load(&path);
           let layout_note = match loaded {
               Loaded::Reset(note) => Some(note),
               Loaded::Missing | Loaded::Restored => None,
           };
           let system_dark = theme::system_dark(window);
           let theme_error = theme::apply(layout.appearance, window, cx).err();
           let layout_note = layout_note.or(theme_error);
           let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
               this.system_dark = theme::system_dark(window);
               if let Err(error) = theme::apply(this.layout.appearance, window, cx) {
                   this.message = error;
               }
               cx.notify();
           });
   ```

4. Initialise the new fields in the `Self { ... }` literal, after `config,`:

   ```rust
               layout,
               system_dark,
               layout_note,
   ```

5. Add `appearance_subscription` to the existing `_subscriptions: vec![...]` list.

- [ ] **Step 5: Put the header in the transparent titlebar**

In `impl Render for Desktop`, change the returned root so that:
- the old `.child("BRN · local grounded research")` becomes the first child, `TitleBar::new().child("brn / workspace")`;
- the remaining children move into an inner padded column.

Replace:

```rust
        div()
            .id("brn-desktop")
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .child("BRN · local grounded research")
```

with:

```rust
        let mut content = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .gap_3()
            .p_3();
        if let Some(note) = &self.layout_note {
            content = content.child(note.clone());
        }
        div()
            .id("brn-desktop")
            .size_full()
            .flex()
            .flex_col()
            .font_family(crate::tokens::CHROME_FONT)
            .child(TitleBar::new().child("brn / workspace"))
            .child(
                content
```

Close the new `.child(content ...)` call after the existing final `.child(body)`. The page buttons, status, message, cancel and body chain onto `content` unchanged.

- [ ] **Step 6: Use titlebar window options and the 480×480 minimum**

In `run`, replace the `WindowOptions { ... }` literal with:

```rust
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        window_min_size: Some(size(px(layout::WINDOW_MIN), px(layout::WINDOW_MIN))),
                        ..TitleBar::window_options()
                    },
```

- [ ] **Step 7: Verify**

Run:

```bash
cargo +1.98.1 fmt --all -- --check
cargo +1.98.1 clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings
cargo +1.98.1 test -p brn-desktop --features native-ui --locked --offline
```

Expected: all pass. The existing native tests stay green.

Manual native check (unlocked Mac, disposable data):

```bash
data="$(mktemp -d "${TMPDIR:-/tmp}/brn-shell.XXXXXX")"
cargo +1.98.1 run -p brn-desktop --features native-ui --locked --offline -- --data-dir "$data"
```

Observe and record in evidence:
- traffic lights sit in the `brn / workspace` strip without overlap;
- the window drags by the strip;
- colours match the handoff dark palette when macOS is dark, and switch to light when System Settings → Appearance changes, without relaunching;
- square buttons;
- the window shrinks to 480×480;
- existing pages still work.

Then `echo '{bad' > "$data/layout.json"`, relaunch, and confirm the corrupt-layout note appears.

- [ ] **Step 8: Commit**

```bash
git add crates/brn-desktop/src/native
git commit -m "desktop: apply handoff theme, titlebar and system appearance

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 4: Workspace regions, content moves, Focus, tabs and Settings

This task replaces the `Page` switcher. The moved code keeps its `self.` references because every region is an `impl Desktop` block in a descendant module of `native`. Descendant modules can see `native`'s private items, and `use super::*;` imports them.

**Files:**
- Modify: `crates/brn-desktop/src/native/mod.rs`
- Create: `crates/brn-desktop/src/native/shell/mod.rs`
- Create: `crates/brn-desktop/src/native/shell/header.rs`
- Create: `crates/brn-desktop/src/native/shell/history_rail.rs`
- Create: `crates/brn-desktop/src/native/shell/vault_rail.rs`
- Create: `crates/brn-desktop/src/native/shell/centre.rs`
- Create: `crates/brn-desktop/src/native/shell/settings.rs`

**Interfaces:**
- Consumes:
  - `LayoutState::{resolve, toggle_rail, toggle_focus, resize_rail, reset_layout}`, `ResolvedLayout`, `RailDisplay`, `CentreMode`, `Rail`, `Appearance`, `layout::save`, `layout::COLLAPSED_RAIL`, `layout::DIVIDER`, `layout::KEY_STEP` (Tasks 1–2);
  - `theme::{apply, color}`, `tokens::{palette, Palette, CHROME_FONT, READING_FONT}` (Tasks 2–3);
  - `Desktop` fields `layout`, `system_dark`, `layout_note` (Task 3).
- Produces (used by Task 5):
  - `enum DocRef { Draft, Source(Uuid) }`, `enum CentreTab { Document, Chat }`
  - `Desktop` fields `resolved: ResolvedLayout`, `open_doc: Option<DocRef>`, `centre_tab: CentreTab`, `nav: DocNavigation`, `settings_requested: bool`, `history_scroll`/`vault_scroll`/`chat_scroll`/`source_scroll: ScrollHandle`
  - methods `Desktop::{show_document, close_document, open_draft_from_list, cancel_running, phase_status, persist_layout, toggle_rail, toggle_focus, palette, render_shell, render_separator, open_settings, render_chat, render_document}`
  - `shell::{approval_tag, section_label}`

- [ ] **Step 1: Write the failing pure tests**

Append to the existing `mod tests` in `native/mod.rs`:

```rust
    #[test]
    fn approval_tags_pair_text_with_state() {
        assert_eq!(shell::approval_tag(Approval::Approved), "✓ approved");
        assert_eq!(shell::approval_tag(Approval::Draft), "○ not approved");
        assert_eq!(shell::approval_tag(Approval::Withdrawn), "– withdrawn");
    }

    #[test]
    fn draft_completion_navigates_only_without_newer_navigation() {
        let mut nav = DocNavigation::default();
        assert!(!nav.take_draft_completion());
        nav.request_draft();
        assert!(nav.take_draft_completion());
        assert!(!nav.take_draft_completion());
        nav.request_draft();
        nav.moved();
        assert!(!nav.take_draft_completion());
    }
```

Run: `cargo +1.98.1 test -p brn-desktop --features native-ui --locked --offline -- approval_tags draft_completion`
Expected: compile errors, `shell` and `DocNavigation` not found.

- [ ] **Step 2: Add centre state, navigation and helper methods to `native/mod.rs`**

Keep `enum Page`, the `page` field and the old `render` body until Step 9, so every intermediate edit still compiles.

1. Add `mod shell;` next to `mod theme;`. Change the import to `use crate::layout::{self, LayoutState, Loaded, ResolvedLayout};`.
2. Add, next to `enum Page`:

   ```rust
   #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   enum DocRef {
       Draft,
       Source(Uuid),
   }

   #[derive(Clone, Copy, Debug, PartialEq, Eq)]
   enum CentreTab {
       Document,
       Chat,
   }

   /// Correlates a draft-open completion with the navigation that requested it,
   /// so a late completion cannot override a newer Close or source selection.
   #[derive(Debug, Default)]
   struct DocNavigation {
       generation: u64,
       pending_draft: Option<u64>,
   }

   impl DocNavigation {
       fn moved(&mut self) {
           self.generation = self.generation.wrapping_add(1);
       }
       fn request_draft(&mut self) {
           self.pending_draft = Some(self.generation);
       }
       fn take_draft_completion(&mut self) -> bool {
           self.pending_draft.take() == Some(self.generation)
       }
   }
   ```

3. In `struct Desktop`, add after `layout_note: Option<String>,`:

   ```rust
       resolved: ResolvedLayout,
       open_doc: Option<DocRef>,
       centre_tab: CentreTab,
       nav: DocNavigation,
       settings_requested: bool,
       history_scroll: ScrollHandle,
       vault_scroll: ScrollHandle,
       chat_scroll: ScrollHandle,
       source_scroll: ScrollHandle,
   ```

4. In `Desktop::new`, add `let resolved = layout.resolve(1100.0, false);` after the `layout::load` line. In the literal, add:

   ```rust
               resolved,
               open_doc: None,
               centre_tab: CentreTab::Chat,
               nav: DocNavigation::default(),
               settings_requested: false,
               history_scroll: ScrollHandle::new(),
               vault_scroll: ScrollHandle::new(),
               chat_scroll: ScrollHandle::new(),
               source_scroll: ScrollHandle::new(),
   ```

   Each region owns a persistent scroll handle, so collapsing, Focus and tab switches restore its position. Sources and drafts no longer share one.
5. Add these methods to the main `impl Desktop` block, after `choose_session`:

   ```rust
       fn show_document(&mut self, doc: DocRef) {
           self.nav.moved();
           self.open_doc = Some(doc);
           self.centre_tab = CentreTab::Document;
       }
       /// Hides the document pane. A draft's in-memory editor state is kept.
       fn close_document(&mut self) {
           self.nav.moved();
           self.open_doc = None;
           self.centre_tab = CentreTab::Chat;
       }
       fn open_draft_from_list(&mut self, id: Uuid, cx: &mut Context<Self>) {
           if self.draft_state.as_ref().is_some_and(|state| state.id() == id) {
               self.show_document(DocRef::Draft);
               cx.notify();
               return;
           }
           self.choose_draft(id, cx);
       }
       fn cancel_running(&mut self, cx: &mut Context<Self>) {
           if matches!(self.phase, Phase::Running { .. }) && self.worker.cancel() {
               self.phase.cancel();
               self.message = "Cancellation requested; awaiting safe stop.".into();
               cx.notify();
           }
       }
   ```

6. Move the `let status = match &self.phase { ... };` expression out of `render` into a new method with the same match arms, unchanged. Make the old render use `let status = self.phase_status();` until Step 9:

   ```rust
       fn phase_status(&self) -> String {
           match &self.phase {
               Phase::Opening => "Opening workspace…".into(),
               Phase::Idle => "Ready".into(),
               Phase::Running { label, since } => {
                   format!("{label} · {}s elapsed", since.elapsed().as_secs())
               }
               Phase::Cancelling { label, since } => format!(
                   "Cancelling {label} · {}s elapsed · awaiting safe stop",
                   since.elapsed().as_secs()
               ),
               Phase::Failed(error) => {
                   format!("Workspace failed to open · {}", compact_title(error))
               }
           }
       }
   ```

7. Request navigation with each draft open:
   - in `choose_draft`, immediately before `self.submit(Action::OpenDraftComments { id }, "Open draft", cx);`, add `self.nav.request_draft();`;
   - in `create_draft`, immediately before its `self.submit(Action::CreateDraft { ... }, ...)` call, add `self.nav.request_draft();`.
8. In `poll`, inside both `if may_open {` blocks, add as the first statement:

   ```rust
                           if self.nav.take_draft_completion() {
                               self.show_document(DocRef::Draft);
                           }
   ```

   One block is in the `Ok(Outcome::DraftCreated { draft }) | Ok(Outcome::DraftOpened { draft, .. })` arm; the other is in the `Ok(Outcome::DraftCommentsOpened { .. })` arm. `may_open` still protects edits; `nav` protects newer navigation.
9. In `choose_session`, after `self.selected_session = session;`, add `self.centre_tab = CentreTab::Chat;`.

- [ ] **Step 3: Create `shell/mod.rs`**

```rust
//! Workspace shell: region composition and layout preference changes.

mod centre;
mod header;
mod history_rail;
mod settings;
mod vault_rail;

use super::theme::{self, color};
use super::*;
use crate::layout::{Appearance, CentreMode, Rail, RailDisplay};
use crate::tokens::{self, Palette};
use gpui_kit::{AnyElement, component::Selectable, relative};

pub(super) fn approval_tag(approval: Approval) -> &'static str {
    match approval {
        Approval::Approved => "✓ approved",
        Approval::Draft => "○ not approved",
        Approval::Withdrawn => "– withdrawn",
    }
}

fn section_label(text: &str, p: Palette) -> impl IntoElement {
    div()
        .pt_2()
        .text_size(px(10.))
        .text_color(color(p.muted))
        .child(text.to_uppercase())
}

impl Desktop {
    pub(super) fn palette(&self) -> Palette {
        tokens::palette(self.layout.appearance.scheme(self.system_dark))
    }

    pub(super) fn persist_layout(&mut self, cx: &mut Context<Self>) {
        self.layout_note = None;
        if let Err(error) = layout::save(&self.path, &self.layout) {
            self.message = format!("Layout preferences were not saved: {error}");
        }
        cx.notify();
    }

    pub(super) fn toggle_rail(&mut self, rail: Rail, cx: &mut Context<Self>) {
        let shown = match rail {
            Rail::History => self.resolved.history,
            Rail::Vault => self.resolved.vault,
        };
        if self.layout.toggle_rail(rail, shown) {
            self.persist_layout(cx);
        } else {
            let name = match rail {
                Rail::History => "history",
                Rail::Vault => "the vault",
            };
            self.message = format!("Widen the window to show {name}.");
            cx.notify();
        }
    }

    pub(super) fn toggle_focus(&mut self, cx: &mut Context<Self>) {
        self.layout.toggle_focus();
        self.persist_layout(cx);
    }

    fn set_appearance(&mut self, appearance: Appearance, window: &mut Window, cx: &mut Context<Self>) {
        self.layout.appearance = appearance;
        if let Err(error) = theme::apply(appearance, window, cx) {
            self.message = error;
        }
        self.persist_layout(cx);
    }

    /// Static 5 pt separator budgeted by `LayoutState::resolve`; Task 5 makes it interactive.
    fn render_separator(&self) -> impl IntoElement {
        div()
            .w(px(layout::DIVIDER))
            .h_full()
            .flex_shrink_0()
            .bg(color(self.palette().line))
    }

    pub(super) fn render_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = f32::from(window.viewport_size().width);
        self.resolved = self.layout.resolve(width, self.open_doc.is_some());
        let resolved = self.resolved;
        let p = self.palette();
        if std::mem::take(&mut self.settings_requested) {
            let desktop = cx.entity().downgrade();
            window.defer(cx, move |window, cx| {
                if let Some(desktop) = desktop.upgrade() {
                    desktop.update(cx, |this, cx| this.open_settings(window, cx));
                }
            });
        }
        let mut row = div()
            .flex()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .child(self.render_rail_slot(Rail::History, resolved.history, cx));
        if resolved.history == RailDisplay::Open {
            row = row.child(self.render_separator());
        }
        row = row.child(self.render_centre(&resolved, window, cx));
        if resolved.vault == RailDisplay::Open {
            row = row.child(self.render_separator());
        }
        row = row.child(self.render_rail_slot(Rail::Vault, resolved.vault, cx));
        let root = div()
            .id("brn-desktop")
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .bg(color(p.background))
            .text_color(color(p.text))
            .font_family(tokens::CHROME_FONT)
            .text_size(px(12.))
            .child(self.render_header(&resolved, cx))
            .child(self.render_status_line())
            .child(row);
        root
    }

    fn render_rail_slot(&mut self, rail: Rail, display: RailDisplay, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette();
        match display {
            RailDisplay::Hidden => div().into_any_element(),
            RailDisplay::Open => match rail {
                Rail::History => self.render_history_rail(cx).into_any_element(),
                Rail::Vault => self.render_vault_rail(cx).into_any_element(),
            },
            RailDisplay::Collapsed | RailDisplay::AutoCollapsed => {
                let auto = display == RailDisplay::AutoCollapsed;
                let (id, label, tip) = match (rail, auto) {
                    (Rail::History, false) => ("expand-history", "›", "Show history (⌘0)"),
                    (Rail::History, true) => ("expand-history", "›", "History is hidden to fit the window. Widen the window to show it."),
                    (Rail::Vault, false) => ("expand-vault", "‹", "Show vault (⌥⌘0)"),
                    (Rail::Vault, true) => ("expand-vault", "‹", "Vault is hidden to fit the window. Widen the window to show it."),
                };
                let slot = div()
                    .w(px(layout::COLLAPSED_RAIL))
                    .flex_shrink_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .pt_2()
                    .bg(color(p.panel))
                    .border_color(color(if auto { p.amber } else { p.line }));
                let slot = match rail {
                    Rail::History => slot.border_r_1(),
                    Rail::Vault => slot.border_l_1(),
                };
                slot.child(
                    Button::new(id)
                        .label(label)
                        .compact()
                        .tooltip(tip)
                        .on_click(cx.listener(move |this, _, _, cx| this.toggle_rail(rail, cx))),
                )
                .into_any_element()
            }
        }
    }
}
```

- [ ] **Step 4: Create `shell/header.rs`**

```rust
use super::*;

impl Desktop {
    pub(super) fn render_header(&mut self, resolved: &ResolvedLayout, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let tone = match self.phase {
            Phase::Idle => p.green,
            Phase::Failed(_) => p.amber,
            Phase::Opening => p.muted,
            Phase::Running { .. } | Phase::Cancelling { .. } => p.cyan,
        };
        let mut bar = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_1()
            .min_w(px(0.))
            .pr_2()
            .child(div().flex_shrink_0().child("brn"))
            .child(div().flex_shrink_0().text_color(color(p.muted)).child("/ workspace"))
            .child(
                div()
                    .min_w(px(0.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_color(color(tone))
                    .child(self.phase_status()),
            )
            .child(div().flex_1());
        if matches!(self.phase, Phase::Running { .. }) {
            bar = bar.child(
                Button::new("cancel")
                    .label("Cancel")
                    .compact()
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_running(cx))),
            );
        }
        bar = bar
            .child(
                Button::new("toggle-focus")
                    .label("Focus")
                    .compact()
                    .selected(self.layout.focus)
                    .toggled(self.layout.focus)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_focus(cx))),
            )
            .child(
                Button::new("toggle-vault")
                    .label("Vault")
                    .compact()
                    .selected(resolved.vault == RailDisplay::Open)
                    .toggled(resolved.vault == RailDisplay::Open)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_rail(Rail::Vault, cx))),
            );
        if resolved.history != RailDisplay::Open {
            bar = bar.child(
                Button::new("header-settings")
                    .label("⚙")
                    .compact()
                    .tooltip("Settings (⌘,)")
                    .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx))),
            );
        }
        TitleBar::new().child(bar)
    }

    pub(super) fn render_status_line(&self) -> impl IntoElement {
        let p = self.palette();
        let mut line = div()
            .id("status-line")
            .flex()
            .flex_col()
            .flex_shrink_0()
            .max_h(px(96.))
            .overflow_y_scroll()
            .px_3()
            .py_1()
            .gap_1()
            .border_b_1()
            .border_color(color(p.line))
            .bg(color(p.paper))
            .text_color(color(p.muted))
            .child(self.message.clone());
        if matches!(self.phase, Phase::Failed(_)) {
            line = line.child("Workspace unavailable. Review the error above, correct the workspace, then relaunch.");
        }
        if !self.progress.is_empty() && matches!(self.phase, Phase::Running { .. } | Phase::Cancelling { .. }) {
            line = line.child(format!("Progress: {}", self.progress));
        }
        if let Some(note) = &self.layout_note {
            line = line.child(note.clone());
        }
        line
    }
}
```

Also extend the `shell/mod.rs` imports to `use gpui_kit::{AnyElement, component::Selectable, relative};` so the region files below can use them through `use super::*;`. Task 5 adds `MouseButton`.

- [ ] **Step 5: Create `shell/history_rail.rs`**

The session button label and handler come from the former `Page::Activity` arm, unchanged apart from `.selected(...)`.

```rust
use super::*;

impl Desktop {
    pub(super) fn render_history_rail(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let open_draft = self
            .draft_state
            .as_ref()
            .filter(|_| self.open_doc == Some(DocRef::Draft))
            .map(|state| state.id());
        let mut list = div()
            .id("history-rail-list")
            .track_scroll(&self.history_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_1()
            .p_2()
            .child(
                Button::new("new-session")
                    .label("+ New chat")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| this.choose_session(None, cx))),
            )
            .child(section_label("Conversations", p));
        for session in &self.sessions {
            let id = session.id;
            list = list.child(
                Button::new(format!("session-{id}"))
                    .label(format!(
                        "{}… · {} turns{}",
                        &id.to_string()[..8],
                        session.turns,
                        if session.has_thread { " · provider linked" } else { " · recovery needed" }
                    ))
                    .selected(self.selected_session == Some(id))
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(move |this, _, _, cx| this.choose_session(Some(id), cx))),
            );
        }
        list = list.child(section_label("Drafts", p));
        for draft in &self.drafts {
            let id = draft.id;
            list = list.child(
                Button::new(format!("history-draft-{id}"))
                    .label(compact_title(&draft.title))
                    .selected(open_draft == Some(id))
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(move |this, _, _, cx| this.open_draft_from_list(id, cx))),
            );
        }
        div()
            .w(px(self.layout.history_w))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(color(p.panel))
            .child(list)
            .child(
                div().flex_shrink_0().border_t_1().border_color(color(p.line)).p_2().child(
                    Button::new("settings-footer")
                        .label("⚙ Settings")
                        .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx))),
                ),
            )
    }
}
```

- [ ] **Step 6: Create `shell/vault_rail.rs`**

The import, index and draft-creation buttons come from the former `Page::Workspace` and `Page::Drafts` arms, with unchanged ids, labels, guards and handlers.

```rust
use super::*;

impl Desktop {
    pub(super) fn render_vault_rail(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let open_draft = self
            .draft_state
            .as_ref()
            .filter(|_| self.open_doc == Some(DocRef::Draft))
            .map(|state| state.id());
        let mut list = div()
            .id("vault-rail-list")
            .track_scroll(&self.vault_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .overflow_y_scroll()
            .gap_1()
            .p_2()
            .child(section_label("Sources", p))
            .child(div().text_color(color(p.muted)).child(format!("{} sources", self.sources.len())));
        for source in &self.sources {
            let id = source.source_id;
            list = list.child(
                Button::new(format!("source-{id}"))
                    .label(format!("{} · {}", compact_title(&source.title), approval_tag(source.approval)))
                    .selected(self.open_doc == Some(DocRef::Source(id)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.open_doc != Some(DocRef::Source(id)) {
                            this.source_scroll.set_offset(point(px(0.), px(0.)));
                        }
                        this.show_document(DocRef::Source(id));
                        cx.notify();
                    })),
            );
        }
        list = list
            .child(section_label("Working drafts", p))
            .child(Input::new(&self.draft_title).aria_label("New draft title"))
            .child(
                Button::new("create-draft")
                    .label("Create blank draft")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| this.create_draft(cx))),
            )
            .child(
                Button::new("refresh-drafts")
                    .label("Refresh list")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submit(Action::ListDrafts, "Draft list", cx);
                    })),
            )
            .child(div().text_color(color(p.muted)).child(format!("{} drafts", self.drafts.len())));
        for draft in &self.drafts {
            let id = draft.id;
            list = list.child(
                Button::new(format!("vault-draft-{id}"))
                    .label(format!("{} · {}…", compact_title(&draft.title), &id.to_string()[..8]))
                    .selected(open_draft == Some(id))
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(move |this, _, _, cx| this.open_draft_from_list(id, cx))),
            );
        }
        list = list
            .child(section_label("Import", p))
            .child(
                div()
                    .text_color(color(p.muted))
                    .child("Import a UTF-8 Markdown or text file and explicitly approve it for search."),
            )
            .child(
                Button::new("choose-file")
                    .label(if self.choosing_file { "Choosing…" } else { "Choose file…" })
                    .disabled(self.choosing_file || !self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| this.choose_file(cx))),
            )
            .child(
                Button::new("import-approved")
                    .label("Import and approve")
                    .disabled(!self.phase.can_submit() || self.import_path.is_none())
                    .on_click(cx.listener(|this, _, _, cx| this.import(cx))),
            )
            .child(
                Button::new("build-index")
                    .label("Build index")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submit(Action::Build, "Index build", cx);
                    })),
            )
            .child(
                Button::new("refresh")
                    .label("Refresh")
                    .disabled(!self.phase.can_submit())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submit(Action::Refresh { session: this.selected_session }, "Refresh", cx);
                    })),
            )
            .child(div().min_w(px(0.)).text_color(color(p.muted)).child(format!(
                "Selected file: {}",
                self.import_path
                    .as_ref()
                    .map_or("none".into(), |path| spaced_identifier(&path.display().to_string()))
            )));
        div()
            .w(px(self.layout.vault_w))
            .flex_shrink_0()
            .h_full()
            .flex()
            .flex_col()
            .bg(color(p.panel))
            .child(list)
    }
}
```

- [ ] **Step 7: Create `shell/centre.rs` and move chat and draft content**

Create the file with this skeleton. Then paste the three marked blocks verbatim from the old `render` body in `native/mod.rs`:

```rust
use super::*;

impl Desktop {
    pub(super) fn render_centre(&mut self, resolved: &ResolvedLayout, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let centre = div().flex().flex_col().flex_1().min_w(px(0.)).min_h(px(0.)).h_full();
        match resolved.mode {
            CentreMode::ChatOnly => centre.child(self.render_chat(cx)),
            CentreMode::Split => centre.child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .child(div().w(px(resolved.doc_w)).flex_shrink_0().h_full().child(self.render_document(cx)))
                    .child(self.render_document_divider(window, cx))
                    .child(div().flex_1().min_w(px(0.)).h_full().child(self.render_chat(cx))),
            ),
            CentreMode::Tabs => {
                let tab = self.centre_tab;
                let bar = div()
                    .flex()
                    .flex_shrink_0()
                    .gap_1()
                    .p_1()
                    .border_b_1()
                    .border_color(color(p.line))
                    .bg(color(p.panel))
                    .child(
                        Button::new("tab-document")
                            .label("Document")
                            .compact()
                            .selected(tab == CentreTab::Document)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.centre_tab = CentreTab::Document;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("tab-chat")
                            .label("Chat")
                            .compact()
                            .selected(tab == CentreTab::Chat)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.centre_tab = CentreTab::Chat;
                                cx.notify();
                            })),
                    );
                let pane = match tab {
                    CentreTab::Document => self.render_document(cx).into_any_element(),
                    CentreTab::Chat => self.render_chat(cx).into_any_element(),
                };
                centre.child(bar).child(div().flex_1().min_h(px(0.)).child(pane))
            }
        }
    }

    /// Task 5 replaces this static separator with the interactive divider.
    fn render_document_divider(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.render_separator()
    }

    fn render_document(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let (label, body) = match self.open_doc {
            Some(DocRef::Draft) => (
                format!(
                    "Drafts / {}",
                    self.draft_state.as_ref().map_or("opening…".to_string(), |state| state.title().to_string())
                ),
                self.render_draft_document(cx),
            ),
            Some(DocRef::Source(id)) => (
                format!(
                    "Sources / {}",
                    self.sources
                        .iter()
                        .find(|source| source.source_id == id)
                        .map_or("unavailable".to_string(), |source| compact_title(&source.title))
                ),
                self.render_source_reader(id, cx),
            ),
            None => (String::new(), div().into_any_element()),
        };
        let scroll = match self.open_doc {
            Some(DocRef::Source(_)) => self.source_scroll.clone(),
            _ => self.draft_scroll.clone(),
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(color(p.paper))
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .border_b_1()
                    .border_color(color(p.line))
                    .text_color(color(p.muted))
                    .child(div().flex_1().min_w(px(0.)).overflow_hidden().whitespace_nowrap().child(label))
                    .child(
                        Button::new("close-document")
                            .label("Close")
                            .compact()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_document();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("document-body")
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .gap_3()
                    .p_3()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .vertical_scrollbar(&scroll)
                    .child(body),
            )
    }

    fn render_draft_document(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let mut panel = div().flex().flex_col().flex_shrink_0().min_w(px(0.)).gap_3();
        if self.draft_state.is_none() {
            return panel.child("Opening draft…").into_any_element();
        }
        // MOVED BLOCK C (see below).
        panel.into_any_element()
    }

    fn render_source_reader(&mut self, id: Uuid, cx: &mut Context<Self>) -> AnyElement {
        let p = self.palette();
        let Some(source) = self.sources.iter().find(|source| source.source_id == id) else {
            return div()
                .child("This source is no longer listed. Refresh the vault or close this document.")
                .into_any_element();
        };
        let source_id = source.source_id;
        let version = source.version_id;
        div()
            .flex()
            .flex_col()
            .gap_3()
            .min_w(px(0.))
            .child(div().font_family(tokens::READING_FONT).text_size(px(24.)).child(source.title.clone()))
            .child(div().text_color(color(p.muted)).child(format!(
                "Source · read only · {} · {} bytes",
                approval_tag(source.approval),
                source.bytes.len()
            )))
            .child(
                div()
                    .text_color(color(p.muted))
                    .child(format!("Revision: {}", spaced_identifier(&version.to_string()))),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new(format!("approve-{source_id}"))
                            .label("Approve")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.submit(
                                    Action::SetApproval { source: source_id, version, approval: Approval::Approved },
                                    "Approve",
                                    cx,
                                );
                            })),
                    )
                    .child(
                        Button::new(format!("withdraw-{source_id}"))
                            .label("Withdraw")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.submit(
                                    Action::SetApproval { source: source_id, version, approval: Approval::Withdrawn },
                                    "Withdraw",
                                    cx,
                                );
                            })),
                    ),
            )
            .child(
                div()
                    .font_family(tokens::READING_FONT)
                    .text_size(px(15.))
                    .line_height(relative(1.6))
                    .child(String::from_utf8_lossy(&source.bytes).into_owned()),
            )
            .into_any_element()
    }

    pub(super) fn render_chat(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let mut body = div()
            .id("chat-transcript")
            .track_scroll(&self.chat_scroll)
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.))
            .gap_3()
            .p_3()
            .overflow_y_scroll();
        if self.history.is_empty() && self.search.is_none() && self.streamed_text.is_empty() {
            body = body.child(
                div()
                    .text_color(color(p.muted))
                    .child("Ask about approved sources. Answers, citations and saved conversations appear here."),
            );
        }
        // MOVED BLOCK A (see below).
        // MOVED BLOCK B (see below).
        let composer = div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_2()
            .p_2()
            .border_t_1()
            .border_color(color(p.line))
            .bg(color(p.panel))
            .child(
                Editor::new(&self.query)
                    .h(px(90.))
                    .flex_shrink_0()
                    .aria_label("Question for approved sources"),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(
                        Button::new("profile-keyword")
                            .label("Keyword")
                            .compact()
                            .selected(matches!(self.profile, Profile::Keyword))
                            .on_click(cx.listener(|this, _, _, cx| this.choose_profile(Profile::Keyword, cx))),
                    )
                    .child(
                        Button::new("profile-semantic")
                            .label("Semantic")
                            .compact()
                            .selected(matches!(self.profile, Profile::Semantic))
                            .on_click(cx.listener(|this, _, _, cx| this.choose_profile(Profile::Semantic, cx))),
                    )
                    .child(
                        Button::new("profile-hybrid")
                            .label("Hybrid")
                            .compact()
                            .selected(matches!(self.profile, Profile::Hybrid))
                            .on_click(cx.listener(|this, _, _, cx| this.choose_profile(Profile::Hybrid, cx))),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("search")
                            .label("Search")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(|this, _, _, cx| this.search(cx))),
                    )
                    .child(
                        Button::new("ask")
                            .label("Ask from sources")
                            .disabled(!self.phase.can_submit())
                            .on_click(cx.listener(|this, _, _, cx| this.ask(cx))),
                    ),
            );
        div().flex().flex_col().size_full().child(body).child(composer)
    }
}
```

Moved blocks, pasted verbatim at the marked comments:

- **Block A**, into `render_chat`: from the former `Page::Activity` arm, the statement beginning `for (i, turn) in self.history.iter().enumerate() {` through the end of the `if let Some(hit) = &self.selected_saved_evidence { ... }` statement. This includes the turn buttons, the selected-turn details, the saved-evidence buttons and the **Save as candidate** block.
- **Block B**, into `render_chat` after Block A: from the former `Page::Workspace` arm, the three statements `if let Some(search) = &self.search { ... }`, `if let Some(hit) = &self.selected_evidence { ... }` and `if self.phase.shows_live_answer() && !self.streamed_text.is_empty() { ... }`.
- **Block C**, into `render_draft_document`: from the former `Page::Drafts` arm, the statement beginning `if let Some(state) = &self.draft_state {` (the one after the `for draft in &self.drafts` loop) through its matching `}` just before `body = body.child(panel);`.

Intentionally dropped:
- the `Page::Workspace` "Saved answer…" block with its `view-saved-answer` button, because Block A now shows saved answers inline;
- the "Selected profile" and "Selected session" text lines, replaced by `.selected(...)` styling, which is visible; `.toggled` is accessibility-only in the pinned toolkit;
- the "Grounded workspace" heading, now the vault Import hint;
- the page-switch buttons.

Record these removals in evidence.

- [ ] **Step 8: Create `shell/settings.rs` (toolkit modal dialog)**

Settings uses gpui-component's modal host via `WindowExt::open_dialog`. That host traps focus, closes on Escape and on overlay click, and returns focus afterwards; `Root::new` already wraps `Desktop` in `run`. The Connection lines come from the former `Page::Settings` arm, unchanged.

```rust
use super::*;
use gpui_kit::{App, Entity, component::WindowExt};

impl Desktop {
    pub(super) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if window.has_active_dialog(cx) {
            return;
        }
        let desktop = cx.entity();
        window.open_dialog(cx, move |dialog, _, cx| {
            dialog.title("Settings").w(px(460.)).child(settings_body(&desktop, cx))
        });
    }
}

/// Rebuilt on every dialog render from the current `Desktop` state.
fn settings_body(desktop: &Entity<Desktop>, cx: &App) -> impl IntoElement {
    let this = desktop.read(cx);
    let p = this.palette();
    let prefs = this.layout.clone();
    let data_dir = spaced_identifier(&this.path.display().to_string());
    let codex = this
        .config
        .codex
        .as_ref()
        .map_or("not selected".into(), |path| spaced_identifier(&path.display().to_string()));
    let model_dir = this
        .config
        .model_dir
        .as_ref()
        .map_or("not selected".into(), |path| spaced_identifier(&path.display().to_string()));
    let appearance_button = |id: &'static str, label: &'static str, value: Appearance| {
        let target = desktop.downgrade();
        Button::new(id)
            .label(label)
            .compact()
            .selected(prefs.appearance == value)
            .on_click(move |_, window, cx| {
                let _ = target.update(cx, |this, cx| this.set_appearance(value, window, cx));
            })
    };
    let resize_button = |id: &'static str, label: &'static str, rail: Rail, delta: f32| {
        let target = desktop.downgrade();
        Button::new(id).label(label).compact().on_click(move |_, _, cx| {
            let _ = target.update(cx, |this, cx| {
                this.layout.resize_rail(rail, delta);
                this.persist_layout(cx);
            });
        })
    };
    let reset_target = desktop.downgrade();
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(section_label("Workspace", p))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(140.)).child("Appearance"))
                .child(appearance_button("appearance-system", "System", Appearance::System))
                .child(appearance_button("appearance-dark", "Dark", Appearance::Dark))
                .child(appearance_button("appearance-light", "Light", Appearance::Light)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(140.)).child(format!("History: {} pt", prefs.history_w.round() as i32)))
                .child(resize_button("history-narrower", "−", Rail::History, -layout::KEY_STEP))
                .child(resize_button("history-wider", "+", Rail::History, layout::KEY_STEP)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(140.)).child(format!("Vault: {} pt", prefs.vault_w.round() as i32)))
                .child(resize_button("vault-narrower", "−", Rail::Vault, -layout::KEY_STEP))
                .child(resize_button("vault-wider", "+", Rail::Vault, layout::KEY_STEP)),
        )
        .child(div().text_color(color(p.muted)).child(format!(
            "Document share: {}% of the centre",
            (prefs.doc_share * 100.0).round() as i32
        )))
        .child(Button::new("reset-layout").label("Reset layout").on_click(move |_, _, cx| {
            let _ = reset_target.update(cx, |this, cx| {
                this.layout.reset_layout();
                this.persist_layout(cx);
            });
        }))
        .child(section_label("Connection", p))
        .child(format!("Data directory: {data_dir}"))
        .child(format!("Codex executable: {codex}"))
        .child(format!("Retrieval model directory: {model_dir}"))
        .child("Model access uses the existing managed ChatGPT sign-in in Codex. Select an absolute executable with --codex before asking.")
}
```

The header gear and History footer call `this.open_settings(window, cx)` directly. App-level ⌘, (Task 5) sets `settings_requested`; `render_shell` then opens the dialog through `window.defer`, outside the current render.

- [ ] **Step 9: Replace the old render body**

After Blocks A–C are pasted and compile, replace the whole `impl Render for Desktop` with:

```rust
impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_shell(window, cx)
    }
}
```

This deletes the Task 3 temporary root, the page buttons, the old Cancel button and the `match self.page` body. Now also delete `enum Page`, the `page: Page` field and its `page: Page::Workspace` initialiser; no other code references them. Remove any import that becomes unused, such as `TitleBar` if it is only used in `header.rs`. Move it to `use` there instead, or keep it in `mod.rs` if `run` uses it.

- [ ] **Step 10: Verify automated checks**

Run:

```bash
cargo +1.98.1 fmt --all -- --check
cargo +1.98.1 clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings
cargo +1.98.1 test -p brn-desktop --features native-ui --locked --offline
cargo +1.98.1 test -p brn-desktop --locked --offline
```

Expected: all pass, including `approval_tags_pair_text_with_state`, `draft_completion_navigates_only_without_newer_navigation`, `handoff_theme_configs_deserialize_for_both_schemes`, the existing `drafts`/`comments` tests and the layout/tokens tests.

- [ ] **Step 11: Manual native check (disposable data)**

Launch as in Task 3 Step 7. Create a small fixture: `printf '# Sample\n\nIndexes stay derived.\n' > "$data/sample.md"`. Then, in the app:
1. Choose file → `sample.md`, then Import and approve. The source appears in Vault with `✓ approved`.
2. Build index, type a question, then Search. Passages appear in the chat transcript. Do not use Ask unless a Codex executable was supplied and the task authorises live provider use; record it as skipped otherwise.
3. Create a blank draft from Vault. It opens in the document pane, split beside chat. Type text, then Save working copy and Save checkpoint. Click another draft while dirty and confirm the guard message. Close the document, then reopen it from History; edits are intact.
4. Select text, capture and add a comment. Compare two revisions.
5. Open the source. The reader shows title, approval, revision and text. Withdraw, then Approve.
6. Toggle Vault and Focus from the header; History collapses via the `›` rail. Shrink the window to 1000, 700 and 500 pt. The vault auto-collapses first (amber rail), then history, then Document | Chat tabs appear. Widen it again and the rails return.
7. Open Settings from the footer and from the header gear (history collapsed). Tab stays inside the dialog, Escape closes it, and focus returns. Switch appearance among System, Dark and Light (the label, button and editor colours all change), adjust widths (the labels update), and Reset layout. Typed composer text survives untouched.
8. Scroll History, Vault and the chat transcript. Collapse and restore each rail, enter and exit Focus, and switch Chat → Document → Chat in tabs mode; each scroll position is restored. Scroll a long draft, open a short source, reopen the draft: the draft position is kept.
9. Click a draft, then immediately click a source before the draft finishes opening. The source stays displayed and the draft does not take over.
10. Quit and relaunch with the same `--data-dir`. The layout is restored.

Record the observations, and anything that did not behave as described, in `evidence.md`.

- [ ] **Step 12: Commit**

```bash
git add crates/brn-desktop/src/native
git commit -m "desktop: arrange workspace into history, centre and vault regions

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 5: Interactive dividers, shortcuts and menus

**Files:**
- Create: `crates/brn-desktop/src/native/shell/divider.rs`
- Modify: `crates/brn-desktop/src/native/shell/mod.rs` (`mod divider;`, row composition, root mouse handlers, composer focus)
- Modify: `crates/brn-desktop/src/native/shell/centre.rs` (use the interactive document divider)
- Modify: `crates/brn-desktop/src/native/mod.rs` (actions, fields, `run` bindings, menus, routing)

**Interfaces:**
- Consumes:
  - `LayoutState::{drag, nudge, resolve}`, `layout::{Divider, KEY_STEP, KEY_STEP_LARGE, DIVIDER}` (Task 1);
  - `Desktop::{toggle_rail, toggle_focus, cancel_running, choose_session, persist_layout, render_shell, render_separator}` and the fields `resolved`, `centre_tab`, `settings_requested` (Task 4).
- Produces:
  - `Desktop` fields `dragging: Option<Divider>`, `divider_focus: [FocusHandle; 3]`, `focus_composer: bool`
  - `Desktop::{render_divider, drag_divider}` (`pub(super)`), `Desktop::end_divider_drag` (`pub(in crate::native)`, called from the window-activation observer)
  - actions `ToggleHistory`, `ToggleVault`, `ToggleFocus`, `FocusComposer`, `NewChat`, `OpenSettings`, `CancelRunning`. Escape is handled by the toolkit dialog; there is no global Escape action.

- [ ] **Step 1: Add fields and actions**

In `native/mod.rs`:

1. Replace `gpui_kit::actions!(brn, [Quit]);` with:

   ```rust
   gpui_kit::actions!(
       brn,
       [Quit, ToggleHistory, ToggleVault, ToggleFocus, FocusComposer, NewChat, OpenSettings, CancelRunning]
   );
   ```

2. Add `FocusHandle`, `Focusable`, `WeakEntity` and `App` to the `gpui_kit::{...}` import, along with `crate::layout::{Divider, Rail}`.
3. Add these fields to `struct Desktop` after `source_scroll: ScrollHandle,`:

   ```rust
       dragging: Option<Divider>,
       divider_focus: [FocusHandle; 3],
       focus_composer: bool,
   ```

4. Initialise them in `Desktop::new`:

   ```rust
               dragging: None,
               divider_focus: [
                   cx.focus_handle().tab_stop(true),
                   cx.focus_handle().tab_stop(true),
                   cx.focus_handle().tab_stop(true),
               ],
               focus_composer: false,
   ```

   Also end any divider drag when the window deactivates, so a release outside the app still persists the width. Add this before `Self {` and push it into `_subscriptions`:

   ```rust
           let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
               if !window.is_window_active() {
                   this.end_divider_drag(cx);
               }
           });
   ```

5. Confirm `palette`, `persist_layout`, `toggle_rail`, `toggle_focus` and `render_shell` are `pub(super)` in `shell/mod.rs` (Task 4), so `run` and `render` in `native/mod.rs` can call them.

- [ ] **Step 2: Create `shell/divider.rs`**

```rust
use super::*;
use crate::layout::Divider;
use gpui_kit::{KeyDownEvent, MouseMoveEvent};

fn index(divider: Divider) -> usize {
    match divider {
        Divider::History => 0,
        Divider::Document => 1,
        Divider::Vault => 2,
    }
}

impl Desktop {
    pub(super) fn render_divider(&mut self, divider: Divider, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = self.palette();
        let handle = self.divider_focus[index(divider)].clone();
        let active = handle.is_focused(window) || self.dragging == Some(divider);
        let id = match divider {
            Divider::History => "divider-history",
            Divider::Document => "divider-document",
            Divider::Vault => "divider-vault",
        };
        div()
            .id(id)
            .w(px(layout::DIVIDER))
            .h_full()
            .flex_shrink_0()
            .cursor_col_resize()
            .bg(color(if active { p.cyan } else { p.line }))
            .track_focus(&handle)
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, window, cx| {
                this.dragging = Some(divider);
                this.divider_focus[index(divider)].focus(window, cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                let step = if event.keystroke.modifiers.shift { layout::KEY_STEP_LARGE } else { layout::KEY_STEP };
                let delta = match event.keystroke.key.as_str() {
                    "left" => -step,
                    "right" => step,
                    _ => return,
                };
                let resolved = this.resolved;
                this.layout.nudge(divider, delta, &resolved);
                this.persist_layout(cx);
                cx.stop_propagation();
            }))
    }

    pub(super) fn drag_divider(&mut self, event: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(divider) = self.dragging else {
            return;
        };
        if event.pressed_button != Some(MouseButton::Left) {
            self.end_divider_drag(cx);
            return;
        }
        let width = f32::from(window.viewport_size().width);
        let resolved = self.layout.resolve(width, self.open_doc.is_some());
        self.layout.drag(divider, f32::from(event.position.x), width, &resolved);
        cx.notify();
    }

    pub(in crate::native) fn end_divider_drag(&mut self, cx: &mut Context<Self>) {
        if self.dragging.take().is_some() {
            self.persist_layout(cx);
        }
    }
}
```

Add `mod divider;` to `shell/mod.rs`.

- [ ] **Step 3: Wire dividers and composer focus into the shell**

In `shell/mod.rs` `render_shell`:

1. As the first statements, add:

   ```rust
           if std::mem::take(&mut self.focus_composer) {
               let handle = self.query.read(cx).focus_handle(cx);
               handle.focus(window, cx);
           }
   ```

2. Replace the two `row.child(self.render_separator())` calls with `row.child(self.render_divider(crate::layout::Divider::History, window, cx))` and `row.child(self.render_divider(crate::layout::Divider::Vault, window, cx))`, respectively.

3. Add the drag handlers on `root` before `.child(self.render_header(...))`. Releasing over the window ends the drag through `on_mouse_up`; releasing outside the root ends it through `on_mouse_up_out`; a later move without the button or window deactivation also ends it:

   ```rust
               .on_mouse_move(cx.listener(|this, event: &gpui_kit::MouseMoveEvent, window, cx| {
                   this.drag_divider(event, window, cx)
               }))
               .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, cx| this.end_divider_drag(cx)))
               .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, cx| this.end_divider_drag(cx)))
   ```

   Add `MouseButton` to the `gpui_kit::{...}` import in `shell/mod.rs`.

In `shell/centre.rs`:
- delete `render_document_divider`;
- in `render_centre`, replace `.child(self.render_document_divider(window, cx))` with `.child(self.render_divider(crate::layout::Divider::Document, window, cx))`.

Then delete `render_separator` from `shell/mod.rs`; nothing else uses it.

- [ ] **Step 4: Bind shortcuts, menus and action routing**

In `run`, replace the `cx.bind_keys([...])` and `cx.set_menus([...])` lines with:

```rust
            cx.bind_keys([
                KeyBinding::new("cmd-q", Quit, None),
                KeyBinding::new("cmd-0", ToggleHistory, None),
                KeyBinding::new("alt-cmd-0", ToggleVault, None),
                KeyBinding::new("shift-cmd-enter", ToggleFocus, None),
                KeyBinding::new("cmd-l", FocusComposer, None),
                KeyBinding::new("cmd-n", NewChat, None),
                KeyBinding::new("cmd-,", OpenSettings, None),
                KeyBinding::new("cmd-.", CancelRunning, None),
            ]);
            cx.set_menus([
                Menu::new("BRN").items(vec![
                    MenuItem::action("Settings…", OpenSettings),
                    MenuItem::separator(),
                    MenuItem::action("Quit BRN", Quit),
                ]),
                Menu::new("View").items(vec![
                    MenuItem::action("Toggle History", ToggleHistory),
                    MenuItem::action("Toggle Vault", ToggleVault),
                    MenuItem::action("Toggle Focus", ToggleFocus),
                ]),
                Menu::new("Navigate").items(vec![
                    MenuItem::action("New Chat", NewChat),
                    MenuItem::action("Focus Composer", FocusComposer),
                    MenuItem::action("Cancel Running Action", CancelRunning),
                ]),
            ]);
```

Add this free function to `native/mod.rs`:

```rust
/// Routes an app-level action to the desktop entity, like the existing Quit handler.
fn route<A: gpui_kit::Action>(cx: &mut App, target: WeakEntity<Desktop>, apply: fn(&mut Desktop, &mut Context<Desktop>)) {
    cx.on_action::<A>(move |_, cx| {
        let _ = target.update(cx, |this, cx| apply(this, cx));
    });
}
```

Inside the `open_window` closure, directly after the existing `cx.on_action::<Quit>(...)` registration, add:

```rust
                        route::<ToggleHistory>(cx, desktop.downgrade(), |this, cx| this.toggle_rail(Rail::History, cx));
                        route::<ToggleVault>(cx, desktop.downgrade(), |this, cx| this.toggle_rail(Rail::Vault, cx));
                        route::<ToggleFocus>(cx, desktop.downgrade(), |this, cx| this.toggle_focus(cx));
                        route::<FocusComposer>(cx, desktop.downgrade(), |this, cx| {
                            this.centre_tab = CentreTab::Chat;
                            this.focus_composer = true;
                            cx.notify();
                        });
                        route::<NewChat>(cx, desktop.downgrade(), |this, cx| this.choose_session(None, cx));
                        route::<OpenSettings>(cx, desktop.downgrade(), |this, cx| {
                            this.settings_requested = true;
                            cx.notify();
                        });
                        route::<CancelRunning>(cx, desktop.downgrade(), |this, cx| this.cancel_running(cx));
```

`choose_session` already returns early unless `phase.can_submit()`, so ⌘N cannot interrupt running work. `cancel_running` acts only while `Running`.

- [ ] **Step 5: Narrow the dead-code allowances**

Every layout and token item now has a native consumer. In `main.rs`, replace both temporary `#[allow(dead_code)]` attributes (and their comments) with:

```rust
#[cfg_attr(not(feature = "native-ui"), allow(dead_code))]
```

Run `cargo +1.98.1 clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings`. For any item still reported as never used, delete it if no task needs it. If it is reserved for a later slice, give it an item-level `#[allow(dead_code)]` with a one-line reason. Record which in evidence.

- [ ] **Step 6: Verify automated checks**

Run the four commands from Task 4 Step 10, then `bash scripts/verify-desktop-shell.sh --native`.
Expected: all pass.

- [ ] **Step 7: Manual native check (disposable data)**

Record each of the following in evidence:
1. Drag the history, document and vault dividers. Widths clamp at the bounds and the cursor shows column-resize.
2. Click a divider (it highlights in cyan), then use ←/→ (8 pt) and ⇧←/→ (32 pt). Tab can reach the dividers.
3. Use ⌘0, ⌥⌘0 and ⇧⌘↩. On an auto-collapsed rail, the "Widen the window…" message appears.
4. ⌘L focuses the composer and switches to the Chat tab in tabs mode. ⌘N starts a new chat when idle. ⌘, opens Settings and Esc closes it. Esc inside the composer and the draft editor keeps its normal editor behaviour. ⌘. cancels a running index build.
5. The View, Navigate and BRN menus list the items above.
6. Start a divider drag, release outside the window, and relaunch: the width persists. Also drag, then switch apps mid-drag: the drag ends. Relaunch: dragged widths persist.
7. ⌘Q with a dirty draft is still blocked by the guard.

- [ ] **Step 8: Commit**

```bash
git add crates/brn-desktop/src/main.rs crates/brn-desktop/src/native
git commit -m "desktop: add resizable dividers, shortcuts and menus

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 6: Documentation, decision record and evidence

**Files:**
- Create: `docs/ui/feature-backlog.md`
- Create: `docs/architecture/decisions/2026-10-01-workspace-shell.md`
- Modify: `docs/architecture/decisions/README.md`, `docs/architecture/overview.md`, `crates/brn-desktop/README.md`, `docs/roadmap.md`, `docs/README.md`, `docs/status.md`, `docs/superpowers/specs/2026-10-01-workspace-shell-design.md` (status line), `docs/work/active/workspace-shell/evidence.md`, `docs/work/active/README.md`

**Interfaces:**
- Consumes: the implemented module layout from Tasks 1–5 and the observations recorded in evidence.
- Produces: durable documentation only.

- [ ] **Step 1: Create the feature backlog**

Create `docs/ui/feature-backlog.md`:

```markdown
# UI feature backlog

Handoff features hidden from the workspace shell until a backend exists for them. Each was intentionally left out of [slice 1](../superpowers/specs/2026-10-01-workspace-shell-design.md) so the shell never presents fixture data as connected behaviour. Source: the BRN UI/UX handoff (`UI-SPEC.md`, `COMPONENT-INVENTORY.md`). The handoff assumes a Pi SDK provider; this repository uses the Codex App Server, so provider-facing items must be mapped to `brn-workflow` capabilities rather than re-implemented in the UI.

| Feature (handoff component) | Planned slice | Prerequisite |
| --- | --- | --- |
| Model and thinking-effort control (ModelEffortControl) | 2 · Chat polish | Workflow exposes provider model and effort capabilities and read-back |
| Context usage, capacity and estimate markers (ContextUsage) | 2 · Chat polish | Provider-reported usage surfaced by workflow |
| Explicit context chips (ContextChip) | 2 · Chat polish | Workflow accepts explicit attachments for a turn |
| Evidence inspection with Return (SourceInspection) | 2 · Chat polish | Existing evidence data; navigation stack design |
| Connection status, reconnect and actionable provider errors | 2 · Chat polish | Provider state surfaced by workflow |
| Notes, folder hierarchy and Archive branch (VaultExplorer) | 3 · Vault rail and note editor | [Safe Markdown note editing](../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md); archive design |
| Editorial Markdown reader and Markdown/reader toggle (DocumentReader, MarkdownView) | 3 · Vault rail and note editor | Safe Markdown rendering decision |
| Selection toolbar: Ask AI and Add comment (SelectionToolbar) | 3 · Vault rail and note editor | Editor selection API |
| Revision list with AI events, Undo AI edit, restore as new revision (RevisionList, UndoPreview) | 4 · History and diff | [Roadmap 13](../roadmap.md#13-revision-from-a-comment-batch) for AI edits |
| Diff with Show previous, change navigation, Expand all, Changes/Clean (RevisionDiff) | 4 · History and diff | Diff algorithm decision |
| Comment batch revision (CommentBatch) | 4 · History and diff | Roadmap 13 |
| Review queue and pending count (ReviewQueue) | 5 · Review and publication | Roadmap 13 and 14 |
| Exact publication preview, Approve & publish, publication states (PublicationReview) | 5 · Review and publication | [Roadmap 14](../roadmap.md#14-approval-and-publication) |
| Manual import review with metadata, provenance and placement (ImportReview) | 5 · Review and publication | Import-to-working-draft workflow |
| Persist unsent composer text across relaunch | Later | Credential-free local persistence decision |
| VoiceOver hierarchy, announcements and high-contrast qualification | Later | GPUI accessibility support review |

Remove a row only when its slice ships and its evidence is recorded.
```

- [ ] **Step 2: Create the decision record**

Create `docs/architecture/decisions/2026-10-01-workspace-shell.md`:

```markdown
# Workspace shell UI decisions

Date: 1 October 2026. Status: accepted for slice 1 by the user during brainstorming on 1 October 2026 ([spec](../../superpowers/specs/2026-10-01-workspace-shell-design.md)).

## Context

The BRN UI/UX handoff selects a combined workspace: history, document and chat, and vault, in a refined-terminal style. The desktop had a single-column page switcher. GPUI Kit 0.6.6 remains provisional under the [architecture baseline](2026-09-28-architecture-baseline.md).

## Decisions

- Build the shell on GPUI Kit 0.6.6. This does not end GPUI's provisional status or the deferred native editor acceptance.
- Restructure `brn-desktop` in place. `Desktop` remains the single owner of workflow state; regions are render-only `impl Desktop` modules. Layout rules and tokens are GPUI-free and unit-tested.
- Persist layout and appearance in a versioned `layout.json` in the explicit data directory. It is presentation state only, not authoritative application data, and not credentials or content. Corrupt or unknown files fall back to defaults.
- Follow macOS system appearance by default, with Dark and Light overrides using the handoff tokens.
- Use a transparent titlebar that carries the BRN header.
- Hide handoff features without a backend and track them in the [UI feature backlog](../../ui/feature-backlog.md).

## Alternatives rejected

- One GPUI entity per region: rewrites state ownership and risks dirty-state and stale-response guards.
- A parallel new desktop binary: doubles maintenance.
- Fixture-backed previews of unbacked features: the handoff rejects fixtures that pose as connected behaviour.

## Consequences

SQLite authority is unchanged; `layout.json` can be deleted without data loss. VoiceOver qualification remains open. Later UI slices add regions or controls inside this shell.
```

Append to `docs/architecture/decisions/README.md` after the existing bullet:

```markdown
- [2026-10-01: workspace shell UI](2026-10-01-workspace-shell.md): GPUI shell layout, render-only regions over a single `Desktop` owner, presentation-only `layout.json`, system appearance and hidden unbacked features.
```

- [ ] **Step 3: Update architecture, crate README, roadmap and index**

1. `docs/architecture/overview.md`, in the ownership table, replace the `brn-desktop` row's responsibility with: `GPUI workspace shell (history, document/chat, vault), interaction state, presentation-only layout preferences and sample headless checks`. In **Build boundaries**, append the sentence: `The desktop's layout.json in the data directory is presentation state only; deleting it restores default layout without affecting authoritative data.`
2. `crates/brn-desktop/README.md`, replace the **Interfaces and source** paragraph with:

   ```markdown
   [Entry point](src/main.rs), [layout model](src/layout.rs) and [tokens](src/tokens.rs) (GPUI-free, default-feature tests), [native shell](src/native/mod.rs) with [theme](src/native/theme.rs) and [regions](src/native/shell/mod.rs), [draft UI](src/drafts.rs), [comment UI](src/comments.rs), [CLI tests](tests/cli.rs). Layout and appearance persist to `layout.json` in the data directory. See the [workspace shell decision](../../docs/architecture/decisions/2026-10-01-workspace-shell.md) and the [UI feature backlog](../../docs/ui/feature-backlog.md).
   ```

3. `docs/roadmap.md`, append a section before **Scope and sequencing**:

   ```markdown
   ## UI redesign slices

   The handoff workspace is delivered in slices: 1 workspace shell ([design](superpowers/specs/2026-10-01-workspace-shell-design.md)), 2 chat polish, 3 vault rail and note editor, 4 history and diff, 5 review and publication. Hidden handoff features and their prerequisites are tracked in the [UI feature backlog](ui/feature-backlog.md). Slices after 1 need their own design approval.
   ```

4. `docs/README.md`, add a table row after the safe Markdown editing row:

   ```markdown
   | Review the UI redesign | [Workspace shell design](superpowers/specs/2026-10-01-workspace-shell-design.md), [UI feature backlog](ui/feature-backlog.md), [shell decision](architecture/decisions/2026-10-01-workspace-shell.md) |
   ```

- [ ] **Step 4: Record evidence and status**

1. Complete `docs/work/active/workspace-shell/evidence.md` with:
   - the date, tested commit(s) and worktree branch;
   - toolchain and features;
   - every command run, with its result;
   - the manual observations from Tasks 3–5, each marked observed, failed or skipped;
   - any GPUI signature adaptations;
   - the intentionally dropped UI lines from Task 4 Step 7;
   - remaining limitations: VoiceOver unqualified, Ask not exercised without authorised provider access, and user acceptance pending.
2. `docs/status.md`, add an **Implemented baseline** row: `| Workspace shell | Combined history/document-chat/vault shell, handoff theme, system appearance, persisted layout, reflow, dividers and shortcuts; default and native tests plus native observations recorded | [Shell](work/completed/workspace-shell/evidence.md) |`. Update **Next work** to mention UI slice 2 as a candidate needing design approval. Add the limitation that VoiceOver and user acceptance of the shell remain open.
3. Spec status line: `Status: Implemented (slice 1); verification recorded in the workspace shell evidence. User acceptance pending.`
4. Move the task folder: `git mv docs/work/active/workspace-shell docs/work/completed/workspace-shell`. Remove its row from `docs/work/active/README.md` and add it to `docs/work/completed/README.md`, following that file's existing format. Repair any relative links in the moved files: from `completed/workspace-shell/`, the spec is `../../../superpowers/specs/...`, which is unchanged.

- [ ] **Step 5: Documentation checks**

```bash
git diff --check
grep -rn "work/active/workspace-shell" docs crates README.md AGENTS.md
```

Expected: no whitespace errors. The grep finds no stale links except historical text inside the moved evidence, if any. Validate every new or changed relative link resolves, including heading fragments such as `roadmap.md#13-revision-from-a-comment-batch`, with a local link check script or by opening each target.

- [ ] **Step 6: Commit**

```bash
git add docs crates/brn-desktop/README.md
git commit -m "docs: record workspace shell decision, backlog and evidence

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Spec coverage

| Spec requirement | Task |
| --- | --- |
| §1 decisions: GPUI, hidden-and-tracked features, JSON persistence, System appearance, reflow, hybrid titlebar, approach A | 1–6 (tracking: 6) |
| §2 `layout.rs` (GPUI-free, unit-tested), `theme.rs`, `shell/*`, single `Desktop` owner | 1, 2, 3, 4 |
| §3 header, status line, history rail, centre (chat-first, draft, source reader), vault rail, Settings (Workspace and Connection), navigation preserving composer, selection, document and scroll | 4 |
| §4 bounds, reflow order, chat-first minimum, collapsed 28 pt rail, auto-collapse not persisted, 480×480 minimum | 1, 3, 4 |
| §4 Focus | 1, 4 |
| §4 persistence: missing, corrupt and version behaviour | 2, 3 |
| §4 appearance | 2, 3, 4 |
| §4 keyboard, menus and dividers | 5 |
| §4 accessibility limitation | 6 (evidence and status) |
| §5 unit tests | 1, 2 (+ native pure tests 3, 4) |
| §5 checks and manual evidence | 3, 4, 5, 6 |
| §6 documentation | 6 |
| §7 out of scope respected | Global constraints |

## Risks and handling

- **GPUI API drift:** Signatures were checked against the cached gpui-pre 0.3.6 and gpui-component 0.6.6 sources. Adapt minor differences without changing behaviour, and note each one in evidence.
- **Guard regressions from moved code:** Blocks A–C are pasted verbatim. Task 4 Step 11 exercises dirty switching, closing and comments natively.
- **Escape and modality:** Settings uses the toolkit modal host, which owns focus trapping and Escape. No global Escape binding is registered, so editor Escape handling is unaffected.
- **Theme coverage:** Handoff colours are installed as gpui-component `ThemeConfig`s, so cached tokens and editor highlight colours resolve together. Any control still showing toolkit defaults needs a key added to `tokens::theme_config_json`, not a post-hoc field write.
- **Intermediate warning gates:** The layout and tokens modules carry a temporary `#[allow(dead_code)]` until Task 5 Step 5, because native consumers arrive across Tasks 3–5.
- **Tab focus on dividers:** If Tab traversal does not reach a divider, clicking still focuses it for arrow keys. Record the observed behaviour; do not add a custom focus manager in this slice.
