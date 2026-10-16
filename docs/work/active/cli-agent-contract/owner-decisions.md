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
