# Note reload generation guard witness

- Test: `note_newer_buffer_generation_refuses_save_and_discard_until_explicit_recovery`
- Commit: `28bb965c113b0896baabfe89f9d1008ebba14a8e` (`test: guard newer note generation transitions`)
- Changed file: `crates/brn-writing-ui/examples/writing_host/journey.rs`
- Branch: `codex/note-reload-generation-guard`; initial clean HEAD observed as `8c18845ca5b70a43d36b0bc0d2811da573086a79`; final worktree clean at the test commit.

The existing witness now checks ordinary same-note reload and navigation to a second synthetic Note after a newer generation has advanced in the same durable session. Both replacement attempts must fail while retaining the old visible text, local session, open record, canonical Note, recovery buffer, and history. It then confirms explicit same-note Recover adopts the newer generation and that ordinary navigation succeeds afterward with that recovery buffer intact. Existing Save refusal and Discard refusal assertions remain in place.

`git diff --check` passed. No test/build/native/provider/account/network/dependency/toolchain action was run; the lead must run the witness against current production code to establish the red baseline and proceed with implementation.
