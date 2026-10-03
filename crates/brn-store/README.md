# brn-store

Operational SQLite authority for BRN. WorkStore owns `brn.sqlite`, checked
migrations, local chat, settings and unfinished editor/save recovery. Vault files
own saved Markdown; disposable retrieval indexes live outside this crate.

## Interfaces and source

[WorkStore](src/work/mod.rs), [editor/save journal](src/work/editor.rs),
[chat records](src/work/chat.rs), [unfinished edit compatibility](src/work/edits.rs),
[backup/restore](src/work/backup.rs), [filesystem proof DTOs](src/files.rs) and
[workspace marker guards](src/workspace_mode.rs).

## Database ownership and recovery

WorkStore uses application ID `BRN2`, schema V3, and retains `brn.owner.lock`
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
