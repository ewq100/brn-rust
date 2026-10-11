# BRN Threads development setup

The implemented Threads baseline merged in PR #114 at `d768db7`. Read [AGENTS.md](../../AGENTS.md), [status](../status.md), and the [current plan](../work/active/threads-rebuild/phase0-plan.md) before running product commands.

## Isolate the current assignment

Inspect the existing checkout and remote main before choosing the task baseline. Create a new sibling worktree on a task-specific branch; preserve existing worktrees and unrelated edits. The current assignment uses `work/phase0-stale-input` from verified main `d768db7`. The old `rebuild/threads` branch is historical, not a required destination for new changes.

## Check this machine

```sh
python3 scripts/development-preflight.py --native
```

This existing read-only preflight reports Git, Bash, Python, Cargo/Rustup, the pinned toolchain, protoc, and Mac compiler tools when relevant. Missing optional workflow skills are informational.

Use [rust-toolchain.toml](../../rust-toolchain.toml), which currently pins Rust 1.98.1 with rustfmt and Clippy. If Rustup is already installed and installing the pinned compiler is within the machine setup assignment:

```sh
rustup toolchain install 1.98.1 --profile minimal --component rustfmt --component clippy
cargo fetch --locked
```

Rust 1.99.0 is the selected qualification target in the [dependency plan](../work/active/threads-rebuild/plan.md#rust-1990-as-the-build-baseline). During the assigned build, install that exact toolchain with the same components and qualify it against the unchanged lockfile before adopting its pin. Update actively retained scripts that currently force `cargo +1.98.1` so local checks and CI use the same repository-selected compiler. The pin and installation command above still describe the actual baseline; this handoff has not performed the upgrade.

Follow the planned qualification before changing the pin or removing dependency patches. Resolve system prerequisites through the available machine/package setup; report privilege or account requirements specifically. Native GPUI verification targets macOS Apple Silicon first and requires an unlocked graphical session for interaction checks. A Linux checkout can perform supported headless work without establishing native readiness.

## Keep build and app data isolated

Choose a task-owned Cargo target directory and a fresh explicit BRN data directory outside Git. Never point rebuild binaries at existing BRN data or use the old app's default directory.

The core checks a distinct marker, application ID and supported schema before opening data. Old-format data is not imported or migrated. When running existing verification scripts, read their fixture-parent requirements and use synthetic material.

Run provider authentication/model availability checks early in the build assignment when bounded live tests are authorized. Use the selected provider's normal authentication flow, never credential dumps. If sign-in needs the owner, report it and continue independent offline work.

## Current commands

The retained manifests and verification scripts build the implemented Threads workspace. Intake fixture qualification requires the maintained helper and explicit Poppler paths as described in [verification](verification.md). A successful build is not proof of native usability or owner acceptance. Historical standalone experiments are not production engines.
