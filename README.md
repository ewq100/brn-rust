# BRN Rust desktop trials

This repository contains the Rust BRN personal desktop trial: selected Markdown/text import, explicit search approval, persistent versions, keyword/semantic/hybrid retrieval, grounded Codex App Server answers, exact source evidence and saved conversation reopening. The native UI and headless driver share the same workflow. The original CLI probe and standalone experiments remain available for regression and historical evidence. Writing/comments, publication, graph integration and migration from the existing TypeScript application are still later work.

The intended application target is macOS on Apple Silicon. This initial command is platform-neutral so a Rust-capable cloud runner can verify the source build before native macOS work begins. The [approved architecture](docs/architecture-checkpoint.md) selects the application boundaries, with Codex App Server as the first provider route and replaceable local retrieval adapters. The synthetic native workflow has passed import, retrieval, grounded answers, restart, cancellation and active-turn shutdown; broader editor and accessibility acceptance remains deferred.

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

## End-to-end personal trial

Build and verify the local flow without credentials or model downloads:

```sh
bash scripts/verify-end-to-end.sh
```

For the native application, create an absolute data directory first, then supply the installed Codex executable and a verified local model directory from the retrieval trial:

```sh
cargo run -p brn-desktop --features native-retrieval -- \
  --data-dir /absolute/existing/trial-data \
  --codex /absolute/path/to/codex \
  --model-dir /absolute/path/to/verified/model
```

Choose a `.md`/`.txt` file using **Choose file…**, review the selected path, then choose **Import and approve**. Build the index, select a profile, search and inspect passages, then ask from sources. Activity provides saved conversations and evidence snapshots. Reopen the same data directory to continue. Selecting a file does not import or approve it; search approval does not authorize publication.

For repeatable Finder launching, build once and generate a local app with explicit paths:

```sh
cargo build -p brn-desktop --features native-retrieval --locked
bash scripts/make-macos-app.sh \
  --output /absolute/path/BRN-Usability-Trial.app \
  --binary /absolute/path/to/target/debug/brn-desktop \
  --data-dir /absolute/existing/trial-data \
  --codex /absolute/path/to/codex \
  --model-dir /absolute/path/to/verified/model
```

Open the generated app in Finder. It retains the selected workspace and copies the built BRN executable; Codex and model assets remain at their specified paths. Use the actual `CARGO_TARGET_DIR` if you built into a shared cache. The output app must not already exist; generate a new output path after rebuilding. Omit `--codex` for local search only, and omit `--model-dir` for keyword-only indexing/search. Startup failures open a local log at `~/Library/Logs/BRN Usability Trial/startup.log`. This unsigned launcher is for local trials, not distribution. [Usability evidence](docs/desktop-usability-evidence.md) records observed checks and remaining limits.

The headless `brn-flow` driver exposes the same operations; `cargo run -p brn-workflow --features native-retrieval --bin brn-flow -- --help` lists its commands. Keyword-only builds work without model assets; semantic/hybrid never silently fall back when their resources are unavailable.

[Plan](docs/end-to-end-flow-plan.md) and [live evidence, commands and limitations](docs/end-to-end-flow-evidence.md) record the integrated flow. Live subscription answers and separate-process resume are verified on synthetic sources. Synthetic native rendering/input/close acceptance passed after unlock; GPUI remains provisional. This is not a signed or released application.

The earlier sample-shell checks remain available through `scripts/verify-desktop-shell.sh`. Without `--data-dir`, native launch uses `~/Library/Application Support/BRN` and now opens authoritative storage there. Use a disposable explicit directory for trials. Headless sample checks still require an explicit directory and perform no document writes.

## Storage and recovery (chunk 06)

`brn-store` provides authoritative source revisions, local session/message records, durable operation identities and recovery in an existing data directory. The shared workflow now uses this library for imported versions and conversation projections. The verification script builds the workspace, runs storage tests with disposable databases, and exercises existing CLI/headless paths:

```sh
bash scripts/verify-storage.sh
```

[Storage plan](docs/storage-recovery-plan.md) and [evidence/recovery limits](docs/storage-recovery-evidence.md) describe the checkpoint. No original documents or provider credentials are used.
