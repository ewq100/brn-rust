#!/bin/zsh
set -eu
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-native-mac13-build
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
# Preserve target executable first (supervisor), then rebuild only desktop
# to bind this output to the actual current source. No dependency/helper clean.
cargo clean -p brn-desktop --target-dir "$CARGO_TARGET_DIR"
cargo build -p brn-desktop --bin brn-desktop --features native-ui --locked --offline --jobs 1
