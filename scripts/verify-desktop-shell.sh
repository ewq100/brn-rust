#!/usr/bin/env bash
# Compile and verify the Threads desktop state on the repository toolchain.
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
if [[ -n "${BRN_VERIFY_OUTPUT_DIR:-}" && -z "${_BRN_EVIDENCE_CHILD:-}" ]]; then
  exec python3 "$repo_root/scripts/verification_evidence.py" "$(basename "$0")" -- "$@"
fi
widgets_only=false
if [[ $# -eq 1 && "$1" == '--widgets-only' ]]; then
  widgets_only=true
elif [[ $# -ne 0 ]]; then
  printf 'Usage: verify-desktop-shell.sh [--widgets-only]\n' >&2
  exit 2
fi
cd "$repo_root"
widget_test_args=(-p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui:: --locked --offline)
check_widget_discovery() {
  local test_list widget_count
  test_list="$(cargo test "${widget_test_args[@]}" -- --list --format terse)"
  widget_count="$(printf '%s\n' "$test_list" | awk '/^threads_ui::[[:alnum:]_]+_tests::[^[:space:]]+: test$/ { count++ } END { print count+0 }')"
  if [[ "$widget_count" -eq 0 ]]; then
    printf 'No Threads widget tests were discovered; refusing an empty verification run.\n' >&2
    return 1
  fi
  printf 'Discovered %s Threads widget tests.\n' "$widget_count"
}
run_tests_with_widget_evidence() {
  widget_log="$(mktemp)"
  trap 'rm -f "$widget_log"' EXIT
  cargo test "$@" -- --format pretty --color never | tee "$widget_log"
  if ! awk '/^test threads_ui::[[:alnum:]_]+_tests::[^[:space:]]+ \.\.\. ok$/ { passed=1 } END { exit !passed }' "$widget_log"; then
    printf 'No Threads widget tests passed; refusing an empty or entirely ignored run.\n' >&2
    exit 1
  fi
}
if [[ "$widgets_only" == true ]]; then
  check_widget_discovery
  run_tests_with_widget_evidence "${widget_test_args[@]}"
  printf 'Headless widget checks passed; rendered native interaction is a separate gate.\n'
  exit 0
fi
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo build -p brn-desktop --features native-ui,native-test-support --locked --offline
cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets --locked --offline -- -D warnings
check_widget_discovery
run_tests_with_widget_evidence -p brn-desktop --features native-ui,native-test-support --locked --offline
printf 'Native compilation and state checks passed; hands-on interaction is a separate gate.\n'
