# UI polish: implementation plan

**Date:** 15 October 2026. **Branch:** `work/ui-polish` in `.worktrees/ui-polish`, fast-forwarded to `771116f0` (main with #144 document conversion and #145 helper sub-agents); no commits of its own yet.
**Design authority:** [plan.md](plan.md) (critique, design direction, components, layout, copy, small-window and focus rules, tests). This file orders the work into slices and adds the items that arrived after design: the open round-5 findings, the base move and the reliability branch's desktop findings.
**Step detail:** each slice has a feature file under `feat/features/` with concrete steps, acceptance criteria and commands; `feat/context.json` holds the build, test and convention facts. Those artifacts are workflow files and stay uncommitted, so they are cited as paths, not links.

## Design review status

[design-review.json](design-review.json) (round 5) is **CHANGES_REQUESTED**: 0 HIGH, 1 MEDIUM, 4 NIT. The MEDIUM is a must-fix and each NIT is a should-fix, all owned by a slice:

| Finding | Severity | Slice | Requirement |
|---|---|---|---|
| M1 Stacked comments can't be opened during review | MEDIUM, **MUST-FIX** | S4 | `toggle-margin` (Comment icon plus count, same ID, handler and state) renders in the review bar in every margin mode, as the bar's last control before the primary; only one of toolbar and review bar renders. Below 480 pt the Proposal label reads "Proposal", with the shape ("whole note", "N notes", "N changes") in its tooltip and accessible name next to the +a −r counts. The one-row fit at 900×600 (about 374 of ~382 usable pt) is confirmed from measured widths. Layout rule: during review, the review bar's `toggle-margin` expands and collapses `comments-section`, which sits between the review text and the footer. Tests: at 900×600 in Write, on a protected note with two comments, `toggle-margin` is visible in the one-row bar; clicking it expands `comments-section` between the review text and the footer, and both cards render; the footer stays fully visible; Escape collapses the section and `toggle-margin` reopens it. At 1250×860 in review the cards render beside the text (`threads_omakase_tests.rs` ~827). |
| N1 Spec line 15 | NIT, should-fix | S4 | Writing polish spec line 15 becomes "Feedback is reachable beside both views (in a narrow Note tile, in the in-tile comments section below the text)…". Lines 3 and `docs/status.md` line 11 stay as historical records. |
| N2 Focus after Escape | NIT, should-fix | S4 | When Escape collapses `comments-section`, focus moves to the visible editor (`focus_visible_editor`, as `cancel-comment` does), or to the review text during review. The Escape test asserts it. |
| N3 Order and height share | NIT, should-fix | S4 | Below the text: `comments-section`, `writing-run-status`, `writing-proposals` or `writing-result`, then the writing bar. The text keeps at least 120 px; the section's cap is 40 % of the height left after those rows, and it shrinks before the text. Test at 900×600 with a failed run, its partial reply and an expanded section: `note-body` ≥ 120 px. |
| N4 Theme journey test | NIT, should-fix | S2 | Rename to `themes_switch_from_settings_and_launcher_and_wait_for_running_work`; after the Settings switch to Virmalised assert `!theme_pending` and `settings().theme == Some("virmalised")`. |

## Hard constraints

- **Toolkit.** `gpui-kit = "=0.7.3"` (gpui-component 0.7.3, GPUI 0.3.9); `Cargo.lock` is the authority. No version bumps, no new dependencies. Supported Kit capabilities only:
  - `TabBar` and `RadioGroup` replace child IDs, so segmented controls are `ButtonGroup`s.
  - `Alert` cannot hold actions, so banners are BRN's own.
  - There are no editor selection bounds, so the selection strip stays a row.
  - `Diff` is line-based, so prose review stays BRN's own.
- **Safety invariants.** Preserve protection (Approve on protected writes), exact-candidate review (Apply binds the displayed text; the untruncated footer says so in every state and at every width), version checks and stale refusals, replay, recovery and Undo. Every write keeps its existing checked service call.
  - Important status stays visible: needs you, unsaved, changed since your edit began, protected (also while dirty), failed with its recorded reason, paused, archived, reviewing, partial coverage, needs refresh, still running elsewhere, Stop.
  - Decluttering never hides a consequence.
- **IDs and tests.** Controls that stay visible keep their element IDs and accessible labels. Moved controls follow plan.md's "Actions that become menu-only" table. Tests are updated to new labels or menu items chosen by label, never weakened or deleted.
- **Shared seams.** Do not edit `crates/brn-ai/src/threads.rs`, `crates/brn-threads-app/src/chat.rs` or `crates/brn-threads-app/src/intake.rs`.
  - Leave `threads_ui.rs::useful_intake`'s logic alone (U2 lands later), and add no lease check to `resume_chat_queue`.
  - Do not change service reason strings, and do not edit `docs/status.md` (shared; applied at integration).
- **Evidence tooling.** The observation harness (`observation/*.swift`, `*.rs.txt`, `snap.sh`) is evidence tooling only. Never copy it into `crates/` or `scripts/`, never compile it into the product, never commit it as product source.
- **No native windows outside the gated verify step.** Slice and review steps run headless only: GPUI test harness, `--headless-check startup`. Data directories live under `/private/tmp/...`.
- **Git.** Local commits on `work/ui-polish` only. Never push, merge, publish, release or touch `main`. Stage files by name; leave `feat/`, `reviews/` and `code-review.json` uncommitted.

## Commands

From `/Users/evokessler/repos/brn-rust/.worktrees/ui-polish` with `PATH=/opt/homebrew/opt/rustup/bin:$PATH` and `CARGO_TARGET_DIR=/Users/evokessler/repos/brn-rust/.worktrees/ui-polish/target`:

- Once: `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline`.
- Focused: `cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline <filter>`.
- **Slice gate**, recorded in the feature file's `findings` with the test count:
  - `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` builds the helper and the desktop, runs clippy with `-D warnings` on all targets, then every native test; serial avoids the known dialog-timing flake
  - `cargo fmt --all -- --check`
  - `python3 scripts/check-markdown-links.py`
  - `git diff --check`
- **Full gate** for the integration loop and the verify step: the slice gate plus CI's other desktop lanes. These are `cargo clippy` and `cargo test` for `-p brn-desktop` with `--features native-ui` alone and with `--features native-ui,native-retrieval`, plus `cargo test -p brn-desktop --locked --offline` (default features, which compile `threads_state.rs` tests) and `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`. The full workspace test suite is not rerun when `git diff --stat 771116f0 -- crates ':!crates/brn-desktop'` is empty; say so in the evidence.

## Changes since the design

- **Base move (#144, #145).** `threads_tiles.rs` gained the "Helper sub-agents" Settings section (`helper-provider-*`, `helper-effort-*`, the helper model input, `HELPER_TESTED_NOTE`). `threads_ui.rs` gained the helper settings in `configure`, run notices appended to finish text, more import kinds and a "Document conversion was stopped before saving." path. plan.md's Settings → Models design now covers the helper section (S6). The stopped-conversion notice is a warning, because the owner stopped it. Contract line numbers moved: target lines 388 and 390; the Settings theme hint is at `threads_tiles.rs` ~1180.
- **Added scope** from `work/daily-workflow-reliability` (its evidence.md, "UI findings for the UI agent (final)"), desktop-only:
  - **U6, long failure reasons** such as "Run failed: Reconnect this account before continuing. Continue when the provider is ready."
    - Needs-you rows and chat rows show one line with an ellipsis, with the full text in the tooltip and accessible label (S6).
    - The chat recovery card shows the full reason, wrapping (S5). For the same reason, the Note's `writing-run-status` line also wraps in full (S3). Both supersede plan.md's one-line truncation for these two places only.
    - Tests seed their own failure text and assert it passes through; none asserts the service's default failure wording.
  - **U3, identical review reasons.** Review items for prepared changes show their target note titles and a Protected marker when a protected note is affected. This uses data the UI already has: `Attention.record` holds the operation, `self.candidates` holds `Prepared.request.writes`, and the records hold `protected`. The service string stays unchanged (S5 helper and cards, S6 rows). If a surface lacks the data, `evidence.md` records what the service should expose.
  - **U1, no Stop for an orphan.** A saved Working run with no local worker gets a secondary "Stop…" (`stop-saved-run-{run}`) in the chat recovery card and in the Note writing bar's still-running state (S5).
    - It opens a consequence dialog that says another BRN window or the CLI may still be running that reply. ↵ is inert, and the confirm is a `danger()` "Stop reply".
    - Confirm calls the existing `stop_chat_run`. On `771116f0` that is `ChatCommand::Pause { run }`: it pauses the queue, marks a Working Run Interrupted and bumps its fence, so later writes from whoever runs it are refused.
  - **Out of scope here:** U2 (needs `Workspace::source_intake_chat`, which exists only on the reliability branch), U4 (needs the service's new error) and U5 (an owner rule).
- **Refinement.** `Finished(Err)` after the owner's own Stop (`worker.cancel.is_cancelled()`) is a warning; every other `Finished(Err)` is an error, as designed.

## Slices, in priority order

Each slice leaves the tree building and every test passing, ends with the slice gate and a local commit, and is reviewed before the next starts.

- [x] 1. **S1 Foundation: tokens, tones and shared components** (`feat/features/FEAT-001.json`).
      - Semantic color roles (hairline, danger, warning, success and info plus tints) and Kit tone channels in `threads_theme.rs`, with the five-theme contrast matrix.
      - Lucide icon composition (`icon_assets!` beside `Assets`).
      - New `threads_components.rs`: `icon_button`, `segmented`, `overflow_menu`, `section_header`, `list_row` and its two-line form, `empty_state`, `banner`, `status_chip`, `toolbar`, `prose_editor_state`, and the consequence-dialog helper. Glyph controls become icons; nested bordered panels are retired.
      - `Tone` notices: `inform`, `confirm`, `warn` and `fail` with the `notice_tone_for` guard, mapped at every notice site; `last_work_notice: Option<(Tone, String)>`.
      - `human_time`; new `TileKind::min_width` values plus the pure `panel_widths` and `margin_mode`.
      - The keyboard ⋯-menu test, which decides the ⌘K fallback.
      Files: `threads_theme.rs`, `threads_components.rs` (new), `threads_ui.rs`, `threads_shell.rs`, `threads_layout.rs`, `threads_lists.rs`, every file with notice writes, test files.
      Verify: `cargo test … theme` and `… layout` pass, then the slice gate. Done when every glyph control is an icon with its label, no double-bordered nesting remains, the error, warning and Info-overwrite notice tests pass, and the contrast tests cover the new roles.

- [x] 2. **S2 Shell and panel chrome** (`feat/features/FEAT-002.json`).
      - Bars: title bar with segmented Write/Chat/Solo; navigation bar; status bar with run state and Stop, `status-save`, the "Layout" menu and Keys, with **`status-theme` removed** and no theme item in the status bar.
      - Panel headers: sentence-case titles with icons and exactly one header ⋯ with a Panel section. `note-menu` moves into the Note header. Split chip.
      - Launcher: sizing and grouping, destination labels, a Projects command, Kbd shortcuts. Keys help rows, Compact chrome, and the horizontal scrollbar on overflow.
      - "Also in ⌘K." hint; Omakase spec lines 12 and 17 annotated with the owner decision.
      - **Should-fix N4.**
      Files: `threads_shell.rs`, `threads_tiles.rs`, `threads_lists.rs`, `threads_layout.rs`, `docs/work/active/omakase-ui/spec.md`, `threads_omakase_tests.rs`, `threads_tiled_tests.rs`.
      Verify: `cargo test … omakase` and `… tiled`, then the slice gate. Done when each shell control appears once, theme is reachable only from Settings → General and ⌘K (with deferral and persistence), and no launcher label starts with "Open tile:".

- [x] 3. **S3 Note tile: writing** (`feat/features/FEAT-003.json`).
      - One toolbar row (Write | Read, formatting, the shared Outline, `note-status-chip`, `toggle-margin`, Edit note, Connections), collapsing at about 760, 620 and 480 pt from measured widths. `section-*` retires into Outline.
      - Note ⋯ order, with "Discard edit…" last. Four toned banners, with `note-stale-banner` and `note-stale-preview`; the selection strip restyled.
      - The seven-row first-match writing bar and `writing-run-status`, which wraps in full (U6). The Discard dialog (`discard-note-confirm`, ↵ inert).
      - Gutter-free prose editors through `prose_editor_state`, and the 680 px measure.
      Files: `threads_writing.rs`, `threads_workbench.rs`, `threads_document.rs`, `threads_shell.rs`, `threads_context_actions.rs`, `threads_projects.rs` (editor constructor only), `threads_state.rs` copy, Note test files.
      Verify: `cargo test … writing`, `… workbench` and `… document`, then the slice gate. Done when there is at most one filled primary in every writing state, the toolbar is one row at 900×600, 1050×700 and 1250×860 (dirty or clean), all four banners are visible at 900×600, and content gets ≥ 70 % of the Note height at 1250×860.

- [x] 4. **S4 Note tile: review and comments** (`feat/features/FEAT-004.json`).
      - The one-row review bar: every row of the state table, Changes | Result (`review-show-changes` / `review-read-result`), Approve on protected writes, and the collapse at 620 and 480 pt.
      - The per-shape exact-text footer, never clamped. The review section of the Note ⋯ (Compare side by side, Edit final text, Inputs, then Keep for later and Dismiss below 620 pt).
      - Comment placement through `margin_mode` (Wide 280, Narrow 220, Stacked `comments-section`), reusing the margin renderers and IDs.
      - **MUST-FIX M1** and **should-fix N1, N2, N3** (table above). Contract corrections: writing polish spec lines 7 and 15, and target lines 388 and 390.
      Files: `threads_writing.rs`, `threads_shell.rs` (`tile_row` margin width), `threads_layout.rs` (if needed), `docs/work/active/writing-polish/spec.md`, `docs/architecture/threads-target.md`, review and comment tests.
      Verify: `cargo test … review` and `… comment`, then the slice gate. Done when the M1 tests pass; at 900×600 the review bar is one row with Protected visible and the footer whole; and every comment capability works in Stacked mode with no card in Chat.

- [x] 5. **S5 Chat** (`feat/features/FEAT-005.json`).
      - The transcript: 680 px measure, "You" and "BRN" labels, Copy icon, "Save as decision…" ghost.
      - Toned needs-you and proposal cards that name every target note, with a Protected chip (U3, through a pure `review_summary`/`review_label` in `threads_state.rs`).
      - The recovery card through a pure `continue_path`, which `continue_run` now dispatches on:
        - state word, then the full recorded reason, wrapping (U6)
        - line 2 by path, and the "W waiting messages stay paused" line
        - **U1** "Stop…" with the consequence dialog, here and in the Note's still-running state
      - Queue notices (first match wins) and a queue panel that labels other chats.
      - Archive copy and one Unarchive (`reopen-thread`); Steer and Stop consequences.
      - A one-row composer: model display names, model · effort below 420 pt, "Queue" while work runs.
      Files: `threads_writing.rs`, `threads_chat.rs`, `threads_state.rs`, `threads_ui.rs` (`continue_run`), `threads_workbench.rs` (Stop in the Note), chat test files.
      Verify: `cargo test … steering` and `… chat`, then the slice gate. Done when each shot state (22, 23, 25, 29, 40) shows one clear next action with its consequence visible, the U1/U3/U6 tests pass, and the existing steering tests still pass.

- [x] 6. **S6 Lists and workspace pages** (`feat/features/FEAT-006.json`).
      - Chats, Notes and All chats rows: two-line needs-you rows, one-line ellipsis with full text in the tooltip and label (U6), and review labels with Protected (U3). One-row A–Z strip and Topics row.
      - Home: first-run intro only; the pending delegation card stays independent of first run; rows; human times.
      - Tasks: sections, one outline main action per open task, and a row ⋯ by state. Changes: Background · N, the "May need refresh" sub-row, and Undo ghosts.
      - Settings: segmented sections in a 640 px column. Models adds the helper sub-agent section with the tested-models note visible and one "Save settings" covering it. Delegation uses Radios and a Switch; Data keeps the backup folder and status lines visible.
      - Projects: Active | Trash, a Projects ⋯, per-state primaries, and the danger-confirm delete dialog.
      - Search: Spinner and error banner.
      Files: `threads_tiles.rs`, `threads_lists.rs`, `threads_shell.rs` (needs badge), `threads_projects.rs`, `threads_search.rs`, `threads_connections.rs`, `threads_writing.rs` (`linked_tile`), list, page and project tests.
      Verify: `cargo test … projects` and `… focus`, then the slice gate. Done when no page nests bordered cards, each section has at most one primary, the U3/U6 row tests pass, and the helper settings save as before.

- [x] 7. **S7 Dialogs, focus, small windows, complete states and copy sweep** (`feat/features/FEAT-007.json`).
      - Every dialog gets its ↵ class (`on_ok`), a `Dialog::footer` action row, height at most window − 96, and, for multi-line forms, the ⌘↵ `Enter` capture plus `on_action::<Ask>`. Pick-list rows become tab stops.
      - The focus landing table. Escape order, with Settings moved to a bubble-phase handler.
      - The Short-window queue collapse.
      - An audit of every empty, loading, error, disabled and success state, recorded in findings, and a final wording sweep (no tile, thread, candidate, receipt or "What changed" in visible strings; sentence case; human times).
      Files: `threads_ui.rs`, `threads_projects.rs`, `threads_context_actions.rs`, `threads_shell.rs`, `threads_chat.rs`, all view files for the sweep, dialog and focus tests.
      Verify: `cargo test … dialog` and `… focus`, then the slice gate. Done when the dialog, focus, Escape and 900×600 no-overflow tests pass and the audit list has no open item.

After S7, the integration loop runs the full gate on the combined tree, fixes the seams between slices, and reviews the whole branch against this file and plan.md.

## Integration watch items

These tests seed a Working run with no live worker. None of them calls Resume or Dispatch on it; keep it that way. After merging with `work/daily-workflow-reliability`, rerun them, because that branch interrupts lease-free orphans in `resume_queue`:

- the U1 Stop tests (chat card and Note writing bar)
- the recovery-card "May still be running" and "Resume queue" label tests
- the existing `threads_steering_tests.rs` Working-run fixtures and `threads_projects_tests.rs` ~176 (pre-existing)

Two more items for integration:

- `docs/status.md`, `threads-behavior.md` and `invariants.md` wording from both branches is applied once, at integration.
- U2 (`source_intake_chat` in `useful_intake`) lands with or after the reliability merge; this branch leaves that function's logic untouched.

## Workflow contract

- **Slice loops.** Each slice runs its own implement → review loop. The coder implements (iteration 1) or fixes every finding in the slice review (iteration 2+). The reviewer, which runs last, writes `docs/work/active/ui-polish/reviews/code-review-s<N>.json` every iteration with `{"verdict": "APPROVED" | "CHANGES_REQUESTED", "slice": "S<N>", "findings": [...]}`. The loop stops on APPROVED.
- **Integration loop.** After S7, a fixer runs the full gate and fixes the seams (iteration 1), or fixes the review's findings (iteration 2+). The reviewer writes `docs/work/active/ui-polish/code-review.json` last, every iteration. It may write `"verdict": "APPROVED"` only when:
  - all seven feature files are `completed` and every `reviews/code-review-s<N>.json` is APPROVED
  - every must-fix and should-fix above is implemented or explicitly justified
  - the full gate passes on the final tree
- **Native observation.** Only the verify step observes natively.
  - Before launching or driving any BRN window, it sends the parent a warning asking for a go-ahead, and waits.
  - It uses `/private/tmp/...` data and a scratch export outside Git for the observation harness, and quits every window it launched.
  - It then sends an info message saying so.
- **Record step.** Writes `evidence.md` with implemented, verified, native-observed, owner-accepted and merged reported separately, then commits locally. It does not push, merge, publish or release.
  - It commits `plan.md`, this file, `design-review.*`, `evidence.md`, `before/` and `after/`.
  - It also commits `observation/`, because `plan.md` links to it and CI checks links. Those files are documentation under `docs/` only and keep their `.rs.txt`, `.swift` and `.sh` names.
- **Iteration limits.** A slice loop that reaches 3 iterations without approval pauses for the parent; the integration loop aborts after 3.

## Review checklist

Slice and integration reviewers check each item, and rerun the gate themselves:

1. Every step and acceptance criterion in the slice's feature file is implemented and tested.
2. Every must-fix and should-fix item assigned to the slice is implemented and tested exactly as written above.
3. Protection, exact-candidate review (the review footer is never truncated), version checks, replay, recovery and Undo are intact.
   - Every write uses its existing checked service call.
   - Important status and consequences stay visible.
4. `git diff 771116f0 -- Cargo.lock crates/*/Cargo.toml` is empty. Only supported Kit 0.7.3 capabilities are used.
5. Visible controls keep their IDs and accessible labels. No test assertion was weakened or deleted without an equivalent replacement.
6. No edits to `crates/brn-ai/src/threads.rs`, `crates/brn-threads-app/src/chat.rs`, `crates/brn-threads-app/src/intake.rs`, `docs/status.md` or `useful_intake`'s logic. No observation-harness file under `crates/` or `scripts/`.
7. Copy follows plan.md's Copy guidelines. Tests for U6 seed their own failure text.
8. Tokens, components and wording are consistent with the slices already merged into the branch.

## Out of scope

As listed in plan.md, plus U2, U4 and U5, any service or schema change, `docs/status.md`, and any lease or liveness API.
