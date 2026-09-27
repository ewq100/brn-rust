#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_root/experiments/editor-trial/Cargo.toml"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"

cargo +1.98.1 test --manifest-path "$manifest" --locked --offline
cargo +1.98.1 build --manifest-path "$manifest" --features native-ui --locked --offline
cargo +1.98.1 fmt --manifest-path "$manifest" --check
cargo +1.98.1 clippy --manifest-path "$manifest" --features native-ui --all-targets --locked --offline -- -D warnings

binary="$repo_root/experiments/editor-trial/target/debug/brn-editor-trial"
if [[ ! -x "$binary" ]]; then
  printf 'Expected executable not found: %s\n' "$binary" >&2
  exit 1
fi

scratch="$(mktemp -d "${TMPDIR:-/tmp}/editor-trial-smoke.XXXXXX")"
trap 'rm -f "$invalid_utf8"; rmdir "$scratch"' EXIT
invalid_utf8="$scratch/invalid-utf8.txt"
printf '\377' > "$invalid_utf8"

"$binary" --help >/dev/null
if "$binary" --unknown >/dev/null 2>&1; then
  printf 'Unexpected success for --unknown\n' >&2
  exit 1
fi
if "$binary" one two >/dev/null 2>&1; then
  printf 'Unexpected success for surplus arguments\n' >&2
  exit 1
fi
if "$binary" "$scratch/does-not-exist.txt" >/dev/null 2>&1; then
  printf 'Unexpected success for nonexistent file\n' >&2
  exit 1
fi
if "$binary" "$invalid_utf8" >/dev/null 2>&1; then
  printf 'Unexpected success for invalid UTF-8 file\n' >&2
  exit 1
fi

printf 'Editor trial verification passed.\n'
