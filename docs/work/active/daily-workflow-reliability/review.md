# Review: orphaned-run recovery, one failure notice per chat, and project-scoped useful-intake chat reuse

Commit `422c9582` on local branch `work/daily-workflow-reliability` (base `34c9239d`) fixes the three top-ranked findings from the daily-workflow evaluation. First, a Run now holds an OS file lock under `derived/run-leases/` while it executes. Resume and queue dispatch interrupt any Working Run whose lock is free, so a crash, kill or quit no longer blocks every queue with no signal. Second, a new failed or interrupted result replaces the chat's earlier host terminal notices and names the owner-facing cause, so three failures leave one Blocker that still says why after restart. Third, CLI `intake --intent useful` now honours `--project` and reuses the open chat in that scope that already received the same source, so retries stop creating general chats and recording project email as general (`chat: null`). The invariants hold: every write goes through the checked owner service with expected versions and deterministic replay keys, Undo refuses the recovery, and there is no schema or dependency change.

Watch for: chat reuse can route a concurrent second supply into a chat whose Run is still live in another process, and `begin()` then fences that Run (confirmed by code trace; needs a concurrent same-source supply). The failure notice tells the owner to wait for the provider even when the cause is a tool limit or local storage (confirmed). Recovery reads its snapshot once, so two simultaneous orphans in one chat need two Resumes and leave a stray owner-direct preparation (likely; rare).

**Verdict**: APPROVED

## High-level view

The OS releases a `flock` however the process dies, so a free lock on a Working Run is good evidence of an orphan. Any doubt, such as a bad ID, a symlinked folder or an I/O error, counts as live, so ambiguity never interrupts real work. The lease is taken in `begin()` before the Working start group commits and is held in `Scope` for the Run's lifetime, so a Working Run seen by another process either holds a lease or is orphaned. One gap remains: Working Runs from a BRN build without leases look orphaned, so the desktop and CLI must ship from the same build. The evidence records this.

Recovery is one checked `owner_change` per orphan. It sets the Run to Interrupted with a fence bump and adds the standard Interrupted Blocker through the same helper `finish()` uses. Expected versions are checked on both the Run and the Thread, and the key is derived from those versions, so a retry replays and a race comes back `Stale`. Queue semantics survive. A queue-claimed orphan still hits the core's "queued instruction is claimed; explicit Resume is required" guard, and the desktop already pauses a non-empty queue on startup. Recovery runs only on Resume and Dispatch (decision D2), so an orphan with an empty queue still has no Needs-you entry until the owner continues it. This is a documented trade-off that keeps desktop fixtures stable.

`finish()` now applies one superseded-run rule to both success and failure. Questions, reviews, conflicts and unrelated Blockers survive, and the saved fixed text is still recognised. The cause text is `AiError`'s fixed Display, which carries no provider detail. The suffix "Continue when the provider is ready." is wrong for non-provider failures.

Chat reuse keys on a Run that received a Source with the same locator, filtered to open, non-archived chats whose current project equals the requested scope. Project boundaries hold in both directions, and the CLI validates `--project` (and refuses `--project` with `--thread`) before reading the source. The reuse filter ignores whether the chosen chat has a live Working Run, and `begin()` interrupts any Working Run in its chat.

Gate checks:
- Ownership is respected. The diff touches only `crates/brn-ai`, `crates/brn-threads-app`, `crates/brn` and this work folder. `crates/brn-desktop`, `docs/status.md`, `docs/architecture/*`, `agent/`, the Cargo manifests and the lockfile are untouched.
- Nothing was published. The branch has no upstream, there is no remote-tracking ref for it, and HEAD is not in `origin/main`. GitHub was unreachable from this shell, so the remote side was not queried directly.
- Test counts in the evidence match the files: `run_recovery` 4, `source_intake_chat` 1, `daily_workflow_eval` 1, CLI `tests/threads.rs` 11 (10 at base), plus 2 lease tests and 3 new `threads.rs` tests.
- The new tests are deterministic. They use no credentials (provider steps fail offline as `ReconnectNeeded`) and scripted models where a model turn is needed. The evidence records a fail-before rerun for each fix.
- The full `verify-storage.sh` run was not green: 12 `brn-threads-intake` sandbox tests failed. That crate and its only dependency (`brn-intake`) are unchanged, and the evidence reports this honestly.

