# Current development status

Updated 2026-10-01. Current qualification baseline:
`feat/markdown-note-editing` at `8574ed0`, based on `main@9786d2d`, not merged.
The merged-baseline table below retains historical verification recorded at
local `main@a82dd25954cbd809b290d88676aec6344fb326ac`; those older results and
its origin merge-state observation were not rerun for this documentation change.

Safe Markdown editing implementation: **implemented**; verification: **partial**; native/user
acceptance: **pending**. The controller's integrated run passes 429 tests and
its provider-free fixture after a recorded CLI compatibility fix. Native UI
tests pass 54 cases and native Clippy passes; native-retrieval compilation is
unavailable because `protoc` is missing (no semantic/hybrid qualification).
See [qualification and manual acceptance](work/active/markdown-note-editing/evidence.md).

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

PR #2 (including the review fixes for signals and pipe recoverability, durable-outcome reporting after SIGINT, help before argument validation, structured ask failure context, typed error codes, follow-up findings F1–F3 and test-only cleanup T1–T2) is merged to `main` at `1b49378`; the implementation record is closed under [completed work](work/completed/agent-cli-foundation/plan.md) and [PR #2 CLI review fixes](work/completed/pr2-cli-review-fixes/plan.md). [Active work](work/active/README.md) records the implemented Markdown note-editing branch with native acceptance still pending.

### Agent-facing CLI (`brn`)

`crates/brn` provides the `brn` CLI: another interface over the shared application workflow, with the dependency direction `brn -> brn-workflow -> {store, retrieval, provider}`. It owns no SQL, retrieval, provider or separate business logic; the desktop drives the same workflow. Available surfaces include `status`, `import`, `documents`, `index build`, `search`, `ask`, `conversations`, `drafts create`, `drafts checkpoint`, `drafts save`, `drafts list/show`, `comments add`, `comments resolve`, `comments reopen`, `comments list`, and revision review. `--json` emits a versioned envelope for command results and errors; help and version output remain plain text even when `--json` is supplied. The command reference is [crates/brn/README.md](../crates/brn/README.md).

Current limitations to keep in view:

- Workspace ownership is exclusive: the CLI and the desktop (or any other process) cannot operate the same data directory at the same time.
- Native retrieval qualification is separate from the default/headless verification that covers the CLI; `status` reports `native_retrieval: false` in default builds.
- The CLI writing integration record is default-feature/headless evidence only; it does not qualify native UI/retrieval, live provider calls, model assets, GUI startup, real-vault access or native size-report behavior.
- The merged CLI foundation does not implement managed Markdown editing; the feature branch adds shared `notes` commands and the native editor. Archive behavior, graph retrieval, publication and the future UI redesign remain [roadmap](roadmap.md) work.
- Managed-note Save writes the live file; SQLite buffer recovery does not. Coordination covers participating writers only; late direct-writer races may be detected after installation. File durability uses `F_FULLFSYNC`, directory durability explicit `libc::fsync`; only process-kill recovery is tested, not power loss. Observed APFS is case-/normalization-insensitive; other volumes remain unqualified.
- Window close, application Quit/Cmd-Q and note switching guard recovery. Dock/system termination cannot be vetoed by pinned GPUI; the unsafe optional final-hook flush is omitted. Unacknowledged typing can be lost, while restart reconciliation covers durably recorded interrupted saves.

All implementation listed above is present in the inspected local `main` baseline, including anchored comments and the CLI writing batch. Older feature-branch-only statements remain historical in completed records. Inclusion in main and agent-observed verification do not establish user acceptance or release readiness.

## Open qualification and scope

- User subjective suitability, IME, accessibility and sustained near-limit editor performance remain unqualified.
- Search relevance on a representative approved corpus and production-scale indexing remain unqualified; synthetic results are not corpus-quality evidence.
- Natural authentication expiry/revocation, clean-machine installation, distributable sidecar/model handling and signing remain open.
- Comment-batch revision generation, candidate adoption, approval/publication, graph integration and backup/restore are future work.
- The original TypeScript BRN and vault remain separate; no migration or distributable release is claimed.

## Next work

The next qualification action for [safe Markdown editing](superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md) is human observation on an unlocked target Mac using the disposable checklist in its [evidence](work/active/markdown-note-editing/evidence.md), plus resolving the optional native-retrieval build prerequisite. Tasks 1–8 are implemented; the record stays active pending native acceptance. No vault migration, merge or release is claimed.

The UI redesign's first slice, the [workspace shell](superpowers/specs/2026-10-01-workspace-shell-design.md), has an approved specification and an [implementation plan](work/active/workspace-shell/plan.md); execution has not started.

[13: revision from a comment batch](roadmap.md#13-revision-from-a-comment-batch) remains a future roadmap outcome. Neither a roadmap item nor a design approval authorizes implementation beyond its agreed scope.

The [historical status ledger](work/completed/early-checkpoints/status-history.md) preserves prior checkpoints and superseded next-step notes. Use [completed work](work/completed/README.md) to locate individual plans and evidence.
