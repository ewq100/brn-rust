# brn-store

Operational SQLite authority for BRN. WorkStore owns `brn.sqlite`, checked
migrations, local chat, settings, proposal review/approval journals and unfinished editor/save recovery. Vault files
own saved Markdown; disposable retrieval indexes live outside this crate.

## Interfaces and source

[WorkStore](src/work/mod.rs), [editor/save journal](src/work/editor.rs),
[chat records](src/work/chat.rs), [proposal review](src/work/proposals.rs), [unfinished edit compatibility](src/work/edits.rs),
[approval journals](src/work/proposal_apply.rs), [Undo admission](src/work/proposal_undo.rs),
[explicit repair admission](src/work/proposal_repair.rs),
[owned Rewrite jobs](src/work/proposal_rewrite.rs),
[managed note identity](src/note_identity.rs),
[saved note classification](src/note_metadata.rs),
[durable vault provenance](src/note_provenance.rs),
[backup/restore](src/work/backup.rs), [filesystem proof DTOs](src/files.rs) and
[workspace marker guards](src/workspace_mode.rs).

## Database ownership and recovery

WorkStore uses application ID `BRN2`, schema V11, and retains `brn.owner.lock`
for its lifetime. Current settings, text-only conversations and unfinished work
are preserved by additive migrations. Earlier WorkStore V1 unsaved-edit rows
remain available; matching text moves atomically into the generation-aware
editor record, while conflicting recovery stays protected.

Every open checks integrity, upgrades supported schemas, reconciles Running chat and Rewrite
pairs to Interrupted, then creates a startup backup and keeps the five newest
copies. Missing/corrupt databases restore from the newest usable backup;
corrupt originals are moved aside. Foreign and newer databases remain refused.
Database and lock paths must be regular single-link files. A held lock produces
typed WorkspaceBusy; other lock I/O failures return immediately.

The retired `brn.sqlite3` database is never opened or migrated. WorkStore refuses
its database/sidecar markers before opening SQLite, under the owner lock.
Advisory classification also detects current database/sidecar/backup markers and
mixed folders, including dangling symlinks. Existing legacy folders, backups and
vaults remain untouched.

## Exact editor work and Save journals

`EditorRecord` separates exact baseline/buffer text, baseline tokens and
monotonic generations. Opening preserves existing recovery. Recovery accepts an
older acknowledged generation only with the same baseline token, a submission
at least as new as durable text and exact bytes for an equal generation.
Invalid or stale submissions never replace protected work.

Save commits its exact request and intent before filesystem work. UUID binding
includes the request and staging path. Prepared identity, destination-parent
identity and the explicit verified no-op marker remain distinct. Pending or
Uncertain original saves block another original Save. Applied completion
atomically advances the baseline while retaining later typing; a copy does not
rebind the original editor. Matching bytes alone cannot prove installation.

One most-recent Applied recovery pair remains per path; no-op/refused saves do
not refresh it. Workflow retires only proven obsolete artifacts before storage
compacts settled payloads into hash-checked receipts. Pending/Uncertain work and
the latest Applied original retain full journals. Exact UUID replay survives
compaction without granting permission to repeat file writes. Confirmed reload
uses an exact stamp and explicit discard of local changes.

[files](src/files.rs) contains only serializable file/vault/prepared/artifact proof
values. Their fields and wire shape are preserved from the existing Save
implementation. Storage performs no filesystem installation, coordination or
artifact removal.

## Typed proposal review

V4 stores typed Markdown Create/Replace/Trash drafts, exact before-text and file,
parent, vault and source bindings. Creation UUIDs bind the initial payload;
identical creation replay returns current review work without replacing edits.
Records and that binding are checked by hashes, row identity and bounded domain
validation. Each proposal supports 1–64 changes, 1 MiB per note, 8 MiB aggregate
review text and at most 64 comments of 16 KiB each.

Editing, comments, explicit reattachment and rejection use one exact review
version and transactional updates. Changed target content marks anchored comments
Unresolved while retaining their old range/quote; no text search guesses a new
anchor. Late Rewrite results use the same version guard and preserve newer edits
or comments. Rejection retains review work. Group listings keep independently
reviewable proposals separate. Later domains extend this same typed lifecycle.

