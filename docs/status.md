# BRN Rust development status

Status: cloud and macOS Apple Silicon CLI build probes verified; private GitHub repository created. The chunk 01 provider trial is recorded in [the experimental App Server harness](../experiments/codex-app-server/README.md). The requested Chunk 02 provider lifecycle follow-up is recorded in [its evidence report](../experiments/codex-app-server/CHUNK-02.md). No desktop app build has been attempted.

## Design and implementation record

- A virtual Cargo workspace contains one `brn` binary crate. It has no external dependencies.
- The CLI accepts exactly one `--help` or `--version` argument. Missing, extra, and unknown arguments return a failure exit code and print an error.
- Rust 1.98.1 is pinned in `rust-toolchain.toml`; the official stable manifest version was verified before recording it. The profile is minimal, with rustfmt and Clippy components.
- The existing TypeScript BRN remains separate. No UI framework, agent runtime, storage, retrieval adapter, or user data migration is included.

## Verification and next step

- Verified on Linux x86_64 using an isolated installation of Rust 1.98.1: `cargo build --workspace --locked`, `cargo fmt --all -- --check`, and `cargo clippy --workspace --all-targets --locked -- -D warnings` all passed.
- `cargo test --workspace --locked` passed with zero unit tests. Five subprocess smoke checks passed: help and version succeed; missing, unknown, and extra arguments fail. The version output is `brn 0.0.0`.
- Private repository `ewq100/brn-rust` was created on GitHub on 2026-09-27. This starter records the verified cloud build probe and the remaining native verification work.
- Verified on macOS Apple Silicon (`Darwin arm64`) using Rust/Cargo 1.98.1: `cargo build --workspace --locked`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked` passed. Five CLI smoke checks passed: help and version succeeded; missing, unknown, and extra arguments failed.
- The App Server trial demonstrated managed-ChatGPT streamed text, one read-only fixture tool call, conversation continuation, server-reported interruption, and persisted thread resume across two App Server processes. Four credential-free protocol tests passed. See its README for versions, commands, security limits, and the App Server versus Rig recommendation.
- Desktop interaction and packaging remain unverified. The desktop window and editor belong to later chunks.

## Chunk 02 provider lifecycle follow-up

- Based on the actual clean, pushed `d3b2410` handoff, which already extended `c8f01c8` with App Server-process resume.
- Separate harness invocations now persist and resume a synthetic conversation using a versioned private state file. Missing/invalid/uncertain state fails closed.
- Existing managed ChatGPT sign-in and one supported proactive refresh succeeded without exposing credentials or changing account settings. Natural expiry/revocation remains unverified.
- Installed Apple Silicon App Server launch/initialization/shutdown succeeded outside the command sandbox. No sidecar redistribution or signed app packaging is included.
- The original editor trial numbered 02 in the roadmap remains pending. Full commands, final check counts, review outcomes and remaining limitations live in the evidence report.
