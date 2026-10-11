#!/usr/bin/env bash
# Assemble a local unsigned Finder launcher around one explicitly built desktop binary.
set -euo pipefail

usage() {
  cat <<'HELP'
Usage: make-macos-app.sh --output ABSOLUTE.app --binary ABSOLUTE_BINARY [--intake-helper ABSOLUTE_HELPER] --data-dir ABSOLUTE_DIRECTORY

Creates an unsigned local .app. The specified binary is copied into the bundle.
An explicit fresh Threads data directory is required.
Data directories remain at their selected paths; no old data is inspected or copied.
Startup failures are logged at ~/Library/Logs/BRN Threads/startup.log
HELP
}

output= binary= intake_helper= data_dir=
while (($#)); do
  case "$1" in
    --help) usage; exit 0 ;;
    --output|--binary|--intake-helper|--data-dir)
      key="$1"
      if (($# < 2)); then echo "$key needs a value" >&2; exit 2; fi
      case "$key" in
        --output) output="$2" ;;
        --binary) binary="$2" ;;
        --intake-helper) intake_helper="$2" ;;
        --data-dir) data_dir="$2" ;;
      esac
      shift 2 ;;
    *) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

for value in "$output" "$binary" "$data_dir"; do
  if [[ "$value" != /* ]]; then echo "Required paths must be absolute and nonempty" >&2; exit 2; fi
done
if [[ "$output" != *.app ]]; then echo "--output must end in .app" >&2; exit 2; fi
if [[ -e "$output" ]]; then echo "Output already exists: $output" >&2; exit 2; fi
if [[ ! -f "$binary" || ! -x "$binary" ]]; then echo "Desktop binary is missing or not executable: $binary" >&2; exit 2; fi
if [[ -n "$data_dir" && ( "$data_dir" != /* || ! -d "$data_dir" || ! -w "$data_dir" ) ]]; then echo "Data directory is missing or not writable: $data_dir" >&2; exit 2; fi

if [[ -z "$intake_helper" && -x "$(dirname "$binary")/brn-intake-helper" ]]; then
  intake_helper="$(dirname "$binary")/brn-intake-helper"
fi
if [[ -n "$intake_helper" && ( "$intake_helper" != /* || ! -f "$intake_helper" || ! -x "$intake_helper" ) ]]; then echo "Intake helper is missing or not executable" >&2; exit 2; fi

mkdir -p "$output/Contents/MacOS" "$output/Contents/Resources/bin"
cp "$binary" "$output/Contents/Resources/bin/brn-desktop"
chmod +x "$output/Contents/Resources/bin/brn-desktop"
if [[ -n "$intake_helper" ]]; then
  cp "$intake_helper" "$output/Contents/Resources/bin/brn-intake-helper"
  chmod +x "$output/Contents/Resources/bin/brn-intake-helper"
fi
cat > "$output/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>BRN Threads</string>
  <key>CFBundleDisplayName</key><string>BRN Threads</string>
  <key>CFBundleIdentifier</key><string>dev.brn.threads</string>
  <key>CFBundleExecutable</key><string>BRN-Threads</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1</string>
</dict></plist>
PLIST

launcher="$output/Contents/MacOS/BRN-Threads"
cat > "$launcher" <<'LAUNCH'
#!/bin/bash
set -u
resource_dir="$(cd "$(dirname "$0")/../Resources" && pwd)"
binary="$resource_dir/bin/brn-desktop"
log_dir="${BRN_LAUNCH_LOG_DIR:-$HOME/Library/Logs/BRN Threads}"
mkdir -p "$log_dir"
log="$log_dir/startup.log"
LAUNCH
printf 'data_dir=%q\n' "$data_dir" >> "$launcher"
cat >> "$launcher" <<'LAUNCH'
fail() {
  printf '%s\n' "$1" >> "$log"
  if [[ "${BRN_LAUNCH_TEST_NO_OPEN:-}" != 1 ]]; then
    /usr/bin/open -a TextEdit "$log" || /usr/bin/open -R "$log"
  fi
  exit 1
}
: > "$log" || exit 1
[[ -x "$binary" ]] || fail "Desktop binary is missing or not executable: $binary"
args=()
if [[ -n "$data_dir" ]]; then
  [[ -d "$data_dir" && -w "$data_dir" ]] || fail "Data directory is missing or not writable: $data_dir"
  args+=(--data-dir "$data_dir")
fi
printf 'Launching BRN Threads in %s\n' "$data_dir" >> "$log"
"$binary" "${args[@]}" >> "$log" 2>&1
code=$?
if ((code != 0)); then fail "BRN exited with status $code"; fi
LAUNCH
chmod +x "$launcher"
printf 'Created %s\n' "$output"
