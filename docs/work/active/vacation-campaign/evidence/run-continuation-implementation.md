# Run continuation ownership implementation

Candidate: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/run-continuation-owner`, based on `f6dd4d6666320437d631759440b68f2b43d33a9d`.

Implemented the selected Run-owned continuation boundary in `crates/brn-desktop/src/threads_worker.rs`, `threads_ui.rs`, `threads_chat.rs`, and `threads_projects.rs`, with existing steering test migrations and one worker invariant witness in `threads_steering_tests.rs` / `threads_worker.rs`.

`WorkerOperation::Run` now owns only an optional saved prompt alongside its existing `RunIdentity`. Narrow methods arm without overwriting, query, discard, and consume it once while deriving the destination thread from that identity. `ThreadsUi.pending_send` / `PendingSend` were removed. Stop-and-send still commits to Workspace before arming, clearing the composer, and cancelling. Terminal extraction occurs in the old worker's Finished handling, while the prompt stays poll-local until the admitted batch is complete. Cancel, successful matching archive/project fencing, disconnect, close and Quit follow their distinct discard ordering.

Checks performed: source inspection, `rg` confirming no remaining `pending_send` field/type references, and `git diff --check` passed. Per assignment, no formatting, Cargo command, build, or test was run because the shared build slot is occupied and the lead owns final qualification.

Limitations: compile and test status remain unverified. Existing native interaction and owner acceptance remain outside this source-only candidate.

## Lead-requested source correction

The reviewed candidate was adjusted before qualification. `Worker::continuation()` now exposes an immutable `Option<&str>` so the existing steering widget assertion still checks the exact armed prompt; no mutable or test-only accessor was added. Successful archive again discards the current worker continuation unconditionally, matching the original `pending_send = None` behavior, while cancellation remains limited to a matching thread. The earlier matching-only discard was an incorrect inference from the design mapper and would have changed unrelated-archive behavior; that change is corrected here. Duplicate-arm preservation is also asserted in the worker witness.

Correction commit: see the final report from the author for its exact HEAD. No formatting/build/test command was run; `git diff --check` passed.
