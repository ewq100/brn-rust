# Current development status

Updated 2026-10-01. Main baseline: local `main` at `a82dd25954cbd809b290d88676aec6344fb326ac`, which includes the reviewed CLI writing batch and merged size-report task; merge state checked against `origin/main` at that checkpoint. Workspace shell code is implemented separately on `feat/workspace-shell` at `4c3ddd4`, branched from `9786d2d`, not merged. Shell automated checks were rerun for the final whole-branch review fix wave; other verification results below remain historical.

## Implemented baseline

| Area | Implementation and recorded verification | Evidence |
| --- | --- | --- |
| Architecture | Baseline approved 2026-09-28 with native editor acceptance deferred at that time; GPUI remains provisional | [Decision](architecture/decisions/2026-09-28-architecture-baseline.md) |
| Storage | Authoritative SQLite, exact revisions, operation recovery; deterministic and process-crash checks recorded | [Storage](work/completed/storage-recovery/evidence.md) |
| Import, retrieval, grounded chat | Shared desktop/headless flow; keyword, semantic and hybrid search; synthetic native flow, restart, cancellation and owned-sidecar close observed | [End-to-end](work/completed/end-to-end-flow/evidence.md) |
| Desktop usability | File selection, small-window scrolling, progress/errors and unsigned local launcher exercised | [Usability](work/completed/desktop-usability/evidence.md) |
| Workspace shell | Combined history/document-chat/vault shell, handoff theme, system appearance, persisted layout, reflow, dividers and shortcuts; default and native automated checks pass; manual native observations pending | [Shell](work/completed/workspace-shell/evidence.md) |
| Drafts and revisions | Working copies, immutable checkpoints/candidates, comparison and dirty-state protection; native and restart checks recorded | [Drafts](work/completed/draft-revisions/evidence.md) |
| Anchored comments | Exact original provenance, conservative mapping, resolve/reopen and native review; 116 workspace tests and native/restart observations recorded | [Comments](work/completed/anchored-comments/evidence.md) |
| Agent-facing CLI | `brn` binary exposes import, approval, index, search, ask, conversations, drafts, comments and revisions over the shared workflow with a versioned JSON envelope and subprocess tests | [CLI](work/completed/agent-cli-foundation/evidence.md) |
| CLI writing primitives | `brn` exposes draft creation, checkpoint/save and comment add/resolve/reopen commands over the shared workflow; the combined default-feature headless integration result is recorded separately | [Integration](work/completed/cli-writing-primitives/evidence.md) |

PR #2 (including the review fixes for signals and pipe recoverability, durable-outcome reporting after SIGINT, help before argument validation, structured ask failure context, typed error codes, follow-up findings F1–F3 and test-only cleanup T1–T2) is merged to `main` at `1b49378`; the implementation record is closed under [completed work](work/completed/agent-cli-foundation/plan.md) and [PR #2 CLI review fixes](work/completed/pr2-cli-review-fixes/plan.md). [Active work](work/active/README.md) now records Markdown note-editing planning; its implementation has not started.

### Agent-facing CLI (`brn`)

`crates/brn` provides the `brn` CLI: another interface over the shared application workflow, with the dependency direction `brn -> brn-workflow -> {store, retrieval, provider}`. It owns no SQL, retrieval, provider or separate business logic; the desktop drives the same workflow. Available surfaces include `status`, `import`, `documents`, `index build`, `search`, `ask`, `conversations`, `drafts create`, `drafts checkpoint`, `drafts save`, `drafts list/show`, `comments add`, `comments resolve`, `comments reopen`, `comments list`, and revision review. `--json` emits a versioned envelope for command results and errors; help and version output remain plain text even when `--json` is supplied. The command reference is [crates/brn/README.md](../crates/brn/README.md).

Current limitations to keep in view:

- Workspace ownership is exclusive: the CLI and the desktop (or any other process) cannot operate the same data directory at the same time.
- Native retrieval qualification is separate from the default/headless verification that covers the CLI; `status` reports `native_retrieval: false` in default builds.
- The CLI writing integration record is default-feature/headless evidence only; it does not qualify native UI/retrieval, live provider calls, model assets, GUI startup, real-vault access or native size-report behavior.
- The CLI foundation does not implement Markdown-first storage, archive behavior, graph retrieval, publication/approval, or the future UI redesign; those remain [roadmap](roadmap.md) work.

Except for the workspace shell on `feat/workspace-shell`, implementation listed above is present in the inspected local `main` baseline, including anchored comments and the CLI writing batch. Older feature-branch-only statements remain historical in completed records. Inclusion in main and agent-observed verification do not establish user acceptance or release readiness.

## Open qualification and scope

- Workspace shell manual native verification, VoiceOver qualification and user acceptance remain open; automated checks do not establish native usability.
- User subjective suitability, IME, accessibility and sustained near-limit editor performance remain unqualified.
- Search relevance on a representative approved corpus and production-scale indexing remain unqualified; synthetic results are not corpus-quality evidence.
- Natural authentication expiry/revocation, clean-machine installation, distributable sidecar/model handling and signing remain open.
- Comment-batch revision generation, candidate adoption, approval/publication, graph integration and backup/restore are future work.
- The original TypeScript BRN and vault remain separate; no migration or distributable release is claimed.

## Next work

The [Rig-first architecture reset](superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md) has approved design sections and a written specification awaiting review. It targets direct ChatGPT/Codex and Copilot subscriptions, macOS distribution, FTS5/sqlite-vec and basic editor-grade Markdown saves with protected local credential files. No product implementation or live qualification has started. Its saving scope supersedes the earlier coordination/exchange protocol for this reset; the existing note-editing implementation plan is paused and must be revised, not executed unchanged.

The earlier proposed feature is [opening and safely editing real Markdown notes](superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md): explicit Save to a local vault, separate editing-buffer recovery, conservative external-conflict handling, and current-evidence invalidation. Its written specification was approved; the [implementation plan](work/active/markdown-note-editing/plan.md) was prepared but is now paused for revision under the reset. No application code or vault migration has started; [active work](work/active/README.md) records planning separately from implementation.

The UI redesign's first slice, the [workspace shell](superpowers/specs/2026-10-01-workspace-shell-design.md), is implemented on its unmerged branch with [automated evidence and a pending human checklist](work/completed/workspace-shell/evidence.md). UI slice 2 (chat polish) is a candidate needing its own design approval; hidden features and prerequisites are tracked in the [UI feature backlog](ui/feature-backlog.md).

[13: revision from a comment batch](roadmap.md#13-revision-from-a-comment-batch) remains a future roadmap outcome. Neither a roadmap item nor a design approval authorizes implementation beyond its agreed scope.

The [historical status ledger](work/completed/early-checkpoints/status-history.md) preserves prior checkpoints and superseded next-step notes. Use [completed work](work/completed/README.md) to locate individual plans and evidence.
