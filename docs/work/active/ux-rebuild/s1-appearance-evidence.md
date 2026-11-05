# S1 — Appearance (System/Light/Dark) slice evidence

Slice S1 of the BRN UX rebuild (BRN-UX-002). Spec §§5, 7, 9, 12 (THEME-01..04),
AC-18, AC-29. Status words follow the repository rule; this records
*implemented* and *verified (automated)* work and a *partial* native
observation. Final native usability and owner acceptance remain separate gates.

## Branch and commit

- Branch: `work/ux-s1-appearance`, from `origin/work/ux-rebuild-20261105` (base `8c02ca56`).
- Head: `7309a8ee85b97587e2a756d60bb747c18227b9f7`.
- Commits:
  - `807ea9b9` theme: add Appearance preference (System/Light/Dark + legacy)
  - `86c85d18` core/app: non-fencing appearance preference persistence (THEME-02)
  - `7309a8ee` desktop: System/Light/Dark appearance UI, live OS, non-fencing save
- Files touched (8; all within S1 ownership, no edits to `threads_shell.rs`,
  `threads_layout.rs`, `threads_components.rs`, or the Render/layout code):
  `brn-writing-ui/src/theme.rs`, `brn-threads-core/src/store.rs`,
  `brn-threads-app/src/settings.rs`, `brn-threads-app/tests/thread_lifecycle.rs`,
  `brn-desktop/src/threads_ui.rs`, `threads_tiles.rs`, `threads_lists.rs`,
  `threads_omakase_tests.rs`. 617 insertions, 121 deletions.

## What was implemented

### Preference model (THEME-01) — `brn-writing-ui/src/theme.rs` (additive)
- New `Appearance` enum over the resolved `ThemeId` palette:
  `System | Light | Dark | Legacy(ThemeId)`.
- `Appearance::from_key(Option<&str>)`: `"system"`/absent/unknown → `System`
  (the new default); `"light"` → `Light`; `"dark"` → `Dark`; a recognized legacy
  key (`sammal`, `laanemeri`, `raba`, `virmalised`, `paber`) → `Legacy(theme)`.
  An explicitly saved legacy key is never silently reset.
- `Appearance::resolve(os_dark) -> ThemeId`: `System` → `Light`/`Dark` from the
  OS appearance; explicit choices ignore the OS. `follows_os()` is true only for
  `System`. `key()`, `label()`, `description()`, `PRIMARY = [System, Light, Dark]`.
- No existing `Palette`/`ThemeId` field renamed; the lead's Light/Dark token
  pairs, legacy palettes, `metrics` and new roles are used as-is. The thread-local
  pre-apply fallback changed from `Sammal` to `Light` (the System default resolves
  to Light), harmless since `apply` runs before the first frame.

### Persistence without fencing (THEME-02) — core + app
Root cause of the old deferral (investigated): theme lived in the single
versioned `WorkspaceSettings` record. A run binds its authority to that record's
version (`brn-ai/src/threads.rs` `for_settings(id, version)`, rechecked in
`live()` and at `prepare_owner`); bumping the version would fence/interrupt the
run. The old code therefore *deferred* the theme write while any run was Working
(`store.rs update_workspace_settings` returns `ApplyOutcome::Deferred`). This is
a genuine authority binding, not merely a shared generation counter — so the
fix is to persist the preference **outside** the versioned record.

- `crates/brn-threads-core/src/store.rs`: new singleton `presentation` table
  (`CREATE TABLE IF NOT EXISTS presentation(id INTEGER PRIMARY KEY CHECK(id=1),
  theme TEXT)`), modeled on `workspace_profile`. Created idempotently on every
  open, so existing schema-13 workspaces gain it with no schema bump; the whole
  SQLite file (and thus this table) travels with the backup-API backup and
  restore. New `Store::presentation_theme()` / `save_presentation_theme(theme)`
  (owner-only validation `[a-z-]{1,40}`, immediate transaction, no receipt, no
  run fence).
- `crates/brn-threads-app/src/settings.rs`: `Workspace::appearance()` (read),
  `save_appearance(key)` (non-fencing write, persists before returning),
  `adopt_legacy_theme()` (one-time carry-over of a legacy `settings.theme`). The
  legacy `choose_theme`/`SettingsUpdate::Theme` settings-write path is kept
  intact for callers wanting a settings revision (and for the unrelated
  `manual_inbox` tests that use it as a settings-write vehicle).
- The desktop never reports a durable save that has not happened: `save_appearance`
  persists synchronously and only a real write error surfaces a notice.

### Live OS appearance (THEME-01, THEME-04) — `threads_ui.rs`
- Startup resolves the saved `Appearance` against the live window OS appearance
  (`os_dark(window)` reads `window.appearance()`, treating `Dark`/`VibrantDark`
  as dark) and applies **before the first normal frame**, avoiding a bright
  flash in dark startup. `adopt_legacy_theme()` runs first so an owner's earlier
  theme carries over.
