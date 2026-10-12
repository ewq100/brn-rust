# Omakase native UI rebuild evidence

**Date:** 12 October 2026. **Baseline:** `0602258` (PR #124). **Branch:** `ewq100-omakase-native-ui-rebuild`; the PR records the exact published head. [Scope](spec.md) and [plan](plan.md) define the assignment.

## Implemented boundary

- Native shell rebuilt as tiles: `threads_ui.rs` keeps state and the existing checked service calls; `threads_shell.rs` (top bar, tile row, status bar, launcher, menus), `threads_tiles.rs` (threads, all threads, notes, overview, Actions, What changed, Settings) and `threads_writing.rs` (chat, composer, note, margin, review, compare, linked work) render it. Pure `threads_layout.rs`, `threads_lists.rs`, `threads_diff.rs` and `threads_theme.rs` carry unit-tested rules.
- App/core additions: `Workspace::reopen_thread` and `resolve_thread_because` (existing owner checked path; `resolve_thread` unchanged); `WorkspaceSettings.theme` (serde default, omitted when absent; no SQL schema change) written only by `Store::choose_theme`, which returns `Deferred` inside the writer transaction while any run is Working and is preserved by Configure.
- No new dependencies or lockfile change. No provider calls, real data, merge, release or push to main.

## Local verification

macOS arm64, Rust 1.98.1, locked offline dependencies, `CARGO_TARGET_DIR=/tmp/brn-ui-repairs-target-20261010`, synthetic temporary data only. Logs are in `/tmp/brn-omakase-ui-logs/` on the implementation host.

| Check | Result |
|---|---|
| Baseline `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop --locked --offline` at `0602258` | 56 passed (`baseline-native.log`). |
| Same command on the candidate | **83 passed**, 0 failed: 18 new pure unit tests (layout, lists at 5,000 notes, quick-question rule, grants, launcher, diff, theme contrast) and 9 new Omakase journeys, with every earlier suite migrated to the tile model (`native-final.log`). |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings` | Pass (`native-clippy.log`). |
| `cargo test -p brn-threads-app --test thread_lifecycle --locked --offline` | 3 passed: theme deferral without stopping a Working run, Configure preservation, legacy serialized form; reopen/resolve reasons. |
| `bash scripts/verify-storage.sh` with explicit `BRN_INTAKE_HELPER`, `BRN_PDFTOTEXT`, `BRN_PDFIMAGES` | Pass on the final tree: formatting, workspace build, strict all-target Clippy, **461 passed / 3 ignored** (baseline record 458), headless startup (`storage-final.log`). |
| `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, each `-p brn-desktop --features native-ui,native-retrieval --locked --offline` | Pass; 38 tests (`combined.log`, `combined-clippy-final.log`, `combined-test-final.log`). |
| `python3 scripts/check-markdown-links.py`; `git diff --check` | Pass. |

New journeys exercise real headless clicks, double-clicks, right-click menus, gap and header drags, key bindings and the launcher: layouts/resize/swap/maximize/tile menu; theme switching from status bar, launcher and Settings with deferral while a run works and persistence on reopen; resolved threads hidden as a count, filter/all-threads/launcher discovery and reply-to-reopen; auto-resolve for a completed quick question but not for linked, two-message or failed-run threads; notes tile sections, letters, filter and New thread about this note with 240 notes (bounded lists at 5,000 are unit-tested); mode pill grants and reset; comment disabled with reason during an unsaved edit; in-place review, compare, exact Apply, Mark addressed and receipt Undo; Edit final text creating a new candidate before Apply; card Apply/Dismiss refused while that proposal's final text is being edited; proposal card Dismiss and launcher commands; a note draft following the note's first thread.

## Independent review

A read-only code review of the full branch diff reported three findings, all fixed with regression tests: (1) preparing edited final text re-entered `keep_review` on the retired candidate, leaving review stuck — replacement candidates now replace the review directly; (2) chat-card Apply/Dismiss could discard final text being edited — they now refuse with a notice; (3) a note's saved draft became unreachable when its first thread was created while another thread was shown — the draft now moves to that thread. The review found no issues in the theme store path, grant mapping, auto-resolve rule, recovery-edit handling or layout focus. During review the lead also made re-opening the candidate already under review keep edited text.

## Toolkit limits and deliberate deviations

- No floating selection toolbar or hover card: GPUI Kit 0.7.1 exposes neither selection pixel bounds nor editor hover offsets. A selection strip above the text and margin cards (Show passage) replace them. Highlights use the editor's range decorations; review uses a read-only editor with text decorations.
- The tracked-change view interleaves deleted text with the candidate as Markdown source; rendered figures remain available in the compare tile.
- Day groups (Today/This week/Older) use UTC dates.
- A theme chosen while another process has a working run stays applied and is saved after the next local run finishes or when chosen again.
- Toolkit accessibility snapshots report `disabled` only when true and not Button `selected`; tests assert behavior (no effect when clicked, visible reason, view state) instead.

## Not established

Native interaction was not performed. Headless tests do not prove pointer feel, drag thresholds, menus as rendered on macOS, focus rings, IME, text selection feel, scrolling, theme appearance or responsiveness, nor live-provider behavior.

## Owner UI qualification checklist

Use a fresh data directory with synthetic notes (`cargo run -p brn-desktop --features native-ui -- --data-dir /absolute/fresh/threads`).

1. Layouts: Write/Chat/Solo buttons and ⌥1/⌥2/⌥3; drag a gap to resize; drag a header onto another tile; double-click a header; right-click a header (Maximize, Move left/right, Close, Open tile); ⌘[ ⌘] ⌘⇧↵ ⌘\.
2. Themes: switch all five from the status bar, ⌘K and Settings › General; check readability of text, buttons, highlights and tracked changes; reopen the app and confirm the theme persisted.
3. Launcher ⌘K: find a note, an open and a resolved thread, run commands; ↑/↓/↵/Esc.
4. Threads: ask a quick standalone question and watch it resolve itself; reply to it and see it reopen; resolve a thread and find it via the resolved count and all-threads filters; Needs you badge and list.
5. Notes tile: Recent, Linked, Protected & imported, A–Z letters, filter and note-text search; right-click New thread about this note.
6. Writing: type in place, ⌘S and status-bar Save, Discard, recover an edit after restart; select text → strip → Comment ⌘/ and right-click menu; confirm Comment is disabled with the reason while unsaved; include/exclude in rewrite; Rewrite with AI ⌘⇧R.
7. Review: proposal card preview; Show in note; tracked changes for a light edit and a heavy rewrite; inputs ⓘ, Compare, Edit final text → use this text, Dismiss, Apply/Approve; Mark addressed; Undo.
8. Composer: ↵ send, ⇧↵ newline, ⌘↵; mode pill (Ask / Edit open note / Record decision) resets after sending; model and effort pills change Settings; ＋ intake choices; Stop and Stop & send during a run.
9. Overview ⌘0, Actions, What changed (Undo) and Settings sections (Models, Delegation, Data: export and backup).
10. Narrow window (850×600): every control reachable by scrolling the tile row and tiles.
