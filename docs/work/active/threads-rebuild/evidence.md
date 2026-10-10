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

## Toolchain and remaining dependencies, 10 October 2026

This follow-up assessed the clean branch at `c2881e84958fde525fa03b361973baa30801007e` after the owner asked about Rust 1.99.0 and other updates. Three bounded read-only assessments covered the compiler, remaining direct dependencies and CI/dependency hygiene; root checked official releases, registry entries and the actual repository pins.

The plan now selects Rust 1.99.0 for compiler-only qualification before the separate Rig/GPUI changes. It identifies six verification scripts forcing Rust 1.98.1 and requires one consistent repository-selected compiler. CI setup and preflight already read the root pin. The remaining shortlist keeps current SQLite/BetterOffice/image versions, defers ZIP/HTML decisions to import qualification, avoids low-value patch churn and records a separate checkout-action maintenance candidate.

This is a documentation handoff update. Rust 1.98.1, Rig 0.43.0, Kit 0.6.6, the lockfile, scripts and CI remain unchanged. No compiler/native/provider qualification or complete advisory audit was run; the earlier machine limitations still apply. Source findings support the chosen next checks, not a claim that the upgrades pass.

Documentation validation: `git diff --check` passed; `python3 scripts/check-markdown-links.py` passed with **85 files, 437 local links, 0 failures**. Only six current Markdown handoff files changed in this follow-up; historical review and implementation evidence were preserved.


## Build started on the target Mac, 10 October 2026

Baseline: `a1c61e245ec90a0eed98ffc5bbfcab57ddeb1926`; isolated checkout `/Users/evokessler/repos/brn-threads`, tracking the fetched remote branch. Other worktrees/dirty files are preserved. Preflight found Rustup1.98.1, protoc and Apple clang. Cargo proxies are available under `/opt/homebrew/opt/rustup/bin`; initial PATH omission was corrected. Rust1.99.0 installed with rustfmt/Clippy; formatter check passed against the unchanged dependency lock SHA-256 `16fe749fc0e4592f2bf2065c660203ded43e5c1c7b03c803d52abceaa9a8ffb7`. Full compiler qualification remains in progress, separate from library upgrades.

The first workspace test attempt failed four CLI intake cases because the existing helper binary had not been built. The documented helper build then passed and the affected workspace qualification was restarted. This is a baseline prerequisite correction, not a Threads behavior result. Task-owned targets/evidence are outside Git under `/Users/evokessler/repos/BRN-local-builds/threads-20261010`. No old BRN data was opened.

Native inventory reports the Mac locked. No unlock attempted. One native interaction qualification task is retained in the [implementation record](implementation.md); compilation is not native usability. Selected provider/model settings reside in old app data; the exact route and credentials-directory path were requested from the owner without inspecting it. Offline work continues. Official Homebrew Poppler26.10.0 installed for the bounded PDF qualification path. No provider call, external message, merge or release occurred.

Retained verification scripts now invoke Cargo from the repository root to respect its pin. Experiment binaries also respect the task-owned target directory. Script syntax and whitespace checks passed; the compiler pin remains1.98.1 until qualification justifies adoption.
