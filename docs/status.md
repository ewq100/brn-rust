# BRN Threads status

**Updated:** 11 October 2026. **Merged baseline:** [PR #122](https://github.com/ewq100/brn-rust/pull/122), `7c189c71d3eb664ef6493294d5b73d3e12b90b7d`. **Current assignment:** [app-wide tiled UX and first preview](work/active/tiled-app-preview/plan.md).

The implemented product has one SQLite core, shared checked changes/Undo, current-input validation, native notes/threads/Actions/review/recovery, per-context drafts, bounded source conversion, keyword search, portable Markdown export and complete backup/restore. Production uses Rig 0.44.0, host-scoped authority, nine registered tools and packaged runtime guides. See [architecture](architecture/overview.md) and [runtime behavior](architecture/threads-behavior.md#implemented-response-and-authority-contract).

| Gate | State |
|---|---|
| Rebuild and Phase 0/F1 | Merged in PRs #114–115; detailed proof is [archived](archive/threads-rebuild/evidence.md). |
| Phase 1 | Delegation #116, Stop-and-send #117, atomic backup #118, restore safety #119 and remembered backup folder #120 merged after required CI. All required main checks for #116–120 were recovered and confirmed green. |
| Platform scope | Personal macOS first. Non-required Windows credential and Linux native-retrieval/model-install failures remain visible; no all-platform green claim. |
| Cleanup | Merged in #121 with required PR/main checks passing; exact integration receipts are recorded on the cleanup PR. |
| Writing workbench | Merged in #122 after all four required PR checks passed. Tiled note/full chat, multiple passage comments, saved rewrite context, proposal-only Apply and Undo are implemented; [evidence](work/active/tiled-writing-workbench/evidence.md) records limits. All four required main checks also passed; unrelated optional platform failures remain visible. |
| App-wide tiled UX | Implemented shared styling for Home, search, Actions, history, settings and all existing writing surfaces. All 56 native widget/state tests pass, including six new app-wide journeys; strict native lint passes. Independent review and integration are recorded in the [task evidence](work/active/tiled-app-preview/evidence.md). |
| Native observation | Explicitly deferred. Compilation/native state tests do not establish typing, focus, dialogs, IME, selection or responsiveness. |
| Recovery acceptance | Cold interruption/restart and broader native recovery acceptance remain outstanding before non-disposable data adoption. |
| Release | Owner authorized `v0.1.0-preview.1`, a macOS 15+ arm64 preview with portable packaging and exact-commit CI gates. The [install guide](development/preview-release.md) records ad-hoc signing, external PDF tools and qualification limits. Publication awaits reviewed integration and required main CI; the GitHub prerelease is the publication receipt. |

[Selected historical reports](archive/README.md) preserve exact tested sources, local checks, synthetic provider journeys and prior integration evidence. Their overlapping pass counts are dated proof, not fresh qualification of the cleanup candidate. Complex/scanned/math PDF fidelity, general IMEs and external distribution remain unqualified. The [roadmap](roadmap.md) separates remaining first-release gates from later capabilities.
