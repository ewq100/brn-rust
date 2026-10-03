# BRN Rust desktop

BRN is a local-first macOS notes trial: saved Markdown reads/search and explicit
Rig ChatGPT/Copilot chat, with a shared AppWorker for native and CLI consumers.
Legacy local editing, recovery, drafts, comments and history remain supported
in separate legacy data folders. Production App Server has been removed.
Simple proposal/approval tools and Markdown Save remain unimplemented; the old
Steps 5/6 are superseded by the current roadmap.

Start with [current status](docs/status.md) for implemented capabilities and qualification gaps. Agents should read [AGENTS.md](AGENTS.md); the [documentation index](docs/README.md) routes setup, architecture, testing and task history.

The original TypeScript BRN and vault remain separate. This is a private trial repository; no license or public release decision has been made. The frozen [target architecture](docs/architecture/overview.md#frozen-target) serves the [product vision](docs/product/BRN_PRODUCT_VISION.md); the [roadmap](docs/roadmap.md) now starts with safe Save/recovery, then legacy removal, scoped provider capability checks and whole proposals. The former publication/graph-engine roadmap is retired.

## Quick start

Use the Rust toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml):

```sh
cargo build --workspace --locked
cargo run -p brn -- --help
cargo run -p brn-workflow --bin brn-flow -- --help
bash scripts/verify-end-to-end.sh
```

Set `TMPDIR` to an explicit existing disposable synthetic fixture parent outside
Git before verification (credential safety refuses repository-local paths).
`brn` is the shared-workflow CLI; see its [reference](crates/brn/README.md).
`brn-flow` retains legacy local search/history only. The integrated script uses
cached dependencies, separate simple-vault and legacy-local fixtures, no accounts
or model assets. `--fixtures-only` avoids repeating a completed workspace gate;
`--retirement-only` checks production references without builds.
See [verification](docs/development/verification.md) for native-specific checks.

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
