# Helper effort reset regression

Branch: `codex/settings-draft-isolation`  
Base: `3d25183ba134b75a5d290d33182a651aafc3bd7f`  
Commit: `29a02b4f52506001bc79be69a2c1c78ae7a210d4` (`Cover helper effort reset after disabling`)

Only `crates/brn-desktop/src/threads_pages_tests.rs` changed. Extended the existing positive helper Save test with real UI clicks for helper effort Low → provider Off → provider Codex before Save. Its existing saved helper expectation remains High, so the current candidate should fail at the exact persisted-effort assertion if disabling and re-enabling no longer applies the previous reset-to-High behavior.

Pinned `cargo-fmt --check -p brn-desktop` and `git diff --check` passed. No build or test was run, per assignment. This is a test-only expected-red extension for the lead's bounded reproduction; production remains unchanged in this commit.
