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
