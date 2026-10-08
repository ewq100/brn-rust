# Compensate an unchanged approved Action replacement

Selected next bounded P7 outcome after operational backups. An owner who approved
incorrect Action details can review and restore their earlier values when the
installed record remains exact. Restoring details creates a new revision and
preserves immutable origin and completion history; it never rolls a revision back.
Action creation Undo and mixed file/Action Undo stay unsupported in this slice.
No silently selected subset, new deletion meaning or reopening Completed work.

## Reuse decision and scope

Read-only readiness inspection found the existing Undo request/preview/binding,
ApplyJournal action_records, original Replace.before, monotonic after-record
builder, exact-record CAS, atomic receipt/terminal mirror and recovery paths
sufficient. Adapt those paths; no schema, DTO, framework or second journal.
The native draft body already renders full Action before/after. Only its capture
admission currently assumes nonempty file members. Existing CLI/AppWorker Undo
commands remain the shared client boundary.

For an Applied Action-only operation containing only Replace members, derive each
inverse Replace with the exact installed source.action_records entry as `before`
and the original member's before.data as candidate. Keep the original ActionOrigin;
build revision N+1 at a clock no earlier than any exact baseline. Unchanged Waiting
keeps its current waiting clock; re-entering Waiting starts at compensation time,
using existing replacement semantics. Preview changes nothing and is historical;
confirmation freshly validates all exact records and restored references.

Store Undo admission must reuse transactional actions::check_changes and the
existing action_records builder. Workflow preflight must permit Action-only work
without a file adapter and still call existing Action reference/graph validation.
The file-only historical Undo exemption must not permit restored Action dependency
cycles or invalid note references. Conservatively require current eligibility for
newly reintroduced note references. Existing current-evidence/recovery fences stay.

## Acceptance criteria

1. Exact complete prior candidate and current installed record are reviewable;
   preview has no effects. Confirmed compensation restores data at N+1 with the
   same immutable origin and documented Waiting clocks, then persists on restart.
2. Missing, changed, newer, equal-version-forked or Completed records refuse before
   admission. Reintroduced missing references or dependency/parent cycles refuse.
   Unsupported Create/mixed operations and scoped Trash on Actions remain refused.
3. Exact UUID replay returns the recorded receipt without overwriting later edits
   or completion. Same UUID with another source/scope conflicts. Bounds remain
   enforced; oversized inverse refuses whole, never truncates.
4. Interrupted vaultless compensation without terminal authority becomes
   NotApplied; empty file proofs cannot establish Applied. Existing terminal mirror
   recovery reconstructs complete compensation after old-DB restoration without
   rerunning writes or requiring the original application journal.
5. Native state captures and displays complete supported Action-only preview;
   stale generation/receipt mismatch refuses. CLI and AppWorker exercise one
   preview/confirm/restart/replay journey. All actual interactive observation stays
   pending in the single morning task.

Lead retains selected model/effort and owns integration. At most two active bounded
helpers without recursion; one may own Store inverse/validation/tests behind fixed
interfaces, another desktop state/tests while lead owns workflow/recovery/CLI/docs.
One Cargo across checkouts. No inference for compensation/replay, computer/GUI use,
private data/credentials, downloads, resets, unrelated work, ports or release.
One independent complete read-only review, applicable final checks, actual required
CI, normal protected integration and resulting-main verification are required.
Reassess only demonstrated integrity blockers or an unresolved consequential
product decision; keep unsupported deletion/mixed semantics parked and continue
safe work. Implementation branch `codex/p7-action-replacement-compensation` reuses the lead
checkout, based on fully locally qualified PPTX candidate
8b403853bd5acab24a0efccca1cb08e120deae96 (PR98 required CI still active). This
Action slice is independent of pending backups in the other checkout; normal
integration will incorporate resulting main and keep strict up-to-date checks.
Store helper owns proposal_undo.rs/proposal_apply.rs private reuse and Store tests;
desktop helper owns approval.rs/Undo state/native wording/tests. Lead owns workflow
preflight/reference/recovery, public worker/CLI tests and shared docs/integration.
Neither helper may run Cargo until explicit grant after backup gate12095 releases.
