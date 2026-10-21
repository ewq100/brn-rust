#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-run-continuation-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
qualification_source="$1"
test "$(git rev-parse HEAD)" = "$qualification_source"
test -z "$(git status --porcelain)"
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::worker::tests --locked --offline --jobs 1 -- --nocapture
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::steering_tests --locked --offline --jobs 1 -- --nocapture
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop --locked --offline --jobs 1
cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline --jobs 1 -- -D warnings
cargo build -p brn-desktop --features native-ui --bin brn-desktop --locked --offline --jobs 1
cargo fmt -p brn-desktop -- --check
git diff --check
