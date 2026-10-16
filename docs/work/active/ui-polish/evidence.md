# UI polish: evidence

**Date:** 16 October 2026. **Branch:** `work/ui-polish` (local only, not pushed), base `771116f0`. Product code is at `ed8daa82`; the record commit adds only these documents. **Design:** [plan.md](plan.md). **Slices:** [impl-plan.md](impl-plan.md). **Detailed verification and native observation:** [verification.md](verification.md).

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

**Defects seen natively and not fixed** (detail in [verification.md](verification.md)):

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

## Owner-accepted

No. The owner has not reviewed the implemented UI.

## Merged

No. Nothing is pushed, merged, published or released.

## Open items

- **Native defects 1–5 above**, and the non-blocking integration findings:
  - INT-1: in the kept-edit state, Rewrite with AI is the filled primary instead of Recover edit.
  - INT-2: Undo stays enabled during an open edit.
  - INT-3: Edit note and Connections are menu-only at every tested size.
  - INT-4: the stale-banner actions can scroll out at the banner-stack floor.
  - INT-5: some new states and routes have no tests.
  - INT-6: a disabled Show passage drops its quote from the accessible name.
  - INT-7: the link check depends on this record commit, which resolves it.
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
