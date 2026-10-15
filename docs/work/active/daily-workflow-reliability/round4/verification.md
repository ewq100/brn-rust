# Round 4 verification: a killed Run's notice and candidate stay reachable

**Branch:** `work/daily-workflow-reliability`, worktree `/Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability`, on base `771116f0`. Implementation commit `3919c09a` (local WIP; the finalize step makes the final commit). The backup ref `work/daily-workflow-reliability-pre-rebase` is kept. Nothing was pushed or merged. [Plan](plan.md).

**Owner request:** "1. i did not see Resume button  2. i dont know. also why cant you just run this test yourself?"

**Status:** implemented and verified (automated, including the headless desktop state harness). Not native-observed, not owner-accepted, not merged.

## The owner's two points

**1. No Resume button (finding, no code change).** The Resume pill (`queue-resume`, `crates/brn-desktop/src/threads_chat.rs` `composer_controls`) is drawn only while the saved queue holds a pending or waiting item. It is enabled only when that queue is paused and no worker runs. In `native-b1-kill` the queue was empty, so there was nothing to resume. The owner's first-run delegation choice had already cancelled the orphan, and the next message dispatched at once. Since Fix 2, Dispatch also recovers a lease-free orphan by itself. Resume appears only for a paused queue: after a restart with saved items, after a Stop, or after a failure. The `resume` store below produces exactly that state, and the headless test `resume_after_a_kill_recovers_the_intake_and_keeps_its_candidate_reviewable` finds `queue-resume` in the rendered window.

**2. Why the agent cannot run the native check itself.** In this session the agent shell still cannot use the Mac's process and sandbox services: `sandbox-exec -p '(version 1)(allow default)' /usr/bin/true` fails with "sandbox_apply: Operation not permitted" (exit 71), and `ps` fails with "operation not permitted". The orchestrator reports that its own shell has the sandbox off; this step's shell does not. The assignment also forbids this step from driving the Mac UI or calling a live provider. The automated tests therefore cover everything except on-screen rendering:
- `killed_run_candidate.rs` reproduces the native sequence with the real runtime tools and a scripted model.
- Two headless desktop tests run the same `ThreadsUi` state and render code without a screen, including Home's Needs you row, the chat's attention cards with the proposal card, and the Resume pill.
- Two offline stores (below) make the native check deterministic, with no live provider and no timing.

## What changed

| Gap | Change | Files |
|---|---|---|
| Shared rules | New core module `brn_threads_core::run_attention`: `INTERRUPTED_ATTENTION`, `is_terminal_notice`, `replace_terminal_notice` (moved verbatim from `brn-ai`, which re-exports them), `REVIEW_ATTENTION` ("Review prepared changes", the existing reason), `add_review_attention`, `ended_run_attention` and `Store::active_candidates`. No new user-facing text. | `crates/brn-threads-core/src/run_attention.rs`, `lib.rs`; `crates/brn-ai/src/threads.rs` |
| G1 | Every checked change that ends a Working Run outside `finish()` calls `ended_run_attention` inside that same change. Where a notice is right (table below), it adds the Interrupted Blocker through `replace_terminal_notice`, so a chat keeps one terminal notice. `Cancelled` now counts as superseded wherever notices are replaced. | `store.rs` (settings), `chat.rs`, `steering.rs`, `projects.rs` (core); `chat.rs`, `intake.rs` (app); `begin` and `finish` in `brn-ai` |
| G2 | The same helper first adds the standard Review item (`Review`, "Review prepared changes", `record` = operation) for each candidate the Run prepared that is still prepared, unretired and without a receipt, unless an attention item already names that operation. In-process Interrupted and Failed results in `finish()` do the same (⌘Q, cancellation, provider failure). No candidate is applied, dismissed or retired. | same |
| G3 | `run_lease::remove_free` deletes a lease file only while holding its lock; a held lease is never touched. Orphan recovery calls it only after its change is `Applied`. | `crates/brn-ai/src/run_lease.rs`, `crates/brn-threads-app/src/chat.rs` |

