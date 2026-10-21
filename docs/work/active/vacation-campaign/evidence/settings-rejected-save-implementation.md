# Retain Settings form after rejected Save

- Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`
- Branch: `codex/settings-save-rejection`
- Test-only witness: `2ea1c3752c307049e51a9e44c7b0e6e046b2d271`
- Implementation commit: `07e3b533a33955063563c1b38c6a91435bf6ff62`
- Changed files: `crates/brn-desktop/src/threads_ui.rs` and `crates/brn-desktop/src/threads_pages_tests.rs`.

`ThreadsUi::configure` now clones `ProviderForm` only when `Workspace::configure` returns an error, preserves the existing error reporting, performs the ordinary `refresh()` in all cases, then restores the form snapshot after that refresh. This keeps canonical settings/history authoritative while allowing a rejected visible attempt to be corrected and retried. Successful Save behavior is unchanged. The existing UI regression now also requires the corrected retry to persist parent effort `high`, alongside its model/helper/auth assertions.

Validation performed: pinned `cargo-fmt --check -p brn-desktop` and `git diff --check` passed. No build or tests were run; lead owns the final serialized qualification and independent review. The original negative witness remains at `/tmp/brn-settings-rejected-save-qualification/baseline.log`.
