#!/usr/bin/env bash
set -euo pipefail
if [[ $# -gt 1 || ( $# -eq 1 && "$1" != "--native" ) ]]; then
  printf 'Usage: %s [--native]\n' "$0" >&2
  exit 1
fi
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# Resolve Cargo through the repository toolchain pin even when called elsewhere.
cd "$repo_root"
manifest="$repo_root/experiments/retrieval-trial/Cargo.toml"
export PATH="/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"

cargo test --manifest-path "$manifest" --locked --offline
cargo build --manifest-path "$manifest" --locked --offline
cargo fmt --manifest-path "$manifest" --check
cargo clippy --manifest-path "$manifest" --all-targets --locked --offline -- -D warnings
binary="${CARGO_TARGET_DIR:-$repo_root/experiments/retrieval-trial/target}/debug/brn-retrieval-trial"
"$binary" --help >/dev/null
"$binary" check
for argument in --unknown unknown; do
  if "$binary" "$argument" >/dev/null 2>&1; then
    printf 'Unexpected success for %s\n' "$argument" >&2
    exit 1
  fi
done
if "$binary" >/dev/null 2>&1; then
  printf 'Unexpected success without a command\n' >&2
  exit 1
fi
if [[ "${1:-}" == "--native" ]]; then
  cargo build --manifest-path "$manifest" --features native --locked --offline
  "$binary" --help >/dev/null
  "$binary" check
  cargo test --manifest-path "$manifest" --features native --locked --offline
  cargo clippy --manifest-path "$manifest" --features native --all-targets --locked --offline -- -D warnings
fi
printf 'Retrieval automated checks passed; model/index evaluation is a separate explicit command.\n'
