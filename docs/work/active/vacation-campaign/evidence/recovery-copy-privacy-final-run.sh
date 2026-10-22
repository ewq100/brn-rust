#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-recovery-copy-final-qualification-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
python3 - <<'CHECK'
import subprocess,shutil
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='ccd05b28ea7e1d7bcd92fe266f6e37aa23461d1b'
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
assert shutil.disk_usage('.').free>3*1024**3
CHECK
stage() {
 local label="$1"
 shift
 printf '[gate:%s:start]\n' "$label"
 "$@"
 printf '[gate:%s:pass]\n' "$label"
}
stage rebuild-identity cargo clean -p brn-threads-app -p brn-desktop --locked --offline
stage current-inputs cargo test -p brn-threads-core --test current_inputs --locked --offline --jobs 1
stage app cargo test -p brn-threads-app --test writing_context --test writing --test writing_setup --test keyed_owner_operations --test cw2_review --test projects --test chat_decisions --test product --locked --offline --jobs 1
stage desktop cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline --jobs 1
stage app-clippy cargo clippy -p brn-threads-app --all-targets --locked --offline --jobs 1 -- -D warnings
stage desktop-clippy cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline --jobs 1 -- -D warnings
stage native-build cargo build -p brn-desktop --features native-ui --locked --offline --jobs 1
stage fmt cargo fmt --all -- --check
stage diff git diff --check
