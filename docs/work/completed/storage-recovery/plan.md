# Storage and recovery implementation plan — chunk 06

> Historical record: branch names, commands, approvals and results below describe the recorded task, not new instructions or current authorization. See [current status](../../../status.md) for the present checkout and remaining gaps.

2026-09-28 · Branch `trial/storage-recovery` · Base `8540de4f8ef468cfa4812d9253f2512bf655750c`.

## Scope and ownership

Implement the authoritative storage boundary selected in the [approved architecture](../../../architecture/decisions/2026-09-28-architecture-baseline.md): a new UI-independent `brn-store` workspace crate with SQLite migrations, immutable source versions, local session/message projections and durable operation identities. The user requested continued building after storage was identified as the next chunk. Native shell/editor acceptance remains deferred; this backend can be qualified independently, without importing private documents or changing the shell's sample task.

Astra supplies architecture advice and an independent final review. Sol owns the storage crate and workspace dependency changes; Luna owns its verification script and dependency inspection. The lead owns integration, documentation, final checks and the trial push. These scopes share the existing isolated trial checkout without overlapping edits. Original and earlier pushed trial branches remain unchanged.

## Design

A non-cloneable store privately owns its SQLite connection and a permanent owner-lock file. Acquire a lifetime OS file lock before opening or recovering the database; another store must fail clearly, even in the same process. Never delete the lock file as stale. Reject database aliases that would let different owner locks protect the same file. Mutation methods require exclusive mutable access; future application integration will place this owner behind a worker boundary.

Separate a missing database from an existing one. Existing databases must have the expected SQLite/application/schema identity. Refuse foreign, newer, malformed and unsupported schemas without replacing data. Recognized databases may require SQLite's own hot-journal recovery before integrity/schema validation. Apply application migrations transactionally only after validation. Do not promise byte-for-byte immutability when SQLite legitimately recovers a recognized interrupted database.

Use foreign keys, untrusted-schema mode, DELETE journaling, synchronous EXTRA and macOS fullfsync; verify active settings. These settings support the durability intent; process-kill tests do not establish hardware power-loss guarantees. See [SQLite pragmas](https://sqlite.org/pragma.html), [rollback locking and recovery](https://www.sqlite.org/lockingv3.html), and [Rust lifetime file locks](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).

Store opaque UUID identities, exact UTF-8 revision bytes and SHA-256. Parent revisions must belong to the same source. Stored source/session/version identities and interrupted operations must be discoverable after reopening. Reads verify bytes and hashes; no implicit Unicode or newline normalization. Sessions store provider association metadata and messages as local projections; Codex remains authoritative for provider history and authentication.

Bind operation UUIDs to command kind and an internally computed hash of exact payload bytes. Matching duplicates return existing state/results, while conflicting reuse fails. Local effects and their operation result commit atomically. External-work records commit pending before callers may submit; pending/running transitions are explicit, terminal results cannot be overwritten. On ownership after restart, mark both pending and running interrupted, with no automatic replay. Retrying uncertain external work requires a new operation identity and later reconciliation policy.

## Implementation and acceptance sequence

1. Add dependencies and storage schema/migrations, then test creation, upgrade and rollback of a failed migration.
2. Add source/version and session/message APIs with transactionally bound operation results; verify exact bytes, parent constraints and duplicate/conflicting requests.
3. Add operation transitions and startup interruption, then verify owner exclusion and safe refusal paths.
4. Use real child processes with bounded readiness handshakes and forced termination to prove acknowledged persistence, recovery of unfinished operations and rollback of spilled uncommitted pages. Always reap children.
5. Run workspace build, formatting, Clippy, tests and CLI/headless smoke checks; keep existing provider/starter regressions passing. Inspect dependency licenses and the final diff for secrets and unrelated edits.
6. Obtain independent Astra review, resolve material findings, record reproducible commands and limitations, then commit/push this trial branch and compare the remote commit.

Desktop storage wiring, document import, retrieval invalidation, provider submission/reconciliation and signed packaging remain subsequent integration work. The milestone is a reviewed, crash-qualified backend, not a complete usable app or native acceptance claim.
