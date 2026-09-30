# Agent guide

This guide applies throughout this repository. Follow the user's current task and authorization; historical plans and evidence do not authorize new account actions, live calls, merges, releases or data migration. No particular model, plugin or agent framework is required.

## Start here

1. Inspect `git status --short`, the current branch and HEAD. Preserve unrelated changes.
2. Read [current status](docs/status.md) and the [documentation index](docs/README.md).
3. Read the relevant crate README, [architecture overview](docs/architecture/overview.md) and [invariants](docs/architecture/invariants.md).
4. For ongoing planned work, read its plan and evidence under [active work](docs/work/active/README.md). Use completed work only as supporting history.
5. Select checks from [verification](docs/development/verification.md) before making changes.

## Repository map

| Path | Responsibility |
| --- | --- |
| `crates/brn` | Agent-facing `brn` CLI over the shared workflow |
| `crates/brn-core` | UI-independent sample shell and worker lifecycle |
| `crates/brn-store` | SQLite authority, revisions, drafts, comments and recovery |
| `crates/brn-provider` | Owned Codex App Server process and protocol |
| `crates/brn-retrieval` | Derived keyword/native retrieval indexes and evidence |
| `crates/brn-workflow` | Shared application workflow, worker and headless driver |
| `crates/brn-desktop` | Native views and sample headless shell checks |
| `experiments` | Standalone trials with their own manifests and lockfiles |
| `scripts` | Verification and local macOS launcher helpers |

## Rules to preserve

- Preserve exact UTF-8 revision bytes, hashes and original comment/evidence provenance. Never guess an ambiguous anchor.
- Keep SQLite authoritative and indexes rebuildable. UI operations go through the workflow; keep provider and retrieval implementation details out of UI state contracts.
- Keep working copies, immutable checkpoints and AI candidates distinct. Do not silently overwrite later edits with asynchronous results.
- Search approval is not publication approval. Publication requires its own implemented and approved workflow.
- Use disposable explicit data directories and synthetic fixtures for checks. Preserve the original vault and existing trial workspaces.
- Do not log or commit credentials. Live provider checks and model acquisition require task authorization; deterministic checks are the default.
- Respect the pinned toolchain and lockfiles. Optional native features and standalone experiments need separate verification.

## Finishing work

Run relevant checks and report actual results and limitations; a successful build does not establish native usability. Keep current status concise, update affected contracts and links, and record a reproducible handoff using [the development workflow](docs/development/workflow.md). Distinguish implemented, verified, accepted and merged. Do not treat an old approval or historical agent assignment as a standing instruction.
