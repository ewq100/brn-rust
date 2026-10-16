# Review: lock-marked backup and restore stages, with a sweep for abandoned ones (round 5 slice A)

Scope: `e07354db` against `f519534e`. I read the diff, the commit message, `round5/verification.md` and the slice A plan. I did not re-run any suite or spot-check, because the recorded evidence matched the source on every point I could check.

A SIGKILLed backup or restore left a hidden `.brn-backup-stage-*` or `.brn-restore-stage-*` folder that nothing removed, and a restore stage is a full copy of BRN data. Now each stage holds a `.brn-stage` marker. The marker's text names the stage's kind, and the creating process holds an OS lock on it for the stage's lifetime, the same pattern `run_lease` uses. The next backup (or restore) into the same folder removes direct-child stages of its kind only after nine checks pass, and removes them while holding the lock. Restore now stages the workspace at `<stage>/workspace`, so the marker is never published.

Watch for: staging now requires advisory-lock support, so backup or restore to a lock-less mount fails where it used to work (likely). Anything nested inside an abandoned stage folder is deleted with it (confirmed mechanics, unusual trigger). Verification ran on macOS only, while CI also runs the new non-Unix-gated tests on Windows and Linux (confirmed).

**Verdict**: APPROVED

## High-level view

The stage rules the brief asked about hold in the code. A stage is removed only if it has BRN's exact name, is a real directory, has a regular-file marker with matching identity and BRN's exact marker text for that kind, and its lock is free. The sweep never follows a symlink and never goes beyond direct children of the destination's parent. Final backups are files, so they fail the directory check. A stage killed after publication holds only an extra hard link to the backup, and removing the stage drops just that link. Restored folders never carry a marker. `StageDir` removes its folder while still locked. Combined with "empty marker means keep", that means no create/sweep interleaving removes a live stage.

There are two edge-case gaps. `StageDir::create` now fails when the OS refuses the lock, a new refusal for filesystems that support hard links or rename but not `flock`. And the sweep removes a stage's whole tree without looking inside it. Both fail toward an error or lost nested content, never toward losing the source workspace.

The evidence matches the source. E1 to E8 cover every new test. Core has 36 `#[test]` attributes at HEAD against 25 at the base, one ignored, which matches "lib 35 passed, was 25". All nine checks are in the committed `stage.rs`, and no `FAIL-BEFORE` marker remains. The regression tests' fail-before is E1 (sweep disabled after the fix), not the plan's run against unfixed code. That behaves the same for cleanup, and the method is written down.

The slice touches no SQLite state, shared write service, project, protected-note, candidate, version, dirty-guard, replay or Undo path. The sweep cannot reach `derived/run-leases`, because backup folders must be outside the workspace and only stage-named direct children match. Desktop, CLI, `main.rs`, `docs/status.md`, `docs/architecture/*`, the schema and `Cargo.lock` are unchanged.

<details>
<summary>Issues (6)</summary>

1. **Lock is now mandatory for staging**: on a mount where `flock` fails (for example NFS with `nolocks`, which returns ENOTSUP), `mark()` fails and so does the whole backup or restore. If `lock()` returns `Unsupported`, either keep the stage without a marker (the sweep then skips it, which matches old behavior), or document the requirement next to the hard-link requirement in the README.
2. **Nested content is deleted with an abandoned stage**: the sweep removes the whole tree without checking what is inside. A backup file or restore target placed in a stage folder, or a data folder opened from a killed restore's `<stage>/workspace`, is deleted by the next sweep in that parent. Refuse backup and restore destinations with a stage-named ancestor in `create`/`prepare`, or document it.
3. **A partial removal can become permanent debris**: `remove_dir_all` deletes in directory order, so an I/O error partway through can remove `.brn-stage` before the rest, leaving a markerless stage no later sweep will remove. Remove the marker last, or record this as a limitation.
4. **The SIGKILL test can hang or orphan its child**: the parent reads child stdout with no deadline, and a panic between `spawn` and `kill` leaves the child asleep for 60 s holding a stage in a tempdir being deleted. Kill the child from a drop guard and bound the read.
5. **Verified on macOS only**: `a_stage_is_private_marked_locked_and_removed_on_drop`, `an_abandoned_stage_keeps_its_folder_and_frees_its_lock` and the edited restore/backup cleanup tests also run on the Windows and Linux CI legs. There, `LockFileEx` semantics and Drop's remove-while-locked fallback are untested. State macOS-only in `verification.md`, and treat the CI headless matrix as the gate for those platforms.
6. **The no-published-marker assertion was never seen failing**: `staging_never_exposes_ready_destination` now asserts that the restored folder has no `.brn-stage`. No experiment stages the workspace directly in the wrapper to show that assertion failing. Optional: add that experiment to `verification.md`.

</details>

<details>
<summary>Details</summary>

### Marker-as-lock lifecycle and the lock-support requirement

```
creator                                  sweeper (another backup into the same parent)
mkdir <parent>/.brn-backup-stage-<uuid>  read_dir(parent) → name/keep/type checks
create_new .brn-stage (empty)            lstat marker → open rw → dev/ino match
lock() ── blocks while a sweep holds it  try_lock() → WouldBlock ⇒ keep
write "BRN backup stage v1\n", sync      read ≤ len+1 bytes; empty/foreign ⇒ keep
… stage lifetime, lock held …            lstat folder again; dev/ino match
Drop: remove_dir_all while locked        remove_dir_all while locked
      unlock; retry removal if needed
```

