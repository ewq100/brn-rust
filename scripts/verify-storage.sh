#!/usr/bin/env bash
# Verify the complete workspace and exercise storage-safe CLI/headless paths.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
  if [[ "$CARGO_TARGET_DIR" != /* ]]; then
    printf 'CARGO_TARGET_DIR must be an absolute path when set: %s\n' "$CARGO_TARGET_DIR" >&2
    exit 1
  fi
else
  export CARGO_TARGET_DIR="$repo_root/target"
fi

if [[ $# -ne 0 ]]; then
  printf 'Usage: %s\n' "$0" >&2
  exit 1
fi

cd "$repo_root"
cargo +1.98.1 fmt --all --check
cargo +1.98.1 build --workspace --locked --offline
cargo +1.98.1 clippy --workspace --all-targets --locked --offline -- -D warnings
cargo +1.98.1 test --workspace --locked --offline

brn="$CARGO_TARGET_DIR/debug/brn"
desktop="$CARGO_TARGET_DIR/debug/brn-desktop"
for binary in "$brn" "$desktop"; do
  if [[ ! -x "$binary" ]]; then
    printf 'Expected executable not found: %s\n' "$binary" >&2
    exit 1
  fi
done

scratch="$(mktemp -d "${TMPDIR:-/tmp}/brn-storage.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
data_dir="$scratch/data"
mkdir "$data_dir"

expect_failure() {
  local label="$1"
  shift
  if "$@" >"$scratch/output" 2>&1; then
    printf 'Unexpected success for %s: %s\n' "$label" "$*" >&2
    exit 1
  fi
}

"$brn" --help >/dev/null
"$brn" --version >/dev/null
expect_failure 'brn without arguments' "$brn"
expect_failure 'brn with unknown arguments' "$brn" --unknown

"$desktop" --help >/dev/null
expect_failure 'desktop unknown option' "$desktop" --unknown
"$desktop" --headless-check completion --data-dir "$data_dir"
"$desktop" --headless-check cancellation --data-dir "$data_dir"
"$desktop" --headless-check stale --data-dir "$data_dir"

printf 'Storage verification passed (workspace checks and headless smoke checks; no GUI or credentials used).\n'
