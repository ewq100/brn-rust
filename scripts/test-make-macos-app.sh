#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/brn-app-test.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

binary="$scratch/stub ' % \` \$(touch injected) ü"
data_dir="$scratch/workspace ' % \` \$(touch injected) ü"
codex="$scratch/codex ' % \` \$(touch injected) ü"
model_dir="$scratch/model ' % \` \$(touch injected) ü"
bundle="$scratch/BRN Trial.app"
mkdir -p "$data_dir" "$model_dir"
cat > "$binary" <<'STUB'
#!/bin/bash
printf '%s\n' "$@" > "$BRN_LAUNCH_ARGS_FILE"
STUB
cp "$binary" "$codex"
chmod +x "$binary" "$codex"

"$root/scripts/make-macos-app.sh" --output "$bundle" --binary "$binary" --data-dir "$data_dir" --codex "$codex" --model-dir "$model_dir"
plutil -lint "$bundle/Contents/Info.plist" >/dev/null
cd "$scratch"
BRN_LAUNCH_ARGS_FILE="$scratch/args" BRN_LAUNCH_LOG_DIR="$scratch/logs" BRN_LAUNCH_TEST_NO_OPEN=1 "$bundle/Contents/MacOS/BRN-Usability-Trial"
printf '%s\n' --data-dir "$data_dir" --codex "$codex" --model-dir "$model_dir" > "$scratch/expected"
cmp "$scratch/expected" "$scratch/args"
test ! -e "$scratch/injected"

if "$root/scripts/make-macos-app.sh" --output "$scratch/invalid.app" --binary "$scratch/missing" --data-dir "$data_dir" > "$scratch/failure" 2>&1; then
  echo "Missing binary unexpectedly accepted" >&2
  exit 1
fi
if "$root/scripts/make-macos-app.sh" --output "$scratch/invalid.app" --binary "$binary" --data-dir "$scratch/missing" > "$scratch/failure" 2>&1; then
  echo "Missing data directory unexpectedly accepted" >&2
  exit 1
fi
rm "$codex"
if BRN_LAUNCH_ARGS_FILE="$scratch/args" BRN_LAUNCH_LOG_DIR="$scratch/logs" BRN_LAUNCH_TEST_NO_OPEN=1 "$bundle/Contents/MacOS/BRN-Usability-Trial" > "$scratch/failure" 2>&1; then
  echo "Missing Codex dependency unexpectedly accepted at launch" >&2
  exit 1
fi
rg -q 'Codex executable' "$scratch/logs/startup.log"
echo "macOS launcher checks passed"
