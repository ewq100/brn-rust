# BRN Threads agent guide

The Threads rebuild is implemented and merged at `d768db7`. Phase 0 merged in PR #115 at `53e5289`; explicit first-use delegation merged in PR #116 at `36ad2d6`; the current assignment continues foundation hardening. Follow the user's current task. Setup, review, implementation, verification, acceptance, and integration are distinct activities.

## Start here

1. Inspect branch, HEAD, remote head, and working-tree changes. Preserve unrelated work and use an isolated task branch/worktree from the confirmed baseline.
2. Read [status](docs/status.md) and the current [Phase 1 recovery plan](docs/work/active/threads-rebuild/phase1-backup-folder-plan.md). The [rebuild plan](docs/work/active/threads-rebuild/plan.md) preserves implementation history. For implementation or architecture review, read the [canonical Threads target](docs/architecture/threads-target.md), then the [thread and runtime-guidance design](docs/architecture/threads-behavior.md) and task-relevant code.
3. Read [workflow](docs/development/workflow.md), [setup](docs/development/setup.md), and [verification](docs/development/verification.md) as needed.
4. Follow the completed [review dispositions](docs/work/active/threads-rebuild/review.md#build-lead-dispositions) and prove them during implementation; do not repeat the whole-plan review.

## Current authority

On 11 October 2026 the owner authorized Phase 0/F1, then explicitly deferred blocked native observation, requested PR publication and merge after CI, and asked to continue building. The [current plan](docs/work/active/threads-rebuild/phase1-backup-folder-plan.md) scopes the next Phase 1 increment: remembered backup folder. Stop-and-send merged in PR #117 at `ed9a1af` after required CI passed. Keep isolated worktrees, GPT-6.1 Sol implementation subagents and focused checks. Continue authorized work without routine reauthorization; broader later phases are not automatically assigned.

The canonical Threads target owns the selected product and architecture contract. The other current overview files are navigation and implementation summaries. Update the target and affected summaries together when a justified correction is made; record substantive dispositions in the rebuild plan. Routine implementation choices are delegated. Ask the owner only when a real product choice or permission outside the assignment cannot be resolved from current instructions.

This branch supersedes the old architecture freeze, expired overnight permissions, V1 stage ordering, exact-byte live Markdown authority, blanket review-before-AI-write rule, mandatory original retention, and temporary-comment deletion. These changes implement the owner's current direction; they do not weaken repository protection or external access controls.

The active task is `docs/work/active/threads-rebuild/phase1-backup-folder-plan.md`; the other rebuild files retain the selected contracts and prior evidence. Other old active folders, audits, prior architecture decisions, crate contracts, and baseline tests are evidence about the old implementation. Their historical authorization is not a new task. Inspect them only when useful for reuse or a specific behavior.

## Essential target boundaries

- A fresh SQLite core/schema owns notes, revisions, threads, Actions, source references, changes, and authorization. Search indexes are rebuildable.
- One shared checked commit/Undo service serves desktop, built-in agent, CLI, and eventual external agents.
- Ordinary working notes support delegated AI maintenance. Whole-note protection and owner confirmation are separate; applicable direct owner instructions can authorize protected changes.
- Runtime semantic guidance uses the packaged base guide and four SKILL.md playbooks; the application explicitly loads them, and they cannot grant permissions. Root AGENTS.md is development guidance, not the shipped runtime prompt.
- Originals remain external. Routine intake uses useful information; explicitly requested full-note import preserves complete substantive content and necessary assets. Full imports start protected without another creation approval. Mathematical equation fidelity is outside the first-release requirements.
- Markdown export is on demand; full backup covers BRN state. DOCX/PPTX/HTML generation is later scope.
- No gradual old-data migration, dual writes, permanent legacy runtime, mandatory Jujutsu/CRDT, or universal converter. Reuse useful GPUI, providers, converters, and retrieval deliberately.

## Working rules

Use maintained components when they reduce total complexity. Keep semantic work in agent instructions/tools and integrity in deterministic code. Preserve useful dependency patches. Crate count and old command names are not requirements.

One lead owns shared interfaces and integration. Delegate bounded independent work when useful; separate writers by files/worktrees. Use the owner's selected model. Sol is the normal build choice; the Opus plan review is complete, and Astra is reserved for a named unresolved question. Model availability is not an extra gate.

Use synthetic/public fixtures and fresh explicit data directories. Do not open or migrate old BRN data. Follow the current assignment for live provider testing; do not inspect or print raw credentials, silently change provider/account, send external messages, release the app, or weaken protection.

Commit task-owned changes on the isolated task branch; publish only within the current assignment. Never force-push shared history. Merging a new candidate into `main` requires the applicable owner instruction and repository checks; the owner now explicitly authorizes publishing and merging delegation after required CI passes, then continuing the next Phase 1 items without routine reauthorization. Apply the same checked, independently reviewed CI-gated integration workflow to these authorized increments. Release remains outside this assignment.

Report implemented, verified, owner-accepted, and merged separately. Keep [status](docs/status.md) concise and record durable evidence in the rebuild plan or evidence file. Do not claim native usability from compilation.
