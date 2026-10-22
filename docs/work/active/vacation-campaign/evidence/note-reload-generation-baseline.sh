#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-note-reload-generation-baseline
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = 28bb965c113b0896baabfe89f9d1008ebba14a8e
test -z "$(git status --porcelain)"
cargo test -p brn-writing-ui --features native-ui,native-test-support --example writing_host note_newer_buffer_generation_refuses_save_and_discard_until_explicit_recovery --locked --offline --jobs 1 -- --nocapture
