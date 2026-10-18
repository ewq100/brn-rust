# Reuse and retention checkpoint

17 October 2026; current source is base000ed920 plus the source-associated WIP recorded by each command, not yet an immutable reviewed pin.

The [R0 proof](../../../../../experiments/r0-rig-reuse/REPORT.md) established Rig0.44's normal hook-aware runner and append-only request contract. D3b uses the existing AgentBuilder/tools/BatchHook, shared request/result BudgetHook and authenticated provider construction. It adds no driver, registry, checkpoint format or provider route. The parent validates a complete batch's schemas and frozen skill ID/file before any tool effect. The helper's existing allowed-tool list remains independent; its BatchHook has skills:None and its role cannot read skills. D1A budgets, D1B frozen summary and D2 continuation remain the single production mechanisms.

Built-in IDs/text/preloads remain packaged. Metadata comes from each parsed SKILL.md. The replaced hard-coded descriptions and packaged-only production preamble renderer are removed; tests use the same frozen catalogue renderer. Installed instructions are JSON-escaped untrusted user context, not new system or write authority. No instruction can install, enable a tool/connector, read outside checked scope or inherit protected grants.

## Canonical versus local recovery

Schema8 records the changed policy/fencing protocol. Canonical Settings scope/ID/digest/enablement/selection and Run delivered file identities are SQLite data and included by the existing snapshot mechanism. Exact local skills/ package bytes remain machine-local instruction configuration excluded from SQLite backup and Markdown export; preserve original sources separately. No backup format, arbitrary-folder inclusion or source lookup is introduced.

Actual app backup/restore tests retain canonical work plus exact selected policy, expose missing local bytes as unavailable, refuse a different package digest and allow explicit exact reinstall. The actual CLI process witness additionally asserts the safe unavailable/failed envelope and zero localhost authentication/summary/chat requests. Permitted delivered provenance survives removal; the shared app projection filters by current scope before the16-entry bound and visible total/omitted counts, so moving a chat hides former Project entries.

The actual schema7 binary from000ed920 and new WIP binary are separately built/source-bound. schema8-compatibility.json retains explicit upgrade refusal, verified external snapshot, unchanged canonical table values and unpublished restore-copy upgrade. The initial mode=ro inspector sidecar effect and corrected immutable=1 comparison remain qualified to that WIP binary; these are not fabricated final-pin executions. Original schema7 snapshot remains7 and old binary refuses8.

## Qualification limits

Installation/owner management inventory and scoped runtime catalogue are distinct surfaces. Runtime never enumerates installation inventory. Project policy, instructions, references and model catalogue use the exact current Project; general installed opt-in does not implicitly expand Project context. No D4 work is included.

Owner L is now approved in the [principal decision](../owner-live-qualification-20261017.md). Deterministic/source-pin checks precede explicit selected-route live fixture preparation. No live result or semantic superiority is claimed here. Native synthetic state checks are distinct from native-human observation; hosted, owner/full-program, integration and carried Poppler PDF qualification remain separate.
