# Persistent anchored comment verification evidence

2026-09-28 · `feature/anchored-comments` · base main `1b6623a`.

## Execution and baseline

The requested checkout was clean at `1b6623a7b1f08602a84d3cabc88acdd86e57ca0f`. Work is isolated in the native-managed `anchored-comments/brn-rust` worktree. Astra owns architecture and independent review, Sol implementation, Luna narrow checks, and the controller native acceptance and integration. The user authorized routine decisions, commit and feature-branch push; merge and release are excluded.

Luna ran `cargo test --workspace --locked --offline` with Rust 1.98.1, `PATH=/opt/homebrew/opt/rustup/bin:$PATH`, `CARGO_TARGET_DIR=/private/tmp/brn-editor-trial/target`, `CARGO_BUILD_JOBS=4`, and `CARGO_INCREMENTAL=0`: 79 tests passed, zero failed. Baseline log: `/private/tmp/brn-chunk12-baseline.log`. Disk inspection showed approximately 2.6 GiB free before and after baseline.

The existing editor experiment and [chunk 11 evidence](draft-revisions-evidence.md) inform the interaction design; their in-memory full-buffer history is not a production persistence model. Initial native access reported a locked Mac; the user was asked to unlock while independent work continued. Native acceptance has not yet run for this chunk.

## Scope and qualification

Comment-batch generation, candidate adoption and publication are outside this chunk. Existing trial workspaces and the original vault must remain untouched. Native acceptance will use a fresh binary and a new disposable synthetic workspace. Subjective usability, IME/accessibility, and sustained near-limit performance remain unqualified.
