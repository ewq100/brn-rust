#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-precise-edit-trace-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust
test "$(git rev-parse HEAD)" = "c4f0e2e663cb04654c2bc49f9a540499fdd139b9"
test -z "$(git status --porcelain)"
cargo test -p brn-ai --lib threads::trace::tests --locked --offline --jobs 1 -- --nocapture
cargo test -p brn-ai --lib --locked --offline --jobs 1
cargo clippy -p brn-ai --all-targets --locked --offline --jobs 1 -- -D warnings
cargo build -p brn --locked --offline --jobs 1
cargo fmt -p brn-ai -- --check
git diff --check
