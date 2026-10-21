# RunActivity implementation handoff

- Branch: `codex/run-activity-state-owner`
- Base: `7fd7142c893b7aefcbebb53953336f00dcfc3212`
- Candidate commit: `1af5a124cb0fb576adeb69231d70cd3322bbf3c4`
- Subject: `Own bounded Run activity state and transitions`

## Owned files

- `crates/brn-desktop/src/threads_run_activity.rs` (new): private Idle/Live/Ended owner; Started, Observed, Finished and Disconnected transitions; immutable step/status/chat reads; 3 focused pure transition tests.
- `crates/brn-desktop/src/threads_ui.rs`: replaces the three parallel activity fields with the owner and connects start, actual worker-thread progress/activity, finish and disconnect transitions. Existing worker cancellation/progress handling, terminal status strings, and durable effects remain in their prior paths.
- `crates/brn-desktop/src/threads_chat.rs`: delegates activity admission to the owner and scrolls only when a step is appended; renders owner steps/status.
- `crates/brn-desktop/src/threads_writing.rs`: checks the owner for retained activity belonging to the displayed chat.
- `crates/brn-desktop/src/threads_steering_tests.rs`: existing UI witness reads immutable owner state.

## Boundaries and behavior

The owner copies only chat context and bounded presentation values; it owns no Run ID, Worker, queue, cancellation, stream, or toolkit scroll state. First scoped observation can establish Live from Idle for synthetic/transport fixtures. Wrong-chat and post-terminal observations are ignored. Started resets the owner. Empty/duplicate filtering and 8,192 Unicode-scalar / 32-step bounds are retained. Finish/disconnect retain the matching chat's steps and terminal label. No broader source consumers required migration.

## Verification and limits

- Pinned Rust 1.98.1 `rustfmt --edition 2024` run on the five owned Rust files: passed.
- `git diff --check` before commit and on the committed change: passed.
- No build, tests, native checks, provider/account calls, or external actions run, per lead instruction to serialize checks. The three new tests are unexecuted and require lead-owned checks.
- Worktree clean after commit.
