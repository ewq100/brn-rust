# Draft and revision verification evidence

> Historical record: branch names, commands, approvals and results below describe the recorded task, not new instructions or current authorization. See [current status](../../../status.md) for the present checkout and remaining gaps.

2026-09-28 · `feature/draft-revisions` · base `5d48737` (main).

## Execution record

The requested baseline was clean. The roadmap, status, architecture and desktop usability evidence were read. Work is isolated in a native-managed worktree. Astra owns architecture/review, Sol implementation, Luna narrow checks, and the controller native inspection and integration. The [plan](plan.md) records the scoped design under the user’s explicit authorization for routine autonomous decisions. No merge or release is authorized.

## Deferred native editor checks

A fresh current-source editor experiment was built using Rust 1.98.1 and launched as a disposable unsigned `BRN Editor Check.app`. An older already-running process was excluded from acceptance. All data is synthetic and in-memory.

Observed through native computer use:

- The 14,814-byte, 323-line sample renders. Deep editor scrolling reaches section 40; resizing to approximately 850 by 650 points keeps editing, page scrolling and review controls reachable.
- Keyboard line selection survives Capture selection, Unicode comment-entry focus and Add comment. Mouse-drag selection of `Préface 東京 🌊` likewise produces the correct 20-byte quote.
- Show passage scrolls the outer page back to the editor and visibly selects the correct passage.
- Inserting `Préface 東京 🌊\n` before an anchored heading shifts its range from 0–38 to 21–59. Deletion and native copy/paste duplication display an explicit unresolved warning and retain the original quote.
- Native Cmd-Z restores the original complete document and its anchor; refreshed diff reports no changes. Cmd-Shift-Z reapplies the Unicode insertion. Refreshed diff after duplication shows both added lines correctly.
- Multiple comment quote panels preserve separate original quotes after deleting the whole working document. A third quote contains all 14,874 edited bytes.

The long-quote test exposed a defect: the GPUI scrollbar wrapper did not expose useful scroll movement for this nested fixed-height quote. Sol replaced it with a stable-ID direct vertical scroller and width-constrained wrapped lines. Fresh native rechecks proved the full introductory sentence readable at narrow width and scrolling through the final Unicode sample in section 40 after deleting the working document. Build, format, all 11 editor tests and all-target Clippy passed after the correction. Astra’s scoped final review found no remaining issue. The deferred agent-observable native checks are complete; user subjective suitability acceptance remains separate.

Accessibility `selectText` reports that the editor does not expose a settable selected range; actual keyboard/mouse selection works. Direct `typeText` in preliminary testing truncated non-ASCII input, while native clipboard paste preserved the complete Unicode text. Acceptance uses the observed clipboard route and does not qualify IME, screen readers, sustained editing, or near-limit performance. Agent observations do not substitute for the user’s own suitability evaluation.

## Automated baseline

Luna ran the clean baseline with `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/private/tmp/brn-editor-trial/target`, and `CARGO_BUILD_JOBS=4`:

- `cargo +1.98.1 test --workspace --locked --offline`: passed.
- Editor manifest tests: 11 passed.
- Editor native-ui build, formatting check and all-target Clippy with warnings denied: passed.
- Five editor CLI smoke cases passed (help success; unknown/surplus/missing path/invalid UTF-8 failure).
- Provider harness: 16 tests passed without live calls.
- Known upstream Cargo future-incompatibility notice: `block 0.1.6`.

## Draft implementation verification

The implementation provides schema-v4 draft authority, exact mutable working copies, immutable checkpoint/candidate revisions, transactional compare-and-swap saves and repeatable operation results. Retry result snapshots deliberately retain historical results even after later edits; this adds storage and future decoder-migration overhead. Draft operations run through the existing serial worker. Revision comparison uses the existing `similar` dependency and preserves newline-only changes.

Astra’s storage review requested two corrections: changed checkpoint text must advance generation; saved retry-result snapshots need typed integrity/identity checks. Sol added failing regressions and fixed both. The focused storage suite passed 28 tests; workflow/worker tests passed 13. Astra scoped re-review found both addressed and no new breakage.

### Native draft acceptance

