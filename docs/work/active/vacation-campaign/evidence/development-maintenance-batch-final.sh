#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-development-maintenance-batch-final
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = 7dde6a41a4fbc66468e0400d2cc3c859f04c8ef5
test -z "$(git status --porcelain)"
cargo test -p brn-threads-core --locked --offline --jobs 1
cargo test -p brn-threads-app --test projects --locked --offline --jobs 1
cargo clippy -p brn-threads-core -p brn-threads-app --all-targets --locked --offline --jobs 1 -- -D warnings
cargo fmt --all -- --check
python3 -m unittest discover -s scripts/tests -v
python3 scripts/render-work-overview.py --check
python3 scripts/check-markdown-links.py
git diff --check
test -z "$(git status --porcelain)"