Managed note identity uses the ordinary top-level frontmatter scalar
`brn_id: <nonnil canonical UUID>`. The pure reader/insertion helper supports
optional BOM, LF/CRLF, complete `---` / `...` delimiters and quoted UUID scalars.
It preserves unrelated metadata/body bytes and never assigns a path/hash identity.
Malformed/duplicate managed fields and unsupported root layouts are reported;
identity assignment refuses incomplete ambiguous frontmatter. Ordinary unmanaged
Markdown remains readable. Full edits/Rewrite preserve an established proposed ID.
Historical records/receipts are not revalidated under this new format, and exact
Undo can remove a prior assignment. No identity table or database migration is added.

`note_identity::body_start` reuses that exact frontmatter walk to locate saved body
bytes, including BOM, CRLF and both closing fences. It checks managed field layout
and the 1 MiB limit while leaving value semantics to their existing readers.

The pure `note_metadata::classify` reader accepts optional ordinary scalars
`brn_kind: knowledge|source` and `brn_state: current|history`; absence means
current knowledge. Quoted values and comments are supported, while duplicate,
continued or unsupported managed syntax reports an error. It neither stamps
metadata nor changes source bytes. Archive-path policy belongs to workflow.
Classification edits use ordinary full proposals or explicit Save; identity
protection and exact historical Undo retain their existing semantics.

The pure `note_provenance` helpers read or propose one optional ordinary root
`brn_provenance: <single-line JSON array>` field. Each typed vault citation retains
a nonnil note UUID, complete saved SHA-256, UTF-8 byte range and exact quote.
Bounds are 32 distinct citations, 16 KiB per quote and 1 MiB per complete note;
duplicate/unknown JSON members and unsupported managed layouts are refused.
Absent provenance does not add a legacy YAML or body-text constraint. Replacement
preserves unrelated bytes, BOM and line endings; an identical typed list returns
the exact original bytes. Workflow owns fresh source resolution and approval;
these helpers do not write files, resolve sources or change operational storage.

## Whole-proposal approval journal

V5 adds a narrow application journal. An exact review stamp and operation UUID
freeze the full Draft snapshot and original creation binding, allocate sibling
staging identities, and advance the review to Applying in one transaction. At
most one proposal application remains unresolved. Same-request replay returns
its current journal before fresh review checks; it never permits another write.

Preparation records one complete immutable fingerprint set matching the proposed
bytes and distinct file identities. Applied completion needs exact proofs for
every installed destination and retained original; partial proof cannot succeed.
NotApplied requires all destinations to retain their original identities or be
absent for Create. Uncertain retains review work and can settle through explicit
reconciliation. Each actual transition advances the review version. Journal and
review commit together, and only Applied clears temporary comments from the live
review and every journal snapshot for that proposal, including older refused
attempts. This removes annotations while preserving approval and file bindings. Settled
receipts are immutable even after later review edits following NotApplied.

Storage validates hashes, indexed/request/creation bindings, encoded size and
domain bounds. It performs no filesystem work and cannot independently observe
the proofs supplied by workflow. The workflow now supplies file application and retained ordinary recovery
snapshots. `refuse_proposal_before_effects` accepts a pending-only, N+2 certificate
from a fresh known-no-attempt path; it cannot discharge Uncertain work and does
not claim ownership of observed staging. Normal NotApplied reconciliation keeps
its strict original-destination proof.

`restore_proposal_apply` transactionally imports validated recovery snapshots,
checks immutable lineage/operation/member/proof bindings and merges forward.
Settled receipts cannot downgrade; newer review work stays intact. Historical
Applied import removes annotations only through its approved version, preserving
later review comments. Storage itself never inspects or writes ordinary files.
Activity projects checked Applied journals in workflow. Native review remains
a subsequent slice.

## Owned Rewrite jobs

