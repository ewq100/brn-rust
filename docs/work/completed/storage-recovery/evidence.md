# Storage and recovery evidence — chunk 06

> Historical record: branch names, commands, approvals and results below describe the recorded task, not new instructions or current authorization. See [current status](../../../status.md) for the present checkout and remaining gaps.

2026-09-28 · Branch `trial/storage-recovery` · Base `8540de4f8ef468cfa4812d9253f2512bf655750c`.

## Scope

This checkpoint adds an independent authoritative SQLite backend under the [accepted architecture](../../../architecture/decisions/2026-09-28-architecture-baseline.md) and [implementation plan](plan.md). The user requested continued implementation after the next storage milestone was identified. It uses disposable synthetic data. The desktop shell remains an in-memory sample; provider submission, retrieval and document import are not wired to storage yet.

The existing isolated trial checkout preserves the original repository and all earlier pushed branches. Astra advised on ownership, durability and crash qualification; Sol implemented storage; Luna prepared verification and inspected dependencies; the lead owns integration and delivery. Independent final review and verification results are recorded below.

## Verification record

The final commands exited 0 on macOS 15.3.1 arm64 with Rust 1.98.1:

```sh
bash scripts/verify-storage.sh
PATH=/opt/homebrew/opt/rustup/bin:$PATH bash scripts/verify-trial.sh
```

The storage script passed workspace build, formatting, Clippy with warnings denied, **one storage unit test and 13 integration-test entries** (one entry is the subprocess helper), **12 core tests**, **five desktop CLI tests**, CLI smoke checks and three headless scenarios: completion, cancellation and stale-result rejection. Its unknown argument and relative `CARGO_TARGET_DIR` refusal paths were also checked. The regression script passed starter/provider builds, formatting, Clippy, the workspace tests, **16 provider tests** and nine starter/provider CLI smoke cases. The regression run preceded the final additions to storage crash tests; the final storage run covers all completed storage code/tests. No live or credentialed command ran.

Storage tests cover exact UTF-8/CRLF bytes, hash verification, parent-source consistency, identical retries and conflicting payloads, terminal-state immutability, optional argument changes, session/message persistence, record discovery, same-process and cross-process owner exclusion, database/lock aliases, schema migration and transactional failure rollback. Refusal fixtures include foreign/newer databases, unexpected schema objects (including a prefix designed to catch an SQL wildcard mistake), malformed headers, recognized truncated databases, foreign-key violations and orphan journals. Rejection fixtures without hot journals retain their main-file bytes.

Three real child-process scenarios use a ten-second acknowledgment deadline, a guard that kills/reaps on panic, and explicit forced termination after readiness:

- Committed source, version, session and message survive. Both pending and running operations become interrupted. Reusing their identities returns `Existing`; attempting to start them again fails.
- A large uncommitted version update spills to the main database. After termination, reopening restores the original exact bytes and valid hash. The test checks journal presence and that the main-file hash changed before termination.
- A v1 migration fixture begins uncommitted DDL and spills changed data. The test **explicitly writes a v2 value into the on-disk header** while retaining a valid rollback journal, then terminates the child. This fault injection verifies that reopening uses the recovered v1 header/schema before successfully migrating. It is not a claim that this exact header-write sequence was naturally observed in the production migration function. The child retains the owner lock throughout.

The original chunk 01 and provider lifecycle worktrees remain clean. No experiment source, desktop UI source or core source changed. Native UI build/interaction was not repeated for this independent backend; the prior shell evidence remains the latest native-build record.

## Reproducing individual recovery checks

From the repository root, use the pinned toolchain. Each check creates and cleans its own synthetic directory:

```sh
cargo +1.98.1 test -p brn-store --locked --offline killed_process_preserves_acknowledged_rows_and_interrupts_pending_work -- --exact
cargo +1.98.1 test -p brn-store --locked --offline killed_uncommitted_spilled_transaction_recovers_original_bytes -- --exact
cargo +1.98.1 test -p brn-store --locked --offline killed_migration_recovers_header_and_then_upgrades_v1 -- --exact
cargo +1.98.1 test -p brn-store --locked --offline refuses_foreign_newer_and_unexpected_schema_without_replacing_bytes -- --exact
```

