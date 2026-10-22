# Recovery-copy private-boundary witness

- Checkout: `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`
- Branch: `codex/recovery-copy-privacy`
- Base: `13813f6ff827de776a663036f5d468500e7a5ed1` (`origin/main` as verified before branch creation)
- Candidate: `2ff42451adf97268884afd3a0b91c3ec97922e3f`
- Changed path: `crates/brn-desktop/src/threads_polish_tests.rs` only
- Focused witness: `stale_private_note_copy_stays_out_of_general_knowledge`
- Patch: `/tmp/brn-recovery-copy-privacy-witness.patch`
- Patch SHA-256: `3877096527a860a64ad7f8c3a1bb7395eaea386e225196eb6e1df9e9dca91e4a`

The test creates a synthetic chat-scoped Note through the checked owner change path, verifies it is visible in that chat and invisible in general scope, then advances its saved markdown before typing distinct multiline Unicode recovery text. It focuses the real editor, sends actual window input containing `õun 私密` plus a distinct marker, verifies the stale/dirty row and visible `save-buffer-copy` control, and clicks that control. The postcondition is policy-neutral: either a future safe private copy or a refusal that retains the draft is acceptable, but the private text may not appear in general `Workspace::notes`, `records_for_chat(None)`, general search, or general export. It also checks that the newer canonical private source is unchanged and chat-visible and that the exact original edit session remains available.

The existing general stale-copy positive control in `the_four_note_banners_stay_visible_in_write_at_900x600` remains unchanged. No production code or fixtures were added. This is a before-source regression witness; no test result is claimed, and the lead owns its single baseline execution.

Validation performed: `git diff --check` passed; final checkout status was clean; `codex/indexed-settings-read` still points to `eae61dde04cda19c764246993966b13b997014eb`. The initial direct command `rustup run 1.98.1 cargo fmt --all -- --check` returned 101 because cargo-fmt was not on PATH. Without installing anything, I sourced `/tmp/brn-launcher-focus-qualification/environment.sh`, ran `cargo fmt --all`, then `cargo fmt --all -- --check`; both completed successfully. No build or test command was run.

Correction after lead inspection: added literal Unicode `õun 私密` to the actual editor input and a distinctive `private recovery copy entered general Notes` message to the general Notes assertion. The focused test remains `stale_private_note_copy_stays_out_of_general_knowledge`; no other behavior or files changed.
