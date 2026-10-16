# Round 5 verification

Plan: [plan.md](plan.md). Branch `work/daily-workflow-reliability`, base for this round `f519534e`. Each slice appends its own section. Evidence class for every slice so far: automated tests only (native-observed: no, owner-accepted: no, merged: no).

Environment for every command: worktree root, `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`, `--locked --offline`.

## Baseline (before slice A, HEAD `f519534e`)

| Command | Result |
| --- | --- |
| `cargo test --workspace --locked --offline --no-fail-fast` | 732 passed, 1 failed, 5 ignored |
| `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | 152 passed, 0 failed, 1 ignored |

The one failure is `brn-threads-intake --test qualification` `prepared_rust_fixtures_keep_source_and_preview_bindings_in_owned_scratch` (`Pdf preparation failed: Unavailable`). It is one of the 12 known sandbox-class tests from round 4, and the one that still fails with the sandbox off because Poppler is not installed on this Mac ([evidence.md](../evidence.md)). Round 4's 721 passed / 12 failed is the same 733 tests; with the sandbox off, 11 of those 12 now pass.

## Slice A: clean up abandoned backup and restore stage folders

### Change

- New crate-private `brn-threads-core/src/stage.rs` owns both stage kinds.
  - `StageDir::create(parent, kind)` makes `<parent>/.brn-backup-stage-<uuid>` or `.brn-restore-stage-<uuid>` (0700), then `create_new`s the marker `.brn-stage` (0600), locks it with std `File::lock`, writes `BRN backup stage v1\n` or `BRN restore stage v1\n`, and syncs it. The lock is held for the stage's lifetime; the OS frees it on SIGKILL, as with run leases. A failure after `mkdir` removes the folder.
  - Dropping a `StageDir` removes the folder while still locked, then unlocks. So every error path of backup, restore and the upgrade pre-snapshot removes its own stage, as before.
  - `sweep_abandoned(parent, kind, keep)` is best effort and Unix-only (returns 0 elsewhere). For each direct child of `parent` it removes the child only if all checks pass:
    1. the name is exactly the kind's prefix plus a lowercase hyphenated UUID (`.brn-backup-stage-not-owned`, uppercase or simple UUID forms fail);
    2. it is not `keep`, the requested destination;
    3. `DirEntry::file_type` and `symlink_metadata` say real directory, not symlink;
    4. `symlink_metadata(<child>/.brn-stage)` is a regular file;
    5. the marker opened read/write (no create, no truncate) has the same dev/ino as that `lstat`;
    6. `try_lock` succeeds (`WouldBlock` means a live stage: skip);
    7. while locked, the marker text is exactly the kind's text (empty or other text: skip);
    8. the child is still a real directory with the same dev/ino;
    9. `remove_dir_all(child)` runs while the lock is held (std does not follow symlinks inside).
    Any error skips the entry and never fails the backup or restore.
- Backup (`backup.rs`): `StagedBackup` holds a `StageDir`; its own `Drop` is gone. `create` sweeps backup stages in the destination's parent after the symlink-ancestor and occupied-destination checks, before staging. The upgrade pre-snapshot uses the same path.
- Restore (`restore.rs`): the stage is a wrapper and the workspace is staged at `<stage>/workspace` (0700). `publish` renames `<stage>/workspace` to the destination (same parent, still atomic), so the marker is never published and the restored folder is unchanged (still 0700). `prepare` sweeps restore stages in the parent after `create_dir_all(parent)` and the symlink re-check. Its own `Drop` is gone.
- `crates/brn-threads-core/README.md` describes the marker, the sweep and the kept markerless stages.

No API change outside the core crate, no dependency change, no `threads.sqlite3` schema change, no desktop, CLI or `main.rs` change.

### Tests and fail-before

All new tests are in the core crate's unit tests. The fail-before for each was taken after the fix, by temporarily disabling one named part of it, running `cargo test -p brn-threads-core --locked --offline --lib -- stage:: backup::tests:: restore::tests::`, and restoring the source. The files were then compared byte-for-byte with saved copies (`cmp`, all identical) and searched for the `FAIL-BEFORE` edit marker (none).

| Experiment (temporary edit) | Tests that failed, with the failing assertion |
| --- | --- |
| E1: `sweep_abandoned` returns 0 (the old code's behavior: nothing is cleaned) | `a_killed_backups_stage_is_removed_by_the_next_backup_to_that_folder` ("leftover stage must be removed"); `a_sigkilled_backup_process_leaves_a_stage_the_next_backup_removes` (one `.brn-backup-stage-<uuid>` left vs `[]`); `a_killed_restores_stage_is_removed_by_the_next_restore_into_that_parent` ("the full leftover copy must be removed"); `stage::a_sweep_removes_only_abandoned_stages_of_its_kind_with_exact_names` (removed 0 vs 1) |
| E2: step 6 skipped (`WouldBlock` treated as free) | `a_live_stage_survives_another_backup` and `a_live_restore_stage_survives` ("a held stage is never removed"); `stage::a_sweep_removes_only…` (removed 2 vs 1) |
| E3: steps 3 and 8 (symlink/real-directory checks) dropped | `stage_named_entries_that_are_not_brn_stages_are_never_followed_or_removed` (folder tree changed: the stage-named symlink was removed) |
| E4: step 7 (marker text) dropped | `stage_named_entries_that_are_not_brn_stages_are_never_followed_or_removed` (folder tree changed: the folder with the restore marker was removed) |
| E5: backup sweeps `parent.parent()` instead of the destination's parent | `a_backup_sweeps_only_its_own_parent` ("…/area/.brn-backup-stage-… is outside the backup's folder"); also the E1 backup tests |
| E6: `StageDir` drop does not remove the folder (no error-path cleanup) | error path: `invalid_snapshot_and_abandoned_stage_clean_up_without_final_file` (new no-stage assertion after the failed `create`), `abandoned_stage_and_sqlite_copy_failure_clean_up_without_publication` (`!staging.exists()`); `stage::a_stage_is_private_marked_locked_and_removed_on_drop`; `staging_never_exposes_ready_destination` (wrapper left). 19 tests failed in all, including 9 existing backup/restore cleanup tests |
| E7: `abandon()` keeps the lock (test helper) | `stage::an_abandoned_stage_keeps_its_folder_and_frees_its_lock` (`try_lock` → `WouldBlock`); `stage::a_sweep_removes_only…` |
| E8: step 1 accepts any UUID spelling and step 2 (`keep`) dropped | `stage::a_sweep_removes_only…` (removed 4 vs 1: simple, uppercase and `keep` entries) |

What the new tests cover:

- **Leftovers are cleaned by the next run.** Backup: an empty abandoned stage and one holding a complete snapshot (`StagedBackup` killed before publication) are removed by the next `store.backup` into that folder; `.brn-backup-stage-not-owned` survives; the published backup validates. Restore: an abandoned stage holding a full `workspace/` copy and a wrapper-only stage (killed after rename) are removed by the next `Store::restore` into that parent; an earlier published restore target (whole tree), the backup file and `.brn-restore-stage-not-owned` are unchanged.
- **A real SIGKILL.** `a_sigkilled_backup_process_leaves_a_stage_the_next_backup_removes` spawns the test binary (`--exact backup::tests::stage_holder_child --ignored`), which stages a backup, prints `BRN_STAGE_READY` and sleeps. The parent SIGKILLs it, checks one stage remains with a free lock, then the next backup removes it and leaves the child's workspace folder alone. `stage_holder_child` is `#[ignore]` and returns at once without `BRN_STAGE_CHILD_PARENT`, which is why the ignored count rises by one.
- **A live stage survives** (backup and restore): a held `StageDir` with content is unchanged after another run in the same parent.
- **Stage-named entries that are not BRN's stages are never followed or removed:** a stage-named symlink to an outside folder holding a valid free marker, a stage-named folder without a marker, one with the restore text, one whose marker is a symlink, a stage-named regular file, and `.brn-backup-stage-not-owned` with a valid marker. The folder and the outside folder are unchanged byte for byte.
- **Only the destination's parent is swept:** abandoned stages in the grandparent, a sibling folder and a subfolder all survive.
- **Error path:** a failed backup `create` (snapshot validation failure after staging) and a failed restore `copy_from` leave no stage entry.
- `staging_never_exposes_ready_destination` now also checks the wrapper sits in the destination's parent, both folders are 0700, the published folder has no `.brn-stage` and is 0700, and the wrapper is gone. Every restore test that checked `!staging.exists()` now also checks the wrapper is gone.

### Commands and results (after the fix)

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo test -p brn-threads-core --locked --offline` | 129 passed, 0 failed, 1 ignored (lib 35 passed, was 25) |
| `cargo test -p brn-threads-app --locked --offline --test product --test backup_directory` | 4 + 19 passed |
| `cargo test -p brn --locked --offline` | 13 passed (includes `tests/threads.rs` no-stage-after-CLI-backup/restore checks) |
| `cargo test --workspace --locked --offline --no-fail-fast` | 742 passed, 1 failed (the baseline Poppler test), 6 ignored |
| desktop test command | 152 passed, 0 failed, 1 ignored |
| `git diff --exit-code Cargo.lock`; `git diff --check` | clean |

### Limitations

- Stages without a marker are never removed automatically. That includes every stage left by earlier builds (such as the 7 in [evidence.md](../evidence.md)) and a stage killed between `mkdir` and its marker write (an empty folder). The owner may delete them by hand.
- The sweep runs only on Unix. On Windows, stages still clean up on every in-process error path, but a killed process's stage stays.
- A stage is removed only by a later backup (or restore) into the same folder. Nothing sweeps on startup.
- The sweep's own I/O errors are silent: the entry is kept and the backup or restore proceeds.

### Integration items

- On-disk format: a restore stage now holds `workspace/` plus `.brn-stage`; a backup stage holds `snapshot.sqlite3` plus `.brn-stage`. Final backups and restored folders are unchanged.
- No API, CLI, desktop or UI-branch impact. Proposed sentence for `docs/architecture/invariants.md` (not edited here): "A backup or restore removes only abandoned BRN stages in its destination's parent: exact BRN stage name, real directory, BRN marker text, and no process holding the marker's lock."

## Slice B: count one file as one source regardless of path spelling

### Change

- New `brn-threads-core/src/locator.rs`, exported at the crate root (and so through `brn_threads_app`):
  - `canonical_locator(locator)`: on Unix, an absolute path (`/…`) whose `fs::canonicalize` succeeds, names a regular file and is UTF-8 becomes its real path. Every other locator is returned unchanged: URLs, `synthetic://` and other schemes, relative, missing, non-UTF-8 and directory paths.
  - `same_locator(stored, canonical)`: exact match, or both absolute and `canonical_locator(stored) == canonical`. Non-path locators never touch the filesystem.
- Runtime (`brn-ai/src/threads.rs`):
  - `prepare_run` rewrites each `request.sources[i].reference` to its canonical locator before `begin`. The prompt, `read_source`, the Scope's source map and the recorded Source then use one spelling. `run()` goes through `prepare_run`, so the CLI and desktop need no change.
  - `begin()` looks up the Source by exact canonical locator in the chat scope first, then by `same_locator`. When the match is a Source saved under an older spelling and no report is supplied, the start change now also writes that Source: a clone with only `locator` (canonical) and `observed_at` updated, at its current version and archived flag. With a report the existing Source Put already writes the canonical locator. This is a new revision in the existing checked start change, so history keeps the old spelling. It cannot collide on the `(chat, locator)` unique index: an exact canonical match in the same scope is always chosen first.
- App (`brn-threads-app/src/intake.rs`):
  - `source_intake_chat` canonicalizes its `locator` and counts every Source that matches through `same_locator`, so old duplicates (one `/tmp`, one `/private/tmp`) all lead to the same chat.
  - `import_full_scoped` uses the canonical locator for the Source it records and matches exact first, then `same_locator`. The Source Put it always writes records the canonical locator. Its operation binding still hashes `converted` as given, so retry identity is unchanged.
  - `keep_in_brn`'s global-Source lookups (note source and figure source) match exact first, then an older spelling. A global Source it creates copies the chat Source at its real path (review round, B-R5).
