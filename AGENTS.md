# BRN agent guide

## Start here

1. Confirm branch, HEAD, remote baseline and unrelated changes; use an isolated task checkout.
2. Read [status](docs/status.md) and the plan for the current user assignment. The [cleanup record](docs/work/active/pre-release-cleanup/plan.md) explains this repository simplification; it does not assign future work.
3. For implementation or architecture review, read the [Threads target](docs/architecture/threads-target.md), [behavior contract](docs/architecture/threads-behavior.md) and [invariants](docs/architecture/invariants.md), then relevant source.
4. For environment, checks or integration, use [setup](docs/development/setup.md), [verification](docs/development/verification.md) and [workflow](docs/development/workflow.md).

## Current authority

Current user instructions define the assignment. For the 11 October 2026 cleanup, the owner authorized isolated worktrees, GPT-6.1 Sol implementation helpers, independent review and publication/merge after required CI without routine reauthorization. Historical permission does not assign unrelated work. Native observation is explicitly deferred; release and real user-data access remain outside the cleanup.

The canonical target owns product requirements. Update affected summaries with justified contract corrections. Resolve routine implementation choices within the assignment; ask only for an unresolved consequential product choice or authority outside it. [Archived evidence](docs/archive/README.md) preserves provenance and historical instructions, which do not assign new work.

## Working boundaries

Use one shared checked service for desktop, agent and CLI writes; SQLite owns BRN state and search indexes are derived. Runtime guidance is packaged separately from this development guide and cannot grant authority. Preserve protection, exact-candidate review, version checks, replay and Undo guarantees described in the invariants.

One lead owns shared interfaces and integration. Give helpers bounded file/worktree ownership and evidence requirements. Preserve useful provider, converter, retrieval and dependency fixes; use the repository toolchain and lockfile as version authority.

Use fresh explicit data directories and synthetic/public fixtures. Preserve owner files and unrelated changes. Provider access follows current task authority; keep credentials/private payloads out of evidence. Never force-push shared history or bypass repository checks. Report implemented, verified, native-observed, owner-accepted and merged separately.
