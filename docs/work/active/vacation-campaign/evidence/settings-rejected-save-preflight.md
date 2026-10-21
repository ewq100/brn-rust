# Rejected Settings Save preflight

## Result and source identity

A concrete user-visible draft loss is reachable in frozen PR196 source `6fb5323fa37c1def0f46b2979a480c173a70b0f1`, branch `codex/settings-draft-isolation`, at `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust` (clean). The lead checkout is `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, HEAD `e3bc6c2ddf8a0f9d930072b1a6e50afc86b7b638`; its pre-existing campaign/overview edits and untracked evidence were preserved. No source or shared record was changed.

The concrete path is Save with a relative credential path. The button remains enabled whenever no worker is active (`crates/brn-desktop/src/threads_tiles.rs:1674-1698`). `ThreadsUi::configure` builds a Settings value from the editable `provider_form` and text inputs, calls `Workspace::configure`, displays the error on `Err`, then unconditionally calls `refresh` (`crates/brn-desktop/src/threads_ui.rs:3558-3588`). The app service rejects non-absolute `auth_file` or `credentials_dir` at `crates/brn-threads-app/src/settings.rs:89-110`, before it allocates an operation or calls the Store, so this refusal leaves canonical settings unchanged. `refresh` replaces `provider_form` from the current saved settings but does not replace the model, auth-path, credentials-path or helper-model `InputState`s (`threads_ui.rs:782-791`; initial input ownership at `threads_ui.rs:472-496`).

Observable consequence: after choosing an unsaved segmented effort such as High, entering a new model and a relative auth path, then clicking Save, the visible error is reported, canonical settings remain as before, but effort reverts to its saved value while the new model and rejected path remain in their text fields. The selection loss has no separate draft-reset notice. A later correction and Save can therefore persist the retained text using the reverted effort.

## Existing evidence and separation

The prior activation audit found and led to the Settings isolation correction; the refresh preflight found the segmented/text split on unrelated refresh; PR196's current worker-completion fix is specifically scoped to that event and is not the failed-Save path. The frozen source still calls the ordinary `refresh` directly after `configure` returns an error. This assessment does not reopen the solved connection/toolbar persistence leak, the unrelated-worker-completion fix, or navigation retention.

Existing `crates/brn-desktop/src/threads_pages_tests.rs:1063-1105` exercises a successful helper Save; `:1126-1173` exercises the current synthetic Copilot event and finished-worker Settings update. Neither checks a rejected Save. The existing app service validation establishes the precise deterministic rejection condition, but no desktop test currently checks post-error form state.

## Smallest offline widget witness

Extend the existing `threads_pages_tests.rs` Settings fixture/helpers (`fixture`, `update`, `click`; lines 15-86) with one widget journey: capture canonical settings; open Models; select High; change the model text; enter a relative auth-path string; click Save. Assert the reported absolute-path validation error, unchanged canonical settings/receipt history, the segmented effort restored to its pre-save canonical value, and the model/auth text inputs still holding their attempted values. This uses only the fresh synthetic workspace, performs no credential read or provider operation, and directly distinguishes the current partial reset. No build/test was run under this read-only task.

This is a test-only candidate selection, not implementation authorization. The lead owns the contract choice and any source/test assignment; native interaction remains separately deferred.
