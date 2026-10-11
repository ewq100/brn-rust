#!/usr/bin/env bash
# Assemble a local unsigned Finder launcher around one explicitly built desktop binary.
set -euo pipefail

usage() {
  cat <<'HELP'
Usage: make-macos-app.sh --output ABSOLUTE.app --binary ABSOLUTE_BINARY [--intake-helper ABSOLUTE_HELPER] --data-dir ABSOLUTE_DIRECTORY
       make-macos-app.sh --output ABSOLUTE.app --binary ABSOLUTE_BINARY --intake-helper ABSOLUTE_HELPER --preview-version vX.Y.Z-preview.N [--build-number N]

Creates an unsigned local .app. The specified binary is copied into the bundle.
An explicit fresh Threads data directory is required.
Data directories remain at their selected paths; no old data is inspected or copied.
Portable previews select ~/Library/Application Support/BRN Threads Preview/workspace
on the recipient's Mac. Core directory/schema checks preserve unrelated data.
Startup failures are logged at ~/Library/Logs/BRN Threads/startup.log
HELP
}

output= binary= intake_helper= data_dir= preview_version= build_number=1
while (($#)); do
  case "$1" in
    --help) usage; exit 0 ;;
    --output|--binary|--intake-helper|--data-dir|--preview-version|--build-number)
      key="$1"
      if (($# < 2)); then echo "$key needs a value" >&2; exit 2; fi
      case "$key" in
        --output) output="$2" ;;
        --binary) binary="$2" ;;
        --intake-helper) intake_helper="$2" ;;
        --data-dir) data_dir="$2" ;;
        --preview-version) preview_version="$2" ;;
        --build-number) build_number="$2" ;;
      esac
      shift 2 ;;
    *) echo "Unknown argument: $1" >&2; usage >&2; exit 2 ;;
  esac
done

for value in "$output" "$binary"; do
  if [[ "$value" != /* ]]; then echo "Required paths must be absolute and nonempty" >&2; exit 2; fi
done
if [[ -n "$preview_version" ]]; then
  if [[ -n "$data_dir" || ! "$preview_version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-preview\.[1-9][0-9]*$ ]]; then echo 'Choose a valid preview version or an explicit data directory' >&2; exit 2; fi
  if [[ -z "$intake_helper" ]]; then echo 'Portable previews require --intake-helper' >&2; exit 2; fi
elif [[ "$data_dir" != /* ]]; then
  echo 'An absolute --data-dir or --preview-version is required' >&2; exit 2
fi
if [[ ! "$build_number" =~ ^[1-9][0-9]*$ ]]; then echo 'Build number must be a positive integer' >&2; exit 2; fi
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
PLIST
short_version=0.1.0
if [[ -n "$preview_version" ]]; then short_version="${preview_version#v}"; short_version="${short_version%-preview.*}"; fi
printf '  <key>CFBundleShortVersionString</key><string>%s</string>\n  <key>CFBundleVersion</key><string>%s</string>\n' "$short_version" "$build_number" >> "$output/Contents/Info.plist"
if [[ -n "$preview_version" ]]; then
  printf '  <key>LSMinimumSystemVersion</key><string>15.0</string>\n' >> "$output/Contents/Info.plist"
fi
cat >> "$output/Contents/Info.plist" <<'PLIST'
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
if [[ -n "$preview_version" ]]; then
  cat >> "$launcher" <<'LAUNCH'
[[ "${HOME:-}" == /* ]] || { printf '%s\n' 'HOME must be an absolute directory' >&2; exit 1; }
data_dir="$HOME/Library/Application Support/BRN Threads Preview/workspace"
portable_preview=1
LAUNCH
else
  printf 'data_dir=%q\nportable_preview=0\n' "$data_dir" >> "$launcher"
fi
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
if [[ "$portable_preview" == 1 ]]; then
  [[ -x "$resource_dir/bin/brn-intake-helper" ]] || fail 'Bundled intake helper is missing or not executable'
fi
args=()
if [[ -n "$data_dir" ]]; then
  if [[ "$portable_preview" == 0 ]]; then
    [[ -d "$data_dir" && -w "$data_dir" ]] || fail "Data directory is missing or not writable: $data_dir"
  fi
  args+=(--data-dir "$data_dir")
fi
printf 'Launching BRN Threads in %s\n' "$data_dir" >> "$log"
"$binary" "${args[@]}" >> "$log" 2>&1
code=$?
if ((code != 0)); then fail "BRN exited with status $code"; fi
LAUNCH
chmod +x "$launcher"
printf 'Created %s\n' "$output"
