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

## Implementation and qualification checkpoint — 08 October 22:30 UTC

Complete candidate implemented through Store, AppWorker, CLI and native Settings,
committed/pushed as0723a698879b14edc831730897d8f296348c89cb.
One fresh independent read-only review covered all tracked and untracked changes
against main441120ce7f6ee47de3e4fa0f784355e2fc0b3257 and found no actionable
production defect. Its Store README precision note was corrected and re-reviewed.
Complete reviewed dirty-candidate hash:
`1e9862c62bb948b7a37fa9cb500f6e7ca051d04d0325f69de2bec8c3ce9ef66e`.
Binary diff hash90521a7640e1db0ea8e06305cc90d2b5ec9a19a438d36492f12efe3d88ed59c0;
complete identity additionally covers the canonical four-untracked-file hash manifest.

Passed: full Store472 tests plus final12 focused witnesses (including one later
added primary editor refusal witness); Store strict Clippy; three new real worker
witnesses; two CLI cancellation/correlation witnesses; two real-process CLI tests;
four desktop state witnesses; full native desktop348 plus7 CLI tests; three native
Clippy feature lanes and native shipping build. Representative64 complete chat
turn snapshot74,928,128 bytes measured217ms including validation/sync/publication;
individual SQLite steps have no hard wall-time guarantee. New exhaustive test
variant names, a CLI reference comparison, required native traits and an existing
shutdown-event expectation needed routine corrections; no guard/test weakening.

Final broader gate is active, serial Cargo in target/budgets: fmt and workspace
all-target default Clippy passed; full default tests excluding reused Store are
running, then native workflow, combined desktop/CLI, strict combined Clippy,
shipping/helper,52fixtures and links. Required CI/integration and all interactive
acceptance are pending. No new model calls;14/16 shared calls used. The consolidated
morning task in the lead checkout owns expected backup controls and eventual
matching final runtime; no GUI acceptance is claimed.

Final gate exposed old successful-shutdown empty-event expectations in private
Action reads and three worker integration witnesses. They now require exactly one
separate nil BackupStatus with a usable file/no copy error and preserve every
original correlation/privacy/receipt/replay assertion; the failed-startup empty
lane remains unchanged. Private-read focused rerun passed. Independent read-only
delta reviews are clean at66e5c9af61d71be7cbc42807df6256d7da0b483b4bf00d00941be9996599c981
and447c86b5bfec1f294f0bd77fb825cb3c4eed74937d2a4499da7c09dd49bc247e.
Unchanged418 workflow unit successes/full recovery are reused with the corrected
one-test pass; remaining integration/doctest/native gates continue serially.

The same successful-shutdown witnesses also drain command endings through
exhaustive matches. Each now accepts exactly one successful nil checkpoint status
and still rejects duplicates, nonnil backup replies, failed/unknown events; every
original command/result/recovery assertion remains. Independent test-delta review
clean atadb479f911736d96a20d2f9ee0ca74d4f2ad9f6a7157a5aecb15e1c1e9e33f2b.
Final integration tests and all workspace doctests passed; native workflow and
remaining combined/Clippy/shipping/fixture gates continue in gate12095. No production
change since reviewed0723a69. Prior default418 unchanged unit successes/full-size
recovery and corrected private-read focused pass are reused rather than rerunning
unchanged full-size witnesses. Failed intermediate logs remain retained.

## Final local gates — 08 October 22:42 UTC

Final candidate sourcee6b8169 preserves reviewed production0723a69; only reviewed
shutdown test adaptations and evidence documentation followed. All local gates
passed. Default coverage1749 tests/17 existing ignores combines final integrations
and corrected private-read pass with unchanged successful default unit/recovery
and Store473 distinct coverage; intermediate failed assumptions remain logged.
Native workflow/models427 passed/15 existing ignores; combined native desktop/CLI
557 passed/zero ignores. All workspace doctests, strict default/combined all-target
Clippy, shipping CLI/desktop/helper,52 fixture assertions,605links and diff checks
passed. Full-size workflow recovery remained enabled (native131second gate).

Immutable qualification runtime:
`/Users/evokessler/repos/brn-overnight-artifacts-20261008/backup-runtime`, with
build-manifest.json exact source/files/qualification. It excludes pending PPTX
and Action compensation and must not replace the final combined morning runtime.
Required CI/integration and interactive acceptance remain pending. Next: incorporate
normally merged PPTX main, qualify affected merge seams, open backup PR, inspect
required CI, merge normally and verify resulting main. Cargo released; Action Store
helper now owns sole slot in the lead checkout. No new inference/live calls.
