# Round 4 review: a killed Run's notice and its candidate stay reachable

When a Working Run ends outside its own `finish()`, its chat now gets attention in the same checked change. That is an Interrupted notice when the stop came from outside the chat, and a "Review prepared changes" item for each candidate the Run left prepared. `finish()` adds the same Review on its Interrupted and Failed results, and orphan recovery removes the free leftover lease file. The notice rules moved verbatim into a new core module, `brn_threads_core::run_attention`, which `brn_ai::threads` re-exports, so the core settings, chat-control, steering and project transactions can use them. The owner's two questions get answers, not code: Resume never appeared because the queue was empty, the agent's shell cannot run the native check, and two offline stores make the check deterministic.

Watch for: an archived chat whose stopped Run left a candidate now shows under NEEDS YOU (confirmed). The proposed invariant sentence overstates what the code does for Completed Runs (confirmed). The "needs a desktop change" reason given for the failure and resume-stale stops is wrong (likely). A killed queued Run followed by Resume takes an untested path (confirmed). None of these blocks the round.

**Verdict**: APPROVED

## High-level view

One helper, `ended_run_attention`, runs inside every checked change that ends a Working Run outside `finish()`: settings, orphan recovery, Stop, Archive, Steer, project commands (including Delete into Trash), full-note import into a project, and `begin()`'s fencing. A grep of every production site that sets a Run to Interrupted, Cancelled or Failed finds no other path except the legacy `resolve_thread`, which has no caller. The helper edits a Thread write the change already has instead of adding a second one, and writes nothing when the attention is unchanged. Replays return the recorded receipt before the helper runs. Every one of these changes writes run control, so Undo refusals stay refusals.

The notice depends on who acted. Settings, project changes, imports and a dead process add it. Stop, Archive, Steer and Continue do not, because the owner acted in that chat or a new Run follows at once. Archive is the weak spot: the Review alone already raises the archived chat into NEEDS YOU, which is the effect the notice was withheld to avoid. The desktop's failure stop and resume-stale stop share the notice-free `stop_chat_run` with the deliberate Stop. That is recorded as a limitation, but the service could tell those stops apart by lease without a desktop change.

`Cancelled` now counts as superseded in `finish()`, in the helper and in orphan recovery, so the settings path's new notice gives way to the next terminal result. Candidates are only read, through the `candidates()` predicate limited to the Run's own operations. A Completed Run still adds no Review (Decision 5), and the proposed contract sentence does not reflect that.

The logs in `/private/tmp/brn-dwr-data/r4-logs/` match the recorded counts. The workspace run was 721 passed, 12 failed and 5 ignored, and the 12 failures are the known `brn-threads-intake` sandbox tests. The desktop run was 152 passed and 1 ignored. Every new integration test has a disable-then-restore entry. The six pure-helper unit tests are marked n/a. Round 4 changes only `threads_intake_tests.rs` in the desktop, adds no user-facing text and leaves `docs/status.md` untouched. The sandbox tests and `verify-storage.sh` were not verified in this shell.

<details>
<summary>Issues (6)</summary>

1. **Archived chat enters NEEDS YOU through the Review** (confirmed, non-blocking). In `round4/verification.md` (Archive row and Limitations), state that archiving a chat whose stopped Run left a candidate now lists the archived chat under NEEDS YOU with the proposal card. Reword the reason that a notice "would raise a just-archived chat".
2. **Proposed contract sentences overstate reachability** (confirmed, non-blocking). Before integration, limit the `invariants.md` sentence to Runs that end Interrupted, Failed or Cancelled, or through a checked stop. A Completed Run's unapplied candidate gets no chat attention. Also add `finish()`'s Interrupted and Failed results to the `threads-behavior.md` sentence.
3. **"Needs a desktop change" is the wrong reason for the failure and resume-stale stops** (likely, non-blocking). Both reach `stop_chat_run` with no live in-process worker. The service could recognise a lease-free Run there, as orphan recovery already does, and give it the notice and the lease cleanup. Correct the limitation text and record the follow-up as a service-side change.
4. **Killed queued Run + Resume is untested** (confirmed, non-blocking). When the killed Run is the pending queue item, Resume goes through the desktop's resume-stale `stop_chat_run` instead of orphan recovery. The Review is added, no notice is added, the lease file stays, and the item is re-dispatched. Add this case to the limitations and the native procedure's expectations, or cover the same sequence with an app-level test.
5. **Helper's own Cancelled supersession has no fail-before record** (confirmed, non-blocking). U1 is marked n/a, and T3 exercises only `finish()`'s Cancelled match. Reading the code, each U1 case fails when its rule is removed. Record one mutation result for `a_notice_on_a_cancelled_run_is_replaced`.
6. **Sandbox-dependent checks still unverified** (confirmed, environment). Rerun the 12 `brn-threads-intake` tests and `scripts/verify-storage.sh` from a shell where `sandbox-exec` works, or rely on required CI, before calling the branch verified for merge.

</details>

<details>
<summary>Details</summary>

### One helper inside each stopping transaction

```
update_workspace_settings ─┐ (notice)
project_command ───────────┤ (notice; Delete edits the trashed Thread write)
import_full_scoped ────────┤ (notice)
recover_orphaned_runs ─────┤ (notice + remove_free after Applied)
ChatCommand::Pause{run} ───┤ (no notice)          ┌─ add_review_attention(active candidates)
ChatCommand::Archive* ─────┼─ ended_run_attention ┤
stop_and_post ─────────────┤ (no notice)          └─ replace_terminal_notice(INTERRUPTED_ATTENTION)
begin() fencing ───────────┘ (no notice)
finish() Interrupted/Failed ── add_review_attention + replace_terminal_notice (existing call)
```

