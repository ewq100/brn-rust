# BRN Threads roadmap

The core, provider runtime and native workspace are implemented. Reconciled against main `f30f37e0` (PR #225): later work includes Projects, hybrid retrieval/Topics/Connections, instruction-only skills, narrow local MCP, the current Xberg intake routes, Linux UI support and precise editing. These shipped capabilities do not establish complete native or owner acceptance. The [canonical target](architecture/threads-target.md) owns the selected scope; [status](status.md) owns actual evidence.

| Work | Owner-visible result | Remaining proof |
|---|---|---|
| Tiled UX | Consistent Home, search, Actions, history, settings and document/chat workspace | Implemented; later bounded Linux observations exist, with broader Mac/owner acceptance remaining |
| First preview | Relocatable macOS arm64 app with explicit limits | v0.1.0-preview.1 published, v0.1.0-preview.2 adds the Omakase shell; package/startup/public-download integrity verified, broader acceptance below |
| Recovery qualification | Understandable interruption/restart, backup and restoration before daily data adoption | Cold interruption/restart and broader recovery acceptance; fault/reopen tests are partial evidence |
| Native acceptance | Dependable Home, thread, editor, review, comments, search, Actions, history and export | Complete target-Mac interaction and restart journeys remain; bounded historical Mac and Linux observations are separate evidence |
| Daily-use acceptance | Source/email → reliable working note → supported Action or reply | Representative accepted results, clear partial coverage and recoverable local changes |
| Knowledge and retrieval | English-first hybrid search, agent search tools and an agent-maintained knowledge layer for 5,000–10,000 notes | Implemented [retrieval/index](../crates/brn-retrieval/README.md), Topics and Connections; English-first corpus/model quality, broader native and owner acceptance remain separate |

Local reading, writing, search, Actions, review and recovery work offline; inference pauses visibly when the selected provider is unavailable. The first-release scope includes both useful-information intake and full-note import for qualified formats, Markdown snapshots and complete BRN backup/restore. Universal conversion and mathematical equation fidelity are outside it.

DOCX is the first later output format, followed by PPTX/HTML. External sending, broad connectors, sync/multiplayer, a background daemon, arbitrary branch merging and plugin/theme marketplaces remain later scope. New adopted-schema changes require transactional upgrades and pre-upgrade backups; migration from old BRN is outside the target.

The [decision backlog](work/active/daily-use-reliability/decision-backlog.md) reconciles shipped, partial and future outcomes without assigning them. [Documentation audit](work/active/daily-use-reliability/documentation-audit.md) records stale claims and preserved historical boundaries. Current batch execution remains in the selected task record reached through status.
