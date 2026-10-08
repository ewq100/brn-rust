# Explicit predecessor attachment to an Inbox Knowledge draft

## Baseline and selected outcome

Baseline is P4 candidate `354fd35`, stacked on merged main `409f68b`.
A retained supplemental Inbox Knowledge draft cannot currently be converted into
an explicit replacement while keeping its identity, owner edits and comments.
The owner may select one saved Current predecessor, see the complete revised
successor + protected History pair and approve that exact revised version.
This operation does not decide semantic replacement or close a Finding.
Changing/removing an existing predecessor and general split/regroup are separate.

## Reuse decision and contract

Reuse existing InboxSupersedesBinding, Previous-version link, History conversion,
source/identity validators, versioned ProposalRecord and atomic Store transactions.
Existing ApplyJournal and capture already represent the complete two-member pair;
no schema/runtime/dependency/recovery-envelope extension is required. Existing
ProposalEdit intentionally cannot change shape; add a dedicated host command,
not a generic mutable proposal engine or a model-owned authority operation.

Strict request `KnowledgePredecessorRequest { expected: ProposalStamp,
predecessor_path: String }`; AppCommand::AttachInboxKnowledgePredecessor returns
the complete existing Proposal event. CLI `proposals attach-predecessor` takes
that JSON request. Native review exposes a separate Current predecessor path and
clearly explains that approval makes that note History. Require acknowledged,
clean review before dispatch, fence local typing/navigation while pending and
validate the exact structural acknowledgement. Preserve conflicting local text
and reject unrelated/late events. Generated History must be readonly.

Only a Draft Inbox Knowledge proposal with one Create and no supersedes is
eligible. Preserve proposal/note UUID, destination and parent proof, session/group,
title, owner text prefix, comments, citations and Source/intake evidence. Require
one separate, uniquely identified saved Current knowledge predecessor. Capture
its complete proof; if already retained as ordinary context, require the same
proof and promote it without refreshing any captured binding. Revalidate existing
sources/destination and Source/intake binding before changing the draft. Reject
stale, ambiguous, hidden/invalid, occupied, oversized or malformed inputs.
Append the existing readable Previous-version footer and exact History Replace,
insert/promote predecessor proof in the required position and advance once in one
transaction. No vault effect before approval; old approval stamp must refuse.

Creation replay must reconstruct the original one-Create input from its original
callback arguments and retained exact proofs, omit the attached member for that
original creation-hash check and keep `creation_sha256` unchanged. Replaying the
original callback after owner changes/restart/Source loss must return the current
record, never observe fresh files, invent a new original payload or bypass the
hash check. Ordinary same-shape creation replay remains unchanged.

## Behavioral acceptance and qualification

- Attach after owner text edits and anchored comments; preserve exact prefix,
  identities, bindings and original creation hash; bump once and refuse old stamp.
- Promote existing retained context unchanged; stale context/Source/predecessor,
  ambiguity, invalid target, hidden footer, occupied destination and size limits
  refuse without changing the draft or vault.
- Original creation callback replays before/after further owner edits, restart and
  Source loss; changed original payload still conflicts.
- Approve revised pair, restart/rebuild links and exercise an existing-format
  revised approval recovery witness. Exact before/after and protected History hold.
- Native pending/late-event/local-text protection, successor editable/History
  readonly, and strict CLI JSON/subprocess roundtrip pass headlessly.
- One independent complete-candidate read-only review, applicable final tests,
  required CI and normal protected integration. No overnight GUI acceptance.

## Ownership and next action

Lead owns workflow capture/replay, command routing, CLI, integrated tests and
integration. Store helper owns the bounded atomic transition and pure transition
validator/tests. Desktop helper owns native review/acknowledgement and headless
presentation tests. Helpers do not spawn, run live inference, use GUI or integrate.
Cargo is coordinated serially. All interactive checks extend the single morning
UI task. Retained outputs can qualify owner revision without additional inference.