The core paths compute the attention inside the same IMMEDIATE transaction that fences the Run (`record_on(&tx, …)`, `active_candidates_on(&tx, …)`). `begin()`, orphan recovery and the import read from a `records` snapshot instead. Their Thread writes carry the snapshot version, so a concurrent chat change makes the whole change Stale. Adding a Thread write to `begin()`'s "Start a fresh invocation" change does not invalidate the new Run's own inputs, for two reasons. The project start input is checked against the transaction's prestate (`store.rs` ~1652). The Run's later proposals bind `ChatContext`, which compares only `context_epoch`, and the helper never changes that field.

### Where Archive breaks the no-notice rationale

`verification.md` says an Archive notice "would raise a just-archived chat" into Needs you, with a Continue that is refused until Unarchive. The Review the helper does add raises the chat the same way (confirmed). `threads_lists::thread_rows` keeps archived chats as `resolved` rows with their reasons. `sidebar` puts every row that has reasons into `needs`. `conversation_detail` draws attention cards for an archived chat, so the proposal card with Apply and Dismiss can be reached. T4 asserts `[Review(op)]` on the archived Thread. G2 asks for the Review on every path, so the effect is defensible. But the owner will see it, and the docs do not say so.

### Failure and resume-stale stops share the deliberate Stop

`stop_chat_run` also serves `pause_failed_worker` (the worker channel disconnected) and the resume-stale stop in `resume_chat_queue`. `verification.md` says telling these apart "needs a desktop change". The resume-stale branch runs only when `self.worker.is_none()`. The Run it stops is therefore either live in another process or lease-free. The same likely holds after a disconnected worker has dropped its scope. `Workspace::stop_chat_run` could check `run_is_live`, or call `recover_orphaned_runs` first, and treat lease-free Runs as orphans without any desktop edit.

For resume-stale, the missing notice matters little. Resume clears the pending item's `run` and `dispatch`, and `dispatch_next` re-dispatches it in the same chat, so a new Run's result follows. The leftover lease file stays, because only orphan recovery calls `remove_free`. No round-4 test covers this sequence: a killed Run that is the pending queue item, then Resume. It is the variant closest to the owner's Resume question. T8b and T2 use an intake orphan that is not bound to the queue.

### Contract sentences vs. Completed Runs

The proposed `invariants.md` sentence says a still-active candidate a Run prepared "stays reachable from its chat's attention". T7 case (c) asserts the opposite for a Completed Run (confirmed): attention stays `[]` while the candidate remains listed. The proposed `threads-behavior.md` sentence covers only "a Working Run [that] ends without its own terminal result". It leaves out the new Review on `finish()`'s Interrupted and Failed results.

### Test evidence gaps

Commit `d83ba18d` changes only docs, and the logs were written after the code commit `3919c09a`. The helper's `Cancelled` supersession inside `ended_run_attention` is exercised only by U1, which has no recorded fail-before. Reading the code, removing `RunState::Cancelled` from its predicate leaves `[blocker(run-old), blocker(run-new)]`, so `a_notice_on_a_cancelled_run_is_replaced` would fail.

Not tested:
- The queued-kill Resume path above.
- Replaying a project command or import receipt. The code returns the receipt before building writes.
- The settings change across two processes. T1-live uses a lease held in the same process.
- The 12 sandbox-dependent intake tests. They fail here with "sandbox_apply: Operation not permitted", and so does `verify-storage.sh`.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/run_attention.rs`: new module with the moved notice rules, `REVIEW_ATTENTION`, `add_review_attention`, `ended_run_attention`, `active_candidates(_on)` and 6 unit tests.
- `crates/brn-threads-core/src/lib.rs`: `pub mod run_attention`.
- `crates/brn-threads-core/src/store.rs`: the settings path collects cancelled Runs and calls the helper with a notice.
- `crates/brn-threads-core/src/chat.rs`: the Pause and Archive arms collect ended Runs and call the helper without a notice.
- `crates/brn-threads-core/src/steering.rs`: `stop_and_post` calls the helper without a notice.
- `crates/brn-threads-core/src/projects.rs`: the `project_command` fence loop collects Runs and calls the helper with a notice over the Vec of writes.
- `crates/brn-threads-core/tests/delegation.rs`: the settings receipt now has 5 writes and one notice per chat.
- `crates/brn-ai/src/threads.rs`: re-exports; `REVIEW_ATTENTION` in `attention()`; the helper call in `begin()`; the Review and Cancelled supersession in `finish()`; `run_prepared_scripted_then_abandon`; T7.
- `crates/brn-ai/src/run_lease.rs`: `remove_free` and U2.
- `crates/brn-threads-app/src/chat.rs`: orphan recovery uses the helper and removes the free lease after Applied.
- `crates/brn-threads-app/src/intake.rs`: the full-note import into a project calls the helper with a notice.
- `crates/brn-threads-app/tests/killed_run_candidate.rs`: T1–T6 and the ignored native-store generator.
- `crates/brn-threads-app/tests/run_recovery.rs`: lease-file assertions (R1).
- `crates/brn-desktop/src/threads_intake_tests.rs`: headless T8a and T8b.
- `docs/work/active/daily-workflow-reliability/{evidence.md,round4/plan.md,round4/verification.md}`: round 4 plan, evidence and status rows.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability diff 0efb6bef d83ba18d`.

</details>
