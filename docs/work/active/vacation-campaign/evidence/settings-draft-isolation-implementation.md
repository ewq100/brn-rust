# Settings draft-isolation correction

Branch: `codex/settings-draft-isolation`  
Base: regression-only `7462f0a04b5e195a4618cdd27af2a4fe7281c293`  
Commit: `3d25183ba134b75a5d290d33182a651aafc3bd7f` (`Isolate Settings form selections from active config`)

Changed only `crates/brn-desktop/src/threads_state.rs`, `threads_ui.rs`, and `threads_tiles.rs`. `threads_pages_tests.rs` remains the committed red regression from the prior commit.

- `threads_state.rs`: adds the small `ProviderForm` containing only the provider, effort, helper-provider and helper-effort segments. Model and credential text remains in the existing `InputState` entities.
- `threads_ui.rs`: initializes and refreshes `ProviderForm` from canonical `WorkspaceSettings`, preserving the existing refresh/reset behavior. Save now overlays the form segments and existing text inputs onto a local clone of canonical settings, constructs helper settings from the selected helper segments and existing text fields, and passes that value to the unchanged `Workspace::configure`. It does not mutate active `self.settings` before the checked write succeeds. Existing `configure_explicit`, Copilot connection events, chat display and run preparation continue using canonical settings.
- `threads_tiles.rs`: Models segmented controls read and mutate `ProviderForm`; other Settings sections and the Save action remain on the existing checked path.

Checks run:

- Pinned `cargo-fmt -p brn-desktop` and follow-up `cargo-fmt --check -p brn-desktop` — passed.
- `git diff --check` — passed before commit.
- No build or tests were run, per assignment. The regression-only commit's expected baseline failure was reproduced by the lead; the corrected candidate still needs the lead's serialized run of that focused test, existing positive Save and worker-fence tests, and applicable broader checks.

No provider/account/native/network/credential action was performed. Candidate is ready for independent Sol review and lead verification; no merge or acceptance claim is made.
