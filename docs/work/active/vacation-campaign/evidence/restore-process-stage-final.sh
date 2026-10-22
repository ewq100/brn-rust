#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-restore-process-stage-qualification/final-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = "f4650db41dde3c1460af946aa59244f74a481671"
test -z "$(git status --porcelain)"
python3 - <<'INNER'
import shutil
assert shutil.disk_usage('.').free > 2*1024**3
INNER
cargo test -p brn-threads-core --locked --offline --jobs 1
cargo clippy -p brn-threads-core --all-targets --locked --offline --jobs 1 -- -D warnings
cargo fmt -p brn-threads-core -- --check
git diff --check
