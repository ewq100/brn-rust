#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-knowledge-refresh-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = "b0480c3b97ba67e9b582f2f7fc4dd012f7253d00"
test -z "$(git status --porcelain)"
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::tiled_tests::knowledge_check_runs_off_the_ui_thread_and_seeds_a_review_chat --locked --offline --jobs 1 -- --exact --nocapture
