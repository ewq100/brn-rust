# Remaining M4: model-proposed captured feedback

Selected from the supplied M4 requirement after the first reviewed feedback slice.
That slice already provides explicit owner selection and truthful separate Apply
outcomes. It does not yet let the candidate suggest which comments it addresses.
Bounded Luna tracing and Sol design inspection atb2904c2c identify the existing
writing-evidence path as sufficient; no classifier or new workflow is selected.
This specification selects the next bounded implementation after the current
combined candidate is published with a frozen PR head. It does not change that
candidate or grant a second build slot.

## Contract

Add optional `proposed_addressed_comment_ids` to existing `prepare_changes.writing`,
host-recorded `WritingCandidateEvidence` and checked `WritingCandidateReview`.
Omitted/empty means no suggestions. Bound to128 unique exact IDs in the immutable
request's captured comments. Unknown, repeated, cross-request or excluded IDs
refuse before operation allocation/run updates, both in whole-batch preflight and
direct prepare. Existing correction limits apply only to eligible argument errors;
authority refusals remain dominant. Validate persisted membership again at the
checked evidence boundary. Do not infer semantic correctness from membership.
Here excluded means absent from that immutable captured subset; later
next-Rewrite UI exclusions do not change the saved request or its membership.

Preserve host-only request/thread/chat identity. Include suggestions in immutable
prepare-retry comparison; changing them under the same request key refuses.
Serde default plus skip-empty serialization preserves old omitted-field evidence
bytes/hashes. New source reads old saved evidence without migration. Older source
rejecting a new nonempty field is a named downgrade limitation; do not claim
bidirectional compatibility or change snapshot envelope/schema versions.

Existing comment cards show a clearly model-reported suggestion, including its
accessible description. Owner checkboxes remain empty until explicitly clicked.
Suggestions never alter selection, exclusions, content-edit authority or comment
proofs. Scope/private/current-request verification precedes their display.
Owner-edited or selected replacement candidates clear suggestions: the model has
not assessed replacement text. Keep the original immutable evidence unchanged.
The existing core replacement guard currently requires the evidence Message data
to survive byte-for-byte. Extend only that checked owner-replacement path to
permit clearing this one field in the new evidence; preserve all other evidence
fields, bindings, inputs and the original bytes. A direct forged replacement
must not gain a general evidence-editing exception.

Current simple Rewrite already uses v2 with default setup, so advanced references
or requirements remain optional. Preserve persisted v1 retries without silently
upgrading them; they and ordinary non-Rewrite runs do not gain writing evidence.
Keep existing proposal-only Rewrite, stale/dirty/exact review, changed feedback,
replay, partial outcomes, proof Undo and scoped visibility guarantees.

## Verification and ownership

Use current tests and actual tool serialization: old absent/empty bytes and
hashes; subset prepare/restart/review; identical retry and changed-input refusal;
invalid mixed batch with zero effects; forged persisted evidence; rendered label
with unchecked selection; Apply-only without proofs versus explicit subset;
replacement clearing and candidate/note isolation; ordinary/v1/v2 schemas on both
provider transports. Strict Responses may normalize optional fields to required
on wire; verify it accepts the explicit empty list through the real pinned SDK
serialization rather than weakening the schema assertions.

One source worker may own the affected runtime/core evidence/app review/desktop
paths and their focused fixtures in an isolated checkout. One independent Sol
review covers complete source and boundary tests. Root owns shared docs,
integration and the single build slot. No database schema, dependency, CLI-result-contract
or live-provider change is selected. Source review, deterministic usefulness,
hosted/main and actual native observation stay separate. Stop and reassess an
unresolved compatibility or authority question before expanding this design.
