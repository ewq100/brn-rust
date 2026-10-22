#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-strict-mac-scale-qualification/tmp
export BRN_SCALE_STRICT=1
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust
test "$(git rev-parse HEAD)" = "9d5a43ed1baf2725b84872f0f2bb4b889d913023"
git diff --exit-code 88866d40f3f3ff1d5d0d351cf687076890f9ab79 -- crates agent tools Cargo.toml Cargo.lock
python3 - <<'CHECK'
import shutil
assert shutil.disk_usage('.').free>3*1024**3,'Insufficient space for new release graph'
CHECK
cargo test --release -p brn-threads-app --test user_scale ten_thousand_notes_reopen_without_rebuild_sync_one_note_and_query_quickly --locked --offline --jobs 1 -- --exact --nocapture
