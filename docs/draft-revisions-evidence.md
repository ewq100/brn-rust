# Draft and revision verification evidence

2026-09-28 · `feature/draft-revisions` · base `5d48737` (main).

## Execution record

The requested baseline was clean. The roadmap, status, architecture and desktop usability evidence were read. Work is isolated in a native-managed worktree. Astra owns architecture/review, Sol implementation, Luna narrow checks, and the controller native inspection and integration. The [plan](draft-revisions-plan.md) records the scoped design under the user’s explicit authorization for routine autonomous decisions. No merge or release is authorized.

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

Pending implementation and final review.
