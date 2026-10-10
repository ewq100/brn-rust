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
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo build -p brn-desktop --features native-ui --locked --offline
cargo clippy -p brn-desktop --features native-ui --all-targets --locked --offline -- -D warnings
cargo test -p brn-desktop --features native-ui --locked --offline
printf 'Native compilation and state checks passed; hands-on interaction is a separate gate.\n'
