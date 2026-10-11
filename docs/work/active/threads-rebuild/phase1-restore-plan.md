# Phase 1 restore and missing-storage safety plan

**Goal:** Interrupted restore must not expose an empty ready workspace, and an existing workspace whose database is missing must never silently become empty.

**Evidence:** `Store::restore` currently calls `Self::open(destination)` before Online Backup, making an empty marked schema-v3 database visible during copy/failure. `Store::open` accepts an existing valid marker with no database and initializes a new empty database. Both behaviors are observable in `crates/brn-threads-core/src/store.rs` and defeat fail-closed recovery.

**Authority:** The owner authorized continued Phase 1 implementation and CI-gated integration without further routine approval. Preserve the checked writer, existing SQLite schema/dependencies and fresh/empty destination API. No old-BRN migration, actual user data, provider calls, release or speculative schema version is involved.

**Baseline:** New isolated `brn-phase1-restore`, branch `work/phase1-restore`, at backup merge `087471fd760c7c32e0f7e3f919ca0eb98ca1e65e`. GPT-6.1 Sol implementation and independent review; parent owns docs and integration.

## Contract and design

- On open, distinguish an existing valid data marker from a marker created by this call. If the existing marker has no `threads.sqlite3`, return a clear unsafe/missing-storage error directing recovery from a backup into a fresh directory. Do not create a database, rewrite the marker, or remove WAL/SHM/other evidence. Missing, zero-byte and incompatible databases remain distinct failure cases. For an existing marker, open SQLite read/write without CREATE, validate identity/schema/integrity on that same connection before mutating pragmas, and never enter schema initialization. This closes the deletion/replacement window between path validation and connection. Fresh genuinely empty unmarked directories still initialize normally.
- Preserve `Store::restore(backup, fresh_absolute_directory) -> Result<Store>`. Accept a nonexistent or empty destination and reject nonempty/foreign/symlink destinations. Validate the source read-only before preparing the destination. Preserve relative backup-source behavior; destination stays absolute as required by existing Store::open. Preserve nested nonexistent destination compatibility by checking ancestors for symlinks and safely creating missing parent directories before sibling staging.
- Create a unique private (Unix 0700) sibling staging directory under the destination parent; use existing Store::open only inside that owned stage. Copy with SQLite Online Backup, normalize its database to standalone DELETE journal mode, close SQLite handles, validate, and flush database, marker and stage directory before publication.
- Publish the complete staged directory with a filesystem rename. On Unix rename can atomically replace an empty destination directory; a nonempty destination that appears during staging must remain untouched and cause failure. For Windows, preserve existing-empty destination compatibility by removing only an empty destination immediately before rename if the OS cannot replace it; remove_dir must fail for occupied directories. Never recursively remove or replace the destination. Check actual rename behavior against these rules; if it cannot provide no-overwrite of occupied state on a supported platform, report the concrete gap before changing the contract.
- Stage ownership cleanup removes only its private directory on ordinary pre-publication failure. Process termination may leave a recognizable stage but no new ready workspace at the destination. After successful publication, directory-flush or reopen failure must preserve the complete published directory. Flush the destination parent on Unix and reopen the final Store only after publication.
- Reuse current backup/validation helpers where useful without introducing a generic filesystem framework. A focused `restore.rs` module and internal staged restore type are appropriate. Keep source bytes, caller-owned nonempty destinations, receipts, candidates, assets, guards and recovery buffers unchanged.

## Implementation and verification

- [x] Independent read-only GPT-6.1 Sol plan review approved after adding no-CREATE/same-connection validation and nested-parent compatibility.
- [x] Add behavioral red tests: existing marker + removed database must error without recreating anything; stage preparation must not expose a valid empty destination.
- [x] Implement missing-storage refusal and staged restore with unchanged public API/dependencies/schema.
- [x] Deterministic tests cover success/reopen with complete prior backup witnesses, initially empty destination, destination becoming nonempty after staging, invalid/corrupt/incompatible source, copy/publication failure cleanup, stage abandonment, preserved source bytes, missing database with retained WAL/SHM evidence, and relative source/absolute destination rules, nested nonexistent parents, and existing-marker connection refusal without CREATE or schema initialization. Exercise production stage/publish seam, not a duplicated algorithm. No global cwd changes or privileged-only success claims.
- [x] Run focused restore/open tests, complete core/app suites, CLI full backup/restore and missing-storage refusal, strict affected all-target Clippy, formatting and docs links. Use fresh synthetic directories only.
- [x] Independent candidate review; fix supported findings and rerun affected checks. Commit, publish, merge after required CI and verify main. Record exact source and evidence; no release.

## Gates and remaining work

Plan review approved; implementation and local checks complete. Independent candidate review approved; merged in PR #119 after all four required checks passed. Main verification is recorded in the next [backup-folder evidence](phase1-backup-folder-evidence.md). Current schema stays3; unsupported versions already fail before mutation. Implement real transactional migration and pre-upgrade orchestration with an actual adopted-schema change, not a fabricated version bump. Remembered external backup destination and broader native/recovery qualification remain separate work.
