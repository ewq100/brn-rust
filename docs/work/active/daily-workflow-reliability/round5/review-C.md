# Lease-free Runs end like orphans on every stop path (round 5 slice C)

Slice C (`abd9a84f`) makes Stop, the failure stop, the desktop's resume-stale stop, Archive, Steer and Continue treat a Working Run whose lease no process holds as an orphan. Such a Run gets one Interrupted notice through the shared `ended_run_attention_with` helper, a Review item for each unapplied candidate, and removal of its free lease file. Settings, project commands and project imports already added the notice, and now also remove the free file. The core still cannot see leases, so the service computes the lease-free set before the checked change and passes it in as data outside the operation binding. Lease files are removed afterwards through `remove_free`, which deletes a file only while holding its lock.

Watch for: Continue in a killed chat now writes "Run interrupted. Continue starts fresh." while the new Run works. For a killed Run with no candidate, that puts the chat in Needs you during the Continue, which conflicts with `threads-behavior.md` line 34 (confirmed, documented as a deliberate consequence). The T6 rewrite swapped the Completed result for a Failed one, so no test shows that a successful Continue retires the new notice (confirmed). One limitation sentence says a Run that dies after the liveness check keeps its file, but on the service paths `remove_ended_leases` removes it (confirmed). None of these block.

**Verdict**: APPROVED

## High-level view

No live lease can be removed. Every removal goes through `run_lease::remove_free`, which takes the lock first and backs off on `WouldBlock`. The service calls it only for Runs the receipt moved from Working to an ended state, and `begin()` only for its orphans after its change is `Applied`. The set is read before the transaction, and the plan's safety argument holds in the code. The only production construction of a Working Run is in `begin()`, which first takes a lease for a fresh `unique_host_key` Run ID. Helper sub-runs save no Run, and a terminal Run cannot reactivate. So a free lease never becomes held again, and a stale set can only treat a Run as live. The desktop's Stop (`cancel`), Archive (`resolve_thread`) and Steer (`stop_and_send`) each commit the stop before cancelling the worker. The live worker therefore still holds its lease when the set is computed, and a deliberate Stop adds no notice.

Each chat keeps one terminal notice. `replace_terminal_notice` drops notices of Runs ended in the same change and of earlier Interrupted, Failed or Cancelled Runs in that chat, and `finish()` replaces or retires the begin-time notice. Both `chat_command_with_orphans` and `stop_and_post_checked` return the recorded receipt before reading the orphan set. C-T1, C-T2 and T4 show that a replay adds no receipt, notice or partial Message. Candidates are only named in Review items, never applied or retired, and every killed-path test ends with `assert_reviewable`.

Continue is the one path where the new notice is transient by design. It is shown from the new Run's start to its result, and during that time a chat with no candidate is in Needs you.

The killed-queued-then-Resume path is covered at both layers. The app test C-T2 follows the desktop's call order: `queue_message`, `dispatch_queued`, `prepare_run` with `request_message`/`queue_claim`, kill, `stop_chat_run("resume-stale:…")`, `resume_queue`, `dispatch_queued`. The desktop test C-D1 drives the real `ui.resume_chat_queue(cx)` after a simulated restart. Ownership holds. Desktop changes are confined to `threads_intake_tests.rs` (fixture plumbing and C-D1). `main.rs`, `docs/status.md`, `docs/architecture/*`, `Cargo.lock` and the schema are untouched. The `brn-ai` production change is the plan's C3 `begin()` edit.

The evidence matches the code and test counts. Workspace 759 → 765 matches five new app tests and one core unit test, and desktop 153 → 154 matches C-D1. The recorded failing lines in `killed_run_candidate.rs` (319, 339, 371, 441, 488, 489, 601, 632, 633, 731, 750, 772) land on the asserts named. C-D1's lines predate a `cargo fmt` reflow, as the doc notes. Every new or tightened test has a fail-before, either from the stashed source or from a named guard mutation (G1–G9). The manual CLI stores `r5-c-archive` and `r5-c-pause` exist under `/private/tmp/brn-dwr-data/`.

<details>
<summary>Issues (5)</summary>

1. **Continue's notice puts a resumed chat in Needs you** (non-blocking, confirmed). While the new Run works, the row shows "Run interrupted. Continue starts fresh.", which conflicts with `threads-behavior.md` line 34. Add it to slice C's integration items as a behavior-doc question for the owner and the UI branch, next to the proposed `invariants.md` sentence.
2. **No test for a successful Continue after a kill** (non-blocking, confirmed). T6 now ends in Failed, and the remaining success test starts from a Cancelled Run that `begin()` does not fence. Add a Completed leg to T6 that expects `[Review(op)]`.
3. **Limitation overstates a kept lease file** (non-blocking, confirmed). "Its file stays until orphan recovery or a later stop removes it" is true only for `begin()`. On the service paths `remove_ended_leases` removes it in the same call, with no notice. Reword the limitation in `round5/verification.md`.
4. **`configure` and `choose_backup_directory` lease removal untested** (non-blocking, confirmed). Only `choose_delegation` (T1) and, manually, `pause` are exercised, and the two replay returns of `import_full_scoped` are unexercised. Loop T1 over the four settings wrappers, or accept the gap.
5. **Full record scan before each Stop, Archive and Steer** (non-blocking, confirmed). `lease_free_working_runs` parses every record body, notes included. Filter Working Runs in SQL as the core's queue checks do, or accept the cost.