<details>
<summary>Issues (7)</summary>

1. **Reuse can fence a live run (non-blocking)**: `source_intake_chat` may pick a chat with a Working Run that is live in another process, and `begin()` then interrupts it. Skip reuse candidates whose chat has a Working Run with a live lease (`run_lease::run_is_live`), and fall back to a new chat.
2. **Provider-centric failure suffix (non-blocking)**: "Continue when the provider is ready." is appended for every `AiErrorKind`, including tool-limit, time-limit, storage, index and unsafe-credential failures, and it persists after restart. Choose the suffix by kind or use neutral wording, coordinated with the UI agent.
3. **Single-snapshot recovery (non-blocking)**: two simultaneous orphans in one chat make the second recovery `Stale`. Resume stays refused until pressed again, and the unretired owner-direct preparation then appears in `candidates()`. Re-read after each applied recovery, or retire the stale operation.
4. **Destructive journey script (non-blocking)**: `cli-journey.py` runs `shutil.rmtree(ROOT)` on whatever path it is given. Refuse an existing path, or require a fresh directory under the synthetic data root.
5. **Contract docs not yet updated (integration follow-up)**: the proposed text for `threads-behavior.md`, `invariants.md` and `status.md` is only in the evidence, and plan item 4 is still open. Apply it at integration, and refresh the stale "No product code changed in this step" header in `evidence.md`.
6. **Full check not green here (integration follow-up)**: `verify-storage.sh` stopped at the `brn-threads-intake` sandbox tests in this sandboxed shell. Rerun it unsandboxed on the combined branch.
7. **Mixed builds and cross-process lease (integration follow-up)**: a lease-less build's live Runs look orphaned, and no two-process check was run. Run the combined desktop state suite and a desktop-plus-CLI check from one build, then do the pending native ⌘Q observation (not alongside the UI agent).

</details>

<details>
<summary>Details</summary>

### Run lease lifetime and the orphan guarantee

```
begin():   allocate op ──> RunLease::acquire(derived/run-leases/<run>.lock)
                       ──> apply start group (Run = Working)
                       ──> Scope { _lease } ─┬─ PreparedThreadRun
                                             ├─ watcher task (awaited after abort)
                                             └─ tools / run_model
finish():  terminal commit ──> Scope dropped ──> remove file, unlock
crash:     OS releases flock ──> Working Run + free lock = orphan
```

`Drop` removes the file before unlocking. A concurrent `run_is_live` may see the lease as free during that window. By then the terminal commit has already landed, so the recovery's expected Run version is stale and the change returns `Stale` rather than interrupting finished work. One residual case remains. If a long-lived process drops the run future mid-run, the detached watcher keeps the `Scope`, and so the lease, alive while the Run stays Working. That reproduces the pre-fix block rather than a new failure, and it clears once anything fences the Run.

### Recovery write and its snapshot (`crates/brn-threads-app/src/chat.rs:103-190`)

The recovery reads `records()` once (line 104) and builds every orphan's change from that snapshot, including the Thread's `expected_version`. `begin()` normally leaves at most one Working Run per chat. If two concurrent `begin()` calls in one chat both crash, though, the first recovery bumps the Thread and the second returns `Stale` (line 181). Resume then still says "Finish or stop current work".

The store also does not retire a stale owner-direct preparation. The `candidates()` query in `crates/brn-threads-core/src/store.rs:484` selects hashed, unretired, receipt-less operations, so an "Interrupt work whose process ended" entry stays listed. Confidence is likely: the mechanism is confirmed, and the trigger needs concurrent starts in one chat. `a_second_orphan_in_the_same_chat_leaves_one_notice` creates its orphans one after the other, so it does not cover this.

An error from `prepare_owner` (`?` on line 172), or any outcome other than Applied or Stale, fails the whole Resume or Dispatch. That is fail-closed: before the change, both were already refused in that state.

### Useful-intake chat reuse and live runs (`crates/brn-threads-app/src/intake.rs:34-103`, `crates/brn/src/main.rs:681-797`)

