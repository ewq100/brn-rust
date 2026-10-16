# Modular foundation and reference-assisted writing — proposal

**Status: proposed, not started.** This folder stores the reviewed work pack for later use. It is not an active assignment and grants no implementation, merge or release authority. [Status](../../../status.md) remains the source of truth for current work.

**Owner decision, 15 October 2026:** `work/ui-polish` and `work/daily-workflow-reliability` are finished and merged into `main` first. C0 and all later slices start only after both have landed. **16 October:** `work/cli-agent-contract` also lands first.

**Sequencing now lives in the [forward build sequence](../build-sequence/README.md).** It orders this program together with the [agent-capabilities](../agent-capabilities/README.md) program under one integration lead and a combined C0. Where it differs from the stored pack, it wins.

## What to use

| File | Use |
|---|---|
| [proposal/final/00-start-here.md](proposal/final/00-start-here.md) | Pack overview. Its C0 bootstrap message is superseded by the forward build sequence's combined C0 message |
| [proposal/final/01-master-plan.md](proposal/final/01-master-plan.md) | Reconciled master plan (scope, contracts, slices, F01–F22 registry, milestone and program status) |
| [proposal/final/](proposal/final/) | All seven reconciled pack files (role prompts 02–04, review prompt 05, native outline 06) |

When the owner starts C0, the lead installs the single active `plan.md` in this folder (master plan §6) and links it from status.

## Experiment results

- [X1 — Xberg PDF qualification](experiments/x1-xberg.md): gates 1–2 passed; gate 3 written design delivered. Bounded adaptation, not adopted; production unchanged.
- [Z1 — Zorite editor qualification](experiments/z1-zorite.md): gate 1 passed, gate 2 failed. Not adopted; current editor retained. A follow-up positioning prototype on the current viewer is preserved as draft #149 (not to be merged).

These record the owner's isolated experiment override. They do not start C0.

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
