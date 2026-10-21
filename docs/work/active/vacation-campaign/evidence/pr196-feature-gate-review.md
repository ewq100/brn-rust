# PR 196 feature-gate correction review

Result: **no actionable findings** in clean frozen candidate `6fb5323fa37c1def0f46b2979a480c173a70b0f1`. The two item-level feature gates match the existing native consumer boundary. Final execution checks remain lead-owned and are not inferred from this source review.

## Identity and actual delta

- Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/settings-draft-isolation`.
- Verified baseline/merge base: `d2967cdd787459dd01ed291219b9045155b18511`.
- Previous PR head: `7665a5766f109bf2db947887d6008c2f2c506d7c`.
- Reviewed final HEAD: `6fb5323fa37c1def0f46b2979a480c173a70b0f1`; no staged, unstaged or untracked files reported.
- Complete baseline-to-final candidate remains five Settings files, 166 insertions/44 deletions. The entire previous-head-to-final delta is exactly two additions in `crates/brn-desktop/src/threads_state.rs`: `#[cfg(feature = "native-ui")]` on `ProviderForm` at line 14 and its implementation at line 22.
- Compared all four other Settings files directly against independently reviewed `c01ce1993217063ac63a53ee90750c8e208c737d`; there are no differences. The state file differs from that reviewed candidate only by these two gates. The prior complete-candidate review remains `/tmp/brn-settings-worker-retention-review.md`.

Final file SHA-256 values (under `crates/brn-desktop/src/`):

| File | SHA-256 |
| --- | --- |
| threads_state.rs | `3d6e3b94b3db401f42b39f20084930a6c6c1316dcfc774f7ecd8a9babf16cd72` |
| threads_tiles.rs | `7722d90fe3b2c342505d5d65edc4bc3578794966d5fa2a2fdaaa0a2dfa14e937` |
| threads_ui.rs | `a36ef22e5f8256afd00b582c59dee37aabc2a80d7441adfe5f6d60bb57433455` |
| threads_pages_tests.rs | `437e7053310935a0b7c032342f148b6b1ea9c590b09e8edd3409f774b7e8cfa2` |
| threads_tiled_tests.rs | `9c3922bdc7ec7975a78f2c98050fc5ad0ba05a41f12715c541a5cb04597f1c76` |

## Evidence and correctness

Read applicable AGENTS, current status/verification guidance, the existing review/Rust guidance, complete correction diff, desktop manifest, `main.rs`, the affected state module, native module wiring and all `ProviderForm` consumers. No source or shared record was changed.

The required Ubuntu Core log `/tmp/brn-pr196-ci-failure/ubuntu-core.log` shows `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` failing on the unconstructed `ProviderForm` and unused `from_settings`. `/tmp/brn-pr196-ci-failure/local-negative.log` independently contains the same two errors for the desktop test binary. These logs support the reported default all-target compilation failure; they are negative evidence, not a passing final check.

`main.rs:3-7` has two explicit routes. Without `native-ui`, only the test build includes `threads_state.rs` directly to exercise portable presentation/state rules; the native `threads_ui` consumer is absent. The new form and constructor have no consumers in those portable tests. Removing only these native-only items from that route addresses the two dead-code diagnostics while preserving the rest of the portable module and its tests. The `WorkspaceSettings` import remains necessary for ungated `delegation_status` at `threads_state.rs:40`; it is correctly retained.

With `native-ui`, `main.rs` includes `threads_ui`; that module includes the same file as `state` at `threads_ui.rs:68-69`. Both new gates are true, so the `ProviderForm` import, field, startup initialization, refresh, Models rendering, checked Save overlay and completion-tail retention remain present with identical bodies. `native-test-support` and `native-retrieval` both enable `native-ui` in the unchanged manifest. There is no feature combination among these declared routes that compiles a form consumer while omitting its type or constructor.

The correction does not suppress a lint, change CI commands or protection, omit portable tests, alter dependencies/features, or change persistence/runtime behavior. It expresses the existing responsibility of this UI form instead of weakening `-D warnings`. The previously reviewed isolation, helper Off reset, Connect exclusion, full-cache equality guard and restoration after the entire worker/queue completion tail have identical native source bytes. Prior review limits concerning refresh failure and native acceptance therefore remain applicable.

## Checks and limits

Read-only `git diff --check` on the full verified baseline-to-final delta passed. I ran no compiler, tests, formatting tool, network/provider/account action or native interaction, and no nested work. Source reasoning establishes that the gates match the actual consumer configuration; it does not establish passing final default/native checks or required CI. Lead-owned qualification and ordinary integration gates remain required. This report grants no new feature, owner acceptance, merge or release authority.