- Label: a new Source's label is still its locator in `begin()` (now the real path) and the title in full import. An existing Source keeps its label unless it receives a report, as before.

No dependency, `Cargo.lock`, `threads.sqlite3` schema, desktop production, CLI, `main.rs`, `HELP` or exit-code change.

#### Review round ([review-B](review-B.md))

All six findings are fixed; none changes what an owner sees beyond the slice spec.

- B-R1 (blocking): new tests cover the older-spelling path in full import, both Keep in BRN lookups and `begin()`'s report-bearing branch (tables below).
- B-R2: a pure respell no longer flags derived records. The core's apply (`store.rs`) skips a written Source whose only meaningful change is its locator when both spellings canonicalize to the same path (`respelled_source`). It reads the filesystem only in that case. `meaningful_input_change` and `derivation_is_current` are unchanged, so the derivation walk stays filesystem-free. A locator change to another file still refreshes.
- B-R3: the older-spelling fallback only resolves host-recorded Sources: those a Run's `sources`, a Note's `import.source` or an Asset's `source` names (`locator::older_spelling_candidates`). Exact matches need no host record. New core helpers `find_source(records, canonical, scope)` (exact first, then a host-recorded older spelling) and `matching_sources(records, canonical)` now carry this rule for `begin()`, `import_full_scoped`, `source_intake_chat` and both Keep in BRN lookups. `source_intake_chat` only uses Sources a Run names, so its result is unchanged.
- B-R4: `retained_useful_report` first matches issue prefixes exactly, as before. If none matches, it canonicalizes each `" · "`-delimited prefix and takes the longest one equal to a current Source locator. An issue an older build scoped under `/tmp/a` therefore stays scoped to Source A after A is respelled to `/private/tmp/a`.
- B-R5: both Keep in BRN global lookups now use `find_source`, so the exact real path wins over an older spelling. A global Source that Keep in BRN creates records the real path. The staged lookup compares exactly, since every staged global Source records the real path.
- B-R6: the three side effects are listed under Limitations.

### Tests and fail-before

Integration tests were written first and run against the unfixed code (HEAD `e07354db`):

| Test | Result before the fix |
| --- | --- |
| app `the_same_file_through_a_symlinked_folder_reuses_one_chat_and_one_source` | FAILED: `source_intake_chat` returned `(<new chat>, false)` vs `(<first chat>, true)` |
| app `tmp_and_private_tmp_spellings_reuse_one_chat_and_one_source` (macOS) | FAILED: same, new chat for `/private/tmp/…` after `/tmp/…` |
| app `a_source_saved_under_an_older_spelling_still_matches_and_is_recorded_canonically` | FAILED: `(<new chat>, false)` vs `(<seeded chat>, true)` |
| app `full_import_of_one_file_through_two_spellings_observes_one_source` | FAILED: reason `Full-note import: Crane report (Unchecked)` vs `Observed previously imported full note: Crane report` |
| app `a_different_file_with_the_same_name_gets_its_own_chat_and_source` (guard) | passed, as expected |
| CLI `useful_intake_of_one_markdown_file_through_two_spellings_reuses_its_chat` | FAILED: `reused_chat` `(false, false)` vs `(false, true)` |
| desktop `supplying_one_file_through_two_spellings_reuses_its_chat` | FAILED: second chat id differs from the first |

After the fix, temporary edits (each restored, `git diff` then showed only intended changes):

| Experiment (temporary edit) | Tests that failed |
| --- | --- |
| `canonical_locator` never canonicalizes (`if false && …`) | core `tmp_maps_to_private_tmp` (`/tmp/…` vs `/private/tmp/…`), `a_file_through_a_symlinked_folder_maps_to_its_real_path` (`alias/mail.txt` vs `real/mail.txt`), `an_older_spelling_matches_and_another_file_with_the_same_name_does_not`; app symlink, `/tmp`, older-spelling and full-import tests as before the fix |
| `real.is_file()` check dropped (folders canonicalized) | core `a_file_through_a_symlinked_folder_maps_to_its_real_path`: the symlinked folder became `…/real` vs `…/alias` |
| `same_locator` compares file names only | core `an_older_spelling_matches_and_another_file_with_the_same_name_does_not`; app guard `a_different_file_with_the_same_name_gets_its_own_chat_and_source` |
| `begin()` respelling Put disabled (`&& false`) | app `a_source_saved_under_an_older_spelling_still_matches_and_is_recorded_canonically`: locator stayed `…/alias/mail.txt` vs `…/real/mail.txt` |

The older-spelling test seeds a Source with the alias locator and a Failed Run naming it through a checked owner change (`prepare_owner` + `apply`). Supplying the real path reuses that chat and that Source ID; the Source's latest revision has the real path, revision 1 keeps the alias, and both Runs name the same Source.

Review-round tests. Each seeds what an older build left through checked owner changes (`prepare_owner` + `apply`, or `owner_change`): a global Source under the alias spelling, its protected full note and its full-import link. They were run first against the unfixed review baseline (HEAD `02bd457b`):

| Test (finding) | Result at `02bd457b` |
| --- | --- |
| core `another_spelling_of_the_same_file_does_not_refresh_but_another_file_does` (B-R2) | FAILED: `needs_refresh` `["…:0"]` vs `[]` for the real path of the same file |
| app `full_import_reuses_a_source_an_older_build_saved_under_another_spelling` (B-R1, B-R2) | FAILED: `needs_refresh` `["…:0"]` (the derived note), not empty |
| app `keep_in_brn_prefers_the_global_source_at_the_real_path` (B-R5) | FAILED: both lookups chose `…:0` (alias) vs `…:3` (real path) |
| brn-ai `requalifying_a_source_respelled_since_an_older_build_drops_its_retained_omission` (B-R4) | FAILED: omission kept as `synthetic://qualified-b · …/alias/a.txt · chart` |
| app `keep_in_brn_reuses_a_global_source_saved_under_an_older_spelling` (B-R1) | passed (fallback present); fail-before below |
| app `a_supply_with_a_coverage_report_reuses_a_source_saved_under_an_older_spelling` (B-R1) | passed (fallback present); fail-before below |
| core `only_a_host_recorded_source_matches_under_an_older_spelling` (B-R3) | new API; fail-before below |

After the fix, temporary edits (each restored; `grep MUTATION` empty and `git diff` showed only intended changes):

| Experiment (temporary edit) | Tests that failed |
| --- | --- |
| `import_full_scoped` exact lookup only | `full_import_reuses_…`: reason `Full-note import: Crane report (Unchecked)` vs `Observed previously imported full note: Crane report` |
| `import_full_scoped` keeps the stored locator (`source.locator = locator` removed) | `full_import_reuses_…`: locator `…/alias/report.md` vs `…/real/report.md` |
| Keep in BRN note-source lookup exact only | `keep_in_brn_reuses_…`: `[global, <new global>]` vs `[global, global]` |
| Keep in BRN figure-source lookup exact only | `keep_in_brn_reuses_…`: `[<new global>, <new global>]` vs `[global, global]` |
| `begin()` exact lookup only | `a_supply_with_a_coverage_report_…` and `a_source_saved_under_an_older_spelling_…`: 2 Sources vs 1 |
| `find_source` ignores the host-recorded filter | `only_a_host_recorded_source_matches_…`: a Source only a model named matched |
| `respelled_source` always false | core `another_spelling_…` and app `full_import_reuses_…`: `needs_refresh` not empty |
| canonical issue-prefix fallback disabled (`.take(0)`) | brn-ai `requalifying_…`: the omission is kept under B again |

### Commands and results (after the fix)

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | clean |
| `cargo test -p brn-threads-core --locked --offline` | lib 39 passed (was 35), 1 ignored; all others pass |
| `cargo test -p brn-threads-app --locked --offline --test source_intake_chat --test host_intake --test import_coverage --test chat` | 7 + 15 + 2 + 9 passed |
| `cargo test -p brn-ai --locked --offline` | 340 passed, 1 ignored |
| `cargo build -p brn --locked --offline && cargo test -p brn --locked --offline` | `source_locator` 1 passed, `threads` 13 passed |
| `cargo test --workspace --locked --offline --no-fail-fast` | 752 passed, 1 failed (the baseline Poppler test), 6 ignored |
| desktop test command | 153 passed, 0 failed, 1 ignored |
| `git diff --exit-code Cargo.lock` | clean |

After the review round (same environment):

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | clean |
| `cargo test --workspace --locked --offline --no-fail-fast` | 759 passed, 1 failed (the baseline Poppler test), 6 ignored. Core lib 40 (+1), core `core` 45 (+1), app `source_intake_chat` 11 (+4), `host_intake` 15, `import_coverage` 2, `chat` 9, brn-ai lib 341 (+1), CLI `source_locator` 1, `threads` 13 |
| desktop test command | 153 passed, 0 failed, 1 ignored |
| `git diff --exit-code Cargo.lock` | clean |

Manual CLI check after the review round on fresh synthetic data, `/private/tmp/brn-dwr-data/r5-b-review/` (`HOME`, `CODEX_HOME` and `TMPDIR` inside the case; `alias` is a symlink to `src`):
- `intake --intent full` of `/tmp/brn-dwr-data/r5-b-review/src/report.md` gave `Full-note import: Crane report (Partial)`.
- The same file through `…/alias/report.md` gave `Observed previously imported full note: Crane report`, with `needs_refresh` `[]`.
- `intake --intent chat` through the alias, then `keep-in-brn` of that attachment, applied. That left one global Source (v2) and one chat Source, both at `/private/tmp/brn-dwr-data/r5-b-review/src/report.md`. Both global full notes name the one global Source.

Manual CLI check on fresh synthetic data, `/private/tmp/brn-dwr-data/r5-b-locator/` (offline: missing auth file, every run `Failed:ReconnectNeeded`): `intake --intent useful` of one Markdown file as `/tmp/brn-dwr-data/r5-b-locator/src/mail.md`, then `/private/tmp/…/src/mail.md`, then through the symlinked `…/alias/mail.md` gave one chat (`reused_chat` false, true, true) and one Source at `/private/tmp/brn-dwr-data/r5-b-locator/src/mail.md`. A different `other/mail.md` got its own chat (`reused_chat` false) and its own Source.

### Limitations

- Canonicalization is Unix-only; elsewhere locators stay as given.
- A file is matched by path, not content: a Source whose file was deleted or moved keeps its stored spelling and matches only exactly. A hard link with another path is a different source.
- A Source saved under an older spelling is rewritten to the real path only when a later run or full import of that file records it. Keep in BRN reuses an older-spelling global Source but does not respell it. Old duplicate Sources (one per spelling) are not merged; both still lead to the same chat.
- When no exact match exists, the older-spelling fallback runs `realpath` once for each host-recorded absolute-path Source in scope: `source_intake_chat` over all scopes, `begin()` and full import within their scope, Keep in BRN over global Sources. Paths that only a model named are never resolved. A host-recorded path on a stalled network volume or a removed drive can still block that call inside `realpath`.
- A Source only a model named, under an older spelling of a file, is not matched. The host's Source for that file is a separate record, as before this slice.
- Excluding a pure respell from `needs_refresh` reads the filesystem in the core's apply, once per spelling, only for a written Source whose sole meaningful change is its locator.
- The canonical issue-prefix fallback for retained coverage needs the file to still exist. If it is gone, an issue an older build scoped under another spelling falls back to the note's primary Source, as before.
- The provider prompt and `read_source` carry the resolved path. That can name another location than the one the owner typed, for example the target of a symlinked folder.
- A stored symlink path matches whatever file it points to now, not the file it pointed to when recorded.
- Two spellings of one file in a single request collapse to one source key, and `begin()` rejects the run with its generic rejection. Both hosts build one-source requests today (`main.rs`, `threads_ui.rs`).
- New file Sources show the real path, so a `/tmp/...` supply now shows `/private/tmp/...` in Source rows.

### Integration items

