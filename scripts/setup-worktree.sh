#!/usr/bin/env bash
# Prepare a fresh checkout: fetch locked crates, provide both document helpers
# from a shared content-keyed cache, and create an ignored synthetic data folder.
# Writes .brn-dev/env.sh; source it before running tests or the app.
set -euo pipefail

repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
dev="$repo/.brn-dev"
cache_root="${BRN_HELPER_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/brn/document-helpers}"
target_dir="${CARGO_TARGET_DIR:-$repo/target}"

mkdir -p "$dev/data"
cd "$repo"

host=$(rustc --version --verbose | sed -n 's/^host: //p')
cargo fetch --locked --quiet
cargo fetch --locked --quiet --manifest-path tools/xberg-intake-helper/Cargo.toml --target "$host"

# Key on the working-tree bytes of every build input, so uncommitted helper edits rebuild.
key=$(git ls-files -z --cached --others --exclude-standard tools/xberg-intake-helper patches/xberg scripts/build-document-helpers.sh scripts/prepare-xberg-adapted.py \
  | sort -z | xargs -0 shasum -a 256 2>/dev/null | shasum -a 256 | cut -c1-16)
helpers="$cache_root/$key"
if [[ ! -x "$helpers/artifacts/brn-xberg-helper" || ! -x "$helpers/artifacts/brn-xberg-helper-brn" ]]; then
  echo "Building document helpers into $helpers (once per helper change)"
  mkdir -p "$cache_root" && cache_root=$(cd "$cache_root" && pwd -P) && helpers="$cache_root/$key"
  archive="${CARGO_HOME:-$HOME/.cargo}/registry/cache/index.crates.io-1949cf8c6b5b557f/xberg-1.3.10.crate"
  python3 scripts/prepare-xberg-adapted.py --archive "$archive"
  cargo fetch --locked --quiet --manifest-path target/xberg-adapted/workspace/Cargo.toml --target "$host"
  staging=$(mktemp -d "$cache_root/.build-$key.XXXXXX")
  bash scripts/build-document-helpers.sh --offline --jobs 1 --target-root "$staging"
  # Another checkout may have finished the same key first; keep whichever landed.
  python3 -c 'import os,sys; os.rename(sys.argv[1], sys.argv[2])' "$staging" "$helpers" 2>/dev/null || rm -rf "$staging"
fi

# CLI process tests discover helpers beside the built binaries.
mkdir -p "$target_dir/debug"
cp "$helpers/artifacts/brn-xberg-helper" "$helpers/artifacts/brn-xberg-helper-brn" "$target_dir/debug/"

tmp=$(cd "${TMPDIR:-/tmp}" && pwd -P)

# BRN refuses data folders under a symlink; macOS TMPDIR sits under /var -> /private/var.
cat > "$dev/env.sh" <<EOF
export TMPDIR="$tmp"
export BRN_XBERG_HELPER="$helpers/artifacts/brn-xberg-helper"
export BRN_XBERG_HELPER_BRN="$helpers/artifacts/brn-xberg-helper-brn"
export BRN_DEV_DATA_DIR="$dev/data"
EOF
echo "Ready: source .brn-dev/env.sh (data folder $dev/data)"
