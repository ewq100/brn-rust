#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-derived-index-integrity-cost
export BRN_SCALE_STRICT=1
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust
python3 - <<'CHECK'
import subprocess,hashlib,pathlib,shutil,json
assert subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip()=='a499ae47c81de2e9101676743d164d6e5ca2cb9b'
assert subprocess.check_output(['git','diff','--name-only','HEAD'],text=True).strip()=='crates/brn-retrieval/src/search_index.rs'
assert not subprocess.check_output(['git','ls-files','--others','--exclude-standard'],text=True).strip()
p=pathlib.Path('crates/brn-retrieval/src/search_index.rs')
assert hashlib.sha256(p.read_bytes()).hexdigest()=='61cf38702f310fc1c65de835291535cc8f4cee91f08fae058145b7f23b5c0fe9'
assert shutil.disk_usage('.').free>3*1024**3
print(json.dumps({'head':'a499ae47c81de2e9101676743d164d6e5ca2cb9b','temporary_source_sha256':'61cf38702f310fc1c65de835291535cc8f4cee91f08fae058145b7f23b5c0fe9','purpose':'one healthy-index admission-cost experiment, not a repair candidate'}),flush=True)
CHECK
cargo test --release -p brn-threads-app --test user_scale ten_thousand_notes_reopen_without_rebuild_sync_one_note_and_query_quickly --locked --offline --jobs 1 -- --exact --nocapture
