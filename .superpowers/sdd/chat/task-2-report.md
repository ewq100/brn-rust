# Task 2 report — WorkStore V2 local conversations

Date: 2026-10-02. Status: **DONE**, implemented and offline-verified; separate
Opus review and user acceptance remain pending. Not merged or pushed.

## Checkout and commit

- Checkout: `/Users/evokessler/repos/brn-rust/.worktrees/task-4-ai-chat`
- Branch: `task-4-ai-chat`
- Base: `7eab5a18928c264fc194b3b8aababb169811352b`
- Implementation: `d0d03ca761074aebae941ec6f1237f66b2e6d2ff`
- This report is committed separately after that implementation commit.
- Both commits include the requested Copilot co-author trailer.
- Initial working tree was clean. Only the five implementation files below and
  this report are owned changes.

## Changes and files

| File | Change |
| --- | --- |
| `crates/brn-store/src/work/chat.rs` | Serializable DTOs, safe string validation, paired reads, transactional begin/finish, exact UUID replay and conflict handling, ordered complete local history, startup reconciliation. |
| `crates/brn-store/src/work/mod.rs` | Export DTOs, append the exact V2 conversations/messages schema and index, reconcile before creating the startup backup. |
| `crates/brn-store/src/lib.rs` | Add the required typed `Error::NotFound` and its ordinary Display handling. No legacy schema change. |
| `crates/brn-store/tests/work_chat.rs` | Twelve substantive synthetic integration tests covering lifecycle, replay, atomicity, validation, schema, upgrade, restored backups, all 21 local turns, and preservation of safety behavior. |
| `crates/brn-store/README.md` | Document the directly related WorkStore contract and focused checks. |

### Contract decisions and coverage

- V1 migration SQL is byte-for-byte unchanged; V2 is appended. Legacy Store V6,
  dependencies, lockfile, toolchain, backups implementation and owner-lock
  implementation are unchanged.
- Both user/assistant rows share operation UUID, increasing conversation
  sequence, provider, model, status and safe error category. Atomic selection
  storage here means the provider/model of both message rows commit together;
  application selection/settings behavior belongs to the workflow tasks.
- `None` allocates a new conversation. Unknown `Some` returns typed NotFound
  before any insert. Missing turns and missing conversation reads are also typed.
- Existing UUID is checked first inside the transaction. Exact
  question/provider/model and any supplied conversation must match; otherwise
  OperationConflict. `None` replay returns the existing conversation. Recorded
  Running results never authorize another external call.
- Finish accepts terminal statuses only. Identical terminal replay succeeds;
  a changed terminal status, answer or error fails. Question/answer strings are
  never trimmed or normalized. Tests cover CRLF, LF, Unicode, combining
  characters, leading/trailing whitespace and partial text.
- Finish updates both rows and first-question title in one transaction. Injected
  assistant-insert, assistant-update and conversation-title failures verify
  rollback, including no conversation left behind by failed begin.
- Providers are exactly `chatgpt`/`copilot`. Models use independently implemented
  1–128 byte ASCII identifier validation, without AI/Rig imports or provider
  model policy. Tests exercise empty, blank, unsafe, Unicode and 129-byte models
  plus the accepted 128-byte boundary and punctuation.
- All twelve safe error categories are tested; arbitrary categories, raw-shaped
  metadata, token-shaped errors and device-code categories are rejected. V2
  has only the exact requested text-pair columns, not tool/Rig/body/credential
  columns. Plain text is a safe caller boundary, not a secret-detection heuristic:
  arbitrary credential/provider objects must never be passed as chat text.
- Pair reads validate both rows, identity, sequence, role, matching metadata and
  safe strings. Malformed pairs fail explicitly rather than disappearing from
  history or being silently repaired.
- Open checks/migrates, validates/reconciles Running pairs to Interrupted, then
  makes its backup. Reconciliation uses the same transaction-level finalization
  helper as normal finish, preserving already durable assistant text and error
  category. There is no per-token persistence promise.
- Restore of V1 backups upgrades while preserving settings, edit hash, text and
  timestamp. Restore of a V2 Running backup preserves durable partial Unicode/
  CRLF text and captures the Interrupted result in the new startup backup.
- All 21 local pairs survive restart, including the oldest; sequence remains
  1 through 21 and the first question remains the title. No storage truncation.
- V2 numerical/schema checks cover two rows per pair, application ID, schema
  version 2, exact column lists, named index, foreign-key cascade, role/provider/
  status constraints, primary-key/sequence uniqueness and five-backup pruning.
  Schema version 3 is refused unchanged; existing work tests also cover damaged
  foreign/newer databases, integrity restore, fallback backups and owner lock.

