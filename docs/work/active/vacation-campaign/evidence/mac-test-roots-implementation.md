# macOS MCP policy test-root implementation

Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/mac-mcp-test-roots`, base `a23f486b109c67e8d07c77800030e527b9a18002`.

Exactly one source file changed: `crates/brn-threads-core/tests/mcp_policy.rs`.

Added the file-local `root() -> tempfile::TempDir` helper using `tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap()`, matching the adjacent `tests/skill_policy.rs` pattern. Replaced the four calls to `tempfile::tempdir()` with `root()` in the MCP policy integration tests. This puts their synthetic BRN data folders beneath the canonical system temporary path and preserves the production symlink guard.

No production, assertions, scripts, environment settings, dependencies, or shared test support changed. No build/test/commit ran; `git diff --check` passed. The original macOS failure evidence remains in `/tmp/brn-restore-refusal-qualification/first-green.log`.
