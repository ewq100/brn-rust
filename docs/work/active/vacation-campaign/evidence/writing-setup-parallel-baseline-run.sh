#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-writing-setup-parallel-baseline-tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
python3 /tmp/brn-writing-setup-parallel-baseline/cold_processes.py
