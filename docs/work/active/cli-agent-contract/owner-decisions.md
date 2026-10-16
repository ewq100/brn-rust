# Owner decisions for the CLI agent contract

Recorded 16 October 2026. The owner made these decisions in conversation with the lead and they are final. They authorize the work below inside this assignment. Release, publication and merge stay unassigned.

## Round 2: implement now

- **B2. No default credentials in CLI-created workspaces.**
  - A workspace that the CLI creates in an explicit `--data-dir` starts with no provider credential source. Only an explicit `configure` sets one.
  - The desktop/default workspace and existing workspaces keep their stored settings and current behavior.
  - In machine mode, `ask` and provider-backed intake on such a workspace are refused with a stable code before any provider contact or commit.
- **B4. No-op converge commands record nothing.**
  - Affected commands: `archive`, `resolve`, `unarchive`, `chat-move`, `project-link` and `project-unlink`.
  - When the requested state already holds, they return a distinct `Unchanged` outcome with no new receipt and no version bump.
  - The change lives in the checked service, so other hosts get it through the same path. It is a core-semantics change and it is approved.
  - A keyed retry still replays its original receipt.
- **B3, part 1. Stale edits stay conflicts.**
  - No automatic stale rebase: invariant 8 keeps stale typed work as a conflict.
  - `edit_session_open` no longer exposes the other session's ID.
  - The supported manual recovery path is documented: keep the buffer from the Stale result, `discard`, `begin-edit` at the current version, reapply, then `save`.

## Round 3: after round 2

- **B3, part 2. Caller-bound edit sessions.**
  - `begin-edit` returns a secret that later buffer, save and discard calls must present.
  - The design must cover restart and recovery, reclaiming a session after a lost token, and desktop sessions.
  - Persisted dirty guards and invariant 8 must keep holding.
- **B1. Workspace-scoped CLI authority.**
  1. **Owner workspaces.** Agents may change the owner's real workspace only as proposals that the owner approves. They use the existing exact-candidate review (invariant 9), not a new mechanism.
  2. **Test workspaces** are created only with an explicit `brn init --purpose test` and keep full CLI authority. A workspace created in an explicit `--data-dir` without that flag is not a test workspace.
  3. **Defaults and changes.** Existing and desktop workspaces are owner workspaces. Only the desktop, with the owner present, can change a workspace's purpose.
  4. **Phases:**
     - Phase 1 (round 3): record the workspace purpose; record a caller-supplied actor on receipts; report the effective CLI authority in the machine description and in the envelope.
     - Phase 2: in owner workspaces, CLI writes become candidates and owner-decision commands are refused. This needs desktop review UI and is a separate assignment.
     - Phase 3: the owner issues scoped, expiring grants from the desktop.
  5. **Limit.** This guards against agent overreach. It is not a boundary against malicious code running as the same OS user.
  6. **Effect on CLI users.** Once phase 2 lands, in-repo tests, scripts and witnesses that write through the CLI create their workspaces with `--purpose test`. The owner's own CLI writes to the real workspace also become proposals.

## Coordination exception

B4 may add a minimal, private no-op detection hook inside the `Store` chat and project command functions, even though `work/daily-workflow-reliability` also edits them. The lead accepts the resulting merge conflict. The implementation records the expected conflict and its resolution with the other merge-resolution notes and checks them with `git merge-tree`.

## After round 2b

Recorded 16 October 2026, in reply to the lead's two questions:

> "ok keepit , let do falloup pr"

- **`projects --trash` tightening: kept.** Values other than `yes`/`no` are now refused with `invalid_value` in both modes. Before this change they listed active projects with exit 0 (README Compatibility 8).
- **Round 3 is a follow-up PR.** This branch's PR carries rounds 1, 2 and 2b only. Caller-bound edit sessions (B3 part 2) and workspace-scoped CLI authority phase 1 (B1) go on a new branch from `main` after this PR merges, and become their own PR under the same rules. Before that design is approved, its shared-seam API is posted on the coordination board for the desktop lead.

## Round 3

Recorded 16 October 2026. The owner had reviewed the round-3 API proposal, which the lead derived from the approved round-3 design and posted on the coordination board, together with the lead's options for the schema upgrade. The owner replied:

> "approve lets go wiht b"

- **API approved.**
  - **Desktop:** none of its call sites change, and it keeps its in-process owner-host binding.
  - **CLI sessions:** CLI callers use token-bound edit sessions (`begin_caller_edit`, `update_caller_buffer`, `save_caller_edit`, `discard_caller_edit`). Only the token's hash is stored.
  - **Desktop-held sessions** appear to the CLI as `holder:"desktop"`, with no ID.
  - **Discard and adoption:** owner-host `discard` closes any session without a token, and owner typing adopts a caller session.
  - **Lost tokens:** `reclaim-edit` works only in test workspaces. Nothing expires.
  - **Schema 6** adds bindings, the workspace profile and receipt actors.
  - **Restore** removes bindings and forces purpose `owner`.
  - **B1 phase 1:** `brn init --purpose test` at creation only, an `--actor` recorded on receipts, and reported authority with `enforced:false`.
- **Schema upgrade: option B.** The CLI never upgrades an existing workspace as a side effect.
  - Any CLI command on a store older than the current schema, reads included, is refused with a stable `upgrade_required` code. The refusal names the explicit, backup-first `brn upgrade --backup-directory ABSOLUTE_DIR` command.
  - Only that explicit command upgrades a store from the CLI.
  - The desktop keeps its current upgrade behavior.
  - Fresh workspaces created by `init` start at the current schema.
- **Publication:** like the earlier rounds, the round-3 follow-up PR may be opened and merged to `main` after its required checks pass, with no force-push and no bypassed checks.
