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

## Implemented candidate and focused evidence

Productionf2ff215 is committed/pushed after one fresh independent complete review
of all22 tracked files, clean at binary patch
810237f8f73f6de6a14a7131e02965a7ab862089e75ff28de54937439c2a8a12.
The patch hash was rechecked before committing. PPTX PR98 merged normally at
4db30319662c97b272f3ba2e6877036d1906f8e0 with a tree exactly matching checked8b40385;
this branch incorporates that resulting main without production changes.

Passed:71 Store regressions plus final affected replay witness and strict Store
Clippy;13 default/14 native Undo state/capture/rendering checks;6 native repair
checks; native strict Clippy with/without test support; four workflow reference/
graph/crash/old-DB mirror tests; one real-process CLI preview/confirm/restart/replay
journey preserving later edits. Initial desktop failures demonstrated shared
vault-only admission; narrow state/native predicates now permit only supported
Action-only compensation while preserving file/scoped Trash/repair guards.
No provider or interactive UI work occurred. Full final gates, required CI,
integration and personal acceptance remain pending.

Full default gate passed all changed Store tests and other packages, then found
one existing mixed asset/Action witness expecting the old blanket diagnostic.
Both mixed Undo operations still correctly refuse with no admission or effects;
only the two expected strings changed to the precise supported-domain refusal.
Independent test-only delta review clean at
b410e9d3d1741764c145f8e6985ff423eb3d77f6ff9830395be953f71c9a1305.
Focused witness passed; unchanged421 workflow unit successes/full-size recovery
are reused with this corrected pass. Remaining integration/doctest/native/combined
and shipping gates continue serially in8195; production unchanged sincef2ff215.


Final standalone qualification passed: 1745 default coverage / 18 existing ignores
using unchanged successes plus the corrected focused unit and final integrations;
430 native workflow/models / 16 ignores; 555 combined desktop/CLI; all doctests,
strict default/combined Clippy, shipping, 52 fixtures and links. Backups merged
normally in PR99 at 544696834d93cf56b82e76a857f2c44cdca7823b and were incorporated
at e0282e26d1c939a50d7b2281ae868589f2f80a59. Only the morning-task wording conflicted;
production merged automatically. Fresh independent composition review and serial
combined final gate are active. Required Action CI/integration remain pending.

Fresh independent composition review found no actionable findings. Exact complete
Action diff against backup main has SHA-256
b8abe1f233acb85707e304e9334ce8540ef733600a9536b3852381856bbadead;
backup integration delta from 1f7f31d has SHA-256
a21d55217454d9c277c6941e38303106a89c46c17352ce5cb1313bfa1c87a304.
Lead rechecked both hashes; only reporting documentation changed afterward.


Final composed candidate8bf40d4 passed 1769 default / 18 existing ignores,
434 native workflow/models / 16, 565 combined desktop/CLI, doctests, strict Clippy,
shipping,52 fixtures and links. PR100 is attached; required run37858092859 active.
Immutable action-backup-runtime includes this behavior. WorkspaceG retains ready
native compensation and completed/changed refusal scenarios without inference;
all actual GUI observation remains pending in the single morning task.
