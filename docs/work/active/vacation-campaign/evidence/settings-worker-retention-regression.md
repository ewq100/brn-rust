# Settings draft retention regression

Branch: `codex/settings-draft-isolation`  
Base: frozen source `fde0477385639913aa9e8dcf39981f95c0aff8d2`  
Commit: `72951e1160278c36da560b64cd9ba05209a406bb` (`Test Settings draft retention on worker completion`)

Only `crates/brn-desktop/src/threads_tiled_tests.rs` changed. Extended the existing `settings_save_preserves_backup_and_requires_explicit_delegation_choice` case: after entering the model text and selecting High effort, it records canonical provider/model/effort, injects one owned synthetic `Worker::output` completion event, and drains it through the existing `poll_worker` seam. The test asserts canonical provider/model/effort remain unchanged before Save, then retains all original Save and delegation assertions and the same test count.

The completion runs through the ordinary worker-finished path, which refreshes persisted settings while Models remains the selected Settings section. On the frozen implementation, that refresh resets the segmented effort form to canonical Medium while leaving the separate model `InputState` at `offline-model-choice`; the existing final saved-effort expectation should therefore fail with Medium. The lead should reproduce this red before selecting the minimal production correction.

Pinned `cargo-fmt -p brn-desktop --check` and `git diff --check` passed. No build or test was run. No provider/account/process/native/credential action was performed.
