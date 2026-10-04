#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$(mktemp -d "${TMPDIR:-/private/tmp}/brn-app-test.XXXXXX")"
scratch="$(cd "$scratch" && pwd -P)"
cleanup() {
  python3 - "$scratch" <<'PY'
from pathlib import Path
import sys
root = Path(sys.argv[1])
assert root.name.startswith("brn-app-test.")
for path in sorted(root.rglob("*"), key=lambda p: len(p.parts), reverse=True):
    if path.is_symlink() or not path.is_dir():
        path.unlink()
    else:
        path.rmdir()
root.rmdir()
PY
}
trap cleanup EXIT

binary="$scratch/stub ' % \` \$(touch injected) ü"
data_dir="$scratch/workspace ' % \` \$(touch injected) ü"
model_dir="$scratch/model ' % \` \$(touch injected) ü"
bundle="$scratch/BRN Trial.app"
mkdir -p "$data_dir" "$model_dir"
cat > "$binary" <<'STUB'
#!/bin/bash
if (($#)); then printf '%s\n' "$@" > "$BRN_LAUNCH_ARGS_FILE"; else : > "$BRN_LAUNCH_ARGS_FILE"; fi
STUB
chmod +x "$binary"

"$root/scripts/make-macos-app.sh" --output "$bundle" --binary "$binary" --data-dir "$data_dir" --model-dir "$model_dir"
plutil -lint "$bundle/Contents/Info.plist" >/dev/null
cd "$scratch"
BRN_LAUNCH_ARGS_FILE="$scratch/args" BRN_LAUNCH_LOG_DIR="$scratch/logs" BRN_LAUNCH_TEST_NO_OPEN=1 "$bundle/Contents/MacOS/BRN-Usability-Trial"
printf '%s\n' --data-dir "$data_dir" --model-dir "$model_dir" > "$scratch/expected"
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
default_bundle="$scratch/Default.app"
"$root/scripts/make-macos-app.sh" --output "$default_bundle" --binary "$binary"
BRN_LAUNCH_ARGS_FILE="$scratch/default-args" BRN_LAUNCH_LOG_DIR="$scratch/logs" BRN_LAUNCH_TEST_NO_OPEN=1 "$default_bundle/Contents/MacOS/BRN-Usability-Trial"
test ! -s "$scratch/default-args"
if "$root/scripts/make-macos-app.sh" --output "$scratch/invalid.app" --binary "$binary" --legacy > "$scratch/failure" 2>&1; then
  echo "Retired legacy flag unexpectedly accepted" >&2
  exit 1
fi
rm "$bundle/Contents/Resources/bin/brn-desktop"
if BRN_LAUNCH_ARGS_FILE="$scratch/args" BRN_LAUNCH_LOG_DIR="$scratch/logs" BRN_LAUNCH_TEST_NO_OPEN=1 "$bundle/Contents/MacOS/BRN-Usability-Trial" > "$scratch/failure" 2>&1; then
  echo "Missing desktop binary unexpectedly accepted at launch" >&2
  exit 1
fi
grep -q 'Desktop binary' "$scratch/logs/startup.log"
echo "macOS launcher checks passed"
