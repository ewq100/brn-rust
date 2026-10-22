#!/bin/bash
set -euo pipefail
source /tmp/brn-launcher-focus-qualification/environment.sh
export TMPDIR=/private/tmp/brn-quote-legacy-qualification/tmp
mkdir -p "$TMPDIR"
chmod 700 "$TMPDIR"
cd /Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust
test "$(git rev-parse HEAD)" = "eac295b14ddb4a57b1e3b62e6c51f0d40ae699a5"
test -z "$(git status --porcelain)"
cp crates/brn-ai/src/threads_note_edit_tests.rs /tmp/brn-quote-legacy-qualification/original-tests.rs
trap 'cp /tmp/brn-quote-legacy-qualification/original-tests.rs crates/brn-ai/src/threads_note_edit_tests.rs' EXIT
cat >> crates/brn-ai/src/threads_note_edit_tests.rs <<'RS'

#[test]
fn capture_legacy_edit_serialization() {
    use sha2::Digest as _;
    let legacy = args("legacy-note:0", 7, 3, "abc", "XY", "legacy-key");
    let bytes = serde_json::to_vec(&legacy).unwrap();
    let expected = r#"{"id":"legacy-note:0","version":7,"start":3,"end":6,"expected_text":"abc","replacement_text":"XY","request_key":"legacy-key","reason":"Correct the exact local passage"}"#;
    assert_eq!(bytes, expected.as_bytes());
    println!("BASE_LEGACY_JSON={}", String::from_utf8(bytes.clone()).unwrap());
    println!("BASE_LEGACY_SHA256={:x}", sha2::Sha256::digest(&bytes));
}
RS
git diff --binary -- crates/brn-ai/src/threads_note_edit_tests.rs > /tmp/brn-quote-legacy-qualification/capture.patch
cargo test -p brn-ai --lib capture_legacy_edit_serialization --locked --offline --jobs 1 -- --nocapture