V6 records one narrow Rewrite job per request UUID. WorkStore and its checked
ChatStore attachment expose `proposal_rewrite`, `begin_proposal_rewrite` and
`finish_proposal_rewrite`. Admission binds an exact Draft stamp, the full capture
digest and explicit provider/model/effort. Only fresh admission returns a full
transient capture; replay returns history without granting another provider call.
Jobs retain bounded hash-checked metadata, safe outcomes and result stamps, never
captured comments, prompts or raw result bodies. Existing proposal review holds
the validated full note/Action candidates. Chat and Rewrite cannot reuse a job UUID.
Action-only and mixed admissions use the same complete capture and settlement;
Rewrite never changes real Actions. Complete ordered ActionData edits preserve
member identities/full Replace baselines. Empty vectors stay omitted in persisted
ProposalEdit/RewriteOutcome so historical Markdown bytes and hashes remain exact.

Completion validates every member and compares both the review stamp and full
capture hash in the same transaction as the proposal edit and terminal job. Later
edits/comments/rejection/approval or capture drift settle Stale without overwriting
work. Atomic failure leaves both records unchanged. Checked startup interrupts
Running jobs without retry; terminal request/outcome replay stays immutable.
The pure `validate_result` uses the same exact edit bounds and anchor rules.

## Exact Undo and Trash admission

`preview_proposal_undo` derives a read-only inverse of one checked Applied
operation. `begin_proposal_undo` atomically creates its inverse proposal and
Applying journal under a new operation UUID. Create becomes Trash, Replace
restores the retained original, and Trash becomes Create. An explicit optional
`trash_member` selects one original Trash member for independent restore; it
cannot select arbitrary Create or Replace changes. External source bindings are
not copied, and the inverse title stays within the original title's byte budget.

The journal's optional Undo binding fixes the source operation, scope and exact
retained member identities/fingerprints. Restoring Create/Replace borrows those
originals; prepared proofs must match, and strict NotApplied reconciliation must
also prove each borrowed stage unchanged. Normal approval of a refused inverse
uses fresh ordinary staging without borrowing. UUID replay preserves the admitted
snapshot, and ordinary recovery can restore it without the source journal.
Absent Undo bindings preserve existing journal JSON bytes and checksums, with no
schema migration. Ordinary journal metadata keeps its existing cap. Undo bounds a
normalized path/member base independently from fixed proof/receipt slots and its
manifest. The base stays stable through repeated inverses, so a valid large
proposal remains undoable without accumulating cap headroom. Storage performs no
file writes or retained-artifact deletion;
workflow qualifies execution and interruption separately.

## Explicit interrupted-operation repair

`ApplyJournal::repair_preview` classifies every member as exactly Before or
Applied from its destination/staging fingerprints and complete prepared set.
Unknown or partial proof refuses repair. Its capture hash binds the full approved
draft, original approval, creation/member/prepared/Undo bindings, prior repair
UUIDs and current proofs; temporary comments and mutable outcomes are excluded.

`begin_proposal_repair` admits an explicit Finish or Restore request atomically,
with a fresh attempt UUID and exact capture hash. Up to 64 attempts retain their
requests and outcomes, with only the latest admission proofs. Admission leaves
the proposal version and review comments intact. Exact UUID replay returns the
current journal before checking new observations and grants no file-write
permission. `interrupt_proposal_repair` records an uncertain latest attempt;
the original first Uncertain observations stay immutable. Whole-operation
settlement updates the latest attempt outcome and receipt together, with Applied
comment cleanup in the same transaction. Repair cannot use a no-effect certificate.
Known terminal repair receipts require the complete exact Before/Applied endpoint
pairs, including staging proofs; unchanged destinations alone cannot clear repair.

Recovery accepts compatible forward history only, checks repair UUID uniqueness
across journals and preserves settled endpoints. `repair_history_covers` exposes
that checked ancestry for workflow artifact cleanup. Absent repair fields retain
old encoded JSON/checksums without a migration. Repair metadata has its own fixed
allowance; normalized member/path bounds and fixed proof slots allow admitted
large journals to settle without consuming growing shared headroom. The complete
encoded journal remains below 64 MiB. Storage performs no repair filesystem work;
workflow qualifies coordinated moves and interruption separately.

## Local chat

`begin_turn_with_effort` atomically inserts a text-only user/assistant pair with
one UUID, conversation sequence, explicit provider/model and optional effort.
V7 binds low/medium/high equally in both rows; invalid or mismatched rows fail
validation. Historical `begin_turn` records unknown effort (`None`), preserving
older history without guessing a default. Exact UUID replay binds effort and returns
its Running or terminal result; changed payloads return OperationConflict.
Unknown conversations return NotFound without inserts.

