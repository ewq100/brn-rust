# Bounded backup process-stage witness

- Branch: `codex/backup-process-witness-bounds`
- Base: `79fa3980c5022e475b07888d9e08f396f45b0238`
- Candidate commit: `dc16973f711828ae8fc3c7923ab150bd1091f963`
- Changed files: `crates/brn-threads-core/src/backup.rs`, `restore.rs`, `lib.rs`, and new `process_test.rs` only.

The backup process witness now re-enters its existing exact parent test in child mode via `BRN_TEST_BACKUP_PROCESS_STAGE_ROOT`; it no longer needs an ignored test target or stdout parsing. The child atomically renames a private ready marker containing the stage path's raw OS bytes, holds the complete snapshot, and has a 30-second fail-safe. The parent suppresses all child streams, polls readiness for at most 10 seconds while checking early exit, and owns the process through `OwnedChild`, which kills and reaps on unwind. It asserts signal 9, exact staged snapshot persistence at SIGKILL, absent child publication path, released OS lock, successful public backup retry removing the abandoned full stage, valid published backup with the source Note sentinel, and preserved child-source `DATA_MARKER`.

Extracted the existing Unix test-only restore `OwnedChild` into private `#[cfg(all(test, unix))]` `process_test.rs`, with `spawn`, `try_wait`, `kill_and_reap`, and Drop reaping. Restore now uses that same owner without raw Child access. Its existing 10-second readiness poll, early-exit check, 30-second child fail-safe, full stage/lock checks, and restore retry behavior remain in place.

Retirement map: removed only backup's ignored `stage_holder_child`, `BRN_STAGE_CHILD_PARENT`, 60-second sleep and `BufRead`/piped stdout readiness path. Retained the distinct same-process `a_live_stage_survives_another_backup` witness unchanged. No production code or backup/restore behavior was changed.

Formatting and whitespace only: `rustup run 1.98.1 rustfmt --edition 2024 crates/brn-threads-core/src/backup.rs crates/brn-threads-core/src/restore.rs crates/brn-threads-core/src/lib.rs crates/brn-threads-core/src/process_test.rs`; `git diff --check` passed. Not run: any build, test, CLI workload, provider, account, native, network, SQL or dependency operation. Lead owns shared-target execution and the negative control. Worktree clean after the candidate commit.
