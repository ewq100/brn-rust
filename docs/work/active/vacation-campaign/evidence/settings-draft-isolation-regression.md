# Settings draft-isolation regression

Branch: `codex/settings-draft-isolation`  
Base: `9e2a81c0e3d5fbc1b617fe54bdb4b103f607b822`  
Commit: `7462f0a04b5e195a4618cdd27af2a4fe7281c293` (`Test unsaved settings isolation on Copilot connect`)

Only `crates/brn-desktop/src/threads_pages_tests.rs` changed. Added `copilot_connection_does_not_save_an_unsaved_effort_choice` at line 1124. It uses the existing offline Settings widget fixture, selects High without Save, confirms canonical effort remains Medium, then injects an owned `Worker::connect` event with a synthetic absolute credentials-directory path. It expects the intended connection fields (provider and directory) to update while effort stays Medium. No sign-in, provider, credential read, or network code runs.

Expected failure against unchanged production: the final `saved.effort == "medium"` assertion receives `"high"`, because the current `CopilotConnected` path persists the entire mutable `self.settings` value. This is the intended red regression; it was not executed here, and the lead should reproduce it before selecting the production correction.

Checks run:

- Pinned `cargo-fmt --check -p brn-desktop` — passed.
- `git diff --check` — passed before commit.
- No build or tests were run, per assignment.
