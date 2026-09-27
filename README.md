# BRN Rust desktop trials

This repository is the isolated Rust starting point for the planned BRN desktop application. The production workspace remains a dependency-free build probe: a single `brn` command that accepts `--help` and `--version`. Separate experiments evaluate the subscription provider and a native Markdown/comment editor; they are not integrated into a production app. Retrieval, authoritative storage, and migration from the existing TypeScript project remain pending.

The intended application target is macOS on Apple Silicon. This initial command is platform-neutral so a Rust-capable cloud runner can verify the source build before native macOS work begins. The eventual architecture will be chosen through later trials, with Rig/Codex considered first for agent integration and interchangeable retrieval approaches evaluated separately.

The source lives in the separate private repository `ewq100/brn-rust`. No license or public release decision has been made.

## Build probe

Install the Rust toolchain selected in `rust-toolchain.toml` . From the repository root, run:

```sh
cargo build --workspace
cargo run -p brn -- --help
cargo run -p brn -- --version
```

Only those two flags succeed. Other invocations exit unsuccessfully. See [docs/status.md](docs/status.md) for native macOS verification, and [the chunk 01 provider trial](experiments/codex-app-server/README.md) for the separate experimental Rust harness.

The [native editor trial](experiments/editor-trial/README.md) builds on Apple Silicon and provides in-memory editing, selected-passage comments and revision diffs. Native interaction and user acceptance remain separate verification gates; see [its evidence](experiments/editor-trial/EVIDENCE.md).
