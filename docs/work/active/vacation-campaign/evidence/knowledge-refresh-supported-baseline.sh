#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-knowledge-refresh-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = "78da4ac26803c16585eedf5d99ad1607d674529c"
test -z "$(git status --porcelain)"
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::tiled_tests::knowledge_check_runs_off_the_ui_thread_and_seeds_a_review_chat --locked --offline --jobs 1 -- --exact --nocapture
