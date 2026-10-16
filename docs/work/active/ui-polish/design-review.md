# UI polish: design review, round 5

**Reviewed:** [`plan.md`](plan.md), revised after round 4. It is untracked on `work/ui-polish`, baseline `4b41f190`. This file replaces the round 4 review.

The step brief names the round 3 findings, but `design-review.json` held round 4's (H1 comments at narrow widths, M1 theme in the status bar, N1–N4), and the plan answers both rounds. I checked both: round 4's fixes and that round 3's still hold after this revision.

**Method:**
- I read the plan body against the `brn-desktop` source in this worktree.
- I read it against the pinned Kit sources in the Cargo registry (`gpui-base` 0.7.3) and against the contract documents it corrects.
- I ran no builds, tests or lint, as instructed.

**Verdict: CHANGES_REQUESTED.** 0 HIGH, 1 MEDIUM, 4 NIT.

Every round 4 finding holds in the body, and so does every round 3 fix. I found no risk to protection, exact-candidate review, version checks, replay, recovery or Undo, and nothing that needs more than Kit 0.7.3.

The one blocking finding is new and comes from round 4's Stacked fix. At 900×600 and 1050×700, a review gives the owner no way to see the note's comments. The only toggle lives in the toolbar, and the review bar replaces the toolbar. Today the comments stay visible during review, and a test asserts it.

## Round 4 fixes: what holds

| Finding | Holds? | Evidence |
|---|---|---|
| H1 Comments at narrow widths | Yes, except during review (M1 below) | See the H1 notes below this table. |
| M1 Theme in the status bar | Yes | See the M1 notes below this table. |
| N1 Footer clamp | Yes | Long content, the review bar section and Preserve all say the footer is never clamped or truncated. Two lines is the fit target at 900×600, and the binding sentence comes first. |
| N2 Toolbar fit | Yes | The thresholds are about 760, 620 and 480 pt, set in step 3 from measured widths. The one-row tests run at 900×600, 1050×700 and 1250×860, dirty or not, with Chats and with Notes in the navigation area. |
| N3 Mixed card tone | Yes | The card takes the strongest tone among its lines. Under "Not answered yet", the Working line reads "An earlier reply may still be running elsewhere…", and this is tested. The receipt-order loop is factored into a helper. |
| N4 Duplicate `save-buffer-copy` | Yes | Row 1 keeps the unsaved label and Discard…, shows Save as a ghost, and omits `save-buffer-copy`. The test asserts that it renders exactly once. |

**H1 notes:**
- **Modes.** `margin_mode` returns Wide (280), Narrow (220) or Stacked, and Rail is gone from the body. Only the response tables mention it, as history.
- **Worked numbers.**
  - The mode minimums add up correctly: 170 + 640 + 300 + 24 = 1134, 170 + 580 + 300 + 24 = 1074, and 170 + 360 + 300 + 24 = 854.
  - Resolving the Write weights (0.62, 2.7 and 1.45, with minimums 170, 360 and 300) gives Note widths of about 406 pt at 900, 556 pt at 1050, 589 pt at 1100 and 687 pt at 1250. That matches Stacked, Stacked, Narrow and Wide.
  - Solo gets the full width, so it is Wide.
  - `TileKind::min_width` today is Threads 170, Note and Compare 320, and everything else 260 (`threads_layout.rs` 79–85). The 250 px margin is added in `threads_shell.rs` 388–390, as the plan says.
- **Stacked section.** `comments-section` sits between the text and the writing bar. The 40 % cap works out to about 138 of 346 px. The section reuses the existing `margin_column` renderers and IDs (`threads_writing.rs` 1219ff), and only one mode renders at a time, so the IDs stay unique.
- **Opening it.** ⌘/ already sets `margin = true` and focuses the comment input (`threads_ui.rs` 1355–1359), so the "expand with a focused draft" route exists.
- **Escape.** Escape inside the section is feasible. Kit's `Input::escape` ends in `cx.propagate()` once it has nothing of its own to dismiss (`gpui-base` `input/base/state.rs` 2169–2206), so a handler on the section receives it.
- **Lists and contract.** The scroller list, Escape order, Out of scope, Preserve, tests, Gates and the contract correction are all updated. The target lines are 384 and 386, and the writing polish spec line is 7, as the plan now cites.

**M1 notes:**
- `status-layout` holds the presets, the panel toggles and Last work status, with no Theme section.
- The plan states that the `status-theme` pill (`threads_shell.rs` 918) is removed.
- The Settings hint at `threads_tiles.rs` 1109 becomes "Also in ⌘K."
- Keys help, Done when and Preserve are updated, and the Omakase spec lines 12 and 17 are annotated.
- The theme journey (`threads_omakase_tests.rs` 273ff) keeps the ⌘K switch, the Settings switch (`settings-theme-*`), the running-work deferral and the persistence-on-reopen assertions. It drops the `status-theme` steps.
- The tiled ID list at 491 drops `status-theme`.
- `choose_theme` and `persist_theme` (`threads_ui.rs` 2180–2215) are unchanged, so the deferral still holds.

