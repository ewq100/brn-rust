# Current development status

Updated 2026-09-30. Code baseline: local `main` at `1b49378`, which merges PR #2 (`feature/agent-cli-foundation`, agent-facing CLI) into `main`; merge state checked against `origin/main`. Verification results below are recorded historical results, not tests rerun for this documentation change.

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

PR #2 (including the review fixes for signals and pipe recoverability, durable-outcome reporting after SIGINT, help before argument validation, structured ask failure context, typed error codes, follow-up findings F1–F3 and test-only cleanup T1–T2) is merged to `main` at `1b49378`; the implementation record is closed under [completed work](work/completed/agent-cli-foundation/plan.md) and [PR #2 CLI review fixes](work/completed/pr2-cli-review-fixes/plan.md). No product task is currently recorded in [active work](work/active/README.md).

### Agent-facing CLI (`brn`)

`crates/brn` provides the `brn` CLI: another interface over the shared application workflow, with the dependency direction `brn -> brn-workflow -> {store, retrieval, provider}`. It owns no SQL, retrieval, provider or separate business logic; the desktop drives the same workflow. Available surfaces: `status`, `import`, `documents`, `index build`, `search`, `ask`, `conversations`, and read-only drafts/revisions/comments surfaces. `--json` emits a versioned envelope per call; the command reference is [crates/brn/README.md](../crates/brn/README.md).

Current limitations to keep in view:

- Workspace ownership is exclusive: the CLI and the desktop (or any other process) cannot operate the same data directory at the same time.
- Native retrieval qualification is separate from the default/headless verification that covers the CLI; `status` reports `native_retrieval: false` in default builds.
- The CLI foundation does not implement Markdown-first storage, archive behavior, graph retrieval, publication/approval, or the future UI redesign; those remain [roadmap](roadmap.md) work.

All implementation listed above is present in the inspected local `main` baseline, including anchored comments. Older feature-branch-only statements remain historical in completed records. Inclusion in main and agent-observed verification do not establish user acceptance or release readiness.

## Open qualification and scope

- User subjective suitability, IME, accessibility and sustained near-limit editor performance remain unqualified.
- Search relevance on a representative approved corpus and production-scale indexing remain unqualified; synthetic results are not corpus-quality evidence.
- Natural authentication expiry/revocation, clean-machine installation, distributable sidecar/model handling and signing remain open.
- Comment-batch revision generation, candidate adoption, approval/publication, graph integration and backup/restore are future work.
- The original TypeScript BRN and vault remain separate; no migration or distributable release is claimed.

## Next work

The next sequential roadmap outcome is [13: revision from a comment batch](roadmap.md#13-revision-from-a-comment-batch). Define its concrete scope and acceptance criteria before implementation. No product task is currently recorded in [active work](work/active/README.md); a roadmap item is not authorization to start it.

The [historical status ledger](work/completed/early-checkpoints/status-history.md) preserves prior checkpoints and superseded next-step notes. Use [completed work](work/completed/README.md) to locate individual plans and evidence.
