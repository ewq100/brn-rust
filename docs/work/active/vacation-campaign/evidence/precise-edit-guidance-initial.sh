#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-precise-edit-guidance-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust
test "$(git rev-parse HEAD)" = "6b719f6e29812a48f67e16f8fd3770e5e83c0fe5"
test -z "$(git status --porcelain)"
cargo test -p brn-ai --lib guide_bundle_change_recomputes_prior_stamp_summary_then_reuses_unchanged_stamp --locked --offline --jobs 1 -- --nocapture
