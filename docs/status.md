# BRN Threads status

**Updated:** 11 October 2026. **Merged baseline:** PR #114 at `d768db7a221ad1245241fa5fdca4461bc3cc2409`, confirmed against remote main. **Current work:** isolated branch `work/phase0-stale-input`.

## Current assignment

The owner authorized [Phase 0 and F1 stale-input hardening](work/active/threads-rebuild/phase0-plan.md), with GPT-6.1 Sol implementation subagents, focused tests and the existing checked-write architecture. The owner accepted the [adoption priorities](architecture/threads-target.md#accepted-adoption-priorities-11-october-2026); this is not acceptance of unseen UI or authority to implement all later phases.

| Gate | State |
|---|---|
| Plan | Independently reviewed and approved. |
| Implementation | Complete: truthful Stop, per-context drafts, portable export metadata, explicit current-read validation, representative headless baseline, current docs. |
| Focused verification | Passed core/app/runtime/desktop tests, affected strict Clippy, formatting and docs/tooling checks; see [Phase 0 evidence](work/active/threads-rebuild/phase0-evidence.md). |
| Independent code review | Approved; no actionable implementation defects found. |
| Native observation | Candidate built with fresh synthetic workspace; Computer Use permission denied. Startup, typing and rendered UI remain unobserved. |
| Owner acceptance | Owner explicitly deferred native observation and authorized PR plus merge after CI passes on 11 October 2026. |
| Integration/release | PR publication and CI-gated merge authorized; not yet merged or released. |

## Implemented baseline and retained evidence

The merged product has one SQLite core, shared checked changes and Undo, native notes/threads/Actions/review/recovery, bounded source conversion, keyword search, export and complete backup/restore. Production uses Rig 0.44.0, host-scoped authority, nine registered tools and packaged runtime guides. See [architecture](architecture/overview.md) and [runtime contract](architecture/threads-behavior.md#implemented-response-and-authority-contract).

The prior [rebuild evidence](work/active/threads-rebuild/evidence.md#rig-044-runtime-integration) records its exact source, automated checks and synthetic live provider journeys. Its 352-workspace/194-AI counts overlap; they are historical qualification of that candidate, not fresh results for this branch. Latest prior streaming/activity/Stop-Continue visual observations were blocked by a locked Mac and owner acceptance remained pending. Those limitations are retained until new evidence supersedes them.

Current scope is personal macOS use. Complex/scanned/math PDF fidelity, other IMEs, external distribution and the recorded Windows credential issue were not qualified by the prior pass.

## Next action

Publish Phase 0, satisfy required hosted checks, merge, and verify main CI. Then continue the accepted Phase 1 foundation fixes. Native observation is explicitly deferred by the owner and remains a documented limitation.
