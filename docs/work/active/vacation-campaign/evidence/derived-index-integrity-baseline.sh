#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-derived-index-integrity-baseline
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust
python3 - <<'PY'
import subprocess,shutil,json
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='a499ae47c81de2e9101676743d164d6e5ca2cb9b'
assert not subprocess.check_output(['git','status','--porcelain'],text=True).strip()
assert shutil.disk_usage('.').free>2*1024**3
print(json.dumps({'source':'a499ae47c81de2e9101676743d164d6e5ca2cb9b','purpose':'focused test-only baseline, not production qualification'}),flush=True)
PY
cargo test -p brn-retrieval --test search_index --locked --offline --jobs 1 outdated_damaged_or_interrupted_indexes_are_rebuilt_in_place -- --exact --nocapture
