# Native editor trial implementation plan

Date: 2026-09-27. Base: eb62bfc. Branch: trial/editor-selection-comments.

The user authorized continuing with best judgment after the completed provider trial. This implements the original roadmap editor trial 02, not another provider lifecycle chunk.

## Goal and design

Deliver a runnable Apple Silicon Rust experiment that opens a long UTF-8 Markdown document, captures a real editor selection, attaches a comment, tracks safe edits, marks uncertain anchors visibly, and displays a revision diff. Use GPUI Kit 0.6.6 as the existing roadmap candidate; keep the framework choice provisional pending target-OS interaction and user acceptance. No production CLI or provider harness change.

Standalone `experiments/editor-trial` owns an in-memory document/comment model and optional `native-ui` binary. The immutable opened document is the diff baseline. Model tests run without a graphical session. The native binary loads the synthetic fixture by default or an explicitly supplied UTF-8 path; edits remain in memory, with no automatic saving/overwriting originals. Show that lifetime in the window. No cloud, authentication, CRDT, database or provider integration.

Capture the widget's selected UTF-8 range against its current text before focus moves to the comment input. A pending selection carries its text snapshot; if the document changed, reject attachment and ask for a new selection. Store each comment's original snapshot/range/quote/body plus current anchored range or unresolved reason. Prefer actual widget edit deltas if exposed. Otherwise apply conservative snapshot mapping: UTF-8-safe prefix/suffix replacement; edits wholly before shift, wholly after preserve; overlaps/interior insertion invalidate. Duplicate quote occurrences in either before/after snapshot make mapping unresolved. Start-boundary insertion shifts, end-boundary insertion stays outside. Never use nearest-quote search to recover. Only exact full-buffer restoration of a comment's original snapshot can restore its original range. Decorations may visualize anchors but do not define their identity.

Use the `similar` library for an explicit baseline/current unified line diff, preserving newline distinctions. Generate diff on demand to avoid expensive work per keystroke. Keep the view usable at a narrow window size through scrolling/limits rather than clipping controls.

## Global constraints

- Trial-only edits in a separate worktree; no unrelated work, production refactors, cloud calls or credential access.
- Preserve original documents. No save/export flow in this trial; state is intentionally discarded on exit and labeled in the UI.
- Use real native selection and input; test controls must not stand in for interaction acceptance.
- No automatic guessed reattachment to duplicate/deleted passages. Preserve original quotes in the review pane.
- No merge, release, force-push, signing identity changes or installation into Applications. An unsigned local launch wrapper, if needed for testing, stays outside Git.
- Sol implements source; root owns manifest/dependency integration, docs, verification and push; Astra reviews.

## Task 1: Model and native trial implementation

Files owned by implementer: `experiments/editor-trial/src/lib.rs`, `src/main.rs`, optional `src/ui.rs`, and `fixtures/long-document.md`. Manifest and lock are owned centrally; coordinate dependency changes first.

- [x] Add failing tests for safe anchor shifts/boundaries, overlap/delete/duplicate ambiguity, UTF-8 invalid boundaries, full-buffer undo/redo and diff/newline behavior.
- [x] Implement a small model API sufficient for native UI with immutable baseline and original comment snapshots.
- [x] Build the native pane flow using official downloaded GPUI APIs: Markdown editor, Capture selection/Add comment flow, comments with anchor status, show/focus anchored passage where feasible, refreshable readonly revision diff, document/session status.
- [x] Load built-in long synthetic Markdown fixture or one CLI path with bounded file size and useful invalid/extra argument, missing-file and invalid-UTF8 errors. Test headless loading helpers.
- [x] Run model tests, formatting, native build and Clippy; record commands/results and limitations.
- [x] Independent review for compliance and quality; resolve important findings.

## Task 2: Integration and acceptance evidence

- [x] Verify pinned native dependencies and licenses; compile on this Mac.
- [x] Run appropriate build, format, Clippy, model tests and original starter/provider regression suite.
- [ ] Launch native app and inspect UI with computer-use tools; exercise real selection/comment/edit/diff, scrolling, resize, Unicode, clipboard, undo and deleted anchors as tooling permits.
- [x] Record exact automated versus observed versus user-unverified outcomes; do not mark human acceptance or framework selection complete on the user's behalf.
- [x] Whole-branch Astra review and secret/diff scan (implementation checkpoint approved; native acceptance still blocked).
- [x] Commit/push trial branch and verify remote SHA (implementation checkpoint; native acceptance still blocked).

## Review focus

Repeated text must not steal anchors. A selection must belong to the text snapshot used to create it. Byte ranges must respect Unicode boundaries. Native document changes including undo must reach the model. Diff viewing and focus changes must not overwrite document contents. No UI auto-save or unrequested original-file changes.

## Progress

- Clean provider baseline eb62bfc preserved; editor work isolated at /private/tmp/brn-editor-trial.
- Official GPUI Kit installation/getting-started/editor docs and crates.io metadata consulted; version 0.6.6 is published, Apache-2.0. Target prerequisites confirmed.
- Astra architecture review recommends conservative anchor mapping and explicit human acceptance boundary.

- Task 1 source review and two scoped correction passes approved. Native build, format, Clippy, 11 model tests and five CLI checks passed. Native acceptance is BLOCKED by the locked Mac, not complete; unlock requested.

- Implementation checkpoint `8a81268aff6b7f401b63d815b00cf62fdb63a1c3` pushed; `git ls-remote origin refs/heads/trial/editor-selection-comments` matched exactly. This documentation-only follow-up records the delivery. Nothing merged or released.
