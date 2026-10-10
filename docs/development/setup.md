# BRN Threads development setup

The durable handoff is repository `ewq100/brn-rust`, branch `rebuild/threads`. A local checkout created elsewhere is not evidence that this machine has its toolchain, account, or native session ready.

## Open the prepared branch

For an existing BRN clone, inspect its current work before fetching. Use a new sibling worktree if this branch is not already checked out:

```sh
git status --short --branch
git worktree list
git remote set-branches --add origin rebuild/threads
git fetch origin refs/heads/rebuild/threads:refs/remotes/origin/rebuild/threads
git worktree add ../brn-threads-rebuild --track -b rebuild/threads origin/rebuild/threads
```

Registering the branch adds its fetch/tracking mapping without removing existing mappings; this also supports clones originally limited to main. The final command is for a clone without a local `rebuild/threads` branch and an unused destination. If it already exists, use the appropriate existing worktree or add a worktree for that branch. Never reset, clean, or overwrite another task to make the example work.

Alternatively, use a fresh clone:

```sh
git clone --branch rebuild/threads --single-branch https://github.com/ewq100/brn-rust.git brn-threads-rebuild
cd brn-threads-rebuild
```

Read [AGENTS.md](../../AGENTS.md), [status](../status.md), and [the plan](../work/active/threads-rebuild/plan.md) before running product commands.

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

Do not silently upgrade the pin or remove dependency patches. Resolve system prerequisites through the available machine/package setup; report privilege or account requirements specifically. Native GPUI verification targets macOS Apple Silicon first and requires an unlocked graphical session for interaction checks. A Linux checkout can perform supported headless work without establishing native readiness.

## Keep build and app data isolated

Choose a task-owned Cargo target directory and a fresh explicit BRN data directory outside Git. Never point rebuild binaries at existing BRN data or use the old app's default directory.

The first core implementation must give the new schema an unmistakable identity and explicit safe startup behavior. Old-format data is not imported or migrated. When running existing verification scripts, read their fixture-parent requirements and use synthetic material.

Run provider authentication/model availability checks early in the build assignment when bounded live tests are authorized. Use the selected provider's normal authentication flow, never credential dumps. If sign-in needs the owner, report it and continue independent offline work.

## Existing commands are baseline commands

Before the workspace is replaced, the existing Cargo manifests and scripts still build/test baseline BRN. Baseline conversion checks require building the current brn-intake-helper as documented in its crate; its installation requirement remains until that conversion path is replaced. Their success is not proof of the new target. Update commands and CI as part of the relevant implementation change. [Verification](verification.md) and [the build plan](../work/active/threads-rebuild/plan.md) identify the new proofs.