No schema, dependency or desktop production change. The desktop change is tests only, in `crates/brn-desktop/src/threads_intake_tests.rs`.

## Stop paths and decisions

Which paths stop Runs is unchanged. Each path's helper call sits in the same checked change that ends the Run, so a receipt replay returns the recorded receipt and adds nothing. A Thread write is added only when the attention actually changes, so receipts on paths with nothing to add are byte-identical to before.

| Path | Ends Run as | Notice | Review | Why | Test |
|---|---|---|---|---|---|
| Workspace settings: Configure, delegation Choose, Pause/Resume maintenance, BackupDirectory (`update_workspace_settings`) | `Cancelled`, every Working Run | **added** | added | The owner acted elsewhere; the chat had no trace of the stop. | T1, T1-live, T3, C1 |
| Orphan recovery (Resume, Dispatch) | `Interrupted`, lease free only | kept (now via the helper) | added | Process died. | T2, R1, T8b |
| Stop (`stop_chat_run`: desktop Stop, `pause_failed_worker`, resume-stale stop) | `Interrupted` | **none** | added | Deliberate owner Stop in this chat. Existing behaviour: the fenced worker's `finish()` writes nothing, and the chat already shows the Interrupted status with "Continue with current context" (`threads_writing.rs`). A notice would repeat what the owner just did. | T4 |
| Archive / ArchiveProgress | `Interrupted`, every Working Run in the chat | **none** | added, into the archived Thread write | Deliberate owner action. `needs_you`/`thread_rows` include archived chats, so a notice would raise a just-archived chat with a Continue that is refused until Unarchive. The Review is still added so the candidate stays reachable and its note's candidate slot is visible. | T4 |
| Steer (`stop_and_post(_with_partial)`) | `Interrupted` | **none** | added | The owner's new instruction follows at once; its Run's result replaces notices. | T4 |
| Project commands that fence affected chats (Configure, MoveChat, Link/Unlink/CreateDocument, CreateChat) | `Interrupted` | **added** | added | The owner acted on the project, not in the chat. | T5 |
| Project Delete (chats to Trash), then Restore | `Interrupted` + archived | **added**, edited into the trashed Thread write | added | Same; the Trash `deleted_version` still matches, and Restore brings both back. | T5 |
| Full-note import into a project (`import_full_scoped`) | `Interrupted` | **added** | added | Same as project changes. | T5 |
| Continue / new Run in the same chat (`begin()`) | `Interrupted` | **none** | added, in the "Start a fresh invocation" change | A new Run follows at once. | T6 |
| In-process terminal in `finish()` (⌘Q, cancel, provider failure) | `Interrupted` / `Failed` | existing | **added** | Same gap between prepare and apply. A Completed Run is unchanged (it chose not to apply). | T7 |
| Legacy `Workspace::resolve_thread(_because)` | `Cancelled`, clears all attention | unchanged | unchanged | No production caller; the desktop's resolve is Archive. | none |
| Theme / TopicsNote settings | none (Deferred) | n/a | n/a | | none |

**Recorded, not changed:** the settings path also stops Runs that are live in another process. `update_workspace_settings` cancels every Working Run without consulting `derived/run-leases/`. The other process's watcher sees the fence and stops, and its fenced `finish()` writes nothing. From this round the settings change itself leaves that chat one Interrupted notice. `the_settings_change_also_stops_a_run_that_is_still_live` shows the Run `Cancelled` at fence 2 with the notice while `run_is_live` is still true.

**Changed core test expectation (justified contract correction):** `crates/brn-threads-core/tests/delegation.rs::both_choices_and_pause_fence_every_working_run_and_block_stale_connection` now expects `receipt.writes.len() == 5` (was 3): Settings, two Runs, and their two chats, each chat's attention exactly `[Blocker(run, INTERRUPTED_ATTENTION)]`. Run C's chat gets the same after `pause_workspace`.

## Tests: fail-before and pass-after

