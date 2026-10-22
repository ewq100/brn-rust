#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-backup-process-bounds-negative
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
cargo test -p brn-threads-core --lib --locked --offline --jobs 1 backup::tests::a_sigkilled_backup_process_leaves_a_stage_the_next_backup_removes -- --exact --nocapture
