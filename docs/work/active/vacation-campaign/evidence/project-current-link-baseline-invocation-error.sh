#!/bin/zsh
set -e
. /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-project-current-link-baseline
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust
test "$(git rev-parse HEAD)" = 8c47af45bb0b0ded56cf58632d8d0232de9419c4
test -z "$(git status --porcelain)"
df -h .
cargo test -p brn-threads-app --test projects project_document_links_reject_superseded_notes_but_allow_unlinking_existing_links --exact --nocapture
