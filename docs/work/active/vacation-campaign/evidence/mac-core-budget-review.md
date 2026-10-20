# Independent MacCore CI time-budget review

Verdict: **no actionable findings** in the complete frozen candidate. The change gives the existing MacCore lane and its finalization a bounded additional resource allowance without removing checks or altering verdict handling.

## Exact identity and allowed scope

- Checkout: `/Users/evokessler/.codex/worktrees/export-durability/brn-rust`
- Branch: `codex/mac-core-time-budget`
- Candidate/observed HEAD: `fe65b8ec7f1be643108bb25fbbb5033a67683a76`
- Base and independently verified merge base: `be2de8af21f787b15afb821bbcd7d8fc26039ecf`
- Checkout clean, including untracked files, at inspection.
- Complete actual diff: only `.github/workflows/ci.yml`, two additions/one deletion: a comment and the Mac branch of the existing headless job timeout expression, 45 to 60.
- Lead-owned dirty task/evidence/overview state in the separate lead checkout is excluded from this frozen source review. The supplied timeout evidence there was read as supporting evidence, not as reviewed candidate content.

Applied the previously loaded reviewing-product-candidates skill/resource policy, repository verification/workflow and invariant boundaries. Inspected the entire workflow and local composite Rust setup action, not just the changed scalar. No delegation was used.

## Requirements and correctness

Location: `.github/workflows/ci.yml:54–63`, especially line 59.

Expression: `${{ matrix.runner == 'macos-15' && 60 || 30 }}`.

For the actual matrix values, the equality is true only for `macos-15`; the truthy numeric 60 is selected by `&&` and retained by `||`. Ubuntu and Windows evaluate the false branch and receive 30. Both numeric alternatives are positive nonzero integers, so this familiar conditional expression does not have the falsey-middle-value pitfall. The timeout remains a job-level property under `headless`, covering the job's running work and finalization rather than selectively relaxing a test step. It applies to that Mac matrix job on PR, main push and manual dispatch. No expression or matrix expansion was introduced.

The resource tradeoff is explicit: maximum MacCore running allowance increases by 15 minutes (one third), to the same 60-minute bound already used by both native jobs. A hung/slow MacCore job may therefore occupy a runner longer. Linux/Windows Core limits remain 30 minutes; documentation stays 15; native UI and native retrieval stay 60. This is a bounded time increase, not a timeout removal or a guarantee that every future run fits.

Independently parsed base and candidate with Ruby's standard YAML library. After restoring only the headless timeout scalar in memory, the complete parsed workflows compare equal. The full text diff contains only that scalar and its explanatory comment. Every job/matrix/context/name/step/condition/action pin/permission/concurrency setting and failure rule is preserved. In particular, all workspace/helper/API/restriction/scripted-route/capability/CLI/end-to-end checks remain, no `continue-on-error` or skip/retry was added, and scoped Rust-cache behavior is unchanged. Branch protection is neither changed nor bypassed by this diff; no live protection verification is claimed by the reviewer.

The comment accurately explains finalization allowance and the existing native-lane bound. Placement at the current job timeout is appropriately small; no new scheduler/cache mechanism or test is warranted for this scalar correction.

## Trigger and evidence

Supporting lead receipt: `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust/docs/work/active/vacation-campaign/evidence/ci-main20f-timeout.json`.

Original main source `20fcd7383662cd2f67607c3ce04194aed50809b5`, run `39401033547`, MacCore job `118555511333`, is recorded **cancelled**, not passed. The GitHub annotation explicitly says: `The job has exceeded the maximum execution time of 45m0s`.

Independently verified raw `/tmp/brn-main20f-mac-core-cancelled.log` SHA-256 `75138a5355fb52b1095bbfba6d8abac09a9ade61afa79bcf2ec98b87fcd9ff12` against the receipt. Job timestamps are 09:29:36Z to 10:14:44Z (45m08s). Functional steps ended successfully at 10:13:36Z (44m); the two skipped steps are expected platform branches (Linux formatting and Windows CLI help). Post setup-Rust cache work began at 10:13:36Z; the raw log shows the 993,892,067-byte cache upload completing at 10:14:36.621Z and the post action reporting success at 10:14:37.061Z, across the old 45-minute boundary. Checkout cleanup then completed. These facts support the diagnosed resource/finalization timeout even though individual step conclusions are successful.

The exact-source retry identified by the lead as job `118591050475` is separate from this source correction. Its status/result was not fetched or independently observed. A rerun is not source-fix evidence. Neither the original cancelled job nor this offline review establishes a passing required main gate or hosted qualification of the new 60-minute source.

## Checks and limits

Reviewer performed read-only Git identity/diff inspection, YAML structural comparison, source/log/receipt inspection and raw-log hashing. Standard YAML parse passed; it is not a GitHub-hosted workflow execution or expression-runtime validation. Lead reports diff check passed. No Python YAML dependency was installed, no packages acquired, and no new tests or Rust builds were run.

No Git mutation, Actions operation, account/network call, Page/PR/native action, lead record edit or delegation occurred. The only output write is this file. Original cancellation remains evidence; no log is described as passing. Integration and hosted head/resulting-main verification remain lead-owned, after the existing PR182 gate sequence. Review stop condition reached; no correction is requested.
