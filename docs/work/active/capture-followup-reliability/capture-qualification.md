# Capture outcomes qualification — 2026-10-26

Capture proposes useful Notes, decisions and follow-ups from an explicitly confirmed posted conversation. The confirmation freezes its messages and private/project destination. Cancel persists no request or Run; changed context refuses confirmation. The existing candidate inspection, refresh, Changes review, Apply and Undo services own acceptance. Archive remains separate. Continue follows the matching durable Run instruction, never merely the newest Capture marker. These are deterministic service/control results, not live-model usefulness or owner acceptance.

Implementation source is `66dfba00b539f00e7ef1e1f8743de3c3535497e3` (publication cherry-pick `fe20b7b4`). The required semantic format gate is `bd52ddddee61c00ef50a5e8e99459717169120d9` (publication cherry-pick `03c0a464`). **The implementation must not be published without the schema13 gate.** Local commits, independent review and automated checks do not authorize merge or release. Current integration state remains in [the task record](record.json); this receipt does not replace it.

## Checks and source boundaries

Raw output and SHA-256 manifest are retained in [capture evidence](evidence/capture-20261026/manifest.json).

| Source | Check | Result |
| --- | --- | --- |
| Capture implementation | Runtime Capture boundary tests | 4 passed |
| Capture implementation | Native Capture filter, including three new confirmation/Continue tests | 6 passed |
| Capture implementation | Existing UI state tests | 20 passed |
| Capture implementation | Packaged guidance; prior bundle23 to bundle24 mock cache invalidation | 3 + 1 passed |
| Capture implementation | Strict all-targets Clippy, core/app/AI/native desktop | Passed |
| Schema13 follow-up | Full core, including two new format tests and existing upgrade/backup/restore coverage | 230 passed |
| Schema13 follow-up | Capture app and backup-directory app tests | 7 + 4 passed |
| Schema13 follow-up | Strict all-targets Clippy, core/app | Passed |

All inputs were disposable synthetic fixtures. Runtime fixtures call existing checked services; the cache witness uses a deterministic mock. No live provider/model calls, credential changes, Mac actions or real owner database upgrades occurred. An earlier full-core run passed 228 tests before the count-only preflight helper; it is not substituted for the final schema13 full-core result. The final combined native/binary observation and CI qualification are separate lead-owned receipts.

## Failures, corrections and review findings

- [Precise-edit red](evidence/capture-20261026/precise-red.log) reproduced `Applied` where `NeedsReview` was required in a later ordinary run. Binding the Capture snapshot at the shared checked preparation boundary covers exact edits as well as ordinary proposals. [Final runtime tests](evidence/capture-20261026/runtime-capture-committed.log) pass, including owner Apply/Undo and an ordinary-mode positive control.
- A suspected older unmarked candidate bypass was retracted after source inspection and a negative/ordinary-positive witness confirmed the existing core Runtime authority rejects non-Run application. No redundant permission guard was added.
- The first native command used `--lib` on the binary-only desktop package and [failed before tests](evidence/capture-20261026/native-capture.log). The corrected native command passed. An intermediate count-preflight compile rejected SQLite `usize` conversions; it was corrected to `i64` before final tests and strict lint.
- [Initial format test](evidence/capture-20261026/format13-targeted.log) caught the omitted schema12 backup-validator allowlist entry. Upgrade safely refused before mutation. All upgrade, backup-validation and restore allowlists now include historical12; [the corrected format tests](evidence/capture-20261026/format13-targeted-green.log) pass.
- The restore-fence fixture explicitly enables standing maintenance. After restore, the same authority applies an ordinary unmarked Note but cannot apply the Capture proposal. Exact canonical records, revisions, receipts and prepared-request rows survive schema12 backup/upgrade/restore. Current Capture backup/restore preserves review, then permits owner Apply and Undo.

## Historical writer refusal

Schema12 code accepts the generic Capture Message representation but does not enforce its review/privacy semantics. Guidance versioning cannot protect persisted state from that older writer. Schema13 is therefore an admission gate through the existing verified external backup-first upgrade and unpublished-copy restore engine; it does not rewrite canonical payloads or create a second compatibility engine.

The [historical executable receipt](evidence/capture-20261026/schema12-refusal.json) identifies source `9a8b1af51a08c27e807f2f7d9636471f81c8e37b`, production-equal to `bc699c4164147354b96e433b6327e194275102b0`, and binary SHA-256 `db83b9630b08361da7ecb68bb1a9df10d23bb465c5e5f5251216ab953705d295`. Its read and Apply commands each ran once and rejected the synthetic schema13 store with exit6 / `unsafe_directory` / schema-identity mismatch.

The first read harness incorrectly expected literal `13` in the rejection message. The product correctly refused with a generic schema-identity error; the read was **not retried**, and its pre-call database hash was **not retained**. Do not claim read-call byte immutability from that witness. The separate Apply probe retained matching before/after database SHA-256 and canonical-dump SHA-256, proving no mutation for that call. The receipt preserves both the harness failure and successful refusal evidence.

## Limits and remaining gates

Capture preflights at most 1024 active chat Message records, including host metadata, and accepts at most 256 posted owner/assistant messages with an exact 128KiB encoded snapshot limit. Paged reads and a raw-text byte check refuse overflow without truncation or a saved request/Run. The count-only query uses the existing message filter and a 1025-match limit; [its query plan](evidence/capture-20261026/capture-count-query-plan.log) is a table scan because there is no existing Message expression index. Accepted small captures still use existing global privacy/scoped-context reads. These are snapshot/traversal checks, not a hard process-memory or whole-store work bound.

Semantic selection, duplicate avoidance and usefulness are guided behavior, not demonstrated live-model quality. Actual Linux interaction, broader native focus/accessibility checks, Mac acceptance, owner acceptance and merge remain separately recorded gates. No model dispatch is authorized by an offline open/Cancel observation.

## First hosted head and shared-state correction

[CI40342822182](https://github.com/ewq100/brn-rust/actions/runs/40342822182) at `9b0e65311d91737c58af7dce3a9e67768a2657e7` completed with five supported jobs passing, Linux core failing its default-workspace lint, and optional Windows failing separately. Both default workspace test suites passed. Linux's headless desktop test configuration reported that `ContinuePath::RetryCapture` was never constructed: the real continuation decision and its test were incorrectly housed in the native-only module. Native-only lint had not exposed that feature boundary.

The correction moves that existing pure decision and unchanged four-assertion test into `threads_state`, used by both headless tests and native UI. The native adapter imports the same helper. Independent review found no behavior, authority or assertion change; no dummy use, warning suppression or test deletion was introduced. This is a feature-coverage placement correction. The earlier actual-window receipt remains bound to its recorded combined source; moving the pure helper does not create a new visual observation.

Correction qualification: strict all-targets desktop Clippy passed with default/headless features and with `native-test-support`. The existing continuation test passed headlessly (one selected test); the native `capture_` selection passed all three selected tests, including both dialog controls and the moved pure test. Formatting, Markdown links and generated overview checks passed. [Failure and correction logs](evidence/capture-headless-correction-20261026/manifest.json) preserve this boundary. Corrected exact-head hosted CI is pending publication.
