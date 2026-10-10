#!/usr/bin/env bash
# Verify the complete workspace and exercise storage-safe CLI/headless paths.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
# Opt-in durable evidence; the child executes the same existing gate.
if [[ -n "${BRN_VERIFY_OUTPUT_DIR:-}" && -z "${_BRN_EVIDENCE_CHILD:-}" ]]; then
  exec python3 "$repo_root/scripts/verification_evidence.py" "$(basename "$0")" -- "$@"
fi

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
cargo fmt --all --check
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo build --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
cargo test --workspace --locked --offline

brn="$CARGO_TARGET_DIR/debug/brn"
desktop="$CARGO_TARGET_DIR/debug/brn-desktop"
for binary in "$brn" "$desktop"; do
  if [[ ! -x "$binary" ]]; then
    printf 'Expected executable not found: %s\n' "$binary" >&2
    exit 1
  fi
done

scratch="$(mktemp -d "${TMPDIR:-/private/tmp}/brn-storage.XXXXXX")"
scratch="$(cd "$scratch" && pwd -P)"
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
"$desktop" --headless-check startup --data-dir "$data_dir"

printf 'Storage verification passed (workspace checks and AppWorker startup checks; no GUI, account or network calls).\n'
