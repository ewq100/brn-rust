# Intake-chat reuse skips chats whose Run is still live

`Workspace::source_intake_chat` now drops any reuse candidate whose chat holds a Working Run with a held lease (`brn_ai::run_lease::run_is_live`). Before this change, a same-source re-supply could land in a chat where `begin()` fences another process's Run. Selection falls through to the next most recent eligible chat, or creates a new one. A lease-free (orphaned) Working Run still allows reuse, and the next Run there interrupts it as before. The CLI output keeps its shape (`reused_chat: false` plus the new `thread`), and its help text and comments now describe the skip. A regression test holds a real lease through a prepared Run.

Watch for: the guard is check-then-act. Two supplies that both pass selection before either Run begins still share a chat, and the later `begin()` fences the earlier Run. This is confirmed by code reading; the window is narrow and this change does not close it. Only the new-chat fallback is tested, not fallback to an older eligible chat (confirmed). When `derived/` is unusable, a chat with a Working Run is skipped, and `begin()` then refuses, so each attempt leaves a new empty chat behind (confirmed by trace; rare).

**Verdict**: APPROVED

## High-level view

The `busy` predicate matches exactly the set `begin()` fences: every Run record in the chat whose state is Working, archived or not. It runs after the scope, open-chat and archive filters, so `max_by_key` over the survivors yields the next most recent eligible chat. When `derived/` cannot be resolved, liveness counts as live, in line with `run_is_live`'s "doubt counts as live" rule.

The guard reads a records snapshot and the lease files. `begin()` later opens its own store and fences without checking leases, and the CLI builds a search handle in between. The fix covers the reported case, where the earlier Run already holds its lease when the second supply selects. It does not cover two supplies racing through selection together. The evidence's limitation line reads as if concurrent supplies always end up in separate chats.

The new test is a valid regression with a credible fail-before. The lease is held through a separate file handle, so the test exercises a real `flock` conflict in-process. With the skip disabled, it fails at line 160 with `(Interrupted, 2)` against `(Working, 1)`, as recorded.

The recorded verification matches the saved logs, which postdate the final source edits. `task-a-workspace-test.log` totals 670 passed, 12 failed and 4 ignored, against a baseline of 669/12/4. The 12 failures are the same `brn-threads-intake` sandbox targets (3 in the lib, 9 in `qualification`). The desktop run shows 147 passed. The CLI journey shows `useful_intake_new_threads` 1, `needs_you` 2, 24 receipts and an empty `derived/run-leases/`. The fmt and clippy passes are recorded as claims with no saved output; I accepted them without re-running.

The diff does not touch `crates/brn-desktop` or `docs/status.md`. It only changes how a chat is selected: no new write path, no schema change and no new dependency. New chats still go through the checked `new_thread` / `ProjectCommand::CreateChat` commands. Project boundaries, protected-note authority, exact-candidate review, version checks, dirty guards, replay and Undo sit on code this change does not modify. No reply or delivery wording changed.

<details>
<summary>Issues (3)</summary>

1. **Selection-to-begin race remains** — two same-source supplies that both select before either Run begins still share a chat, and the later `begin()` fences the earlier Run. Record this residual window in the evidence limitations. Closing it needs a liveness check inside `begin()`'s write, which is outside Task A.
2. **Older-chat fallback untested** — the test covers only the fallback to a new chat. Optionally add a case where an older, non-busy chat that received the source is chosen over a newer busy one.
3. **Empty chat per attempt when `derived/` is unusable** — fail-closed liveness skips the chat, a new chat is created, and `begin()` then refuses on the same `derived/` check. Either return the storage error before creating a chat, or note it beside the existing "refused start leaves a chat" limitation.

</details>

<details>
<summary>Details</summary>

### Check-then-act gap between `source_intake_chat` and `begin()`

The guard and the fence run in different transactions, and the CLI builds the search handle between them:

```
CLI supply A                         CLI supply B
source_intake_chat -> chat X         source_intake_chat -> chat X   (no Working Run yet)
search_handle()                      search_handle()
begin(): Run RA Working + lease
                                     begin(): fences every Working Run in X -> RA Interrupted
```

`begin()` (`crates/brn-ai/src/threads.rs:1787-1805`) still interrupts every Working Run in its chat without checking the lease. That is the Continue contract, so changing it is outside this task. Conversion happens before selection, so the window covers only `search_handle()` and the opening of `begin()`. Supplies run back to back from a script would almost never hit it, but two agents firing at once could. The evidence's new limitation ("Two concurrent supplies of the same source still make two chats") covers only the case where the earlier Run has already started. One added sentence about this window would stop integration reviewers from treating Task A as a complete concurrency guard. Confidence: confirmed by code reading; not exercised.

### Fail-closed liveness when `derived/` cannot be resolved

`self.store.derived_directory().ok()` turns an unusable `derived/` (for example, a symlink) into `None`, and `is_none_or` then marks every chat with a Working Run as busy. Selection then creates a new chat through a checked write. The CLI's `begin()` calls the same `derived_directory()` before acquiring the lease and fails, so the supply errors out with the empty chat already saved. On the base commit, the existing chat was reused and `begin()` refused without creating anything. The trigger needs a tampered `derived/` and an existing Working Run, so the impact is low. It sits beside the evidence's existing limitation that a refused start leaves a chat. Confidence: confirmed by trace (`intake.rs:71-78`, `threads.rs:1735-1737`); not exercised.

### Regression test: lease held through a prepared Run

`a_chat_whose_run_is_still_live_is_not_reused_until_its_lease_is_free` holds the lease through `PreparedThreadRun`, which owns `Scope::_lease` on its own `Store` and file handle. It asserts that the live Run stays `(Working, fence)` and that the supply goes to a different chat with `reused == false`. After `drop(live)`, it asserts that the original chat is reused and that the next Run fences the orphan to `(Interrupted, fence + 1)`. Before the second lookup, the test archives the newer chat. Without that step, the newer chat would win on recency and hide whether the original chat became eligible again. That leaves the released lease as the only difference between the two lookups.

The fail-before claim holds up. With `(true || !busy(chat))`, the original chat is the only candidate, so `other == original`. `supply` then fences the live Run before the `reused` and `assert_ne!` checks run, which produces the recorded failure at line 160. No Task A fail-before log was saved; `fail-before.log` in `/tmp/brn-dwr-logs` predates this change. I judged the claim from the code and the matching line number and values.

Not tested: fallback to an older eligible chat when the most recent candidate is busy; the project-scoped variant; CLI output for a skipped chat (`reused_chat: false` with a new `thread`), which passes straight through from `source_intake_chat`; a two-process lease (already recorded in the evidence as an integration follow-up).

</details>

<details>
<summary>File map</summary>

- `crates/brn-threads-app/src/intake.rs`: `busy` predicate (Working Run plus live or unknown lease) added to the reuse filter, and doc comment updated.
- `crates/brn-threads-app/tests/source_intake_chat.rs`: `request()` helper extracted, and `run()` helper plus the live-lease regression test added.
- `crates/brn/src/main.rs`: help line extended to describe the skip, and comment on `reused_chat` updated.
- `docs/work/active/daily-workflow-reliability/evidence.md`: Task A section, before/after table, verification table, limitation, U6 wording hand-off and integration-risk update.
- `docs/work/active/daily-workflow-reliability/plan.md`: item 5 recorded.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability diff`.

</details>
