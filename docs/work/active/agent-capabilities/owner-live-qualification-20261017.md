# Owner decision L — live-provider qualification

17 October 2026. The owner explicitly authorizes live-provider testing in the principal BRN build thread. Exact message:

> i ll give the  Live-provider testing requires authorization now

The principal interprets this in the ongoing build context as affirmative approval of decision L: execute the existing plans' live-provider qualification using synthetic/public fixtures and explicitly selected supported provider/account/model/effort routes. This supersedes earlier dated statements that L is unresolved or that live qualification is unapproved. It does not claim that any live test has run or passed.

## Scope and execution

Use BRN's existing authenticated provider integrations and approved model allowlist; preserve explicit account/model/effort selection, route identity and fail-closed behavior. Existing configured credentials may be used through their normal checked authentication path for these tests. Do not log credentials, authentication payloads or account-private content. Do not substitute another account/provider/model silently. If a route or credentials are unavailable, retain the actual failure and qualification gap.

Follow the [Rig-first acceptance registry and semantic comparison method](proposal/amended/BRN_Rig_First_Implementation_Plan.md#9-acceptance-registry): the same source-labelled synthetic fixtures, model/account/effort and comparable repeated baseline/candidate runs; record requirement retention, evidence correctness, unsupported claims, completion, latency, input/output/cached usage and compactor/helper overhead. Distinguish actual measurements from estimates and unavailable provider counters. A single successful run does not establish superiority or universal semantic reliability. Deterministic authority/visibility/recovery checks remain required.

Run qualification against pinned source/artifacts with isolated synthetic workspaces and source-associated receipts. Coordinate the existing single heavy build slot; no concurrent conflicting builds or duplicate qualification agents. Preserve failed/interrupted attempts and subscription limitations. D3b remains in implementation, D4 still depends on accepted D3b; approval L does not skip those dependency or independent-review gates.

## Unchanged boundaries

This authorizes provider calls for the planned qualification, not real owner-data access, arbitrary external-data egress or model-chosen destinations. D-M remains limited to the approved opt-in read-only connection and scoped destination/tool rules; it does not grant broad connector access. T/PC and X/Xberg remain undecided, with Poppler retained. No publication, push, PR, main merge, release, model download or native-human/GUI authorization is added. D-S/D-M approvals remain as recorded in the [earlier decision receipt](owner-decisions-20261017.md).
