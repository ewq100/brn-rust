# Focused native usability milestone

2026-09-28 · `trial/end-to-end-flow` · baseline `7380b63`.

## Intent and design

Make the already verified synthetic native import → retrieval → grounded-answer flow easier to use repeatedly. The user authorized routine decisions, implementation, verification, review, commit and push on the trial branch. No merge or release is included. Astra supplies architecture/review, Sol implementation, and Luna narrow checks.

Keep the current GPUI desktop and shared worker. There is no storage migration, provider replacement, private-vault import or new publication behavior. Choosing a file only selects it; the existing explicit import-and-search-approval action remains separate. GPUI's installed native picker has no extension-filter option, so the authoritative workflow continues validating supported Markdown/text, UTF-8 and size limits.

Use an explicit initial window size and useful minimum size, a scrolling page body, and persistent navigation/status/cancellation controls. Stack source metadata and compact long conversation labels; show full content in details. Preserve generation checks and saved evidence identity. Provide a clear route from completed answers to saved conversation content.

Distinguish workspace opening, ready, running, cancellation requested and failure. Disable conflicting work while busy and after a fatal open failure, preserve diagnostics and show contextual recovery guidance. Progress is indeterminate and truthful, with no fabricated percentage or automatic replay. Native model work remains cancellable only at the existing safe boundaries.

Generate a local unsigned trial app from explicit binary/workspace/dependency paths. Launch must not depend on a shell's working directory, silently create a different workspace, download resources, or bundle Codex/models. Preserve arguments safely, including spaces and Unicode; make startup failures and their local log discoverable. This is a repeatable personal launcher, not release packaging.

## Execution and review checklist

- [x] Read roadmap, status and end-to-end evidence; confirm clean baseline and branch.
- [x] Astra architecture recommendation and Luna environment checks.
- [x] Baseline `cargo test --workspace --locked --offline`.
- [x] Sol: native picker, layout, presentation state/progress, launcher and focused regressions.
- [x] Credential-free workflow/starter regressions, native build/test/Clippy, launcher checks.
- [x] Native inspection: picker success/cancel/invalid input; small/large window; source/evidence/answer reachability; errors and persistent status; same-workspace relaunch.
- [x] Astra source review and any required scoped correction review.
- [x] Update evidence, roadmap/status and launch instructions; verify final diff.
- [x] Final trial checkpoint prepared for commit/push; no merge or release. Remote synchronization is verified at handoff.

## Review focus

Picker cancellation must preserve the selected file, and a closed view must not receive a new import. Long source names, paths, UUIDs and multiline questions must not hide actions. Opening or fatal startup failure must not look ready. Cancellation remains requested until the terminal outcome arrives. A launcher must preserve literal arguments and expose failures before the GUI opens. Existing session, evidence, provider shutdown and uncertain-operation behavior must remain intact.

## Acceptance limits

Use only disposable synthetic documents and existing local model/provider setup. Native observations are separate from compilation and automated checks. Natural authentication expiry, accessibility qualification, representative-corpus quality, clean-machine installation, signing and release remain outside this milestone.