Each fix was disabled in place, the named test was run and failed as listed, then the source was restored. After the sweep `git diff` against `3919c09a` was empty, and every test passes in the suite runs below. Batches disabled fixes that the other tests in the batch do not use.

| ID | Test (file) | Fix disabled | Failed assertion before the fix |
|---|---|---|---|
| U1 | 6 `run_attention::tests` (core) | n/a (pure helper; each case asserts its own rule) | n/a |
| C1 | `both_choices_and_pause_fence_every_working_run_and_block_stale_connection` (`brn-threads-core/tests/delegation.rs`) | settings helper call | `delegation.rs:361` `left: 3, right: 5` (receipt writes) |
| T1 | `the_delegation_choice_after_a_kill_leaves_one_notice_and_a_review_for_the_candidate` (`brn-threads-app/tests/killed_run_candidate.rs`) | settings helper call | `:284` chat attention `left: []`, `right: [Review(op) "Review prepared changes", Blocker(run) "Run interrupted. Continue starts fresh."]` |
| T1-live | `the_settings_change_also_stops_a_run_that_is_still_live` | settings helper call | `:331` `left: []`, `right: [Blocker(run) INTERRUPTED]` |
| T2 | `resume_or_dispatch_after_a_kill_adds_the_review_and_removes_the_free_lease_file` | orphan recovery `active` closure returning `Ok(vec![])` | `:359` "resume": `left: [Blocker(run)]`, `right: [Review(op), Blocker(run)]` |
| T2 | same | `remove_free` call removed | `:365` `!lease_file(..).exists()` failed ("resume") |
| R1 | `resume_after_a_crash_interrupts_the_orphan_once_and_dispatches_the_queue_once` (`run_recovery.rs`) | `remove_free` call removed | `run_recovery.rs:159` `assertion failed: !lease_file(&workspace, &orphan).exists()` |
| U2 | `a_free_leftover_lease_is_removed_and_a_held_one_is_kept` (`brn-ai/src/run_lease.rs`) | `remove_free` returns `Ok(false)` without removing | `run_lease.rs:184` `assertion failed: remove_free(&derived, "abandoned:0").unwrap()` |
| T3 | `a_later_run_replaces_the_notice_of_a_run_the_settings_change_cancelled` | `RunState::Cancelled` in `finish()`'s superseded match | `:411` `left: [Review(op), Blocker(cancelled run)]`, `right: [Review(op)]` |
| T4 | `stop_archive_and_steer_keep_the_candidate_in_review_without_a_notice` | `ended.push` in core `chat.rs` Pause arm | `:436` "stop": `left: []`, `right: [Review(op)]` |
| T4 | same | `ended.push` in core `chat.rs` Archive arm | `:436` "archive" (stop passed): `left: []`, `right: [Review(op)]` |
| T4 | same | `ended.push` in core `steering.rs` | `:436` "steer" (stop, archive passed): `left: []`, `right: [Review(op)]` |
| T5 | `project_changes_and_project_imports_leave_the_notice_and_the_review` | helper call in `project_command` | `:542` "configure": `left: []`, `right: [Review(op), Blocker(run)]` |
| T5 | same | helper call in `import_full_scoped` | `:542` "import" (configure passed): `left: []`, `right: [Review(op), Blocker(run)]` |
| T6 | `continue_in_the_killed_chat_keeps_the_candidate_in_review` | helper call in `begin()` | `:556` `left: []`, `right: [Review(op)]` |
| T7 | `an_interrupted_or_failed_run_keeps_its_prepared_candidate_in_review` (`brn-ai/src/threads.rs`) | `add_review_attention` call in `finish()` | `threads.rs:4313` "interrupted": `left: [Blocker(run)]`, `right: [Review(op), Blocker(run)]` (cases: interrupted, failed `InvalidToolUse`, completed unchanged, protected NeedsReview keeps one Review) |
| T8a | `a_killed_intake_candidate_reaches_needs_you_after_the_delegation_choice` (`brn-desktop/src/threads_intake_tests.rs`) | settings helper call | `threads_intake_tests.rs:410` row reasons `left: []`, `right: ["Review prepared changes", "Run interrupted. Continue starts fresh."]` |
| T8b | `resume_after_a_kill_recovers_the_intake_and_keeps_its_candidate_reviewable` | orphan recovery `active` closure | `threads_intake_tests.rs:497` `left: ["Run interrupted. Continue starts fresh."]`, `right: ["Review prepared changes", "Run interrupted. Continue starts fresh."]` |

