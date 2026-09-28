#!/usr/bin/env bash
# Assemble a local unsigned Finder launcher around one explicitly built desktop binary.
set -euo pipefail

usage() {
  cat <<'HELP'
Usage: make-macos-app.sh --output ABSOLUTE.app --binary ABSOLUTE_BINARY --data-dir ABSOLUTE_DIRECTORY [--codex ABSOLUTE_EXECUTABLE] [--model-dir ABSOLUTE_DIRECTORY]

Creates an unsigned local .app. The specified binary is copied into the bundle.
The data directory, Codex executable, and model directory remain at their selected paths.
Startup failures are logged at ~/Library/Logs/BRN Usability Trial/startup.log
HELP
}

output= binary= data_dir= codex= model_dir=
while (($#)); do
  case "$1" in
    --help) usage; exit 0 ;;
    --output|--binary|--data-dir|--codex|--model-dir)
      key="$1"
      if (($# < 2)); then echo "$key needs a value" >&2; exit 2; fi
      case "$key" in
        --output) output="$2" ;;
        --binary) binary="$2" ;;
        --data-dir) data_dir="$2" ;;
        --codex) codex="$2" ;;
        --model-dir) model_dir="$2" ;;
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
if [[ ! -d "$data_dir" || ! -w "$data_dir" ]]; then echo "Data directory is missing or not writable: $data_dir" >&2; exit 2; fi
if [[ -n "$codex" && ( "$codex" != /* || ! -f "$codex" || ! -x "$codex" ) ]]; then echo "Codex executable is missing or not executable: $codex" >&2; exit 2; fi
if [[ -n "$model_dir" && ( "$model_dir" != /* || ! -d "$model_dir" ) ]]; then echo "Model directory is missing: $model_dir" >&2; exit 2; fi

mkdir -p "$output/Contents/MacOS" "$output/Contents/Resources/bin"
cp "$binary" "$output/Contents/Resources/bin/brn-desktop"
chmod +x "$output/Contents/Resources/bin/brn-desktop"
cat > "$output/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>BRN Usability Trial</string>
  <key>CFBundleDisplayName</key><string>BRN Usability Trial</string>
  <key>CFBundleIdentifier</key><string>dev.brn.usability-trial</string>
  <key>CFBundleExecutable</key><string>BRN-Usability-Trial</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleVersion</key><string>1</string>
</dict></plist>
PLIST

launcher="$output/Contents/MacOS/BRN-Usability-Trial"
cat > "$launcher" <<'LAUNCH'
#!/bin/bash
set -u
resource_dir="$(cd "$(dirname "$0")/../Resources" && pwd)"
binary="$resource_dir/bin/brn-desktop"
log_dir="${BRN_LAUNCH_LOG_DIR:-$HOME/Library/Logs/BRN Usability Trial}"
mkdir -p "$log_dir"
log="$log_dir/startup.log"
LAUNCH
printf 'data_dir=%q\n' "$data_dir" >> "$launcher"
printf 'codex=%q\n' "$codex" >> "$launcher"
printf 'model_dir=%q\n' "$model_dir" >> "$launcher"
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
[[ -d "$data_dir" && -w "$data_dir" ]] || fail "Data directory is missing or not writable: $data_dir"
args=(--data-dir "$data_dir")
if [[ -n "$codex" ]]; then
  [[ -x "$codex" ]] || fail "Codex executable is missing or not executable: $codex"
  args+=(--codex "$codex")
fi
if [[ -n "$model_dir" ]]; then
  [[ -d "$model_dir" ]] || fail "Model directory is missing: $model_dir"
  args+=(--model-dir "$model_dir")
fi
printf 'Launching BRN Usability Trial in %s\n' "$data_dir" >> "$log"
"$binary" "${args[@]}" >> "$log" 2>&1
code=$?
if ((code != 0)); then fail "BRN exited with status $code"; fi
LAUNCH
chmod +x "$launcher"
printf 'Created %s\n' "$output"
