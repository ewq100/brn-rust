# Delivery roadmap

The owner has frozen the reviewed product architecture as of 2026-10-03. [Product vision](product/BRN_PRODUCT_VISION.md) and [architecture/invariants](architecture/invariants.md) govern the outcomes below. This sequence carries forward the dependency corrections in the dated [independent review](audits/BRN_PRODUCT_ARCHITECTURE_REVIEW.md#g-final-build-sequence). The current v1 mission authorizes sequential implementation and integration within these frozen boundaries; each slice still establishes its own acceptance criteria and verification.

[Status](status.md) owns observed implementation, verification, acceptance and integration. Older milestones and the simple-notes Steps 5/6 are historical; they do not select the next task. Keep existing foundations and data while following the stable V1 core [target](architecture/overview.md#frozen-target).

The owner's 2026-10-04 external-agent amendment preserves this sequence. Every
meaningful domain capability stays headless through workflow/AppWorker. Thin
future protocol adapters are permitted; read-only local stdio MCP is the expected
first external interface. MCP, a daemon and remote/network infrastructure are not
added delivery stages or prerequisites for V1.

## Reviewed outcomes

| Stage | Outcome |
| --- | --- |
| 0 | Retire conflicting guidance and use the frozen product/architecture with the concise development workflow. |
| 1 | Simple manual Save preserves bytes, detects conflicts, avoids overwriting new destinations and recovers unfinished edits. Verify Save/recovery before removal. |
| 2 | Remove legacy production paths and obsolete tests while preserving historical records, existing data and trial workspaces. Export valuable legacy data only when its need is established and authorized. |
| 3 | Narrow capability spikes establish selected-model, effort, retry, native web and image support on actual provider routes. Live checks need separate task authorization. |
| 4 | Complete typed proposals support editing, temporary comments, Rewrite, individual/group approval, recoverable application, activity and practical Undo/Trash. |
| 5 | Minimal note identities, durable provenance, current/history retrieval, relationship extraction and qualified multilingual search. Basic review findings and session timestamps support later work. |
| 6 | Actions/dashboard support approved creation, direct explicit completion and new related follow-up actions. |
| 7 | Text/email Inbox produces independently reviewable consequences, using identity foundations and a small owned AI queue. |
| 8 | Office documents and supplied URLs preserve meaningful content/assets; incomplete conversion retains originals. |
| 9 | Autonomous web research returns attributable evidence and proposes approved durable captures. |
| 10 | Needs Review and maintenance handle stale knowledge, conflicts and neglected work, including web-dependent checks. |
| 11 | Project/person views assemble current context from existing records. |
| 12 | Bounded helper agents extend investigation without independent durable writes. |
| 13 | Session Archive/Restore/Delete, capture warnings before Delete and approved working preferences complete session lifecycle. |
| 14 | A graph view exposes existing derived relationships without a new graph datastore. |
| 15 | Existing-vault cleanup proposes reviewable metadata, organization and supersession batches. |
| 16 | Trusted-user packaging and upgrade/recovery qualification complete delivery. |

Safe Save and verified recovery precede legacy removal; removal precedes Proposal Core. Identity foundations precede linked actions and Inbox. Full profile views can follow those foundations. Vault cleanup does not depend on the graph canvas and may move earlier when the owner selects it. Automatic session archive remains reversible and offline.

## Starting the next slice

Select the next incomplete dependency from [status](status.md), establishing the
actual merged baseline and its pending qualification. Continue sequential
reviewable slices under the current V1 authorization. Pending owner acceptance
need not block later safe work when it is not a dependency. Original-data
inspection/migration, additional live calls, downloads, purchases and release or
public distribution still require applicable owner permission. Use the
[development workflow](development/workflow.md).

Each slice needs relevant offline verification and acceptance for its changed user-visible behavior. Native usability, actual inference, provider validity and packaging remain separate qualification. Preserve useful completed work as [history](work/completed/README.md); old specifications and model/process assignments are not current instructions.
