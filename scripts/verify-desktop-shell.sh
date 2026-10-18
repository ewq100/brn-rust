#!/usr/bin/env bash
# Compile and verify the Threads desktop state on the repository toolchain.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
if [[ -n "${BRN_VERIFY_OUTPUT_DIR:-}" && -z "${_BRN_EVIDENCE_CHILD:-}" ]]; then
  exec python3 "$repo_root/scripts/verification_evidence.py" "$(basename "$0")" -- "$@"
fi
if [[ $# -ne 0 ]]; then printf 'Usage: verify-desktop-shell.sh\n' >&2; exit 2; fi
cd "$repo_root"
if [[ "$(uname -s)" == Darwin ]]; then
  bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root "${CARGO_TARGET_DIR:-$repo_root/target}/document-helpers"
  export BRN_XBERG_HELPER="${CARGO_TARGET_DIR:-$repo_root/target}/document-helpers/artifacts/brn-xberg-helper"
  export BRN_XBERG_HELPER_BRN="${CARGO_TARGET_DIR:-$repo_root/target}/document-helpers/artifacts/brn-xberg-helper-brn"
fi
cargo build -p brn-writing-ui --features native-ui --locked --offline
cargo clippy -p brn-writing-ui --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings
cargo test -p brn-writing-ui --features native-ui,native-test-support --locked --offline
cargo build -p brn-desktop --features native-ui,native-test-support --locked --offline
cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings
cargo test -p brn-desktop --features native-ui,native-test-support --locked --offline
printf 'Native compilation and state checks passed; hands-on interaction is a separate gate.\n'