The reuse filter (lines 64-80) checks archived state, `ThreadState::Open` and project scope, but not whether the chosen chat has a Working Run. The CLI then calls `ask` → `brn_ai::threads::run` → `begin()`, which fences any Working Run in that chat (`crates/brn-ai/src/threads.rs:1795`) without a liveness check.

Take the desktop running a useful intake of `mari-email.txt` while an agent runs `brn intake --intent useful` on the same file. The CLI now routes into the desktop's chat and interrupts its live Run. Before the change, both Runs completed in separate chats (duplication). After it, the earlier Run's in-flight turn is lost, though its applied receipts and prepared candidates stay durable. This matches the existing "Continue supersedes the previous run" contract, except that the routing was automatic rather than the owner's choice. The lease API already supports a precise guard, so the fix is small. Confidence: confirmed by trace.

### Failure notice wording (`crates/brn-ai/src/threads.rs:2124-2127`)

`failed_attention` always appends "Continue when the provider is ready." `AiError`'s Display also covers `ToolLimitReached`, `TimeLimitReached`, `InvalidToolUse`, `ToolRejected`, `Storage`, `IndexStale`, `UnsafeCredentials` and the quote errors. For those kinds the persisted Blocker points the owner at the wrong cause, for example "Run failed: The tool round limit was reached. Continue when the provider is ready." The notice survives restart, which is now its main purpose. Confidence: confirmed. The old fixed text was also provider-centric, so this is a missed refinement rather than a regression. The wording appears in the desktop, so coordinate the final text with the UI agent.

### Test coverage gaps and the evidence script

Each regression asserts the exact persisted outcome of a demonstrated failure and would fail on the base:
- Crash then Resume: the orphan is interrupted once, the chat gets one notice and the queued item dispatches with one owner Message.
- A live lease keeps the refusals.
- Recovery replays and cannot be undone.
- CLI project-scoped retry: one project chat, a scoped Source and one Blocker.

Not tested:
- Lease exclusion across two OS processes. This is documented, and the in-process check uses separate file handles, the same `flock` mechanism.
- Two simultaneous orphans in one chat.
- Reuse while the target chat has a live Run.

`daily_workflow_eval` asserts pre-existing host enforcement: the protected change is `NeedsReview`, the human Action stays not Done, the reply survives reopen and there is no second Source. It does not exercise the three fixes. Model quality is correctly marked as not judged.

`cli-journey.py` (line 23) deletes its `ROOT` argument recursively before running. It is an evidence script, not product code, but a mistyped path would destroy whatever it names.

</details>

<details>
<summary>File map</summary>

- `crates/brn-ai/src/run_lease.rs`: new `RunLease` (flock under `derived/run-leases/`, symlink/ID refusal, remove-then-unlock on drop) and `run_is_live` (doubt counts as live), plus 2 tests.
- `crates/brn-ai/src/lib.rs`: exports `run_lease`.
- `crates/brn-ai/src/threads.rs`: lease taken in `begin()` and held in `Scope`; watcher awaited after abort; public `INTERRUPTED_ATTENTION`, `is_terminal_notice` and `replace_terminal_notice`; cause-naming `failed_attention`; `finish()` keeps one terminal notice per chat; 3 tests.
- `crates/brn-threads-app/src/chat.rs`: `recover_orphaned_runs`, called first in `resume_queue` and `dispatch_queued`.
- `crates/brn-threads-app/src/intake.rs`: `source_intake_chat` (scope-aware reuse by Source locator, or a new project/general chat).
- `crates/brn/src/main.rs`: useful intake validates `--project` before reading, uses `source_intake_chat`, and adds `thread`/`reused_chat` to the JSON; one help line.
- `crates/brn/tests/threads.rs`: CLI project-scoped useful-intake retry regression.
- `crates/brn-threads-app/tests/run_recovery.rs`: 4 recovery regressions.
- `crates/brn-threads-app/tests/source_intake_chat.rs`: reuse scope regression.
- `crates/brn-threads-app/tests/daily_workflow_eval.rs`: assertive scripted-model journey.
- `docs/work/active/daily-workflow-reliability/{plan.md,evidence.md,cli-journey.py}`: plan, evaluation, verification record and CLI journey driver.

Full diff: `git -C /Users/evokessler/repos/brn-rust/.worktrees/daily-workflow-reliability diff 34c9239d..HEAD`.

</details>
