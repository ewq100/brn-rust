#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-provider-test-child-bounds-final
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust
test "$(git rev-parse HEAD)" = a6b6d9902c8797a696cb96e1d9e848c564425b9e
test -z "$(git status --porcelain)"
cargo test -p brn-ai --lib --locked --offline --jobs 1
cargo clippy -p brn-ai --all-targets --locked --offline --jobs 1 -- -D warnings
cargo fmt --all -- --check
git diff --check
test "$(git rev-parse HEAD)" = a6b6d9902c8797a696cb96e1d9e848c564425b9e
test -z "$(git status --porcelain)"
