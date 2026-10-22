#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-indexed-settings-read-qualification
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
python3 - <<'CHECK'
import subprocess,shutil
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='eae61dde04cda19c764246993966b13b997014eb'
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
assert shutil.disk_usage('.').free>3*1024**3
CHECK
cargo test -p brn-threads-core --locked --offline --jobs 1
cargo test -p brn-threads-app --test product --test topics_knowledge --test delegation --test skills --test mcp --test projects --test document_conversion_settings --test backup_directory --test workspace_purpose --test thread_lifecycle --test killed_run_candidate --locked --offline --jobs 1
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline --jobs 1
cargo clippy -p brn-threads-core -p brn-threads-app --all-targets --locked --offline --jobs 1 -- -D warnings
cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline --jobs 1 -- -D warnings
cargo build -p brn-desktop --features native-ui --locked --offline --jobs 1
cargo fmt --all -- --check
git diff --check
