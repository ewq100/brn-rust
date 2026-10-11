# BRN Threads status

**Updated:** 11 October 2026. **Merged baseline:** [PR #115](https://github.com/ewq100/brn-rust/pull/115) at `53e5289e7221a4c3b66c6c454550311e46617a78`. **Current work:** isolated branch `work/phase1-delegation`.

## Current assignment

The owner requested Phase 0 PR publication and CI-gated merge, explicitly deferred blocked native observation, and authorized continued building with GPT-6.1 Sol helpers. The next bounded increment is [Phase 1 explicit delegation](work/active/threads-rebuild/phase1-delegation-plan.md): first-use maintenance/review-first choice, fail-closed standing authority and matching native/CLI controls.

| Gate | State |
|---|---|
| Phase 0 integration | Merged; all four required checks passed before merge and again on main. Documentation/tooling also passed. |
| Phase 0 limitations | Native observation explicitly deferred by owner. Non-required Windows and Linux model-install failures remain visible; no all-platform green claim. |
| Phase 1 plan | Independently reviewed and approved. |
| Phase 1 implementation | Implemented in isolated worktree; existing checked writer, no schema/dependency upgrade. |
| Focused verification | Passed; [evidence](work/active/threads-rebuild/phase1-delegation-evidence.md) records commands, counts and limitations. |
| Independent candidate review | APPROVED; no actionable code defects. |
| Native observation | Computer Use access remains unavailable; native component tests are separate evidence. |
| Phase 1 integration/release | PR publication and CI-gated merge explicitly authorized; not yet merged or released. |

## Implemented baseline and retained evidence

The product has one SQLite core, shared checked changes/Undo, atomic current-input validation, native notes/threads/Actions/review/recovery, per-context session drafts, bounded source conversion, keyword search, portable Markdown metadata and complete backup/restore. Production uses Rig 0.44.0, host-scoped authority, nine registered tools and packaged runtime guides. See [architecture](architecture/overview.md), [runtime contract](architecture/threads-behavior.md#implemented-response-and-authority-contract) and [Phase 0 evidence](work/active/threads-rebuild/phase0-evidence.md).

The prior [rebuild evidence](work/active/threads-rebuild/evidence.md#rig-044-runtime-integration) records its exact source, automated checks and synthetic live provider journeys; those counts are historical and overlapping. They are not fresh qualification of this increment. Personal macOS is the adopted scope. Native responsiveness, general IMEs, complex/scanned/math PDF fidelity and external distribution remain unqualified.

## Next action

The delegation candidate is implemented, tested and independently approved; native observation remains deferred; publish and merge after required CI, then continue Phase 1 under the owner’s explicit instruction. Remaining Phase 1 work includes Stop-and-send steering and storage/backup/recovery foundations before non-disposable adoption; these are not claimed complete by the first-use change.
