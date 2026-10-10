# BRN Threads setup evidence

**Date:** 10 October 2026. **Scope:** repository preparation and handoff, not product implementation.

## Observed baseline

- Remote main: `af9239c741c7ab0983e62f0253e607b9607727e6`.
- Base tree: `b2d7917d15b0fd7a83b436a1f5462a5cf729b3e4`.
- Main includes PR #113. Existing branches and open PRs were inspected; none was merged, closed, deleted, or force-updated by this setup.
- A fresh clone was made from main and local branch `rebuild/threads` was created. The earlier checkout on `ci/hosted-platform-checks` was preserved.
- Root `AGENTS.md` was the only preexisting tracked agent instruction file. Small Claude/Copilot entry points now refer to the same new guide.

## Changes prepared

The latest proposal, including full readable imports and on-demand Markdown export, becomes one canonical Threads target. Current product, architecture, workflow, status, roadmap, and active-work entry points route to it.

Baseline crate/dependency documentation is labeled as reference. Links in retained current baseline documents that depended on overwritten old contract sections now point to the same files at the fixed baseline commit. Historical evidence was not rewritten as current results.

The setup adds the build plan, review brief, pending review record, and exact build prompt. Product source, Cargo manifests/lockfile, provider patches, CI workflow, and repository protection are unchanged.

## Checks

| Check | Observed result |
|---|---|
| Remote baseline and branch inventory | Completed through the repository connection. No existing rebuild/threads branch was observed before preparation. |
| Isolated checkout identity | Main baseline confirmed before local branch creation and setup edits. |
| Development preflight with native flag | Exited 1: Git, Bash, and Python available; Cargo, Rustup, pinned Rust, and protoc absent. This is an environment prerequisite gap, not a product test failure. No Mac native session is available here. |
| Git whitespace check | Passed: git diff --check returned 0. |
| Current Markdown file/fragment links | Passed: 84 files, 408 local links, 0 failures. |
| Setup scope and consistency review | Completed in a separate read-only agent context. One fetch-ref correction was accepted: the worktree setup now fetches explicitly into origin/rebuild/threads, including for a single-branch clone. No other concrete setup blockers were found. This was not the external Opus/Astra architecture review. |
| Rust compilation, model tests, native interaction | Not run: no product code changed, and this environment lacks required tooling/native resources. |
| Independent external Opus/Astra review | Not performed. The supplied brief enables the owner's next review assignment. |
| Remote publication | To be verified against the published branch and commit before handoff. |

## Build machine handoff

The build agent must fetch `rebuild/threads`, inspect current HEAD and [review status](review.md), and run the [setup preflight](../../../development/setup.md) on its own machine. Use a fresh data directory and separate Cargo target. This evidence does not claim that the owner's Mac, credentials, model access, or new core are configured or verified.

