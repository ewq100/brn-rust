# Development setup

Run commands from the repository root. [rust-toolchain.toml](../../rust-toolchain.toml) pins Rust 1.98.1 with rustfmt and Clippy. Use Rustup with that toolchain installed; preserve [Cargo.lock](../../Cargo.lock).

Run `python3 scripts/development-preflight.py` (add `--native` for native prerequisites)
to report this checkout and available tools before choosing checks. See
[tooling](tooling.md) for skill portability and retained evidence.

## Default workspace

```sh
cargo build --workspace --locked
cargo run -p brn -- --help
```

The workspace's default member is the `brn` CLI crate. Use `--workspace` for workspace verification. The verification scripts use `--offline`, so dependencies must already be cached; an offline cache miss is a setup problem, not a product test failure. Scripts also expect Bash and standard Unix utilities; end-to-end verification uses Python 3, not ripgrep.
Set TMPDIR to an existing explicitly disposable synthetic fixture parent
outside Git before running it; credential-path safety must remain enforced.

Set `CARGO_TARGET_DIR` to an absolute path if sharing a build cache. Native graphs are large; check disk capacity before native builds. Do not delete another task's build artifacts or data to make room without coordinating.

## Native desktop

The qualified trial target is macOS on Apple Silicon with Command Line Tools. Native interaction checks require an unlocked graphical session.

```sh
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
trial_data="$HOME/BRN-disposable-trial"
mkdir -p "$trial_data"
cargo run -p brn-desktop --features native-ui,native-retrieval --locked --offline -- --data-dir "$trial_data"
```

This launches the simple note editor/chat UI. Keep the disposable path for
restart checks; choose a fresh outside-repository directory for trials.
Without an explicit directory, startup uses the new
`~/Library/Application Support/BRN-simple` with the exact
`BRN-simple.credentials` sibling, never inspecting/copying/migrating old BRN.
Choose Vault/Refresh/Search are local; Decline optional model consent for a
zero-network keyword-only trial. Use synthetic Markdown for Save/Cmd-S,
Save Copy, compare/reload and restart recovery checks. Recovery stores unfinished
edits in BRN; only explicit Save writes Markdown. Guarded note-switch/close/Quit
wait for acknowledged recovery. Dock/system termination can lose unacknowledged
typing. Native acceptance and automated qualification are recorded separately
in [status](../status.md).

Native retrieval offers a freshly consented installer; startup never downloads.
An explicit `--model-dir /absolute/path/to/verified/model` loads existing
verified assets off GPUI, not a download destination.
Settings Connect/discovery/provider/model controls are explicit and need
separate live authorization during checks. ChatGPT qualification is conditional;
quota reset alone is not availability evidence.
Old/mixed database, sidecar and backup markers refuse before SQLite opens.
Retired commands/flags are unknown; old data stays untouched without migration.

[Root README](../../README.md) gives the local Finder launcher command. See [verification](verification.md) for checks; compiling does not verify native interaction.
