#!/usr/bin/env bash
# Product offline gate, with a fixture-only mode after an existing workspace gate.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
# Opt-in durable evidence; the child executes the same existing gate.
if [[ -n "${BRN_VERIFY_OUTPUT_DIR:-}" && -z "${_BRN_EVIDENCE_CHILD:-}" ]]; then
  exec python3 "$repo_root/scripts/verification_evidence.py" "$(basename "$0")" -- "$@"
fi
cd "$repo_root"
case "${1:-}" in
  ""|--fixtures-only|--retirement-only) ;;
  *) printf 'Usage: verify-end-to-end.sh [--fixtures-only|--retirement-only]\n' >&2; exit 2 ;;
esac
if (($# > 1)); then exit 2; fi
python3 scripts/check-provider-retirement.py
python3 scripts/check-intake-retirement.py
if [[ "${1:-}" == "--retirement-only" ]]; then exit 0; fi
if [[ -z "${TMPDIR:-}" || "$TMPDIR" != /* || ! -d "$TMPDIR" ]]; then
  printf 'Set TMPDIR to an existing explicit synthetic fixture parent outside Git.\n' >&2
  exit 2
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target}"
if [[ "$CARGO_TARGET_DIR" != /* ]]; then
  printf 'CARGO_TARGET_DIR must be absolute\n' >&2
  exit 2
fi
if [[ "${1:-}" != "--fixtures-only" ]]; then
  cargo +1.98.1 fmt --all -- --check
  cargo +1.98.1 build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
  cargo +1.98.1 build --workspace --locked --offline
  cargo +1.98.1 clippy --workspace --all-targets --locked --offline -- -D warnings
  cargo +1.98.1 test --workspace --locked --offline
fi
python3 scripts/verify-end-to-end-fixtures.py "$CARGO_TARGET_DIR/debug"
