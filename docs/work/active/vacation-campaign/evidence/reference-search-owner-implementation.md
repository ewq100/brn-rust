# Reference-search request owner implementation handoff

Candidate was implemented in `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust` from the selected frozen base `28fd581ea8451ecd95d899248132b62c8f890d03` on branch `codex/reference-search-request-owner`.

## Change

- Added private `ReferenceSearchRequest` in `crates/brn-desktop/src/threads_ui.rs` with exactly the existing generation and `Option<CancelOnDrop>`.
- `invalidate()` drops the guard, increments with existing `+= 1` behavior, and returns the generation. Recheck, replace, search admission and Close use it.
- `retain_cancel()` stores the newly admitted ticket guard immediately after the existing synchronous service acquisition.
- `finish(generation)` preserves a newer guard on stale completion; a matching completion clears it even if no guard is present, matching prior behavior. Existing target/observed matching, exact retained-draft pending retirement and current/displayed-only publication remain in the existing completion method.
- Adapted the existing delayed scoped-search witness to inspect the private owner's guard and use `invalidate()` for controlled generations. No new test, state, getter or public API was added.

Only `threads_ui.rs` and its existing `threads_writing_tests.rs` were changed. No behavior bug is claimed. Query, scope, paging, errors, draft state, target/token gates and toolkit tasks remain in their prior owners.

## Checks and limitations

Used the pinned formatter from `/tmp/brn-launcher-focus-qualification/environment.sh` on the two changed Rust files. `git diff --check` passed. No build, tests, network, account, provider or native checks were run, by instruction. The existing hidden-search retirement proof remains prior evidence and was not repeated.

Lead should review the diff, then run the existing focused delayed scoped-search test and relevant desktop checks on the actual reconciled integration head. In particular, confirm stale generation still leaves the replacement guard/pending state untouched, the retained hidden draft still retires pending, and stale canonical state still cannot publish. Native qualification and owner acceptance remain unclaimed.
