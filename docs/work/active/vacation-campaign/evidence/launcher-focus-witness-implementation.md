# Launcher focus-return witness implementation

- Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`, branch `codex/launcher-focus-witness`; starting HEAD `2658f8883b47f224b02ed428055efaaba1bb47a4`.
- Candidate commit: `8912801c24f5f3a3ec4fb55bb8e5d74b118dba51` (`test: witness launcher focus return`).
- Changed source unit only: `crates/brn-desktop/src/threads_dialogs_tests.rs`; SHA-256 `3aef40eeddff4df6b26275f7ec79931435478ce8640519a163db300346778381`.
- Change: in the existing `escape_closes_one_layer_at_a_time_and_never_stops_work` test, immediately after Cmd+K then Escape, assert launcher closed, layout focus remains Chat, composer focus handle actually owns focus, and launcher-query focus handle does not. The existing working-run/cancellation/queue checks and remaining Escape journey are unchanged. This tests the current-source return-focus contract without deriving the expected destination from `focus_landing`.
- Scope: synthetic widget/test fixture only; its existing worker fixture and data authority were not altered. No production source, provider, dependency, or fixture changes.
- Commands: inspected repository guidance and adjacent focus-handle tests; `git diff --check` passed; `rustup run 1.98.1 rustfmt --check crates/brn-desktop/src/threads_dialogs_tests.rs` returned nonzero because pinned rustfmt reports pre-existing formatting differences elsewhere in the file (imports and several unrelated assertions); the added block had no reported formatting diff. No build or test was run, per task boundary. Commit contains only the one requested file; checkout was clean after commit.
- Limit: candidate is unqualified until lead-owned serialized checks run; this synthetic widget witness is not native interaction or OS focus evidence.
