**Current snapshot, 18 October 2026:** the locally accepted foundation/writing and agent-capability source lines are combined with the accepted standard-default Xberg component and inspected main `57c710cc` on `work/final-combined-20261018`. Combined verification and new independent whole-candidate review are in progress; no program PR/main integration, native-human or owner acceptance is claimed. D-S/D-M were approved and D3/D4/INT-D are implemented. The [approved execution amendment](../modular-foundation/x2/standard-default/final-integration-amendment.md) governs this assignment. Earlier proposal/sequence statements below are historical and assign no new work.

# Agent capabilities (Rig-first) — proposal

**Status: proposed, not started.** This folder stores the reviewed plan for later use. It is not an active assignment and grants no implementation, merge or release authority. [Status](../../../status.md) remains the source of truth. Sequencing lives in the [forward build sequence](../build-sequence/README.md): this program is workstream D under the single integration lead and shares C0 with the [modular-foundation](../modular-foundation/README.md) program.

**Scope:** improve BRN's existing chat agent in place:

- budgeted long-conversation context with a persistent semantic summary (Slices 1A/1B);
- recoverable multi-step work (Slice 2);
- owner-installable instruction skills (Slice 3; needs owner decision D-S);
- one opt-in, read-only MCP client connection (Slice 4; needs owner decision D-M).

Rig 0.44.0 facilities are reused wherever they fit BRN's contracts.

## What to use

| File | Use |
|---|---|
| [proposal/amended/BRN_Rig_First_Implementation_Plan.md](proposal/amended/BRN_Rig_First_Implementation_Plan.md) | The plan to execute, with review amendments A1–A9 applied |
| [proposal/REVIEW.md](proposal/REVIEW.md) | Review verdict, verified claims, findings F1–F12 and amendments |

When the integration lead runs the combined C0, it installs the single active `plan.md` in this folder (amended plan, section 4 exit) and links it from status.

## Provenance

| Path | Contents |
|---|---|
| `proposal/original/` | Plan as supplied (prepared 15 October 2026 against `7e9d0376`), unchanged |
| `proposal/amendments.patch` | Unified diff from the original to the amended plan |

The patch was applied to a disposable copy of the original and compared byte-for-byte with the amended plan. The review is source inspection plus public package metadata, not a build or test result.
