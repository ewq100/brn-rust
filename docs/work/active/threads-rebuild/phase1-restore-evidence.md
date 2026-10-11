# Phase 1 restore and missing-storage evidence

## Authority and baseline

Continued Phase 1 implementation and required-CI-gated integration are explicitly authorized. Isolated `brn-phase1-restore` / `work/phase1-restore` starts at `087471fd760c7c32e0f7e3f919ca0eb98ca1e65e` (PR #118 merge). See the [restore plan](phase1-restore-plan.md). Only fresh synthetic stores are used; synced references and the original checkout remain unchanged.

## Prior integration

Delegation PR #116, Stop-and-send PR #117 and atomic-backup PR #118 merged after all four required checks passed. All three also passed all four required checks on main. Backup main run [38138261242](https://github.com/ewq100/brn-rust/actions/runs/38138261242) passed the four required checks plus documentation and Linux native UI after merge `087471fd760c7c32e0f7e3f919ca0eb98ca1e65e`. Broader non-required Windows and Linux retrieval lanes remain failing; no all-platform green claim or release.

## Plan and implementation

Independent GPT-6.1 Sol plan review approved after two corrections: existing marked stores open without CREATE and validate the same connection before mutable pragmas; restore preserves nested nonexistent destination compatibility. Implementation and local candidate checks are complete. Independent GPT-6.1 Sol final review: APPROVED, no actionable issues. No production changes followed approval.

Both behavioral reds reproduced against the baseline: opening a marked directory with its database removed succeeded and recreated storage; restore's initial create/copy seam exposed the requested destination before publication.

The existing-marker branch uses a production no-CREATE connection helper and validates that same connection before mutating pragmas. It never initializes schema. Missing storage now returns actionable fresh-directory recovery guidance and preserves marker/WAL/SHM evidence. Zero-byte, foreign and incompatible database witnesses assert unchanged bytes.

`restore.rs` stages a complete workspace privately beside the destination, copies through SQLite Online Backup, normalizes DELETE journal mode, closes/validates/flushes its database and marker, then publishes with directory rename. It preserves empty and nested nonexistent destinations, refuses occupied/symlink paths and cleans only owned staging. A narrowly scoped Windows fallback follows a failed rename for an empty directory; it has not been exercised locally.

Focused tests exercise the production stage/publication seams for visibility, ownership, collision after preflight, corrupt-copy failure, abandonment, post-publication errors and source-byte preservation. Post-publication error witnesses inject flush/reopen errors through the production publication continuation; they do not claim a physical power-loss test. Core full-state restore reopens the published nested destination and compares records, receipts, pending candidates, revisions and dirty buffers. App and CLI witnesses prove missing storage is refused without recreating it.

## Local verification

Pinned Rust 1.98.1 on macOS; locked/offline builds, with `CARGO_TARGET_DIR=../threads-macbook-checkout/target`:

```sh
cargo test -p brn-threads-core -p brn-threads-app --locked --offline
cargo test -p brn --test threads full_markdown_import_export_and_backup_are_fresh_and_readable --locked --offline
cargo test -p brn --test threads missing_storage_refuses_init_and_reads_preserving_marker_and_recovery_evidence --locked --offline
cargo clippy -p brn-threads-core -p brn-threads-app -p brn --all-targets --locked --offline -- -D warnings
cargo fmt --all --check
python3 scripts/check-markdown-links.py
git diff --check
```

Core: 90 tests passed (19 unit, 71 integration). App: 32 integration tests passed, including the 5,000-note scale witness. Two focused CLI tests passed. Strict Clippy, formatting and diff checks passed; Markdown check: 104 files and 388 local links, zero failures. No production app/CLI changes were necessary.

Independent reviewer reran core filters `restore` (14 tests), `existing` (4 tests) and `missing_storage` (1 test), all passing. These overlap complete suites, not additional test totals. Reviewed six-file implementation fingerprint: `70c7a0f03c7e291090616eccebe0be3a69a3d362e5b0f3cddaae17cbb7d50337` (sorted relative paths + NUL + file bytes + NUL). No schema, dependency or checked-writer changes.

## Integration gate

Local candidate verified; required PR/main CI and merge pending. Owner authorization is already explicit. Native observation remains deferred. No release.

## Limits

Schema remains3; this is not a migration from earlier BRN. Future real schema changes require transactional upgrades and pre-upgrade backup orchestration. Remembered backup destination and native/recovery qualification remain separate; native observation remains deferred.