- New core exports `canonical_locator`, `same_locator`, `find_source` and `matching_sources`. Slice E should use `find_source` (or `matching_sources`) for Source matching, so it gets exact-first and the host-recorded filter. `brn_ai::threads::prepare_run`'s signature is unchanged; it now canonicalizes the request's source references.
- Tests elsewhere that compare a real temp file path with `Source.locator` must use the canonical path. Synthetic locators are unchanged.
- Proposed sentence for `docs/architecture/threads-behavior.md` (not edited here): "A local file supplied as a source is recorded and matched by its real path, so one file is one Source however its path is spelled; a Source saved under an older spelling still matches and is recorded at the real path by the next checked observation."

## Slice C: the remaining stop paths end lease-free Runs like orphans

### Change

The core cannot see leases (`run_lease` lives in `brn-ai` and uses `libc`), so the service passes liveness to it as data and removes free lease files after the change.

- Core (`brn-threads-core`):
  - `run_attention::ended_run_attention_with(writes, ended, notice: Fn(&str) -> bool, record, active)` picks the notice per Run. `ended_run_attention(.., notice: bool, ..)` is now a one-line wrapper, so every round 4 caller is unchanged. A chat still keeps one terminal notice.
  - `Store::chat_command_with_orphans(op, command, owner, orphans)`. The Pause and Archive/ArchiveProgress arms give the Interrupted notice to a stopped Run whose ID is in `orphans`. `chat_command` delegates with an empty set.
  - `Store::stop_and_post_checked(.., orphans)`, the same for steering. `stop_and_post` and `stop_and_post_with_partial` delegate with an empty set.
  - `orphans` is not part of either operation binding. A replay returns the recorded receipt.
- App (`brn-threads-app`):
  - `lease_free_working_runs()`: non-archived Working Runs with `!run_is_live`. It is empty when `derived_directory()` fails, so unknown liveness counts as live, as in `recover_orphaned_runs`. `chat_command` computes it only for `Pause { run: Some }`, `Archive` and `ArchiveProgress`, the commands that end Runs. `stop_and_post_with_partial` always computes it.
  - `remove_ended_leases(receipt)`: for each write whose `before` is a Working Run and whose `after` Run is not Working, it calls `run_lease::remove_free`. That function removes a file only while it holds the lock, so a live lease is never touched.
  - Wired into `chat_command` (Stop, the failure stop, the resume-stale stop, Archive, every queue command), `stop_and_post_with_partial`, the four settings wrappers that stop Runs (`configure`, `choose_delegation`, `choose_backup_directory`, `pause`) on `Applied`, `project_command`, and all three applied returns of `import_full_scoped`. `recover_orphaned_runs` keeps its own call.
- Runtime (`brn-ai/src/threads.rs`, `begin()`): each Run it fences is an orphan when `!run_is_live(&derived, id)`, using the `derived` folder it already opens for the new lease. Orphans get the notice in the "Start a fresh invocation" change, and `remove_free` runs for them after that change is `Applied`.

No dependency, `Cargo.lock`, `threads.sqlite3` schema, desktop production, CLI, `main.rs`, `HELP` or exit-code change. Settings still write `Cancelled` (slice D changes that).

### Path table after slice C

Round 4's table, with this slice's changes in bold. "Lease-free" means no process holds the Run's lease: a leftover unlocked file or no file at all.

| Path | Ends Run as | Notice | Review | Lease file | Test |
|---|---|---|---|---|---|
| Settings: Configure, delegation Choose, Pause/Resume maintenance, BackupDirectory | `Cancelled`, every Working Run | added (round 4) | added | **free file removed; a live one kept** | T1, T1-live |
| Orphan recovery (Resume, Dispatch) | `Interrupted`, lease-free only | added | added | removed (round 4) | T2, R1 |
| Stop (`stop_chat_run`): desktop Stop, `pause_failed_worker`, resume-stale stop | `Interrupted` | **lease-free: added**; live: none | added | **lease-free: removed**; live: kept | C-T1, C-T2, C-T3, T4, T4-live, C-D1 |
| Archive / ArchiveProgress | `Interrupted`, every Working Run in the chat | **lease-free: added, into the archived Thread write**; live: none | added | **lease-free: removed**; live: kept | T4, T4-live |
| Steer (`stop_and_post(_with_partial)`) | `Interrupted` | **lease-free: added**; live: none | added | **lease-free: removed**; live: kept | T4, T4-live |
| Project commands, Delete then Restore | `Interrupted` | added (round 4) | added | **removed** | T5 |
| Full-note import into a project | `Interrupted` | added (round 4) | added | **removed** | T5 |
| Continue / new Run in the same chat (`begin()`) | `Interrupted` | **lease-free: added, then replaced by the new Run's result**; live: none | added | **lease-free: removed**; live: kept | T6, T6-live |
| In-process terminal in `finish()` | `Interrupted` / `Failed` | existing | added | the worker's `RunLease::drop` | T7 (round 4) |
| Legacy `resolve_thread(_because)` | `Cancelled`, clears all attention | unchanged | unchanged | unchanged | none |

### Contract correction (T4, T6)

Round 4 chose no notice for Stop, Archive, Steer and Continue because the owner acted in that chat. That holds for a live Run. A lease-free Run's process had already died, so the owner's action did not stop it, and the chat had no trace of the death. These paths now treat it as orphan recovery does. T4 and T6 expected `[Review(op)]` for the killed Run. They now expect `[Review(op), Blocker(run, INTERRUPTED_ATTENTION)]` and the lease file gone. New live counterparts assert no notice and a kept, live lease.

Two consequences an owner can see:
- Archiving a chat whose Run had died puts the notice into the archived chat. Continue there is refused until Unarchive, as it already is for a trashed project chat, which gets the notice since round 4.
- Continue in a killed chat shows the Interrupted notice while the new Run works. The new Run's own result then replaces it, so the chat keeps one notice.

### Tests and fail-before

New and updated tests are in `crates/brn-threads-app/tests/killed_run_candidate.rs`, unless noted. Each one was run against the unfixed source: the nine source files were stashed with `git stash push -- <files>`, the tests kept, then the stash popped. All nine files were then compared with saved copies (`cmp`, identical).

