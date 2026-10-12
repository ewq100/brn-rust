# Omakase native UI rebuild implementation plan

**Goal:** Replace the page-based native shell with the accepted Omakase tiled workspace, five themes, thread lifecycle refinements and in-place writing review, then publish a reviewed PR without merging.

**Baseline:** `0602258` (merged PR #124) on isolated branch `ewq100-omakase-native-ui-rebuild`; all 56 native widget/state tests pass at baseline (`/tmp/brn-omakase-ui-logs/baseline-native.log`).

**Spec:** [Accepted scope](spec.md). **Architecture:** pure, unit-tested presentation modules (theme, layout, lists/diff) under the existing GPUI view; tile renderers in focused child modules that call the same checked service methods; two small app/core additions (thread reopen/reasoned resolve and a run-deferred theme preference). **Stack:** Rust 1.98.1, GPUI Kit 0.7.1, unchanged lockfile.

## Global constraints

- No new dependencies, SQL schema changes, provider calls, real data, merge, release or push to main.
- Preserve checked writes, Save/dirty/stale guards, grants, exact-candidate review, receipt Undo, rewrite snapshots, anchoring, Stop-and-send, cancellation/recovery, export/backup, intake and delegation policy.
- Use the word **thread**; resolve only; resolved threads never expand inline.
- Lead owns integration and all files; review is independent and read-only.

## Task 1 — app/core support

- [x] `Workspace::reopen_thread(id, reason)` and reasoned `resolve_thread_because`; existing `resolve_thread` unchanged.
- [x] `WorkspaceSettings.theme: Option<String>` (serde default, skipped when absent) and `Store::choose_theme`, which returns `Deferred` while any run is Working, preserves all other settings, and is excluded from Configure overwrites. Core/app tests for replay, deferral and preservation.

## Task 2 — pure presentation modules

- [x] `threads_theme.rs`: five palettes, derived tints, toolkit theme application, contrast tests.
- [x] `threads_layout.rs`: presets, open/close/toggle, swap, move, resize by fraction, maximize, focus cycling.
- [x] `threads_lists.rs`: activity ordering, sidebar/all-thread grouping and filters, notes sections at scale, quick-question rule, mode-to-grant mapping, launcher filtering.
- [x] `threads_diff.rs`: bounded paragraph/word diff producing a display text and decoration ranges.

## Task 3 — native shell and tiles

- [x] Top bar, tile row (resize/swap/maximize/context menu), status bar, notice, launcher, keys dialog and key bindings.
- [x] Threads, all threads, notes, chat (composer, pills, cards), note (always editable, margin, selection strip, foot bar), review in place, compare, linked work, overview, Actions, What changed and sectioned Settings tiles.
- [x] Auto-resolve on completed runs; reply/comment/rewrite reopen resolved threads.

## Task 4 — verification and delivery

- [x] Replace page-based native tests coherently; add focused journeys for layout, theme, resolved hiding/reopen, auto-resolve, notes at scale, mode grants, comment-disabled-while-unsaved and review apply/undo/addressed.
- [x] Native tests, strict native Clippy, default workspace build/lint/tests as affected, formatting, Markdown links. Record commands/results/limits in [evidence](evidence.md).
- [x] Update target editor section, desktop README and status; independent code review; PR with owner UI qualification checklist. Do not merge.

## Gate record

- Defined: owner accepted variant B and requested the build with five themes; scope recorded in [spec](spec.md).
- Implemented and verified locally; independent review findings fixed and retested ([evidence](evidence.md)). Published as a PR for owner review.
- Not merged, released or native-observed; the owner UI qualification checklist in the evidence is the remaining gate.
