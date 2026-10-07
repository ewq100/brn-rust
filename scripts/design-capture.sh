#!/usr/bin/env bash
# Capture design-handbook screenshots of the real Desktop views.
#
# Seeds a fresh synthetic workspace (scripts/design-demo-workspace.py), then
# renders the production view tree with the toolkit's headless Metal renderer.
# No window is shown, no provider is called and no private data is read.
# Captures are visual evidence only; they do not establish native interaction.
#
# Usage: scripts/design-capture.sh OUTPUT_DIR [PREFIX]
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
if [[ $# -lt 1 || $# -gt 2 ]]; then
  printf 'Usage: %s OUTPUT_DIR [PREFIX]\n' "$0" >&2
  exit 1
fi
mkdir -p "$1"
out="$(cd "$1" && pwd -P)"
prefix="${2:-}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target}"

cd "$repo_root"
cargo +1.98.1 build -p brn --locked --offline --quiet
scratch="$(mktemp -d "${TMPDIR:-/private/tmp}/brn-design-capture.XXXXXX")"
scratch="$(cd "$scratch" && pwd -P)"
trap 'rm -rf "$scratch"' EXIT
python3 scripts/design-demo-workspace.py --root "$scratch/workspace" \
  --brn "$CARGO_TARGET_DIR/debug/brn" >/dev/null

cargo +1.98.1 build -p brn-desktop --features native-capture --locked --offline --quiet
BRN_DESIGN_CAPTURE=1 \
BRN_CAPTURE_DATA="$scratch/workspace/data" \
BRN_CAPTURE_VAULT="$scratch/workspace/vault" \
BRN_CAPTURE_OUT="$out" \
BRN_CAPTURE_PREFIX="$prefix" \
  "$CARGO_TARGET_DIR/debug/brn-desktop"
