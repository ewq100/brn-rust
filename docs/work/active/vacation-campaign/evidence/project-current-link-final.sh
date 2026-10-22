#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-project-current-link-final
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = 18abae94fae754834af14214b9fc8910562d046d
test -z "$(git status --porcelain)"
cargo test -p brn-threads-app --test projects --locked --offline --jobs 1
cargo test -p brn-threads-core --locked --offline --jobs 1
cargo clippy -p brn-threads-core -p brn-threads-app --all-targets --locked --offline --jobs 1 -- -D warnings
cargo fmt --all -- --check
python3 scripts/check-markdown-links.py
git diff --check
test -z "$(git status --porcelain)"
