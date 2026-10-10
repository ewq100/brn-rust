# Threads core

This final-use crate owns fresh BRN Threads SQLite state. It has no dependency on
legacy storage, intake, inference, native UI, or retrieval. Small retained assets
live inside SQLite; originals remain external source locators.

## Host and application contract

`Store::open(absolute_directory)` requires an explicit, isolated directory. It
refuses nonempty unmarked directories, foreign files/markers, symlink ancestors,
and mismatched SQLite application/schema IDs. On macOS canonicalize OS aliases
such as `/var` before choosing a synthetic test directory. The database filename
is `threads.sqlite3`; marker `.brn-threads-data` identifies this directory.

1. The trusted host calls `allocate_operation(stable_host_request_key)`. The
   allocation commits before returning. A restart using the same host key returns
   the same identity. `operation(id)` verifies an existing identity.
2. Assign each creation its stable `operation.creation_id(index)`. Construct a
   typed `ChangeRequest` with exact `Put` results, expected versions, meaningful
   reason, and separately recorded derivation inputs.
3. `prepare(operation, request)` commits an immutable candidate and SHA-256 of its
   deterministic typed JSON representation. Repeated identical preparation
   replays; changed preparation is rejected. Only one pending note candidate may
   exist. `retire(operation)` releases that slot while retaining its history.
4. `apply(operation, host_authority)` returns `Applied(receipt)`, `Deferred`,
   `Stale`, `NeedsReview`, or `Superseded`. An existing receipt replays before
   mutable checks; a grant bound to another operation cannot replay it. New
   writes acquire `BEGIN IMMEDIATE`, then recheck all expected versions, exact
   protected-action scope, run fences, persisted edit guards, and typed references.
   Every group either writes all its members plus one receipt or writes none.

`prepare_owner(operation, request, authority)` requires direct host Owner scope.
It bypasses only pending-candidate slot reservation, using the same immutable
candidate and checked apply boundary. It supports direct protection, archive, or
metadata actions while a proposal exists; the proposal remains visibly stale.
Owner Save likewise commits through that same checked boundary.

`HostAuthority` has private fields and deliberately no deserializer. Owner and
maintenance constructors are host entry points, not model tools. A specific
instruction binds an operation, concrete protected targets, and capabilities;
links/mentions cannot expand it. Capabilities separately cover content, protect,
unprotect, archive, supersede, confirmation, commitment, human completion, and
Undo. A content grant cannot archive/unprotect/supersede. Ordinary maintenance
remains available when an instruction authorizes a narrower protected target.
`runtime(actor, run_id)` supplies host-only ManageRun authority for precisely
one Run record; maintenance and instruction grants cannot create or edit runs.
Run control changes must advance a monotonic fence. A terminal Run cannot return
to Working; continuation creates a new Run. Undo explicitly rejects Run creation,
fence, lifecycle, or configuration control changes, preserving cancellation.
`for_run(run_id, fence)` binds host authority to a Working run, matching fence,
and Open thread. The host increments the fence for steering/cancellation; stale
invocations cannot commit. Completed-operation replay still works after a fence
changes. The host must not expose authority constructors to inference inputs.

## Editor, history, and recovery

Call `begin_edit(note, base_version)` at the first dirty transition, then
`update_buffer(session, increasing_generation, text)`. The guard and buffer
survive restart. Starting from a stale revision preserves a recoverable buffer.
`save(session, generation)` checks base and generation under the writer lock;
`discard(session, generation)` closes only that generation. Closed session rows
retain the ordering tombstone, so delayed recovery writes cannot resurrect a
guard. Delayed generations cannot overwrite newer typing. Successful Save retry
returns its receipt; stale Save preserves the user's buffer.

`undo(compensation_operation, original_operation, authority)` constructs an
immutable atomic compensation from the original receipt's actual write set. It
checks precisely the resulting versions of those records. A new unrelated link
cannot block note Undo; a later edit to any written record does. Newly created
records are archived, preserving revisions/provenance. Receipts list directly
recorded derived records that need refresh. No recursive dependency solver or
external-original recovery is implied.

