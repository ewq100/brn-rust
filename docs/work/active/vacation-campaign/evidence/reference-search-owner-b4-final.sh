#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-reference-search-owner-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = "b4b441a3ba51772736e3c15856f49834f3367c41"
test -z "$(git status --porcelain)"
python3 - <<'PY'
import shutil
assert shutil.disk_usage('.').free>2*1024**3,'Insufficient space for cached build'
PY
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::writing_tests::b3_delayed_scoped_search_reply_cannot_publish_titles_or_counts_into_another_form --locked --offline --jobs 1 -- --exact --nocapture
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::writing_tests::b3_reference_next_keeps_its_query_until_owner_starts_a_new_search --locked --offline --jobs 1 -- --exact --nocapture
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop --locked --offline --jobs 1
cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline --jobs 1 -- -D warnings
cargo build -p brn-desktop --features native-ui --bin brn-desktop --locked --offline --jobs 1
cargo fmt -p brn-desktop -- --check
git diff --check
