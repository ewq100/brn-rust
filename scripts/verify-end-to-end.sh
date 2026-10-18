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
  cargo fmt --all -- --check
  if [[ "$(uname -s)" == Darwin ]]; then
    bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root "$CARGO_TARGET_DIR/document-helpers"
    export BRN_XBERG_HELPER="$CARGO_TARGET_DIR/document-helpers/artifacts/brn-xberg-helper"
    export BRN_XBERG_HELPER_BRN="$CARGO_TARGET_DIR/document-helpers/artifacts/brn-xberg-helper-brn"
    mkdir -p "$CARGO_TARGET_DIR/debug"
    cp "$BRN_XBERG_HELPER" "$BRN_XBERG_HELPER_BRN" "$CARGO_TARGET_DIR/debug/"
  fi
  cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
  cargo build --workspace --locked --offline
  cargo clippy --workspace --all-targets --locked --offline -- -D warnings
  cargo test --workspace --no-fail-fast --locked --offline
  if [[ "$(uname -s)" == Darwin ]]; then
    cargo test -p brn --features scripted-document-tests --bin brn document_route_tests --locked --offline --jobs 1
    cargo clippy -p brn --features scripted-document-tests --all-targets --locked --offline --jobs 1 -- -D warnings
  fi
fi
python3 scripts/verify-end-to-end-fixtures.py "$CARGO_TARGET_DIR/debug"
python3 scripts/cli_journey.py "$CARGO_TARGET_DIR/debug/brn"
