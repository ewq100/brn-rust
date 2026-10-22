#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-backup-process-bounds-final
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = dc16973f711828ae8fc3c7923ab150bd1091f963
test -z "$(git status --porcelain)"
cargo test -p brn-threads-core --locked --offline --jobs 1
cargo clippy -p brn-threads-core --all-targets --locked --offline --jobs 1 -- -D warnings
cargo fmt --all -- --check
git diff --check
test -z "$(git status --porcelain)"
