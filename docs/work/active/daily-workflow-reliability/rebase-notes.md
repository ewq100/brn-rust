# Rebase onto main (`771116f0`, helper sub-agents)

**Date:** 15 October 2026. Local only: nothing pushed, merged or released.

- **Backup ref:** `work/daily-workflow-reliability-pre-rebase` → `914e2e6f` (the branch before rebase, based on `34c9239d`).
- **New base:** `771116f0` ("Merge pull request #145 from ewq100/work/subagent-design"), which is `origin/main` and the UI branch's base.
- **Rebased commits:** `422c9582` → `9f70efa7`, `ee53e62e` → `14ad8577`, `dfe698aa` → `2530805f`, `ab338d05` → `f80554fe`, `914e2e6f` → `b49fb660`. Follow-up: `145b8884` (helper lease test).

## Conflicts

| File | Conflict | Resolution |
|---|---|---|
| `crates/brn-ai/src/threads.rs` | `Scope` fields and the `begin()` constructor: main added `helper`/`scout`; this branch added `_lease` | Both kept. The lease field is now `lease: Arc<RunLease>` so a helper's child scope can share it |
| `crates/brn-ai/src/threads_research.rs` | (no textual conflict) `child_scope` builds a `Scope` and needed the new field | `lease: parent.lease.clone()` |
| `crates/brn/src/main.rs` (twice) | `HELP` text: main added the `--helper` options to `configure`; this branch added the useful-intake reuse line (extended by Task A) | Both kept: main's text plus the two-line Task A reuse note |

No other files conflicted. `crates/brn-desktop` was not edited.

## Helper sub-runs and the lease, notice and intake fixes

Checked by reading `threads_research.rs` at `771116f0`:

- **No Run records.** A helper sub-run is a nested model call inside the parent's `research` tool call. `child_scope` reuses the parent's chat, Run ID, fence, store and search. It never calls `begin()` or `finish()` and writes nothing, so it creates no Working Run that `recover_orphaned_runs` could see.
- **Live parent with live helpers.** The parent `Scope` holds the lease from `begin()` until the Run ends, and each helper scope holds a clone of the same `Arc<RunLease>`. While any helper is in flight the Run stays live, so Resume and Dispatch keep refusing it rather than interrupting it.
- **Crashed parent.** The process exit releases the `flock`, whatever helpers were running. The Run is Working with a free lease and is recovered as before.
- **Terminal notices.** Helper outcomes return as tool results to the parent, and an unavailable helper adds a transient `ThreadResult.notices` entry (CLI JSON `notices`, the desktop finish line). Neither touches chat attention. Only the parent's `finish()` writes the terminal notice, and helper failures do not fail the parent.
- **Intake.** `source_intake_chat` and the CLI useful-intake path are independent of helpers. The `research` tool is registered only when the owner has chosen a helper, so offline tests run with it off.

Regression: `threads::research_tests::a_helper_sub_run_holds_its_parent_lease_and_saves_no_run`. From inside a scripted helper's `search_notes` call, it records that the parent lease is held and that the parent Run is the only Working Run. After the scope drops without finishing (a crash), the lease is free and the Run is still Working, so it is recoverable. It guards the contract rather than reproducing a defect: before the `Arc` change the branch did not compile on the new base, so there is no fail-before.

## Verification after rebase

Run from the worktree with `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/tmp/brn-dwr-target`, rustc 1.98.1, `--locked --offline`. All checks are automated: no Mac UI was driven and no provider calls were made.

| Command | Result |
|---|---|
| `cargo fmt --all --check` | passed |
| `cargo build -p brn-intake --features helper --bin brn-intake-helper` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test --workspace --no-fail-fast` | **702 passed, 11 failed, 4 ignored** |
| `cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop` | **147 passed**, 0 failed (headless state) |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | passed |

Parity with the pre-rebase run (670 passed, 12 failed, 4 ignored):

- **+30 passing tests from main:** 29 `research_tests` in brn-ai and 1 CLI test in `crates/brn/tests/threads.rs` (now 12).
- **+1 passing test from this step:** the helper lease test. brn-ai is now 336 passed, 1 ignored.
- **+1 intake test passing, −1 failing:** `cancellation_after_pdf_copy_removes_raw_input_before_return` failed before the rebase and passes now.
- **Unchanged suites:** `run_recovery` 4, `source_intake_chat` 2 and `daily_workflow_eval` 1 all pass.

All 11 failures are the known `brn-threads-intake` sandbox tests: `process::tests` 3 and `tests/qualification.rs` 8. This agent shell is itself sandboxed, so `sandbox-exec` cannot apply a profile. Neither `brn-threads-intake` nor `brn-intake` changed on main or on this branch. The cancellation test that now passes is in the same sandbox-dependent group; its result varies by run in this shell. There are no other failures.

## Integration notes

- The UI branch (`work/ui-polish`) starts from the same `771116f0`, so the remaining overlap is the shared runtime seams already listed in the [plan](plan.md#integration-risks-check-both-branches-together-later).
- Main adds a helper choice to desktop Settings. Desktop and CLI must still ship from one build, because a lease-less build's live Runs look orphaned.
- `ThreadResult.notices` is a second owner-facing channel, separate from the persisted terminal notice. Helper-unavailable text is never persisted, so it does not survive a restart.
