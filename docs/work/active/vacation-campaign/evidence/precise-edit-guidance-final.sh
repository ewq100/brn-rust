#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-precise-edit-guidance-qualification/tmp
cd /Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust
test "$(git rev-parse HEAD)" = "eac295b14ddb4a57b1e3b62e6c51f0d40ae699a5"
test -z "$(git status --porcelain)"
python3 - <<'PY'
import shutil
assert shutil.disk_usage('/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/target').free > 2*1024**3
PY
cargo fmt -p brn-ai -p brn -- --check
cargo test -p brn-ai --lib guide_bundle_change_recomputes_prior_stamp_summary_then_reuses_unchanged_stamp --locked --offline --jobs 1 -- --nocapture
cargo test -p brn-ai --lib --locked --offline --jobs 1
cargo test -p brn --test agent_runs scoped_selected_skill_loads_exact_files_and_run_details_hide_prior_scope_after --locked --offline --jobs 1
cargo clippy -p brn-ai -p brn --all-targets --locked --offline --jobs 1 -- -D warnings
cargo build -p brn --locked --offline --jobs 1
git diff --check
test -z "$(git status --porcelain)"