`finish_turn` atomically records both rows' terminal status/error category, final
or partial assistant text, and the first question as the title. Terminal records
are immutable except for identical replay. Unicode and line endings stay exact.
Only safe user/assistant text is accepted; no tokens, device codes, raw provider
bodies or credential metadata are stored. Restart retains already durable text
without provider resubmission or automatic retry.

An attached ChatStore shares the exact owner lock and uses serialized SQLite
transactions. Dropping WorkStore cannot release ownership while a chat
attachment remains active.

V8 exposes existing session creation time and adds nullable session last-activity
and turn start/finish times. Historical missing times serialize as explicit null;
migration never reconstructs them. Fresh admission and genuine finalization
capture one checked clock value atomically with the pair/activity writes, clamped
against known creation/activity/turn times. Exact running/terminal UUID replay
reads no clock and does not refresh activity. Startup interruption keeps known
start time, unknown finish time and unchanged activity, including in its backup.
Reads/startup validate nonnegative values, equal role-pair times and known temporal
bounds; empty conversations are checked too. Timestamp metadata is nullable and
checked during reads/reconciliation, so malformed times cause refusal rather than
SQLite corruption recovery. Timing does not change exact text, selection, effort,
replay or attachment ownership. Conversation summaries read their title, count and
times in one SQLite snapshot while attached chat writers continue independently.

## Tentative findings

V9 adds [findings](src/work/findings.rs) to the existing operational database.
Each immutable creation draft retains a symbolic issue request, bound vault,
title/summary and exact source fingerprints with optional UUID/quote ranges.
Identity ambiguity requires distinct exact saved paths; path case is preserved.
Unresolved-link occurrence and used definition can share one file with different
ranges. Workflow validates actual saved bytes; Store never reads a vault.
Strict nested JSON, bounded retained/encoded work, hashes and row/creation bindings
are checked before use and at startup before reconciliation/backup.
For supported branded V9+ databases, readable Findings are also validated before
SQLite quick_check so state/hash CHECK violations refuse the main database rather
than restoring older work. Physical corruption retains backup recovery; malformed
Findings backup candidates are skipped without modifying the retained backup.

Open version 1 advances once to Resolved or Dismissed version 2 by exact stamp.
Identical creation or old-stamp/same-outcome closure replay returns retained work;
changed input or a competing closure refuses atomically. Closing changes queue
state only. Bounded pages use descending creation-time/UUID cursors, including a
cursor closed between pages, with the open count in the same SQLite snapshot.
Findings survive ordinary backup/recovery and remain tentative work rather than
knowledge or proposal authority.
Supported V9+ records are validated before SQLite quick_check: readable semantic
damage refuses a main database without restoring older work, while an invalid
backup candidate is skipped. Physical SQLite corruption still uses recovery.

## Checked Action foundation

V10 adds [Actions](src/work/actions.rs) in the existing `brn.sqlite`. This
public standalone API exposes only `action(id)` and bounded `action_list(request)`
reads; creation/changes join exact proposal transactions. `ActionOrigin` retains the
creating proposal stamp and initial exact data. `ActionRecord` retains that
immutable origin, current data/revision and ordered waiting/completion times.
Open, Waiting, Blocked and Completed remain distinct. Optional explicit priority
stays unset when omitted; civil due/follow-up dates use canonical YYYY-MM-DD.

Reads preserve exact strings, filter an optional state and order by immutable
creation time/UUID descending. A typed cursor retains that key independently of
later state changes; the default is all states/25 entries, maximum200. A snapshot
binds validation and paging. Bounded JSON, origin/record hashes, indexed bindings,
owned table/index shape, UUID/date/time/revision semantics are checked. Semantic
damage refuses startup before reconciliation/backup; supported V10 owned schema
and complete records are checked before quick_check can mistake an unexpected
CHECK for physical damage. Invalid backup candidates are skipped; physical damage uses
the existing backup restoration path. Relations/dependency existence and cycles
are approval concerns, not guesses made by retained read APIs.