Two choices here are not obvious. The creator's lock blocks while the sweep's is non-blocking, so a sweep that gets in before the marker text exists reads it empty and backs off, and the creator then simply waits. And the in-process live-stage tests rely on two descriptors in one process conflicting. That works with std's `flock`-backed lock and would not with POSIX record locks.

Lock support is now required: `mark()` returns a `lock()` failure, and `StageDir::create` turns it into a backup or restore error (confirmed in code). Before this change, a backup needed only hard links and a restore needed only rename. A mount that supports those but refuses advisory locks, such as macOS NFS mounted with `nolocks`, would now fail every backup and restore (likely; not tried on a real mount). This fails closed, with no partial file, but the README does not mention the new refusal. A cheap fallback is to treat `ErrorKind::Unsupported` from `lock()` as "stage unmarked". The sweep then finds no marker text and skips the stage, so the old behavior returns on those filesystems.

### What `sweep_abandoned` removes along with a stage

Core has no `libc`, so the sweep cannot open with `O_NOFOLLOW` the way `run_lease` does. Instead it compares the marker's dev/ino before and after opening, and it re-checks the folder's dev/ino before `remove_dir_all`, which removes a symlink itself and never follows one.

The sweep does not look inside a stage before removing it (confirmed). Anything nested in an abandoned stage goes with it. That includes a backup file the owner wrote into `.brn-backup-stage-<uuid>/` and a restore published to `<abandoned restore stage>/workspace`, which is possible after a kill that followed the rename. Most plausibly, it includes a data folder opened directly from a killed restore's `<stage>/workspace` to rescue the copy. BRN holds no long-lived OS lock on a data folder, so that workspace would be removed while in use. Each trigger needs the owner to choose a hidden, UUID-named BRN folder on purpose, so this is not blocking. But "never deletes a restore target or live workspace" only holds outside stage folders. Refusing destinations whose ancestors include a stage-prefixed name, in `StagedBackup::create` and `StagedRestore::prepare`, closes the backup and restore cases. A data folder opened from a stage needs the same check in `Store::open` or a README line.

`remove_dir_all` deletes in directory order. An error partway through, such as an EACCES subtree or EIO, can delete `.brn-stage` first and leave a markerless partial stage that every later sweep skips. Sweep errors are silent by design, so the owner gets no notice of that debris. Deleting the other entries first and the marker last would keep a failed removal retryable.

### Fail-before evidence against the committed source

The commit message, `verification.md` and the source agree. The workspace went from 732 to 742 passed, an increase of ten that matches the ten new non-ignored tests. Ignored went from 5 to 6 (`stage_holder_child`), and the only failure is the baseline Poppler test. The experiment results follow from their edits. E8 removes the UUID-spelling and `keep` checks, so the simple, uppercase and `keep` entries go too: 4 removed instead of 1. With E3, `remove_dir_all` deletes the stage-named symlink itself while the outside folder stays intact, which is the "folder tree changed" failure recorded. E6's "2 + 17 others" adds up to the 19 recorded. The committed source keeps the `parent` (not grandparent) sweep, Drop's removal and `abandon()` dropping the lock.

The plan's A4 asked for the regression tests to run first against unfixed code with hand-built leftovers. E1 stands in for that, and under E1 the real-SIGKILL test reproduces the original bug end to end. Two gaps remain. No experiment shows the new no-marker-in-the-restored-folder assertion failing, because none stages the workspace directly in the wrapper. And no raw test logs were saved for round 5: the evidence is summary tables only.

### Test coverage and platform reach

Not tested: a lock failure during `mark()` (the lock-less filesystem case); destinations nested inside a stage; a removal that fails partway; Windows and Linux behavior. The Windows and Linux CI legs will run `a_stage_is_private_marked_locked_and_removed_on_drop`, `an_abandoned_stage_keeps_its_folder_and_frees_its_lock` and every edited cleanup test. That includes `invalid_snapshot_and_abandoned_stage_clean_up_without_final_file`, which asserts an empty root after a failed `create`. On Windows, that depends on Drop's remove-while-locked path, with its retry after unlock, working under mandatory locks. On Windows, the `<stage>/workspace` layout keeps the open marker handle out of the folder being renamed. Even so, all the evidence comes from macOS, and `verification.md` does not name the OS.

The SIGKILL test reads child stdout until `BRN_STAGE_READY` with no timeout. A child that stalls before that line blocks the parent until the 30-minute CI job timeout. A parent assertion failure between `spawn` and `kill` leaves the child sleeping 60 s, holding a stage in a tempdir that is being deleted. A drop guard that kills and reaps the child would make failures clean.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/stage.rs`: new. `StageKind`, `StageDir` (create, lock, drop-while-locked), Unix `sweep_abandoned` with nine checks, test helpers `stages`/`tree`, three unit tests.
- `crates/brn-threads-core/src/backup.rs`: `StagedBackup` holds a `StageDir` instead of its own `Drop`, sweeps before staging. Five new tests plus the ignored SIGKILL child; existing tests updated for the marker.
- `crates/brn-threads-core/src/restore.rs`: the stage wraps `workspace/`, publish renames only the workspace, sweep in `prepare`. Two new tests; `assert_stage_gone` applied to existing cleanup tests.
- `crates/brn-threads-core/src/lib.rs`: `mod stage;`.
- `crates/brn-threads-core/README.md`: marker, sweep and the markerless-stage limitation.
- `docs/work/active/daily-workflow-reliability/round5/plan.md`: new round 5 plan (slices A to H).
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: new. Baseline, plus the slice A change, fail-before table, commands and limitations.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability diff f519534e e07354db`.

</details>
