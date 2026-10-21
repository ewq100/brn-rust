# Helper effort reset correction

Branch: `codex/settings-draft-isolation`  
Base: test-only regression `29a02b4f52506001bc79be69a2c1c78ae7a210d4`  
Commit: `fde0477385639913aa9e8dcf39981f95c0aff8d2` (`Reset helper effort when disabled`)

Only `crates/brn-desktop/src/threads_tiles.rs` changed. Selecting helper Off now clears the helper provider and resets the draft helper effort to High. Selecting Codex or Copilot directly continues to preserve the current effort. This matches the previous behavior where turning a helper off and then re-enabling it created a new High-effort helper.

The lead reproduced the regression against the preceding candidate: the existing helper-save expectation observed Low after Low → Off → Codex. The unchanged test remains the witness. This correction was not rebuilt or tested here, per assignment.

Pinned `cargo-fmt --check -p brn-desktop` and `git diff --check` passed. The corrected candidate is ready for independent final-delta review and the lead's final qualification.
