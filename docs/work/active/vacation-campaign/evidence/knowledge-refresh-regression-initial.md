# Knowledge refresh regression author handoff

- Checkout: `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`
- Branch: `codex/knowledge-check-refresh`
- Base: `f79f6540276c00155ab3bc0d7f019f4ea5d00c9e`
- Commits: `041e9a3320e22b809fd6b386e6402485b8193cb8`, followed by the test-harness correction recorded in this checkout's HEAD.
- Changed file: `crates/brn-desktop/src/threads_tiled_tests.rs` only
- Test: `knowledge_check_runs_off_the_ui_thread_and_seeds_a_review_chat` (extended in place; no test added)

The first actual run compiled and failed before the stale-state assertion because `knowledge-finding-0` is a raw `div` ID without `.test_support()`, so GPUI did not register it for lookup. This is a test harness failure, not evidence of the production regression. The correction removes every lookup of that unregistered ID and removes the extra initial visibility block. The existing DuplicateTitle count and real Review findings chat assertions remain unchanged.

The replacement journey archives one synthetic duplicate through `Workspace::owner_change` with its current expected version, confirms registered old summary/review controls exist, reveals `knowledge-run` last, and checks the real control is visible. It clicks that control and immediately asserts `Checking…` plus absence of old summary and Review findings before draining the worker. After completion it checks the current result has one fewer active note, unchanged link count, and no `DuplicateTitle`, and verifies that only the intended archived note changed in durable records.

The lead owns the second actual attempt. This author did not run builds or tests after the correction. Pinned Rust 1.98.1 rustfmt formatted the changed file and `git diff --check` passed. No production code changed.
