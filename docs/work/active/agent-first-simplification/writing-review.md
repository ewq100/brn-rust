# M4 instruction consistency and selected feedback review

Selected owner plan M4 uses the existing shared run/authority and exact review
owners. Bounded Luna caller research and independent Sol design inspection at
19efe879 found ThreadRunStart → prepare_thread_run → start_request already carries
host-owned OwnerRequest for ordinary chat, contextual Edit note and Rewrite. No
new action DTO or permission classifier has a demonstrated benefit. Preserve those
routes and understandable ask/edit/proposal wording; prove relevant existing
explicitly authorized target/effect and Rewrite boundaries instead of granting
writes from selection or passage comments. Ordinary chat's None and explicit
EditNote/Rewrite are deliberately different authority inputs.

## Exact review and explicit comment selection

Extend the existing ReviewLifecycle/Review, native rendering and checked review
operations. Display only the verified writing snapshot's captured comment quote
and instruction, with unchecked owner selection separate from proposal groups and
next-rewrite exclusions. No model addressed-comment output exists today; do not
build a semantic classifier or silently select everything. The owner chooses the
exact displayed subset. Preserve the existing per-comment post-Apply action.

Selection belongs to the exact immutable candidate and note. Reopening the same
candidate preserves it; replacement/prepared edited text/groups, switching note,
closing/dismissal/completion clear it. Another candidate card never inherits it.
Dirty draft/partial-group review guards remain. Before Apply, capture operation,
writing-request identity and sorted selected IDs. Empty means Apply only.

Apply the exact reviewed candidate through the current checked owner. Only an
actual Applied receipt permits the existing mark_writing_comments_addressed for
the captured subset. Never resolve on stale/dirty/NeedsReview/unknown failure.
Use a stable resolution identity bound to candidate/request/sorted subset for
identical retries; changed selection is a separate explicit owner action. Existing
proof/version/scope/Undo/receipt checks remain authoritative. Restart reconstructs
committed outcomes, never an uncommitted transient selection.

## Truthful partial outcomes

Current review_apply commits content and then can return Err from attention
cleanup; desktop currently treats any Err as total failure and skips reload and
applied-rewrite state. Preserve the actual Apply outcome/receipt separately from
cleanup status at the existing shared review owner. Carry comment confirmation
status separately too. Prefer a small useful result at that owner, reconcile all
real callers and retire the replaced ambiguity; no transaction framework or schema.

Known committed content stays visibly applied/reloaded and Undo remains available
if attention cleanup or comment confirmation fails. Report revision applied and
which status update remains pending; preserve exact operation identity for retry.
Do not rerun generation or misreport committed Apply as failed. Reading an old
receipt to explain an outcome must still respect current visibility and must not
authorize a new resolution after context inspection failed.

## Reopen, history and checks

The current addressed proof is valid only for captured unchanged feedback and its
actual un-undone Apply. Workspace reopen rebuilds it; Undo of a proof reopens its
subset, Undo of the rewrite invalidates its proofs. Preserve original context and
unresolved anchors. No new direct Reopen control is necessary in this slice.

Required evidence uses existing lifecycle/app/native seams: same-operation state
preservation, clearing/no leakage, Apply-only and selected subset, changed feedback
and stale/dirty/exact review refusal, stable replay/restart/Undo, and truthful
content-committed/status-failed outcomes. Include an actual rendered selector and
Apply click with saved bytes and selected-only proofs; helper calls alone do not
prove wiring. Original UI lacks these controls; do not label new-API compilation
failure as a semantic baseline. Preserve original failures, exact commands/source
and limits. Use a deterministic shared cleanup failure seam, not a timing race.
Keep existing explicitly authorized target/effect and frozen Rewrite
retry/Continue/replacement tests. Queue one actual Mac writing journey separately.

One requested Sol high worker may own brn-writing-ui review/native/writing,
desktop apply/feedback rendering tests, and shared app review/result tests; notify
lead about public interface changes. M3 separately owns brn-ai/agent source.
No builds until the single shared slot is granted. One integration/docs lead;
no provider/private-data/live/native/merge/release grants follow. Reassess after
repeated failed approaches or a consequential unresolved invariant. Source review,
deterministic qualification, real-model usefulness, native observation, owner
acceptance and hosted/main integration remain separate gates.


## Current implementation and separate remaining item

Source3b3f9135 is independently approved and matches the integrated lead's15
changed files. Shared-result fault2, affected app52/0/1, native332/0/1, writing
UI44 plus development-host12 and CLI43/0 pass; strict affected lint and workspace
format pass. The old compatible rendered selector witness fails, and its candidate
counterpart passes. Original small-window, import-order and missing-installed-helper
failures are retained in the [qualification receipt](../vacation-campaign/evidence/writing-review-qualification.json).
Combined7a970 runtime518/native332/CLI43, strict affected lint/format and production build also pass; see the [final receipt](../vacation-campaign/evidence/agent-first-final-qualification.json). Hosted/main and actual Mac gates remain separate.

The owner plan's model-proposed addressed-comment subset is still pending. Current
source implements explicit owner selection, with no automatic proposal indication.
The independently reviewed [next specification](proposed-feedback.md) targets the existing writing-evidence payload and review
DTO rather than a classifier or new workflow. Model suggestions must never
initialize owner selection or create resolution authority. Preserve ordinary v1
writing, old saved evidence, request/candidate replay binding and changed-feedback
refusal. Do not declare all M4 scope complete before this item is accounted for.
