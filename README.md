# BRN Rust desktop

BRN is a local-first personal desktop trial for importing Markdown/text, retrieving attributable passages, asking grounded questions, drafting and reviewing immutable revisions and anchored comments. It targets macOS on Apple Silicon, with a shared workflow used by the native app and headless driver.

Start with [current status](docs/status.md) for implemented capabilities and qualification gaps. Agents should read [AGENTS.md](AGENTS.md); the [documentation index](docs/README.md) routes setup, architecture, testing and task history.

The original TypeScript BRN and vault remain separate. This is a private trial repository; no license or public release decision has been made. Comment-batch revision generation, candidate adoption, publication, graph integration and distributable packaging remain future work in the [roadmap](docs/roadmap.md).

## Quick start

Use the Rust toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml):

```sh
cargo build --workspace --locked
cargo run -p brn -- --help
cargo run -p brn-workflow --bin brn-flow -- --help
bash scripts/verify-end-to-end.sh
```

`brn` is the agent-facing CLI over the shared workflow; see its [command reference](crates/brn/README.md) for subcommands and the `--json` envelope. `brn-flow` is the integrated headless driver. The verification script uses cached dependencies, synthetic data and no live provider or model assets. See [setup](docs/development/setup.md) for prerequisites and [verification](docs/development/verification.md) for feature-specific checks.

## Native personal trial

Create an explicit disposable data directory outside Git repositories, then
launch the simple saved-note reader/chat UI:

```sh
trial_data="$HOME/BRN-disposable-trial"
mkdir -p "$trial_data"
cargo run -p brn-desktop --features native-ui,native-retrieval --locked -- --data-dir "$trial_data"
```

The normal native desktop includes `native-ui,native-retrieval`; enabling the
installer does not download a model. Choose a vault, read/refresh/search saved
Markdown and explicitly select an account/provider/model in Settings before Ask.
Account Connect and model Download require explicit actions; offline trials
must not click them. Declining model consent creates no network request and
keeps keyword-only retrieval. The simple reader has no Markdown Save.

Without an explicit data directory, native startup uses
`~/Library/Application Support/BRN-simple`, with `BRN-simple.credentials` next to
it. It never copies or inspects old BRN data. `--legacy` explicitly selects old
BRN for local editing/recovery/history only; legacy AI and `--codex` are retired.
Explicit directories honor mode markers, including sidecars/backups; mixed or
incompatible `--legacy`/`--vault` launches refuse before opening authority.

Draft working copies are separate from immutable checkpoints and AI candidates. Acknowledged saves are durable; force termination does not promise preservation of unsaved text. Comments retain their exact original revision/quote and show deleted or ambiguous locations explicitly. User suitability, IME/accessibility and broader qualification remain open as recorded in [status](docs/status.md).

## Local Finder launcher

After building, create a new unsigned local app bundle with explicit paths:

```sh
cargo build -p brn-desktop --features native-ui,native-retrieval --locked
bash scripts/make-macos-app.sh \
  --output /absolute/path/BRN-Trial.app \
  --binary /absolute/path/to/target/debug/brn-desktop
```

Use the actual `CARGO_TARGET_DIR` binary if sharing a build cache. The output bundle must not already exist. The launcher copies only the binary, never old data. Omit `--data-dir` for the
new BRN-simple default, or supply an existing disposable absolute directory.
`--legacy` is explicit old/local mode. An optional `--model-dir` is an explicitly
verified existing model, not a download destination. This is a local trial launcher, not a signed distribution. [Usability evidence](docs/work/completed/desktop-usability/evidence.md) records its tested behavior.
