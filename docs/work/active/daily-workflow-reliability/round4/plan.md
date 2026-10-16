# Round 4 plan: a killed Run's notice and candidate stay reachable

**Planned after:** the owner's 15 October 2026 native checks. **Branch:** `work/daily-workflow-reliability` at `0efb6bef` (base `771116f0`), worktree `/Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability`. Keep the backup ref `work/daily-workflow-reliability-pre-rebase`. Local only: never push.

**Owner request (authoritative):** "1. i did not see Resume button  2. i dont know. also why cant you just run this test yourself?" This plan fixes the store gaps behind the [force-kill result](../evidence.md#owner-native-check-results-15-october-2026). It also adds an offline, deterministic store the owner can open natively. That store shows the Resume button and the Review item in Needs you.

## What the code does today

- **Review attention comes from `apply_changes`, not `finish()`.** `prepare()` (`crates/brn-ai/src/threads.rs` ~1015) records the operation in `Run.operations` and prepares the candidate (`candidate_targets` row). Only `apply()` (~1075) calls `attention()` (~1210), which adds `Review "Review prepared changes"` with `record` = the operation, and it does so only when the outcome is NeedsReview. The killed Run prepared operation 7 but never reached `apply_changes`. So a Run that stops between prepare and apply leaves an active candidate with no attention, however it stops. That includes the in-process ⌘Q path, where `finish()` with Interrupted adds only the Interrupted notice.
- **The settings path is in the core and writes no attention.** `Store::update_workspace_settings` (`crates/brn-threads-core/src/store.rs` ~710–940; Configure, Choose, Pause, BackupDirectory) sets every Working Run to `Cancelled` with fence +1 in one checked change, "Owner configured workspace and stopped prior invocations". It does not check leases (recorded below). Theme and TopicsNote defer instead.
- **The notice helpers are in `brn-ai`, which the core cannot depend on.** `INTERRUPTED_ATTENTION`, `is_terminal_notice` and `replace_terminal_notice` live in `crates/brn-ai/src/threads.rs` ~2276–2365. `brn-threads-core` has no BRN dependencies, so the core's checked changes cannot add the notice today.
- **`Cancelled` is not superseded.** The superseded predicates in `finish()` (~2425) and `recover_orphaned_runs` (`crates/brn-threads-app/src/chat.rs` ~143) match only `Interrupted | Failed`. A notice left on a `Cancelled` Run would never be replaced, which would break "one terminal notice per chat".
- **The desktop already lists the candidate, but not in Home's Needs you.** `ThreadsUi::refresh` loads `workspace.candidates()`, and the Threads sidebar shows "Review · …" rows under NEEDS YOU (`threads_tiles.rs` ~249). Home's Needs you panel (`threads_tiles.rs` ~595) shows only chats with attention. The chat view shows a proposal card only for an attention item whose `record` names the candidate (`threads_writing.rs` ~222). No desktop production change is needed once the attention exists. Item 9 checks this headlessly.

### Why the owner saw no Resume button (finding, no code change)

The Resume pill (`queue-resume`, `threads_chat.rs` ~355) is rendered only when the saved queue has a pending or waiting item. It is enabled only when that queue is paused and no worker runs. In `native-b1-kill` the queue was empty. The owner's delegation choice had already cancelled the orphan (operation 9), so the next message dispatched at once and never paused. There was nothing to resume. Since Fix 2, Dispatch also recovers a lease-free orphan by itself. Resume appears only for a paused queue: after a restart with saved items, a Stop, or a failure. Item 8's `resume` store produces exactly that state.

### Why the agent cannot run the native check itself

On 15 October the agent shell was denied process listing, screen capture and UI control ([attempt](../evidence.md#native-observation-attempt-task-b-not-observed)). The headless desktop test (item 9) runs the same `ThreadsUi` state code without a screen. Only on-screen rendering stays with the owner. Item 8 makes that check offline and deterministic, with no live provider and no timing.

## Decisions

1. **Move the terminal-notice helpers into a new core module `brn_threads_core::run_attention` and re-export them from `brn_ai::threads`.** The settings, chat-control, steering and project paths are single core transactions. Adding the notice there is the only way to keep it in the same checked change, with no duplicate on replay. The re-export keeps every existing `brn_ai::threads::INTERRUPTED_ATTENTION` / `is_terminal_notice` / `replace_terminal_notice` user unchanged (app, tests, any UI-branch code). No wording changes.
2. **One shared helper, `ended_run_attention`, for every path that ends a Working Run outside `finish()`.** For each ended Run's open chat, it first adds the standard Review attention (`AttentionKind::Review`, `"Review prepared changes"`, `record` = operation) for each of that Run's still-active candidates that no attention item in the chat names yet. Then, if the path adds notices, it calls `replace_terminal_notice` with `INTERRUPTED_ATTENTION`. It edits a Thread write already in the change in place (Archive, MoveChat and Delete write the chat themselves; a duplicate write target is rejected), otherwise it adds one. It writes the Thread only when the attention actually changed, so receipts on paths with nothing to add stay byte-identical. It never applies, dismisses or retires a candidate.
3. **"Still-active candidate" means exactly what `Store::candidates()` lists:** prepared (`hash` set), `retired = 0`, no receipt, restricted to the Run's own `operations`. Unprepared or Invalid operations are skipped, and so are retired ones (refresh or dismiss) and applied ones.
4. **`Cancelled` counts as superseded wherever terminal notices are replaced:** in the shared helper, in `finish()`'s superseded set (both its Completed and non-Completed branches) and in orphan recovery. Cancelled Runs never carried a notice before, so this changes nothing for existing data.
5. **The in-process Interrupted and Failed results in `finish()` also add Review for the Run's own still-active candidates.** ⌘Q and cancellation reach `finish()` with Interrupted (observed natively), and a provider failure between prepare and apply is the same gap. A Completed Run is left unchanged: it chose not to apply, and its candidate stays in the candidates list (recorded limitation).
6. **Notice per path:**
   - *Add the notice* where the Run ends because of something the owner did elsewhere, or because the process died: workspace settings, project configuration and membership, project Delete, full-note import into a project, and orphan recovery (already does).
   - *No notice* where the owner stopped this chat's work from inside the chat and already sees the result: Stop, Archive, Steer, and Continue. After Stop the chat shows the Interrupted status with "Continue with current context" (`threads_writing.rs` ~290) and "Stopping… Your saved progress will remain available.", which matches existing Stop behaviour (the fenced worker's `finish()` writes nothing). Archive: `needs_you`/`thread_rows` include archived chats, so a notice would push a just-archived chat into Needs you with a Continue that is refused until Unarchive. Steer and Continue start a fresh Run in the same chat, whose own terminal result replaces notices.
   - Review attention (G2) is added on **every** path, including the no-notice ones. Otherwise the candidate stays invisible in its chat and keeps its note's `candidate_targets` slot ("one active candidate per note").
7. **G3 is in scope.** It is small: `run_lease::remove_free` deletes a lease file only while holding its lock, and only after orphan recovery has applied. A held (live) lease is never removed. Run IDs are unique, so no new holder can take an ended Run's lease.
8. **No desktop production change, no schema change, no dependency change.** If item 9 shows that something does not render, record it as a UI finding with steps. Do not change desktop code; pause instead if a change looks necessary.
9. **Contract text stays proposed, not applied.** As in earlier rounds, `threads-behavior.md`, `invariants.md` and `docs/status.md` are integration items. Put proposed sentences in `round4/verification.md`. Do not edit `docs/status.md`.

## Stop and cancel paths (grep: `RunState::Cancelled|RunState::Interrupted`, fence bumps outside tests)

| Path | Code | Ends Working Run as | Notice | Review (G2) | Test |
|---|---|---|---|---|---|
| Workspace settings: Configure (provider/model/credentials), delegation Choose, maintenance Pause/Resume, BackupDirectory | `brn-threads-core/src/store.rs` `update_workspace_settings` ~905–925 | `Cancelled`, every Working Run in the workspace, **no lease check** | **add** | add | T1, T1-live, T3, core delegation test |
| Orphan recovery (Resume, Dispatch) | `brn-threads-app/src/chat.rs` `recover_orphaned_runs` | `Interrupted` (lease free only) | existing (now via the helper) | add | T2 |
| Stop: `stop_chat_run` = `ChatCommand::Pause { run: Some }` (desktop Stop, `pause_failed_worker`, `resume_chat_queue`'s resume-stale stop) | `brn-threads-core/src/chat.rs` ~231–268 | `Interrupted` | no (Decision 6) | add | T4 |
| Archive / ArchiveProgress | `brn-threads-core/src/chat.rs` ~401–490 | `Interrupted`, every Working Run in the chat | no (Decision 6) | add (into the archived Thread write) | T4 |
| Steer: `stop_and_post(_with_partial)` | `brn-threads-core/src/steering.rs` ~120–143 | `Interrupted` | no (Decision 6) | add | T4 |
| Project commands: Configure, MoveChat, Link/UnlinkDocument, CreateDocument, CreateChat | `brn-threads-core/src/projects.rs` `project_command` fence loop ~686–703 | `Interrupted` for affected chats | **add** | add | T5 |
| Project Delete (chats go to Trash) | same loop; Thread write already present (archived) | `Interrupted` + archived | **add**, edited into the trashed Thread write, so the Trash `deleted_version` (version + 1) still matches and Restore shows it | add | T5 |
| Full-note import into a project | `brn-threads-app/src/intake.rs` ~309 via core `project_context_fences` | `Interrupted` for the project's chats | **add** | add | T5 |
| Continue / new Run in the same chat: `begin()` fences that chat's Working Runs | `brn-ai/src/threads.rs` `begin` ~1950–1968 | `Interrupted` | no (a new Run follows at once) | add (Thread write in the "Start a fresh invocation" change; inputs are checked against the prestate, `store.rs` ~1647) | T6 |
| In-process terminal: ⌘Q, cancellation, provider failure | `brn-ai/src/threads.rs` `finish` ~2367 | `Interrupted` / `Failed` | existing | **add** (Decision 5) | T7 |
| Legacy `Workspace::resolve_thread(_because)` | `brn-threads-app/src/lib.rs` ~303 | `Cancelled`; clears **all** attention by design | unchanged | unchanged | none (no production caller; the desktop's `resolve_thread` is Archive) |
| Theme / TopicsNote settings | `store.rs` ~820–838 | none (Deferred) | n/a | n/a | none |
| Project Restore | `projects.rs` ~600–640 | none (fence bump on un-archived Runs that are not Working) | n/a | n/a | T5 (restore shows the Delete notice) |

**Recorded, not changed:** the settings path stops Runs that are still live in another process. `update_workspace_settings` cancels every Working Run without consulting `derived/run-leases/`. The other process's watcher sees the fence within about 100 ms and stops, and its fenced `finish()` writes nothing. From this round, the settings change itself leaves that chat one Interrupted notice. T1-live documents it.

## Items

- [ ] 1. **Core `run_attention` module.** Create `crates/brn-threads-core/src/run_attention.rs` and declare `pub mod run_attention;` in `crates/brn-threads-core/src/lib.rs`. Do not glob-export the items at the crate root.
  - Move verbatim from `brn-ai`: `INTERRUPTED_ATTENTION`, `LEGACY_NOTICES` (private), `FAILED_PREFIX` (now `pub`), `is_terminal_notice` and `replace_terminal_notice`.
  - Add `pub const REVIEW_ATTENTION: &str = "Review prepared changes";`.
  - Add `pub fn add_review_attention(attention: &mut Vec<Attention>, candidates: &[String])`. It pushes `Attention { kind: Review, reason: REVIEW_ATTENTION, record: Some(op) }` for each operation that no item's `record` names, in the given order.
  - Add `pub fn ended_run_attention(writes: &mut Vec<Put>, ended: &[(String, Run)], notice: bool, record: impl Fn(&str) -> Result<Option<Record>>, active: impl Fn(&[OperationId]) -> Result<Vec<String>>) -> Result<()>`. Only `run.thread` and `run.operations` are read.
    - Group the ended Runs by chat, in order.
    - Use the chat's Thread write from `writes` if there is one; otherwise use `record(chat)` only if it is a non-archived Thread.
    - Per Run: `add_review_attention(active(&run.operations)?)`. Then, if `notice` is set, `replace_terminal_notice(…, INTERRUPTED_ATTENTION, superseded)`. A notice is superseded if its Run is another Run ended in this change, or a saved Run of the same chat in `Interrupted | Failed | Cancelled`. A lookup error counts as not superseded.
    - Update the existing write in place, or push `Put { id: chat, expected_version: Some(version), archived: false, … }`, only if the attention changed.
  - Add `impl Store { pub fn active_candidates(&self, operations: &[OperationId]) -> Result<Vec<String>> }` and `pub(crate) fn active_candidates_on(conn: &Connection, …)`. Both use the `candidates()` predicate: `hash IS NOT NULL AND retired=0 AND NOT EXISTS(receipt)`.
  - Unit tests in the module:
    - Review is skipped when any item already names the operation.
    - Two Runs ended in one chat leave one notice.
    - A notice on a `Cancelled` Run is replaced.
    - `notice = false` adds only Review.
    - An unchanged chat produces no write.
    - An existing Thread write is edited, never duplicated.
  - In `crates/brn-ai/src/threads.rs`, delete the moved definitions and add `pub use brn_threads_core::run_attention::{INTERRUPTED_ATTENTION, is_terminal_notice, replace_terminal_notice};`. `failed_attention` uses `brn_threads_core::run_attention::FAILED_PREFIX`. `PROVIDER_RECOVERY` and `FRESH_RECOVERY` stay in brn-ai. Replace the `"Review prepared changes"` literals in `attention()` (~1234, ~1245) with `REVIEW_ATTENTION`.
  - Files: `crates/brn-threads-core/src/{run_attention.rs,lib.rs}`, `crates/brn-ai/src/threads.rs`.
  - Verify: `cargo test -p brn-threads-core --locked --offline run_attention` passes. `cargo test -p brn-ai --locked --offline` passes, including the existing `terminal_notices_are_recognised…`, `failure_notices_name_the_cause…`, `repeated_failures_keep_one_notice…` and `completed_fresh_run_clears_only_superseded…`.

- [ ] 2. **`finish()` keeps prepared candidates in review, and the scripted crash helper.**
  - In `finish()`, capture `run.operations` before the Run is moved into its Put. Add `RunState::Cancelled` to the superseded match. In the non-Completed branch, call `add_review_attention(&mut data.attention, &store.active_candidates(&operations).map_err(storage)?)` before `replace_terminal_notice`. Leave the Completed branch unchanged apart from the Cancelled supersession.
  - Add, under `#[cfg(feature = "scripted-runs")]` next to `run_prepared_scripted`: `pub async fn run_prepared_scripted_then_abandon(prepared: PreparedThreadRun, model: MockCompletionModel) -> AiResult<String>`.
    - It runs `host_intake::with_context` and `run_model` exactly like `run_prepared_scripted`, then returns the Run ID **without** calling `finish()`.
    - Dropping the scope releases the lease and removes its file, like the existing `orphan()` crash simulation.
    - Doc comment: offline crash tests only; a SIGKILL would also leave the unlocked file, which tests recreate.
  - Add brn-ai unit test `an_interrupted_or_failed_run_keeps_its_prepared_candidate_in_review` in `threads.rs` `mod tests`, using `fixture()`, `begin`, `read_all`, `prepare(prepared_note(..))` and `chat_attention`. Cases:
    - (a) prepare, no apply, then `finish` Interrupted. Attention is `[Review(op), Blocker(run, INTERRUPTED_ATTENTION)]` and the candidate is still in `store.candidates()`.
    - (b) Same with `Failed(AiError::new(AiErrorKind::InvalidToolUse))`. The Blocker uses the `failed_attention` text.
    - (c) Completed: attention unchanged (`[]`), which documents Decision 5.
    - (d) A protected note, so `apply` returns NeedsReview and `attention()` adds Review, then Interrupted. There is still exactly one Review for the operation.
  - Files: `crates/brn-ai/src/threads.rs`.
  - Verify: `cargo test -p brn-ai --locked --offline` and `cargo clippy -p brn-ai --features scripted-runs --all-targets --locked --offline -- -D warnings` pass.

- [ ] 3. **G3: free leftover lease removal.** Add `pub fn remove_free(derived: &Path, run: &str) -> io::Result<bool>` to `crates/brn-ai/src/run_lease.rs`.
  - It resolves `lease_path(derived, run, false)` (NotFound gives `Ok(false)`) and opens without create, with `O_NOFOLLOW`.
  - On `try_lock` success it removes the file while holding the lock, unlocks and returns `Ok(true)`. `WouldBlock` returns `Ok(false)` and touches nothing.
  - Unit test `a_free_leftover_lease_is_removed_and_a_held_one_is_kept`: an abandoned 0-byte file is removed. A held `RunLease` is kept (`run_is_live` still true, file still present). A missing file gives `Ok(false)`. A symlinked folder is refused, as in the existing test.
  - Files: `crates/brn-ai/src/run_lease.rs`.
  - Verify: `cargo test -p brn-ai --locked --offline run_lease` passes.

- [ ] 4. **Orphan recovery uses the helper (G2) and cleans the free lease (G3).** In `Workspace::recover_orphaned_runs` (`crates/brn-threads-app/src/chat.rs`):
  - Replace the inline Thread/notice block with `brn_threads_core::run_attention::ended_run_attention(&mut writes, &[(record.id.clone(), run_before)], true, |id| Ok(records.iter().find(|r| r.id == id).cloned()), |ops| self.store.active_candidates(ops))?`. The idempotency key `orphan-recovery:{run}:{run version}:{chat version}` stays as it is.
  - After `ApplyOutcome::Applied`, call `let _ = brn_ai::run_lease::remove_free(&derived, &record.id);`. Never call it on `Stale` or for a live Run.
  - Create `crates/brn-threads-app/tests/killed_run_candidate.rs` with shared fixtures:
    - `world()` (like `run_recovery.rs`, but it keeps the project ID; a project chat "Information from b1-source" plus a general "Questions" chat; offline settings; delegation pending as in the native case).
    - `killed_intake(&w) -> (run, op)`: `prepare_run(intake request)` gives a Working Run holding its lease. Script `[call prepare_changes {request_key: "crane-arrival-source-capture-1", one new Note "Crane arrival — reported date"}, finish(..)]`, using the JSON shape from `daily_workflow_eval.rs`. Then `run_prepared_scripted_then_abandon`, then recreate the empty `derived/run-leases/<run with : → _>.lock` (via `store.derived_directory()`).
    - Assert the native pre-state: Run Working at fence 1, chat attention `[]`, operation in `candidates()`, no receipt, lease file present but not live.
    - Add **T2** (below).
  - Extend `run_recovery.rs`. In `resume_after_a_crash…`, write a leftover lease file before Resume and assert it is gone afterwards. In `a_live_run_keeps_blocking…`, assert the lease file still exists after the refused Resume and the empty `recover_orphaned_runs`.
  - Files: `crates/brn-threads-app/src/chat.rs`, `crates/brn-threads-app/tests/{killed_run_candidate.rs,run_recovery.rs}`.
  - Verify: `cargo test -p brn-threads-app --locked --offline --test run_recovery --test killed_run_candidate` passes.

- [ ] 5. **Settings path (G1 + G2) in the core.**
  - In `update_workspace_settings`, collect `(record.id, run)` for each Working Run it cancels. After the loop, call `crate::run_attention::ended_run_attention(&mut writes, &ended, true, |id| record_on(&tx, id), |ops| active_candidates_on(&tx, ops))?` before building the `ChangeRequest`.
  - The replay path (`hash.is_some()`) is untouched, so a replay returns the recorded receipt.
  - Update `crates/brn-threads-core/tests/delegation.rs::both_choices_and_pause_fence_every_working_run_and_block_stale_connection`. `receipt.writes.len()` goes from 3 to 5 (Settings, two Runs, two Threads). Also assert that each Run's chat attention is exactly `[Blocker(run, INTERRUPTED_ATTENTION)]`, and that run C's chat gets one after `pause_workspace`. This is a justified contract change; note it in the verification doc.
  - Add **T1**, **T1-live** and **T3** to `killed_run_candidate.rs`.
  - Files: `crates/brn-threads-core/src/store.rs`, `crates/brn-threads-core/tests/delegation.rs`, `crates/brn-threads-app/tests/killed_run_candidate.rs`.
  - Verify: `cargo test -p brn-threads-core --locked --offline --test delegation --test backup_directory` and `cargo test -p brn-threads-app --locked --offline --test killed_run_candidate --test delegation` pass.

- [ ] 6. **Stop, Archive, Steer and project commands.** In the core:
  - `chat.rs` `Pause { run: Some }`: if `was_working`, use ended `[(id, r)]` with `notice = false`.
  - Archive/ArchiveProgress: collect ended Runs in the loop, `notice = false`. The helper edits the archived Thread write already in `writes`.
  - `steering.rs`: if the Run was Working, `notice = false`.
  - `projects.rs` `project_command`: collect Runs from the "Fence every affected live invocation" loop. After the WithProgress partial block, convert `writes.into_values()` into a Vec, then call the helper with `notice = true` before `ChangeRequest`. Delete's trashed Thread write is edited in place.
  - Every call passes `|id| record_on(&tx, id)` and `|ops| active_candidates_on(&tx, ops)`.
  - Add **T4** and **T5** (Configure, Delete + Restore).
  - Files: `crates/brn-threads-core/src/{chat.rs,steering.rs,projects.rs}`, `crates/brn-threads-app/tests/killed_run_candidate.rs`.
  - Verify: `cargo test -p brn-threads-core --locked --offline` passes (steering.rs `writes.len() == 2` stays: no candidates, no notice). `cargo test -p brn-threads-app --locked --offline --test chat --test steering --test projects --test chat_decisions --test killed_run_candidate` passes.

- [ ] 7. **Full-note import into a project, and Continue (`begin()`).**
  - `crates/brn-threads-app/src/intake.rs` ~309: after `writes.extend(project_context_fences(&records, &p.chats)?)`, build `ended` from `records` with the same predicate as `project_context_fences` (Run, Working, chat in `p.chats`). Call the helper with `notice = true`, `|id| Ok(records.iter().find(..).cloned())` and `|ops| self.store.active_candidates(ops)`.
  - `crates/brn-ai/src/threads.rs` `begin()`: in the loop that fences this chat's Working Runs, collect `(record.id, run)`. After the loop, call the helper with `notice = false`, using `records` and `store.active_candidates`, mapping errors with `storage`. Read the Thread from `records`, not from the already matched `record`. The new Run's `chat_context_revision` stays the pre-change version. ChatContext compares the context epoch (`store.rs` `current_input_matches`), and the project start input is checked against the prestate.
  - Add **T5** (import) and **T6**.
  - Files: `crates/brn-threads-app/src/intake.rs`, `crates/brn-ai/src/threads.rs`, `crates/brn-threads-app/tests/killed_run_candidate.rs`.
  - Verify: `cargo test -p brn-ai --locked --offline`, then `cargo test -p brn-threads-app --locked --offline --test host_intake --test projects --test daily_workflow_eval --test source_intake_chat --test killed_run_candidate` pass.

- [ ] 8. **Offline native-check store (answers the owner's two points).**
  - In `killed_run_candidate.rs`, add `#[tokio::test(flavor = "multi_thread")] #[ignore = "writes a store for the owner's native check; set BRN_DWR_NATIVE_DATA and BRN_DWR_NATIVE_MODE"] async fn native_check_store_with_a_killed_intake_and_its_candidate()`.
  - Refuse unless `BRN_DWR_NATIVE_DATA` is an absolute path under `/private/tmp/brn-dwr-data/` that does not exist yet.
  - It creates the workspace with offline settings (`auth_file` and `credentials_dir` None) and a **general** chat "Information from b1-source". Then it runs `killed_intake` (this leaves the leftover lease file) and prints the chat, Run and operation IDs.
  - Mode `settings`: leave the delegation pending, so Home shows the two choices.
  - Mode `resume`: choose `MaintainOrdinaryNotes` **before** the orphan is created, create "Questions", `queue_message` there, then `recover_queue` (paused).
  - Files: `crates/brn-threads-app/tests/killed_run_candidate.rs`.
  - Verify: `BRN_DWR_NATIVE_DATA=/private/tmp/brn-dwr-data/r4-native-settings/data BRN_DWR_NATIVE_MODE=settings cargo test -p brn-threads-app --locked --offline --test killed_run_candidate -- --ignored native_check_store` passes. Do the same for `resume` with a different fresh path. Inspect both stores read-only through a copy (the `brn_inspect` helper in evidence.md): Run Working at fence 1, attention `[]`, the lease file present. Then run `brn-desktop --headless-check startup --data-dir <path>` on another copy of each; it prints "Threads ready". Delete the generated folders afterwards, or leave them for the owner and say so.

- [ ] 9. **Headless desktop state test (T8).** In `crates/brn-desktop/src/threads_intake_tests.rs` (this branch owns the file; no other desktop file changes).
  - Helper `killed_intake_in(ui)`. There is no scripted runtime in desktop dev-deps, so it reproduces the stored state directly. It reuses the `brn_ai::threads::prepare_run` request from `a_chat_whose_run_is_still_live_is_not_reused`:
    1. `ui.workspace.new_thread("Information from b1-source")`, then `prepare_run` in that chat (Working, lease held).
    2. `store.allocate_operation("{run}:proposal:crane-arrival-source-capture-1")`.
    3. `owner_change` appends the operation to `Run.operations` at the same fence (not a run-control change, `store.rs` ~2170).
    4. `store.prepare(&op, new Note "Crane arrival — reported date")`, then `drop(live)` and recreate the 0-byte lease file.
  - Test `a_killed_intake_candidate_reaches_needs_you_after_the_delegation_choice`:
    - *Before:* the chat's `ui.thread_rows` row has no reasons and is not in `lists::sidebar(..).needs`, while `ui.candidates` contains the operation. This records what the owner saw: only the sidebar "Review · …" row.
    - Then `ui.choose_delegation(DelegationChoice::MaintainOrdinaryNotes, cx)`.
    - *After:* the row's `reasons == [REVIEW_ATTENTION, INTERRUPTED_ATTENTION]`, the row is in `sidebar.needs`, `ui.candidates` still contains the operation, and after `ui.open_thread`, attention item 0's `record` matches a `Prepared` in `ui.candidates`. That is the join `conversation_detail` uses for the proposal card. If the fixture renders the chat tile (as in `threads_workbench_tests.rs`), also assert `attention-card-0` exists; otherwise note it in the verification doc.
  - Test `resume_after_a_kill_recovers_the_intake_and_keeps_its_candidate_reviewable`:
    - Choose delegation first, then create the orphan. `queue_message` in another chat, `recover_queue`, `refresh`.
    - Assert the Resume-pill condition: `queue_state()` is paused, its queue is non-empty, and there is no worker.
    - `ui.resume_chat_queue(cx)`, then poll until no worker. The intake row's reasons are `[REVIEW_ATTENTION, INTERRUPTED_ATTENTION]`, the lease file is gone, and the queued message ran once and failed offline.
  - If anything fails to show without a production desktop change, do not change it. Record a UI finding with reproduction steps, and pause with `send_message` severity `warning` if the fix would need desktop code.
  - Files: `crates/brn-desktop/src/threads_intake_tests.rs`.
  - Verify: `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` passes (expected 152 passed, 1 ignored). `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` passes.

- [ ] 10. **Fail-before sweep.** For each row of the test table, temporarily disable the named fix in place, run that test, and record the failing assertion and message. Restore the source, rerun until it passes, and confirm `git diff` shows only intended changes.

- [ ] 11. **Repository verification** (list below), from the worktree root.

- [ ] 12. **Docs.**
  - Write `docs/work/active/daily-workflow-reliability/round4/verification.md`:
    - the path table above with the final decisions, and the changed core test expectation (3 to 5 writes);
    - the fail-before/pass-after table, repository results and evidence classes;
    - the native re-check procedure (item 8 stores) and the "why no Resume button" finding;
    - the "why the agent cannot run the native check" answer;
    - limitations, and proposed contract sentences for `threads-behavior.md` / `invariants.md`.
  - Update `docs/work/active/daily-workflow-reliability/evidence.md`: Status rows for round 4 (Implemented / Verified / Native-observed: no / Owner-accepted: no / Merged: no), plus a pointer under the owner native results. Do not edit `docs/status.md`, `threads-behavior.md` or `invariants.md`.
  - Verify: `python3 scripts/check-markdown-links.py` and `git diff --check` pass.

## Tests and fail-before method

| ID | Test (file) | Asserts | Fail-before: disable |
|---|---|---|---|
| T1 | `the_delegation_choice_after_a_kill_leaves_one_notice_and_a_review_for_the_candidate` (`killed_run_candidate.rs`) | Native sequence. After `killed_intake`, `workspace.choose_delegation(MaintainOrdinaryNotes)` (the desktop's API) leaves: Run `Cancelled` at fence 2; chat attention exactly `[Review(op), Blocker(run, INTERRUPTED_ATTENTION)]`; operation still in `candidates()` with no receipt; Needs you lists the chat. A second `choose_delegation`, and a replay of the first receipt's operation through `workspace.store.choose_delegation(&receipt.operation, choice, &HostAuthority::owner(..))`, add no attention, and the replay adds no receipt. The leftover lease file remains (settings path, recorded limitation). | `ended_run_attention` call in `update_workspace_settings` (gives `[]`) |
| T1-live | `the_settings_change_also_stops_a_run_that_is_still_live` | With a `prepare_run` still held (`run_is_live` true), `choose_delegation` sets it `Cancelled` and adds one notice. Records the cross-process behaviour. | same call (no notice) |
| T2 | `resume_or_dispatch_after_a_kill_adds_the_review_and_removes_the_free_lease_file` | For each of `resume_queue` (after `queue_message` + `recover_queue`) and `dispatch_queued`, in fresh worlds: Run `Interrupted` at fence 2; attention exactly `[Review(op), Blocker]`; candidate active; lease file removed; a second `recover_orphaned_runs` returns `[]` with no new receipt; the queued question dispatches once. Then `review_apply(op)` is Applied, its Review is removed and the notice stays (exact-candidate review still works). | `active` closure in `recover_orphaned_runs` returning `Ok(vec![])` (no Review); separately the `remove_free` call (file remains) |
| T3 | `a_later_run_replaces_the_notice_of_a_run_the_settings_change_cancelled` | After T1's state, a scripted Completed Run in the intake chat leaves `[Review(op)]`. | `RunState::Cancelled` in `finish()`'s superseded match (notice stays) |
| T4 | `stop_archive_and_steer_keep_the_candidate_in_review_without_a_notice` | Fresh world per case: `stop_chat_run`, `archive_chat`, `stop_and_post`. Each leaves the Run `Interrupted`, attention exactly `[Review(op)]` (archived Thread for Archive), the candidate active, and no Interrupted Blocker. Replaying the same key returns the same receipt with unchanged attention. | helper call in the matching arm (`chat.rs` Pause, `chat.rs` Archive, `steering.rs`) |
| T5 | `project_changes_and_project_imports_leave_the_notice_and_the_review` | Fresh world per case: `ProjectCommand::Configure`; `import_full_scoped(.., Some(project))` with a FullNote `ConversionResult`; Delete (`project_delete_preview`, no actions) then Restore. Each leaves `[Review(op), Blocker(run)]` (the restored chat for Delete) with the candidate active, and Restore succeeds. | helper call in `project_command`; helper call in `import_full_scoped` |
| T6 | `continue_in_the_killed_chat_keeps_the_candidate_in_review` | A new `prepare_run` in the killed chat fences the orphan (`Interrupted`, fence 2) and the chat shows `[Review(op)]` with no notice. The new Run, scripted to Completed, leaves `[Review(op)]`. | helper call in `begin()` |
| T7 | `an_interrupted_or_failed_run_keeps_its_prepared_candidate_in_review` (`brn-ai` `threads.rs` tests) | Item 2 cases (a)–(d). | `add_review_attention` call in `finish()` |
| T8 | two tests in `threads_intake_tests.rs` | Item 9. | the settings helper call (first test); the `recover_orphaned_runs` `active` closure (second test) |
| U1 | `run_attention` unit tests (core) | Item 1. | n/a (pure helper; each case asserts its own rule) |
| U2 | `a_free_leftover_lease_is_removed_and_a_held_one_is_kept` (`run_lease.rs`) | Item 3. | `remove_free` returning `Ok(false)` without removing |
| C1 | updated `delegation.rs` core test | 5 writes and one notice per chat. | settings helper call |
| R1 | `run_recovery.rs` additions | lease file removed after recovery, kept while live | `remove_free` call |

## Verification (worktree root, `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`)

- `cargo fmt --all --check`
- `cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline`
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
- `cargo test --workspace --locked --offline --no-fail-fast`. Baseline: 706 passed, 4 ignored, and only the 12 known `brn-threads-intake` sandbox failures (`process::tests` 3, `tests/qualification.rs` 9). Expect 706 plus the new tests, and 5 ignored (the item 8 generator). Report the exact counts and list any other failure.
- `cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop`. Baseline: 150 passed, 1 ignored; expect 152 and 1.
- `cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings`
- `TMPDIR=/private/tmp/brn-dwr-data/r4-e2e scripts/verify-end-to-end.sh --fixtures-only` (fresh folder)
- `cargo build -p brn --locked --offline`, then `python3 docs/work/active/daily-workflow-reliability/cli-journey.py /tmp/brn-dwr-target/debug/brn /private/tmp/brn-dwr-data/r4-cli-journey`. The script **deletes ROOT recursively**, so pass only this fresh path. Expect the round 3 observations unchanged (no run prepares a candidate offline).
- `git diff --check`; `python3 scripts/check-markdown-links.py`

Data stays under fresh `/private/tmp/brn-dwr-data/<case>/` folders and test temp directories. No provider calls, no owner data or credentials, and no Mac UI driven by the agent.

## Native re-check procedure (for `round4/verification.md` and finalize)

The owner runs this in their own Terminal. It is offline, so no provider is needed, and the UI agent must not be driving the Mac. Build `brn-desktop` with `--features native-ui` and the helper as in [Owner manual check](../evidence.md#owner-manual-check). Generate each store with item 8's command. Launch with `env HOME=<case>/home CODEX_HOME=<case>/codex BRN_INTAKE_HELPER=… brn-desktop --data-dir <case>/data`, then inspect with `brn_inspect` after ⌘Q.

1. **`settings` store (Review item in Needs you).**
   - Home shows the two maintenance choices. Pick one.
   - *Expected:*
     - Home's Needs you lists "Information from b1-source" with "Review prepared changes" and "Run interrupted. Continue starts fresh.".
     - The sidebar NEEDS YOU shows that chat and a "Review · …" row.
     - Opening the chat shows two attention cards; the first carries the proposal ("Crane arrival — reported date") with Apply and Dismiss.
   - *Store:* Run `Cancelled` at fence 2, one Blocker and one Review for the chat. The lease file is still present (recorded limitation).
2. **`resume` store (Resume button).**
   - The composer area shows "Queue · 1 waiting · paused" with Resume enabled. Click Resume.
   - *Expected:*
     - The intake chat shows the same two Needs you reasons.
     - The queued message runs once and fails offline. "Questions" gets "Run failed: The provider account needs reconnecting. Continue when the provider is ready." and the queue pauses again.
   - *Store:* Run `Interrupted` at fence 2, and `derived/run-leases/` no longer holds that Run's file.

## Limitations to record (not fixed in round 4)

- **Settings and leases.** The settings path stops Runs that are live in another process (now with a notice) and leaves an orphan's lease file in place. Only orphan recovery removes it.
- **Continue does not clean the lease.** `begin()` fences Working Runs in its chat whether or not they are live, so it does not remove their lease files.
- **Completed Runs.** A Completed Run that prepared but never applied a candidate still gets no Review attention; the candidate stays in the candidates list (Decision 5).
- **Stop from a failure.** `pause_failed_worker` and the resume-stale stop share `stop_chat_run` with the deliberate Stop, so they also add no notice. The service cannot tell them apart without a desktop change.
- **Legacy Resolve.** `Workspace::resolve_thread` still clears every attention reason (legacy; no production caller).
- **Native observation of round 4.** Needs the owner (item 8 procedure). Until then: implemented and verified (automated, headless desktop), not native-observed, not owner-accepted, not merged.

## Ownership

- Never touch `/Users/evokessler/repos/brn-rust/.worktrees/ui-polish`.
- Desktop: only `crates/brn-desktop/src/threads_intake_tests.rs` changes. Any other desktop change needs a `warning` pause.
- Do not edit `docs/status.md`.
- Shared seams touched this round, which the UI branch must rerun at integration: `begin`, `finish` and `attention` in `brn-ai/src/threads.rs`; `recover_orphaned_runs`; core `chat.rs`, `steering.rs`, `projects.rs` and `store.rs` settings; the `brn_ai::threads` re-exports.
