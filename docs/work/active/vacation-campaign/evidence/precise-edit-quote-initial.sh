#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-quote-selector-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = "e4b9418d38a8fee547248884e5d5038c8ed3753c"
test -z "$(git status --porcelain)"
cargo test -p brn-ai --lib note_edit_tests --locked --offline --jobs 1 -- --nocapture