- `cx.observe_window_appearance(window, …)` re-resolves and re-applies on a live
  OS change, but only when the preference is `System` (`follow_os_appearance`).
  GPUI Kit 0.9.0 (gpui-pre 0.3.11) exposes `Window::appearance()`,
  `observe_window_appearance` and `WindowAppearance`; there is no
  `ThemeMode::System` and none was invented.
- `choose_appearance` applies the resolved palette immediately and persists via
  the non-fencing path. The `theme_pending` field and the deferred-retry on run
  finish were removed (no longer needed). Theme changes preserve editor text,
  drafts, selection, focus, scroll, proposals and run state (THEME-03): only
  `theme::apply`, note-mark sync and diff re-sync run; no run/editor/draft state
  is touched.

### Settings and launcher (AC-18, §9) — `threads_tiles.rs`, `threads_lists.rs`
- Settings > General > **Appearance**: primary System/Light/Dark segmented
  control (toolkit `segmented`, keyboard reachable, accessible names
  `"<Label>: <description>"`, selection marked by control state — not colour
  only) above an **Advanced palettes** disclosure (`appearance-advanced`
  `open_sections` key) listing the five legacy palettes with swatches.
- `Command::Theme(Appearance)` (payload type changed; variant name kept so the
  S2-owned `threads_shell.rs` `Command::Theme(_)` wildcard still compiles).
  `Command::all()` now emits System/Light/Dark first, then the legacy palettes
  under clear labels (`"Appearance: Raba (legacy) · …"`). `run_command` calls
  `choose_appearance`.

## Theme coverage sweep (§9.3)

Scope: hard-coded colours (`rgb(0x`, `hsla(`, literal `Hsla`) in
`crates/brn-writing-ui/src` and `crates/brn-desktop/src`, excluding the S2-owned
`threads_shell.rs`, `threads_layout.rs`, `threads_components.rs` and the
sidebar/list rendering in `threads_lists.rs`.

Result: **no hard-coded colour literals found in any in-scope file** (counts
below). The codebase already routes every colour through palette roles via
`pal()` / `theme::apply`, so Markdown/code highlighting, selection, diffs,
comments, menus, tooltips and inputs follow the resolved light/dark mode. Syntax
highlighting follows the toolkit mode, which `theme::apply` switches with
`Theme::change(mode, …)` before setting colours (THEME-04). Representative
palette-role uses confirmed in `brn-writing-ui/src/document.rs` (diff add/delete
via `p.add`/`p.delete`/`p.ink`) and `components.rs` (`tone.color()`).

- `rgb(0x…)` literals in `brn-writing-ui/src` excluding `theme.rs`: **0**
- `rgb(0x…)` literals in `brn-desktop/src` (all files): **0**
- `hsla(…)` literals in either crate: **0**
- The only `Hsla`/hex occurrences are: `theme.rs` palette definitions (the token
  source, by design) and omakase-test assertions reading palette values
  (`rgb(palette.ink)` etc., legitimate), plus `threads_components.rs:211`
  `rgba(0)` (transparent border, S2-owned, not a theme colour).

### Literals found in S2-owned files (reported, not changed)
- Sidebar/list rendering in `threads_lists.rs`: **no colour literals** (`rgb(0x`,
  `hsla(`, `Hsla`, 6-hex): none.
- `threads_components.rs:211`: `.border_color(gpui_kit::rgba(0))` — a fully
  transparent border, not a theme colour; nothing to route through a role.
- `threads_shell.rs`, `threads_layout.rs`: no colour literals found.

## Tests (added/updated)

- `brn-writing-ui/src/theme.rs` (4 new): `appearance_defaults_to_system_and_
  round_trips_every_key`, `an_explicitly_saved_legacy_choice_is_not_reset`,
  `system_resolves_against_the_operating_system_appearance`; existing contrast
  and `keys_round_trip…` tests kept.
- `brn-threads-app/tests/thread_lifecycle.rs`: `appearance_persists_without_
  fencing_a_working_run` (immediate non-fencing save, Settings record untouched),
  `legacy_settings_theme_is_adopted_once_as_the_appearance_preference`,
  `choose_theme_still_defers_the_settings_record_write_while_working` (legacy
  path contract preserved).
- `brn-desktop/src/threads_omakase_tests.rs`:
  `appearance_switches_from_settings_and_launcher_without_fencing_runs` (System
  default → Light; launcher legacy + Light; Advanced disclosure reveals legacy
  rows; save while a run is Working does not fence it; reopen restores
  preference), `live_os_appearance_reapplies_only_under_system_and_keeps_composer_
  text` (System follows a simulated OS flip to dark and back; an explicit Light
  choice ignores the OS flip; composer draft preserved throughout),
  `companion_focus_and_theme_changes_keep_navigation_readable` (per-palette
  toolkit channels, now driven through `choose_appearance`),
  `launcher_keeps_a_matching_command_ahead_of_chat_messages` and the
  `launcher_finds…` list test updated to the new command labels.

