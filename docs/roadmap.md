# BRN Threads roadmap

The core, provider runtime and native workspace are implemented. Phase 0/F1 and the Phase 1 delegation, steering and backup/restore increments are merged through PR #120. The [canonical target](architecture/threads-target.md) owns the selected scope; [status](status.md) owns actual evidence.

| Work | Owner-visible result | Remaining proof |
|---|---|---|
| Chat and AI writing UX research | Evidence-led improvement proposal before first release | Upcoming research; no accepted design or implementation yet |
| Pre-release cleanup | Clear current instructions and a smaller maintained checkout | Reviewed deletion justification, unchanged behavior, relevant checks and required-CI-gated integration |
| Recovery qualification | Understandable interruption/restart, backup and restoration before daily data adoption | Cold interruption/restart and broader recovery acceptance; fault/reopen tests are partial evidence |
| Native acceptance | Dependable Home, thread, editor, review, comments, search, Actions, history and export | Actual target-Mac interaction and restart journeys; observation is currently deferred |
| Daily-use acceptance | Source/email → reliable working note → supported Action or reply | Representative accepted results, clear partial coverage and recoverable local changes |

Local reading, writing, search, Actions, review and recovery work offline; inference pauses visibly when the selected provider is unavailable. The first-release scope includes both useful-information intake and full-note import for qualified formats, Markdown snapshots and complete BRN backup/restore. Universal conversion and mathematical equation fidelity are outside it.

DOCX is the first later output format, followed by PPTX/HTML. External sending, broad connectors, sync/multiplayer, a background daemon, arbitrary branch merging and plugin/theme marketplaces remain later scope. New adopted-schema changes require transactional upgrades and pre-upgrade backups; migration from old BRN is outside the target.
