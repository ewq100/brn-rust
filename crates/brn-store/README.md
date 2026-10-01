# brn-store

Authoritative SQLite storage: sources/versions, durable operations, local sessions, drafts, immutable revisions, anchored comments and managed-note recovery. Owns migrations, integrity validation and recovery.

## Interfaces and source

[Store API](src/lib.rs), [drafts](src/drafts.rs), [comments](src/comments.rs), [anchor mapping](src/anchors.rs), [managed-note records](src/notes.rs).

## Managed-note storage contract

Schema V6 adds a single registered vault, note/path identities, one exact-byte
editing buffer per note, durable save/copy intents, recovery pairs and
hash-checked tagged receipts. V1–V5 upgrades preserve existing records; existing
chat turns default to `EvidenceCurrentness::Unqualified`. The
`complete_turn_with_currentness` method atomically records terminal text,
status and evidence currentness; `complete_turn` keeps its prior semantics.

The note APIs return typed `NoteResult` failures without changing existing
store error contracts. Text is exact UTF-8, limited to 1 MiB **in bytes**;
generations must fit SQLite's signed integer range. Buffer acknowledgements
establish SQLite recovery, not Markdown publication. Recovery baseline bytes
are not a fresh observation of the saved file.

Check `note_write_result` before accessing the vault or validating fresh state.
Operation IDs bind submissions, destinations and write kinds. Replays return
their recorded receipt/failure and never acquire permission to write again.
One unresolved original-path intent blocks another original save even after
startup interruption; independently reserved copies remain allowed.
Only note-specific completion/reconciliation can resolve these writes.
Enrollment cannot allocate a competing identity at a reserved copy destination;
it may reopen the already registered reserved target. Failed refusal recording
returns `Storage` with the original refusal context, actual phase/outcome and
no recovery acknowledgement.

This crate performs **no vault filesystem operations**. The workflow must
verify prepared/installed/displaced identities and durability before supplying
verification or reconciliation. It must also verify artifact retirement or an
unexpected untouched occupant before `record_note_cleanup` records that
bookkeeping. Cleanup is monotonic: `Pending` may become `Retired` only for a
resolved known terminal outcome, or `RetainedUnexpected`; terminal values
cannot be reversed or exchanged. Unresolved, accepted-current and
uncertain/unknown outcomes cannot authorize retirement.

`prune_completed_note_payloads` removes only superseded, successfully completed
payloads with retired artifacts and no protected dependency. The latest
recovery pair/buffer, unresolved work, unexpected artifacts and receipts
survive. Only verified Applied outcomes advance the successful-save recovery
pair, including Applied reconciliation that preserves a historical failure
result; no-op and reconciled NotApplied outcomes leave that pair unchanged.
Pruned operations remain replayable through `note_write_result`;
their full intent is no longer returned by intent reads/lists. A normal
unchanged-save completion creates no artifact; interrupted intent
reconciliation cannot infer artifact absence merely from missing metadata.

## Dependencies and features

No workspace dependencies. Uses bundled SQLite through rusqlite; consumed by `brn-workflow`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-store --locked
bash scripts/verify-storage.sh
```

Tests cover drafts, comments, workflow records, managed-note transactions and
cleanup, migrations and process-crash recovery. The bounded note check is:

```sh
cargo test -p brn-store --test notes --test storage --test workflow --locked
```

Use disposable data; preserve exact bytes, immutable provenance and
transactional acknowledgement. These checks do not qualify the macOS
filesystem adapter, native editing or live-vault behavior.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
