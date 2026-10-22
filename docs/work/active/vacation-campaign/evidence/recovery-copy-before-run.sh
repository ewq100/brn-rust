#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-recovery-copy-before-qualification
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
python3 - <<'CHECK'
import subprocess,hashlib,pathlib,shutil,json
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='2ff42451adf97268884afd3a0b91c3ec97922e3f'
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
assert hashlib.sha256(pathlib.Path('crates/brn-desktop/src/threads_polish_tests.rs').read_bytes()).hexdigest()=='8f029275d8448d5c33bc028f03e87d217083d7fb452050c78259129a7e140ac4'
assert shutil.disk_usage('.').free>3*1024**3
print(json.dumps({'candidate':'2ff42451adf97268884afd3a0b91c3ec97922e3f','base':'13813f6ff827de776a663036f5d468500e7a5ed1','purpose':'one before-source private recovery-copy desktop witness; actual failure expected, not native/product acceptance'}),flush=True)
CHECK
cargo clean -p brn-desktop --locked --offline
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop --locked --offline --jobs 1 threads_ui::polish_tests::stale_private_note_copy_stays_out_of_general_knowledge -- --exact --nocapture
