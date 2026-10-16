# UI polish: verification

**Date:** 16 October 2026 (times are EEST, UTC+3). **Branch:** `work/ui-polish`, HEAD `ed8daa82` (base `771116f0`), worktree `.worktrees/ui-polish`. Nothing here is owner-accepted, published, merged or released. The record step commits this file and [`after/`](after/) with [evidence.md](evidence.md).

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

All seven slice reviews and the integration review (`code-review.md`, an uncommitted workflow file) are APPROVED. `git diff 771116f0 -- crates ':!crates/brn-desktop'`, `Cargo.lock` and the crate manifests are all empty.

## Automated-verified

Run on HEAD `ed8daa82` with `PATH=/opt/homebrew/opt/rustup/bin:$PATH` and `CARGO_TARGET_DIR=<worktree>/target`, Rust 1.98.1. All commands exited 0. The only warning was the existing `block 0.1.6` future-incompatibility notice.

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

**Journeys observed at 900×600 (one at 1250×860)**, in light (Sammal) and dark (Virmalised, Raba). Shots are 1100 px wide in [`after/`](after/); [`before/`](before/) was captured mostly at 1250×860 with the same fixture.

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
  - Shots: [22](after/22-chat-needs-you-900-light.png), [29](after/29-chat-failed-900-dark-raba.png), [25](after/25-archived-chat-900-light.png); compare [before 22](before/22-chat-needs-you-light.png), [before 25](before/25-archived-chat-light.png) and [before 29](before/29-chat-failed-dark-raba.png).
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

**Defects and rough edges seen natively.** No fixes are made in this step.

1. **Keyboard focus is lost after Escape closes a dialog opened from a ⋯ menu item.** This is the most important finding, because it breaks predictable keyboard use.
   - After Projects ⋯ → Move to Trash… → Escape, and again after Projects ⋯ → Instructions & settings → Escape, ⌘K and Tab did nothing until a click in the window.
   - Escape on the menu alone, ⌘N's dialog and Keys help (opened from a button) kept the keyboard working.
   - The Note ⋯ "Discard edit…" path was not checked. These menu entry points are new on this branch; the base was not compared.
2. **"Paused" is truncated** to "1 waiting · Pa…" in the queue strip of the narrow Write-layout chat companion at 900×600. Chat layout shows "1 waiting · Paused" in full. The tooltip was not checked.
3. **Clipping at narrow widths:**
   - The delegation card's "BRN updates ordinary notes and source referen…" runs past the card edge in the chat companion, and its "?" wraps onto its own line ([43](after/43-delegation-card-clipped-900-light.png)).
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

- Live streaming, Stop/Steer, U1 Stop… on an orphaned Working run, the Note's still-running state, and the Quiet-off trace. These need a running worker; no provider calls were made.
- The U3 review rows with Protected, a stale-edit or newer-version banner, the import-summary banner, and the recoverable-edit banner after a restart. The fixture doesn't create these states.
- Attachments, Keep in BRN, Save as decision, Edit note, Connections, Knowledge check, Compare, Search, the Trash restore flow, the delete dialog's task choices, export, Change folder, and native file pickers.
- Hover tooltips (apart from incidental ones), drag-resize, the 1050×700 size, Solo, Läänemeri, Paber, VoiceOver, IME, and contrast measurement.
- Tab-to-⋯ → ↵ → ↓ ↵ by real keys. Native-test-verified only.
- Owner acceptance, and comparison against a native base build.
