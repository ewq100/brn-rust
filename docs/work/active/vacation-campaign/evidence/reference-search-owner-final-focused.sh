#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-reference-search-owner-qualification/tmp
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = "dee96c9ed4542eb3c8b595280116628580dc896a"
test -z "$(git status --porcelain)"
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::writing_tests::b3_delayed_scoped_search_reply_cannot_publish_titles_or_counts_into_another_form --locked --offline --jobs 1 -- --exact --nocapture
cargo fmt -p brn-desktop -- --check
git diff --check
