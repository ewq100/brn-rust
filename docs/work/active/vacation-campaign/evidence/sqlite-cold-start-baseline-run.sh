#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-sqlite-cold-start-baseline-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
python3 /tmp/brn-sqlite-cold-start-baseline/cold_processes.py
