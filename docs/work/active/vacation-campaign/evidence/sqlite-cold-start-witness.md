# Concurrent cold `Store::open` witness

Added one test-only public-core integration test in `crates/brn-threads-core/tests/sqlite_startup.rs` on branch `codex/sqlite-cold-start-guard`, based on `origin/main` at `aded398a0c1960d1feabe48166284ec0824ee4e1`.

`simultaneous_cold_store_opens_initialize_independent_workspaces` creates one canonical temporary parent and synchronizes 16 threads plus the caller on a barrier. Each worker's first SQLite API call is `Store::open` on its own distinct, previously absent child path. Each then asserts the canonical new-workspace state through public methods (`records()` is empty and `workspace_purpose()` is `Owner`). The test joins every worker before surfacing either a worker error or panic. It changes no production code, SQLite configuration, global test threading, dependencies, or existing tests.

This is a diagnostic witness for whether parallel cold opens through the core API can exercise the initialization boundary. It is not a presumed reproduction or proof of a SQLite defect; root cause and behavior under the original app suite remain unestablished until the lead runs it. No build or test was executed by this author. The first formatter invocation via the unqualified `rustfmt` command was unavailable on PATH; formatting then succeeded with `/Users/evokessler/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin/rustfmt --edition 2024 crates/brn-threads-core/tests/sqlite_startup.rs`. `git diff --check` passed.

Candidate commit: `4c786a962ed52998648a0ae73232c61a0b41a5e9`.

Source SHA-256: `73d0eae7229c66eaaed50dcd7340f0a75480e1b3c6e246add0c09474a3391023`.

The checkout is clean after the commit. No runtime result is claimed; lead-owned execution and independent review remain pending.
