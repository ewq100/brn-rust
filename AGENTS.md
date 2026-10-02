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

The repository is moving to the [simple Rig-based notes app](docs/superpowers/specs/2026-10-02-simple-rig-notes-design.md) ([roadmap](docs/work/active/simple-rig-notes/plan.md)). New code follows these rules; existing code keeps working until the cleanup step removes it.

- Notes are vault Markdown files; preserve their exact bytes. `index.sqlite` is disposable; `brn.sqlite` holds user work, is checked at start and backed up.
- The AI writes only proposals; only the user's Approve writes AI text to the vault. No automatic fallback between providers, models or accounts.
- Comments are temporary review notes, deleted when the note's review is approved. Never re-anchor a comment by guessing.
- UI and CLI go through `brn-workflow`; keep provider and retrieval details out of UI state.
- Use disposable explicit data directories and synthetic fixtures for checks. Preserve the original vault, old data folders and existing trial workspaces.
- Do not log or commit credentials. Live provider checks and model downloads require the user's request; deterministic offline checks are the default.
- Respect the pinned toolchain and lockfiles. Optional native features and standalone experiments need separate verification.

## Finishing work

Run relevant checks and report actual results and limitations; a successful build does not establish native usability. Keep current status concise, update affected contracts and links, and record a reproducible handoff using [the development workflow](docs/development/workflow.md). Distinguish implemented, verified, accepted and merged. Do not treat an old approval or historical agent assignment as a standing instruction.