## TDD evidence

All cargo commands below ran in the checkout above, using the pinned Rust
1.98.1 toolchain and unchanged lockfile. Logs were captured inside
`.superpowers/sdd/chat`, not in external scratch directories. No forced TMPDIR,
provider calls, downloads, credential inspection or real vault access occurred.
New fixtures use `tempfile::tempdir_in(canonicalize("."))`, explicitly disposable
canonical project paths. Existing `work.rs` fixtures and assertions are unchanged.

### Initial RED — tests existed before production changes

Exact test invocation:

```sh
cargo test -p brn-store --test work_chat --locked
```

Exit 101. The initial API-level RED could not compile because the new DTO,
methods and NotFound variant did not exist. Representative exact output:

```text
error[E0432]: unresolved import `brn_store::work::WorkTurnStatus`
 --> crates/brn-store/tests/work_chat.rs:3:23
  |
3 |     work::{WorkStore, WorkTurnStatus},
  |                       ^^^^^^^^^^^^^^ no `WorkTurnStatus` in `work`

Some errors have detailed explanations: E0432, E0599.
For more information about an error, try `rustc --explain E0432`.
error: could not compile `brn-store` (test "work_chat") due to 41 previous errors
```

The tests were substantive behavior tests, not placeholder assertions.
After adding the connection-based lifecycle implementation but before V2,
the **same command** exited 101 with ten behavior failures. Exact schema
assertion and aggregate output:

```text
---- v2_has_only_text_pair_schema_and_constraints stdout ----

thread 'v2_has_only_text_pair_schema_and_constraints' (126223400) panicked at crates/brn-store/tests/work_chat.rs:236:5:
assertion `left == right` failed
  left: []
 right: ["turn_id", "conversation_id", "sequence", "role", "text", "provider", "model", "status", "error_code"]

test result: FAILED. 1 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s

error: test failed, to rerun pass `-p brn-store --test work_chat`
```

Other schema-stage failures were missing `messages`/`conversations` tables and
the consequent failed typed-error assertions. There were no test-only production
states or stub methods.

### Substantive restart RED — after migration, before reconciliation

Exact command was again:

```sh
cargo test -p brn-store --test work_chat --locked
```

Exit 101. Complete output:

```text
   Compiling brn-store v0.1.0 (/Users/evokessler/repos/brn-rust/.worktrees/task-4-ai-chat/crates/brn-store)
    Finished `test` profile [unoptimized] target(s) in 2.40s
     Running tests/work_chat.rs (target/debug/deps/work_chat-b84f41382d4299a1)

running 11 tests
test missing_conversation_and_turn_are_typed_and_insert_nothing ... ok
test exact_uuid_replay_returns_running_and_terminal_without_new_rows ... ok
test injected_second_row_failure_rolls_back_begin_and_finish ... ok
test v2_has_only_text_pair_schema_and_constraints ... ok
test terminal_pair_preserves_exact_text_and_selection ... ok
test restart_reconciles_running_before_backup_and_never_repeats ... FAILED
test malformed_pairs_are_rejected_not_silently_reconciled ... FAILED
test v1_upgrade_and_restored_v1_backup_preserve_work ... ok
test validation_rejects_unsafe_selection_and_error_codes_without_changes ... ok
test newer_schema_is_untouched_and_backups_keep_five ... ok
test twenty_one_local_turns_and_partial_terminal_text_survive_restart ... ok

failures:

---- restart_reconciles_running_before_backup_and_never_repeats stdout ----

thread 'restart_reconciles_running_before_backup_and_never_repeats' (126224898) panicked at crates/brn-store/tests/work_chat.rs:110:5:
assertion `left == right` failed
  left: Running
 right: Interrupted
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- malformed_pairs_are_rejected_not_silently_reconciled stdout ----

thread 'malformed_pairs_are_rejected_not_silently_reconciled' (126224895) panicked at crates/brn-store/tests/work_chat.rs:269:9:
assertion failed: matches!(WorkStore::open(dir.path()), Err(Error::Invalid(_)))


failures:
    malformed_pairs_are_rejected_not_silently_reconciled
    restart_reconciles_running_before_backup_and_never_repeats

test result: FAILED. 9 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.66s

error: test failed, to rerun pass `-p brn-store --test work_chat`
```

Reconciliation made the original eleven tests green. The final coverage adds a
restored-Running-backup test and strengthens schema, sequence and title-rollback
assertions. Transaction-level finalization was then shared with reconciliation.

## Final GREEN evidence

Exact invocation:

