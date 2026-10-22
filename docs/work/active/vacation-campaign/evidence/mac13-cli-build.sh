#!/bin/zsh
set -eu
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-mac13-build
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
# Shared target last compiled privacy source in a worker checkout. Force the
# changed local app and CLI crates, retaining dependency and helper artifacts.
cargo clean -p brn -p brn-threads-app --target-dir "$CARGO_TARGET_DIR"
cargo build -p brn --locked --offline --jobs 1