Typed proposal `ActionChange::Create` and `Replace` members retain exact candidate
data and immutable full replacement baselines. Action-only drafts have no vault
binding; Markdown members/source proofs require one. The existing review, edit,
temporary comments, rejection, exact stamp and replay lifecycle covers these
members, with one combined64-member/8MiB budget. Completed work cannot be changed
or reopened through these members. Empty additions stay omitted from historical
Markdown serialization. Shared workflow creation/application uses these typed
members; owned AI Action Rewrite and Action-bearing Undo remain refused.
Creation/replacement remains proposal-only. Identified completion has the separate
narrow storage contract below; it does not create, edit or reopen Actions.

`ApplyJournal::action_records` retains complete ordered after-state derived from
the exact approved members, stamp and captured time. Admission checks absent
Create/full Replace baselines before Applying. Applied settlement writes Actions,
whole receipt, review and annotation cleanup in one transaction; other outcomes
write no Actions, and terminal replay preserves later work. Snapshot encoding has
a separate bound within the existing total journal/mirror limits.
Read-only `validate_proposal_action_changes` checks complete fresh baselines;
`validate_proposal_apply_actions` checks the admitted unresolved snapshot before
workflow effects or completion authority. Admission and settlement retain their
transactional CAS; these helpers grant no mutation permission.

Recovery imports already-real Replace baselines before Applied after-state in
the same transaction. Immutable origins and equal-version records must match;
newer work wins and completed work cannot be reopened. A known before-state fork
refuses even when its revision precedes the imported after-state. Imports do not
repeat filesystem effects. Action-bearing Undo explicitly refuses until its
inverse contract qualifies. Workflow owns ordinary mirrors, interruption and
mixed Finish/Restore; this storage contract does not establish the dashboard.

## Dependencies and verification

No workspace dependencies. Uses bundled SQLite through rusqlite; only
`brn-workflow` consumes these records in the production architecture.

Run offline checks with disposable synthetic fixtures:

```sh
cargo test -p brn-store --locked --offline
cargo clippy -p brn-store --all-targets --locked --offline -- -D warnings
```

Tests cover migrations, foreign/newer/corrupt/missing databases, backup restore,
marker refusal, lock/attachment ownership, exact bytes, stale acknowledgements,
uncertain saves, parent proof, no-op replay and compact recovery. Workflow tests
qualify filesystem execution and process interruption separately.

## Identified completion foundation

V11 adds checked [Action completion receipts](src/work/action_completion.rs) to
the same WorkStore. `CompleteActionRequest` binds an operation UUID and the full
unfinished before-record. The exact after-record preserves origin and content,
advances one revision, clears Waiting time and records completion/update time at
least as late as the before-record. Completed work never reopens.

`complete_action_with` requires a publisher of the exact recovery snapshot. An
Immediate SQLite transaction checks full CAS and all bounds before publication,
holds write exclusion during publication, then settles the record and immutable
receipt together. Publication or SQL failure rolls back SQLite. The workflow
caller must publish durable ordinary evidence and reconcile it after uncertainty;
a callback success alone does not establish filesystem durability. Exact bound
replay returns the retained receipt without publishing again or changing time.

`restore_action_completion` imports checked evidence into older/fresh SQLite,
refuses incompatible origins/equal-version forks or newer unfinished records,
and preserves newer Completed records. Bounded encoded bytes, request/receipt
hashes, canonical indexed identities, exact owned schema and the retained
Completed record are validated before quick_check. Readable semantic damage
refuses the main database; invalid backup candidates are skipped.

Workflow/AppWorker now qualifies publication, startup recovery, uncertainty,
operation correlation and shutdown draining; the CLI exposes exact Complete.
Native controls remain a later slice. Clients continue through workflow/AppWorker;
they do not call these storage APIs or write SQLite directly.

[Dashboard queries](src/work/actions/dashboard.rs) project one checked read
transaction, without a schema change or stored cache. An explicit canonical
civil date, filter and immutable cursor bind a page of1–200 complete records.
Global counts inspect every retained row, including hidden/filter-excluded work;
dependency states share that snapshot and preserve ordered/missing targets.
Completed Actions have no overdue/follow-up signals; other due dates compare
strictly before the date and follow-up dates on/before it. Workflow owns date
defaults and current-evidence eligibility. No ranking or Action mutation occurs.
