#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-knowledge-refresh-qualification/final-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = "6746eea67ee227209bb39fb0f879c12a6d7f88b3"
test -z "$(git status --porcelain)"
python3 - <<'PY'
import shutil
assert shutil.disk_usage('.').free>2*1024**3,'Insufficient space for cached build'
PY
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop --locked --offline --jobs 1
cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline --jobs 1 -- -D warnings
cargo build -p brn-desktop --features native-ui --bin brn-desktop --locked --offline --jobs 1
cargo fmt -p brn-desktop -- --check
git diff --check
