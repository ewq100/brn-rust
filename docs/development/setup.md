# Development setup

Run commands from the repository root. [rust-toolchain.toml](../../rust-toolchain.toml) pins Rust 1.98.1 with rustfmt and Clippy. Use Rustup with that toolchain installed; preserve [Cargo.lock](../../Cargo.lock).

## Default workspace

```sh
cargo build --workspace --locked
cargo run -p brn -- --help
cargo run -p brn-workflow --bin brn-flow -- --help
```

The workspace's default member is the `brn` CLI crate. Use `--workspace` for workspace verification. The verification scripts use `--offline`, so dependencies must already be cached; an offline cache miss is a setup problem, not a product test failure. Scripts also expect Bash and standard Unix utilities; end-to-end verification uses Python 3 and ripgrep.

Set `CARGO_TARGET_DIR` to an absolute path if sharing a build cache. Native graphs are large; check disk capacity before native builds. Do not delete another task's build artifacts or data to make room without coordinating.

## Native desktop

The qualified trial target is macOS on Apple Silicon with Command Line Tools. Native interaction checks require an unlocked graphical session.

```sh
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
trial_data="$HOME/BRN-disposable-trial"
mkdir -p "$trial_data"
cargo run -p brn-desktop --features native-ui,native-retrieval --locked --offline -- --data-dir "$trial_data"
```

This launches the simple saved-file reader/chat UI. Keep the disposable path for
restart checks; choose a fresh outside-repository directory for trials.
Without an explicit directory, startup uses the new
`~/Library/Application Support/BRN-simple` with the exact
`BRN-simple.credentials` sibling, never inspecting/copying/migrating old BRN.
Choose Vault/Refresh/Search are local; Decline optional model consent for a
zero-network keyword-only trial. Simple Markdown Save remains future work.

Native retrieval offers a freshly consented installer; startup never downloads.
An explicit `--model-dir /absolute/path/to/verified/model` loads existing
verified assets off GPUI, not a download destination.
Settings Connect/discovery/provider/model controls are explicit and need
separate live authorization during checks. ChatGPT qualification is conditional;
quota reset alone is not availability evidence.
Use `--legacy` with a separate disposable directory for legacy local
editing/recovery/history. Without a directory, `--legacy` explicitly opens old
BRN; never use that real-data route for checks. Legacy AI/`--codex` are retired.
Explicit markers/sidecars/backups enforce mode, and incompatible
`--legacy`/`--vault` arguments refuse before database work.

[Root README](../../README.md) gives the local Finder launcher command. See [verification](verification.md) for checks; compiling does not verify native interaction.
