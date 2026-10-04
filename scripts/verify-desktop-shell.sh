#!/usr/bin/env bash
# Build and smoke-check the desktop shell without opening its native window.
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

if [[ $# -gt 1 || ( $# -eq 1 && "$1" != "--native" ) ]]; then
  printf 'Usage: %s [--native]\n' "$0" >&2
  exit 1
fi
native=0
if [[ $# -eq 1 ]]; then
  native=1
fi

cd "$repo_root"

# Check formatting and the complete locked workspace in its lightweight mode.
cargo +1.98.1 fmt --all --check
cargo +1.98.1 build --workspace --locked --offline
cargo +1.98.1 clippy --workspace --all-targets --locked --offline -- -D warnings
cargo +1.98.1 test --workspace --locked --offline

if [[ "$native" -eq 1 ]]; then
  cargo +1.98.1 build -p brn-desktop --features native-ui,native-retrieval --locked --offline
  cargo +1.98.1 clippy -p brn-desktop --features native-ui,native-retrieval,native-test-support --all-targets --locked --offline -- -D warnings
  cargo +1.98.1 test -p brn-desktop --features native-ui,native-retrieval,native-test-support --locked --offline
fi

binary="$CARGO_TARGET_DIR/debug/brn-desktop"
if [[ ! -x "$binary" ]]; then
  printf 'Expected executable not found: %s\n' "$binary" >&2
  exit 1
fi

scratch="$(mktemp -d "${TMPDIR:-/private/tmp}/brn-desktop-shell.XXXXXX")"
scratch="$(cd "$scratch" && pwd -P)"
trap 'rm -rf "$scratch"' EXIT
headless_data="$scratch/data-headless"
mkdir "$headless_data"

expect_failure() {
  local label="$1"
  shift
  if "$binary" "$@" >"$scratch/output" 2>&1; then
    printf 'Unexpected success for %s: %s\n' "$label" "$*" >&2
    exit 1
  fi
}

"$binary" --help >/dev/null
# Never invoke the no-argument route for a native build, since it launches the GUI.
if [[ "$native" -eq 0 ]]; then
  expect_failure 'no arguments'
fi

expect_failure 'unknown option' --unknown
expect_failure 'help with surplus argument' --help extra
expect_failure 'repeated data directory' --data-dir "$headless_data" --data-dir "$headless_data"
expect_failure 'surplus argument' --data-dir "$headless_data" extra
expect_failure 'missing data directory option' --headless-check startup
expect_failure 'missing data directory value' --data-dir
expect_failure 'nonexistent data directory' --headless-check startup --data-dir "$scratch/missing"
file_data="$scratch/not-a-directory"
: > "$file_data"
expect_failure 'non-directory data path' --headless-check startup --data-dir "$file_data"
expect_failure 'relative data path' --headless-check startup --data-dir relative-path

"$binary" --headless-check startup --data-dir "$headless_data"
printf 'Desktop shell verification passed (AppWorker startup; native window not launched).\n'
