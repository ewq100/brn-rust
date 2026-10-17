# BRN agent guide

## Start here

1. Confirm branch, HEAD, remote baseline and unrelated changes; use an isolated task checkout.
2. Read the current snapshot in [status](docs/status.md) and the plan for the current user assignment. Check their branch/merge claims against Git before resuming work; dated evidence and proposed plans do not assign work.
3. For implementation or architecture review, read the [architecture overview](docs/architecture/overview.md), [invariants](docs/architecture/invariants.md), affected sections of the [Threads target](docs/architecture/threads-target.md) and [behavior contract](docs/architecture/threads-behavior.md), then relevant source and callers.
4. For environment, checks or integration, use [setup](docs/development/setup.md), [verification](docs/development/verification.md) and [workflow](docs/development/workflow.md).

## Current authority

Current user instructions define the assignment. Existing authorization continues within that assignment without routine reauthorization; it does not assign unrelated work. Publication, merge, release, provider access, real user data and native interaction follow the applicable assignment. Historical cleanup permissions remain in the [cleanup record](docs/work/active/pre-release-cleanup/plan.md), not a standing grant for new work.

The canonical target owns product requirements. Update affected summaries with justified contract corrections. Resolve routine implementation choices within the assignment; ask only for an unresolved consequential product choice or authority outside it. [Archived evidence](docs/archive/README.md) preserves provenance and historical instructions, which do not assign new work.

## Strategic programming

Optimize for correct owner outcomes and the ease of the next change. Follow the [strategic programming guide](docs/development/strategic-programming.md) for meaningful code and architecture changes.

- Identify the behavior's owner, callers, authoritative state and existing implementation before extending it. Check pinned libraries and platform facilities before adding custom machinery; judge the complete adapter and maintenance cost.
- Make bounded design improvements in the affected code when they remove a demonstrated source of complexity. Address repeated causes at their shared owner. A wider refactor belongs in an explicit follow-up; neither unrelated cleanup nor a refactor quota is required.
- Prefer modules that hide decisions behind a small useful interface. Keep workflow policy in shared services and coherent UI state under explicit owners. Use typed states and transitions where they remove invalid combinations or repeated coordination. File splitting alone is not modularity; speculative frameworks and pass-through layers need a concrete present benefit.
- Replacing a mechanism includes retiring its obsolete production route, wiring and implementation-only tests. Preserve supported saved-data, recovery and compatibility obligations. Explain any necessary temporary coexistence and its removal trigger.
- Protect behavior and important boundaries with proportionate tests and existing checks. Tests must distinguish correct from incorrect outcomes; counts, coverage, smaller files and fewer lines are diagnostic signals, not completion criteria. Native interaction remains separate from headless state tests.
- Independent review checks responsibility placement, caller complexity, reuse, retirement and behavior preservation. For material changes, report what became easier, any justified complexity/debt, and the evidence. Reconcile current status after integration; do not leave dated branch claims presented as current.

## Working boundaries

Use one shared checked service for desktop, agent and CLI writes; SQLite owns BRN state and search indexes are derived. Runtime guidance is packaged separately from this development guide and cannot grant authority. Preserve protection, exact-candidate review, version checks, replay and Undo guarantees described in the invariants.

One lead owns shared interfaces and integration. Give helpers bounded file/worktree ownership and evidence requirements. Preserve useful provider, converter, retrieval and dependency fixes; use the repository toolchain and lockfile as version authority.

Use fresh explicit data directories and synthetic/public fixtures. Preserve owner files and unrelated changes. Provider access follows current task authority; keep credentials/private payloads out of evidence. Never force-push shared history or bypass repository checks. Report implemented, verified, native-observed, owner-accepted and merged separately.