Live-OS note: GPUI Kit 0.9.0's public test surface cannot flip a window's OS
appearance (`TestWindow::simulate_appearance_change` is `pub(crate)` and the test
platform's `window_appearance()` is fixed to Light). The test exercises the same
resolution the `observe_window_appearance` callback runs via a `#[cfg(test)]`
seam `simulate_os_appearance`, which routes through the identical
`follow_os_appearance`/`apply_resolved_theme` logic.

## Verification (Rust 1.98.1, --locked, --jobs 6; TMPDIR under /private/tmp)

Document helpers built with `scripts/build-document-helpers.sh --offline --jobs 1`
(both `brn-xberg-helper` and `brn-xberg-helper-brn`); `BRN_XBERG_HELPER*` exported
for the full desktop/app suites.

- `cargo fmt --check` — clean.
- `cargo clippy -p brn-writing-ui --features native-ui,native-test-support --all-targets --locked -- -D warnings` — clean.
- `cargo test -p brn-writing-ui --features native-ui,native-test-support --locked` — pass (theme unit tests incl. the 4 new; all suites green).
- `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked -- -D warnings` — clean.
- `cargo test -p brn-desktop --features native-ui,native-test-support --locked` — **387 passed, 0 failed, 1 ignored**.
- `cargo test -p brn-threads-app --locked` — all suites pass (incl. `thread_lifecycle` 3/3 new theme tests, `manual_inbox` 23/23 with helpers).
- `cargo test -p brn-threads-core --locked` — all suites pass (50 lib unit tests + integration suites).

## Native observation (partial)

Built `cargo build -p brn-desktop --features native-ui --locked`; launched
`target/debug/brn-desktop --data-dir <fresh /private/tmp dir>`; window resized to
1280×928 and captured with the owner's `windows.swift` + `screencapture -x -o -l`
(BRN window only — the full screen was never captured). The OS was in light mode
throughout and was **not** changed (owner's machine). Captures in
`/Users/evokessler/repos/brn-ux-rebuild-artifacts/shots/s1/`:

- `appearance-system-light-os.png` — default **System** (fresh workspace),
  resolving to the neutral Light palette; Settings > Appearance block showing the
  System/Light/Dark segmented control + Advanced palettes disclosure (Show 5).
- `appearance-light.png` — explicit **Light** selected.
- `appearance-dark.png` — explicit **Dark** selected; the whole app (title bar,
  nav, settings panel, segmented control, button) renders in the dark palette.
- `appearance-system-switch.png` — switched back to **System**; palette
  re-resolved from Dark to Light (System follows the light OS).

Observed: the default on a fresh workspace is System (not a legacy palette);
System resolves to the neutral Light tokens; explicit Dark applies the full dark
palette across all surfaces; switching back to System re-resolves to the OS
appearance. The launcher appearance commands (`Appearance: Dark`,
`Appearance: System`) applied correctly.

Pending native evidence (recorded as Partial/Blocked per spec §16):
- **Live OS light↔dark switch while System is selected** is covered only by the
  automated seam test; a real OS-appearance flip was not performed because the
  owner's machine appearance must not be changed. Record this as the documented
  platform limitation for AC-29's "change the OS appearance while System is
  selected" step.
- The Advanced-palettes disclosure expansion and a legacy-palette native capture
  were not recorded (pointer-coordinate automation was unreliable); the behaviour
  is verified by the automated omakase test (`appearance-advanced-toggle` →
  `settings-theme-virmalised`).
- Dark-startup no-flash (THEME-04) is implemented (resolve-before-first-frame)
  but not visually captured, since the OS could not be set dark at launch.
- Full AC-29 coverage across an open conversation/editor draft/proposal/dialog in
  both themes is a native journey that remains a separate gate.

## Behaviour decisions

- **Non-fencing write over deferral.** Chosen because the deferral is a real run
  authority-version binding, not a shared generation/fence; a separate
  presentation record removes the fencing entirely while keeping every store
  contract (owner scope, immutable intent, replay, Undo, version checks)
  untouched and needs no schema bump.
- **Kept the legacy `choose_theme` settings path.** It is still a valid
  capability and unrelated `brn-threads-app` tests (`manual_inbox`) depend on it
  as a settings-write vehicle; removing it would silently weaken their coverage.
- **`Command::Theme(Appearance)` keeps the variant name** so the S2-owned
  `threads_shell.rs` wildcard match compiles unchanged.
- **Resolve System from the window appearance**, not `App::window_appearance()`,
  because the per-window value is what `observe_window_appearance` reports live
  and what the test harness can drive.
