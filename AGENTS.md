# BRN

Rust workspace for BRN, a Mac-first notes and threads app with an AI agent. Crates live in `crates/` (`brn` CLI, `brn-desktop` native UI, `brn-threads-core`/`brn-threads-app` state and services, `brn-ai`, `brn-retrieval`, `brn-intake`). Shipped agent prompts live in `agent/`; they are product code, separate from this file.

## Product invariants

These protect user data; treat a violation as a bug.

- SQLite owns accepted state; search indexes are derived. Desktop, CLI and agent write through the same checked services.
- Preserve exact bytes, provenance, version checks, replay and Undo. Async results never silently overwrite later edits.
- Details: [invariants](docs/architecture/invariants.md), [behavior](docs/architecture/threads-behavior.md), [target](docs/architecture/threads-target.md), [threat model](docs/threat-model.md).

## Working

- Work on a branch or worktree per task and open a PR into `main`.
- Use fresh synthetic `--data-dir` folders; leave the owner's real BRN data untouched. Keep credentials out of logs, PRs and commits.
- You may run the app, use computer use, call AI providers and rebase your own branch, pushing with `--force-with-lease`. `main` changes only through merged PRs.
- `rust-toolchain.toml` and `Cargo.lock` pin the toolchain and dependencies.

## Checks

- Run the smallest check that covers the change; CI runs the full set. The [verification guide](docs/development/verification.md) maps changes to checks and scripts.
- Workspace tests need the two document helpers built first; see [setup](docs/development/setup.md).
- If tools are missing: `python3 scripts/development-preflight.py --native`.

## PRs

Merge when required CI is green and review findings are fixed or answered.

## Code review rules

Use the [invariants](docs/architecture/invariants.md) as the behavior contract and the [threat model](docs/threat-model.md) for security context. Flag actionable regressions introduced by the change:

- **Accepted work:** bypasses of the shared checked writer, lost or stale edits, duplicate mutations on retry, unsafe schema upgrades, Undo or recovery that erases later work.
- **Authority and privacy:** model- or source-controlled grants, approval reused for a changed candidate, private content escaping its scope, exposed credentials, tool dispatch after authority is revoked.
- **Async and integration:** hangs, duplicate effects or silent provider/model fallback in Stop/Resume, queued messages, delayed results and provider/tool failures.

Explain the triggering path and impact for each finding. Leave formatting and lint to CI; skip speculative findings and blanket test demands.
