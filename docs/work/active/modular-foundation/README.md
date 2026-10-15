# Modular foundation and reference-assisted writing — proposal

**Status: proposed, not started.** This folder stores the reviewed work pack for later use. It is not an active assignment and grants no implementation, merge or release authority. [Status](../../../status.md) remains the source of truth for current work.

**Owner decision, 15 October 2026:** `work/ui-polish` and `work/daily-workflow-reliability` are finished and merged into `main` first. C0, the X1/Z1 experiments and all later slices start only after both have landed.

## What to use

| File | Use |
|---|---|
| [proposal/final/00-start-here.md](proposal/final/00-start-here.md) | Start here: launch order and the C0 bootstrap message |
| [proposal/final/01-master-plan.md](proposal/final/01-master-plan.md) | Reconciled master plan (scope, contracts, slices, F01–F22 registry, milestone and program status) |
| [proposal/final/](proposal/final/) | All seven reconciled pack files (role prompts 02–04, review prompt 05, native outline 06) |

When the owner starts C0, the lead installs the single active `plan.md` in this folder (master plan §6) and links it from status.

## Provenance

| Path | Contents |
|---|---|
| `proposal/original/` | Pack as supplied for review, unchanged |
| `proposal/REVIEW.md` | Independent plan review: verdict, findings, simplified plan |
| `proposal/amended/`, `proposal/amendments.patch` | First amended pack and its patch against the originals |
| `proposal/RECONCILIATION.md` | Final reconciliation pass and owner-decision addendum |
| `proposal/final.patch` | Unified diff from the originals to the final pack |
| `proposal/delta-from-amended.patch` | Unified diff from the first amended pack to the final pack |

Each patch was applied to a disposable copy and compared byte-for-byte with its target pack.

The review inspected `b139221` and the local worktrees on 15 October 2026. It is source inspection, not a build or test result.
