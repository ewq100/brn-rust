# BRN Rust desktop trials

This repository is the isolated Rust starting point for the planned BRN desktop application. The workspace now contains the original dependency-free `brn` build probe, a UI-independent `brn-core`, an optional native `brn-desktop` shell, and an independent `brn-store` SQLite backend. Separate experiments evaluate the subscription provider, a native Markdown/comment editor, and local retrieval; they are not integrated into the desktop shell. Production retrieval, desktop storage integration, and migration from the existing TypeScript project remain pending.

The intended application target is macOS on Apple Silicon. This initial command is platform-neutral so a Rust-capable cloud runner can verify the source build before native macOS work begins. The [approved architecture](docs/architecture-checkpoint.md) selects the application boundaries, with Codex App Server as the first provider route and replaceable local retrieval adapters. Native UI acceptance remains deferred.

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

The [retrieval adapter trial](experiments/retrieval-trial/README.md) exercises keyword, real FastEmbed/LanceDB semantic, and hybrid search on synthetic documents. [Evidence and limitations](docs/retrieval-trial-evidence.md) include local reopening, version/status filters, corruption checks, timings and dependency findings.

## Desktop shell (chunk 05)

The shell provides Workspace, Activity and Settings navigation and an in-memory sample task with progress/cancellation. It does not yet import documents, query models or persist work. Native UI acceptance remains pending; GPUI is provisional under the approved architecture.

```sh
# All lightweight core/CLI checks; no window opens.
bash scripts/verify-desktop-shell.sh
# Native build and the same headless checks; still no window opens.
bash scripts/verify-desktop-shell.sh --native
# Native launch. Explicit data directories must already exist.
cargo run -p brn-desktop --features native-ui -- --data-dir /absolute/existing/directory
```

Without `--data-dir`, native launch creates `~/Library/Application Support/BRN`. The shell checks writability with a temporary file that it removes; no documents or database are written. Headless checks require an explicit directory. [Shell plan](docs/desktop-shell-plan.md) and [verification/remaining native checks](docs/desktop-shell-evidence.md) record the current status.

## Storage and recovery (chunk 06)

`brn-store` provides authoritative source revisions, local session/message records, durable operation identities and recovery in an existing data directory. It is a library boundary; the desktop sample does not yet persist through it. The verification script builds the workspace, runs storage tests with disposable databases, and exercises existing CLI/headless paths:

```sh
bash scripts/verify-storage.sh
```

[Storage plan](docs/storage-recovery-plan.md) and [evidence/recovery limits](docs/storage-recovery-evidence.md) describe the checkpoint. No original documents or provider credentials are used.