```sh
cargo test -p brn-store --test work --test work_chat --locked
```

Exit 0. Complete final output:

```text
   Compiling brn-store v0.1.0 (/Users/evokessler/repos/brn-rust/.worktrees/task-4-ai-chat/crates/brn-store)
    Finished `test` profile [unoptimized] target(s) in 2.17s
     Running tests/work.rs (target/debug/deps/work-ce5627dee47e4246)

running 18 tests
test missing_data_dir_is_rejected ... ok
test foreign_database_is_refused ... ok
test unbranded_database_is_refused_unchanged ... ok
test damaged_foreign_database_is_refused_unchanged ... ok
test empty_database_without_backups_starts_fresh ... ok
test corrupt_database_without_backups_starts_fresh ... ok
test newer_schema_is_refused_and_left_in_place ... ok
test damaged_newer_database_is_refused_unchanged ... ok
test new_backup_sorts_after_existing_ones_even_with_future_names ... ok
test settings_survive_reopen ... ok
test unsaved_edits_round_trip_replace_and_clear ... ok
test unsaved_edit_limits ... ok
test missing_database_is_restored_from_newest_backup ... ok
test corrupt_database_is_restored_from_newest_backup ... ok
test each_open_backs_up_and_keeps_five ... ok
test empty_database_is_restored_from_newest_backup ... ok
test unusable_newest_backup_falls_back_to_older_one ... ok
test second_owner_is_rejected ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.12s

     Running tests/work_chat.rs (target/debug/deps/work_chat-b84f41382d4299a1)

running 12 tests
test missing_conversation_and_turn_are_typed_and_insert_nothing ... ok
test v2_has_only_text_pair_schema_and_constraints ... ok
test exact_uuid_replay_returns_running_and_terminal_without_new_rows ... ok
test injected_second_row_failure_rolls_back_begin_and_finish ... ok
test terminal_pair_preserves_exact_text_and_selection ... ok
test restart_reconciles_running_before_backup_and_never_repeats ... ok
test restored_running_backup_retains_only_durable_partial_text ... ok
test v1_upgrade_and_restored_v1_backup_preserve_work ... ok
test validation_rejects_unsafe_selection_and_error_codes_without_changes ... ok
test newer_schema_is_untouched_and_backups_keep_five ... ok
test twenty_one_local_turns_and_partial_terminal_text_survive_restart ... ok
test malformed_pairs_are_rejected_not_silently_reconciled ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s
```

Exact format and strict lint commands:

```sh
cargo fmt --all -- --check
cargo clippy -p brn-store --all-targets --locked -- -D warnings
```

Both exit 0; format produced no output. Complete final Clippy output:

```text
    Blocking waiting for file lock on build directory
    Checking brn-store v0.1.0 (/Users/evokessler/repos/brn-rust/.worktrees/task-4-ai-chat/crates/brn-store)
    Finished `dev` profile [unoptimized] target(s) in 2.25s
```

`git diff --check` and the staged equivalent both exited 0. An independent
Python comparison extracted the original and new first migration SQL strings
from base/current source and asserted exact equality:

```text
V1 migration SQL byte-for-byte preserved; diff whitespace check passed
```

## Self-review, limitations and handoff

- Reviewed schema against the exact brief, migration ordering, transaction
  rollback/replay branches, all four statuses, safe string allowlists, title
  sequencing and backup ordering. Invalid/SQL errors propagate; no broad catches
  or silent recovery were introduced.
- New NotFound was necessary because the baseline store error enum lacked it.
  Existing workflow conversion uses a wildcard for other store errors, so this
  does not introduce an exhaustive-match break in existing workspace consumers.
- Reconciliation and user finish share one finalization implementation over a
  transaction on the existing connection. Future Task 5 can reuse the current
  connection-based begin/finish helpers; no additional owner/attached-connection
  lifetime machinery has been implemented prematurely.
- No workers, AI integrations, outbound-history policy, broader plan tasks,
  account actions, old-data migration or unrelated documentation were added.
- No known implementation blocker. Startup pair validation and complete history
  reads scale with local turn count; there is intentionally no truncation or
  summarization. This bounded task does not qualify native UI or live providers.
- The initial API RED was a compilation failure, transparently recorded above;
  later schema and restart RED runs demonstrate actual behavior failures.
- Verification is focused: 30 targeted integration tests, store all-targets
  strict Clippy, formatting and migration/diff checks. No whole-workspace suite
  was repeated and no live/native usability claim is made.
- Next action: separate Opus review of the implementation commit against
  `task-2-brief.md` and the shared WorkStore/Global Constraints sections.
  User acceptance, merge and release remain pending.
