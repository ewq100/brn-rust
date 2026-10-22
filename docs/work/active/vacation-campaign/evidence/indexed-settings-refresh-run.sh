#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-indexed-settings-refresh-qualification
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust
python3 - <<'CHECK'
import subprocess,hashlib,pathlib,shutil,json
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='88c61df18cf6d09e7efc3a2d2349484645b2378a'
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
p=pathlib.Path('crates/brn-desktop/src/threads_omakase_tests.rs')
assert hashlib.sha256(p.read_bytes()).hexdigest()=='832d9e68810d6f52ea5eda317e301b8b574c04516a7ba6d16659339fb89a40ba'
assert shutil.disk_usage('.').free>3*1024**3
print(json.dumps({'candidate':'88c61df18cf6d09e7efc3a2d2349484645b2378a','base':'5fab84d87f2cff3bc03f40fa5987c1fe73be98ab','purpose':'one headless directrefresh diagnostic, no native/latency/owner acceptance','profile':'test opt-level1/debugassertions; native-ui,native-test-support; offlinefixture'}),flush=True)
CHECK
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop --locked --offline --jobs 1 threads_ui::omakase_tests::ten_thousand_notes_report_headless_refresh_duration -- --exact --nocapture
