# Clean historical Note Close witness — author handoff

- Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`
- Branch: `codex/note-exit-preservation-20261022`
- Base: `660a737174030320ca4c412248117a168e69c8d6`
- Candidate commit: `7a1a530f11ed7c06331b4439f4e92b2086b641be`
- Final source SHA-256: `crates/brn-desktop/src/threads_writing_tests.rs` = `d199d1ca7a4788ff79b1fbcb552ad8bc9e71579b91df1bcb4d8b4b39837791da`
- Only changed path: `crates/brn-desktop/src/threads_writing_tests.rs` (138 inserted lines).

Added `threads_ui::writing_tests::normal_close_accepts_a_clean_historical_checkpoint_without_writing`. It uses the existing desktop fixture and actual `install_exit_handlers`, opens fixture Note A without typing, and advances it from a second `Workspace` using checked `begin_edit`, `update_buffer`, and `save`. The witness invokes the production `view.refresh()`, then proves the displayed/lifecycle checkpoint remains the original saved Record while the refreshed list/current Store show the newer Record; it also checks the old immutable revision, absence of an open recovery edit, closed external session and exact history. The installed headless Close predicate is expected from source inspection to refuse at the test's `simulate_close()` assertion on this base. That is a predicted source-level red, **not an observed test run**.

If accepted by the lead, the focused command is:

```sh
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop normal_close_accepts_a_clean_historical_checkpoint_without_writing --locked --offline -- --exact threads_ui::writing_tests::normal_close_accepts_a_clean_historical_checkpoint_without_writing
```

Formatting/diff checks actually run: pinned Rust 1.98.1 `rustfmt --edition 2024 --check crates/brn-desktop/src/threads_writing_tests.rs` and `git diff --check`, both passed. No build or test was run. `VisualTestContext::simulate_close()` checks the installed predicate only; it does not establish native teardown. Candidate is committed and worktree clean.