Expected outcomes are passing tests, preserved committed records, interrupted uncertain operations, rollback of incomplete writes and visible refusal of rejected databases. Do not manually remove lock/database files from real application data to reproduce these checks.

## Storage interface

`Store::open(existing_directory)` returns the owned store and a recovery report. The directory must already exist; a missing database is initialized once. Existing malformed/newer/foreign files fail visibly. Schema version 1 contains source/version records; version 2 adds sessions/messages/operations. SQL schema definitions are checked against the expected version, including unexpected user-created objects.

`create_source`, `add_version`, `create_session` and `add_message` each take an operation UUID. They encode their own arguments, commit the entity and completed operation together, and return the same entity UUID on an identical retry. Reusing that UUID for a different command or payload fails. Source versions retain the original bytes; parent versions must belong to that source. Message ordinals preserve projection order. Read/list methods support discovering sources, versions and sessions after restart without another ID cache.

`begin_operation` handles external work separately and returns `New` or `Existing`. Only `New` authorizes the caller to consider a new submission. `mark_running` and `finish_operation` enforce state transitions; `operation` reads the stored state. Reserved local mutation kinds cannot be submitted through this generic path. `interrupted_operations` supplies identities for later reconciliation; this crate performs no external submission or replay.

## Recovery contract and limitations

One lifetime owner lock serializes cooperating BRN processes. Keep `brn.owner.lock` in place; do not delete it to bypass a busy owner. The lock is advisory and does not prevent an external SQLite editor from modifying a database. Store ownership will move behind an application worker during desktop integration; callers must not perform blocking database work on the UI thread.

A successful mutation acknowledges a committed transaction. Immutable revisions retain exact UTF-8 bytes and hashes. Local writes and their durable operation result share one transaction, so a duplicate command returns its original identity. External-work records only prevent BRN from starting a duplicate operation ID: they cannot establish exactly-once execution by an external service. Pending and running records become interrupted after a new owner opens the store. They are not automatically retried.

SQLite performs recovery of recognized hot journals before application validation and recovery. Rejected databases are not replaced, reset or silently migrated. A crash during first initialization may leave an incomplete file that is refused on the next open; there is no automatic reset, and no user write has yet been acknowledged. Preserve the database and its sidecars on failure; diagnosing or restoring real data is a separate explicit action.

The selected journal/sync settings are documented in [SQLite's pragma reference](https://www.sqlite.org/pragma.html). The automated crash checks concern terminated processes on this Mac; they do not simulate power failure, storage-controller behavior or disk exhaustion. [Rust file locking](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock) supplies owner exclusion without a stale PID-file recovery policy.

Native shell/editor interaction acceptance remains open under the user's deferral. GPUI is provisional. No packaging/signing, account change, credential access, vault migration, merge or release is part of this checkpoint.

## Review and delivery

Astra's architecture review identified restart discovery, recovery-time schema version re-reading and database alias handling; these were addressed. The lead caught a schema-object filter that treated `_` as an SQL wildcard; exact-prefix filtering and a regression fixture now cover it. Independent source review also corrected the migration test's ownership and made child cleanup safe on assertion failure. Astra independently reran the final storage suite (one unit and 13 integration entries) and approved the focused checkpoint with no remaining Critical or Important findings. The lead’s final workspace script also passed. Final documentation review found no blockers; its requested distinction between directly tested cases and source-reviewed behavior was incorporated. Secret-pattern and local Markdown-link scans covered all 12 changed/authored files with no findings; every root lockfile dependency source remains crates.io. `git diff --check` passed. The final task response records the pushed, remote-verified commit.

This completes the bounded storage backend implementation checkpoint. Desktop storage wiring, Markdown/text import and version tracking are next; native shell/editor acceptance remains a separate open gate. Nothing is merged or released.
