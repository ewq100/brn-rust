# brn-store

Authoritative SQLite storage: sources/versions, durable operations, local sessions, drafts, immutable revisions and anchored comments. Owns migrations, integrity validation and recovery.

## Interfaces and source

[Store API](src/lib.rs), [drafts](src/drafts.rs), [comments](src/comments.rs), [anchor mapping](src/anchors.rs).

## Dependencies and features

No workspace dependencies. Uses bundled SQLite through rusqlite; consumed by `brn-workflow`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-store --locked
bash scripts/verify-storage.sh
```

Tests cover drafts, comments, workflow records, migrations and process-crash recovery. Use disposable data; preserve exact bytes, immutable provenance and transactional acknowledgement.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