With the settings helper disabled, T3's precondition (`:398`) also failed; T3's own fix was checked separately as listed.

**T1 native sequence:** `prepare_run` takes the lease and saves a Working Run; the real `prepare_changes` tool, driven by the scripted model, prepares "Crane arrival — reported date" (`{run}:proposal:crane-arrival-source-capture-1`); `run_prepared_scripted_then_abandon` returns without `finish()`; the test recreates the unlocked 0-byte lease file a SIGKILL leaves. Pre-state matches `native-b1-kill`: Run Working at fence 1, chat attention `[]`, candidate active with no receipt, lease file present and not live. Then `Workspace::choose_delegation` (the desktop's call) leaves Run `Cancelled` at fence 2, attention exactly `[Review(op), Blocker(run)]`, the candidate active and the chat in `needs_you()`. A second choice and a replay of the first receipt's operation through `store.choose_delegation` add no attention, and the replay adds no receipt.

**T2:** for both Resume and Dispatch, in fresh worlds: Run `Interrupted` at fence 2, attention `[Review(op), Blocker(run)]`, the candidate active, the lease file removed, a second `recover_orphaned_runs` returns `[]` with no new receipt, and the queued question dispatches once. `review_apply(op)` is then Applied, its Review is removed and the notice stays.

## Desktop visibility (headless)

No desktop production change was needed, and there is no UI finding. Once the attention exists, the existing code shows it:
- **Before the choice:** the intake chat's `ThreadRow` has no reasons and is not in `lists::sidebar(..).needs`, while `ui.candidates` lists the operation. That is what the owner saw: the sidebar "Review · …" row only. Home is rendered and has no `home-attention-{chat}`.
- **After `ui.choose_delegation(MaintainOrdinaryNotes)`:** the row's reasons are `["Review prepared changes", "Run interrupted. Continue starts fresh."]`, it is in the sidebar's NEEDS YOU, and Home renders `home-attention-{chat}`. After `open_thread`, the window has `attention-card-0` and `attention-card-1`, and `attention-card-0` contains `proposal-{operation}`, the exact candidate's proposal card with Apply and Dismiss.
- **Resume after a kill:** with a paused saved queue and no worker, the window has `queue-resume`. `ui.resume_chat_queue` recovers the orphan (Interrupted, fence 2), the intake row shows the same two reasons, the candidate stays listed, the lease file is gone, and the queued question ran once in "Questions" and failed offline (one `Failed` Run, one owner Message).

## Repository verification

All from the worktree root with `PATH=/opt/homebrew/opt/rustup/bin:$PATH CARGO_TARGET_DIR=/tmp/brn-dwr-target`. Logs are kept outside Git in `/private/tmp/brn-dwr-data/r4-logs/`.

| Command | Result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 0 (rechecked after touching `brn-threads-core`: core, brn-ai, app, brn, desktop checked) |
| `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` | exit 0 |
| `cargo test --workspace --locked --offline --no-fail-fast` | **721 passed, 12 failed, 5 ignored** (baseline 706 / 12 / 4; +15 new tests: core 6, brn-ai 2, app 7; +1 ignored store generator) |
| `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | **152 passed, 0 failed, 1 ignored** (baseline 150 / 1) |
| `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | exit 0 |
| `TMPDIR=/private/tmp/brn-dwr-data/r4-e2e scripts/verify-end-to-end.sh --fixtures-only` | exit 0: provider retirement, intake retirement and offline Save/recovery/search/export/backup/startup/refusal journeys passed |
| `cargo build -p brn --locked --offline`; `python3 docs/work/active/daily-workflow-reliability/cli-journey.py /tmp/brn-dwr-target/debug/brn /private/tmp/brn-dwr-data/r4-cli-journey` | exit 0. Unchanged from round 3: `needs_you` 2, `useful_intake_new_threads` 1, both edit-session refusals are the `edit_session_open` JSON with `base_version: 1`, `restore_exit` 0, `restored_threads_equal` true, `restore_into_existing_refused` true. No offline run prepares a candidate, as expected. |
| `CARGO_TARGET_DIR=/tmp/brn-dwr-target TMPDIR=/private/tmp/brn-dwr-data/r4-storage scripts/verify-storage.sh` | exit 101, stopping at its `cargo test --workspace` step on `brn-threads-intake --lib` (3 `process::tests` failures). Same sandbox cause as below. |
| `git diff --check` | clean |
| `python3 scripts/check-markdown-links.py` | 86 files, 396 local links, 0 failures (including this note and the `evidence.md` update) |

**The 12 workspace failures** are the known `brn-threads-intake` sandbox tests: `process::tests` (3: `custom_homebrew_runtime_keeps_external_source_and_network_denied`, `renderer_uses_only_owned_font_config_and_keeps_source_and_network_denied`, `sandbox_denies_external_source_and_network_before_parsing`, each `Err(Unavailable)`) and `tests/qualification.rs` (9, for example `markdown_text_and_email_preserve_meaning_without_source_payloads`, `Err(Invalid)`). No other test failed. The orchestrator asked to retry these now that the owner turned the sandbox off: in this step's shell `sandbox-exec` is still denied ("sandbox_apply: Operation not permitted"), so they still fail here. That is an environment limit, not a result of this change; they need a rerun from a shell where `sandbox-exec` works.

The fresh `r4-e2e`, `r4-storage` and `r4-cli-journey` folders were deleted after the runs.

## Native re-check procedure (owner or orchestrator)

Offline, so no provider is needed. Make sure no agent is driving the Mac at the same time. Two stores were generated by this build and are **left in place** for this check: `/private/tmp/brn-dwr-data/r4-native-settings/data` and `/private/tmp/brn-dwr-data/r4-native-resume/data`. A read-only copy of each showed the killed state: Run `Working` at fence 1, intake chat attention empty, candidate "Crane arrival — reported date" prepared, unretired and without a receipt, and an unlocked 0-byte lease file in `derived/run-leases/`. `brn-desktop --headless-check startup` on a separate copy printed "Threads ready: 6 records" (settings) and "Threads ready: 7 records" (resume); those copies were deleted.

To regenerate a store, pass a folder that does not exist yet:

```sh
cd /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability
export PATH=/opt/homebrew/opt/rustup/bin:$PATH CARGO_TARGET_DIR=/tmp/brn-dwr-target
BRN_DWR_NATIVE_DATA=/private/tmp/brn-dwr-data/r4-native-settings-2/data BRN_DWR_NATIVE_MODE=settings \
  cargo test -p brn-threads-app --locked --offline --test killed_run_candidate -- --ignored native_check_store --nocapture
```

Build and launch (the last `brn-desktop` build in this target folder was made without `native-ui`, so rebuild):

```sh
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo build -p brn-desktop --features native-ui --locked --offline
S=/private/tmp/brn-dwr-data/r4-native-settings   # or r4-native-resume
mkdir -p "$S/home" "$S/codex"
env HOME="$S/home" CODEX_HOME="$S/codex" BRN_INTAKE_HELPER=/tmp/brn-dwr-target/debug/brn-intake-helper \
  /tmp/brn-dwr-target/debug/brn-desktop --data-dir "$S/data"
```

After ⌘Q, inspect with `brn_inspect "$S/data"` from the [owner manual check](../evidence.md#owner-manual-check).

1. **`settings` store (the Review item reaches Needs you).** Delegation is pending, so Home shows the two maintenance choices. Pick one.
   - *Expected on screen:* Home's Needs you lists "Information from b1-source" with "Review prepared changes" and "Run interrupted. Continue starts fresh.". The sidebar NEEDS YOU shows that chat and a "Review · …" row. Opening the chat shows two attention cards; the first carries the proposal "Crane arrival — reported date" with Apply and Dismiss.
   - *Expected in the store:* the Run is `Cancelled` at fence 2; the chat has one Blocker and one Review row; the lease file is still present (recorded limitation).
2. **`resume` store (the Resume button).** Delegation is already chosen, and "When can the crane arrive?" waits in the queue, paused as after a restart. Open "Questions".
   - *Expected on screen:* the composer shows "Queue · 1 waiting · paused" with Resume enabled. Click Resume. The intake chat then shows the same two Needs you reasons. The queued message runs once and fails offline; "Questions" gets "Run failed: The provider account needs reconnecting. Continue when the provider is ready." and the queue pauses again.
   - *Expected in the store:* the intake Run is `Interrupted` at fence 2, and `derived/run-leases/` no longer holds its file.

## Evidence classes

- **Automated, executed:** everything in the tables above.
- **Headless desktop state and render (gpui test harness, no screen):** T8a and T8b, plus `--headless-check startup` on copies of both stores.
- **Native observation:** none in this step. The procedure above is for the owner or the orchestrator.
- **Unverified:** on-screen appearance of the two Needs you reasons and the proposal card on the UI branch's layout; the two-process case for settings (only an in-process held lease was used); the 12 sandbox tests in a shell where `sandbox-exec` works.

## Limitations (not fixed in round 4)

- **Settings and leases.** The settings path stops Runs that are live in another process (now with a notice), and it leaves an orphan's lease file in place. Only orphan recovery removes it.
- **Continue does not clean the lease.** `begin()` fences Working Runs in its chat whether or not they are live, so it does not remove their lease files.
- **Completed Runs.** A Completed Run that prepared but never applied a candidate still gets no Review item. The candidate stays in the candidates list and Home's "Review changes".
- **Stop from a failure.** `pause_failed_worker` and the resume-stale stop share `stop_chat_run` with the deliberate Stop, so they also add no notice; the Review is added. Telling them apart needs a desktop change.
- **Stale candidates.** A Review is added for any still-active candidate, including one whose inputs changed since (for example after a project Configure). Applying it then returns the existing Stale outcome. This matches how earlier Review items behave.
- **Legacy Resolve.** `Workspace::resolve_thread` still clears every attention reason (no production caller).

## Proposed contract sentences (integration, not applied)

For `docs/architecture/threads-behavior.md`:
> When a Working Run ends without its own terminal result (process ended, workspace settings, project changes or imports, Archive, Stop, Steer or Continue), the same checked change adds a "Review prepared changes" item for each candidate that Run prepared and left unapplied. When the stop came from outside the chat (process ended, settings, project changes or imports), it also leaves the chat's one "Run interrupted. Continue starts fresh." notice. Stop, Archive, Steer and Continue add no notice.

For `docs/architecture/invariants.md`:
> Ending a Run never applies, dismisses or retires its candidates; a still-active candidate it prepared stays reachable from its chat's attention. A Cancelled Run's terminal notice is superseded like an Interrupted or Failed one, so a chat keeps one terminal notice.

## Integration items for the UI branch

Shared seams touched this round, which the UI branch must rerun at integration: `begin`, `finish` and `attention` in `crates/brn-ai/src/threads.rs`; `Workspace::recover_orphaned_runs`; core `chat.rs`, `steering.rs`, `projects.rs` and the settings path in `store.rs`; and the `brn_ai::threads` re-exports of `INTERRUPTED_ATTENTION`, `is_terminal_notice` and `replace_terminal_notice` (unchanged names). Tests that counted settings-change writes or expected empty attention after a settings change, Archive or Stop with a prepared candidate need the new expectations.
