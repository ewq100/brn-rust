#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-note-reload-generation-final
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = d521722ab984eb2fd125a990b9d4e13956138717
test -z "$(git status --porcelain)"
cargo test -p brn-writing-ui --features native-ui,native-test-support --all-targets --locked --offline --jobs 1
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline --jobs 1
cargo clippy -p brn-writing-ui -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline --jobs 1 -- -D warnings
cargo build -p brn-desktop --features native-ui --locked --offline --jobs 1
cargo fmt --all -- --check
python3 scripts/check-markdown-links.py
git diff --check
test "$(git rev-parse HEAD)" = d521722ab984eb2fd125a990b9d4e13956138717
test -z "$(git status --porcelain)"
