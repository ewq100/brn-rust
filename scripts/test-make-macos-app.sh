#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scratch="$(mktemp -d "${TMPDIR:-/private/tmp}/brn-launcher.XXXXXX")"
scratch="$(cd "$scratch" && pwd -P)"
trap 'rm -rf "$scratch"' EXIT
data_dir="$scratch/data ' % \` \$(touch injected) ü"
binary="$scratch/stub ' % \` \$(touch injected) ü"
mkdir "$data_dir"
cat > "$binary" <<'BIN'
#!/bin/bash
printf '%s\n' "$@" > "$BRN_TEST_ARGS"
exit "${BRN_TEST_EXIT:-0}"
BIN
chmod +x "$binary"
expect_failure() { if "$@" >"$scratch/refusal" 2>&1; then echo 'Unexpected launcher success' >&2; exit 1; fi; }
expect_failure bash "$repo_root/scripts/make-macos-app.sh" --output "$scratch/missing.app" --binary "$binary"
bash "$repo_root/scripts/make-macos-app.sh" --output "$scratch/Threads.app" --binary "$binary" --data-dir "$data_dir"
expect_failure bash "$repo_root/scripts/make-macos-app.sh" --output "$scratch/Threads.app" --binary "$binary" --data-dir "$data_dir"
export BRN_TEST_ARGS="$scratch/args" BRN_LAUNCH_LOG_DIR="$scratch/logs" BRN_LAUNCH_TEST_NO_OPEN=1
"$scratch/Threads.app/Contents/MacOS/BRN-Threads"
[[ "$(head -n 1 "$scratch/args")" == --data-dir ]]
[[ "$(tail -n 1 "$scratch/args")" == "$data_dir" ]]
test ! -e "$scratch/injected"
export BRN_TEST_EXIT=7
expect_failure "$scratch/Threads.app/Contents/MacOS/BRN-Threads"
rg -q 'status 7' "$scratch/logs/startup.log"
printf 'Threads explicit launcher and failure reporting passed\n'