</details>

<details>
<summary>Details</summary>

### Liveness as data

```
host (brn-threads-app)                         core (brn-threads-core)
lease_free_working_runs()  ── orphans ──▶  chat_command_with_orphans / stop_and_post_checked
  records() + run_is_live                     ended_run_attention_with(|id| orphans.contains(id))
remove_ended_leases(receipt) ◀── receipt ──  (one Immediate transaction)
  remove_free per Working→ended Run

begin() (brn-ai): run_is_live per fenced Run → notice → Applied → remove_free(orphans)
```

`lease_free_working_runs` calls `Workspace::records()`, which reads and parses every record body, notes included, before each Stop, Archive and Steer. Resume already pays the same scan through `recover_orphaned_runs`. The core's queue checks find Working Runs with a `json_extract` filter, and the same filter would bound this read on an interactive control.

### Continue's transient notice and its missing success test

`begin()` now gives a lease-free fenced Run the Interrupted notice in the "Start a fresh invocation" change. The desktop row's `reasons` are the chat's attention reasons, and `needs_you()` is `!reasons.is_empty()`. So between the new Run's start and its result, the row reads "Run interrupted. Continue starts fresh." and the chat is in Needs you. Before the slice that happened only if the killed Run had left a candidate. `threads-behavior.md` says "An interruption only enters Needs you when resuming requires an owner action", and here the owner has just resumed. The verification doc lists the visible effect under "Two consequences an owner can see". Its proposed doc text covers only `invariants.md`, so nobody has yet asked the owner or the UI branch to settle this as a behavior-doc question.

By reading, `finish()` treats every terminal notice of an Interrupted, Failed or Cancelled Run in the chat as superseded, so a Completed result retires the begin-time notice. But T6 used to run the Continue to Completed and now runs it to Failed, as the plan asked. The closest remaining success test, `a_later_run_replaces_the_notice_of_a_run_the_settings_change_cancelled`, starts from a Cancelled Run that `begin()` does not fence. No test covers the path this slice creates: a begin-time notice followed by success.

### Limitation wording and untested wrappers

The limitation "A Run whose process dies after the check is treated as live: no notice, and its file stays until orphan recovery or a later stop removes it" holds for `begin()`, which removes only its orphans. On the service paths, `remove_ended_leases` calls `remove_free` for every Run the receipt ended. A Run that died after the check therefore loses its file in the same call, but gets no notice.

The settings wiring spans four wrappers, but tests and guards exercise only `choose_delegation` (T1, T1-live) and, through the manual CLI check, `pause`. `configure` and `choose_backup_directory` have no lease-file assertion, and the two replay returns of `import_full_scoped` are unexercised. Each site is one `remove_leases_ended_by` call, so the risk is a later edit that drops one without a failing test.

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-core/src/run_attention.rs`: `ended_run_attention_with` (per-Run notice), `ended_run_attention` now wraps it; unit test `per_run_notice_marks_only_lease_free_runs`.
- `crates/brn-threads-core/src/chat.rs`: `chat_command_with_orphans`; Pause and Archive arms use the per-Run notice.
- `crates/brn-threads-core/src/steering.rs`: `stop_and_post_checked`; old entry points delegate with an empty set.
- `crates/brn-threads-app/src/chat.rs`: `lease_free_working_runs`, `remove_ended_leases`, `remove_leases_ended_by`; wired into `chat_command`.
- `crates/brn-threads-app/src/steering.rs`: steering passes the set and removes ended leases.
- `crates/brn-threads-app/src/settings.rs`: `configure`, `choose_delegation`, `choose_backup_directory`, `pause` remove ended leases on `Applied`.
- `crates/brn-threads-app/src/projects.rs`, `intake.rs`: project commands and the three applied returns of `import_full_scoped` remove ended leases.
- `crates/brn-ai/src/threads.rs`: `begin()` marks lease-free fenced Runs as orphans and removes their files after `Applied`.
- `crates/brn-threads-app/tests/killed_run_candidate.rs`: C-T1, C-T2, C-T3, T4-live, T6-live; T1, T1-live, T4, T5, T6 updated.
- `crates/brn-desktop/src/threads_intake_tests.rs`: fixture plumbing (`offline_workspace`, `lease_path`, `killed_run_in`) and C-D1.
- `docs/work/active/daily-workflow-reliability/round5/verification.md`: slice C section.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability show abd9a84f`.

</details>
