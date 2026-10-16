# UI polish: evidence

**Date:** 16 October 2026. **Branch:** `work/ui-polish` (local only, not pushed), base `771116f0`. Product code was at `ed8daa82` for the native observation; the record commit added only these documents, and the [fix pass](#fix-pass-native-defects-integration-findings-u7-u8-u10) commit changes product code after it. **Design:** [plan.md](plan.md). **Slices:** [impl-plan.md](impl-plan.md). **Detailed verification and native observation:** [verification.md](verification.md).

Each status below is reported on its own. Being implemented or verified does not mean the change is owner-accepted or merged.

| Status | State |
|---|---|
| Implemented | yes, slices S1–S7 plus one integration fix, all in `crates/brn-desktop` and three contract docs |
| Verified (automated) | yes, full gate on `ed8daa82` |
| Native-observed | partly, with real input on the product build; the gaps are listed below |
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

**Defects seen natively** (detail in [verification.md](verification.md)); all five are fixed in the fix pass below, automated-verified and not yet re-observed:

1. Closing with Escape a dialog opened from a ⋯ menu item loses keyboard focus until the next click.
2. The queue strip in the narrow chat companion truncates its state to "1 waiting · Pa…".
3. Clipping at narrow widths: the delegation card ([43](after/43-delegation-card-clipped-900-light.png)), the stacked comment draft input, Notes rows, and Navigation panel titles.
4. Copy: Apply/Undo notices run the service reason straight into the hint. After Undo, comments read "Passage not found". The backup path appears twice. Changes times show UTC.
5. Minor: the Settings focus ring stays on General; Continue and Send are both filled; the Note text box is about 110 pt in the busiest 900×600 state.

**Not observed:**

- live streaming, Stop/Steer and U1 Stop… on an orphaned run (these need a running worker)
- U3 rows with Protected, and the stale-edit, newer-version, import and recoverable-edit banners
- attachments, Compare, Search, Trash restore, export and file pickers
- hover tooltips, drag-resize, 1050×700, Solo, the Läänemeri and Paber themes, VoiceOver, IME, contrast measurement, and keyboard-only use of the ⋯ menus

## Fix pass (native defects, integration findings, U7, U8, U10)

Implemented and automated-verified only; nothing in this pass was observed natively. Service strings, the shared seams, `useful_intake`, `resume_chat_queue`, `docs/status.md`, `Cargo.lock` and the manifests are unchanged.

| Item | Fix | Tests |
|---|---|---|
| 1 Focus after Escape on a menu-launched dialog | Every BRN dialog opens through `ThreadsUi::open_dialog`. When the click came from a menu (focus in a `PopupMenu`) or nothing is focused, focus first moves to the focused panel's landing from the focus table (editor, composer, search, filter or panel body), so Kit gives it back on close. Covers Projects ⋯ (all four dialogs), Note ⋯ Edit note… and Discard edit…, Chats ＋ New note and the row menus' Move to project…. | `escape_from_a_projects_menu_dialog_keeps_the_keyboard_working` (failed before the fix), `escape_from_a_menu_launched_dialog_returns_focus_to_the_panel`: focus in the panel, ⌘K opens the launcher, Tab moves within the workspace |
| 2 Queue strip "1 waiting · Pa…" | The state leads and never truncates ("Paused · 2 waiting", "Queue paused · 2 waiting"); the count truncates first; full text is the tooltip and accessible name. plan.md's Short rule updated. | `a_short_window_collapses_the_paused_queue_to_one_row` (narrow Write companion, `queue-summary-state` whole before Resume queue) |
| 3a Delegation card | Explanations wrap inside the card (beside the button when there is room); "notes?" is one unit, so "?" never wraps alone. | `the_delegation_card_keeps_its_text_inside_a_narrow_chat` |
| 3b Stacked draft input cut off | The draft sits above the cards' scroller, and while it is open the section's and the text area's floors include the whole draft. | `stacked_comments_keep_every_comment_capability` now asserts `comment-input-box` whole inside the section |
| 3c Notes rows "Protect…" | Protected notes show the Lock chip (as in Chats), announced "Protected". | `a_narrow_notes_list_marks_protected_notes_with_the_lock` |
| 3d Navigation titles "…" | Below 220 pt a panel header leaves Maximize to its ⋯ Panel section (also double-click and ⌘⇧↵). | `a_narrow_list_header_keeps_its_title_and_its_menu_opens_beside` |
| 4a, U8 Receipt reasons in notices | `applied_message`: "Saved." or "Undone.", " Running work was stopped." only when the receipt moved a Working run, then the refresh hint, each its own sentence. The recorded reason stays in Changes. | `saved_notices_name_changes_and_the_refresh_follow_up` (unit), `delegation_and_maintenance_notices_never_show_a_receipt_reason` |
| 4b "Passage not found" after Undo | `unlocated_reason`: "Tied to v1, not re-anchored to v3." when the comment is tied to an earlier version; "Passage not found in vN." only when the service unresolved it. `exact_writing_range` unchanged. | `unlocated_comments_say_what_is_true` (unit), `review_in_place_applies_exact_candidate_then_marks_addressed_and_undoes` |
| 4c Backup path twice | With Settings → Data shown, the notice reads "Full backup created." (or "Markdown snapshot created."); the status line keeps the path. Elsewhere the notice keeps the path. | `first_picker_saves_then_backs_up_and_saved_click_reloads_folder_and_uses_fresh_name` |
| 4d UTC times | `human_time` shows the Mac's local time (C `localtime_r` per instant, no new dependency; a small documented `unsafe` call); the tooltip keeps the UTC ISO value. The All chats Today / This week grouping is unchanged (UTC days). | `human_times_read_as_today_or_a_short_local_date` (UTC, UTC+3, day rollover, DST) |
| 5a Settings ring on General | A pointer click on a section moves focus to the Settings body; a key press keeps it on the pressed section. | `a_clicked_settings_section_takes_the_focus_ring_off_general` |
| 5b Continue and Send both filled | Beside an available recovery card, Send is outline (as in an archived chat). | `a_failed_reply_with_a_paused_queue_names_what_stays_paused` asserts `recovery_card_leads()` |
| 5c Busiest Note text box | The expanded saved rewrite inputs moved out of the writing bar into their own row that gives way (to 40 pt) before the text; the bar no longer overflows the Note. `note-body` keeps 120 px. | `the_busiest_note_keeps_its_text_and_whole_bar_at_900x600` (the bar ran 160 pt out of the Note before) |
| INT-1 Kept-edit primary | Recover edit is the filled primary; a new `WritingRow::Recover` makes Rewrite with AI and Retry ghosts "unavailable: Recover the kept edit first". | `the_four_note_banners_stay_visible_in_write_at_900x600` |
| INT-2 Undo during an open edit | Base `771116f0` rendered no Undo while dirty (its dirty branch was exclusive); this branch had added it. Restored: `undo-writing-apply` is not rendered while the note has an open edit; Changes keeps Undo. No version check changed. | `bound_terminal_clarification_stays_in_note_and_unrelated_chat_cannot_replace_it` (hidden while dirty, back after Discard) |
| INT-3 Edit note and Connections | The fold uses this note's measured row (labels, chip, count) instead of the widest row. In the test text system (0.6 em per character, wider than macOS fonts) a plain note needs about 706 pt for that row and the Note at 1250×860 has 687, so there they still fold in tests; with system fonts the row is narrower. Not observed natively. | `edit_note_and_connections_stay_in_the_toolbar_when_this_notes_row_fits` (1330×860: plain inline, widest folded), `note_toolbar_folds_in_order_as_the_note_narrows` |
| INT-4 Stale actions at the floor | The first banner leads the stack whole with its actions pinned; only its text scrolls in a short Note. Later banners and the saved version share a box that gives way first, down to its first line. Banners outrank the 120 px beside open comments, so with a banner shown the text keeps its 48 px floor. | `the_banner_stack_keeps_its_budget_above_the_text_at_900x600` asserts `save-buffer-copy` and `show-current-note` in view (its text assertion is now ≥ 48 px, by the banner rule) |
| INT-5 Untested states | Disabled reasons (INT-1, INT-2, INT-6 tests), empty states, Link document by keyboard, the running short-window queue row, the header ⋯ Open beside submenu. | `first_run_empty_states_name_the_next_step`, `link_document_rows_are_reached_by_tab_and_pressed_by_space`, `a_short_window_keeps_a_running_queue_to_one_quiet_row`, `a_narrow_list_header_keeps_its_title_and_its_menu_opens_beside` |
| INT-6 Disabled Show passage | Announced "Show passage “quote”, unavailable: {why}", tooltip "Show passage is unavailable: {why}". | `review_in_place_applies_exact_candidate_then_marks_addressed_and_undoes` |
| U7 Needs-you rows | Already correct: the Chats panel and Home use two-line rows with the title first, and the title-bar menu leads with the title. Confirmed, no code change. | `needs_you_rows_keep_the_chat_title_above_the_reason_in_every_list` |
| U10 One failure wording | When a run ends Failed or Interrupted (and the owner didn't stop it), the notice is the run's recorded terminal text (the recovery card's source) in that state's tone, not the raw error. | `a_failed_or_interrupted_finish_shows_the_recorded_terminal_text` (seeded text) |

**Gate on the fix-pass tree** (`PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=<worktree>/target`): `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` passed, helper and desktop build, clippy `-D warnings`, **263 passed**, 0 failed; `cargo fmt --all -- --check` clean; `python3 scripts/check-markdown-links.py` 116 files, 432 links, 0 failures; `git diff --check` clean. Also run: `cargo test -p brn-desktop --features native-ui` **76 passed**, default features **28 passed**, clippy `-D warnings` for `brn-desktop` with default features and with `native-ui` clean. The `native-retrieval` lane and the workspace suite were not rerun; nothing outside `crates/brn-desktop` changed.

## Owner-accepted

No. The owner has not reviewed the implemented UI.

## Merged

No. Nothing is pushed, merged, published or released.

## Open items

- **Native re-observation of the fix pass**: native defects 1–5, INT-1 to INT-6, U7, U8 and U10 are fixed and automated-verified (see Fix pass), not re-observed. INT-3 in particular depends on macOS font widths: whether a plain note keeps Edit note and Connections at 1250×860 needs a native look. INT-7 (the link check) was resolved by the record commit.
- **Fix-pass trade-offs to confirm:** with a Note banner shown, the text beside open comments keeps 48 px instead of 120 px (banners are never hidden to meet the budget); the All chats day grouping stays in UTC while times are local.
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
