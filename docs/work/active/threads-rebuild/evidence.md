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
| Setup scope and consistency review | Completed in a separate read-only agent context. A fetch-ref correction was accepted. The actual single-branch checkout additionally exposed a missing tracking mapping; the setup now registers the branch and fetches into its explicit tracking ref. Both fetch and upstream setup succeeded after correction. No other concrete setup blockers were found. This was not the external Opus/Astra architecture review. |
| Rust compilation, model tests, native interaction | Not run: no product code changed, and this environment lacks required tooling/native resources. |
| Independent external Opus/Astra review | Not performed. The supplied brief enables the owner's next review assignment. |
| Remote publication | Setup commit c7e3eda380c1fc069ff4bafbb162c4e56557a95d was published on rebuild/threads. Its complete tree matched the local staged files exactly. This evidence update and the tracking correction are a follow-up documentation commit; use the branch head for the latest handoff. No main merge or release is implied. |

## Build machine handoff

The build agent must fetch `rebuild/threads`, inspect current HEAD and [review status](review.md), and run the [setup preflight](../../../development/setup.md) on its own machine. Use a fresh data directory and separate Cargo target. This evidence does not claim that the owner's Mac, credentials, model access, or new core are configured or verified.

## Review reconciliation and behavior design, 10 October 2026

This later update starts from review commit `a2af886e5442af0cfa4b8884181446c7d98d1eea`, which changed only the independent report after setup. The original setup evidence above describes that earlier assignment. The report names Claude Opus 5.5 as reviewer and assesses setup commit `3519bfb85e3c68cb17dd6df31061e9f5dc155e73`; its original findings are preserved, with lead dispositions appended separately.

The owner clarified that mathematical equation imports are not important, asked for concrete thread and agent-guide behavior, and pointed to new Rig and GPUI Kit releases. The target, plan, current summaries and build prompt now incorporate those decisions. The new behavior design specifies thread/attention/run/Action distinctions, a small packaged guide catalog and four planned SKILL.md playbooks. This task created design documentation, not the runtime guide files or their loader.

Three bounded assessments examined core-review corrections, Rig reuse and GPUI/import reuse. Root reconciled their findings and inspected official release/source evidence. Current pins remain Rig 0.43.0 and Kit 0.6.6. Rig 0.44.0 and Kit 0.7.1 are qualification targets, not completed upgrades. The direct shared ChatGPT page could not be fetched; prior-context retrieval recovered the earlier Rig recommendation and its important retry/logging claims were checked against release source.

Documentation checks were run after the changes; see the final recorded result below. All changed paths in this update are Markdown. No product source, dependency pin/lockfile, runtime behavior, native environment or provider qualification changed. The build environment limitations from setup still apply.

Final documentation result for this reconciliation: `git diff --check` passed; `python3 scripts/check-markdown-links.py` passed with **85 files, 434 local links, 0 failures**. The original independent-report body was checked unchanged before the lead-disposition section. The published commit/branch head is the handoff identity; no runtime test result is implied.
