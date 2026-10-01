# Current development status

Updated 2026-10-01. Implementation baseline: `main@6323e53`, including the
workspace shell (PR #12) and managed Markdown editing (PR #13). The Rig reset
documentation is integrated against that baseline without changing product
code. Verification below retains historical results from linked evidence;
those results are not fresh qualification of this documentation integration.

Safe Markdown editing implementation: **implemented and merged**; verification: **partial**; native/user
acceptance: **pending**. During its earlier qualification, the controller's integrated run on the staged merge
result passed (`EXIT=0`, 466 tests passed, 0 failed, provider-free fixture passed),
using the R21 worktree-local TMPDIR and grep-backed `rg` shim substitutions.
The native port run passed 89 unit tests + 5 CLI tests; the review-fix rerun,
including the queued-navigation regression, passed 90 unit + 5 CLI tests and
native Clippy. The controller's integrated result precedes these review fixes;
native-retrieval compilation is
unavailable because `protoc` is missing (no semantic/hybrid qualification).
See [qualification and manual acceptance](work/active/markdown-note-editing/evidence.md).

The workspace shell (PR #12) is merged on `main` at `1a30db0`. Shell automated checks were rerun for its final whole-branch
review fix wave; other verification results below remain historical.

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

PR #2 (including the review fixes for signals and pipe recoverability, durable-outcome reporting after SIGINT, help before argument validation, structured ask failure context, typed error codes, follow-up findings F1–F3 and test-only cleanup T1–T2) is merged to `main` at `1b49378`; the implementation record is closed under [completed work](work/completed/agent-cli-foundation/plan.md) and [PR #2 CLI review fixes](work/completed/pr2-cli-review-fixes/plan.md). [Active work](work/active/README.md) records merged Markdown note editing with native acceptance still pending and the unimplemented Rig reset.

### Agent-facing CLI (`brn`)

`crates/brn` provides the `brn` CLI: another interface over the shared application workflow, with the dependency direction `brn -> brn-workflow -> {store, retrieval, provider}`. It owns no SQL, retrieval, provider or separate business logic; the desktop drives the same workflow. Available surfaces include `status`, `import`, `documents`, `index build`, `search`, `ask`, `conversations`, `drafts create`, `drafts checkpoint`, `drafts save`, `drafts list/show`, `comments add`, `comments resolve`, `comments reopen`, `comments list`, and revision review. `--json` emits a versioned envelope for command results and errors; help and version output remain plain text even when `--json` is supplied. The command reference is [crates/brn/README.md](../crates/brn/README.md).

Current limitations to keep in view:

- Workspace ownership is exclusive: the CLI and the desktop (or any other process) cannot operate the same data directory at the same time.
- Native retrieval qualification is separate from the default/headless verification that covers the CLI; `status` reports `native_retrieval: false` in default builds.
- The CLI writing integration record is default-feature/headless evidence only; it does not qualify native UI/retrieval, live provider calls, model assets, GUI startup, real-vault access or native size-report behavior.
- PR #13 adds shared `notes` commands and the native editor beyond the original CLI foundation. Archive behavior, graph retrieval, publication and further UI redesign remain [roadmap](roadmap.md) work.
- Managed-note Save writes the live file; SQLite buffer recovery does not. Coordination covers participating writers only; late direct-writer races may be detected after installation. File durability uses `F_FULLFSYNC`, directory durability explicit `libc::fsync`; only process-kill recovery is tested, not power loss. Observed APFS is case-/normalization-insensitive; other volumes remain unqualified.
- Window close, application Quit/Cmd-Q and note switching guard recovery. Dock/system termination cannot be vetoed by pinned GPUI; the unsafe optional final-hook flush is omitted. Unacknowledged typing can be lost, while restart reconciliation covers durably recorded interrupted saves.

Implementation listed in the table and managed Markdown editing are present on `main@6323e53`. The table's historical `a82dd25` verification is distinct from later qualification records. Older feature-branch-only statements remain historical in their evidence records. Inclusion in main and agent-observed verification do not establish user acceptance or release readiness.

## Open qualification and scope

- Workspace shell manual native verification, VoiceOver qualification and user acceptance remain open; automated checks do not establish native usability.
- User subjective suitability, IME, accessibility and sustained near-limit editor performance remain unqualified.
- Search relevance on a representative approved corpus and production-scale indexing remain unqualified; synthetic results are not corpus-quality evidence.
- Natural authentication expiry/revocation, clean-machine installation, distributable sidecar/model handling and signing remain open.
- Comment-batch revision generation, candidate adoption, approval/publication, graph integration and backup/restore are future work.
- The original TypeScript BRN and vault remain separate; no migration or distributable release is claimed.

## Next work

The [Rig-first architecture reset](superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md) has a user-approved specification and [implementation plan pack](work/active/rig-first-reset/plan.md), approved at `db18da8` after the requested technical review. Execution choice remains unconfirmed; reset implementation/live qualification have not started. It targets direct ChatGPT/Codex and Copilot subscriptions, macOS distribution, FTS5/sqlite-vec and basic editor-grade Markdown saves with protected local credentials. Before execution, reconcile its earlier planning baseline with PR #13's existing managed-note APIs and schema V6; do not run its proposed new V6 migration unchanged. Dependency/provider/benchmark/release gates remain open in the [evidence record](work/active/rig-first-reset/evidence.md); live/resource/release permissions remain separate. Its saving scope is a future replacement for the implemented coordination/exchange protocol, not a change made by this documentation merge.

The next qualification action for [safe Markdown editing](superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md) is human observation on an unlocked target Mac using the disposable checklist in its [evidence](work/active/markdown-note-editing/evidence.md), plus resolving the optional native-retrieval build prerequisite. Tasks 1–8 are implemented and merged through PR #13; the record stays active pending native acceptance. No vault migration or release is claimed.

The UI redesign's first slice, the [workspace shell](superpowers/specs/2026-10-01-workspace-shell-design.md), is merged on `main` at `1a30db0` with [automated evidence and a pending human checklist](work/completed/workspace-shell/evidence.md). UI slice 2 (chat polish) is a candidate needing its own design approval; hidden features and prerequisites are tracked in the [UI feature backlog](ui/feature-backlog.md).

[13: revision from a comment batch](roadmap.md#13-revision-from-a-comment-batch) remains a future roadmap outcome. Neither a roadmap item nor a design approval authorizes implementation beyond its agreed scope.

The [historical status ledger](work/completed/early-checkpoints/status-history.md) preserves prior checkpoints and superseded next-step notes. Use [completed work](work/completed/README.md) to locate individual plans and evidence.
