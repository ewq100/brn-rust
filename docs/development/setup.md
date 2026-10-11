# Development setup

Read [AGENTS.md](../../AGENTS.md), [status](../status.md) and the [active plan](../work/active/pre-release-cleanup/plan.md). Inspect local/remote state and preserve unrelated work before choosing an isolated task branch/worktree.

## Check the machine

```sh
python3 scripts/development-preflight.py --native
```

The read-only preflight reports checkout identity, Git/Bash/Python/Cargo/Rustup, the pinned compiler, protoc and relevant Mac compiler tools. Missing optional workflow skills are informational.

[rust-toolchain.toml](../../rust-toolchain.toml) is compiler authority; it pins Rust **1.98.1** with rustfmt and Clippy. [Cargo.lock](../../Cargo.lock) records dependencies. When compiler installation belongs to the machine setup assignment:

```sh
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
cargo fetch --locked
```

Keep compiler/dependency changes separately scoped and qualified. Native GPUI targets macOS Apple Silicon first and requires an unlocked graphical session for interaction checks. Linux headless checks do not establish native readiness.

## Isolate build and application data

Use a task-owned Cargo target directory or a known compatible shared cache, plus a fresh explicit absolute BRN `--data-dir` outside Git. Use synthetic material. The core checks directory marker, application ID and supported schema before opening; incompatible old BRN data refuses without migration.

Build the maintained intake helper before Mac workspace tests:

```sh
cargo build --locked -p brn-intake --features helper --bin brn-intake-helper
```

PDF qualification needs Poppler `pdftotext` and `pdfimages`. Select explicit `BRN_INTAKE_HELPER`, `BRN_PDFTOTEXT` and `BRN_PDFIMAGES` executable paths for reproducible checks; [verification](verification.md) describes gates. Default checks do not include every optional native feature.

Provider/account checks follow the current assignment and normal authentication flow; keep credentials out of outputs. Successful builds and component tests are separate from native usability and owner acceptance.