`records`, `record`, `note`, `revision`, `prepared`, `receipt`, `receipts`,
`candidates` (unapplied, unretired only), and
`recovery_buffers` expose typed current/history/recovery state. Note versions
cover content, title, protection, supersession, and archive; links, messages,
comments, source relationships, and assets have independent records. Every asset
creation, update, or archive must include its owning Note in the same atomic
group. Ownership cannot be reassigned. This advances the Note version, enforces
its protection and editing guard, and includes it in the Undo write set. Comment
original byte ranges are checked against their original UTF-8 revision and quote.
Source locators have exact deterministic uniqueness. Full imports begin protected;
import/protection does not confirm their claims. Clearing obsolete confirmation
as part of a content revision needs no redundant confirmation permission; setting
confirmation true still requires owner authority. Confirmation needs host authority
and is cleared on a later autonomous content revision. Human Actions require
instruction scope and evidence for Done; resolving a thread does not finish them.

`backup(new_file)` uses the SQLite backup API and exclusively reserves its output.
It refuses occupied paths. `restore(backup, fresh_absolute_directory)` validates
identity/integrity and restores into an empty isolated directory. Backup includes
notes, sources, asset bytes, Actions, threads, pending candidates, edit guards,
revisions, receipts, and BRN-owned run progress. Run records contain no opaque
provider checkpoint. Exports and current search are later consumers of this core.

## Qualification

`cargo test -p brn-threads-core` exercises synthetic fresh directories, durable
prepare/apply lost-response replay, two live connections, scoped protected writes,
whole-group dirty deferral, stale Save, monotonic generations, closed guards,
metadata/protection/lifecycle conflicts, revision-bound confirmation, full-import
policy, actual-write-set Undo, dependent refresh, typed-reference rollback,
source identity, UTF-8 comment anchors, host run fences, and complete backup.

A lost-response witness deliberately drops a completed call's return value and
reopens storage; it is not an OS power-loss or filesystem forensic-erasure test.
SQLite supplies transaction rollback and writer serialization. This crate does
not prove native usability, semantic import completeness, provider behavior, or
host instruction interpretation.

## Workspace policy

A singleton `WorkspaceSettings` record persists the selected provider, model,
effort, credential-location references, maintenance delegation, and review-first
choice. Only direct Owner authority can Configure it. Standing Maintenance writes
return NeedsReview when maintenance is paused or review-first is enabled. Explicit
host instruction grants remain eligible subject to their scope and run fences.
When standing maintenance is paused or review-first is enabled, every instruction
write must name its exact target and carry the applicable action capability: an
ordinary note content/title edit needs EditContent, archive needs Archive, and
supersession needs Supersede. An instruction for A cannot supply fallback writes
to ordinary B. Enabled standing maintenance retains ordinary delegated fallback.
Before bootstrap creates Settings, synthetic core tests retain the default
maintenance policy. Hosts should cancel active runs and advance their fences in
the same owner transaction when pausing active work. No credential contents are
stored in Settings.

## Product service additions

Schema version 3 includes immutable host-input SHA-256 binding during operation
allocation (`allocate_bound_operation`), before preparation dispatch. Earlier
synthetic Threads databases are refused safely rather than migrated.
`replace_candidate` atomically prepares the owner-edited replacement, retires the
old candidate, and transfers independent Review attention. Preparation failure
rolls all of those effects back. Its persisted replacement binding permits
identical replay after retirement/application and rejects changed original IDs
or requests. Attention may name a known operation as well as a canonical record;
Link endpoints remain canonical records only.

`configure_workspace` reads current Settings and all Working runs under the
writer lock and commits their cancellation/fence changes atomically. Its retry
checks the immutable Settings payload before receipt replay. Every Working Run
creation/control update matches the current selected provider/model/
effort, including a planned Settings overlay in the same owner group. There is
at most one Working Run per Thread. A bound invocation also rechecks its current
selection before writing; opening/reading storage does not cancel live runs.
`HostAuthority::for_settings(id, version)` binds the complete persisted Settings
snapshot, including account locations. Runtime Run creation requires this binding
when Settings exist; writer checks precede new writes, while committed receipt
replay survives configuration changes. Codex and ChatGPT aliases identify the
same subscription provider; model and effort still match exactly.

Save shifts supported comment ranges using UTF-8 prefix/suffix boundaries and
marks intersecting or stale mappings unresolved. Original revision/quote/range
remain intact. These Comment writes belong to the actual Save receipt and Undo
write set; later comment editing therefore prevents silent restoration. Undo of
a supported Save restores its anchor at the new compensating revision. Closed
Save/discard sessions clear buffer text while preserving ordering tombstones.
