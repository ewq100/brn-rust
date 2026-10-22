#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-restore-process-stage-qualification/focused-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = "2f3f65c884d18919a5553b18b7bbe72ccf0962a5"
test -z "$(git status --porcelain)"
python3 - <<'INNER'
import shutil
assert shutil.disk_usage('.').free > 2*1024**3
INNER
cargo test -p brn-threads-core --lib restore::tests::a_killed_restores_stage_is_removed_by_the_next_restore_into_that_parent --locked --offline --jobs 1 -- --exact --nocapture
