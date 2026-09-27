# BRN Rust desktop — chunk 00

This repository is the isolated Rust starting point for the planned BRN desktop application. The current deliverable is only a dependency-free build probe: a single `brn` command that accepts `--help` and `--version`. It has no desktop UI, model integration, retrieval, data store, or migration from the existing TypeScript project.

The intended application target is macOS on Apple Silicon. This initial command is platform-neutral so a Rust-capable cloud runner can verify the source build before native macOS work begins. The eventual architecture will be chosen through later trials, with Rig/Codex considered first for agent integration and interchangeable retrieval approaches evaluated separately.

The source lives in the separate private repository `ewq100/brn-rust`. No license or public release decision has been made.

## Build probe

Install the Rust toolchain selected in `rust-toolchain.toml` . From the repository root, run:

```sh
cargo build --workspace
cargo run -p brn -- --help
cargo run -p brn -- --version
```

Only those two flags succeed. Other invocations exit unsuccessfully. See [docs/status.md](docs/status.md) for what has been verified in this workspace and what remains for a Rust-capable runner.
