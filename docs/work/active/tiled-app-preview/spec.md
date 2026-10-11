# App-wide tiled UX and first preview — accepted scope

The owner requests the whole app use the accepted tiled writing style, explicitly including Home, search, Actions, history and settings; GPT-6.1 Sol subagents; preserved checked writes and Undo; a PR and a release. This authorizes implementing, publishing, integrating after required CI and releasing this scope without routine reapproval. Native/live-provider acceptance remains deferred, so the first distribution is a clearly labelled macOS Apple Silicon preview (`v0.1.0-preview.1`), not a claim of daily-use qualification.

## UX contract

Use one shared cream/green/ink palette, square bordered tiles, compact monospace chrome/headings and readable system-font prose. Standard toolkit controls retain keyboard, focus, disabled and selected states. Apply the same treatment to global navigation, imports, search/results, status/errors, composer, Home, Actions, history, settings, standalone thread/source context, notes, comments and exact-candidate review. This is presentation work over existing domain state and callbacks, not a new component framework.

Home presents decision/recovery/review/recent-work tiles with meaningful empty states. Search remains current-note search and exposes result count, snippets and empty/error feedback without changing its scope. Actions are grouped by existing status with all current controls and full descriptions. History retains receipt contents and Undo on the same receipt. Settings has readable labelled fields and distinct provider, delegation, export and backup groups. Small windows stack/scroll panes with controls accessible, preserving drafts and the single note editor.

Do not change checked service authority, persistence, ordinary delegation, snapshot rewrite semantics, dirty/stale protections, exact review, cancellation, recovery, exports/backups or receipt Undo. No paragraph shortcut controls, new dependencies or schema changes. Existing action IDs and keyboard shortcuts stay stable.

## Preview contract

Provide a relocatable macOS arm64 app containing the native UI binary and required intake helper. Assets/guides already compiled into the binaries stay there. Use a stable preview-only per-user workspace under `~/Library/Application Support/BRN Threads Preview/workspace`, without embedding builder paths, inspecting old owner data or creating a new workspace per version. The core's existing marker/schema checks remain authoritative. Preserve the explicit local packaging mode.

Package an ad-hoc-signed app archive, SHA-256 checksums and build manifest with exact source commit, architecture/features and qualification limits. Do not invent Developer ID/notarization credentials or claim notarization. Keyword search works offline; no embedding-model payload or provider credentials are bundled. PDF conversion requires external Poppler and must be documented. First-use installation/provider instructions use supported flows.

Publish a GitHub prerelease only from the integrated intended source after the four existing required CI jobs succeed on that exact current main commit. Existing optional platform failures do not become fabricated success. Trigger only an explicitly approved version manifest change, reject moved main/version collisions, and never overwrite/retarget existing release assets. A failed or incomplete upload must not become a published complete release. Verify published assets and checksum links.
