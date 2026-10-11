# BRN Threads status

**Updated:** 11 October 2026. **Merged baseline:** [PR #120](https://github.com/ewq100/brn-rust/pull/120), `3a3aa7ac6f2a70815ffc778a694663f729203a79`. **Current assignment:** [pre-release cleanup](work/active/pre-release-cleanup/plan.md).

The implemented product has one SQLite core, shared checked changes/Undo, current-input validation, native notes/threads/Actions/review/recovery, per-context drafts, bounded source conversion, keyword search, portable Markdown export and complete backup/restore. Production uses Rig 0.44.0, host-scoped authority, nine registered tools and packaged runtime guides. See [architecture](architecture/overview.md) and [runtime behavior](architecture/threads-behavior.md#implemented-response-and-authority-contract).

| Gate | State |
|---|---|
| Rebuild and Phase 0/F1 | Merged in PRs #114–115; detailed proof is [archived](archive/threads-rebuild/evidence.md). |
| Phase 1 | Delegation #116, Stop-and-send #117, atomic backup #118, restore safety #119 and remembered backup folder #120 merged after required CI. All required main checks for #116–120 were recovered and confirmed green. |
| Platform scope | Personal macOS first. Non-required Windows credential and Linux native-retrieval/model-install failures remain visible; no all-platform green claim. |
| Cleanup | Implemented with local checks and independent candidate review passed. Hosted PR/main integration receipts are recorded on the cleanup PR; the active plan records the frozen candidate. |
| Native observation | Explicitly deferred. Compilation/native state tests do not establish typing, focus, dialogs, IME, selection or responsiveness. |
| Recovery acceptance | Cold interruption/restart and broader native recovery acceptance remain outstanding before non-disposable data adoption. |
| Release | Not authorized or performed. |

[Selected historical reports](archive/README.md) preserve exact tested sources, local checks, synthetic provider journeys and prior integration evidence. Their overlapping pass counts are dated proof, not fresh qualification of the cleanup candidate. Complex/scanned/math PDF fidelity, general IMEs and external distribution remain unqualified. The [roadmap](roadmap.md) separates remaining first-release gates from later capabilities.
