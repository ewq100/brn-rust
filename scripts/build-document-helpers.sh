#!/usr/bin/env bash
set -euo pipefail
repo=$(cd "$(dirname "$0")/.." && pwd -P)
target_root=''
while (($#)); do
  case "$1" in
    --offline) shift ;;
    --jobs) [[ "${2:-}" == 1 ]] || { echo 'only jobs=1 is qualified' >&2; exit 2; }; shift 2 ;;
    --target-root) target_root="${2:?}"; shift 2 ;;
    *) echo "unexpected argument: $1" >&2; exit 2 ;;
  esac
done
[[ "$target_root" = /* && ! -L "$target_root" ]] || { echo 'absolute non-symlink target root required' >&2; exit 2; }
archive="${CARGO_HOME:-$HOME/.cargo}/registry/cache/index.crates.io-1949cf8c6b5b557f/xberg-1.3.10.crate"
python3 "$repo/scripts/prepare-xberg-adapted.py" --repo "$repo" --archive "$archive"
mkdir -p "$target_root/artifacts" "$target_root/receipts"
task_tmp=$(mktemp -d "${TMPDIR:-/tmp}/brn-document-helpers.XXXXXX")
task_tmp="$(cd "$task_tmp" && pwd -P)"
chmod 700 "$task_tmp"
for mode in stock adapted; do
  if [[ "$mode" == stock ]]; then
    workspace="$repo/tools/xberg-intake-helper"; name=brn-xberg-helper
  else
    workspace="$repo/target/xberg-adapted/workspace"; name=brn-xberg-helper-brn
  fi
  identity=$(sed -n 's/^engine_identity=//p' "$workspace/build-identity.txt")
  # Separate literal targets; never reuse another source root's output directory.
  if [[ -e "$target_root/$mode-root.txt" ]]; then
    [[ "$(cat "$target_root/$mode-root.txt")" == "$workspace" ]] || { echo 'target/source association mismatch' >&2; exit 2; }
  else printf '%s\n' "$workspace" > "$target_root/$mode-root.txt"; fi
  stamp=$(date -u +%Y%m%dT%H%M%SZ)-$$
  if [[ -f "$target_root/$mode/debug/brn-xberg-helper" ]]; then
    mkdir "$target_root/receipts/$mode-before-$stamp"
    cp "$target_root/$mode/debug/brn-xberg-helper" "$target_root/receipts/$mode-before-$stamp/"
    cp "$target_root/$mode/debug/brn-xberg-helper.d" "$target_root/receipts/$mode-before-$stamp/"
  fi
  XBERG_BUILD_ID="$identity" TMPDIR="$task_tmp" CARGO_TARGET_DIR="$target_root/$mode" \
    cargo +1.98.1 build --offline --locked --jobs 1 --manifest-path "$workspace/Cargo.toml" --bin brn-xberg-helper \
    2>&1 | tee "$target_root/receipts/$mode-$stamp.log"
  cp "$target_root/$mode/debug/brn-xberg-helper" "$target_root/artifacts/$name"
  cargo +1.98.1 tree --locked --offline --manifest-path "$workspace/Cargo.toml" \
    --target "$(rustc +1.98.1 -vV | sed -n 's/^host: //p')" \
    --edges normal,build --prefix none --format '{p}' > "$target_root/receipts/$mode-target-graph.txt"
  shasum -a 256 "$target_root/artifacts/$name" > "$target_root/receipts/$name-$stamp.sha256"
  printf '%s\n' "$target_root/artifacts/$name"
done
