# BRN Threads roadmap

The core, provider runtime and native workspace are implemented. Phase 0/F1 and the Phase 1 delegation, steering and backup/restore increments are merged through PR #120; cleanup, the writing workbench and app-wide tiled UX are merged through PR #123. The [canonical target](architecture/threads-target.md) owns the selected scope; [status](status.md) owns actual evidence.

| Work | Owner-visible result | Remaining proof |
|---|---|---|
| Tiled UX | Consistent Home, search, Actions, history, settings and document/chat workspace | Merged in #123; observed native interaction remains deferred |
| First preview | Relocatable macOS arm64 app with explicit limits | v0.1.0-preview.1 published, v0.1.0-preview.2 adds the Omakase shell; package/startup/public-download integrity verified, broader acceptance below |
| Recovery qualification | Understandable interruption/restart, backup and restoration before daily data adoption | Cold interruption/restart and broader recovery acceptance; fault/reopen tests are partial evidence |
| Native acceptance | Dependable Home, thread, editor, review, comments, search, Actions, history and export | Actual target-Mac interaction and restart journeys; observation is currently deferred |
| Daily-use acceptance | Source/email → reliable working note → supported Action or reply | Representative accepted results, clear partial coverage and recoverable local changes |
| Knowledge and retrieval | English-first hybrid search, agent search tools and an agent-maintained knowledge layer for 5,000–10,000 notes | Proposed [architecture and plan](work/active/knowledge-retrieval/spec.md), building on chat stage 1 and the ordinary-wiki direction; not yet accepted or started |

Local reading, writing, search, Actions, review and recovery work offline; inference pauses visibly when the selected provider is unavailable. The first-release scope includes both useful-information intake and full-note import for qualified formats, Markdown snapshots and complete BRN backup/restore. Universal conversion and mathematical equation fidelity are outside it.

DOCX is the first later output format, followed by PPTX/HTML. External sending, broad connectors, sync/multiplayer, a background daemon, arbitrary branch merging and plugin/theme marketplaces remain later scope. New adopted-schema changes require transactional upgrades and pre-upgrade backups; migration from old BRN is outside the target.
