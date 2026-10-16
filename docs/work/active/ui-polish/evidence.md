# UI polish: evidence

**Date:** 16 October 2026. **Branch:** `work/ui-polish` (local only, not pushed), base `771116f0`. The first native observation ran on `ed8daa82`. The [fix pass](#fix-pass-native-defects-integration-findings-u7-u8-u10) commits (`3fdf5489` and its [review round 2](#fix-pass-review-round-2), `354c1940`) change product code after it. They were [rechecked natively](#native-recheck-of-the-fix-pass-354c1940) on `354c1940`. **Design:** [plan.md](plan.md). **Slices:** [impl-plan.md](impl-plan.md). **Detailed verification and native observation:** [verification.md](verification.md).

Each status below is reported on its own. Being implemented or verified does not mean the change is owner-accepted or merged.

| Status | State |
|---|---|
| Implemented | yes, slices S1–S7 plus one integration fix, all in `crates/brn-desktop` and three contract docs |
| Verified (automated) | yes, full gate on `354c1940` (263 / 76 / 76 / 28, clippy clean), rerun by the verify step; earlier on `ed8daa82` and `9fecaea1` |
| Native-observed | partly, with real input on the product build: the first observation on `ed8daa82`, then a recheck of the fix pass on `354c1940`. The gaps are listed below |
| Owner-accepted | **no** |
| Merged / published / released | **no** |

## Implemented

Commits `13d6224e`…`ed8daa82` on top of `771116f0`: 33 files, about 16,000 lines added and 3,200 removed. Outside `crates/brn-desktop`, only the writing polish spec, the Omakase spec and `threads-target.md` changed (owner-decision contract corrections). `Cargo.lock`, the crate manifests, the shared seam files (`brn-ai/src/threads.rs`, `brn-threads-app/src/{chat,intake}.rs`), `docs/status.md`, `threads-behavior.md`, `invariants.md` and the logic of `useful_intake` are unchanged.

- **S1 Foundation** (`13d6224e`, `7634d613`). Semantic tones (danger, warning, success and info, each with a tint) and a hairline role, wired into Kit's tone channels. Contrast tests cover all five themes. Lucide icons replace the glyph controls. New `threads_components.rs` holds the icon button, segmented control, overflow menu, list rows, empty state, banner, status chip, toolbar, gutter-free prose editor and consequence dialog. Notices carry a tone that an overwrite can't inherit. Also: human-readable times, and the pure `panel_widths`/`margin_mode` helpers.
- **S2 Shell** (`2c2dff2b`). A segmented Write/Chat/Solo control, sentence-case panel headers with icons and a single ⋯ per panel (including a Panel section), and a status bar with no theme control (owner decision 4). Theme is now set in Settings → General and ⌘K. Launcher commands name their destinations, and a Projects command was added.
- **S3 Note writing** (`01b9e9e9`). The toolbar is one row that collapses by measured width. Four toned banners, including the stale-edit banner with a preview of the saved version. The writing bar picks its primary from the first matching state row. The Discard dialog confirms with a danger button and ↵ does nothing. Editors use a 680 px measure with no line gutter.
- **S4 Review and comments** (`87e2a289`, `c82d142e`). The review bar is one row, and Approve is shown for protected writes. The exact-text footer is never truncated. Comments are placed Wide, Narrow or Stacked (owner decision 2), and the round-5 must-fix M1 and should-fixes N1–N3 are done.
- **S5 Chat** (`2ccc9fe8`, `3e1dd17f`, `7db71705`). The recovery card is built by the pure `continue_path`, which `continue_run` now dispatches on. It shows the full recorded failure reason (U6). Review cards name their target notes with a Protected chip (U3). "Stop…" works for a saved Working run with no local worker (U1, consequence dialog, existing `stop_chat_run`). Also: queue notices, one Unarchive, and a one-row composer.
- **S6 Lists and pages** (`ba5d0c59`, `56938c69`). Two-line needs-you rows, and one-line rows that ellipsize with the full text in the tooltip and label. Tasks get one main action plus a row ⋯. Changes groups bookkeeping receipts under "Background · N" (owner decision 1). Settings gets segmented sections and the helper sub-agent section. Projects gets Active | Trash, a Projects ⋯ and a danger-confirm delete.
- **S7 Dialogs, focus, states** (`6843df35`). Each dialog's ↵ behavior is set by class, with footer action rows. Focus lands in a defined place, Escape closes things in a defined order, and the queue collapses in short windows. Every state was audited and the copy swept.
- **Integration** (`ed8daa82`). Settings and the chat activity box now use the shared panel tokens.

All seven slice reviews and the whole-branch integration review are APPROVED, with seven non-blocking findings left open (see Open items).

## Verified (automated)

On `ed8daa82`, Rust 1.98.1, with `--locked --offline`:

| Check | Result |
|---|---|
| `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` (helper and desktop build, clippy `-D warnings`, native tests) | **245 passed**, 0 failed |
| `cargo test -p brn-desktop --features native-ui` | **75 passed** |
| `cargo test -p brn-desktop --features native-ui,native-retrieval` | **75 passed** |
| `cargo test -p brn-desktop` (default features) | **27 passed** |
| clippy `-D warnings`, both feature sets and `--workspace --all-targets` | clean |
| `cargo fmt --all -- --check`, `git diff --check` | clean |
| `python3 scripts/check-markdown-links.py` | 0 failures; rerun on this record commit |

The workspace test suite was not rerun, because `git diff --stat 771116f0 -- crates ':!crates/brn-desktop'` is empty.

### Sync with `origin/main` `8bef439b`

Merge commit `9fecaea1` (`--no-ff`, not a rebase) brings in #146 quality-simplify (one `safe_markdown` policy with `external_link` and `stored_image`, and the rendered-Markdown Read-view test), #147 docs and #148 status. `threads_writing.rs` and `threads_document_tests.rs` merged automatically. The one conflict was the end of `threads_ui.rs`, where both sides appended a test module. Both are kept unchanged: the branch's `presentation_tests` (icons, notice tones) and #146's `policy_tests`. `markdown()` and `safe_message_for_thread` both go through `safe_markdown`, so there is still one rendering policy. No service reason string and no `useful_intake` logic changed. `Cargo.lock` and the crate manifests are unchanged from `771116f0`, and `docs/status.md` carries only #148's change.

Full gate on `9fecaea1`:

| Check | Result |
|---|---|
| `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` | **248 passed**, 0 failed (245 plus #146's two policy tests and its Read-view test) |
| `cargo test -p brn-desktop --features native-ui` | **75 passed** |
| `cargo test -p brn-desktop --features native-ui,native-retrieval` | **75 passed** |
| `cargo test -p brn-desktop` (default features) | **27 passed** |
| clippy `-D warnings`, both feature sets and `--workspace --all-targets` | clean |
| `cargo fmt --all -- --check`, `git diff --check` | clean |
| `python3 scripts/check-markdown-links.py` | 116 files, 432 local links, 0 failures |

The workspace test suite was not rerun: `git diff --stat 771116f0 -- crates ':!crates/brn-desktop'` is still empty. The merge was not observed natively.

## Native-observed

**How:** real keyboard and mouse input (CGEvents with Accessibility trust) on the unmodified product build, wrapped in scratch `.app` bundles. Windows were captured with ScreenCaptureKit. The data was synthetic, in fresh `/private/tmp` directories, with the `offline-fixture` provider and no provider requests. The observation harness was not used. Window use ran from 07:21 to 07:49 EEST on 16 October 2026, and every window was quit afterwards.

**Observed** at 900×600 (and once at 1250×860), in light and dark themes:

- the shell and its status bar, with no theme control
- ⌘K theme switching, and theme switching from Settings
- Note Write, with Stacked comments opened, drafted into and closed with Escape
- review with the whole footer, the ⋯ menu, Apply and Undo
- a dirty edit and the Discard dialog (↵ does nothing; the danger button confirms)
- the chat states: needs you, failed with its reason and the paused-queue line, and archived
- Tasks, Changes, Settings and a Data backup
- Projects and the Move to Trash dialog
- the first-run empty states

Shots are in [`after/`](after/); [`before/`](before/) holds the baseline.

**Defects seen natively** (detail in [verification.md](verification.md)). All five are fixed in the fix pass below and automated-verified. The [recheck](#native-recheck-of-the-fix-pass-354c1940) saw all of them fixed natively except 5c and the menu-launched dialogs outside Projects ⋯:

1. Closing with Escape a dialog opened from a ⋯ menu item loses keyboard focus until the next click.
2. The queue strip in the narrow chat companion truncates its state to "1 waiting · Pa…".
3. Clipping at narrow widths: the delegation card (the clipped shot was replaced by the fixed card, [43](after/43-delegation-card-900-light.png)), the stacked comment draft input, Notes rows, and Navigation panel titles.
4. Copy: Apply/Undo notices run the service reason straight into the hint. After Undo, comments read "Passage not found". The backup path appears twice. Changes times show UTC.
5. Minor: the Settings focus ring stays on General; Continue and Send are both filled; the Note text box is about 110 pt in the busiest 900×600 state.

**Not observed:**

- live streaming, Stop/Steer and U1 Stop… on an orphaned run (these need a running worker)
- U3 rows with Protected, and the stale-edit, newer-version, import and recoverable-edit banners
- attachments, Compare, Search, Trash restore, export and file pickers
- hover tooltips, drag-resize, 1050×700, Solo, the Läänemeri and Paber themes, VoiceOver, IME, contrast measurement, and keyboard-only use of the ⋯ menus

## Fix pass (native defects, integration findings, U7, U8, U10)

Implemented and automated-verified when written. The verify step later rechecked most of these items natively on `354c1940`; see [the recheck](#native-recheck-of-the-fix-pass-354c1940). Service strings, the shared seams, `useful_intake`, `resume_chat_queue`, `docs/status.md`, `Cargo.lock` and the manifests are unchanged.

| Item | Fix | Tests |
|---|---|---|
| 1 Focus after Escape on a menu-launched dialog | Every BRN dialog opens through `ThreadsUi::open_dialog`. When the click came from a menu (focus in a `PopupMenu`) or nothing is focused, focus first moves to the focused panel's landing from the focus table (editor, composer, search, filter or panel body), so Kit gives it back on close. Covers Projects ⋯ (all four dialogs), Note ⋯ Edit note… and Discard edit…, Chats ＋ New note and the row menus' Move to project…. | `escape_from_a_projects_menu_dialog_keeps_the_keyboard_working` (failed before the fix), `escape_from_a_menu_launched_dialog_returns_focus_to_the_panel` (Note ⋯, Chats ＋ and, since review round 2, a chat row's Move to project…): focus in the panel, ⌘K opens the launcher, Tab moves within the workspace |
| 2 Queue strip "1 waiting · Pa…" | The state leads and never truncates ("Paused · 2 waiting", "Queue paused · 2 waiting"); the count truncates first; full text is the tooltip and accessible name. plan.md's Short rule updated. | `a_short_window_collapses_the_paused_queue_to_one_row` (narrow Write companion, `queue-summary-state` whole before Resume queue) |
| 3a Delegation card | Explanations wrap inside the card (beside the button when there is room); "notes?" is one unit, so "?" never wraps alone. | `the_delegation_card_keeps_its_text_inside_a_narrow_chat` |
| 3b Stacked draft input cut off | The draft sits above the cards' scroller, and while it is open the section's and the text area's floors include the whole draft. | `stacked_comments_keep_every_comment_capability` now asserts `comment-input-box` whole inside the section |
| 3c Notes rows "Protect…" | Protected notes show the Lock chip (as in Chats), announced "Protected". | `a_narrow_notes_list_marks_protected_notes_with_the_lock` |
| 3d Navigation titles "…" | Below 220 pt a panel header leaves Maximize to its ⋯ Panel section (also double-click and ⌘⇧↵). Listed in plan.md's menu-only table since review round 2. | `a_narrow_list_header_keeps_its_title_and_its_menu_opens_beside` |
| 4a, U8 Receipt reasons in notices | `applied_message`: "Saved." or "Undone.", " Running work was stopped." only when the receipt moved a Working run, then the refresh hint, each its own sentence. The recorded reason stays in Changes. | `saved_notices_name_changes_and_the_refresh_follow_up` (unit), `delegation_and_maintenance_notices_never_show_a_receipt_reason` |
| 4b "Passage not found" after Undo | `unlocated_reason`: "Tied to v1, not re-anchored to v3." when the comment is tied to an earlier version; "Passage not found in vN." only when the service unresolved it. `exact_writing_range` unchanged. | `unlocated_comments_say_what_is_true` (unit), `review_in_place_applies_exact_candidate_then_marks_addressed_and_undoes` |
| 4c Backup path twice | With Settings → Data shown, the notice reads "Full backup created." (or "Markdown snapshot created."); the status line keeps the path. Elsewhere the notice keeps the path. | `first_picker_saves_then_backs_up_and_saved_click_reloads_folder_and_uses_fresh_name` |
| 4d UTC times | `human_time` shows the Mac's local time (C `localtime_r` per instant, no new dependency; a small documented `unsafe` call, Unix only since review round 2, with a UTC fallback elsewhere); the tooltip keeps the UTC ISO value. The All chats Today / This week grouping is unchanged (UTC days). | `human_times_read_as_today_or_a_short_local_date` (UTC, UTC+3, day rollover, DST) |
| 5a Settings ring on General | A pointer click on a section moves focus to the Settings body; a key press keeps it on the pressed section. | `a_clicked_settings_section_takes_the_focus_ring_off_general` |
| 5b Continue and Send both filled | Beside an available recovery card, Send is outline (as in an archived chat). | `a_failed_reply_with_a_paused_queue_names_what_stays_paused` asserts `recovery_card_leads()` |
| 5c Busiest Note text box | The expanded saved rewrite inputs moved out of the writing bar into their own row that gives way (to 40 pt) before the text; the bar no longer overflows the Note. `note-body` keeps 120 px. | `the_busiest_note_keeps_its_text_and_whole_bar_at_900x600` (the bar ran 160 pt out of the Note before) |
| INT-1 Kept-edit primary | Recover edit is the filled primary; a new `WritingRow::Recover` makes Rewrite with AI and Retry ghosts "unavailable: Recover the kept edit first". | `the_four_note_banners_stay_visible_in_write_at_900x600` |
| INT-2 Undo during an open edit | Base `771116f0` rendered no Undo while dirty (its dirty branch was exclusive); this branch had added it. Restored: `undo-writing-apply` is not rendered while the note has an open edit; Changes keeps Undo. No version check changed. | `bound_terminal_clarification_stays_in_note_and_unrelated_chat_cannot_replace_it` (hidden while dirty, back after Discard) |
| INT-3 Edit note and Connections | The fold uses this note's measured row (labels, chip, count) instead of the widest row. In the test text system (0.6 em per character, wider than macOS fonts) a plain note needs about 706 pt for that row and the Note at 1250×860 has 687, so there they still fold in tests; with system fonts the row is narrower. Natively at `354c1940` and 1250×860, a plain note kept both in the row ([46](after/46-plain-note-toolbar-1250-dark-virmalised.png)). | `edit_note_and_connections_stay_in_the_toolbar_when_this_notes_row_fits` (1330×860: plain inline, widest folded), `note_toolbar_folds_in_order_as_the_note_narrows` |
| INT-4 Stale actions at the floor | The first banner leads the stack whole with its actions pinned; only its text scrolls in a short Note. Later banners and the saved version share a box that gives way first, down to its first line. Since review round 2 the text keeps 120 px beside open comments when a banner shows and there is room, and goes below it (to 48 px) only in the measured infeasible state; see [review round 2](#fix-pass-review-round-2). | `the_banner_stack_keeps_its_budget_above_the_text_at_900x600` asserts `save-buffer-copy` and `show-current-note` in view, and `note-body` ≥ 120 px with the stale banner and open comments |
| INT-5 Untested states | Disabled reasons (INT-1, INT-2, INT-6 tests), empty states, Link document by keyboard, the running short-window queue row, the header ⋯ Open beside submenu. | `first_run_empty_states_name_the_next_step`, `link_document_rows_are_reached_by_tab_and_pressed_by_space`, `a_short_window_keeps_a_running_queue_to_one_quiet_row`, `a_narrow_list_header_keeps_its_title_and_its_menu_opens_beside` |
| INT-6 Disabled Show passage | Announced "Show passage “quote”, unavailable: {why}", tooltip "Show passage is unavailable: {why}". | `review_in_place_applies_exact_candidate_then_marks_addressed_and_undoes` |
| U7 Needs-you rows | Already correct: the Chats panel and Home use two-line rows with the title first, and the title-bar menu leads with the title. Confirmed, no code change. | `needs_you_rows_keep_the_chat_title_above_the_reason_in_every_list` |
| U10 One failure wording | When a run ends Failed or Interrupted (and the owner didn't stop it), the notice is the chat's recorded notice for that run (its Blocker attention, the text Needs you shows) in that state's tone, not the raw error. Without one, the previous "Work stopped: …" text stays. Corrected in review round 2: the first version read the run's progress. | `a_failed_or_interrupted_finish_shows_the_recorded_terminal_text` (seeded attention and a different progress) |

**Gate on `3fdf5489`** (superseded by [review round 2](#fix-pass-review-round-2)): the slice gate passed with **263 passed**, plus `native-ui` (76), default features (28) and `brn-desktop` clippy for default and `native-ui`. The `native-retrieval` lanes and workspace clippy were not run there. The commit message says 433 links and this record said 432; the round-2 run below is the current count.

### Fix pass review round 2

The workflow review `reviews/code-review-fix.md` (uncommitted, so cited as a path; verdict CHANGES_REQUESTED) found eight issues in `3fdf5489`. All eight are fixed below. They were implemented and automated-verified when this was written. The later native recheck did not cover FIX-1 (U10), FIX-2, FIX-3 or FIX-6's row-menu path.

| Finding | Fix | Tests |
|---|---|---|
| FIX-1 U10 read the agent's progress | `terminal_notice(run, run_id, attention)` reads the chat's Blocker attention whose `record` is this run (the Needs you text) and uses that state's tone. Run progress is never used. With no such attention, the previous "Work stopped: …" text and `finish_tone` stay. | `a_failed_or_interrupted_finish_shows_the_recorded_terminal_text` seeds that attention, an older run's notice and a different non-empty progress, then asserts that only the attention text is shown. It also asserts the fallback with no attention and the owner's own Stop. `record_terminal`'s doc comment now describes what it seeds. |
| FIX-2 Text floor under banners | The Note text asks for 120 px beside open comments even with a banner shown (a 120 px flex basis), and its area gives way last. Inside the area the open section gives way first (`YIELD_FIRST`). The text keeps a shrink factor of 1, so once the section is at its floor the text takes up the rest of the overflow and never runs onto the rows below. The 48 px floor applies only when the rows that never hide leave less, which is measured below. plan.md records the rule under "Height share (round-5 N3)", **owner-accepted 16 Oct 2026** (it was pending when this pass ran). | `the_banner_stack_keeps_its_budget_above_the_text_at_900x600`: with the stale banner, open comments and a dirty bar, `note-body` ≥ 120 px and `save-buffer-copy` and `show-current-note` are in view. In the busiest state every row that gives way before the text is at its floor (saved version 28, reply 32, open section 0), and the text fills the rest of its area and stops above the run's reason. The doc comment states both. |
| FIX-3 Windows build | `local_offset` is `#[cfg(unix)]`. It converts through `c_long::try_from` and `i64::from`, with the same-type lints allowed and the reason stated. A `#[cfg(not(unix))]` fallback returns 0 (UTC). | `human_times_read_as_today_or_a_short_local_date` is unchanged. No Windows target is installed here, so the fallback was checked by temporarily switching the two `cfg`s and running clippy `-D warnings` with `native-ui` (clean, then restored). Windows itself was not compiled. |
| FIX-4 Full gate | Run on this tree, see below. | — |
| FIX-5 Narrow Maximize | plan.md's menu-only table has a Maximize row: below 220 pt, `tile-max-*` moves to the header ⋯ Panel section. Double-click and ⌘⇧↵ also work, and the button stays at 220 pt and wider and as Restore. | — |
| FIX-6 Row menu Move to project… | The menu-Escape test right-clicks a chat row, chooses Move to project…, presses Escape and asserts focus in the workspace, ⌘K opens the launcher, and the chat's record is unchanged. | `escape_from_a_menu_launched_dialog_returns_focus_to_the_panel` |
| FIX-7 Doc comment | `finish_tone`'s doc comment is back above `finish_tone`. `terminal_notice` has its own. | — |
| FIX-8 Drift | The status table names this gate, plan.md's Changes row reads "Today 20:05", and the link count is from the run below. | — |

**Height budget, measured.** These are GPUI test-harness bounds at 900×600 in Write, in the test text system (0.6 em per character, wider than macOS fonts). Stacked comments are open, the notice is dismissed, and the Note body is 416 px.

| Row | Busiest stale state (stale edit, failed run, partial reply, dirty bar) | Stale edit, dirty bar, no run |
|---|---|---|
| Lead banner `note-banners` (pinned, actions in view) | 64 | 104.5 |
| `note-banners-more` (saved version) | 28 (its floor) | 28 (its floor) |
| `writing-run-status` with its padding and rule | 76.5 (three wrapped lines) | — |
| `writing-result` | 32 (its floor) | — |
| `note-bar` (feedback field and wrapped controls) | 132 (three control rows) | 100 |
| `comments-section` | 0 (gave way first) | 63.5 (gave way from 40 %) |
| `note-body` | 83.5 (all that is left) | 120 |

In the busiest state the rows that never hide take 332.5 px, which leaves 83.5 px. The text can't keep 120 px there unless one of them is hidden or clipped: the banner's actions (INT-4), the failed run's full reason (U6), the reply's label, or the writing bar. Before `3fdf5489` the banner's actions scrolled out instead. With macOS fonts the bar and the reason probably wrap to fewer rows, but that wasn't observed.

**Gate on the final tree** (`PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=<worktree>/target`, `--locked --offline`, Rust 1.98.1):

| Check | Result |
|---|---|
| `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` (helper and desktop build, clippy `-D warnings` on all targets, native tests) | **263 passed**, 0 failed |
| `cargo clippy -p brn-desktop --features native-ui --all-targets -- -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui` | **76 passed** |
| `cargo clippy -p brn-desktop --features native-ui,native-retrieval --all-targets -- -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui,native-retrieval` | **76 passed** |
| `cargo clippy -p brn-desktop --all-targets -- -D warnings` (default features) | clean |
| `cargo test -p brn-desktop` (default features) | **28 passed** |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check`, `git diff --check` | clean |
| `python3 scripts/check-markdown-links.py` | 117 files, 438 local links, 0 failures (it also scans the uncommitted workflow files under `feat/` and `reviews/` and `code-review.md`) |

The workspace test suite was not rerun, because `git diff --stat 771116f0 -- crates ':!crates/brn-desktop'` is empty. `Cargo.lock` and the crate manifests are unchanged from `771116f0`. The shared seam files and `docs/status.md` are unchanged from `origin/main` `8bef439b`.

### Native recheck of the fix pass (`354c1940`)

**Automated first.** The verify step reran the full gate on `354c1940` before opening any window. Every lane passed:

- `verify-desktop-shell.sh`: **263 passed**, 0 failed.
- `native-ui`: **76**. `native-ui,native-retrieval`: **76**. Default features: **28**.
- The four clippy lanes with `-D warnings`, fmt and `git diff --check` are clean.
- Links: 117 files, 438 local links, 0 failures, and 472 links with 0 failures after these doc edits.
- The workspace suite was not rerun, because nothing outside `crates/brn-desktop` changed.

Detail is in [verification.md](verification.md#full-gate-on-head-354c1940).

**Native, with real input.** Window use ran from 10:43:23 to 10:59:09 EEST, under the coordination Mac lock (10:43 to 10:59:21), after the parent's go-ahead. It used two scratch bundles of the product build and fresh synthetic `/private/tmp` data seeded through the shared service. The provider was `offline-fixture`, with no provider requests. Sizes were 900×600 in Sammal and Virmalised, plus 1250×860. Every window was quit, and no BRN process remains. The per-item table is in [verification.md](verification.md#recheck-of-the-fix-pass-head-354c1940).

- **Fixed and rechecked natively:**
  - native defects 1 (Projects ⋯ → Move to Trash… and Instructions & settings), 2, 3a, 3b, 3c, 3d, 4a (U8), 4b, 4c, 4d, 5a and 5b
  - INT-2
  - INT-3: with macOS system fonts at 1250×860, a plain note keeps Edit note and Connections, and all five format buttons, in the toolbar row. A note with three comments also keeps them, with formatting in Format ▾.
  - INT-6's disabled-Show-passage tooltip
- **Automated-verified only, not rechecked natively:**
  - 5c
  - INT-1: a restart reopened the unsaved edit as an open recoverable edit, not the kept-edit state
  - INT-4 and FIX-2: no stale banner in the fixture. The text-floor rule is still **pending owner acceptance**.
  - INT-5 beyond the empty states
  - U10: needs a finishing run
  - the Note ⋯, Chats ＋ and row-menu dialog paths of item 1
  - FIX-3, the Windows build
- **New native findings, not fixed:**
  1. With a stacked comment draft open at 900×600, the writing bar's last row is clipped at the Note's bottom edge. After Undo this includes the filled "Retry saved rewrite".
  2. While the paused queue holds a waiting message, Edit note says "unavailable: BRN is working" and the status bar says "Idle". The gating is from base; the reason wording is new.
  3. Changes' "May need refresh" pills name a writing-request record by its raw JSON, which overflows the panel. The `title()` naming is from base.
  4. The recoverable-edit needs-you row reads "Recov… [Unsaved]" at 170 pt.

Replaced shots: 01, 04, 05, 06, 08, 11, 14, 16, 29 (now `29-chat-failed-900-dark-virmalised.png`) and 43 (now `43-delegation-card-900-light.png`). New shots: 44 to 48.

## Owner-accepted

No. The owner has not reviewed the implemented UI.

## Merged

No. Nothing is pushed, merged, published or released.

## Open items

- **Native recheck gaps:**
  - The [recheck](#native-recheck-of-the-fix-pass-354c1940) confirmed natively: native defects 1 (Projects ⋯ paths), 2, 3a–3d, 4a–4d, 5a and 5b, plus INT-2 and INT-3.
  - Still automated-verified only: 5c, INT-1, INT-4/FIX-2, the rest of INT-5, U10, the other menu-launched dialog paths, and the Windows build.
  - INT-7 (the link check) was resolved by the record commit.
- **New native findings at `354c1940`** (not fixed): the writing bar clipped under an open stacked draft at 900×600; Edit note's "BRN is working" reason while the queue is only paused; raw JSON in Changes' refresh pills; the "Recov…" needs-you row. See [verification.md](verification.md#recheck-of-the-fix-pass-head-354c1940).
- **Fix-pass trade-offs to confirm:**
  - plan.md's height-share rule under a banner was owner-accepted on 16 October 2026 (pending when this record was written). With a Note banner shown, the text keeps 120 px beside open comments when there is room. In the measured busiest stale state at 900×600 it gets 83.5 px, above a 48 px floor, because banners, a failed run's reason and the writing bar are never hidden to meet the budget.
  - The All chats day grouping stays in UTC while times are local.
- **U10 sources elsewhere:** the status-line notice now reads the chat's recorded notice. The chat recovery card's reason line (S5) and the Note's `writing-run-status` (S3) still read the run's progress, which is the agent's own summary. On this base the recorded notice is the service's generic failure text, so aligning those two fits the reliability merge, where `failed_attention` names the cause. Not changed here.
- **U2:** `useful_intake` still creates a new chat on every supply. The fix is `Workspace::source_intake_chat`, which exists only on `work/daily-workflow-reliability`, so it lands with or after that merge.
- **U4:** offering Compare/Discard for an open stale buffer needs the service's new typed error in place of the raw SQLite UNIQUE failure. Not started here, and no string matching was added.
- **U5:** verbatim imports showing "Partial" is an owner rule. Unchanged.
- **U6 wording:** the UI shows the recorded reason in full and adds no cause of its own. The service-side change (the provider hint only for provider failures, keeping the "Run failed: " prefix) belongs to the reliability branch.

## Service follow-ups for `work/daily-workflow-reliability`

- **U3, review targets without a prepared candidate.** A Review attention names its operation (`Attention.record`). The chat's needs-you card names the target notes, and marks a protected one, from the matching prepared candidate (`Prepared.request.writes` and each record's `protected`). When that candidate is no longer prepared (applied, dismissed, stale or superseded elsewhere), the card and the needs-you row can show only the service reason "Review prepared changes". If the attention itself carried the target note IDs (or titles) and whether any of them is protected, every review surface could name its targets without the candidate. This branch leaves the reason string unchanged.

## Integration watch items

These tests seed a Working run with no live worker. None of them calls Resume or Dispatch on that run. After merging with `work/daily-workflow-reliability`, which marks lease-free orphans Interrupted in `resume_queue` and dispatch, rerun them, and hold a `RunLease` where a test needs the run to look live:

- the U1 Stop tests (chat recovery card and Note writing bar)
- the recovery-card "May still be running" and "Resume queue" label tests
- the existing `threads_steering_tests.rs` Working-run fixtures, and `threads_projects_tests.rs` around line 176

Also at integration:

- Apply the `docs/status.md`, `threads-behavior.md` and `invariants.md` wording from both branches once. This branch edits none of them.
- U2 (`source_intake_chat` in `useful_intake`) lands with or after the reliability merge.
- `resume_chat_queue` still stops a queue-claimed Working run without a lease check. This branch adds none, as agreed; a `run_lease::run_is_live` check fits there after the merge.
- Build the desktop and CLI from the same merged tree when testing leases.
