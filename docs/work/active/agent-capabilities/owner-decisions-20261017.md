# Owner decisions D-S and D-M — 17 October 2026

The owner explicitly answered **Approve D-S** and **Approve D-M** in the principal orchestration thread on 17 October 2026. These are affirmative scope decisions, not inferred permission from silence. This record supersedes earlier unresolved D-S/D-M status paragraphs without rewriting their historical evidence.

## D-S: instruction-only skill packages

Approved question: “may I start D3—owner-installed instruction-only skill packages using the bounded manifest format, path confinement and explicit enablement already specified in the approved plan?”

D3 may start in the authoritative dependency order: D3a parse/validate/install/list/remove, then D3b loading, frozen per-run catalogue and settings/composer integration. Preserve the four built-ins and their IDs. Use the C0 strict manifest subset and existing checked application/runtime boundaries. Owner installation and explicit scope enablement remain distinct from model-led lazy loading. Instruction/reference files cannot execute, self-install, enable connectors or grant authority.

The parser remains the C0 decision: bounded UTF-8 files/header, exactly the single-line name and description keys, strict validation and unsupported-shape rejection. It is not YAML and adds no parser dependency or lockfile change. Full D3 acceptance S01–S04 includes actual installation/loading/reopen, containment/collision refusals, frozen version/revocation behavior and project scope/injection checks. D3a alone does not claim D3b runtime/UI acceptance.

## D-M: one opt-in read-only MCP connection

Approved question: “may I start D4—one opt-in, read-only MCP connection with explicit destination/tool allowlists and synthetic localhost verification? Live-provider qualification and real-data access would remain unapproved.”

D4 is authorized only after its actual prerequisites, including D3b now that D3 has started. Use the existing Rig/rmcp adapter and the chosen single streamable HTTP transport; keep child-process transport, broad connectors and connector writes deferred. One disabled-by-default owner-configured connection, destination/tool/schema validation and scoped external-data grants remain required. A read-only hint is not an enforcement or no-side-effect guarantee. Cancellation/disconnect must not imply that remote work was undone.

Synthetic localhost qualification is permitted. This approval does not authorize live-provider/account qualification, real owner data, arbitrary executable servers, credentials in evidence or model-chosen endpoints.

## Boundaries and next action

The [canonical target](../../../architecture/threads-target.md) carries dated amendments. [Build ordering](../build-sequence/README.md) and [the amended Rig-first plan](proposal/amended/BRN_Rig_First_Implementation_Plan.md) retain the original requirements and acceptance registry.

D2 is locally accepted at fd4e6a366171ddf306829aceb50658000b86bca4. D3a is the next eligible implementation on a new local descendant branch; D3b and D4 must not be bundled into that slice. O1 remains CW-first. T/PC, X/Xberg and L/live qualification are unchanged. No push, PR, main merge, release, native-human or owner-data authority is added. No implementation or test pass is claimed by this documentation record.
