# BRN Threads agent guide

This branch prepares the BRN Threads rebuild. Follow the user's current task. Setup, review, implementation, verification, acceptance, and integration are distinct activities.

## Start here

1. Inspect branch, HEAD, remote head, and working-tree changes. The shared rebuild branch is `rebuild/threads`; preserve unrelated work and use an isolated checkout/worktree.
2. Read [status](docs/status.md) and the [rebuild plan](docs/work/active/threads-rebuild/plan.md). For implementation or architecture review, read the [canonical Threads target](docs/architecture/threads-target.md), then task-relevant code.
3. Read [workflow](docs/development/workflow.md), [setup](docs/development/setup.md), and [verification](docs/development/verification.md) as needed.
4. Check [review status](docs/work/active/threads-rebuild/review.md) before committing to the core interfaces.

## Current authority

The owner authorized preparing this rebuild environment on 10 October 2026. This setup does not start product implementation. An explicit build assignment, including the supplied [build prompt](docs/work/active/threads-rebuild/build-prompt.md), authorizes implementation of its scope. Once assigned, continue through the agreed milestones without routine reauthorization.

The canonical Threads target owns the selected product and architecture contract. The other current overview files are navigation and implementation summaries. Update the target and affected summaries together when a justified correction is made; record substantive dispositions in the rebuild plan. Routine implementation choices are delegated. Ask the owner only when a real product choice or permission outside the assignment cannot be resolved from current instructions.

This branch supersedes the old architecture freeze, expired overnight permissions, V1 stage ordering, exact-byte live Markdown authority, blanket review-before-AI-write rule, mandatory original retention, and temporary-comment deletion. These changes implement the owner's current direction; they do not weaken repository protection or external access controls.

Only `docs/work/active/threads-rebuild/` is active for this rebuild. Other old active folders, audits, prior architecture decisions, crate contracts, and baseline tests are evidence about the old implementation. Their historical authorization is not a new task. Inspect them only when useful for reuse or a specific behavior.

## Essential target boundaries

- A fresh SQLite core/schema owns notes, revisions, threads, Actions, source references, changes, and authorization. Search indexes are rebuildable.
- One shared checked commit/Undo service serves desktop, built-in agent, CLI, and eventual external agents.
- Ordinary working notes support delegated AI maintenance. Whole-note protection and owner confirmation are separate; applicable direct owner instructions can authorize protected changes.
- Originals remain external. Routine intake uses useful information; explicitly requested full-note import preserves complete substantive content and necessary assets. Full imports start protected without another creation approval.
- Markdown export is on demand; full backup covers BRN state. DOCX/PPTX/HTML generation is later scope.
- No gradual old-data migration, dual writes, permanent legacy runtime, mandatory Jujutsu/CRDT, or universal converter. Reuse useful GPUI, providers, converters, and retrieval deliberately.

## Working rules

Use maintained components when they reduce total complexity. Keep semantic work in agent instructions/tools and integrity in deterministic code. Preserve useful dependency patches. Crate count and old command names are not requirements.

One lead owns shared interfaces and integration. Delegate bounded independent work when useful; separate writers by files/worktrees. Use the owner's selected model. Sol is the normal build choice; an Opus review is recommended where available, and Astra is for a named unresolved question. Model availability is not an extra gate.

Use synthetic/public fixtures and fresh explicit data directories. Do not open or migrate old BRN data. Follow the current assignment for live provider testing; do not inspect or print raw credentials, silently change provider/account, send external messages, release the app, or weaken protection.

Commit/push task-owned changes to the rebuild branch when assigned. Never force-push shared history. Merging the rebuild into `main` requires the applicable owner instruction and repository checks; this setup does not grant that merge.

Report implemented, verified, owner-accepted, and merged separately. Keep [status](docs/status.md) concise and record durable evidence in the rebuild plan or evidence file. Do not claim native usability from compilation.
