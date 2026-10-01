# Current development status

Updated 2026-10-01. Code baseline: local `main` at `a82dd25954cbd809b290d88676aec6344fb326ac`, which includes the reviewed CLI writing batch and merged size-report task; merge state checked against `origin/main`. Verification results below are recorded historical results, not tests rerun for this documentation change.

## Implemented baseline

| Area | Implementation and recorded verification | Evidence |
| --- | --- | --- |
| Architecture | Baseline approved 2026-09-28 with native editor acceptance deferred at that time; GPUI remains provisional | [Decision](architecture/decisions/2026-09-28-architecture-baseline.md) |
| Storage | Authoritative SQLite, exact revisions, operation recovery; deterministic and process-crash checks recorded | [Storage](work/completed/storage-recovery/evidence.md) |
| Import, retrieval, grounded chat | Shared desktop/headless flow; keyword, semantic and hybrid search; synthetic native flow, restart, cancellation and owned-sidecar close observed | [End-to-end](work/completed/end-to-end-flow/evidence.md) |
| Desktop usability | File selection, small-window scrolling, progress/errors and unsigned local launcher exercised | [Usability](work/completed/desktop-usability/evidence.md) |
| Drafts and revisions | Working copies, immutable checkpoints/candidates, comparison and dirty-state protection; native and restart checks recorded | [Drafts](work/completed/draft-revisions/evidence.md) |
| Anchored comments | Exact original provenance, conservative mapping, resolve/reopen and native review; 116 workspace tests and native/restart observations recorded | [Comments](work/completed/anchored-comments/evidence.md) |
| Agent-facing CLI | `brn` binary exposes import, approval, index, search, ask, conversations, drafts, comments and revisions over the shared workflow with a versioned JSON envelope and subprocess tests | [CLI](work/completed/agent-cli-foundation/evidence.md) |
| CLI writing primitives | `brn` exposes draft creation, checkpoint/save and comment add/resolve/reopen commands over the shared workflow; the combined default-feature headless integration result is recorded separately | [Integration](work/completed/cli-writing-primitives/evidence.md) |

PR #2 (including the review fixes for signals and pipe recoverability, durable-outcome reporting after SIGINT, help before argument validation, structured ask failure context, typed error codes, follow-up findings F1–F3 and test-only cleanup T1–T2) is merged to `main` at `1b49378`; the implementation record is closed under [completed work](work/completed/agent-cli-foundation/plan.md) and [PR #2 CLI review fixes](work/completed/pr2-cli-review-fixes/plan.md). No product task is currently recorded in [active work](work/active/README.md).

### Agent-facing CLI (`brn`)

`crates/brn` provides the `brn` CLI: another interface over the shared application workflow, with the dependency direction `brn -> brn-workflow -> {store, retrieval, provider}`. It owns no SQL, retrieval, provider or separate business logic; the desktop drives the same workflow. Available surfaces include `status`, `import`, `documents`, `index build`, `search`, `ask`, `conversations`, `drafts create`, `drafts checkpoint`, `drafts save`, `drafts list/show`, `comments add`, `comments resolve`, `comments reopen`, `comments list`, and revision review. `--json` emits a versioned envelope for command results and errors; help and version output remain plain text even when `--json` is supplied. The command reference is [crates/brn/README.md](../crates/brn/README.md).

Current limitations to keep in view:

- Workspace ownership is exclusive: the CLI and the desktop (or any other process) cannot operate the same data directory at the same time.
- Native retrieval qualification is separate from the default/headless verification that covers the CLI; `status` reports `native_retrieval: false` in default builds.
- The CLI writing integration record is default-feature/headless evidence only; it does not qualify native UI/retrieval, live provider calls, model assets, GUI startup, real-vault access or native size-report behavior.
- The CLI foundation does not implement Markdown-first storage, archive behavior, graph retrieval, publication/approval, or the future UI redesign; those remain [roadmap](roadmap.md) work.

All implementation listed above is present in the inspected local `main` baseline, including anchored comments and the CLI writing batch. Older feature-branch-only statements remain historical in completed records. Inclusion in main and agent-observed verification do not establish user acceptance or release readiness.

## Open qualification and scope

- User subjective suitability, IME, accessibility and sustained near-limit editor performance remain unqualified.
- Search relevance on a representative approved corpus and production-scale indexing remain unqualified; synthetic results are not corpus-quality evidence.
- Natural authentication expiry/revocation, clean-machine installation, distributable sidecar/model handling and signing remain open.
- Comment-batch revision generation, candidate adoption, approval/publication, graph integration and backup/restore are future work.
- The original TypeScript BRN and vault remain separate; no migration or distributable release is claimed.

## Next work

The next proposed feature is [opening and safely editing real Markdown notes](superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md): explicit Save to a local vault, separate editing-buffer recovery, conservative external-conflict handling, and current-evidence invalidation. Its four design sections are approved; the written specification awaits review before implementation planning. No application code or vault migration has started, and no product implementation task is currently recorded in [active work](work/active/README.md).

[13: revision from a comment batch](roadmap.md#13-revision-from-a-comment-batch) remains a future roadmap outcome. Neither a roadmap item nor a design approval authorizes implementation beyond its agreed scope.

The [historical status ledger](work/completed/early-checkpoints/status-history.md) preserves prior checkpoints and superseded next-step notes. Use [completed work](work/completed/README.md) to locate individual plans and evidence.
