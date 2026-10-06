# Remaining V1 dependency map

2026-10-06; planning only, no roadmap resumption. [Status](../../../status.md) owns implemented/qualified state; [roadmap](../../../roadmap.md) and [Product Vision](../../../product/BRN_PRODUCT_VISION.md) own intended outcomes. Existing [Office plan](../office-inbox/plan.md), [text Inbox plan](../text-email-inbox/plan.md), [knowledge plan](../knowledge-foundations/plan.md) and [invariants](../../../architecture/invariants.md) remain authoritative inputs. H1/H2 specs are [here](next-specs.md); H3–H5 remain evaluations in the [queue](../v1-handoff.md#ordered-task-queue).

| Stage / state | Actual prerequisite and next specification gap |
| --- | --- |
| 5–7 foundations integrated; qualification incomplete | Native usability, multilingual/corpus search, live semantic behavior and owner acceptance need named acceptance tasks. Findings native queue/scheduled detection and deeper semantic qualification remain gaps; do not call synthetic results product acceptance. |
| 8 partial: bounded DOCX and one PNG integrated | H4 is evaluated (result PR pending): docx-rs0.4.22, rdocx0.15.0, office_oxide0.1.13 and betteroffice-docx-parse0.3.0 are not adopted, and broader DOCX extends BRN's own strict converter ([findings](../../../../experiments/docx-reader-eval/FINDINGS.md#next-stage8-acceptance)). Next, select one content/asset profile with explicit preserve/refuse/bounds and recovery witnesses. JPEG/multiple occurrences, charts/diagrams, PDF/PPTX and supplied URLs lack executable specs. Binary cleanup depends on a qualified meaningful-preservation proof and separate owner confirmation, not merely Source approval. |
| 9 web research | Provider native-web capability evidence from Stage3 is an input, not full attribution support. Specify evidence/citation capture, limits/cancellation and durable proposal path; depends on Source/provenance foundations. Supplied-URL capture may share this work only after a concrete common requirement. |
| 10 Needs Review/maintenance | Builds on current/history/conflict facts and Findings. Specify one detector→evidence→review outcome and freshness/repetition/closure policy; web-dependent detectors additionally need Stage9 evidence. Findings do not grant knowledge application authority. |
| 11 project/person views | Identity/links/Actions/current evidence exist. Specify one read-only context view and uncertain/missing references; full views need no graph canvas, maintenance refactor or H1–H5 dependency. |
| 12 bounded helpers | Uses shared AI/application lanes and approved tools. Specify delegation budget, evidence return, cancellation/joining and no independent durable write; first check Rig0.43.0 and companions. No new orchestration service implied. |
| 13 sessions/preferences | Existing sessions/timestamps provide foundations for proposed Archive/Restore, inactivity30days, warning before Delete and preservation of captured knowledge. Specify warning evidence and offline lifecycle; unresolved exact deletion/Undo/retention mechanics require a bounded spec, not guessed behavior. Preferences require separate exact approval. |
| 14 graph view | Derived relationships exist. Specify one view/navigation slice using existing queries/index; no graph datastore. Depends on relationship evidence, not all profile/maintenance work. |
| 15 vault cleanup | Existing proposals/identity/history underpin reviewable organization/metadata/supersession batches. Specify one bounded batch, ambiguous identity/refusal and Undo limits. Does not depend on graph; private-vault inspection/migration needs selection/authorization. |
| 16 trusted-user delivery | Needs selected V1 acceptance, packaging/upgrades/backup recovery and clean-machine evidence. Signing, distribution, accounts and model acquisition require their own authorization. Informational portability failures do not gate unrelated prep/maintenance. |

Detailed specs beyond H1/H2 would exceed evidence here. The table identifies future spec work, not ready implementation tickets. No new maintenance prerequisite is imposed on unrelated product outcomes.

## Recommended explicit selections

1. **H1**: small, settled visible defect; already pinned parser and test seam; best first task for a cross-agent handoff trial.
2. **H4 evaluation**: evaluated with non-adoption (2026-10-06; result PR pending). No conversion replacement follows. The next Stage8 slice still needs explicit selection.

H2 is also technically ready within its compatibility stop; H3/H5 are ready for bounded evaluation. H1/H4/H5 have disjoint product/evaluator files and may run in separate worktrees if each is selected; H2/H3 should be serialized. Lead alone integrates and updates shared status, ADR, queue and parent specs. Evaluators return their findings in task-owned records for lead reconciliation. Recheck overlap after fetching the actual baseline; use distinct Cargo targets and one Cargo process per target. The paused roadmap remains paused after any one task.
