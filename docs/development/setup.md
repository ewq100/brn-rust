# Development setup

Run commands from the repository root. [rust-toolchain.toml](../../rust-toolchain.toml) pins Rust 1.98.1 with rustfmt and Clippy. Use Rustup with that toolchain installed; preserve [Cargo.lock](../../Cargo.lock).

## Default workspace

```sh
cargo build --workspace --locked
cargo run -p brn -- --help
cargo run -p brn-workflow --bin brn-flow -- --help
```

The workspace's default member is only the `brn` probe. Use `--workspace` for workspace verification. The verification scripts use `--offline`, so dependencies must already be cached; an offline cache miss is a setup problem, not a product test failure. Scripts also expect Bash and standard Unix utilities; end-to-end verification uses Python 3 and ripgrep.

Set `CARGO_TARGET_DIR` to an absolute path if sharing a build cache. Native graphs are large; check disk capacity before native builds. Do not delete another task's build artifacts or data to make room without coordinating.

## Native desktop

The qualified trial target is macOS on Apple Silicon with Command Line Tools. Native interaction checks require an unlocked graphical session.

```sh
cargo build -p brn-desktop --features native-ui --locked
trial_data="$(mktemp -d "${TMPDIR:-/tmp}/brn-local.XXXXXX")"
cargo run -p brn-desktop --features native-ui --locked -- --data-dir "$trial_data"
```

This launches the local writing/keyword-capable UI against a new disposable directory. Keep the directory path if you need restart checks. Without an explicit directory, the app can open the default real workspace; always supply one for trials.

For semantic/hybrid retrieval, build with `--features native-retrieval` and pass `--model-dir /absolute/path/to/verified/model`. For live grounded answers, pass `--codex /absolute/path/to/codex` under the task's authorization. Use [retrieval trial setup](../../experiments/retrieval-trial/README.md) for model/runtime requirements and [provider packaging boundaries](../../experiments/codex-app-server/PACKAGING.md) for the installed sidecar. Neither executable nor model is bundled by these commands.

[Root README](../../README.md) gives the local Finder launcher command. See [verification](verification.md) for checks; compiling does not verify native interaction.
