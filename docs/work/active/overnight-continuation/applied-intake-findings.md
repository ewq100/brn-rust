# Conflict Findings from an exact Applied intake Source

## Selected outcome and baseline

Baseline merged P4 `bb66a1b884106fd18b08d2fa4864aff0537811bc` (PR92).
Reused clean budget checkout `/Users/evokessler/repos/brn-p3-work-budgets`, branch
`codex/p4-applied-intake-findings`. The separate predecessor candidate is being
qualified in the other checkout and is not a dependency of this slice.

Private-intake investigations currently reject formal conflict capture even after
their exact Source prerequisite is Applied. Support the existing owned
KnowledgeAndActions `report_conflict` operation for that case. Pending/private
Sources still cannot supply durable saved Finding evidence. Require Applied
eligibility when a fresh Finding is captured; no new admission-time claim.

## Reuse and exact authority

Reuse FindingOrigin::InboxConflict, its analysis_id and existing FindingEvidence
SourceVersion. That analysis reaches the unchanged canonical intake binding,
snapshot, Source proposal stamp and text/hash. No new origin/schema/envelope,
capture/question mutation, model runtime or semantic authority rule is needed.

Add a shared Store resolver `inbox_conflict_source(analysis_id) -> SourceVersion`.
Saved-Source captures retain their original proof. Intake captures require exactly
one historical Source approval journal matching the bound Draft stamp and an
Applied receipt. Validate its Source Create, snapshot, path, UUID, complete text
and capture; derive the checked installed fingerprint from prepared[0]. Never
replace it with a freshly observed proof, or infer approval from equal bytes.
Missing/ambiguous/incomplete/mismatched receipt lineage refuses.

Fresh workflow capture requires existing `validate_intake_dependency(..., true)`,
then complete observed Source fingerprint **and bytes** equal the retained Applied
proof. Existing opposing evidence scope, identity, exact quote and retention
checks remain. Update private-intake task guidance to permit this only with the
exact Applied Source prerequisite. Do not claim pending intake has a saved Source.

Store historical Finding reads validate against the historical Applied journal,
not the Source proposal's current eligibility/state. Later Source edits/loss/Undo
must leave retained Finding evidence inspectable and explicit closure possible.
Original exact callback replay returns retained work before fresh checks. Backups
already preserve all required canonical rows and journals; do not manufacture a
Finding or missing authority from an approval-recovery mirror.

Legacy original-cleanup certificate validation still explicitly rejects private
intake. Keep that refusal unchanged. Supporting private-intake cleanup lineage is
separate work, and binary cleanup remains unsupported. Older readers already
refuse newly supported intake-backed Findings semantically, rather than accepting
weaker evidence silently; preserve byte compatibility for old saved-Source cases.

## Acceptance and checks

- Applied retained Source creates an exact two-sided Finding with unchanged
  canonical capture/question, no Knowledge/Action effects and no inference for replay.
- Pending, wrong stamp, missing/ambiguous/incomplete Applied journal, changed inode
  with equal bytes, changed original/asset and ambiguous identity refuse insertion.
- Closed Finding exact replay and historical inspection survive Source loss and
  Undo; restart/backup retain proof and changed/unavailable observations.
- Existing saved-Source Finding and cleanup certificate fixtures remain compatible;
  private-intake cleanup still refuses explicitly.
- Real worker/MIME/App tests plus Store lineage/backup tests, affected guidance
  transport/context tests, independent complete-candidate read-only review, final
  applicable local gates, actual required CI, normal protected integration.
- Paired live Medium comparison may use the existing retained North Quay private
  binding only after qualification, within the amended shared ledger. No repeat
  inference for approval/replay. All GUI observations go in the one morning task.

## Ownership

Bounded implementation helper owns Store/workflow/guidance code and behavioral
checks in this checkout, with no recursive delegation, live calls, GUI or Git
integration. Lead owns coherent review, final checks and integration. Cargo slots
are serial across the active overnight work; request a slot before starting Cargo.
