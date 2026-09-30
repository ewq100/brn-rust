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

Create an explicit disposable data directory, then launch the writing/keyword-capable UI:

```sh
trial_data="$(mktemp -d "${TMPDIR:-/tmp}/brn-local.XXXXXX")"
cargo run -p brn-desktop --features native-ui --locked -- --data-dir "$trial_data"
```

For semantic/hybrid search and live grounded answers, use `--features native-retrieval` and provide the installed Codex executable and verified local model directory:

```sh
cargo run -p brn-desktop --features native-retrieval --locked -- \
  --data-dir /absolute/existing/trial-data \
  --codex /absolute/path/to/codex \
  --model-dir /absolute/path/to/verified/model
```

Choose a text/Markdown file, then explicitly import and approve it for search. Build the index, select a search profile and inspect source passages before asking from sources. Search approval does not authorize publication. Activity exposes saved conversations; reopen the same data directory to continue. Missing native resources do not silently substitute a different search profile.

Draft working copies are separate from immutable checkpoints and AI candidates. Acknowledged saves are durable; force termination does not promise preservation of unsaved text. Comments retain their exact original revision/quote and show deleted or ambiguous locations explicitly. User suitability, IME/accessibility and broader qualification remain open as recorded in [status](docs/status.md).

## Local Finder launcher

After building, create a new unsigned local app bundle with explicit paths:

```sh
cargo build -p brn-desktop --features native-retrieval --locked
bash scripts/make-macos-app.sh \
  --output /absolute/path/BRN-Trial.app \
  --binary /absolute/path/to/target/debug/brn-desktop \
  --data-dir /absolute/existing/trial-data \
  --codex /absolute/path/to/codex \
  --model-dir /absolute/path/to/verified/model
```

Use the actual `CARGO_TARGET_DIR` binary if sharing a build cache. The output bundle must not already exist. The launcher copies BRN; the provider and model stay at their explicit paths. Omit `--codex` for local-only work and `--model-dir` for keyword-only retrieval. This is a local trial launcher, not a signed distribution. [Usability evidence](docs/work/completed/desktop-usability/evidence.md) records its tested behavior.
