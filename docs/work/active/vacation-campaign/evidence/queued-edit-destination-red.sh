#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-queued-edit-destination-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/export-durability/brn-rust
test "$(git rev-parse HEAD)" = "97fa29c21480cd995f4aa96f1b7c1c04acf48d84"
test -z "$(git status --porcelain)"
cargo test -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::chat_tests::editing_a_queued_message_does_not_redirect_the_next_chat_send --locked --offline --jobs 1 -- --exact --nocapture
