# Export interruption witness implementation

- Checkout: `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`
- Branch: `codex/export-interruption-witness`
- Base: `9e2a81c0e3d5fbc1b617fe54bdb4b103f607b822`
- Candidate: `7b77eeb42577cce50748487b9bd77a0643c3737a`
- Changed file only: `crates/brn-threads-app/src/export.rs` (115 added test lines; no production changes)
- File SHA-256: `079d6c504f35b08f79ee95ae9051c8408d903abbe624fbd63e954bd4cd68ae84`

The Unix-only test creates one fresh workspace and general synthetic Note under a temp root separate from the export parent. It re-executes only its own test as a child, waits at most five seconds for the existing first `assets/` directory-flush seam, and terminates/reaps that owned process with `Child::kill` (asserted as `SIGKILL`). An RAII child guard kills and reaps on assertion panic or timeout. It verifies the final destination is absent after interruption, reopens the source, compares all canonical records and receipts to the pre-interruption snapshot, and retries through public `Workspace::export_now` at the same destination. The retry checks manifest format, note identity/path/version, independently recomputed content hash, on-disk manifest fields, and exact expected Markdown bytes. Temp-root cleanup owns any abandoned stage; the test introduces no stage sweeper or cleanup policy.

Validation performed: pinned `cargo fmt --all -- --check` passed after formatting only the target file with pinned Rust 1.98.1 `rustfmt`; `git diff --check` passed. The test/build was not run because the lead owns the serialized shared target. No provider, account, model, network, native, or data outside the fresh test temp root was used. A SIGKILL/process interruption witness is not power-loss or filesystem hardware qualification.

Commit contains exactly the requested one-file test-only change. The commit command reported Git's auto-configured committer identity (`Evo Kessler <evokessler@Evos-Mac-mini.lan>`); no identity changes were made. Working tree was clean after commit.
