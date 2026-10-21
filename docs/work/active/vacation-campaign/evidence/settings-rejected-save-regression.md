# Rejected Settings Save regression witness

- Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`
- Branch: `codex/settings-save-rejection`
- Base: `6fb5323fa37c1def0f46b2979a480c173a70b0f1`
- Test-only commit: `2ea1c3752c307049e51a9e44c7b0e6e046b2d271`
- Changed file: `crates/brn-desktop/src/threads_pages_tests.rs`, extending the existing `the_helper_section_shows_its_tested_note_and_saves_with_settings` journey.

The test selects helper effort Low, turns the helper Off, switches it back to Codex, and selects parent effort High. It enters a synthetic model string and relative synthetic auth path, then clicks the actual Save control. It checks the reported absolute-path error, unchanged canonical settings and history, retained model/auth text, and visible helper and parent selections. It then corrects the auth field to the fixture's existing absolute synthetic path, retries Save, and retains the original positive helper/auth/working-button checks while asserting the saved model.

Expected baseline failure: the invalid Save currently refreshes the page from canonical settings. Canonical settings/history and text inputs should remain unchanged, but the refreshed segmented form loses the unsaved helper selection; the assertion that the helper effort control is visible should fail. The test also checks the parent High selection, which is likewise expected to be lost. No credential contents are read and the synthetic relative path is only validated as a string.

Validation performed: pinned `cargo-fmt --check -p brn-desktop` and `git diff --check` passed. The test and builds were intentionally not run; lead owns the serialized baseline-negative qualification.
