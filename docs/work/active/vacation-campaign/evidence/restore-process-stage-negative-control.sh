#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-restore-process-stage-qualification/negative-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = "f4650db41dde3c1460af946aa59244f74a481671"
test "$(git diff --name-only)" = "crates/brn-threads-core/src/restore.rs"
python3 - <<'INNER'
import shutil
assert shutil.disk_usage('.').free > 2*1024**3
INNER
cargo test -p brn-threads-core --lib restore::tests::a_killed_restores_stage_is_removed_by_the_next_restore_into_that_parent --locked --offline --jobs 1 -- --exact --nocapture