| ID | Test | Result before the fix |
|---|---|---|
| C-T1 | `stop_after_a_failure_treats_a_lease_free_run_as_interrupted_work` (the `pause_failed_worker` call, with the leftover file and with no file) | FAILED `:371` leftover case: `left: [Review(op)]`, `right: [Review(op), Blocker(run) INTERRUPTED]` |
| C-T2 | `resume_after_a_killed_queued_run_leaves_one_notice_and_redispatches_once` (`queue_message`, `dispatch_queued`, `prepare_run` with `request_message` and `queue_claim`, kill, then `stop_chat_run("resume-stale:…")`, `resume_queue`, `dispatch_queued`) | FAILED `:441` `left: [Review(op)]`, `right: [Review(op), Blocker(run)]` |
| T4 | `stop_archive_and_steer_of_a_killed_run_add_the_notice_and_remove_the_lease` (renamed from `…keep_the_candidate_in_review_without_a_notice`) | FAILED `:601` "stop": `left: [Review(op)]`, `right: [Review(op), Blocker(run)]` |
| T1 | `the_delegation_choice_after_a_kill_leaves_one_notice_and_a_review_for_the_candidate` (now expects the lease file removed) | FAILED `:319` `assertion failed: !lease_file(&workspace, &id).exists()` |
| T5 | `project_changes_and_project_imports_leave_the_notice_and_the_review` (now expects the lease file removed) | FAILED `:731` "configure": lease file still present |
| T6 | `continue_in_the_killed_chat_adds_the_notice_and_removes_the_lease` (renamed from `…keeps_the_candidate_in_review`; the new Run now fails, scripted) | FAILED `:746` `left: [Review(op)]`, `right: [Review(op), Blocker(old run)]` |
| C-D1 | desktop `resume_after_a_killed_queued_intake_shows_one_notice` (`crates/brn-desktop/src/threads_intake_tests.rs`) | FAILED `:623` `assertion failed: !lease_file(ui, &run).exists()`. With that line commented out for the run, it FAILED `:649`: the resume-stale receipt's chat write `left: ["Review prepared changes"]`, `right: ["Review prepared changes", "Run interrupted. Continue starts fresh."]` |
| Guards | C-T3 `a_deliberate_stop_of_a_live_run_adds_no_notice_and_keeps_its_lease`, T4-live `stop_archive_and_steer_of_a_live_run_add_no_notice`, T6-live `continue_beside_a_live_run_adds_no_notice_and_keeps_its_lease`, T1-live `the_settings_change_also_stops_a_run_that_is_still_live` (now also asserts the live lease file is kept, then removed by its holder's drop) | passed, as expected; see G1 and G2 |

C-D1's line numbers are from the fail-before run, before `cargo fmt` reflowed three statements in that file.

After the fix, temporary edits (each restored; the files were compared with saved copies and `grep` found no leftover edit):

| Experiment (temporary edit) | Tests that failed |
|---|---|
| G1: `lease_free_working_runs` and `begin()` treat every Working Run as lease-free (`true \|\| !run_is_live(..)`) | `a_deliberate_stop_of_a_live_run…` `:488` attention not empty; `stop_archive_and_steer_of_a_live_run…` `:632` "stop"; `continue_beside_a_live_run…` `:772` attention not empty. The desktop suite still passed (154), so the live-Stop guard lives in these app tests |
| G2: lease removal deletes the file without taking the lock (`fs::remove_file` instead of `remove_free`) | `the_settings_change_also_stops_a_run_that_is_still_live` `:339` and `a_deliberate_stop_of_a_live_run…` `:489` `assertion failed: run_is_live(&derived, &id)`; `stop_archive_and_steer_of_a_live_run…` `:633` |
| G3: `ended_run_attention_with` ignores `notice(id)` (always adds) | core `per_run_notice_marks_only_lease_free_runs` (`left: [Review(op-a), Review(op-b), Blocker(live)]`, `right: [Review(op-a), Blocker(orphan), Review(op-b)]`); also round 4's `without_a_notice_only_the_review_is_added` and `an_unchanged_or_archived_chat_gets_no_write` |
| G5: `import_full_scoped` returns without lease removal | T5 `:731` "import" (configure passed) |
| G6: `project_command` returns without lease removal | T5 `:731` "configure" |
| G7: steering passes an empty orphan set | T4 `:601` "steer" (stop, archive passed): `left: [Review(op)]` |
| G8: `chat_command` computes orphans only for `Pause` | T4 `:601` "archive" (stop passed): `left: [Review(op)]` |
| G9: `begin()` skips `remove_free` for orphans (`.take(0)`) | T6 `:750` `assertion failed: !lease_file(&workspace, &id).exists()` |

What the new tests cover beyond the table:
- C-T1 also checks one `assistant-partial` Message, and that a replay of `worker-failure:1` returns the same receipt and adds no receipt, notice or Message.
- C-T2 checks the old Run is `Interrupted` at fence 2, the re-dispatch is the same item with the same owner Message (one owner Message in the chat), and a third dispatch is refused. A replayed resume-stale stop adds nothing. The item then runs once more against a scripted provider failure, and the chat holds `[Review(op), Blocker(new run, "Run failed: …")]`: one notice.
- C-D1 builds the killed queued intake before the window opens, so the desktop's startup `recover_queue` pauses the queue as after a real restart. `ui.resume_chat_queue(cx)` then goes through the resume-stale stop. Afterwards: the old Run is `Interrupted` at fence 2, the lease file is gone, the resume-stale receipt ("Chat control: stop and pause") wrote `[REVIEW_ATTENTION, INTERRUPTED_ATTENTION]`, the chat's Runs are one `Interrupted` and one `Failed` (offline), the row's reasons are `[REVIEW_ATTENTION, "Run failed: …"]`, the chat is in Needs you, the candidate is listed, and the chat has one owner Message.
- Plan deviation: the plan expected C-D1's final row reasons to be `[REVIEW_ATTENTION, INTERRUPTED_ATTENTION]`. The re-dispatched item runs in the same chat and fails offline, and its result replaces the notice. That is the one-notice rule, so the test asserts the notice in the resume-stale receipt and the failure notice on the row.
- T6 now runs the new Run against a scripted provider failure, as the plan asks, and asserts `[Review(op), Blocker(new run, "Run failed: …")]`.

### Commands and results (after the fix)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | clean |
| `cargo test --workspace --locked --offline --no-fail-fast` | 765 passed, 1 failed (the baseline Poppler test), 6 ignored. Core lib 41 (+1), core `delegation` 7, core `steering` 8, core `core` 45, app `killed_run_candidate` 12 (+5) and 1 ignored, `run_recovery` 4, `chat` 9, `steering` 2, `projects` 8, `delegation` 3, `product` 19, brn-ai lib 341 |
| desktop test command | 154 passed (+1), 0 failed, 1 ignored |
| `git diff --exit-code Cargo.lock`; `git diff --check` | clean |

Manual CLI check on fresh synthetic data. Each store was written by the ignored `native_check_store_with_a_killed_intake_and_its_candidate` (a killed intake with its candidate and an unlocked leftover lease file). `HOME`, `CODEX_HOME` and `TMPDIR` were inside the case folder.
- `/private/tmp/brn-dwr-data/r5-c-archive/` (mode `resume`): `brn archive --thread <intake>` applied "Chat control: archive chat". The lease file went from present to gone. The Run is `Interrupted` at fence 2, and the archived chat holds `[Review(op), Blocker(run, "Run interrupted. Continue starts fresh.")]`.
- `/private/tmp/brn-dwr-data/r5-c-pause/` (mode `settings`): `brn pause` applied. The lease file went from present to gone. The Run is `Cancelled` at fence 2 (slice D changes the state), and the chat holds the same two items.

### Limitations

- Liveness is read before the checked change. A Run found lease-free stays lease-free, since only a new Run ID takes a lease, so the unsafe direction cannot happen. A Run whose process dies after the check is treated as live: no notice, and its file stays until orphan recovery or a later stop removes it.
- A Working Run whose lease file was deleted while its process still runs counts as lease-free, as in orphan recovery since round 4.
- Lease removal is best effort. An I/O error keeps the file, and the change still applies.
- The legacy `resolve_thread(_because)` (no production caller) is unchanged: it still writes `Cancelled`, clears attention and leaves the file.
- No native observation in this slice.

### Integration items

- New core APIs: `run_attention::ended_run_attention_with`, `Store::chat_command_with_orphans`, `Store::stop_and_post_checked`. Old signatures are kept and behave as before.
- No desktop production change. The desktop's existing `stop_chat_run`, `resume_queue`, `archive_chat(_progress)` and `stop_and_post_with_partial` calls get the new behavior.
- The UI branch must rerun its Stop, Archive and Resume tests. A Stop, Archive or Steer of a lease-free Run now adds the notice, and Continue in a killed chat shows it until the new Run's result.
- Proposed sentence for `docs/architecture/invariants.md` (not edited here): "Any checked change that ends a Working Run no process holds a lease for treats it as orphaned: one Interrupted notice in its chat, a Review item for each candidate it left, and removal of its free lease file. A live lease is never removed, and a deliberate stop of live work adds no notice."

## Slice D: owner decision U9, go with Interrupted

### Change

`update_workspace_settings` (`crates/brn-threads-core/src/store.rs:918`) now ends every Working Run as `Interrupted` instead of `Cancelled`. The reason text, fence bump, `ended_run_attention` call and slice C's lease removal are unchanged. This covers Configure, the delegation choice, maintenance Pause/Resume and the backup folder. After a settings stop the run line now says Interrupted, matching the notice "Run interrupted. Continue starts fresh."

That was the only notice-adding path that wrote `Cancelled`. Every other path that adds the notice because the stop came from outside the chat already wrote `Interrupted`, in the same checked change as the notice: orphan recovery (`brn-threads-app/src/chat.rs:193`), project commands and project imports (`brn-threads-core/src/projects.rs:114`, `:697`), and slice C's lease-free Stop, Archive, Steer and Continue (`chat.rs:259`, `:488`, `steering.rs:160`, `brn-ai/src/threads.rs:1994`).

No dependency, `Cargo.lock`, `threads.sqlite3` schema, desktop production, CLI, `main.rs`, `HELP` or exit-code change. Saved `Cancelled` Runs are not migrated. A settings operation prepared before this change and replayed later replays its recorded request, so it still writes `Cancelled`.

### Fact check: a deliberate Stop

The spec says a deliberate owner Stop of a live worker keeps `Cancelled`. It never wrote `Cancelled`. Desktop Stop (`threads_ui.rs` `cancel`) calls `stop_chat_run`, which is `ChatCommand::Pause { run }`, which writes `Interrupted` with no notice for a live Run. The fenced worker's `finish()` then writes nothing. So "keeps Cancelled" is implemented as "keeps its current behavior": `Interrupted`, no notice, lease kept. Nothing changes for Stop, and slice C's `a_deliberate_stop_of_a_live_run_adds_no_notice_and_keeps_its_lease` still passes.

The only other `Cancelled` writer is the legacy `Workspace::resolve_thread(_because)` (`brn-threads-app/src/lib.rs:324`). It adds no notice, clears attention and has no production caller (the desktop's Archive calls `archive_chat`), so it is unchanged.

### Consumers checked

| Consumer | Where | Effect of `Interrupted` instead of `Cancelled` |
|---|---|---|
| Queue `Finish` (`success = false`) | core `chat.rs:409` | Accepts `Failed`, `Interrupted` or `Cancelled`; pauses the queue. New test D-T1 covers a settings-stopped queued Run |
| Desktop queue acknowledgement | `threads_chat.rs` `acknowledge_run` | Key `finish-queue:{run}:{fence}` uses the fence only; the non-queue branch checks `!= Working` |
| Continue | desktop `can_continue` (`threads_state.rs:104`) | Allows `Working`, `Interrupted`, `Failed` and `Cancelled`; `continuation_notice` only distinguishes `Working` |
| Notice supersession | `run_attention.rs:132`; `finish()` in `brn-ai/src/threads.rs:2474` | Treat `Interrupted`, `Failed` and `Cancelled` alike, so one notice per chat holds |
| `is_terminal_notice` | `run_attention.rs:20` | Text-based; state not read |
| Replay keys | orphan recovery `orphan-recovery:{run}:{version}:{thread version}`; settings replay the recorded request; chat commands bind the command | No key or binding includes the Run state |
| CLI terminal strings | `crates/brn/src/main.rs:363` | Prints `AiTerminal`, not `RunState`. `records` and receipts print the record JSON, now `"state":"Interrupted"` |
| Desktop run line | `threads_writing.rs:301`, `threads_ui.rs:2804` (`format!("{:?}", run.state)`) | Shows "Interrupted" with no desktop code change |
| Desktop rewrite status | `threads_workbench.rs:190` | A settings-stopped rewrite now reads "rewrite interrupted · saved feedback is ready to retry" (was "rewrite cancelled · …") |

### Tests and fail-before

Expectations were changed first and run against the unfixed `store.rs`; then the fix was applied.

| ID | Test | Result before the fix |
|---|---|---|
| D-T1 (new) | core `delegation.rs` `a_settings_stop_records_interrupted_matching_its_notice`: a queued instruction dispatched and claimed by its Working Run, then `pause_workspace(true)`. Expects the Run `Interrupted` at fence 2, the chat `[Blocker(run, INTERRUPTED_ATTENTION)]`, the queue not yet paused, then `Finish { success: false }` accepted with the queue paused and the item still pending on that Run | FAILED `:530` `left: (Cancelled, 2)`, `right: (Interrupted, 2)` |
| D-T2 | core `delegation.rs` `both_choices_and_pause_fence_every_working_run_and_block_stale_connection` | FAILED `:366` `left: Cancelled`. With `:366` reverted for the run, FAILED `:400` (the Pause leg) `left: Cancelled` |
| D-T3 | core `core.rs` `configuration_reads_and_fences_runs_at_the_actual_writer_boundary` | FAILED `:2227` `left: Cancelled` |
| D-T4 | core `backup_directory.rs` `folder_patch_preserves_current_settings_fences_runs_and_replays_raw_intent_without_filesystem` (its Run is seeded `Working` and stopped by the backup-folder change) | FAILED `:153` `left: Cancelled` |
| D-T5 | app `product.rs` `configuration_and_pause_fence_runs_while_thread_resolution_preserves_actions` | FAILED `:857` `left: Cancelled` |
| D-T6 | app `killed_run_candidate.rs` T1 `the_delegation_choice_after_a_kill_leaves_one_notice_and_a_review_for_the_candidate` | FAILED `:289` `left: (Cancelled, 2)` |
| D-T7 | app `killed_run_candidate.rs` T1-live `the_settings_change_also_stops_a_run_that_is_still_live` | FAILED `:337` `left: (Cancelled, 2)` |
| D-D1 | desktop `a_killed_intake_candidate_reaches_needs_you_after_the_delegation_choice`: the Run is `Interrupted`, and after opening the chat the run line's latest Run is that Run with status text "Interrupted" and `continue-{run}` is drawn | FAILED `:461` (state). With the state line reverted for the run, FAILED `:496`: `left: (run, "Cancelled")`, `right: (run, "Interrupted")` |

Line numbers are from the fail-before runs; `cargo fmt` later moved D-T1's assertion to `:532`. Each temporary revert was made on a saved copy and restored (`git diff --stat` afterwards showed only the intended files). Left as they are: tests that seed `Cancelled` Runs (`core.rs:1031`, `:1501`, `:1537`, `steering.rs:216`, the `run_attention.rs` unit fixtures, including `a_notice_on_a_cancelled_run_is_replaced` for old saved Runs) and the legacy resolve test (`thread_lifecycle.rs:71`).

### Commands and results (after the fix)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | clean |
| `cargo test -p brn-threads-core --locked --offline --no-fail-fast` | all passed: lib 41 (1 ignored), `delegation` 8 (+1), `core` 45, `backup_directory` 5, `steering` 8 |
| `cargo test -p brn-threads-app --locked --offline --test product --test killed_run_candidate --test delegation --test thread_lifecycle` | 19, 12 (1 ignored), 3, 3 passed |
| desktop test command | 154 passed, 0 failed, 1 ignored |
| `cargo test --workspace --locked --offline --no-fail-fast` | 766 passed, 1 failed (the baseline Poppler test), 6 ignored |
| `git diff --exit-code Cargo.lock`; `git diff --check` | clean |

Manual CLI check on fresh synthetic data, `/private/tmp/brn-dwr-data/r5-d-settings/` (store written by the ignored `native_check_store_with_a_killed_intake_and_its_candidate`, mode `settings`; `HOME`, `CODEX_HOME` and `TMPDIR` inside the case). Before: the Run `Working` at fence 1 with an unlocked leftover lease file. `brn --data-dir …/data pause` applied "Owner configured workspace and stopped prior invocations". After: the Run is `Interrupted` at fence 2, the chat holds `[Review(op), Blocker(run, "Run interrupted. Continue starts fresh.")]`, and the lease file is gone.

### Limitations

- Saved `Cancelled` Runs keep their state; the run line still shows "Cancelled" for them.
- A settings operation prepared under an older build and replayed after this change writes its recorded `Cancelled`.
- No native observation in this slice.

### Integration items

- The UI branch shows "Interrupted" after a settings change with no code change. Its tests that expect `Cancelled` after a settings change need `Interrupted`.
- No `main.rs` edit was needed.
- Proposed sentence for `docs/architecture/threads-behavior.md` (not edited here): "A Run stopped because of something outside its chat (settings, project changes or imports, process ended, or a lease-free stop) is recorded as Interrupted, matching its notice."
## Slice E: reusing an intake chat does not repeat the owner instruction

### Change

- **One instruction.** `brn_threads_app::SOURCE_INTAKE_INSTRUCTION` (`crates/brn-threads-app/src/intake.rs`, re-exported in `lib.rs`) holds the desktop's existing text verbatim. The desktop and the CLI both use it.
- **One reuse rule, in the runtime.** `brn_ai::threads::reusable_intake_instruction(records, messages, chat, prompt, references)` returns the chat's latest owner Message (durable receipt order) when:
  - its text equals `prompt`;
  - a non-archived Run in that chat has it as `instruction`;
  - that Run's `sources` hold, for every supplied reference, a Source that `matching_sources` (slice B) matches.

  `begin()` applies it when the request has no `request_message` and no `owner_request`, and the chat's last Message is not already that instruction (the existing rule). It sets `instruction` to that Message and writes none. A Run without sources never matches. `begin()`'s message ordering moved into a shared `chat_messages` helper, with the same order as before.
- **Host wrapper.** `brn_ai::threads::intake_instruction_to_reuse(store, chat, prompt, references)` applies the same rule to saved state and canonicalizes the references. It returns the core `Result` rather than `AiResult`, so the app keeps the store's error through `AppError::Core`.
- **Desktop.** `Workspace::post_intake_instruction(chat, locator) -> AppResult<Option<Receipt>>` writes nothing and returns `None` when that rule finds an instruction; otherwise it posts `SOURCE_INTAKE_INSTRUCTION`. `useful_intake` (`crates/brn-desktop/src/threads_ui.rs`) uses the constant and calls `post_intake_instruction` instead of `post_message`; the `Ok(_)` arm is unchanged. rustfmt split that `match` head over four lines. This is the call plumbing the slice allows. The desktop's visible text is unchanged.
- **CLI.** The useful-intake `ask(..)` call (`crates/brn/src/main.rs`) passes `brn_threads_app::SOURCE_INTAKE_INSTRUCTION.into()`. The old literal had kept rustfmt from formatting the call; without it, rustfmt split the call over eight lines (`git diff --stat`: 8 insertions, 1 deletion). The call and its arguments are otherwise unchanged. `HELP`, exit codes and `ask`'s signature are unchanged.
- **Noted visible change (CLI wording).** CLI useful intake now posts the desktop wording. It drops the CLI-only phrases "Reconcile current notes and suggested Actions", "leave raw text external" and "ask for review of protected decisions". The playbooks preloaded for every source run still carry that guidance: `agent/skills/intake/SKILL.md` ("Original files and messages remain external", "A source request does not authorize changing a protected decision") and `agent/skills/maintain-notes/SKILL.md` (write groups include "related Thread and supported Action context").

No dependency, `Cargo.lock`, `threads.sqlite3` schema, `HELP`, exit-code or `docs/status.md` change. All writes still go through the checked store (`post_message` and `begin()`'s start change); the rule only decides whether to add a Message.

### Behavior table

| Sequence (same source, same chat) | Before | After |
|---|---|---|
| CLI, then desktop | 2 owner Messages (two wordings) | 1; both Runs name it |
| Desktop, then desktop | 2 identical (`post_message` was unconditional) | 1 |
| CLI or desktop re-supply after an assistant reply | 2 identical | 1 |
| CLI, then CLI, offline failure (last Message is the instruction) | 1 (existing rule) | 1 |
| New owner text between supplies | fresh instruction | fresh instruction |
| Another source in the chat (owner-named chat) | fresh instruction | fresh instruction |
| Latest instruction in other wording (a chat from an older CLI) | fresh instruction | fresh instruction |

### Tests and fail-before

Fail-before method. The new tests need `SOURCE_INTAKE_INSTRUCTION` and `post_intake_instruction`, so they cannot compile against the unfixed tree. "Rule disabled" makes `reusable_intake_instruction` return `None` (`if true || references.is_empty()`). That restores both pre-fix behaviors: the desktop posts every time, and `begin()` reuses only a last Message. "Old CLI wording" puts the CLI literal back in `main.rs`. Guard tests got their fail-before by breaking the named check. Each mutation was made on a saved copy and then restored; `git diff --stat` afterwards showed only the intended files.

| ID | Test | Before the fix (mutation) | After |
|---|---|---|---|
| E-T1 | app `source_intake_chat.rs` `a_cli_then_desktop_supply_of_one_source_leaves_one_instruction_message`: CLI-shape Run with a scripted reply; the desktop reuses the chat, `post_intake_instruction` returns `None` and writes no receipt; one owner Message with the constant, named by both Runs | FAILED `:798` `assertion failed: w.post_intake_instruction(..).is_none()` (rule disabled) | ok |
| E-T2 | app `a_resupply_after_an_assistant_reply_reuses_the_instruction` (`begin()` alone, last Message `assistant`) | FAILED `:822` `left: 2, right: 1` (rule disabled) | ok |
| E-T3 | app `new_owner_text_in_between_posts_a_fresh_instruction`: desktop leg and CLI leg after owner text; owner texts `[I, "Focus on the berth depth", I, "Also the crane date", I]`, each Run names its own | guard, ok with the rule disabled. FAILED `:835` (`post_intake_instruction` returned `None`) when the rule takes the latest owner Message *with this text* instead of the latest owner Message | ok |
| E-T4 | app `a_different_source_in_the_same_chat_posts_its_own_instruction`: desktop leg (second source) and CLI leg (third source); 3 instructions | guard. FAILED `:871` when the per-source check is dropped | ok |
| E-T5 | app `other_instruction_wording_posts_a_fresh_instruction`: the latest instruction is the old CLI text; a fresh shared instruction is posted | guard. FAILED `:902` when the text check is dropped | ok |
| E-D1 | desktop `supplying_the_same_source_twice_posts_one_instruction`: two `supply` calls, one owner Message with the constant, both `Failed` Runs name it | FAILED `:290` `left: 2, right: 1` (rule disabled) | ok |
| E-D2 | desktop `a_cli_useful_intake_then_a_desktop_supply_post_one_instruction`: the CLI's `source_intake_chat` and `run` in-process (offline, `Failed`), then the desktop harness `supply` | FAILED `:311` `left: 2, right: 1` (rule disabled) | ok |
| E-D3 | desktop `owner_text_between_supplies_gets_a_fresh_instruction` | guard. FAILED `:331` (texts `[I, "Focus…"]`, expected `[I, "Focus…", I]`) with the latest-owner mutation used for E-T3 | ok |
| E-D4 | desktop ignored `a_desktop_supply_reuses_the_chat_a_cli_useful_intake_left`, extended: no owner Message added, the latest is the constant, and at least 2 Runs name it | FAILED `:872` with a CLI built with the old wording: owner Messages `[CLI text, desktop text]` against `[CLI text]` | ok (below) |
| E-C1 | CLI `source_locator.rs` `two_cli_useful_intakes_of_one_source_post_one_instruction`: one owner Message whose text is the constant; both Runs' `instruction` name it | count part is a guard (ok with the rule disabled: an offline failure leaves the instruction last). FAILED `:151` with the old CLI wording: `left: "Retain useful information from the supplied source. Reconcile…"` | ok |

The first run of the old-wording E-C1 failed once at `:18` instead (a `brn` command exited non-zero; stderr was not captured). It was the first run after a relink. The next six runs of the same tree failed only at `:151`. Eleven runs of the fixed tree, including three after a forced relink and two after content changes to `main.rs`, all passed. The cause is not known.

### Desktop plus CLI from one build, shared synthetic data

Each case below ran with its own `HOME`, `CODEX_HOME` and `TMPDIR`, Codex configured with a missing auth file, and the binary `/tmp/brn-dwr-target/debug/brn`.

- **Before, `/private/tmp/brn-dwr-data/r5-e-cli-desktop-before/`** (CLI built with the old literal):
  1. CLI `intake --source …/mari-email.txt --intent useful`: `Failed:ReconnectNeeded`, `reused_chat: false`.
  2. The desktop harness (E-D4) with `BRN_DWR_SHARED_DATA`, `_SOURCE` and `_CHAT`, `-- --ignored`: FAILED `:872`, because the desktop added its own instruction.
- **After, `/private/tmp/brn-dwr-data/r5-e-cli-desktop/`** (fixed build; the binary has no "Reconcile current notes and suggested Actions" string):
  1. CLI intake: `Failed:ReconnectNeeded`, chat `e39c029e-…:0`, `reused_chat: false`.
  2. The desktop harness E-D4: ok.
  3. CLI intake again: `reused_chat: true`, same chat.

  Persisted state: 1 Thread, **1** owner Message (the shared text), 3 `Failed` Runs that all name it, and 1 Source at the real path.

### Commands and results (after the fix)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | clean |
| `cargo test -p brn-threads-app --locked --offline --test source_intake_chat --test daily_workflow_eval` | 16 (+5), 1 passed |
| `cargo test -p brn-ai --locked --offline --lib` | 341 passed, 1 ignored |
| `cargo build -p brn --locked --offline && cargo test -p brn --locked --offline --test source_locator --test threads` | 2 (+1), 13 passed |
| desktop test command | 157 passed (+3), 0 failed, 1 ignored (E-D4, run by hand above) |
| `cargo test --workspace --locked --offline --no-fail-fast` | 772 passed (+6), 1 failed (the baseline Poppler test), 6 ignored |
| `git diff --exit-code Cargo.lock`; `git diff --check` | clean |

### Limitations

- If the desktop posts the instruction and the Run then never starts (another worker is busy, or `prepare_run` refuses), no Run names that Message. A re-supply after other Messages then posts another. If the instruction is still the chat's last Message, `begin()`'s existing rule reuses it.
- A chat whose latest instruction is the old CLI wording gets one fresh instruction in the shared wording on its next supply (E-T5).
- `post_intake_instruction` and `begin()` read the state separately. A write in between (for example, owner text from another process) can make the desktop skip the post while `begin()` then posts a fresh instruction. The result is a correct instruction, never a missing one.
- No native observation in this slice.

### Integration items

- `crates/brn/src/main.rs`: one statement in the useful-intake sub-block, reflowed by rustfmt to 8 lines. It must survive the merge with work/cli-agent-contract, which keeps that block untouched; take this side.
- `crates/brn-desktop/src/threads_ui.rs`: the `useful_intake` call plumbing above. The UI branch reruns its intake tests. Any UI test that counts owner Messages after repeated supplies of one source now sees one.
- New public items: `SOURCE_INTAKE_INSTRUCTION`, `Workspace::post_intake_instruction`, and `brn_ai::threads::{reusable_intake_instruction, intake_instruction_to_reuse}`.
- Proposed sentence for `docs/architecture/threads-behavior.md` (not edited here): "Useful intake uses one owner instruction from every host. A re-supply of the same source in its chat reuses the chat's latest instruction for that source unless the owner has written since."

## Slice F: saved chat replies are findable in owner search

### Change

- **Core scope (`crates/brn-threads-core/src/reader.rs`, `context.rs`, `lib.rs`).** Owner search places a chat's Messages where its notes would appear:
  - `MessageScope { General, Project(id), Chat(id), Hidden }` and `message_scope(thread, rows)`. An archived chat is `Hidden`; this covers Archive and a project Delete, which archives the project's chats. A chat in a project is `Project(p)`, or `Hidden` while `p` is in Trash. A chat with private context is `Chat(id)`. Every other chat is `General`.
  - Private context is the existing `chat_has_private_context` rule outside project membership: the chat's own notes (archived or not) or sticky `Thread.private_context`. That rule now lives in a pure `context::chat_private(rows, chat)`, and `chat_has_private_context` keeps its behavior.
  - `message_scopes_for(rows, context)`: general search shows `[General]`. A project, or a chat in it, shows `[Project(p)]`, plus `General` when `use_brn_knowledge` is on; a project in Trash shows nothing. A general chat shows `[General, Chat(itself)]`.
  - `message_searchable(record)`: a current Message with role `owner` or `assistant` (`SEARCHABLE_MESSAGE_ROLES`). `assistant-partial` and `writing-request` are never searchable.
  - `Snapshot::{message_scope_rows, message_scopes, searchable_messages}`. `searchable_messages` is the authority re-check for every hit, over fresh rows: the chats, every Project, and the Notes saved in those chats. Agent runs keep `record_in_chat`; nothing about them changed.
- **Derived index (`crates/brn-retrieval/src/search_index.rs`), schema v3.** Messages live in their own tables, so no note lane, count, fact, link count or embedding job sees them. `docs.kind` still writes only `'note'`.
  - New tables: `chats(id, scope)` and `messages(rid AUTOINCREMENT, id UNIQUE, chat, version, role CHECK owner|assistant, text)`, with `messages_chat`.
  - New FTS tables `fts_message_exact` and `fts_message_stem` (`content='messages'`), with insert and delete triggers for both and a `messages_no_update` trigger (replace-only).
  - `OBJECTS` lists every new name. The version bump turns a v2 file into `Found::Rebuild`: it is rebuilt in place, its sync state is cleared, and the next sync is full and backfills every Message. `threads.sqlite3` is unchanged.
  - `ChatScope`, `MessageInput`, `MessageHit`; `Changes.{upsert_messages, remove_messages, chat_scopes}` (a full sync also clears `messages` and `chats`).
  - `message_hits(query, scopes, per_lane ≤ MAX_MESSAGE_LANE = 100)`: RRF over the exact and stem lanes, ties broken by text then ID. `message_count(query, scopes)` is exact. `doc_chats(ids)` gives the `chat:` visibility a note had.
- **Worker (`crates/brn-threads-app/src/search.rs`).**
  - `sync()` upserts each changed searchable Message and removes any other changed Message. It rescopes chats from changed Threads, from the chat of each changed Note, from the chat a changed note had before (`doc_chats`, for Keep in BRN), and from changed Messages. A Project change rescopes every chat from one `records()` read.
  - `Work::Search` gained `messages: bool`. `request_page` and `request_keyword_page` (owner, desktop, CLI) set it. `message_search()` filters by scope tags in the index, re-checks each hit with core `searchable_messages`, and drops and retries a hit whose Message version moved, as for notes.
  - Pages share the notes' offset. `message_next` is set while more Messages remain. `cursor`/`next` stay notes-only, so the desktop's Previous/Next is unchanged.
- **Agent tools do not see Messages.** `RunNoteSearch for SearchHandle` now calls a private `notes_request(.., messages: false)`; `search_notes` output is unchanged. This is the plan's default (Decision 5): an agent-facing chat lane would need its own scope analysis against `scope_allowed`.
- **Page (`crates/brn-threads-app/src/lib.rs`).** `SearchPage` gained serde-default `messages: Vec<ChatMessageHit { message, version, chat, chat_title, role, snippet, highlights }>`, `message_total` and `message_cursor`. `total`, `hits`, `cursor` and `reachable` still describe notes only.
- **CLI.** No `main.rs` change. `search` prints `encode(page)`, so its JSON gains `messages`, `message_total` and `message_cursor`. `shown` still counts note hits. `HELP` and exit codes are unchanged.
- **Desktop.** No production change. The notes-tile search renders `page.hits` only, so Message hits reach the desktop but are not shown: UI finding F-UI1 below. One desktop test checks the data the desktop receives.

A saved reply stays text in BRN: hits carry the Message's role and text, and nothing is labelled sent. No dependency, `Cargo.lock`, `threads.sqlite3` schema, `main.rs`, `HELP`, exit-code or `docs/status.md` change.

### Scope table (owner search, after the fix)

| Chat | General search | That project (knowledge on / off) | Another project | Itself | Another general chat |
|---|---|---|---|---|---|
| General chat | shown | shown / not shown | shown if its knowledge is on | shown | shown |
| Project chat | not shown | shown / shown | not shown | shown (project scope) | not shown |
| Chat with its own notes, or moved out of a project | not shown | not shown | not shown | shown | not shown |
| Archived chat, or project in Trash | not shown | not shown | not shown | not shown | not shown |
| After Unarchive or Restore | as before | as before | as before | as before | as before |

### Tests and fail-before

Method: the implementation was written first. Each mutation below was applied by an exact single-match replacement on a backup copy, and the tests were run and then restored. After every restore the four source files matched recorded `shasum`s. G1 is the plan's "stub fields always empty" state: owner pages never fill `messages` (`if *messages && query.is_empty()`). Guard behaviors got their own mutations. Line numbers are those of the committed files.

| ID | Test | Before the fix (mutation) | After |
|---|---|---|---|
| F-A1 | app `search_messages.rs` `a_saved_reply_is_found_with_its_chat`: a scripted run completes with "Draft reply (not sent): …"; `messages[0]` is `assistant`, with chat and title, snippet and highlight; note hits and `total` unchanged | FAILED `:189` `left: 0, right: 1` (G1) | ok |
| F-A2 | app `owner_messages_are_found_and_partial_progress_is_not`: Stop with partial text | FAILED `:251` `left: [], right: [(…:1, "owner")]` (G1) | ok |
| F-A3 | app `project_chats_appear_only_in_their_project_search`: general, `Some(project)` and `Some(project chat)` for two projects, a general chat, knowledge off | FAILED `:269` `left: {}` (G1) | ok |
| F-A4 | app `archived_and_trashed_chats_disappear_and_return_on_restore`: `archive_chat`/`unarchive_chat`, project Delete then Restore | FAILED `:294` `left: {}` (G1) | ok |
| F-A5 | app `an_archived_chats_messages_never_crowd_out_visible_ones`: 100 archived matches rank above one visible match | FAILED `:353` `left: 0, right: 101` (G1). FAILED `:87` `left: 1, right: 0` (P3: no rescope on a Thread change; the archived chat's 100 hits fill the window and the visible one is unreachable) | ok |
| F-A6 | app `a_private_attachment_chat_is_searchable_only_from_itself`, then the attachment kept in BRN (its `chat` cleared) | FAILED `:370` (G1). FAILED `:405` `left: {}` (P1: the chat a note left is not rescoped) | ok |
| F-A7 | app `rebuilding_the_index_backfills_messages`: the previous build's state (synced to the current watermark, no messages, `user_version` 2 patched into the closed file), then a deleted index | FAILED `:442` `left: {}` (G1). FAILED `:442` (R2: schema stays 2) | ok |
| F-A8 | app `agent_search_never_returns_messages`: `RunNoteSearch::search` hybrid and keyword | FAILED `:465` (G1, owner precondition). Guard: FAILED `:469` `left: (1, 0, 0, None)` (A1: agent tools receive owner Message results) | ok |
| F-A9 | app `message_pages_follow_the_cursor`: 60 matches, limit 50 | FAILED `:495` `left: (0, 0, None, None, 0)` (G1). FAILED `:495` `left: (50, 60, None, None, 0)` (P2: message lanes keep 50 per lane) | ok |
| F-C1 | CLI `crates/brn/tests/search_messages.rs` `search_lists_a_saved_chat_message_with_its_chat`: `thread-start`, offline `ask`, `search --query berth` (exit 0), then `--thread <other general chat>` | FAILED `:68` `left: Number(0), right: Number(1)` (G1) | ok |
| F-D1 | desktop `threads_intake_tests.rs` `the_notes_search_answer_carries_a_saved_chat_message`: `search_changed("berth")`; `results.page.messages[0]` has the chat | FAILED `:901` `left: 0, right: 1` (G1) | ok |
| F-R1 | retrieval `search_index.rs` `messages_are_replaced_and_removed_without_stale_terms`: inflection, replace, remove, replace-only trigger, role refusal, full resync, FTS `integrity-check` | FAILED `:687` (R1: upserts ignored). FAILED `:670` `database disk image is malformed` (R4: no FTS delete triggers) | ok |
| F-R2 | retrieval `message_scopes_filter_hits_and_count_exactly_apart_from_notes`: six scope sets, exact counts, note lanes empty, rescope without reindex, tie order, lane bound | FAILED `:803` (R1). FAILED `:803` `left: ["mc", "mg", "mh", "mp"], right: ["mg"]` (R3: scope filter dropped) | ok |
| F-R3 | retrieval `a_version_2_index_is_rebuilt_with_message_tables` | FAILED `:878` `left: Existing, right: Rebuilt` (R2). FAILED `:894` (R1) | ok |
| F-S1 | core `message_search_scope.rs` `a_general_chat_is_searchable_wherever_general_notes_appear` | FAILED `:185` `left: Hidden, right: General` (C1: stubs) | ok |
| F-S2 | core `a_project_chat_is_searchable_only_in_its_project` | FAILED `:211` (C1). FAILED `:211` `left: General` (C7: project chats scoped general) | ok |
| F-S3 | core `a_project_without_brn_knowledge_shows_only_its_own_chats` | FAILED `:240` (C1). FAILED `:243` (C4: knowledge ignored) | ok |
| F-S4 | core `an_archived_chat_is_hidden_until_it_is_unarchived` | FAILED `:274` (C1). FAILED `:264` `left: General, right: Hidden` (C2: archived not hidden) | ok |
| F-S5 | core `a_deleted_project_hides_its_chats_and_restore_returns_them` | FAILED `:308` (C1). FAILED `:293` `scopes(..).is_empty()` (C3: a project in Trash keeps its scopes) | ok |
| F-S6 | core `a_chat_moved_out_of_a_project_stays_searchable_only_from_itself` | FAILED `:320` (C1). FAILED `:339` `left: General` (C5: private context ignored) | ok |
| F-S7 | core `a_chat_with_its_own_attachments_is_searchable_only_from_itself` (also `chat_has_private_context`) | FAILED `:368` (C1). FAILED `:368` (C5) | ok |
| F-S8 | core `partial_progress_and_writing_requests_are_never_searchable` | FAILED `:399` (C1). FAILED `:399`, 4 IDs instead of 2 (C6: every role searchable) | ok |

P3 alone passes F-A4, because core `searchable_messages` still hides the archived chat's hits. F-A5 was added so the index-side rescope has a failing test: without it, the archived chat's rows fill the 100-hit window.

Build note: the first mutation pass restored files with their old mtime. Cargo then reused the P3-mutated library once, and F-A5 failed right after it was added, before a fresh build passed it. The helper now touches each restored file. Every result in this table comes from a second, complete pass with fresh builds. Every after-fix command below ran on builds made after the final restore.

### F-UI1: the desktop does not show Message hits

The notes-tile search (`crates/brn-desktop/src/threads_search.rs`) renders `self.hits`, which holds note hits only, and its status line uses `page.total` (notes). `page.messages` arrives but nothing renders it. Repro:

1. `export HOME=… CODEX_HOME=… TMPDIR=…` (fresh), with a fresh data folder `D=/private/tmp/brn-dwr-data/r5-f-ui1/data`.
2. `brn --data-dir $D init`, then `brn --data-dir $D configure --provider Codex --model gpt-6.1-sol --effort medium --auth-file $D/../missing-auth.json`.
3. `brn --data-dir $D thread-start --title Replies` and note the `id`.
4. `brn --data-dir $D ask --thread <id> --prompt "Berth survey reply draft 1.4 m"`. The run fails offline (`Failed:ReconnectNeeded`) and the owner Message is saved.
5. `brn --data-dir $D search --query berth`: `message_total: 1`, and `messages[0]` has `chat: <id>`, `chat_title: "Replies"`, `role: "owner"`.
6. `brn-desktop --data-dir $D`, open Notes, and type `berth` in "Filter titles and search note text". The tile shows only "No current note text matches. Try another word or clear the filter to browse.", with no status line and no result row.

Headless observation of step 6, on the same synthetic content, in a temporary desktop test that was then removed: after `open-notes` and `search_changed("berth")`, `results.page.messages.len() == 1`. The only element under `search-results` was `search-empty`; there was no `search-status` and no `hit-0`. The UI branch can render `page.messages` (for example as an "In chats" block that opens `chat`) without app changes.

### Manual CLI case (fresh synthetic data)

`/private/tmp/brn-dwr-data/r5-f-search-messages/` used its own `HOME`, `CODEX_HOME` and `TMPDIR`, a missing auth file and `/tmp/brn-dwr-target/debug/brn`:

1. `ask` in general chat "Replies" and in project chat "Crane chat": both `Failed:ReconnectNeeded`, and both owner Messages are saved.
2. `search --query berth`: `message_total=1`, Replies [owner]. Notes `total=0 shown=0`.
3. `search --query berth --project <Crane>`: `message_total=2`, Crane chat and Replies.
4. `archive --thread <Replies>`, then search: `message_total=0`. `unarchive`, then search: Replies is back.
5. A v2 index from the previous build: Python `sqlite3` dropped the message objects and set `user_version=2` while keeping `watermark=19`. The next `search` found Replies, and the file then had `user_version=3`, 2 messages and 2 chats (backfilled).
6. A deleted `derived/search.sqlite` was re-created by the next `search`, which found Replies; exit code 0.
7. **Observed, not part of this slice:** `project-delete` of Crane failed with `invalid request: run control changes must advance the fence` (exit 1). The project's chat had a Run from `ask`. `ProjectCommand::Delete` (`projects.rs`) archives the chat's Runs without advancing their fence, and `store.rs` requires a fence advance for an archive change; Restore does advance it. The code is the same at `771116f0`. The Trash and Restore behavior for Messages is covered by F-A4 and F-S5, whose project chats have no Run. This needs its own slice and owner decision.

The case folder was deleted afterwards.

### Commands and results (after the fix)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | clean |
| `cargo test -p brn-threads-core --locked --offline` | all passed; `message_search_scope` 8 (new) |
| `cargo test -p brn-retrieval --locked --offline` | lib 20, `search_index` 16 (+3) |
| `cargo test -p brn-threads-app --locked --offline --test search_messages --test search_worker --test chat --test projects` | 9 (new), 11, 9, 8 passed |
| `cargo test -p brn-threads-app --locked --offline --test user_scale -- --nocapture` | ok, non-strict bounds: 10,000 notes, cold build 545 ms, reopen 24.2 ms, one-note sync 0.55 ms, query p95 22.9 ms (owner path, with the message lanes; an earlier run gave 569 ms / 24.8 ms / 0.65 ms / 23.1 ms) |
| `cargo test -p brn-threads-app --features native-retrieval --locked --offline --test search_dense` | 6 passed (native lane builds offline) |
| `cargo build -p brn --locked --offline && cargo test -p brn --locked --offline --test search_messages --test source_locator --test threads` | 1 (new), 2, 13 passed |
| desktop test command | 158 passed (+1), 0 failed, 1 ignored |
| `cargo test --workspace --locked --offline --no-fail-fast` | 793 passed (+21), 1 failed (the baseline Poppler test), 6 ignored |
| `git diff --exit-code 771116f0 -- Cargo.lock`; `git diff --check` | clean |

### Limitations

- The desktop shows no Message hits until the UI renders `page.messages` (F-UI1).
- Owner pages reach the best 100 Messages (`MAX_MESSAGE_LANE`); `message_total` still counts every match. The dense (semantic) lane covers notes only.
- The index's chat scope tag is only a prefilter, and core `searchable_messages` decides every hit. A tag that misses a change can only hide a Message that should show, never show one that should not. Every known scope change rescopes the chat: a Thread, chat-note or Project change, or Keep in BRN. A chat-only note that was never indexed (for example archived) and then loses its `chat` is the one case left; the Thread's next change or a rebuild corrects it.
- The first open after the merge rebuilds `search.sqlite` in place (`Opened::Rebuilt`). A rollback to an older build rebuilds it again, since v3 is not v2.
- The pre-existing project Delete refusal for project chats with a Run (manual case, step 7).
- No native observation in this slice.

### Integration items

- `search.sqlite` schema v3 (rebuilt in place on first open).
- New public items: core `MessageScope`, `SEARCHABLE_MESSAGE_ROLES`, `message_scope`, `message_scopes_for`, `message_searchable` and `Snapshot::{message_scope_rows, message_scopes, searchable_messages}`. Retrieval `ChatScope`, `MessageInput`, `MessageHit`, `MAX_MESSAGE_LANE`, the `Changes` fields and `SearchIndex::{message_hits, message_count, doc_chats}`. App `search::MessageMatch`, the `SearchResults` fields, `ChatMessageHit` and the serde-default `SearchPage` fields.
- CLI `search` JSON gains `messages`, `message_total` and `message_cursor`. Tell work/cli-agent-contract in case its `--json` description lists output fields.
- UI branch: F-UI1. Render `page.messages` as their own kind, open `chat` on click, and page with `message_cursor` (the same `cursor` input).
- The knowledge-retrieval plan's projects hook "Indexing of `kind=message` for earlier-chat search" is done for owner search, through separate tables rather than `docs.kind`. Agent `search_notes` still excludes Messages.
- Proposed sentence for `docs/architecture/threads-behavior.md` (not edited here): "Owner search also lists saved final chat Messages (owner and assistant, never partial progress) as their own kind with their chat, scoped like notes: general chats in general search, project chats only in their project's search, and a chat with private context only from itself. Archived and Trashed chats are excluded until restored. Agent note search never returns chat Messages."
- Separate finding for a later slice: project Delete refuses when a project chat has a Run (manual case, step 7).

## Slice G: the agent can withdraw its own obsolete proposal

### Change

- **Core shared dismissal path (`crates/brn-threads-core/src/run_attention.rs`).** The app's review cleanup loop moved into `impl Store`:
  - `clear_review_attention(candidate, key_prefix, reason, authority) -> CleanupOutcome` removes every attention item naming the candidate, in one checked change keyed `{key_prefix}:{candidate}:{digest}`, with up to 3 Stale retries. Other attention stays.
  - `dismiss_candidate(..)` is `prepared`, then `retire` (durable first; refuses an applied operation), then `clear_review_attention`. It never applies anything.
  - `CleanupOutcome { Done(Option<Receipt>), Unexpected(ApplyOutcome), Contended }`.
- **App (`crates/brn-threads-app/src/review.rs`).** `review_dismiss` and `review_apply` call these with the same key prefix (`review-attention-cleanup`), reasons, `HostAuthority::owner("owner interaction")` and error texts as before. Owner receipts, keys and messages are unchanged; the existing owner dismiss and apply tests pass unedited.
- **Runtime tool `withdraw_proposal { operation }` (`crates/brn-ai/src/threads.rs`).** Strict schema; `WithdrawArgs` has `deny_unknown_fields`, and `parse_valid` accepts it. The handler runs in this order:
  1. It is rejected in a writing (Rewrite) run, and `live(scope)` must hold.
  2. The operation must be in a Run of this chat (the `apply_changes` membership check). Owner-created operations are never in `Run.operations`, and other chats' and projects' proposals are in other chats' Runs, so all of these get the generic rejection, as does an unknown ID.
  3. Applied (a receipt exists): `{"outcome":"Refused","reason":"This proposal was applied; prepare a new change to revise it."}`.
  4. Already retired: `clear_review_attention` (which repairs a lost cleanup), then `{"outcome":"Withdrawn","operation":…,"replay":true}`.
  5. Otherwise `dismiss_candidate(op, "{run}:withdraw", "Agent withdrew its obsolete proposal; preserve other attention requests", bind(scope, owner("runtime withdrawal")))`, then `{"outcome":"Withdrawn","operation":…}`. The authority is bound to this Run's fence, chat, Project context and Settings, so a stopped Run's cleanup is refused (`Superseded`, returned as a rejection).
- **Registration.** `runtime_catalog!` gains `(WithdrawProposal, "Withdrawing an obsolete proposal…")`, which is new visible progress text. The activity outcome reads `< withdraw_proposal · Withdrawn`, `Refused` or `error` instead of `success`. The trace records only the operation ID. The tool is not in `HELPER_TOOLS`, the helper builder does not register it, and the `tool!` guard rejects it in a helper scope. **Registered tools: 9 → 10.**
- **Guidance (bundle `brn-threads-guidance-11` → `brn-threads-guidance-12`).**
  - `agent/BRN.md`, after the candidate paragraph: "When the owner's decision makes one of your own pending proposals in this chat obsolete, withdraw it with `withdraw_proposal`. Never withdraw the owner's changes or another chat's proposals; withdrawing never applies anything."
  - `agent/skills/resolve-conflict/SKILL.md` step 4: "After the owner decides, withdraw your own pending proposal that contradicts the decision with `withdraw_proposal`."
  - The research-scout role guide is unchanged and never names the tool.
- **`crates/brn-ai/README.md`** says ten registered tools.

Undo follows the owner's Dismiss. Retirement is recorded on the operation; the attention cleanup is a checked change with a receipt whenever an attention item named the candidate. Undo of that receipt restores the Review item, and the candidate stays retired, so an owner Apply is still refused. A withdrawal of a candidate that no attention named leaves no cleanup receipt, as Dismiss does.

No dependency, `Cargo.lock`, `threads.sqlite3` schema, `main.rs`, `HELP`, exit-code, desktop or `docs/status.md` change.

### Tests and fail-before

Method: the implementation was written first. Each mutation below was applied to the fixed source by an exact single-match replacement. The affected tests were run, then the file was restored and touched so Cargo rebuilt it. After the pass, all five mutated files matched their recorded SHA-256. M0 is the state before the tool existed (no catalog entry, no `parse_valid` arm). Guards got their own mutations. Line numbers are those of the committed files.

| ID | Test | Before the fix (mutation) | After |
|---|---|---|---|
| G-A1 | app `withdraw_proposal.rs` `the_agent_withdraws_its_own_obsolete_proposal_after_the_owner_decides`: review-first; Run 1 prepares "15 November" (NeedsReview, one Review item); owner posts "Keep 22 November."; Run 2 withdraws. Not listed, retired, no attention names it, one cleanup receipt writing only the chat, no receipt for the operation, owner Apply refused, note unchanged | FAILED `:147`, Run 2 `Failed(InvalidToolUse, origin "unknown_tool")` (M0). FAILED `:222`, attention still names it (M4: retire only) | ok |
| G-A2 | app `withdrawing_another_chats_proposal_is_refused`: outcome `error`; candidate and the other chat's attention unchanged | FAILED `:147` (M0). FAILED `:251` `listed(..)`, it was withdrawn (M1: no membership check) | ok |
| G-A3 | app `withdrawing_an_owner_created_candidate_is_refused` (`allocate_operation` + `prepare_owner`, with an owner Review item) | FAILED `:147` (M0). FAILED `:302` `left: "Withdrawn", right: "error"` (M1) | ok |
| G-A4 | app `withdrawing_an_applied_proposal_is_refused` (maintenance, Applied): `Refused`; receipt and note unchanged | FAILED `:147` (M0). FAILED `:318` `left: "Withdrawn", right: "Refused"` (M2: no receipt check; the applied operation is not an active candidate, so the replay path took it) | ok |
| G-A5 | app `withdrawing_twice_replays_without_another_receipt`: second call `Withdrawn` with `replay: true`; cleanup receipts identical | FAILED `:147` (M0). FAILED `:335` `left: "error", right: "Withdrawn"` (M3: retired not replayed). FAILED `:333` `left: 0, right: 1` (M4) | ok |
| G-A6 | app `undo_after_a_withdrawal_matches_undo_after_dismiss`: one candidate withdrawn, another dismissed by the owner in another chat; Undo of each cleanup is `Applied`, each Review item returns, both stay retired and owner Apply is refused for both | FAILED `:147` (M0). FAILED `:353` `Err([])`, no cleanup receipt to undo (M4) | ok |
| G-R1 | brn-ai `research_tests::the_helper_is_never_offered_withdraw_proposal`: a helper that calls it fails and the proposal stays; the helper's `requests()[0].tools` and `HELPER_TOOLS` lack it; a direct call in a helper scope is `ToolRejected`; the parent's direct call withdraws | FAILED `:549` `unwrap_err()` on `Ok({"outcome":"Withdrawn"…})` (M5: helper guard exempts it). FAILED `:536` tool list (M6: helper builder registers it) | ok |
| G-R2 | brn-ai `production_catalog_tools_report_activity_and_tool_loop_saves_only_final_once`: `tool_names().len() == 10` and contains it | FAILED `:3916` `left: 9, right: 10` (M0) | ok |
| G-R3 | brn-ai `threads_guides::guidance_says_when_to_withdraw_an_obsolete_proposal`: preambles carry `brn-threads-guidance-12`; `BASE` and the resolve-conflict skill say when to withdraw; the scout guide does not name it | FAILED `:107` bundle (M7: still -11). FAILED `:109` `BASE` (M8). FAILED `:110` skill (M9) | ok |
| G-R4 | brn-ai `activity_reports_apply_domain_outcomes_without_exposing_results` (extended): `Withdrawn`/`Refused` labels, otherwise "outcome unconfirmed" | FAILED `:2897` `left: "success", right: "Withdrawn"` (M10) | ok |

M1 shows why the membership check matters: without it, an owner-created candidate was withdrawn. M2 shows why the receipt check comes before the replay path.

### Commands and results (after the fix)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | clean (desktop calls `review_dismiss`) |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline` | ok |
| `TMPDIR=/private/tmp/brn-dwr-data/r5-g-withdraw cargo test -p brn-threads-app --locked --offline --test withdraw_proposal` | 6 passed (new), fresh synthetic data |
| `TMPDIR=/private/tmp/brn-dwr-data/r5-g-withdraw cargo test -p brn-ai --locked --offline -- the_helper_is_never_offered_withdraw_proposal guidance_says_when_to_withdraw production_catalog_tools activity_reports_apply` | 4 passed |
| `cargo test -p brn-threads-core --locked --offline` | all passed |
| `cargo test -p brn-ai --locked --offline` | 343 passed (+2), 1 ignored |
| `cargo test -p brn-threads-app --locked --offline --test withdraw_proposal --test agent_journeys --test product --test chat_decisions --test writing --test killed_run_candidate` | 6, 4, 19, 4, 9, 12 passed (1 ignored) |
| desktop test command | 158 passed, 0 failed, 1 ignored |
| `cargo test --workspace --locked --offline --no-fail-fast` | 801 passed (+8), 1 failed (the baseline Poppler test), 6 ignored |
| `git diff --exit-code 771116f0 -- Cargo.lock`; `git diff --check` | clean |

### Limitations

- Whether a live model withdraws at the right moment is not measured; only the guidance and the scripted runs are.
- An operation that refresh or an owner edit already replaced is retired, so withdrawing it replays as `Withdrawn` and leaves the replacement alone. A refreshed replacement is in this chat's Run and can be withdrawn by its own ID. An owner-edited replacement is an owner change and is refused.
- A writing proposal from an earlier Rewrite run in the same chat is the agent's own proposal, so a later ordinary run may withdraw it. A Rewrite run cannot withdraw anything.
- If another process stops the Run between retirement and cleanup, the candidate stays retired with its Review item, and the tool returns a rejection. The owner's Dismiss, or a later withdrawal (replay), clears the item. A lost owner cleanup behaves the same way.
- No native observation in this slice.

### Integration items

- Registered tools 9 → 10 and bundle `brn-threads-guidance-12` (renumber at merge if another branch has taken 12). Docs and UI tests that count tools or pin the bundle need updating; `crates/brn-ai/README.md` is updated here.
- New activity label "Withdrawing an obsolete proposal…" and outcome labels `Withdrawn`/`Refused`.
- New core `Store::{clear_review_attention, dismiss_candidate}` and `run_attention::CleanupOutcome`; owner behavior is unchanged.
- Proposed sentence for `docs/architecture/threads-behavior.md` §142 (not edited here): "Every run registers ten tools: `read_records`, `read_note`, `read_source`, `read_skill`, `prepare_changes`, `apply_changes`, `retain_progress`, `read_history`, `refresh_candidate`, `withdraw_proposal`." Add after the refresh sentence: "`withdraw_proposal` dismisses only an unapplied candidate that a Run of the same chat prepared, through the owner's checked dismissal path with a receipt and replay; it never applies anything, refuses applied operations, owner changes and other chats, and is never offered to helpers."
- `docs/status.md` says "nine registered tools"; it needs "ten" at integration (not edited here).

## Slice H: a clear error for an unusable data folder

### Change

- **Core error (`crates/brn-threads-core/src/lib.rs`).** New `Error::DataFolder { folder, problem: DataFolderProblem, #[source] cause: io::Error }`, displayed as "BRN cannot use the data folder {folder}: {problem}". `DataFolderProblem` texts carry no OS wording:
  - `ReadOnly`: "it is on a read-only disk. Choose a folder you can write to."
  - `PermissionDenied`: "BRN does not have permission to use it. Choose a folder you own."
  - `NotAFolder`: "that path, or part of it, is a file rather than a folder."
  - `SymlinkRefused`: "part of its path is a symbolic link. Choose the folder's real location."
  - `MissingParent`: "a folder above it is missing or unavailable."

  The OS detail stays in `cause`, reachable through `std::error::Error::source()`. Neither host logs errors today, so that is the place kept for logs.
- **Classification (`store.rs` `data_folder_error`).** `ReadOnlyFilesystem` maps to `ReadOnly`, `PermissionDenied` to `PermissionDenied`, `NotADirectory` to `NotAFolder`, `AlreadyExists` to `NotAFolder` when the path is not a directory (how `create_dir_all` reports a file in the folder's place), and `NotFound` to `MissingParent`. Any other kind, including `AlreadyExists` on a real folder and a full disk, keeps `Error::Io`.
- **`Store::open`.** It now uses the classification for `has_symlink_ancestor`'s own I/O errors, `create_dir_all`, `read_dir` and the marker create/write. A symlink ancestor becomes `DataFolder { SymlinkRefused, cause: io::Error::other("symbolic link in path") }` instead of `UnsafeDirectory("symlink directory")`. The relative-path, non-empty-without-marker, foreign-marker and unexpected-entry refusals stay `UnsafeDirectory`.
- **Hosts.** The CLI and desktop show the error through their existing `to_string()` display and exit 1 as before (`AppError::Core` is `transparent`). No change to `crates/brn/src/main.rs` (0 lines), `HELP`, exit codes, desktop code or wording, `Cargo.lock` or the `threads.sqlite3` schema.

| `--data-dir` | Before (CLI stderr, exit 1) | After (CLI and `brn-desktop --headless-check startup`, exit 1) |
|---|---|---|
| `/data` (sealed macOS system volume) | `storage: Read-only file system (os error 30)` | `BRN cannot use the data folder /data: it is on a read-only disk. Choose a folder you can write to.` |
| `<case>/file` (a file) | `storage: File exists (os error 17)` | `BRN cannot use the data folder <case>/file: that path, or part of it, is a file rather than a folder.` |
| `<case>/file/data` (under a file) | `storage: Not a directory (os error 20)` | `BRN cannot use the data folder <case>/file/data: that path, or part of it, is a file rather than a folder.` |
| `<case>/locked/data` (parent mode 0500) | `storage: Permission denied (os error 13)` | `BRN cannot use the data folder <case>/locked/data: BRN does not have permission to use it. Choose a folder you own.` |
| `<case>/alias/data` (symlinked folder) | `incompatible or unsafe data directory: symlink directory` | `BRN cannot use the data folder <case>/alias/data: part of its path is a symbolic link. Choose the folder's real location.` |
| `/tmp/brn-dwr-r5-h` | (same symlink text) | `BRN cannot use the data folder /tmp/brn-dwr-r5-h: part of its path is a symbolic link. Choose the folder's real location.` |

`<case>` is `/private/tmp/brn-dwr-data/r5-h` (after) and `/private/tmp/brn-dwr-data/r5-h-before` (before), fresh synthetic folders. The "after" column is the recorded output of both `/tmp/brn-dwr-target/debug/brn --data-dir <p> notes` and `/tmp/brn-dwr-target/debug/brn-desktop --headless-check startup --data-dir <p>`, which printed identical text and exited 1. Nothing was created under `real/` or `locked/`.

### Tests and fail-before

Method: the implementation was written first. For the fail-before run, `Store::open` was put back to its old body (`?` into `Error::Io`, `UnsafeDirectory("symlink directory")`), and `data_folder_error` was made to return `Error::Io` for every absolute path. The new types stayed, so the tests compiled. The tests were run, then the fixed source was restored. A second mutation removed only the `NotFound` arm, to show the missing-parent case on its own. `store.rs` was then byte-identical to a copy of the fixed file. Lines are those of the committed files.

| Reason | Test | Before the fix | After |
|---|---|---|---|
| read-only | core `tests/data_folder.rs` `a_read_only_disk_is_named` (macOS: `/brn-dwr-readonly-<uuid>`, skipped if `/` is writable; it ran here) | FAILED `:17`, `storage: Read-only file system (os error 30)` | ok |
| read-only | CLI `tests/data_folder.rs` `a_data_folder_on_a_read_only_disk_is_named` (the evidence `/data` case) | FAILED `:20`, `storage: Read-only file system (os error 30)` | ok |
| permission denied | core `a_folder_without_permission_is_named` (parent mode 0500; skipped when privileged; it ran here) | FAILED `:17`, `storage: Permission denied (os error 13)` | ok |
| not a directory | core `a_file_in_place_of_the_folder_is_named` (the path is a file, and the path is under a file) | FAILED `:17`, `storage: File exists (os error 17)` | ok |
| not a directory | CLI `a_file_given_as_the_data_folder_is_named`: exit 1, empty stdout, stderr starts with the owner text and has no "os error" | FAILED `:20`, `storage: File exists (os error 17)` | ok |
| symlink refused | core `a_symlinked_path_is_refused_by_name` (the alias folder, a folder under it, and `/tmp/<uuid>` on macOS; nothing created) | FAILED `:17`, `incompatible or unsafe data directory: symlink directory` | ok |
| symlink refused | core `core.rs` `data_selection_refuses_symlink_ancestor_without_creating_database` (updated to expect `DataFolder { SymlinkRefused }`) | FAILED `:1103` `matches!(.., Err(Error::DataFolder { problem: SymlinkRefused, .. }))` | ok |
| all, plus missing parent | core unit `store::storage_open_tests::data_folder_failures_are_named_in_owner_terms`: raw codes 30, 13, 20, 17 (on a file) and 2 map to their reasons; the message starts with the folder and has no "os error"; `source()` is `Some`; 17 on a real folder and 28 (disk full) stay `Error::Io` | FAILED `:2510`, `storage: Read-only file system (os error 30)` (first mutation). FAILED `:2510`, `storage: No such file or directory (os error 2)` (`NotFound` arm removed) | ok |

Each core integration test checks the variant and problem, that `to_string()` contains the folder path and no "os error", and that `source()` is `Some`. No other core or app test matched the symlink refusal; the `chat_storage.rs` and `core.rs` tests about entries inside the folder stay `UnsafeDirectory` and pass unchanged.

### Commands and results (after the fix)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | clean |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | clean |
| `cargo test -p brn-threads-core --locked --offline` | 150 passed, 1 ignored (unit 42 (+1) and 1 ignored; `data_folder` 4, new; `core` 45) |
| `cargo build -p brn -p brn-desktop --locked --offline && cargo test -p brn --locked --offline --test data_folder` | 2 passed (new) |
| `cargo test -p brn --locked --offline --no-fail-fast` | all passed (`threads` 13, `source_locator` 2, `search_messages` 1, `data_folder` 2) |
| `cargo test -p brn-threads-app --locked --offline --no-fail-fast` | 191 passed, 1 ignored |
| manual CLI and headless desktop, table above | owner text, exit 1 |
| `TMPDIR=/private/tmp/brn-dwr-data/r5-h-ws cargo test --workspace --locked --offline --no-fail-fast` | 808 passed (+7), 1 failed (the baseline Poppler test), 6 ignored |
| `git diff --exit-code 771116f0 -- Cargo.lock`; `git diff --check` | clean |

### Limitations

- `MissingParent` has no deterministic filesystem repro, since `create_dir_all` creates missing parents. It covers a folder above that disappears during the open. Only the classification unit test shows it.
- An existing workspace on a read-only disk still reports the SQLite text (for example from the WAL pragma), because SQLite failures on an existing database are unchanged.
- Other I/O failures (a full disk, an `AlreadyExists` race on a real folder) keep the `storage: …` text.
- `Store::restore` and `Store::upgrade` keep their own folder errors; this slice changes only `Store::open`.
- No native observation in this slice (headless startup only).

### Integration items

- New `brn_threads_core::Error::DataFolder` variant and `DataFolderProblem` enum. work/cli-agent-contract's exhaustive `From<Error>` match in its `main.rs` must map it at merge, as `EditSessionOpen` needed. Suggested code `data_folder_unusable` with details `{folder, problem}`.
- The symlinked data-folder refusal changed from `UnsafeDirectory("symlink directory")` to `DataFolder { SymlinkRefused }`. Callers that matched the old variant for a symlinked data folder need the new one.
- `crates/brn/src/main.rs`: 0 lines changed in this slice. The desktop shows the error through its existing `to_string()` display, with no wording edit.
