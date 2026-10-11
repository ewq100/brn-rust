# Phase 1 atomic backup publication plan

**Goal:** A visible final backup filename must contain a complete, validated SQLite snapshot. Interrupted or failed backup must not leave a partial file at that final name or replace an existing destination.

**Architecture:** Preserve the existing `Store::backup` API and SQLite Online Backup implementation. Stage in a private unique directory under the destination parent, close and validate the snapshot, flush it, and publish the complete file with an exclusive hard link. Drop cleans the owned staging directory. Same-parent placement keeps staging/publication on one filesystem. No new dependency, schema change, backup scheduling, retention policy or old-BRN migration.

**Evidence and authority:** `crates/brn-threads-core/src/store.rs::backup` currently creates the final filename before SQLite copies pages. On ordinary errors it removes the reserved inode, but process termination can expose a partial final backup. This is a concrete foundation issue before relying on pre-upgrade backups. The owner authorized continued Phase 1 implementation and CI-gated integration without another routine approval. Follow [Threads target](../../../architecture/threads-target.md#accepted-adoption-priorities-11-october-2026) and preserve checked writes, fresh explicit destinations and existing source data.

## Baseline

Isolated `brn-phase1-backup`, branch `work/phase1-backup`, starts at reviewed steering PR #117 head `30d1185a75c65e90e8019b0d8505b340af94a616`. Fast-forwarded onto its identical-tree merge `ed9a1af3ed54f6f59a980ceab3d7f2fd4976be76` before publishing this increment.

## Scope and reuse

- Keep SQLite Backup API, current application/schema validation and existing complete backup/restore tests.
- Move backup-specific implementation into a small core `backup.rs` module if it clarifies stage ownership; keep schema/open and restore behavior in existing modules unless a proven compatibility fix is required.
- Use existing UUID dependency for unique stage directory names and standard filesystem operations. Private directory creation must be exclusive and restrict Unix permissions at creation. The final path remains fresh, with current symlink-ancestor refusal. Preserve core API compatibility for relative paths by resolving them against the current directory before staging/publication; app/CLI documentation recommends absolute destinations but their current backup wrappers also forward paths unchanged, so this increment adds no new restriction at any layer.
- Build an internal stage/publish seam that tests can exercise directly; do not expose a staging API or add production callbacks only for fault injection. Stage ownership handles cleanup on all ordinary failure paths.
- Exclusive hard-link publication must fail if the final destination exists, including when created after staging. Do not fall back to replacing rename or copying to the final name. A filesystem that cannot publish exclusively returns an actionable error and leaves no partial final file.
- Flush the completed file before publication and the containing directory after publication on supported Unix targets. A post-publication flush error must not delete the already-complete backup. Unexpected termination may leave a recognizable private staging directory; it never makes that staged file a final backup. No broad automatic cleanup of unknown files.
- Close staging SQLite handles before validation/publication; prove the final snapshot is independently readable/restorable without staging files, WAL or SHM dependencies. Source writes remain legal during Online Backup, and the source store is never renamed or modified by publication.

## Implementation and verification

- [x] Independent Sol plan review before code; one Sol implementation owner in a new isolated worktree based on the reviewed steering candidate while CI runs; fast-forward onto its identical-tree merge before backup publication. Parent owns docs, review and integration.
- [x] Implement staging and exclusive publication behind unchanged backup API.
- [x] Deterministic tests prove the final path is absent while a valid stage exists, destination collision after staging preserves its bytes, failed publication removes only owned staging, successful publication leaves a standalone valid/restorable database and no ordinary temporary residue, relative core paths retain their behavior without changing process current directory, invalid destination/symlink refusal leaves source intact, and a full snapshot restores assets/history/pending candidates/recovery buffers.
- [x] Reuse existing occupied-destination test and backup/restore witnesses. Add focused regression for a database using WAL and later source writes; restored snapshot must remain internally consistent and unchanged.
- [x] Run focused core/app backup tests, complete core/app suites if affected, CLI backup/restore integration, strict affected all-target Clippy, formatting and docs links. Use fresh synthetic directories only; no real workspace backup.
- [ ] Independent candidate review approved and supported finding resolved; commit, publish and integrate after required CI. Verify post-merge checks and record exact source and platform limits.

## Gates and remaining scope

Plan review APPROVED by independent GPT-6.1 Sol after preserving relative core path compatibility. Implementation, focused/complete checks and independent candidate review passed; publication/CI/integration pending. Native observation is unnecessary for this unchanged backend API; actual external-backup destination UX remains a separate item. Transactional Threads upgrades, pre-upgrade backup orchestration, saved external destination and wider crash/recovery qualification remain outstanding. This item establishes complete backup publication, not completion of all Phase 1 storage work.
