# brn-store

Operational SQLite authority for BRN. WorkStore owns `brn.sqlite`, checked
migrations, local chat, settings, proposal review/approval journals and unfinished editor/save recovery. Vault files
own saved Markdown; disposable retrieval indexes live outside this crate.

## Interfaces and source

[WorkStore](src/work/mod.rs), [editor/save journal](src/work/editor.rs),
[chat records](src/work/chat.rs), [proposal review](src/work/proposals.rs), [unfinished edit compatibility](src/work/edits.rs),
[approval journals](src/work/proposal_apply.rs), [Undo admission](src/work/proposal_undo.rs),
[explicit repair admission](src/work/proposal_repair.rs),
[backup/restore](src/work/backup.rs), [filesystem proof DTOs](src/files.rs) and
[workspace marker guards](src/workspace_mode.rs).

## Database ownership and recovery

WorkStore uses application ID `BRN2`, schema V5, and retains `brn.owner.lock`
for its lifetime. Current settings, text-only conversations and unfinished work
are preserved by additive migrations. Earlier WorkStore V1 unsaved-edit rows
remain available; matching text moves atomically into the generation-aware
editor record, while conflicting recovery stays protected.

Every open checks integrity, upgrades supported schemas, reconciles Running chat
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
reviewable proposals separate. Stage 4 remains active.

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
Activity projects checked Applied journals in workflow. AI Rewrite and native
review remain subsequent slices.

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

Recovery accepts compatible forward history only, checks repair UUID uniqueness
across journals and preserves settled endpoints. `repair_history_covers` exposes
that checked ancestry for workflow artifact cleanup. Absent repair fields retain
old encoded JSON/checksums without a migration. Repair metadata has its own fixed
allowance; normalized member/path bounds and fixed proof slots allow admitted
large journals to settle without consuming growing shared headroom. The complete
encoded journal remains below 64 MiB. Storage performs no repair filesystem work;
workflow qualifies coordinated moves and interruption separately.

## Local chat

`begin_turn` atomically inserts a text-only user/assistant pair with one UUID,
conversation sequence, explicit provider and model. Exact UUID replay returns
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
