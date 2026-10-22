#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-public-backup-interruption-build
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = 7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5
test -z "$(git status --porcelain)"
cargo build -p brn --locked --offline --jobs 1
test "$(git rev-parse HEAD)" = 7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5
test -z "$(git status --porcelain)"
