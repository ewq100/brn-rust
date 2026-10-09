# Choose the order of captured group approvals

Owner-authorized next small P4 review outcome, selected09October00:47UTC.
Reuse clean intake checkout branch codex/p4-captured-approval-order based raw
candidatec086849198fe503d3454e5855d98f62e05f3627b; integrate raw PR103 first.
Authoritative lead/morning task stays budget checkout until that merge; this is
not a separate UI checklist. No functional dependency on raw reading.

## Outcome and reuse

The CLI GroupApprovalRequest already preserves owner order among non-Source
members; pending Sources run first. Native captured group review currently only
selects members in inventory order. Allow owner to move captured consequences
earlier/later so Action-before-supersession is achievable in the same review.
This is approval ordering, not mutation of stored group/provenance identity.
Cross-folder retarget is deferred until its relative-reference eligibility policy
and original-parent replay evidence are selected; regroup/split have independent
provenance/comment semantics. Do not broaden this slice into those operations.

Reuse ApprovalCapture exact record/request pairs and operation UUIDs, select(),
AiState confirm_approval exact-current checks, native dialog/button/widget seams,
existing GroupApprovalRequest and stable Source-first workflow. No Store/schema/
provider/tool/dependency change, new orchestration or inference. Pinned Rig handles
AI tools but has no role in this explicit owner capture; existing GPUI widgets suffice.

## Fixed behavior and ownership

ApprovalCapture::new canonicalizes Sources first exactly as workflow (stable
sort key draft.inbox_source.is_none()), before operation IDs are minted.
Public move_earlier(id)/move_later(id) -> Option<Self> clone the existing capture
and swap the identified adjacent non-Source record/request pair only. Refuse
single captures, unknown IDs, boundaries and Source movement/crossing. Preserve
all full records, stamps, operation IDs, selection, group identity and exact bytes.
select() retains chosen order and existing prerequisite inclusion rules. command()
uses that exact order; receipt validation still matches the canonical request sequence.
No proposal review version changes solely from changing confirmation order.

Native group confirmation holds its captured order in one local Rc<RefCell<...>>;
render the ordered exact snapshots and persistent per-proposal selection, numbered
order and explicit earlier/later controls. Pending Sources visibly run first and
cannot move. Move callbacks rerender dialog; disabled while current work blocks
approval. Confirmation selects from the latest locally ordered capture, then
existing exact-current AiState checks; new arrivals never enter the capture.
Close/cancel discards transient order, not owner edits/proposals. No auto retry.

Helper owns only desktop approval.rs/native/approval.rs and narrowly required
new/existing desktop tests/README. Lead owns plan/checkpoint/morning task/review/
integration. Root soleCargo target/budgets for raw final gates; helper must not
run Cargo until explicit release. <=2activehelpers, no recursion; retain selected
lead model/effort. Stop/reassess on uncovered correctness/integrity issue, not
routine implementation choices. NoGUI/private data/credentials/live/purchases/
ports/release/globalconfig; synthetic tests only.

## Acceptance and checks

- Group A/B/C can become C/A/B and selection yields exact corresponding original
  request UUID/stamp/full record pairs; no minting or stored effects on reorder.
- Sources canonical first and pinned; source-dependent selection still requires
  every pending Source. Non-Source boundary moves, single/unknown/empty refuse.
- Actual command preserves owner order; exact group receipt prefix/stop validation
  accepts that sequence and rejects reordered/mismatched receipts.
- Native headless widget clicks reorder and show new numbered exact snapshots,
  selection follows ID across moves, then confirmation submits exact selected
  sequence once. Capture with late/current-changed review refuses; blocked current
  work preserves capture and never submits. RealGUI remains pending.
- Existing Action-before-supersession and Source-first workflow witnesses are
  reused unchanged; add a missing relevant behavioral witness if necessary.
- Focused desktop state/real headless widget checks, broader changed default and
  combined desktop/CLI, strictaffectedClippy/shipping, fixtures/links, one fresh
  complete independent read-only review, actual requiredCI, normal protectedmerge
  and resulting-main verification. Reuse unchanged Store/recovery evidence.
