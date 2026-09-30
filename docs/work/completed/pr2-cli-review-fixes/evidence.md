# Evidence: PR #2 review fixes

Tested commit: `14696b3` on `feature/agent-cli-foundation` (working tree clean; base `df47114` = PR #2 reviewed head; remote branch still at `df47114` at check time — commits not pushed). Environment: macOS arm64 (target Mac), rust 1.98.1 pinned toolchain, offline Cargo, no native features, fake python providers and synthetic CODEX_HOME only. No credentials, live Codex calls, or model downloads.

Baseline before changes (measured fresh at `df47114`): fmt clean, build clean, clippy `-D warnings` clean, `cargo test --workspace --locked` **157 passed / 0 failed**.

## Finding disposition

| Finding | Status | Fix commit | Regression evidence |
| --- | --- | --- | --- |
| R1 SIGPIPE bypasses provider cleanup | Confirmed; fixed | `cb1c454` | Lead reproduced live at baseline: fake provider closing its stdin read end (`os.close(0)`) after answering `initialize` → `brn ask` died by signal 13 (shell exit 141), no envelope, provider child left alive. After fix (lead, live): exit 1, `WORKFLOW_ERROR` envelope with context, child reaped. Test `broken_provider_pipe_is_recoverable_and_reaps_child` red on baseline (signal death), green after. |
| R2 late SIGINT misreports committed operations | Confirmed; fixed | `cb1c454` | Source-confirmed: `escalate_interrupt` converted any Ok non-ask result + flag → INTERRUPTED, discarding committed import/approval IDs; unit test blessed it (deleted). Red on baseline via new unit test (store created then 130). New: pre-dispatch refusal creates no `brn.sqlite3`; `finish` preserves completed results with the flag set. |
| R3 help checked after required arguments | Confirmed; fixed | `af39233` | Lead reproduced at baseline: `brn ask --help` → exit 2 "missing ask QUESTION"; `brn documents --help` → missing subcommand; `brn revisions diff --help` → missing `--draft`. Red: 2 new tests failed exit 2. Green after pre-scan; 16-invocation matrix + no-files-in-data-dir + unchanged-usage guards. |
| R4 ask failures lack structured context | Confirmed; fixed | `b6552a2` | Source-confirmed: IDs only in prose; new-conversation session id lost on error paths. Red: 9 CLI context assertions failed (missing `error.context`). Green incl. timeout-after-submission shape `{operation_id, session_id: <created>, recorded_status: "interrupted", provider_outcome: "unknown"}` with exit 124 (lead re-ran this live); persisted turn matches context ids; same-operation retry does not resubmit (`submitted.log` stays one line). |
| R5 codes from string matching | Confirmed; fixed | `cf09dc0` | Source-confirmed: `classify_workflow` used `starts_with` on English prefixes. Fixed: typed `ErrorKind` from store/retrieval/workflow variants; unit tests prove wording changes and look-alike `Error::Invalid("data directory is already owned: …")` text do not change the code; unknown stays `WORKFLOW_ERROR`. Existing subprocess code tests (WORKSPACE_BUSY, INDEX_MISSING/STALE/INVALID, PROFILE_UNAVAILABLE, OPERATION_CONFLICT) stayed green throughout. |

Implementation credits: packages A (R3), B (R1+R2), C1 (R5), C2 (R4) and the final doc/dead-code pass were implemented by GLM 5.3 Flash subagents under GLM 5.3 lead direction (integration, shared contracts, safety review of signal/cancellation/outcome semantics, verification). A fresh GLM 5.3 Flash agent independently reviewed the integrated diff: verdict APPROVE, R1–R5 CONFIRMED-FIXED, seven non-blocking minors (A1/A2/A3/A6 fixed in `14696b3`; A4/A5/A7 parked with rulings below).

## Verification results (all at `14696b3`, run by the lead unless noted)

- `cargo fmt --all -- --check` — pass.
- `cargo build --workspace --locked` — pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass.
- `cargo test --workspace --locked` — **178 passed / 0 failed** (baseline 157/0; net +21 tests).
- `bash scripts/verify-end-to-end.sh` (workspace checks offline + provider-free brn-flow fixture) — pass, including brn-flow behavior/output unchanged.
- `git diff --check` — clean; working tree clean.
- Live on target Mac: broken provider pipe → structured error + reaping; `brn ask --help` → help exit 0; timeout envelope carries context matching persisted state; provider reaped after timeout.

Per-crate implementer runs (same tree, before the final docs-only commit): brn 47/0 then 56/0 after C2; workspace 168/0 after C1, 178/0 after C2. Reviewer independently ran `cargo test -p brn --locked` (56/0) and `cargo test -p brn-workflow --locked` (21/0).

## Rulings and parked findings

- A4: no subprocess test for SIGINT-during-`index build` exit 130 — no deterministic mid-build seam exists without new test hooks; covered by unit tests and `flow.rs` cancelled-build coverage. Parked.
- A5: TIMEOUT/INTERRUPTED prose messages name the caller's `--session` (compat-frozen strings asserted by brn-flow-era tests) while `context.session_id` carries the actually-established session. By design; context is the machine-readable truth.
- A7: one avoidable `AskFailure` clone on the CLI error path. Parked (churn > value).
- `provider_outcome: "interrupted"` is documented as reserved but unreachable: an interrupted record is ambiguous and reports `"unknown"`; a same-run server-confirmed interruption is also reported `"unknown"` (stateless honesty rule — avoids a second Ok-path return type).

## Limitations

- Signal/process tests are unix-only (consistent with the existing suite). Linux was not used; results above are native macOS (arm64).
- Late-SIGINT-after-commit preservation is covered by the deterministic unit test on the dispatch path, not a subprocess race test (import commits too fast to signal deterministically from outside).
- Native retrieval, live-provider behavior, publication and user acceptance remain unqualified, as before.
- The five commits are local only. Push when authorized: `git push origin feature/agent-cli-foundation` (fast-forward; no force). PR #2 left open and unmerged; no independent reviewer approval of these fixes is claimed beyond the recorded Flash review.

## Second pass: F1–F3 (follow-up review findings)

Tested code head: `9b33058` on `feature/agent-cli-foundation` (base `a1c06f8` = PR #2 head at pass start; first-pass commits `cb1c454..14696b3` unchanged beneath). Environment: macOS arm64 (target Mac), rust 1.98.1 pinned toolchain, offline Cargo, no native features; synthetic tempdir workspaces, in-process lock holders and lsof observation only — no credentials, live providers, model downloads or real vault access. Baseline re-measured fresh at `a1c06f8`: fmt/build/clippy clean, workspace tests **178 passed / 0 failed**.

### Finding disposition

| Finding | Status | Fix commit | Regression evidence |
| --- | --- | --- | --- |
| F1 cancellation after workspace wait still mutates | Confirmed; fixed | `7fb9fdd` | Lead live-reproduced at baseline: lock held externally, `brn import` provably waiting (lsof-confirmed open `brn.owner.lock` fd), SIGINT delivered, lock released inside the retry window → child imported anyway (exit 0, ok:true, source persisted). Fix: post-acquisition CANCEL recheck in `cli/retrieval.rs::run` + typed pre-mutation guards in `Workspace::import_file` (entry and pre-`import_text`) and `set_approval`, matching the existing `build_index` pattern (all callers updated, incl. worker and brn-flow driver, which pass an always-false flag). Subprocess tests (cli_cancel.rs): import and approval both → exit 130, INTERRUPTED envelope, no persisted mutation; workflow flag-guard test in flow.rs; completed-mutation-preservation test through the real `execute` path in main.rs tests. Red for the import subprocess test observed on unfixed code (exit 0 + persisted import). |
| F2 every stdout write/flush error ignored | Confirmed; fixed | `9b33058` | Source-confirmed (`let _ =` on all output paths). Fix: `cli/out.rs` `deliver` (write+flush; BrokenPipe → quiet success) + injectable `finish_ok_to`/`report_error_to` cores. Non-pipe delivery failure of a completed result → exit 1 + stderr `OUTPUT_DELIVERY_ERROR` diagnostic stating the operation completed but its result could not be delivered, with `operation_id` when the data carries one; never claims the operation failed or suggests retry; nothing further written to stdout. Error path keeps its original exit code when reporting also fails; help/version share the policy; stderr progress/delta untouched; SIGPIPE disposition untouched. 14 injected-writer tests (partial write, flush error, pipe-vs-other on both paths, wording, committed-import-not-misdescribed with persisted-state recheck). Unit tests green-by-construction on the new seam (old code had none); no pre-fix red observable without it — stated honestly. |
| F3 every try_lock failure labeled WORKSPACE_BUSY | Confirmed; fixed | `9e83142` | Source-confirmed. Pinned-toolchain API verified by probe: `File::try_lock` returns `Result<(), std::fs::TryLockError>` with variants `WouldBlock` (contention) and `Error(io::Error)` (lock I/O failure). Fix classifies by variant only: WouldBlock keeps the existing bounded 1s/5ms retry → WorkspaceBusy; `Error(e)` returns `Error::Io(e)` immediately (no retry, never "already owned"); store Io → workflow Other → CLI WORKFLOW_ERROR exit 1. 5 injected-closure tests incl. variant-not-text (an Error whose message says "would block" stays Io). Green-by-construction on the new helper — forcing a real non-WouldBlock lock I/O error deterministically is not portable; stated honestly. |

`7d06a85` (lead fix, test-only): the two cli_cancel subprocess tests flaked under full-suite load — a single lsof sweep can outlast the child's ~1s acquisition window. Retry-with-fresh-child on missed observation is sound because the child provably cannot pass `Store::open` while the holder lives; the two tests are serialized and every wait is bounded.

### Verification (all at `9b33058`, run by the lead on the integrated branch)

- `cargo fmt --all -- --check` — pass.
- `cargo build --workspace --locked` — pass.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass.
- `cargo test --workspace --locked` — **201 passed / 0 failed** (baseline 178/0 at `a1c06f8`; net +23).
- Targeted: `-p brn` bin unit 20/0 (incl. out.rs + main.rs delivery tests), cli_cancel 2/2 (stable across repeated runs), cli_signals 3/3 (closed stdout/stderr and broken provider pipe unchanged), cli_ownership 1/1, cli_ask 18/18, cli_basic 11/11, cli_core 10/10, cli_review 8/8; `-p brn-store` 68/0 (incl. 5 new lock-classification tests and existing cross-process exclusion); `-p brn-workflow` 22/0 (flow 9/0 incl. new guard test).
- `bash scripts/verify-end-to-end.sh` — pass (brn-flow fixture behavior unchanged).
- `git diff --check` — clean; working tree clean.

### Second-pass review

A fresh GLM 5.3 Flash agent (no implementation involvement) reviewed `a1c06f8..9b33058` read-only: verdict APPROVE, no blocking findings. Five non-blocking findings with lead rulings: (1) residual load-dependent flake window if lsof spots the fd at the very end of the child's retry window — accepted and documented; outcome assertions stay strict so a real regression can never retry its way to green; (2) hard lsof dependency in cli_cancel — accepted, consistent with the existing suite's pgrep dependency (unix/macOS-targeted); (3) negative "failed"-wording assertion spans the embedded io error text — accepted, the embedded text is fully controlled by the scripted test writers; (4) SeqCst vs Acquire load-ordering split between CLI and workflow cancellation checks — pre-existing and semantically fine; (5) docs update — done in this commit.

### Implementation credits and limitations

F3, F1 and F2 were implemented by GLM 5.3 Flash subagents (`zai/glm-5.3-flash`) under GLM 5.3 lead direction (root-cause validation incl. live F1 reproduction and TryLockError API probe, boundary/ownership decisions, integration, the test-robustness fix, safety review, verification). F1 and F3 ran concurrently in isolated git worktrees with disjoint file ownership; F2 ran on the integrated branch.

- Signal/process tests are unix-only and require `lsof` (macOS verified). No Linux results are claimed.
- The pre-mutation guard window between the final cancellation check and the SQLite commit is a signal race that would require signal-blocking around the transaction to close; per the documented dispatch policy, a mutation that commits stands. Import commits source+operation atomically, so no torn state exists.
- Non-pipe stdout delivery failure is covered by injected-writer unit tests plus the real `execute`-path persistence test; there is no portable macOS way to force a real non-pipe stdout failure in a subprocess (no /dev/full) — no such end-to-end subprocess test is claimed.
- F3's non-contention lock I/O path is exercised through the injected-closure helper; a genuine OS-level lock I/O failure was not reproduced.
- Commits remain local at doc-writing time; push when authorized: `git push origin feature/agent-cli-foundation` (fast-forward; no force). PR #2 left open and unmerged.

## Third pass: T1–T2 (test-only cleanup findings)

Starting head `2e0467a` (PR #2 head; matches remote). Fix commit `9ccf664` (code) plus this documentation commit; no production code changed — the diff touches only the `#[cfg(test)]` module of `crates/brn/src/main.rs` and `crates/brn/tests/cli_cancel.rs`. Environment: macOS 26.5 arm64 (target Mac), rust 1.98.1 pinned toolchain (default matches pin), `--locked`, no native features, disposable tempdir fixtures; no credentials, live providers or model downloads.

| Finding | Status | Fix commit | Change |
| --- | --- | --- | --- |
| T1 process-global CANCEL not isolated in every test | Confirmed; fixed | `9ccf664` | `failed_delivery_after_completed_import_exits_1_and_keeps_result` called `cli::execute()` with no synchronization while sibling tests flip the flag: a parallel `CANCEL=true` made its import return interrupted and the test fail nondeterministically. All three CANCEL-sensitive bin tests now take a test-only RAII `CancelTestGuard` that acquires the existing `CANCEL_TESTS` mutex, installs the required initial value and restores the prior one on `Drop` (panic-safe; restore happens while the lock is still held). No `--test-threads=1`, no sleeps, no global resets, no production cancellation changes. |
| T2 `ChildGuard::wait_output` leaks the child on timeout/panic | Confirmed; fixed | `9ccf664` | The old helper took the child out of the guard before the bounded wait, disarming kill-on-drop. New `wait_output_with_timeout(Duration) -> Result<Output, TimedOut>` polls through `self.0.as_mut()`, takes the child only after `try_wait` confirms exit, and on timeout returns `Err` so dropping the guard SIGKILLs and reaps the still-owned child before the caller sees the error; `wait_output()` keeps the 15s default (panic on timeout, child already reaped). A `child.wait()` after the confirmed exit satisfies `clippy::zombie_processes` (returns the cached status). |

Regression coverage added: `wait_timeout_kills_and_reaps_the_owned_child` (cli_cancel.rs) spawns a disposable `sleep 30` child under a 200ms injected bound and proves the exact owned pid is killed **and** reaped (`kill(pid,0) == -1` + `ESRCH`; a zombie would still answer 0). The test cannot leak on its own assertion failures: cleanup completes inside `wait_output_with_timeout` before any assertion runs.

### Verification (at `9ccf664`, run by the lead; fresh results, earlier sections not rerun)

- `cargo test -p brn --bin brn --locked` — 20/0; `cargo test -p brn --test cli_cancel --locked` — 3/0.
- `for i in $(seq 1 30); do cargo test -p brn --locked; done` under normal parallel execution — **30/30 runs passed**, 74 tests per run (bin 20, cli_ask 18, cli_basic 11, cli_cancel 3, cli_core 10, cli_ownership 1, cli_review 8, cli_signals 3). No suite forces single-threaded execution.
- Leak check after the loop (read-only): no `sleep 30` and no `target/debug/brn` processes remain; unrelated long-running node services on this machine were untouched.
- `cargo fmt --all -- --check`, `cargo build --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings` — pass.
- `cargo test --workspace --locked` — **202 passed / 0 failed** (prior pass 201/0 at `9b33058`; net +1 = the new regression test).
- `bash scripts/verify-end-to-end.sh` — pass (brn-flow fixture unchanged). `git diff --check` — clean.

Third-pass review: a fresh GLM 5.3 Flash agent reviewed the uncommitted diff read-only (lock coverage across all CANCEL uses crate-wide, panic-safe restoration, every timeout/panic path guarded, regression-test leak safety, production-unchanged check): verdict **APPROVE**, no blocking findings. Non-blocking with lead rulings: (1) test 1's mid-test `CANCEL.store(true)` is now redundant with the guard's installed value — kept as an explicit statement of sub-case (a)'s precondition; (2) theoretical pid-reuse between reap and the ESRCH probe can only fail the test, never false-pass — accepted, same exposure as the pre-existing lsof observation tests.

Implementation credits: T1+T2 and the regression test were implemented by a GLM 5.3 Flash subagent (`zai/glm-5.3-flash`) under GLM 5.3 lead direction (root-cause validation, design, integration, rulings, verification). Commits remain local at doc-writing time; push when authorized: `git push origin feature/agent-cli-foundation` (fast-forward; no force). PR #2 left open and unmerged.

Limitations: signal/process tests remain unix-only (macOS verified; no Linux claimed); the regression test proves the guard's timeout cleanup, not new brn runtime behavior (subprocess cancellation behavior unchanged from the second pass); the 30× repeat loop is strong evidence of isolation on this machine, not a formal proof.

## Closure (2026-09-30)

Branch `feature/agent-cli-foundation` (through `f064ce3`) merged to `main` as PR #2, merge commit `1b49378`. Earlier statements above that the commits were local-only and PR #2 was open describe the state at their writing time; integration state is as recorded here.
