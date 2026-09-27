# Chunk 00 status

Status: cloud build probe verified; private GitHub repository created; macOS verification pending. Target: macOS on Apple Silicon. First agent/provider trial: Rig with the user's Codex subscription, subject to verifying an actual authentication route in chunk 01.

## Design and implementation record

- A virtual Cargo workspace contains one `brn` binary crate. It has no external dependencies.
- The CLI accepts exactly one `--help` or `--version` argument. Missing, extra, and unknown arguments return a failure exit code and print an error.
- Rust 1.98.1 is pinned in `rust-toolchain.toml`; the official stable manifest version was verified before recording it. The profile is minimal, with rustfmt and Clippy components.
- The existing TypeScript BRN remains separate. No UI framework, agent runtime, storage, retrieval adapter, or user data migration is included.

## Verification and next step

- Verified on Linux x86_64 using an isolated installation of Rust 1.98.1: `cargo build --workspace --locked`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --locked -- -D warnings` all passed.
- `cargo test --workspace --locked` passed with zero unit tests. Five subprocess smoke checks passed: help and version succeed; missing, unknown, and extra arguments fail. The version output is `brn 0.0.0`.
- Private repository `ewq100/brn-rust` was created on GitHub on 2026-09-27. This starter records the verified cloud build probe and the remaining native verification work.
- macOS Apple Silicon compilation and desktop interaction remain unverified. Identify a target-OS runner for native build proof and the later editor trial.
- After build verification, chunk 01 can test the Codex subscription route with Rig. The desktop window and editor belong to later chunks.