## Round 3 fixes: still hold

Round 4 already confirmed these, and the round 4 revision left them alone, except for N9's numbers.

| Round 3 finding | Still holds? | Evidence |
|---|---|---|
| H1 Rewrite retry in chat | Yes | The card gate is still "`can_continue_thread` and `continue_path` is not `RetryRewrite`". The no-card, visible-`retry-rewrite` test is still in place. |
| M1 ⌘↵ in multi-line dialogs | Yes | `capture_action::<gpui_kit::component::input::Enter>` (secondary, not shift) is still on each multi-line dialog root, with `on_action::<Ask>` for buttons. |
| M2 Writing-bar primary | Yes | The seven-row first-match order and the Stop line are still independent of the bar. |
| M3 Line 1 with a newer owner message | Yes | Line 1 still uses the run state only when no owner message postdates the run, and reads "Not answered yet" otherwise. |
| M4 Review bar | Yes | The bar still collapses at 620 and 480 pt, the footer is never truncated, and Keep for later and Dismiss are in the menu-only table. |
| M5 Status chips | Yes | `note-status-chip` shows Protected or Working with vN in every state. `note-unsaved-chip` is in the writing bar. |
| M6 Writing-run outcomes | Yes | Rows 5 and 6 still carry their tones and recorded reasons, and `writing-run-status` and `writing-result` are placed. |
| N1–N10 | Yes | N9 is rewritten for Stacked: the scrollbar is expected only from 850 to 853 pt. |

## Findings

### MEDIUM

**M1. In Stacked mode the owner cannot see comments during review.**

*Where:*
- Layout rules → Comments → Stacked ("in review, between the review text and the footer"; "`toggle-margin`… is the only expand/collapse control. The section starts collapsed")
- step 3 → Comments ("`comments-section` placed in both Write and review")
- the review bar state table
- Tests → Comments and Review

*Problem:*
- `toggle-margin` lives in the Note toolbar, and during review the review bar replaces the toolbar. Neither the bar's state table nor the Note ⋯ review section (Compare side by side, Edit final text, Inputs, Keep for later, Dismiss) has a comments control.
- The Stacked section starts collapsed and Escape collapses it, so at 900×600 and 1050×700 a review can begin collapsed or be collapsed, and nothing reopens it until the owner leaves the review.
- Inputs doesn't fill the gap. It shows only counts ("Inputs: saved vN · K comments · M chat messages"; `threads_writing.rs` 1880–1913), not the comments themselves.
- Today the margin renders during review whenever `self.margin` is set (it defaults to `true`, `threads_ui.rs` 754). The review document appends `margin_column` (`threads_writing.rs` 2042–2049), and `threads_omakase_tests.rs` 827 asserts `writing-feedback` is visible in review. The plan's small-window layout therefore drops a tested, accepted behavior: the owner reading the candidate against the comments it was meant to address.
- That contradicts the owner decision in Open question 2 ("Every comment capability stays intact") and the writing polish contract ("Feedback is reachable beside both views").
- It also leaves the footer's "K comments stay open until you mark them addressed" pointing at comments the owner cannot open.

*Fix:*
- **Review bar.** `toggle-margin` (Comment icon plus count, same ID, same handler and state) renders in the review bar in every margin mode. It is the bar's last control before the primary. Only one of the toolbar and the review bar renders at a time, so the ID stays unique.
- **Fit below 480 pt.** The Proposal label reads "Proposal", and the shape ("whole note", "N notes", "N changes") moves into its tooltip and accessible name, next to the +a −r counts that already move there below 620 pt.
  - At 900×600 that gives roughly 60 for the label, 130 for Changes | Result, 24 for the Lock, 44 for the comment toggle, 84 for Approve and 32 for gaps: about 374 of the ~382 usable pt.
  - Step 3 confirms this from measured widths, as for the toolbar.
- **Layout rules → Stacked** add: "During review, the review bar's `toggle-margin` expands and collapses the section, which sits between the review text and the footer."
- **Small-window rules → Review bar** add the "Proposal" label rule under "Below 480 pt".
- **Tests (Review):** at 900×600 in Write, on a protected note with two comments:
  - `toggle-margin` is visible in the one-row review bar
  - clicking it expands `comments-section` between the review text and the footer, and both comment cards render
  - the footer stays fully visible with the section expanded
  - Escape inside the section collapses it, and `toggle-margin` reopens it
- **Tests (Comments):** at 1250×860 in review, the cards render beside the text, as the existing assertion at `threads_omakase_tests.rs` 827 expects.

### NIT

**N1. The writing polish spec has a second "beside" line that the contract correction doesn't cover.** Line 15 of the [writing polish spec](../writing-polish/spec.md) says "Feedback is reachable beside both views and has a clear pending count, quote, inclusion and passage reveal." In Stacked mode feedback is below the text, not beside it. AGENTS.md asks for affected summaries to be updated along with the correction.

