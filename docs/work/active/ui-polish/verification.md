# UI polish: verification

**Date:** 16 October 2026 (times are EEST, UTC+3). **Branch:** `work/ui-polish`, worktree `.worktrees/ui-polish`, base `771116f0`, synced with `origin/main` `8bef439b`. The first native observation ran on HEAD `ed8daa82`. The [recheck of the fix pass](#recheck-of-the-fix-pass-head-354c1940) and the latest gate ran on HEAD `354c1940`. Nothing here is owner-accepted, published, merged or released. This file and [`after/`](after/) are committed with [evidence.md](evidence.md).

## Implemented

The seven slices and the integration fixes in [impl-plan.md](impl-plan.md) are on the branch:

- **S1:** tokens, tones and shared components (icon buttons, segmented controls, banners, empty states, consequence dialogs).
- **S2:** shell chrome. `status-theme` is removed, and theme is in Settings → General and ⌘K only.
- **S3:** the Note writing toolbar, banners and writing bar.
- **S4:** the one-row review bar, the exact-text footer, and comment placement: Wide, Narrow, and a Stacked `comments-section`.
- **S5:** chat recovery, queue and composer, plus U1 Stop…, U3 review targets and U6 full failure reasons.
- **S6:** lists, pages and Settings.
- **S7:** dialogs, focus, small windows and the state and copy sweep.
- **Integration:** a fix commit (`ed8daa82`).
- **Fix pass:** `3fdf5489` and its review round 2 (`354c1940`) fix the five native defects below, INT-1 to INT-6, U8 and U10. Detail is in [evidence.md](evidence.md#fix-pass-native-defects-integration-findings-u7-u8-u10).

All seven slice reviews and the integration review (`code-review.md`, an uncommitted workflow file) are APPROVED. `git diff 771116f0 -- crates ':!crates/brn-desktop'`, `Cargo.lock` and the crate manifests are all empty.

## Automated-verified

### Full gate on HEAD `354c1940`

Rerun by the verify step from 10:29:38 to 10:30:14, before any window opened. It used the full gate from [impl-plan.md](impl-plan.md#commands), `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=<worktree>/target` and Rust 1.98.1. Every command exited 0. The only warning was the existing `block 0.1.6` future-incompatibility notice.

| Command | Result |
|---|---|
| `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` | helper and desktop build, clippy `-D warnings`, **263 passed**, 0 failed |
| `cargo fmt --all -- --check` | clean |
| `python3 scripts/check-markdown-links.py` | 117 files, 438 local links, 0 failures before these doc edits; 117 files, 472 local links, 0 failures on this commit |
| `git diff --check` | clean |
| `cargo clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui --locked --offline` | **76 passed** |
| `cargo clippy … --features native-ui,native-retrieval --all-targets … -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline` | **76 passed** |
| `cargo test -p brn-desktop --locked --offline` (default features) | **28 passed** |
| `cargo clippy -p brn-desktop --all-targets --locked --offline -- -D warnings` (default features) | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |

`git diff --stat 771116f0 -- crates ':!crates/brn-desktop'` is empty, so the workspace test suite was not rerun. `Cargo.lock` and the crate manifests are unchanged from `771116f0`. The shared seam files and `docs/status.md` are unchanged from `8bef439b`. `origin/main` has since moved to `027aa7c2`; this branch is not synced with it.

### Earlier gate on HEAD `ed8daa82`

Run with `PATH=/opt/homebrew/opt/rustup/bin:$PATH` and `CARGO_TARGET_DIR=<worktree>/target`, Rust 1.98.1. All commands exited 0. The only warning was the existing `block 0.1.6` future-incompatibility notice.

| Command | Result |
|---|---|
| `RUST_TEST_THREADS=1 scripts/verify-desktop-shell.sh` | helper and desktop build, clippy `-D warnings`, **245 passed**, 0 failed |
| `cargo fmt --all -- --check` | clean |
| `python3 scripts/check-markdown-links.py` | 89 files, 376 local links, 0 failures (before this file was added) |
| `git diff --check`, and `git diff --check 771116f0 HEAD` | clean |
| `cargo clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings` | clean |
| `cargo test -p brn-desktop --features native-ui --locked --offline` | **75 passed** |
| `cargo build` and `cargo clippy … -D warnings`, `-p brn-desktop --features native-ui,native-retrieval` | clean |
| `cargo test -p brn-desktop --features native-ui,native-retrieval --locked --offline` | **75 passed** |
| `cargo test -p brn-desktop --locked --offline` (default features) | **27 passed** |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |

I didn't rerun the full workspace test suite, because nothing outside `crates/brn-desktop` changed. Keyboard, focus, Escape order, dialog ↵ classes, layout fits and theme deferral are covered by the 245 native tests. The real-input checks below are additional to those tests.

## Native-observed

### Recheck of the fix pass (HEAD `354c1940`)

A short check with real input of the items the fix pass changed, at 900×600 in light (Sammal) and dark (Virmalised), plus 1250×860 for INT-3. No fixes were made in this step.

**How:**

- **Build.** `cargo build -p brn-desktop --features native-ui --locked --offline` at `354c1940` produced the binary, sha256 `3a922bcc…28b0be`. `scripts/make-macos-app.sh` wrapped it in two scratch bundles under `/private/tmp/brn-ui-polish-recheck-20261016/apps/`, with bundle IDs `dev.brn.threads.uipolish-recheck.{seeded,empty}`.
- **Data.** [`seed-fixture.rs.txt`](observation/seed-fixture.rs.txt) was built as a temporary example in a `git archive` export outside Git. It ran through the shared `Workspace` service against fresh `seeded` and `empty` folders, and headless startup reported 71 and 2 records.
  - The provider is `offline-fixture` with no credentials. No provider request was made, and nothing that starts a run was pressed: no Continue, Retry, Resume queue or Send.
  - The only writes were synthetic and went through the checked service: one Apply and its Undo, one discarded edit, one full backup to the scratch folder, theme changes, and one unsaved edit left in the scratch data for the restart check.
- **Input and capture.** `AXIsProcessTrusted()` returned `true`. [`input.swift`](observation/input.swift) posted the CGEvents, with the scratch-only keycode typer from the first run. That typer left Shift latched once, so one draft reads in capitals ([06](after/06-comment-draft-stacked-900-light.png)); this is a harness artifact. [`wincap.swift`](observation/wincap.swift) captured the windows. The observation harness was not built or used, and nothing was compiled into `crates/` or `scripts/`.

**Go-ahead and timing.**

- Before any window opened, this step sent the parent a warning asking for a go-ahead. The orchestrator first replied "no go-ahead yet". The go-ahead acted on was the later reply "go" in this step's session.
- Before launching anything, `.worktrees/coordination/MAC-IN-USE` did not exist. This step created it at 10:43 and deleted it at 10:59:21, then added a line to `BOARD.md` under "work/ui-polish (lead)".
- Window use ran from **10:43:23** (seeded bundle launched) to **10:59:09** (empty bundle quit with ⌘Q):
  - The seeded bundle quit with ⌘Q at 10:56:02 for the restart check, relaunched at 10:56:21, and quit at 10:57:25.
  - The empty bundle launched at 10:57:31.
- After quitting, `pgrep` found no `brn-desktop` process and ScreenCaptureKit listed no BRN window.

**Results.** "Rechecked" means seen with real input on this build; the test names are in [evidence.md](evidence.md#fix-pass-native-defects-integration-findings-u7-u8-u10).

| Item | Native result at `354c1940` | Status |
|---|---|---|
| 1 Focus after Escape on a menu-launched dialog | Projects ⋯ → Move to Trash… → Escape, then ⌘K opened the launcher. Projects ⋯ → Instructions & settings → Escape, then Tab moved the focus ring to the Trash segment inside Projects, and ⌘K opened the launcher. [47](after/47-cmdk-after-menu-dialog-escape-900-light.png) | **fixed, rechecked** for the two Projects ⋯ dialogs. The Note ⋯, Chats ＋ and row-menu dialogs were not rechecked. |
| 2 Queue strip "1 waiting · Pa…" | In the narrow Write companion the strip reads "Paused · 1 wa…": the state is whole and the count gives way. Its tooltip is "Paused · 1 waiting". Chat layout shows "Paused · 1 waiting", and 1250×860 shows "Queue paused · 1 waiting". [01](after/01-shell-900-light.png), [48](after/48-queue-strip-tooltip-900-light.png) | **fixed, rechecked** |
| 3a Delegation card | The explanations wrap inside the card, and "notes?" stays one unit, in light and Virmalised. [43](after/43-delegation-card-900-light.png) | **fixed, rechecked** |
| 3b Stacked draft input | `comment-input-box` is whole inside the section, and Escape collapsed it with the caret back in the editor. [06](after/06-comment-draft-stacked-900-light.png) | **fixed, rechecked**. A new clip below the section is listed under "New findings". |
| 3c Notes rows "Protect…" | The protected decision shows the Lock chip, in light and Virmalised. [05](after/05-note-comments-stacked-900-light.png) | **fixed, rechecked** |
| 3d Navigation titles "…" | "Chats" and "Notes" read in full in the 170 pt navigation panel, and Maximize is in the header ⋯. [01](after/01-shell-900-light.png) | **fixed, rechecked** |
| 4a, U8 Receipt reasons in notices | Apply: "Saved. 8 related items may need refresh. Open Changes to review them." Undo: "Undone. 8 related items may need refresh. …". Neither shows the service reason. [44](after/44-applied-notice-900-light.png) | **fixed, rechecked** |
| 4b "Passage not found" after Undo | After Apply (v2) and Undo (v3), all three cards read "Tied to v1, not re-anchored to v3. Exclude it, or select the passage and comment again." [45](after/45-note-wide-toolbar-1250-dark-virmalised.png) | **fixed, rechecked** |
| 4c Backup path twice | In Settings → Data the banner reads "Full backup created.", and the status line keeps the path once. | **fixed, rechecked** |
| 4d UTC times | Changes shows "Today 10:47" and "Today 10:46", the local times of the Undo and the Apply. The UTC tooltip was not hovered. [11](after/11-changes-900-light.png) | **fixed, rechecked** |
| 5a Settings ring on General | After a click on Models (light) or on General (Virmalised), no focus ring stays on General. [16](after/16-settings-models-900-light.png) | **fixed, rechecked** |
| 5b Continue and Send both filled | Beside the Failed card and the "Not answered yet" card, Continue is filled and Send is outline. Checked in the Write companion and Chat layout, in light and Virmalised. [29](after/29-chat-failed-900-dark-virmalised.png) | **fixed, rechecked** |
| 5c Busiest Note text box | The busiest state (stale edit, failed note run, partial reply, dirty bar) can't be built from this fixture. | not rechecked |
| INT-2 Undo during an open edit | After Apply, typing hid `undo-writing-apply` (Discard…, Save as new note, Save ⌘S). After the Discard dialog's danger Discard, Undo was back. | **fixed, rechecked** |
| INT-3 Edit note and Connections at 1250×860 | With system fonts, a plain note (Vendor comparison) keeps all five format buttons, Outline, the status chip, the comment toggle, Edit note and Connections in one toolbar row. The Pilot proposal (3 comments, "Working · v3") also keeps Edit note and Connections, with formatting folded into Format ▾. So the test text system's 706 pt estimate is conservative on macOS. [45](after/45-note-wide-toolbar-1250-dark-virmalised.png), [46](after/46-plain-note-toolbar-1250-dark-virmalised.png) | **fixed, rechecked** |
| INT-6 Disabled Show passage | Hovering a card showed "Show passage is unavailable: the comment is tied to v1, not v3." | rechecked (tooltip only) |
| U7 Needs-you rows | "Confirm the pilot st…" sits above its reason. The recoverable-edit row is listed under "New findings". | partly rechecked |
| INT-1 Kept-edit primary | After ⌘Q with an unsaved edit and a relaunch, BRN reopened it as an open recoverable edit ("Unsaved edit · recoverable", with Save ⌘S as the primary), not the blocked kept-edit state. | not rechecked |
| INT-4, FIX-2 Banner stack and text floor | The fixture has no stale-edit or newer-version banner. The text-floor rule stays **pending owner acceptance**. | not rechecked |
| U10 Recorded terminal notice | This needs a run that finishes Failed or Interrupted, which needs a provider. | not rechecked |
| INT-5 Untested states | The empty states were seen again (no chats, no note open, no comments yet), along with the delegation card. Link document by keyboard and the running short-window queue were not checked. | partly rechecked |

**New findings at `354c1940`** (native, not fixed here):

1. **The writing bar clips while a stacked comment draft is open at 900×600.**
   - The bar's last row runs out of the Note's bottom edge: "Rewrite with AI ⌘⇧R" at v1.
   - After Undo, with the notice row shown, the filled primary "Retry saved rewrite" is about half visible.
   - With the section open and no draft, the bar fits.
   - Shot: [06](after/06-comment-draft-stacked-900-light.png).
2. **Edit note's disabled reason contradicts the status bar.** While a waiting message sits in the paused queue, Edit note is disabled with "Edit note is unavailable: BRN is working", but the status bar reads "Idle" and nothing runs.
   - The gating (`context_work_idle` requires an empty queue) is unchanged from `771116f0`. The reason wording is new on this branch.
   - Shot: [46](after/46-plain-note-toolbar-1250-dark-virmalised.png).
3. **Changes' "May need refresh" sub-row names records through `title()`.**
   - A writing-request record shows raw JSON (`{"version":1,"snapshot":{"request_message":…`), which runs past the panel edge.
   - Comment and link records show "passage comment" and "discusses".
   - The naming is unchanged from base (which labelled them "May need refresh: …"); the pill layout is new.
   - Shot: [11](after/11-changes-900-light.png).
4. **The recoverable-edit needs-you row leads with the action.** At 170 pt it reads "Recov… [Unsaved]", so the note's name shows only in the tooltip ("Recover edit · Vendor comparison · Unsaved").

**Shots replaced in [`after/`](after/)** by captures from this build: 01, 04, 05, 06, 08, 11, 14 and 16 (same states), 29 (now Virmalised), and 43 (the delegation card, now fixed). Added: 44 to 48. The rest (10, 22, 25, 27, 34, 37, 41, 42) are still from `ed8daa82`, and some of them show the old queue strip and navigation titles.

### First observation (HEAD `ed8daa82`)

**How:** real input on the unmodified product build, not the harness.

- **Build.** `cargo build -p brn-desktop --features native-ui --locked --offline` at HEAD produced a binary with sha256 `e64c8463…adb9`. It was wrapped by `scripts/make-macos-app.sh` in two scratch bundles under `/private/tmp/brn-ui-polish-verify-20261016/apps/`. The bundle IDs were `dev.brn.threads.uipolish-verify.{seeded,empty}`, and launch logs went to the scratch folder.
- **Input.** `AXIsProcessTrusted()` returned `true`. Keyboard and mouse events were posted as CGEvents by [`input.swift`](observation/input.swift), with a scratch-only keycode typer added, because Unicode-string events were not delivered to text fields.
- **Capture.** Windows were captured with [`wincap.swift`](observation/wincap.swift).
- **Harness.** The observation harness was compiled in a scratch export outside Git as a fallback and never launched.
- **Data.** [`seed-fixture.rs.txt`](observation/seed-fixture.rs.txt) ran through the shared `Workspace` service against fresh `/private/tmp/brn-ui-polish-verify-20261016/{seeded,empty}` directories. Headless startup reported 71 and 2 records.
  - The provider is `offline-fixture`, with no credentials, and no provider request was made.
  - The only writes were synthetic and went through the checked service: one Apply and its Undo, one discarded edit, and one full backup to the scratch folder.

**Go-ahead and timing.**

- Before any window opened, this step sent the parent a warning asking for a go-ahead, at about 01:03.
- The go-ahead I acted on was the reply "go", the next message in this step's own session. It did not come through the orchestrator, which later reported it never sent one. Its send time isn't visible to me.
- Window use ran from **07:21:05** (seeded bundle launched) to about **07:49:20** (empty bundle quit with ⌘Q). The seeded bundle quit with ⌘Q at about 07:45:30, and the empty bundle launched at 07:46:21.
- Before the go-ahead, only windowless checks ran: the AX-trust query and `--headless-check startup`.
- After quitting, `pgrep` found no BRN process from this step, and ScreenCaptureKit listed no BRN windows.

**Journeys observed at 900×600 (one at 1250×860)**, in light (Sammal) and dark (Virmalised, Raba). Shots are 1100 px wide in [`after/`](after/); [`before/`](before/) was captured mostly at 1250×860 with the same fixture. Shots 01, 04, 05, 06, 08, 11, 14, 16, 29 and 43 were later replaced by recheck captures at `354c1940` (same states, fixed items), so they no longer show the defects listed below.

- **Shell.**
  - Write | Chat | Solo is in the title bar, and the status bar has run state, save state, Layout and Keys, with no theme control.
  - The Layout menu lists Layouts, Panels and Last work status, with no Theme item.
  - ⌘K opened the launcher, typing filtered it to the five themes, and ↵ applied Virmalised. Settings → General applied Raba, and its hint reads "Also in ⌘K."
  - Escape closed the launcher and returned focus to the composer. Escape closed Settings back to the previous page.
  - Shots: [01](after/01-shell-900-light.png); compare [before 02](before/02-home-light.png).
- **Note, Write.**
  - The toolbar is one row. Comments are Stacked behind the count toggle, and Review rewrite is the only filled primary.
  - The toggle expanded `comments-section` between the text and the writing bar, and clicking it again collapsed the section.
  - Double-clicking a word showed the selection strip. ⌘/ opened a focused draft in the section, and the typed text appeared in it.
  - Escape inside the draft collapsed the section and returned the caret to the editor. Reopening kept the draft.
  - At 1250×860 the cards sat beside the text (Wide).
  - Shots: [04](after/04-note-write-pilot-900-light.png), [05](after/05-note-comments-stacked-900-light.png), [06](after/06-comment-draft-stacked-900-light.png), [37](after/37-note-wide-comments-1250-light.png); compare [before 04](before/04-note-write-pilot-light.png).
- **Review.**
  - The bar is one row (Proposal, Changes | Result, comments 3, Apply), and Changes and Result both switch.
  - The footer "Applying saves exactly this text. 3 comments stay open until you mark them addressed." stayed whole in both themes.
  - The Note ⋯ menu holds Compare side by side, Edit final text and Inputs, then Keep for later and Dismiss, then Panel. Escape closed it.
  - Apply moved the note to v2 and showed "Applied · v2" with Undo and Mark 3 comments addressed. Undo restored the original text as v3.
  - Shots: [08](after/08-review-900-light.png), [27](after/27-review-900-dark-virmalised.png); compare [before 08](before/08-review-diff-light.png) and [before 32](before/32-small-write-review-light.png).
- **Dirty edit and Discard.**
  - Typing showed "Unsaved edit · recoverable" with Save ⌘S as the only filled primary, and "Unsaved · ⌘S" in the status bar.
  - Discard… opened "Your changes since v3 will be lost. This can't be undone." ↵ left it open (pixel-identical capture), and the danger Discard confirmed it.
  - The protected decision shows a lock with v2 in the status chip.
  - Shot: [34](after/34-note-dirty-900-dark-raba.png).
- **Chat.**
  - The needs-you card shows the question in the warning tone.
  - The failed run shows a Failed card in the error tone with the full recorded reason, the Continue consequence and "1 waiting message stays paused until you resume the queue."
  - The archived chat shows one Unarchive with Continue disabled, "Unarchive to continue."
  - The composer is one row.
  - Shots: [22](after/22-chat-needs-you-900-light.png), [29](after/29-chat-failed-900-dark-virmalised.png) (replaced at the recheck: Virmalised, Send outline), [25](after/25-archived-chat-900-light.png); compare [before 22](before/22-chat-needs-you-light.png), [before 25](before/25-archived-chat-light.png) and [before 29](before/29-chat-failed-dark-raba.png).
- **Lists and pages.**
  - **Tasks:** one outline action plus a row ⋯ per task; Done and Cancelled are collapsed.
  - **Changes:** Undo ghosts.
  - **Settings:** segmented sections. Data keeps the folder line and status lines visible, and Create full backup showed a success banner and status.
  - **Projects:** Active | Trash, a Projects ⋯ menu, New chat as the primary for the selected project, and "BRN knowledge: on" visible.
  - Shots: [10](after/10-tasks-900-light.png), [11](after/11-changes-900-light.png), [16](after/16-settings-models-900-light.png); compare [before 10](before/10-tasks-light.png), [before 11](before/11-changes-light.png) and [before 16](before/16-settings-models-light.png).
- **Destructive dialog.**
  - Projects ⋯ → Move to Trash… opened the preview with a danger "Move to Trash" and Cancel. ↵ left it open with nothing moved (pixel-identical), and Escape closed it.
  - ⌘N New note opens with a focused title field.
  - Shot: [14](after/14-project-delete-dialog-900-light.png).
- **First run.**
  - Empty states read clearly.
  - Home shows the intro and the pending delegation card with two outline buttons and no filled primary. The card shows in one place at a time.
  - Shots: [41](after/41-empty-first-run-900-light.png), [42](after/42-empty-home-900-dark-virmalised.png); compare [before 41](before/41-empty-first-run-light.png).

**Defects and rough edges seen natively.** No fixes were made in that step. The fix pass fixed all five. The [recheck](#recheck-of-the-fix-pass-head-354c1940) saw these fixed natively: 1 (for the Projects ⋯ dialogs), 2, 3a–3d, 4a–4d, 5a and 5b. It did not recheck 5c.

1. **Keyboard focus is lost after Escape closes a dialog opened from a ⋯ menu item.** This is the most important finding, because it breaks predictable keyboard use.
   - After Projects ⋯ → Move to Trash… → Escape, and again after Projects ⋯ → Instructions & settings → Escape, ⌘K and Tab did nothing until a click in the window.
   - Escape on the menu alone, ⌘N's dialog and Keys help (opened from a button) kept the keyboard working.
   - The Note ⋯ "Discard edit…" path was not checked. These menu entry points are new on this branch; the base was not compared.
2. **"Paused" is truncated** to "1 waiting · Pa…" in the queue strip of the narrow Write-layout chat companion at 900×600. Chat layout shows "1 waiting · Paused" in full. The tooltip was not checked.
3. **Clipping at narrow widths:**
   - The delegation card's "BRN updates ordinary notes and source referen…" runs past the card edge in the chat companion, and its "?" wraps onto its own line. The clipped shot was replaced at the recheck by the fixed card ([43](after/43-delegation-card-900-light.png)).
   - The stacked comment draft's input is cut off at the bottom by the section's height cap.
   - Notes rows show "Protect…".
   - Navigation panel titles collapse to "…" (the icon remains).
4. **Copy and wording:**
   - The Apply and Undo notices run the service reason into the hint: "Saved. Whole-note writing proposal for owner review 8 related items may need refresh. …". The concatenation is the same as the base; only the hint text is new.
   - After Apply and Undo, every comment reads "Passage not found in v3…" even where the passage text is present again. Comments stay exact to their version (`exact_writing_range`, unchanged), but the new wording suggests the text is gone where the base said "not re-anchored".
   - Backup success repeats the full path in the status line and in the banner.
   - Changes times show UTC ("15 Oct, 22:02 UTC") on a UTC+3 machine.
5. **Minor focus and visual issues:**
   - In Settings, the focus ring stays on General after another section is clicked.
   - Continue and Send are both filled in the chat panel.
   - With the comments section, the saved-inputs row and a dirty writing bar, the 900×600 Note text box is visibly small, about 110 pt.

**Fixture artifacts, not findings:**

- "Not answered yet" cards appear because the fixture posts owner messages without runs.
- The failed run is a synthetic Run record.
- Model labels show the default `gpt-6.1-sol`.

## Not observed

- **Fix-pass items not rechecked natively** (automated-verified only):
  - 5c, the busiest Note's text box
  - INT-1, the kept-edit Recover primary
  - INT-4 and FIX-2, the banner stack and the text floor under a banner (the rule is **pending owner acceptance**)
  - U10, the recorded terminal notice
  - the menu-launched dialogs other than Projects ⋯ (Note ⋯ Edit note… and Discard edit…, Chats ＋ New note, and the row menus' Move to project…)
  - FIX-3, the Windows build
- Live streaming, Stop/Steer, U1 Stop… on an orphaned Working run, the Note's still-running state, and the Quiet-off trace. These need a running worker; no provider calls were made.
- The U3 review rows with Protected, a stale-edit or newer-version banner, and the import-summary banner. The fixture doesn't create these states. After a restart, the unsaved edit came back as an open recoverable edit, which was observed at the recheck.
- Attachments, Keep in BRN, Save as decision, Edit note, Connections, Knowledge check, Compare, Search, the Trash restore flow, the delete dialog's task choices, export, Change folder, and native file pickers.
- Hover tooltips (apart from incidental ones and the queue strip's), drag-resize, the 1050×700 size, Solo, Läänemeri, Paber, VoiceOver, IME, and contrast measurement.
- Tab-to-⋯ → ↵ → ↓ ↵ by real keys. Native-test-verified only.
- Owner acceptance, and comparison against a native base build.