All data was synthetic. Unsigned disposable launchers used `/private/tmp/brn-draft-native-20260928/data` and `/private/tmp/brn-draft-native-candidate-20260928/data`, with no model or provider executable configured. The second workspace used the guarded `brn-store` example `seed_draft_candidate` to create a completed saved-answer fixture; this is not a live provider run.

Observed through native computer use:

- Created two drafts. Pasted multiline Markdown containing `Préface 東京 🌊`, saved its exact 56-byte working copy, checkpointed it, then saved a 76-byte changed checkpoint. Read-only SQLite inspection confirmed both checkpoint byte sequences and the independent second draft.
- Viewed the older checkpoint without replacing the working editor. Compared the two revisions: the diff showed `-First observation.`, `+Second observation.`, and `+Added conclusion.` with Unicode context intact.
- Dirty draft switching, new-draft creation, window close, Cmd-Q and native menu Quit preserved the live text and displayed an explicit save/discard instruction. Navigation between Workspace and Drafts retained dirty text. Discard restored acknowledged text.
- Saved, then immediately pasted more text. The later text remained visibly dirty. Native Cmd-Z removed the later paste and Cmd-Shift-Z restored it. Deterministic editor-state tests, rather than UI timing, establish the late-acknowledgement race guarantee.
- Reopened the same workspace in a separate native process. Working text and the immutable revision history reappeared exactly.
- Saved the synthetic completed answer as a candidate while a newer 32-byte user edit remained unsaved. The candidate preserved its root parent and originating answer; working text remained unchanged and dirty. After explicitly saving user text and restarting, both the working copy and candidate reappeared. Candidate review and root-to-candidate diff were read-only and exact.
- Resized to approximately 800×600 content points. Draft controls remained reachable through outer scrolling; editor, revision and diff panes retained useful heights, and identifiers wrapped. The full diff was readable through its own scroller.
- After the final Quit correction, the menu action remained enabled after comparison/navigation button focus, dirty menu/keyboard Quit was vetoed, and clean menu/keyboard Quit exited the process and released the workspace owner lock. Clean window close also exited. Post-exit SQLite checks verified exact working text and candidate text, parent, origin and SHA-256.

Two native defects were corrected before acceptance: flex layout compressed the editor/review/diff panes as history grew, and a root-scoped Quit action became unavailable when no editor held focus. Nonshrinking panes fixed the first; an application-global action with the same dirty/pending guard fixed the second. An initial no-op Quit was investigated as a possible shutdown hang; focused Quit and process checks identified action availability as the cause. The final checks used fresh copied binaries, not an older running trial.

### Automated final verification

- `bash scripts/verify-end-to-end.sh`: passed workspace formatting, build, all-target Clippy with warnings denied, workspace tests and the provider-free source/import/reindex/search fixture.
- Final affected rerun after `feb7e4a`: storage 28 tests, workflow 13 tests, native desktop 10 unit tests and five CLI cases, native build, all-target native Clippy with warnings denied, and workspace formatting passed. Native commands used `--features native-retrieval --locked --offline`. Fresh fixture creation and existing-fixture reuse rejection also passed.
- Provider harness: 16 tests passed; editor trial: 11 tests passed. No live provider call was made.
- Luna reran launcher regression and shell syntax checks successfully. Running the candidate seeder without `BRN_SEED_DISPOSABLE=1` refused the operation and left the proposed workspace nonexistent.

The shared build cache encountered insufficient disk space while linking the Quit correction. Only generated incremental build data under `/private/tmp/brn-editor-trial/target/debug/incremental` was removed, and the native build succeeded with `CARGO_INCREMENTAL=0`. The upstream `block 0.1.6` future-incompatibility notice remains. The debug native link also warned that its large `__eh_frame` section could affect exception-handling performance; the build succeeded and native checks passed.

### Review and scope

Astra reviewed the full implementation range through `f09d9e5` and found no additional correctness issue beyond the observed Quit availability defect. Scoped review of `f09d9e5..feb7e4a` found the global guarded Quit correction and fixture changes sound, with no remaining material finding. Native closure checks above passed. The reviewed feature branch is `feature/draft-revisions`; publication verifies its remote SHA after the evidence commit. Production anchored comments, candidate adoption, publication, release packaging, user subjective suitability, IME/screen-reader qualification and sustained/near-limit performance remain outside this verified slice. No merge, release, private-vault import or credential change occurred.