*Fix:* add to step 3's contract correction and to the Gates line: "writing polish spec line 15: 'Feedback is reachable beside both views (in a narrow Note tile, in the in-tile comments section below the text)…'". Line 3, the owner quote, and `docs/status.md` line 11 are historical records and stay as they are.

**N2. The plan doesn't say where focus goes when Escape collapses the section.** Collapsing the section removes the focused comment input or card control from the render, so keyboard focus is left on nothing visible.

*Fix:* "When Escape collapses `comments-section`, focus moves to the visible editor (`focus_visible_editor`, as `cancel-comment` does today), or to the review text during review." Add that assertion to the "Escape inside the section" test.

**N3. The vertical order and height share in a Stacked Note tile are unspecified when run status, a reply or proposals also show.** Today's order below the text is `writing_activity`, `writing_proposals`, `writing_result`, overall feedback, then the bar (`threads_writing.rs` 1210–1216). At 900×600 an expanded section (about 138 px), plus `writing-run-status`, plus `writing-result` (120 px), leaves the text about 64 px. The plan doesn't say where `comments-section` sits among these, or what wins when they don't fit.

*Fix:*
- Order below the text: `comments-section`, then `writing-run-status`, then `writing-proposals` or `writing-result`, then the writing bar.
- The text keeps a minimum of 120 px.
- The section's cap is 40 % of the height left after the run line, the reply or proposals, and the bar. The section shrinks before the text does.
- Add a 900×600 test with a failed run, its partial reply and an expanded section, asserting that `note-body` is at least 120 px tall.

**N4. The theme journey test keeps its old name and loses its only plain saved-setting check.** `themes_switch_from_status_bar_launcher_and_settings_and_wait_for_running_work` (`threads_omakase_tests.rs` 274) names the status bar. Its only check that a theme is saved right away, with no deferral, follows the `status-theme` step being removed (lines 282–290).

*Fix:* rename the test to `themes_switch_from_settings_and_launcher_and_wait_for_running_work`. After the Settings switch to Virmalised, assert that `!theme_pending` holds and that `settings().theme == Some("virmalised")`.

## Verified assumptions

- **Widths.** `TileKind::min_width` today is Threads 170, Note and Compare 320, and others 260 (`threads_layout.rs` 79–85). The 250 px margin is added to the Note minimum when the margin is on (`threads_shell.rs` 388–390). The Wide, Narrow and Stacked sums, and the weight-resolved Note widths at 900, 1050, 1100 and 1250 pt, match the plan.
- **Margin state.** `margin: bool` defaults to `true` (`threads_ui.rs` 754). `toggle-margin` flips it (`threads_writing.rs` 723–737). ⌘/ sets it and focuses the comment input (`threads_ui.rs` 1355–1359).
- **Today's review.** `note_controls` returns only the "reviewing" chip, so there is no `toggle-margin` (`threads_writing.rs` 667–685). The review document appends `margin_column` when `self.margin` is set (2042–2049), and `threads_omakase_tests.rs` 827 asserts `writing-feedback` is visible in review. Review Inputs shows only comment counts (1880–1913).
- **Kit Escape.** Kit `Input::escape` propagates when it has nothing to dismiss (`gpui-base` 0.7.3 `input/base/state.rs` 2169–2206).
- **Theme routes.** The `status-theme` pill is at `threads_shell.rs` 918, and the Settings hint is at `threads_tiles.rs` 1109. `choose_theme` and `persist_theme`, with the running-work deferral, are at `threads_ui.rs` 2180–2215. The theme journey starts at `threads_omakase_tests.rs` 273, and the tiled ID list includes `status-theme` at `threads_tiled_tests.rs` 491.
- **Contract lines.** In the writing polish spec, line 7 reads "…Comment cards never move into chat or below the note." and line 15 reads "Feedback is reachable beside both views…". In the target, line 384 mentions "margin cards" and line 386 "passage comments beside the document". In the Omakase spec, line 12 has the "theme menu" status-bar item and line 17 "Selectable from the status bar, launcher and Settings › General".
- **Narrow fixture.** The existing narrow fixture (`threads_writing_tests.rs` 546, 760×820) relies on horizontal tile scrolling. That stays valid, since 760 is below Stacked's 854.

## Unverified or wrong assumptions

- **Wrong:** "`comments-section` placed in both Write and review" works in review. Its only toggle isn't rendered there (M1).
- **Incomplete:**
  - the contract correction misses writing polish spec line 15 (N1)
  - the focus target after Escape collapses the section (N2)
  - the Stacked vertical order and minimum text height (N3)
  - the theme test's name and its immediate-save assertion (N4)
- **Unverified (no runs, by instruction):**
  - real label widths for the toolbar and review bar thresholds, including the "Proposal" fit below 480 pt
  - taffy behavior at the 620 and 480 pt boundaries
  - Kit popover focus on Tab and ↵ (the plan's step 1 test covers it)
  - tooltips on disabled Kit buttons, natively
