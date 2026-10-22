#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-quote-selector-qualification/final-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = "28fd581ea8451ecd95d899248132b62c8f890d03"
test -z "$(git status --porcelain)"
python3 - <<'PY'
import shutil
assert shutil.disk_usage('.').free > 2*1024**3, 'Insufficient free space for cached graph'
PY
cargo test -p brn-ai --features scripted-runs --lib --locked --offline --jobs 1
cargo clippy -p brn-ai -p brn --features brn-ai/scripted-runs --all-targets --locked --offline --jobs 1 -- -D warnings
cargo clippy -p brn-ai -p brn --all-targets --locked --offline --jobs 1 -- -D warnings
cargo test -p brn --test agent_runs scoped_selected_skill_loads_exact_files_and_run_details_hide_prior_scope_after --locked --offline --jobs 1
cargo build -p brn --locked --offline --jobs 1
cargo fmt --all --check
python3 scripts/check-markdown-links.py
git diff --check
