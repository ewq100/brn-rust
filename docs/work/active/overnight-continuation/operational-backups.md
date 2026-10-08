# Automatic checkpoints during operational work

Selected next independent P7 slice, baseline merged main
`441120ce7f6ee47de3e4fa0f784355e2fc0b3257` (PR97), branch
`codex/p7-operational-backups` in the existing budget checkout. Useful and ready:
startup-only copies omit a long open session's later chat, review and lifecycle
work. Product Vision34 and P7 require practical automatic internal-state recovery.
All state remains in the same checked SQLite database, including V18 sessions.
PPTX qualification continues independently in the lead checkout.

Reuse/adapt pinned rusqlite0.40.2 online Backup::step (Done/More/Busy/Locked),
existing checked WorkStore database, five-newest backup retention, owned AppWorker
command/idle loop and joined ChatStore shutdown. No schema, daemon, general
scheduler, second database, new dependency or destructive Restore operation.

## Fixed Store interface and guarantees

Add public strict serializable `BackupCheckpointOutcome` with tagged variants
`Unchanged` and `Created { path: PathBuf, completed_at_ms: u64,
retention_warning: Option<String> }`, and
`WorkStore::checkpoint_if_changed(&mut self) -> Result<BackupCheckpointOutcome>`.
Keep private baseline token `(conn.total_changes(), PRAGMA main.data_version)`
from the same live WorkStore connection: own writes and attached ChatStore commits
both matter. Initialize after the successful startup checkpoint. Reads and exact
replays with no writes create no new copies. No queries/copies in every read API.

Capture token before copying, retain that pre-copy token only after successful
publication. A concurrent commit during/after the copied snapshot must remain
eligible on the next attempt; conservative extra checkpoint is acceptable. Failure
keeps the old baseline and all prior usable backups. Avoid indefinitely retrying
SQLite busy/locked: bounded Backup::step work with a monotonic deadline. Do not
change global connection busy policy. Document measured latency separately from
a hard per-step wall guarantee.

Write into an exclusively created private temporary destination under the owned
backup directory, not a restore-candidate filename. Close SQLite, validate the
complete internal snapshot without reconciliation/migration/backup side effects,
sync file, publish exclusively/atomically and sync directory before pruning.
Never overwrite an occupied/foreign destination. Startup uses the same safe
publication helper. Temporary or failed copies never enter restore candidates.
Prune only after successful publication; pruning failure acknowledges the valid
new copy with a separate retention warning. Preserve the existing five-newest
policy; one-minute checkpoints shorten its time window, not its stored history.

## Workflow, CLI and native integration

Existing AppWorker checks on a two-second wake and between commands; at most one
automatic attempt per minute, with bounded failure backoff. Only changed state
copies. Continuous command traffic cannot starve the due check. No automatic
provider work. Final changed-state checkpoint follows joined ChatStore shutdown
and completion of admitted critical operations; no copy before final chat writes.

Lead owns a workflow checkpoint method and separate typed backup-status event/
read projection, CLI status/checkpoint commands and a small native status/control
in existing Settings. Status distinguishes last usable copy, failure and retention
warning. Preserve editor/composer/proposal/error buffers. Backup failure after
committed Save/approval/chat is a separate warning and must never turn the original
operation into a failed/retryable result. Shutdown remains successful for already
settled work; expose backup failure through status/warnings, not false mutation
failure. No fresh Restore command or automatic rollback.

## Acceptance and verification

1. Own and ChatStore-only commits checkpoint complete state; reads/exact replay
   unchanged means no copy. Includes V18 lifecycle/receipts and budgets.
2. Idle worker and continuous commands reach due attempts; shutdown captures final
   chat settlement. Deterministic injected time/due seams avoid minute sleeps.
3. Restore a synthetic copied checkpoint containing chat/partial outcomes,
   edited proposals/comments, Actions/Findings, retained intake bytes and lifecycle;
   normal existing running-work reconciliation and exact replay require no inference.
4. Concurrent commits yield consistent snapshots and remain dirty if not known
   captured. Busy/deadline/creation/publication/pruning failure retains prior valid
   backups; failed temporaries and occupied targets are never adopted/overwritten.
5. Existing newest-invalid/corrupt fallback, semantic-corruption refusal and full
   recovery witnesses remain enabled. Measure a representative large synthetic
   database; do not claim responsiveness from tiny fixtures alone.
6. Applicable Store/workflow/CLI/native tests, one complete independent read-only
   review, final relevant gates, actual required CI, normal protected merge and
   resulting-main verification. GUI observation pending in the single morning task.

Lead keeps selected model/effort and owns workflow/CLI/native/shared docs/integration.
A bounded helper owns Store backup/module/tests only, no recursive delegation;
at most two active helpers. Sole Cargo slot currently belongs to lead PPTX gate,
so helper may edit but cannot compile until explicitly granted. No private data,
credentials, GUI/computer use, live inference, model download, reset, port, release,
unrelated work or global configuration change. Reassess only a concrete integrity
blocker or consequential unresolved product requirement, continue independent work.
