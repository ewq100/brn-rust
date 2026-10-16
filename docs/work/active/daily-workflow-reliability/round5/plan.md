# Round 5 plan: eight daily-workflow reliability slices (A–H)

**Branch:** `work/daily-workflow-reliability` at `f519534e` (base `771116f0`), worktree `/Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability`. Keep the backup ref `work/daily-workflow-reliability-pre-rebase`. Local commits only: no push, PR, merge, fast-forward of `main` or release.

**Owner request (authoritative):** "can you fix all 5 . and let go wiht interrupted". The orchestrator turned that into the eight owner-prescribed slices below. Slice D is the owner's U9 decision ("go with interrupted"). This plan follows the slice specs and records the design choice each one leaves open.

## Shared rules for every slice

- **Environment.** Run every command from the worktree root with this prefix. The cwd of agent shells is the main repo, so always use absolute paths or `git -C`.
  ```sh
  cd /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability
  export PATH=/opt/homebrew/opt/rustup/bin:$PATH CARGO_TARGET_DIR=/tmp/brn-dwr-target
  ```
  Every cargo command uses `--locked --offline`. Add no dependency and no dependency edge, and keep `git diff --exit-code Cargo.lock` clean. `brn-threads-core` has no `libc`, so core code uses `std` only.
- **Data.** Tests use `tempfile` under the canonical temp dir, as existing tests do. Any manual store goes under a fresh `/private/tmp/brn-dwr-data/r5-<case>/`; BRN correctly refuses the `/tmp` symlink. Use no owner data, credentials or live provider. Use scripted Rig models (`run_prepared_scripted`, `run_prepared_scripted_then_abandon`) and offline settings (`auth_file`/`credentials_dir` `None`, so runs fail as `ReconnectNeeded`).
- **Invariants to preserve.** One shared checked write service; SQLite owns state and `search.sqlite` is derived. Also project boundaries, protected-note authority, exact-candidate review, version checks, dirty guards, operation replay, Undo (refusals stay refusals), and "a saved reply never implies delivery". **No `threads.sqlite3` schema change in any slice.** If a slice seems to need one, the coder pauses with `send_message` severity `warning` and justifies it.
- **Ownership.**
  - Never touch `/Users/evokessler/repos/brn-rust/.worktrees/ui-polish`.
  - Desktop production code changes only in slice E: two lines of call plumbing in `useful_intake`, which the slice spec allows. Any other desktop production edit needs a pause naming the file, change and reason. Desktop tests go in `crates/brn-desktop/src/threads_intake_tests.rs`.
  - Do not edit `docs/status.md`. Leave `docs/architecture/*` as integration items, with proposed text in `round5/verification.md`.
  - `crates/brn/src/main.rs` gets one line in the whole round (slice E). The `HELP` constant and exit codes stay unchanged. New CLI tests go in new files under `crates/brn/tests/` (not `threads.rs`), per the cli-agent-contract anti-conflict rules.
- **Fail-before discipline.** For each slice, write the slice's tests first and run them against the unfixed code. Record the failing assertion and message, then implement until they pass. Guard tests that pass before the fix (for example "a live stage survives") get their fail-before by temporarily breaking the named safety check after the fix. Record that check and its failure, then restore the source and confirm `git diff` shows only intended changes.
- **Per-slice gate** (run at the end of each slice, in addition to the slice's focused tests):
  ```sh
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --locked --offline -- -D warnings
  cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings
  ```
  Desktop slices (B, C, D, E, and any slice touching `threads_intake_tests.rs`) also run:
  ```sh
  cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop
  cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings
  ```
- **Evidence.** Each slice appends its section to `docs/work/active/daily-workflow-reliability/round5/verification.md` (created by slice A). Each section holds the change, files, its fail-before/pass-after table, its commands and results, limitations, and integration items. Commit each slice locally once its gate passes, with a message like `fix: <slice summary> (round 5 slice X)`.
- **Baseline (before slice A).** Run `cargo test --workspace --locked --offline --no-fail-fast` and the desktop test command once, and record the counts. Round 4 recorded workspace 721 passed / 12 failed (the `brn-threads-intake` sandbox tests) / 5 ignored, and desktop 152 passed / 1 ignored. With the sandbox off, 11 of the 12 passed natively. Count a failure as the known sandbox class only if it is one of those 12 tests.

Slices are listed in dependency order:
- B before E: E matches sources with B's `same_locator`.
- C before D: both touch the settings path and `killed_run_candidate.rs`.
- The others are independent.

---

## A. Clean up abandoned backup and restore stage folders

### What the code does today

- `StagedBackup::create` (`crates/brn-threads-core/src/backup.rs`) creates `<parent>/.brn-backup-stage-<uuid v4>` (mode 0700), writes `snapshot.sqlite3`, and `publish` hard-links it to the destination. `Drop` runs `remove_dir_all(stage)`. So every in-process error path already removes its own stage. Only process death (SIGKILL) leaves one behind. Upgrade (`upgrade.rs:78`) uses the same `StagedBackup`.
- `StagedRestore::prepare` (`restore.rs`) creates `<parent>/.brn-restore-stage-<uuid>` (0700). `copy_from` runs `Store::open(stage)` inside it, which creates the `DATA_MARKER` and the database, and `publish` **renames the stage directory itself** to the destination. `Drop` removes the stage. Two consequences:
  - A marker file placed inside today's restore stage would be published into the restored workspace.
  - `Store::open` refuses unknown entries ("nonempty directory without Threads marker").
- Nothing ever removes another process's stage. The existing tests use `.brn-*-stage-not-owned` folders that must survive.

### Decisions

1. **A stage is a wrapper folder with a marker that is also its lock.** New `crates/brn-threads-core/src/stage.rs` (`mod stage;` in `lib.rs`, crate-private) owns the stage for both kinds:
   - `StageDir::create(parent, kind)`:
     - `mkdir` `<parent>/<prefix><uuid v4>` with mode 0700.
     - `create_new` the marker `<stage>/.brn-stage` with mode 0600.
     - `File::lock()` (std, blocking) on it, write the kind's marker text (`"BRN backup stage v1\n"` or `"BRN restore stage v1\n"`), and `sync_all`.
     - The lock is held for the stage's lifetime and the OS releases it on SIGKILL, as with `run_lease`.
     - Any error after `mkdir` removes the folder before returning.
   - `Drop` runs `remove_dir_all(stage)`, then unlocks.
   - Rationale: the lock answers "is it live?", and the marker text answers "is it BRN's stage?". One file does both, and no extra entry is added to the parent.
2. **The restore stage holds the workspace in a subfolder.** The workspace is staged at `<stage>/workspace`, created with 0700 before `Store::open`, so the restored folder keeps today's 0700 mode. `publish` renames `<stage>/workspace` to the destination; the stage and destination share a parent and filesystem, so the rename stays atomic. Then `Drop` removes the wrapper.
   - The marker is therefore never published.
   - A kill after the rename but before cleanup leaves only the wrapper (marker, no data), which the next restore removes.
3. **The sweep is conservative.** `stage::sweep_abandoned(parent, kind, keep: &Path) -> usize` is best effort: errors skip the entry and never fail the backup or restore. It is Unix-only; elsewhere it returns 0. For each **direct child** of `parent` (no recursion, no other folder), it removes the child only if every check passes:
   1. The name is exactly the kind's prefix plus a UUID that `uuid::Uuid::parse_str` accepts and whose hyphenated lowercase form equals the suffix. `.brn-backup-stage-not-owned` fails this check.
   2. The child's path is not `keep` (the requested destination).
   3. `entry.file_type()` is a real directory, not a symlink. `symlink_metadata` gives its dev/ino.
   4. `symlink_metadata(<child>/.brn-stage)` is a regular file.
   5. The marker opened read/write without create has the same dev/ino (`MetadataExt`) as the `lstat` result. This guards against a swap without `O_NOFOLLOW`.
   6. `try_lock()` succeeds. `WouldBlock` means live: skip.
   7. While holding the lock, the marker content equals the kind's text exactly. An empty marker (a creator that has not yet written) or a foreign text means skip.
   8. Re-checked `symlink_metadata(child)` is still a directory with the same dev/ino.
   9. `remove_dir_all(child)` runs while the lock is held. Std's `remove_dir_all` does not follow symlinks inside the tree.
   - Backup sweeps only backup stages, and restore only restore stages, each in the destination's parent. Published backups (files) and restore targets (no marker, user names) can never match.
4. **Sweep placement.**
   - Backup: in `StagedBackup::create`, after the symlink-ancestor and occupied-destination checks, before creating its own stage.
   - Restore: in `StagedRestore::prepare`, after `create_dir_all(parent)` and the symlink re-check.
   - The upgrade pre-snapshot inherits the backup behavior.
5. **Old stages without a marker** (from earlier builds, including the 7 stages in `evidence.md`) are never removed automatically. Recorded limitation: the owner may delete them by hand. A stage killed between `mkdir` and its marker write (an empty folder) is also left.

### Items

- [ ] A1. Create `crates/brn-threads-core/src/stage.rs` with `StageKind { Backup, Restore }` (`prefix()`, `marker()`), `STAGE_MARKER = ".brn-stage"`, `StageDir { path, lock: File }` (`create`, `path`, `Drop`), `#[cfg(test)] fn abandon(self)` (drops the lock and keeps the folder, which simulates SIGKILL in-process), and `sweep_abandoned`. Declare `mod stage;` in `crates/brn-threads-core/src/lib.rs`.
  Files: `crates/brn-threads-core/src/{stage.rs,lib.rs}`.
  Verify: `cargo test -p brn-threads-core --locked --offline stage::` passes.
- [ ] A2. Backup on `StageDir`.
  - `StagedBackup` holds `stage: StageDir`, with `snapshot = stage.path()/snapshot.sqlite3`. Remove its own `Drop`, since `StageDir` cleans up. Call `sweep_abandoned(parent, Backup, &destination)` before staging.
  - Update the unit tests: `wal_snapshot_remains_standalone…` now expects 2 entries in the stage (snapshot and marker), and tests that read `stage.directory` use `stage.path()`.
  Files: `crates/brn-threads-core/src/backup.rs`.
  Verify: `cargo test -p brn-threads-core --locked --offline backup::` passes.
- [ ] A3. Restore on `StageDir`.
  - `StagedRestore { stage: StageDir, directory: <stage>/workspace, destination }`. `prepare` sweeps, creates the stage, and creates `workspace` with 0700. `copy_from` opens `Store::open(&self.directory)`. `rename_directory` (including the `cfg(windows)` fallback) renames `self.directory`.
  - Update the unit tests: `staging_never_exposes_ready_destination` asserts `staging.parent().unwrap().parent() == destination.parent()` and the mode on the workspace folder. Every `assert!(!staging.exists())` also asserts the wrapper is gone.
  Files: `crates/brn-threads-core/src/restore.rs`.
  Verify: `cargo test -p brn-threads-core --locked --offline restore::` passes.
- [ ] A4. Tests (write first; the fail-before is noted per test).
  - In `backup.rs` `mod tests`:
    - `a_killed_backups_stage_is_removed_by_the_next_backup_to_that_folder`: `StageDir::create(..).abandon()` twice, then `store.backup(dest)` in the same parent. Both are gone, the published backup validates, and `.brn-backup-stage-not-owned` survives. *Fail-before:* the leftovers remain. Build the leftover by hand (mkdir, marker with backup text, no lock) when running against the old code.
    - `a_sigkilled_backup_process_leaves_a_stage_the_next_backup_removes` (`#[cfg(unix)]`):
      - The parent test spawns `std::env::current_exe()` with `--exact backup::tests::stage_holder_child --ignored --nocapture --test-threads=1` and env `BRN_STAGE_CHILD_PARENT=<dir>`.
      - The `#[ignore]` child creates a `StagedBackup` from a fresh store, prints `BRN_STAGE_READY`, and sleeps 60 s.
      - The parent reads stdout until that line, `child.kill()`s it (SIGKILL), and `wait()`s.
      - It asserts the stage is present with its marker and a free lock, then that the next `store.backup` removes it.
      - *Fail-before:* the stage remains.
    - `a_live_stage_survives_another_backup`: hold a `StageDir`, run `store.backup(other)` in the same parent, and the stage and marker are intact. *Fail-before:* skip step 6 of the sweep; the test then fails.
    - `stage_named_entries_that_are_not_brn_stages_are_never_followed_or_removed`. In one parent:
      - a symlink `.brn-backup-stage-<uuid>` pointing outside to a folder that holds a valid marker;
      - a real folder with that kind of name and no marker;
      - a folder whose marker has the restore text;
      - a folder whose marker is a symlink;
      - a regular file with a stage name;
      - `.brn-backup-stage-not-owned`.
      After `store.backup`, all remain byte-identical and the outside folder is untouched. *Fail-before:* drop the symlink checks (step 3) and the marker-text check (step 7) in turn.
    - `a_backup_sweeps_only_its_own_parent`: an abandoned stage in a sibling folder survives a backup into another folder.
    - Error path: extend `invalid_snapshot_and_abandoned_stage_clean_up_without_final_file` to assert no `.brn-backup-stage-` entry remains after the failed `create`.
  - In `restore.rs` `mod tests`:
    - `a_killed_restores_stage_is_removed_by_the_next_restore_into_that_parent`: two abandoned restore stages (one with a full `workspace/` copy, one wrapper-only), plus an earlier published restore target in the same parent. The next `Store::restore` removes both stages. The earlier target, the backup file and `.brn-restore-stage-not-owned` are unchanged.
    - `a_live_restore_stage_survives` (held `StageDir`).
    - Extend `abandoned_stage_and_sqlite_copy_failure_clean_up_without_publication`: after the failed `copy_from`, the parent has no `.brn-restore-stage-` entry.
  Files: `crates/brn-threads-core/src/{stage.rs,backup.rs,restore.rs}`.
  Verify:
  - `cargo test -p brn-threads-core --locked --offline` passes.
  - `cargo test -p brn-threads-app --locked --offline --test product --test backup_directory` passes (it asserts no stage remains after success).
  - `cargo test -p brn --locked --offline` passes (`tests/threads.rs` asserts no backup or restore stage after CLI backup and restore).
- [ ] A5. Create `round5/verification.md` with the baseline section and slice A's section. Include the limitation (markerless stages from older builds are kept) and the rule set above.

### Integration notes

There is no API change outside the core crate, and the publish semantics are unchanged. One on-disk change: a restore stage now holds `workspace/`, and the final restored folder is identical (still 0700). The UI branch is unaffected.

---

## B. Count one file as one source regardless of path spelling

### What the code does today

- Desktop and CLI pass the as-typed or picked path as `ConversionResult.source_reference`, and from there as `TransientSource.reference`.
- `begin()` (`crates/brn-ai/src/threads.rs` ~2026) finds an existing Source with `s.locator == *reference && s.chat == source_chat`. It writes `Source { locator: reference, label: reference, … }` when the Source is new or `report` is set.
- `Workspace::source_intake_chat` (`crates/brn-threads-app/src/intake.rs`) collects Sources by `s.locator == locator`.
- `import_full_scoped` matches `source.locator == converted.source_reference`, and `keep_in_brn` matches global Sources by exact locator.
- The runtime's own source checks (`threads_sources.rs` :40, :56, :166, :319, :344) compare `source.locator` with the run's `reference` exactly. They stay correct as long as the run's reference and the Source's stored locator agree.
- Uniqueness is the `source_locator` index on `(COALESCE(chat,''), locator)` (`upgrade.rs`).

### Decisions

1. **One canonicalization helper in the core**, used by `brn-ai` and `brn-threads-app` alike. Add `crates/brn-threads-core/src/locator.rs`, exported at the crate root:
   - `pub fn canonical_locator(locator: &str) -> String`: on Unix only, if the locator is an absolute path (`/…`, no URL scheme), `fs::canonicalize` succeeds, the target is a regular file and the result is UTF-8, return the canonical path. Otherwise return the locator unchanged. So `synthetic://`, `https://`, `outlook:`, relative, missing, non-UTF-8 and directory locators all stay as they are.
   - `pub fn same_locator(stored: &str, canonical: &str) -> bool`: `stored == canonical || canonical_locator(stored) == canonical`.
2. **Canonicalize at the service entry points, not in the hosts.** No CLI or desktop change is needed.
   - `brn_ai::threads::prepare_run(mut request)` rewrites each `request.sources[i].reference` to `canonical_locator(..)` before `begin`. The prompt, `read_source`, the Scope's source map and the recorded Source then all use one spelling. `run()` goes through `prepare_run`. Tests that call `begin()` directly use synthetic references, which do not change.
   - `source_intake_chat` canonicalizes its `locator` argument. `import_full_scoped` uses `canonical_locator(&converted.source_reference)` for the Source locator and matching. Its operation binding still hashes `converted` as given, so retry identity is unchanged.
3. **Old non-canonical Sources keep matching.** Every Source lookup prefers an exact match on the canonical locator, then falls back to `same_locator(stored, canonical)`.
   - When the match is an old spelling, the checked change that already writes this Source records the canonical locator. That is the Source Put in `begin()`'s "Start a fresh invocation" change, or in the full-import change. In `begin()` this Put is now also written when only the locator changed (report `None`); the existing record is cloned and only `locator` and `observed_at` are updated.
   - This is a new revision, so history is preserved. It cannot collide on the unique index, because an exact canonical match in the same chat scope would have been chosen first.
   - `source_intake_chat` counts every Source that matches through `same_locator`, so old duplicates (one `/tmp`, one `/private/tmp`) all lead to the same chat.
4. **Label.** A new Source's `label` is the canonical path, as today it is the reference. A real path is a sensible label: the desktop shows `short(locator)` and opens the locator with the system. An existing Source keeps its own label unless it receives a report, as today.

### Items

- [ ] B1. Add `crates/brn-threads-core/src/locator.rs` (`pub mod locator;` plus `pub use locator::{canonical_locator, same_locator};` in `lib.rs`) with unit tests:
  - URL, `synthetic://`, relative, missing and directory locators are unchanged;
  - a file reached through a symlinked folder maps to its real path;
  - `/tmp/x` maps to `/private/tmp/x` (`#[cfg(target_os = "macos")]`);
  - `same_locator` matches an old spelling and rejects a different file with the same name.
  Files: `crates/brn-threads-core/src/{locator.rs,lib.rs}`.
  Verify: `cargo test -p brn-threads-core --locked --offline locator` passes.
- [ ] B2. Runtime: canonicalize in `prepare_run`. In `begin()`, do the two-step lookup (exact, then `same_locator`), record the canonical locator on an old-spelling match (Decision 3), and keep the label rule.
  Files: `crates/brn-ai/src/threads.rs`.
  Verify: `cargo test -p brn-ai --locked --offline` passes.
- [ ] B3. App: `source_intake_chat` canonicalizes and matches through `same_locator`. `import_full_scoped` uses the canonical locator, matches exact first, and records the canonical locator on an old-spelling match. `keep_in_brn`'s two global-Source lookups use `same_locator(&s.locator, &canonical_locator(&observation.locator))`.
  Files: `crates/brn-threads-app/src/intake.rs`.
  Verify: `cargo test -p brn-threads-app --locked --offline --test source_intake_chat --test host_intake --test import_coverage --test chat` passes.
- [ ] B4. Tests (write first; each fails before the fix with two chats or two Sources).
  - In `crates/brn-threads-app/tests/source_intake_chat.rs`, real files in a canonical temp dir, supplied with the file's existing `request()` and scripted-run pattern:
    - `the_same_file_through_a_symlinked_folder_reuses_one_chat_and_one_source`: supply `alias/mail.txt`, then `real/mail.txt`. `source_intake_chat` returns `(same chat, true)`, there is one Source record and its locator is the real path.
    - `tmp_and_private_tmp_spellings_reuse_one_chat_and_one_source` (`#[cfg(target_os = "macos")]`): `tempfile::tempdir_in("/private/tmp")`, spelled `/tmp/<name>/mail.txt`, then `/private/tmp/…`.
    - `a_different_file_with_the_same_name_gets_its_own_chat_and_source`.
    - `a_source_saved_under_an_older_spelling_still_matches_and_is_recorded_canonically`:
      - Seed a Source with the alias locator and a Run whose `sources` names it, through checked owner changes (`workspace.owner_change`, as the desktop tests seed `Run.operations`).
      - Supplying the real path reuses that chat and that Source ID.
      - The Source's latest revision has the canonical locator, and the earlier revision is still in history.
    - `full_import_of_one_file_through_two_spellings_observes_one_source` (`import_full` twice, the second through the alias). One Source, and the second outcome is "Observed previously imported full note".
  - `crates/brn/tests/source_locator.rs` (new file; copy the minimal command helpers and `HOME`/`CODEX_HOME`/`TMPDIR` isolation from `tests/threads.rs`):
    - `useful_intake_of_one_markdown_file_through_two_spellings_reuses_its_chat`: the second CLI `intake --intent useful` (through the symlinked folder) returns `reused_chat: true` and the same `thread`, and `records` shows one Source.
  - In `crates/brn-desktop/src/threads_intake_tests.rs`:
    - `supplying_one_file_through_two_spellings_reuses_its_chat`: a real temp file. `supply(cx, &f, alias)` then `supply(cx, &f, real)` return the same chat, and there is one Source with the canonical locator. Use a local helper rather than the `SOURCE`-based helpers.
  Verify:
  - `cargo test -p brn-threads-app --locked --offline --test source_intake_chat` passes;
  - `cargo build -p brn --locked --offline && cargo test -p brn --locked --offline --test source_locator` passes;
  - the desktop test command passes.
- [ ] B5. Append slice B to `round5/verification.md`.

### Integration notes

- New core exports `canonical_locator` and `same_locator`.
- New file Sources record real paths, so a `/tmp/...` supply now shows `/private/tmp/...` in Source rows. Tests that compare a real temp file path with `Source.locator` must use the canonical path. Synthetic locators are unchanged.
- No CLI or desktop production change.

---

## C. The remaining stop paths end lease-free Runs like orphans

### What the code does today

- Core `ChatCommand::Pause { run: Some }` (`crates/brn-threads-core/src/chat.rs` ~233), Archive/ArchiveProgress (~401) and Steer (`steering.rs` ~167) end Working Runs as `Interrupted` and call `ended_run_attention(.., notice = false, ..)`. The core cannot see leases: `run_lease` lives in `brn-ai` and uses `libc`.
- Through `Workspace::stop_chat_run`, the desktop reaches `Pause` from three places:
  - deliberate Stop (`cancel`, `threads_ui.rs` ~2633), with a live local worker;
  - `pause_failed_worker` (`threads_chat.rs` ~131), where the worker's lease is usually already gone, since `RunLease::drop` removes its file;
  - the resume-stale stop in `resume_chat_queue` (`threads_chat.rs` ~267), for a pending item's Working Run with no local worker.
- `begin()` (`brn-ai/src/threads.rs` ~1975) fences the chat's Working Runs with `notice = false`. The settings path (`store.rs` `update_workspace_settings`), project commands and project imports add the notice but never remove a lease file. Only `recover_orphaned_runs` calls `run_lease::remove_free` (after `Applied`).
- A lease that is free stays free: a Run's lease is taken only in `begin()` for a new Run ID, and helper sub-runs share the parent's lease. So a host-computed "lease-free" set cannot go stale in the unsafe direction. A Run that dies after the check is treated as live (today's behavior).

### Decisions

1. **The host passes liveness to the core as data.**
   - Add `Store::chat_command_with_orphans(op, command, owner, orphans: &BTreeSet<String>)` and `Store::stop_and_post_checked(op, run, thread, text, partial, owner, orphans)`. The existing `chat_command`, `stop_and_post` and `stop_and_post_with_partial` delegate with an empty set, so their behavior is unchanged.
   - `orphans` is not part of the operation binding. A replay returns the recorded receipt as today.
   - No dependency or schema change.
2. **Per-Run notice in the shared helper.** Add `run_attention::ended_run_attention_with(writes, ended, notice: impl Fn(&str) -> bool, record, active)`. `ended_run_attention(.., notice: bool, ..)` becomes a one-line wrapper (`|_| notice`), so every round 4 caller is unchanged. In Pause, Archive/ArchiveProgress and Steer, `notice = |id| orphans.contains(id)`.
3. **The service computes the set and removes free leases after the change.**
   - In `crates/brn-threads-app/src/chat.rs`, add `fn lease_free_working_runs(&self) -> BTreeSet<String>`: non-archived Working Runs with `!run_lease::run_is_live`. It is empty when `derived_directory()` fails (unknown liveness counts as live, as in `recover_orphaned_runs`).
   - Also add `fn remove_ended_leases(&self, receipt: &Receipt)`: for each `Written` whose `before` is a Working Run and whose `after` Run is not Working, call `run_lease::remove_free`. `remove_free` never removes a held lease, so it is safe on every path, including a live Run that the settings path stopped.
   - Wire them in:
     - `Workspace::chat_command`, which covers `stop_chat_run`, Archive and every queue command;
     - `stop_and_post_with_partial`;
     - the settings wrappers that can end Runs (`configure`, `choose_delegation`, `choose_backup_directory`, `pause`) on `ApplyOutcome::Applied`;
     - `project_command`;
     - the project branch of `import_full_scoped` on `Applied`.
     `recover_orphaned_runs` keeps its own call.
4. **`begin()`.**
   - For each Run it fences, compute `orphan = !run_lease::run_is_live(&derived, id)`, using the `derived` directory it already opens for the new lease. Call `ended_run_attention_with` with `|id| orphans.contains(id)`.
   - After the start change is `Applied`, call `remove_free` for the orphans.
   - Continue in a killed chat therefore adds one Interrupted notice in the same change. The new Run's own terminal result then replaces it: superseded, because the old Run is `Interrupted`.
5. **A live lease keeps today's behavior on every path.** A deliberate Stop, Archive or Steer of a live worker adds no notice and keeps its lease file, which the worker's own `RunLease::drop` removes. The settings path stops live Runs as before, with the notice from round 4 (state per slice D).
6. **This changes round 4's T4 expectations.** In T4 the killed Run is lease-free, so Stop, Archive and Steer now add the notice. Add a live-Run counterpart that asserts no notice. Record this as a justified contract correction in the verification doc.

### Items

- [ ] C1. Core helper and APIs.
  - Add `ended_run_attention_with` and turn `ended_run_attention` into its wrapper (`run_attention.rs`). Unit test `per_run_notice_marks_only_lease_free_runs`: two Runs in one chat, one orphan. One notice (for the orphan), Review for both.
  - Add `chat_command_with_orphans` (`chat.rs`; the Pause and Archive arms use the per-run notice) and `stop_and_post_checked` (`steering.rs`).
  Files: `crates/brn-threads-core/src/{run_attention.rs,chat.rs,steering.rs}`.
  Verify: `cargo test -p brn-threads-core --locked --offline` passes, and the existing round 4 receipts are unchanged when the set is empty.
- [ ] C2. App wiring (Decision 3).
  Files: `crates/brn-threads-app/src/{chat.rs,steering.rs,settings.rs,projects.rs,intake.rs}`.
  Verify: `cargo test -p brn-threads-app --locked --offline --test chat --test steering --test projects --test run_recovery --test delegation` passes.
- [ ] C3. Runtime `begin()` (Decision 4).
  Files: `crates/brn-ai/src/threads.rs`.
  Verify: `cargo test -p brn-ai --locked --offline` passes.
- [ ] C4. Tests (write first). In `crates/brn-threads-app/tests/killed_run_candidate.rs`, reuse `world()` and `killed_intake()`.
  - `stop_after_a_failure_treats_a_lease_free_run_as_interrupted_work`: the `pause_failed_worker` call, `stop_chat_run("worker-failure:…", run, Some("partial text"))`, on a killed Run. Expect:
    - Run `Interrupted` at fence 2;
    - attention exactly `[Review(op), Blocker(run, INTERRUPTED_ATTENTION)]`;
    - one `assistant-partial` Message;
    - the lease file gone;
    - a replay of the same key adds nothing.
    *Fail-before:* attention `[Review(op)]` and the lease file present.
  - `resume_after_a_killed_queued_run_leaves_one_notice_and_redispatches_once`. This is the desktop's exact call order, for spec problem (2):
    1. `queue_message` in the intake chat, then `dispatch_queued`, then `prepare_run` with `request_message = q.message` and `queue_claim = q.dispatch` (as `start_run` builds them, `threads_ui.rs` ~1959).
    2. The scripted `prepare_changes`, then `run_prepared_scripted_then_abandon`, then recreate the lease file.
    3. `stop_chat_run("resume-stale:…", run, None)`, then `resume_queue`, then `dispatch_queued`.
    Expect: one notice and the Review; the lease file gone; the pending item dispatched exactly once more with the same owner Message (no repost); the old Run `Interrupted` at fence 2.
  - `a_deliberate_stop_of_a_live_run_adds_no_notice_and_keeps_its_lease`: a held `PreparedThreadRun`, then `stop_chat_run("stop:…")`. Expect attention `[]`, `run_is_live` still true, the lease file present, and the Run `Interrupted`. This guard passes before the fix; its fail-before is to make `lease_free_working_runs` return every Working Run.
  - Update T4 `stop_archive_and_steer_keep_the_candidate_in_review_without_a_notice`. Rename it to `stop_archive_and_steer_of_a_killed_run_add_the_notice_and_remove_the_lease`: expect `[Review(op), Blocker(run)]` (the archived Thread write carries both) and the lease file gone, for each of stop, archive and steer. Add a live counterpart `stop_archive_and_steer_of_a_live_run_add_no_notice` (expect `[]`).
  - Update T1 and T1-live (settings) to assert the killed Run's lease file is **removed** (it was kept in round 4) and the live Run's lease is kept and still live.
  - Update T5 to assert the lease file is removed after the project change and after the project import.
  - Update T6 `continue_in_the_killed_chat_keeps_the_candidate_in_review`. Rename it to `continue_in_the_killed_chat_adds_the_notice_and_removes_the_lease`. After `prepare_run` in the killed chat, expect attention `[Review(op), Blocker(old run)]` and the lease file gone. After the new offline Run fails, expect `[Review(op), Blocker(new run, failed text)]` (one notice).
  - In `crates/brn-desktop/src/threads_intake_tests.rs`, add `resume_after_a_killed_queued_intake_shows_one_notice`: as the second test above, but through `ui.resume_chat_queue(cx)`. Afterwards the intake row's reasons are `[REVIEW_ATTENTION, INTERRUPTED_ATTENTION]`, the lease file is gone, and the item ran once more and failed offline.
  Verify:
  - `cargo test -p brn-threads-app --locked --offline --test killed_run_candidate --test run_recovery --test chat --test steering --test projects` passes;
  - `cargo test -p brn-threads-core --locked --offline --test delegation --test steering` passes;
  - the desktop test command passes.
- [ ] C5. Append slice C to `round5/verification.md`. Include the updated path table (round 4's table plus "lease-free ⇒ notice and lease removal" for Stop, failure stop, resume-stale stop, Archive, Steer and Continue; "lease removal" for settings, project and import) and the T4/T6 contract correction.

### Integration notes

- New core APIs `chat_command_with_orphans`, `stop_and_post_checked` and `ended_run_attention_with`. Old signatures are kept.
- No desktop production change: the desktop's existing `stop_chat_run` and `resume_queue` calls get the new behavior.
- The UI branch must rerun its Stop, Archive and Resume tests: a lease-free Stop now adds the notice.

---

## D. Owner decision U9: go with interrupted

### What the code does today

`RunState::Cancelled` has exactly two production writers:
- `update_workspace_settings` (`crates/brn-threads-core/src/store.rs` ~916), which adds the Interrupted notice since round 4;
- the legacy `Workspace::resolve_thread(_because)` (`crates/brn-threads-app/src/lib.rs` ~324), which has no production caller.

Every other notice-adding path already writes `Interrupted`: orphan recovery, project commands, imports, and slice C's lease-free stops. **Fact check on the spec:** a deliberate owner Stop of a live worker is `ChatCommand::Pause { run }`, which writes `Interrupted`, not `Cancelled`, and the fenced worker's `finish()` writes nothing. So "a deliberate Stop keeps Cancelled" is implemented as "a deliberate Stop keeps its current state", which is `Interrupted`. Nothing changes for Stop.

### Consumers checked (no change needed)

- Core queue `Finish` accepts `Failed | Interrupted | Cancelled` for `success = false` (`chat.rs` ~393).
- Supersession of terminal notices: `run_attention.rs` ~119, and `finish()` and orphan recovery treat Interrupted, Failed and Cancelled alike.
- Desktop `can_continue` (`threads_state.rs` ~104) and the run line, which renders `format!("{:?}", run.state)` (`threads_ui.rs` ~2804, `threads_writing.rs` ~301). It now shows "Interrupted" with no desktop change.
- `is_terminal_notice` is text-based. The orphan-recovery replay key uses record versions only. The CLI prints `AiTerminal`, not `RunState` (`main.rs` has no `RunState` text).
- Saved `Cancelled` Runs stay as they are; there is no migration.

### Items

- [ ] D1. In `update_workspace_settings`, write `RunState::Interrupted` instead of `RunState::Cancelled`. The reason text, fence bump and helper call are unchanged. Update the doc comment.
  Files: `crates/brn-threads-core/src/store.rs`.
- [ ] D2. Update the tests that assert the settings path's state (write the new expectations first; each fails before D1):
  - `crates/brn-threads-core/tests/delegation.rs` (~366, ~400);
  - `crates/brn-threads-core/tests/core.rs` (~2153, `pause_workspace` across connections);
  - `crates/brn-threads-app/tests/product.rs` (~857, `app.pause(true)`);
  - `crates/brn-threads-app/tests/killed_run_candidate.rs` (~282, ~330);
  - `crates/brn-desktop/src/threads_intake_tests.rs` (~408).

  Check `crates/brn-threads-core/tests/backup_directory.rs` ~153 (fence 1): if that Run is seeded `Cancelled` rather than stopped by the settings change, leave it. Leave the tests that seed states (`core.rs` ~957, ~1427, ~1463, `steering.rs` ~216) and the legacy resolve test (`thread_lifecycle.rs` ~71).

  Add `crates/brn-threads-core/tests/delegation.rs::a_settings_stop_records_interrupted_matching_its_notice`. The stopped Run is `Interrupted` at fence +1, and its chat holds `Blocker(run, INTERRUPTED_ATTENTION)`. Then a queue `Finish` with `success = false` for a stopped queued Run is accepted and pauses the queue.
  Files: the test files listed.
  Verify:
  - `cargo test -p brn-threads-core --locked --offline` passes;
  - `cargo test -p brn-threads-app --locked --offline --test product --test killed_run_candidate --test delegation --test thread_lifecycle` passes;
  - the desktop test command passes.
- [ ] D3. Append slice D to `round5/verification.md`: the consumer list above, the Stop fact check, and the proposed contract sentence: "A Run stopped because of something outside its chat (settings, project changes or imports, process ended, or a lease-free stop) is recorded as Interrupted, matching its notice."

### Integration notes

The UI branch shows "Interrupted" after a settings change with no code change. Its tests that expected `Cancelled` after settings need the new value.

---

## E. Reusing an intake chat must not repeat the owner instruction

### What the code does today

- The CLI useful-intake sub-block (`crates/brn/src/main.rs` ~869) calls `ask(..)` with the literal "Retain useful information from the supplied source. …". `ask` sets `request_message: None`, and its signature must stay (cli-agent-contract rule).
- The desktop `useful_intake` (`crates/brn-desktop/src/threads_ui.rs` ~2974) posts "Capture useful supported information from this source, …" with `post_message`, then `start_run`.
- `begin()` reuses an instruction Message only when `request_message` is set, or when `messages.last()` (any role) is an owner Message with exactly the prompt text (`already_posted`, ~1953). Otherwise it creates a new Message.
- So:
  - CLI then desktop gives two different owner instructions.
  - Desktop then desktop gives two identical ones, because `post_message` is unconditional.
  - A re-supply after an assistant reply also posts a new one.

### Decisions

1. **One shared instruction: the desktop's existing wording.** Add `pub const SOURCE_INTAKE_INSTRUCTION: &str` in `crates/brn-threads-app/src/intake.rs`, re-exported at the crate root like `import_report`, holding the desktop text verbatim. The desktop's visible text is unchanged. **Noted visible change:** CLI useful intake now posts the desktop wording, which drops the CLI's "Reconcile current notes and suggested Actions … ask for review of protected decisions". The packaged intake and maintain-notes playbooks, preloaded for every source run, already carry that guidance.
2. **One reuse rule, in the runtime, used by `begin()` and by the desktop helper.**
   - Add `brn_ai::threads::reusable_intake_instruction(records, ordered_chat_messages, chat, prompt, references) -> Option<String>`. It returns the ID of the chat's **latest owner Message** (durable receipt order, as `begin()` sorts) when all of these hold:
     - its text equals `prompt`;
     - a Run in that chat has `instruction == Some(that id)`;
     - that Run's `sources` include, for every supplied reference, a Source matching it through `same_locator` (from slice B).
   - Add a store-level wrapper `intake_instruction_to_reuse(store, chat, prompt, references) -> AiResult<Option<String>>` for hosts.
   - In `begin()`, when `request_message` and `writing_snapshot` are `None`, the request has sources and `already_posted` is false, a `Some(id)` sets `already_posted` with `instruction = id`. No new Message is written.
   - Because the rule needs the *latest owner* Message, new owner text in between (or a different source) still posts a fresh instruction. This also matches `begin()`'s own `request_message` check.
3. **The desktop posts only when needed.**
   - Add `Workspace::post_intake_instruction(&mut self, chat, locator) -> AppResult<Option<Receipt>>`. It returns `Ok(None)` when `intake_instruction_to_reuse(.., SOURCE_INTAKE_INSTRUCTION, [canonical_locator(locator)])` finds one. Otherwise it calls `post_message(chat, SOURCE_INTAKE_INSTRUCTION)`.
   - The desktop changes two lines in `useful_intake`, which the slice spec allows as call plumbing:
     - `let instruction = brn_threads_app::SOURCE_INTAKE_INSTRUCTION;`
     - `self.workspace.post_intake_instruction(&thread.id, &converted.source_reference)` in place of `post_message(..)`.
     The `Ok(_)` arm is unchanged.
4. **The CLI changes one line.** The `ask(..)` call passes `brn_threads_app::SOURCE_INTAKE_INSTRUCTION.into()`, inside the useful-intake sub-block, which cli-agent-contract committed not to edit. `begin()`'s rule then reuses the Message.

### Items

- [ ] E1. Add the constant and `post_intake_instruction`.
  Files: `crates/brn-threads-app/src/intake.rs` (and `lib.rs` only if the re-export needs it).
- [ ] E2. Add `reusable_intake_instruction` and `intake_instruction_to_reuse`, and use them in `begin()` (Decision 2).
  Files: `crates/brn-ai/src/threads.rs`.
  Verify: `cargo test -p brn-ai --locked --offline` passes.
- [ ] E3. The desktop two-line change and the CLI one-line change.
  Files: `crates/brn-desktop/src/threads_ui.rs`, `crates/brn/src/main.rs`.
  Verify:
  - `cargo build -p brn --locked --offline` passes;
  - `cargo test -p brn --locked --offline` passes;
  - the desktop test command passes;
  - `git diff --stat crates/brn/src/main.rs` shows 1 line changed.
- [ ] E4. Tests (write first).
  - In `crates/brn-threads-app/tests/source_intake_chat.rs`:
    - `a_cli_then_desktop_supply_of_one_source_leaves_one_instruction_message`:
      1. CLI shape: `source_intake_chat`, then `prepare_run` with `prompt = SOURCE_INTAKE_INSTRUCTION` and the source, then a scripted finish.
      2. Desktop shape: `source_intake_chat` reuses the chat, `post_intake_instruction` returns `None`, then `prepare_run`.
      The chat holds exactly one owner Message with that text, and both Runs' `instruction` name it. *Fail-before:* 2 Messages, because `SOURCE_INTAKE_INSTRUCTION` does not exist yet. Write the test with the desktop literal first, then switch to the constant.
    - `a_resupply_after_an_assistant_reply_reuses_the_instruction`: the first scripted run completes with text, so the last Message is `assistant`. The resupply posts no owner Message. *Fail-before:* 2.
    - `new_owner_text_in_between_posts_a_fresh_instruction`: `post_message(chat, "Focus on the berth depth")` between supplies. `post_intake_instruction` returns `Some`, and the chat holds 2 instruction Messages plus the owner text.
    - `a_different_source_in_the_same_chat_posts_its_own_instruction`.
  - In `crates/brn-desktop/src/threads_intake_tests.rs`:
    - `supplying_the_same_source_twice_posts_one_instruction`: `supply` twice. There is one owner Message with `SOURCE_INTAKE_INSTRUCTION` in the chat, and both `Failed` Runs name it. *Fail-before:* 2 Messages.
  - In `crates/brn/tests/source_locator.rs` (shared with B):
    - `two_cli_useful_intakes_of_one_source_post_one_instruction` (a guard; record whether it passed before the fix).
  Verify: `cargo test -p brn-threads-app --locked --offline --test source_intake_chat --test daily_workflow_eval` passes, the desktop test command passes, and `cargo test -p brn --locked --offline --test source_locator` passes.
- [ ] E5. Append slice E to `round5/verification.md`, including the CLI wording change.

### Integration notes

- `main.rs`: one line in the useful-intake sub-block. It must survive the merge with work/cli-agent-contract, which keeps that block untouched.
- `threads_ui.rs`: two lines in `useful_intake`; the UI branch reruns its intake tests.
- New public items: `SOURCE_INTAKE_INSTRUCTION`, `Workspace::post_intake_instruction`, `brn_ai::threads::{reusable_intake_instruction, intake_instruction_to_reuse}`.

---

## F. Make saved chat replies findable in search

### What the code does today

- `search.sqlite` (`crates/brn-retrieval/src/search_index.rs`, `SCHEMA_VERSION = 2`) indexes notes only. `docs.kind` already allows `'message'`, but every insert writes `'note'`. All note lanes, counts, facts, embedding queues and `matching_keywords` read `docs`/`passages` without a kind filter.
- The worker (`crates/brn-threads-app/src/search.rs`) syncs by revision watermark. It answers `Work::Search` through `search()` and re-checks every hit with core `Snapshot::searchable_notes`.
- Owner search:
  - `Workspace::search_page`, which the CLI `search` uses; it prints `encode(page)` plus `shown`, so new fields appear in its JSON without a `main.rs` change.
  - The desktop `threads_search.rs` calls `handle.request_page(query, None, …)`, general only, and renders `page.hits` with Previous/Next driven by `page.cursor`.
- The agent's `RunNoteSearch for SearchHandle` (`search.rs` ~1297) also calls `request_page` and `request_keyword_page`, but maps only `results.matches`.
- Core message visibility (`context.rs` `visible`, the `Message` arm) is `scope_allowed(thread, rows, chat)`, which admits a Message only in its own chat. That predicate is right for agent runs and is not the owner-search scope.

### Decisions

1. **Separate message tables in the derived index, not `docs` rows with `kind = 'message'`.** With separate tables, no note query, count, fact or embedding job needs a new filter. Message text also cannot reach the agent's `search_notes` lanes by construction. `docs.kind` keeps writing only `'note'`.
   - Schema v3 adds:
     - `chats(id TEXT PRIMARY KEY, scope TEXT NOT NULL)`;
     - `messages(rid INTEGER PRIMARY KEY AUTOINCREMENT, id TEXT UNIQUE NOT NULL, chat TEXT NOT NULL, version INTEGER NOT NULL, role TEXT NOT NULL CHECK(role IN ('owner','assistant')), text TEXT NOT NULL)`, with index `messages_chat`;
     - FTS5 `fts_message_exact` (`unicode61 remove_diacritics 2`) and `fts_message_stem` (the registered stem tokenizer), both `content='messages', content_rowid='rid'`;
     - insert/delete triggers for both FTS tables, and a `messages_no_update` trigger (replace-only, like passages).
   - `OBJECTS` gets every new name. The version bump makes an existing v2 index `Found::Rebuild`. It is rebuilt in place and the next sync is full, which backfills every message. No `threads.sqlite3` change.
2. **Owner-search message scope lives in core and mirrors notes.** Add to `crates/brn-threads-core/src/reader.rs`:
   - `pub enum MessageScope { General, Project(String), Chat(String), Hidden }` and `pub fn message_scope(thread: &Record, rows: &[Record]) -> MessageScope`:
     - `Hidden` if the record is not a Thread, or is archived (Archive, and project Delete's Trash, both archive the Thread);
     - `Project(p)` if `project_for_chat` finds `p` (`Hidden` if `p` is archived);
     - `Chat(id)` if the chat has private context: `Thread.private_context`, or a Note with `chat == id`. This is the existing `chat_has_private_context` predicate, refactored into a pure `pub(crate) fn chat_private(rows, chat)` in `context.rs` that both use;
     - otherwise `General`.
   - `pub fn message_scopes_for(rows, context: Option<&str>) -> Vec<MessageScope>`:
     - general context: `[General]`;
     - a context inside project `P` (`project_scope`): `[Project(P)]`, plus `General` only when `P.use_brn_knowledge`, as general notes appear there today;
     - a general chat `c`: `[General, Chat(c)]`.
     So project chats appear only in their project's search, general search shows general chats, and a chat whose attachments are private is searchable only from itself, as its chat-only notes are.
   - `Snapshot::searchable_messages(ids, context) -> BTreeSet<String>`: Messages that are not archived, have role `owner` or `assistant` (never `assistant-partial` or `writing-request`), and whose Thread's `message_scope` (from fresh rows: the Threads, all Projects, and Notes with `chat` in those Threads) is in `message_scopes_for(context)`. This is the authority re-check for every hit.
3. **Indexing in the worker's `sync()`.**
   - A changed `Message` with role `owner`/`assistant` that is not archived is upserted; any other changed Message is removed.
   - For each changed `Thread`, each chat of a changed `Note` with `chat`, and every chat when any `Project` changed, the chat's scope tag is recomputed (`general`, `project:<id>`, `chat:<id>`, `hidden`) with `message_scope`.
   - The full branch indexes every Message and Thread. This uses the existing `changed_since` / `records()` paths; a Project change loads `records()` once.
4. **Owner results are a distinct list, and the note fields stay as they are.**
   - Add to `SearchResults`: `messages: Vec<MessageMatch>`, `message_total`, `message_next`.
   - Add to `SearchPage` (`#[serde(default)]`): `messages: Vec<ChatMessageHit { message, version, chat, chat_title, role, snippet, highlights }>`, `message_total` and `message_cursor`.
   - Messages are ranked by RRF over the exact and stem lanes, with ties broken by text then ID. A page uses the same `cursor` offset for both lists, so the one existing `--cursor` input pages both.
   - `cursor` stays notes-only, so the desktop's Previous/Next is unchanged. `message_cursor` is set when more messages remain, and passing it as `cursor` continues the messages, while notes past their end are empty.
   - Message reach is capped at `REACHABLE` (100). `message_total` is exact: the FTS count over the context's scope tags, minus re-check rejections, as for notes.
   - Hits are dropped and retried when the snapshot's Message version differs, as notes do.
5. **The agent never gets messages.** Add `messages: bool` to `Work::Search`. The public `request_page` and `request_keyword_page` (owner and desktop) set it to `true`. The `RunNoteSearch` impl switches to a private `notes_request(query, chat, limit, cursor, hybrid)` with `messages: false`. Chat messages are **not** added to `search_notes`: an agent-facing chat lane would need its own scope analysis against `scope_allowed`, and the slice asks for owner search only.
6. **Desktop.** The desktop renders `page.hits` only, so message hits are returned but not shown. No desktop change. Record this as a UI finding (F-UI1) with steps:
   1. Fresh store; `brn thread-start --title Replies`; `brn queue`/`ask --thread <id> --prompt "Berth survey reply draft 1.4 m"` (offline: the owner Message is saved).
   2. Open the desktop, search "berth": the CLI `search --query berth` lists the Message under `messages`, but the desktop shows "0 matches".

### Items

- [ ] F1. Core scope functions and tests.
  - Add `MessageScope`, `message_scope`, `message_scopes_for`, `Snapshot::searchable_messages`, and the `chat_private` refactor (`chat_has_private_context` keeps its behavior).
  - New `crates/brn-threads-core/tests/message_search_scope.rs`, with one case each: general chat; project chat; archived chat; project Delete (trashed) and Restore; moved chat; private-attachment chat; knowledge off; partial and writing-request roles excluded.
  Files: `crates/brn-threads-core/src/{reader.rs,context.rs,lib.rs}`, test file.
  Verify: `cargo test -p brn-threads-core --locked --offline --test message_search_scope` passes, and the full core suite passes.
- [ ] F2. Index schema v3 and the message API.
  - `MessageInput`; `Changes.{upsert_messages, remove_messages, chat_scopes}` (full rebuild also clears `messages` and `chats`); `SearchIndex::message_hits(query, scopes, per_lane)` and `SearchIndex::message_count(query, scopes)`.
  - Tests in `crates/brn-retrieval/tests/search_index.rs`: upsert, replace and remove with no stale FTS terms (`INSERT INTO fts_message_stem(fts_message_stem) VALUES('integrity-check')`); scope filtering; exact count; a v2 file is rebuilt (`Opened::Rebuilt`); the existing note tests are unchanged.
  Files: `crates/brn-retrieval/src/search_index.rs`, `crates/brn-retrieval/tests/search_index.rs`.
  Verify: `cargo test -p brn-retrieval --locked --offline` passes. Also run `cargo test -p brn-threads-app --features native-retrieval --locked --offline --test search_dense` if the native features build offline; otherwise record that the native lane was not run.
- [ ] F3. Worker sync and query.
  - Sync per Decision 3; `search()` builds the message list when `messages`; `notes_request` for the agent.
  - `MessageMatch` and the `SearchResults` fields; the `SearchPage`/`ChatMessageHit` conversion in `lib.rs`.
  Files: `crates/brn-threads-app/src/{search.rs,lib.rs}`.
- [ ] F4. Tests. Write them first against stub fields that are always empty; each fails on the expected hit.
  New `crates/brn-threads-app/tests/search_messages.rs`:
  - `a_saved_reply_is_found_with_its_chat`: a scripted run completes with "Draft reply (not sent): the berth survey measured 1.4 m". `search_page("berth survey", None, 12, None)` has `messages[0]` with `role == "assistant"`, `chat` and `chat_title`. The note `hits` are unchanged.
  - `owner_messages_are_found_and_partial_progress_is_not`: Stop with partial text. The partial is never found; the owner Message is.
  - `project_chats_appear_only_in_their_project_search`: covers general, the project (knowledge on and off), another project, and CLI-style `Some(project_id)` and `Some(project_chat)` contexts.
  - `archived_and_trashed_chats_disappear_and_return_on_restore`: covers `archive_chat`/`unarchive_chat`, and project Delete then Restore.
  - `a_private_attachment_chat_is_searchable_only_from_itself`.
  - `rebuilding_the_index_backfills_messages`: delete `derived/search.sqlite`, or set its `user_version` to 2, then open a new workspace view. The Message is found.
  - `agent_search_never_returns_messages`: `RunNoteSearch::search` for a message-only word gives `total == 0` and no results.
  - `message_pages_follow_the_cursor`: 60 matching owner Messages, `limit 50`. Expect `message_cursor == Some("50")` with `cursor` `None` (no notes); the next page has 10 messages.

  New `crates/brn/tests/search_messages.rs`: `thread-start`, an offline `ask` (saves the owner Message), then `search --query …`. The JSON has `messages[0].chat` and `message_total: 1`, and the exit code is unchanged.

  Verify:
  - `cargo test -p brn-threads-app --locked --offline --test search_messages --test search_worker --test chat --test projects` passes;
  - `cargo test -p brn --locked --offline --test search_messages` passes;
  - `cargo test -p brn-threads-app --locked --offline --test user_scale` passes (non-strict bounds; record the p95 it prints);
  - the desktop test command passes, unchanged.
- [ ] F5. Append slice F to `round5/verification.md`: the decisions, F-UI1 with its repro, and the knowledge-retrieval plan hook "Indexing of kind=message" marked as done through separate tables.

### Integration notes

- `search.sqlite` v3: the first open after the merge rebuilds in place.
- New serde-default fields on `SearchPage`.
- The CLI `search` JSON gains `messages`, `message_total` and `message_cursor`. Tell work/cli-agent-contract in case its `--json` description lists output fields.
- The desktop shows no message hits until the UI branch renders `page.messages` (F-UI1).

---

## G. The agent can withdraw its own obsolete proposal

### What the code does today

- Owner Dismiss is `Workspace::review_dismiss` (`crates/brn-threads-app/src/review.rs`): `store.prepared`, then `store.retire` (marks the operation retired and frees `candidate_targets`; refuses an applied operation), then `finish_review_attention`. That last step is a checked owner change, keyed `review-attention-cleanup:{op}:{digest}` with up to 3 Stale retries. It removes every attention item naming the operation, across all Threads.
- Runtime tools are declared with `tool!` and registered by `runtime_catalog!` (`crates/brn-ai/src/threads.rs` ~413–575). `tool_names()` has 9 entries (test at ~3608). The macro rejects any tool outside `research::HELPER_TOOLS` in a helper scope, and the helper's builder (`threads_research.rs` ~400) registers only its own tools.
- `apply()` decides membership as: a Run with `thread == run's chat` whose `operations` contains the operation. `prepare` and `refresh_candidate` push to `Run.operations`.
- The guide bundle is `brn-threads-guidance-11` (`threads_guides.rs`).

### Decisions

1. **One shared dismissal path in the core.** Move the cleanup loop into `impl Store` (new section in `crates/brn-threads-core/src/run_attention.rs`):
   - `clear_review_attention(candidate, key_prefix, reason, authority) -> Result<CleanupOutcome>`, with `CleanupOutcome { Done(Option<Receipt>), Unexpected(ApplyOutcome), Contended }`.
   - `dismiss_candidate(candidate, key_prefix, reason, authority) -> Result<CleanupOutcome>`: `prepared`, then `retire`, then `clear_review_attention`.
   - The app's `review_dismiss` and `review_apply` call these with their current key prefix, reasons and `HostAuthority::owner("owner interaction")`. They map `Unexpected` and `Contended` to their existing `AppError::Invalid` texts, so owner receipts, keys and messages stay byte-identical.
   - Undo therefore follows the existing dismiss exactly. Retirement is recorded on the operation, and the attention cleanup is a checked change with a receipt whenever an attention item named the candidate. A withdrawal of a candidate that no attention named leaves no record receipt, exactly like the owner's Dismiss.
2. **Tool `withdraw_proposal { operation }`.** Strict schema; `WithdrawArgs` uses `deny_unknown_fields`. The description is "Withdraw your own pending proposal in this chat after the owner's decision made it obsolete. It never applies anything and cannot touch owner changes or other chats." Handler, in order:
   1. `live(scope)`; rejected in writing (Rewrite) runs.
   2. Resolve the operation. It must belong to a Run in this chat (the `apply()` membership check). Owner-created operations are never in `Run.operations`, so they fail this check, as do other chats and projects. These scope failures stay the generic `rejected()`.
   3. Applied (a receipt exists): `{"outcome":"Refused","reason":"This proposal was applied; prepare a new change to revise it."}`.
   4. Already retired: `{"outcome":"Withdrawn","operation":…,"replay":true}`.
   5. Otherwise `store.dismiss_candidate(op, "{run}:withdraw", "Agent withdrew its obsolete proposal; preserve other attention requests", &bind(scope, HostAuthority::owner("runtime withdrawal")))`. The fenced authority means a stopped Run's cleanup is refused. Return `{"outcome":"Withdrawn","operation":…}`.
   It never calls `apply`.
3. **Registration.** Add it to `runtime_catalog!` with the activity label "Withdrawing an obsolete proposal…", which is new visible progress text. It is not in `HELPER_TOOLS`, and the macro guard rejects it in a helper scope. The tool count goes from 9 to 10.
4. **Guidance and bundle.**
   - `agent/BRN.md`, after the candidate paragraph (line 39): "When the owner's decision makes one of your own pending proposals in this chat obsolete, withdraw it with `withdraw_proposal`. Never withdraw the owner's changes or another chat's proposals; withdrawing never applies anything."
   - `agent/skills/resolve-conflict/SKILL.md` step 4: "After the owner decides, withdraw your own pending proposal that contradicts the decision."
   - `BUNDLE_VERSION = "brn-threads-guidance-12"`. Renumber at merge if another branch has taken 12.

### Items

- [ ] G1. Core `clear_review_attention` and `dismiss_candidate`; the app's `review.rs` delegates.
  Files: `crates/brn-threads-core/src/run_attention.rs`, `crates/brn-threads-app/src/review.rs`.
  Verify: `cargo test -p brn-threads-app --locked --offline --test product --test chat_decisions --test writing --test killed_run_candidate` passes; owner dismiss and apply tests are unchanged.
- [ ] G2. The tool, its schema arm, catalog entry and handler.
  Files: `crates/brn-ai/src/threads.rs`.
- [ ] G3. Guidance and bundle bump; extend the `threads_guides.rs` test: `BASE` and the resolve-conflict skill mention `withdraw_proposal`, and `preamble(&[])` contains `brn-threads-guidance-12`.
  Files: `agent/BRN.md`, `agent/skills/resolve-conflict/SKILL.md`, `crates/brn-ai/src/threads_guides.rs`.
- [ ] G4. Tests (write first; the scripted test fails before G2 because the tool is unknown).
  New `crates/brn-threads-app/tests/withdraw_proposal.rs`, with scripted runs through the real runtime tools. Review-first delegation, so an ordinary change comes back `NeedsReview` with its Review item.
  - `the_agent_withdraws_its_own_obsolete_proposal_after_the_owner_decides`:
    1. Run 1 prepares and applies "Delivery 15 November", which comes back NeedsReview.
    2. The owner posts "Keep 22 November".
    3. Run 2 calls `withdraw_proposal { operation }`.
    Expect `Withdrawn`. The operation is no longer in `candidates()` and is retired. No attention item names it. One cleanup receipt is recorded with the withdrawal reason. The note is unchanged, and no receipt exists for the operation itself.
  - `withdrawing_another_chats_proposal_is_refused`: the tool result is an error, and the candidate and its attention are unchanged.
  - `withdrawing_an_owner_created_candidate_is_refused` (`store.allocate_operation` plus `prepare_owner`, not applied).
  - `withdrawing_an_applied_proposal_is_refused` (maintenance delegation, applied). Outcome `Refused`; the receipt is unchanged.
  - `withdrawing_twice_replays_without_another_receipt`.
  - `undo_after_a_withdrawal_matches_undo_after_dismiss`: Undo of each cleanup receipt gives the same outcome kind.

  brn-ai:
  - `tool_names().len()` goes to 10 (`threads.rs` ~3608).
  - New test `the_helper_is_never_offered_withdraw_proposal` in `threads_research_tests.rs`: the helper model's `requests()[0].tools` exclude it, `HELPER_TOOLS` lacks it, and a direct call in a helper scope is rejected.
  Verify:
  - `cargo test -p brn-ai --locked --offline` passes;
  - `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` passes;
  - `cargo test -p brn-threads-app --locked --offline --test withdraw_proposal --test agent_journeys` passes.
- [ ] G5. Append slice G to `round5/verification.md`.
  - The registered tool count is now 10. Proposed `threads-behavior.md` §142 sentence: "Every run registers ten tools: … `refresh_candidate`, `withdraw_proposal`."
  - The bundle is -12, and the activity label is new.

### Integration notes

- Tool count 9 → 10 and bundle -12: docs and UI tests that count tools or pin the bundle need updating.
- New core `Store::{clear_review_attention, dismiss_candidate}`; owner behavior is unchanged.

---

## H. Clear error for an unusable data folder

### What the code does today

`Store::open` (`crates/brn-threads-core/src/store.rs` ~33):
- refuses a relative path, then a symlink ancestor (`UnsafeDirectory("symlink directory")`);
- then runs `fs::create_dir_all`, `read_dir`, and the marker `create_new`/write, whose `?` turns I/O failures into `Error::Io`, displayed as "storage: Read-only file system (os error 30)".

The CLI (`main.rs` ~410) and the desktop (`brn-desktop/src/main.rs` ~42, `--headless-check`, and the UI startup) show `e.to_string()` and exit with `ExitCode::FAILURE`.

### Decisions

1. **New core variant.** `Error::DataFolder { folder: String, problem: DataFolderProblem, #[source] cause: std::io::Error }`, with `#[error("BRN cannot use the data folder {folder}: {problem}")]`. The `DataFolderProblem` Display texts contain no OS text:
   - `ReadOnly`: "it is on a read-only disk. Choose a folder you can write to."
   - `PermissionDenied`: "BRN does not have permission to use it. Choose a folder you own."
   - `NotAFolder`: "that path, or part of it, is a file rather than a folder."
   - `SymlinkRefused`: "part of its path is a symbolic link. Choose the folder's real location."
   - `MissingParent`: "a folder above it is missing or unavailable."
   The OS detail stays in `cause`, reachable through `std::error::Error::source()` for logs; neither host logs more today. No CLI or desktop line changes, and exit codes are unchanged.
2. **Classification** (`fn data_folder_error(folder, io::Error) -> Error`):
   - `ReadOnlyFilesystem` → `ReadOnly`;
   - `PermissionDenied` → `PermissionDenied`;
   - `NotADirectory`, and `AlreadyExists` from `create_dir_all` when the path is not a directory → `NotAFolder`;
   - `NotFound` → `MissingParent`;
   - any other kind keeps `Error::Io`.
   It applies to `has_symlink_ancestor`'s own I/O errors, `create_dir_all`, `read_dir`, and the marker create/write in `Store::open`. A symlink ancestor becomes `DataFolder { SymlinkRefused, cause: io::Error::other("symbolic link in path") }`. The relative-path, non-empty-without-marker, foreign-marker and unexpected-entry refusals stay `UnsafeDirectory`, unchanged. SQLite failures on an existing database are unchanged; recorded limitation: an existing workspace on a read-only disk still reports the SQLite text.

### Items

- [ ] H1. Add the variant, the problem enum and the classification, and use them in `Store::open`. Unit tests in `store.rs` `mod tests` cover the classification of each `ErrorKind`, including `NotFound` (no deterministic filesystem repro exists for a missing parent, since `create_dir_all` creates parents).
  Files: `crates/brn-threads-core/src/{lib.rs,store.rs}`.
- [ ] H2. Tests (write first; each fails on the raw "storage: … (os error N)" text).
  - New `crates/brn-threads-core/tests/data_folder.rs`. Each test asserts the variant and problem, that `to_string()` contains the folder path, that it does not contain "os error", and that `source()` is `Some`:
    - `a_read_only_disk_is_named` (`#[cfg(target_os = "macos")]`, `/brn-dwr-readonly-<uuid>` on the sealed system volume; skip if `/` is writable);
    - `a_folder_without_permission_is_named` (parent mode 0o500; skip when privileged, as `backup.rs`'s flush test does);
    - `a_file_in_place_of_the_folder_is_named` (the path is a file, and the path is under a file);
    - `a_symlinked_path_is_refused_by_name` (alias folder; also `/tmp/x` on macOS).
    - Update any existing core or app test that matched `UnsafeDirectory` for a **symlinked data folder** (find them with the H1 test run) to match `DataFolder { problem: SymlinkRefused, .. }`. Tests about entries inside the folder (`chat_storage.rs` ~227–240, `core.rs` ~110–123) stay `UnsafeDirectory`.
  - New `crates/brn/tests/data_folder.rs`:
    - `brn --data-dir <file> notes` exits 1, as before, and stderr starts with "BRN cannot use the data folder" and has no "os error";
    - on macOS, `--data-dir /brn-dwr-readonly-<uuid>` names the read-only disk (the evidence `/data` case).
  Verify:
  - `cargo test -p brn-threads-core --locked --offline` passes;
  - `cargo build -p brn -p brn-desktop --locked --offline && cargo test -p brn --locked --offline --test data_folder` passes;
  - manual: `/tmp/brn-dwr-target/debug/brn-desktop --headless-check startup --data-dir <a file under /private/tmp/brn-dwr-data/r5-h>` prints the same owner text and exits 1. Record the output.
  - `cargo test -p brn-threads-app --locked --offline` passes.
- [ ] H3. Append slice H to `round5/verification.md`, with the limitation above.

### Integration notes

New `brn_threads_core::Error::DataFolder` variant. work/cli-agent-contract's exhaustive `From<Error>` match in its `main.rs` must map it at merge, as `EditSessionOpen` needed (suggested code `data_folder_unusable` with details `{folder, problem}`). The desktop shows it through its existing `to_string()` display, with no wording edit.

---

## After the slices: round close

- [ ] R1. Full verification from the worktree root, with results recorded in `round5/verification.md`:
  - `cargo fmt --all --check`;
  - `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline`;
  - workspace Clippy and `brn-ai` scripted-runs Clippy;
  - `cargo test --workspace --locked --offline --no-fail-fast`: counts against the baseline, with only the known sandbox tests allowed to fail;
  - the desktop tests and Clippy;
  - `TMPDIR=/private/tmp/brn-dwr-data/r5-e2e scripts/verify-end-to-end.sh --fixtures-only`;
  - `cargo build -p brn --locked --offline` and `python3 docs/work/active/daily-workflow-reliability/cli-journey.py /tmp/brn-dwr-target/debug/brn /private/tmp/brn-dwr-data/r5-cli-journey`;
  - `git diff --check`; `python3 scripts/check-markdown-links.py`;
  - `git diff --exit-code 771116f0 -- Cargo.lock`; `git diff --stat f519534e -- crates/brn/src/main.rs` (exactly 1 line).
  Delete the generated `/private/tmp/brn-dwr-data/r5-*` folders afterwards.
- [ ] R2. `round5/verification.md` closing sections:
  - evidence classes (automated, headless desktop, native: none unless observed);
  - limitations: A's markerless old stages; F-UI1; H's SQLite case; E's CLI wording;
  - proposed contract sentences for `threads-behavior.md`/`invariants.md` (C, D, F, G tool count);
  - the integration list from each slice.

  Then update the `evidence.md` Status table with rows for slices A–H, each "implemented / verified / native-observed: no / owner-accepted: no / merged: no". Do not edit `docs/status.md` or `docs/architecture/*`.
