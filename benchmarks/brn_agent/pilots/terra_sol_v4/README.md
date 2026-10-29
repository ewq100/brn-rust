# Terra / Sol medium pilot package v4

Prepared for `brn-mac-relay-01`, existing ChatGPT subscription only. Publication
is not dispatch: root must post the authenticated release before the Mac acts.
No production merge/release, purchases, paid API fallback, account changes,
credential copying, private owner data, or restart of consumed jobs is authorized.

Candidate source is `24bec7e130e7fedd282445d316abe1498994c190`, including the
reviewed evaluator correction. The supporting package commit is separately pinned
by a future release. All older packages and consumed jobs remain unchanged.
Pilot001 consumed zero model calls. Pilot002 consumed one Terra invocation
(five HTTP attempts, one conservative12-call reservation), then failed in the
evaluator; all Sol cases were unrun. Neither attempt may be resumed or replayed.

[Mac pilot002 evidence](https://github.com/ewq100/brn-rust/issues/236#issuecomment-6492447591)
retains only the AttributeError class. The offline full writing pipeline reproduced
a Path-versus-CLI argument mismatch during operation lineage collection. The
candidate pin fixes that defect and retains sanitized exception locations. This
fits the Mac symptoms; exact identity remains unconfirmed without its traceback.

The v3 supervisor isolation correction is retained: deliberate corruption touches
a disposable registry copy only. The supervisor itself is byte-identical. The
complete `test_setup.py` control-flow fixture runs real inner supervisor tests and
three source-pinned evaluator regressions with synthetic build/CLI tools. The
three writing tests execute inside the outer setup supervisor so every direct
synthetic CLI child remains registered. No real provider is invoked. The original
lineage bug is also mutation-tested: setup must fail without readiness or dispatch.
A synthetic fixture pass does not claim a native Mac build or semantic quality.

Job003 requires specific owner approval and a fresh authenticated root release;
neither this package nor its publication grants execution authority. The same
60-minute budget includes all offline acceptance before live admission.

[Mac discovery](https://github.com/ewq100/brn-rust/issues/236#issuecomment-6487835954)
at 2026-10-29T04:20:33.066650Z verified exact `gpt-6.1-terra` and `gpt-6.1-sol`
with explicitly supported `medium`, using Codex CLI0.162.0-alpha.17.2 model/list.
Zero model calls; discovery server stopped. The sanitized observation is frozen
in capability-receipt.json. Offline002 ended; its unrelated test result is not
model-quality or capability evidence. Verify no owned process remains before
this independent job starts. Never reopen live001 or offline002.

## Allocation and deadline

Job `brn-terra-sol-pilot-003`, start expiry **2026-10-31T00:00:00Z**, one attempt.
Eight identical focused cases per model, one repetition: F01,F10,F19,F04,F13,F22,
F07,F16. Maximum16 candidate invocations plus six deterministic owner events.
Helpers/AI conversion off; medium effort; ask tool-round budget8. Intake has no
invented tool-round cap. Maximum12 actual model HTTP attempts per event and192
conservative experiment-wide reservations, including native resends and uncertain
calls. No scheduler retry/resume. No grader/helper/reader live allocation.

A single60-minute allowance starts before installed-toolchain checks, isolated
worktree creation and offline builds. prepare_mac.py exclusively persists and
fsyncs the fixed setup receipt, then supervises all setup. Native build products
are isolated under the new worktree's ignored target directory. Missing installed
Rust1.98.1, cached dependencies or working supervisor yields BLOCKED before model
dispatch. No installation or download. Existing verified Cargo/Rustup launcher
location is selected by the runner, with process-local PATH only.

launch.py accepts only the original setup deadline and remaining time; it refuses
less than60seconds. At59m30s from setup start, new dispatch stops and owned sessions
are terminated;30seconds are reserved for cleanup, with hard launcher exit at
60minutes. This budget never resets between phases. BRN commands, seeding and
prewarming additionally have300second limits. Relay's existing independent
owned-process deadline controls must also cover handoff gaps and setup receipt
I/O. No unbounded launch waiting or automatic restart.

The session supervisor handles native helpers creating separate process groups
and orphan groups after leader exit. It refuses concurrent direct children and
verifies no non-zombie owned processes remain before retiring sessions. Pinned
production source has no session-detaching helpers. Nested offline acceptance supervisors share a scoped, fsynced session registry
with serialized spawn/stop admission. Truncated registry data still triggers
cleanup of known sessions and records an error. Arbitrary unregistered daemon/session
escape is unsupported; a different process model blocks admission. Inventory or
cleanup failure is uncertain/BLOCKED, never success. Only this job's registered
sessions may be stopped; no runner-wide kill.

## Exact preparation and launch interfaces

Root's authenticated release pins package manifest and candidate source. The
runner must verify issue authority, unused job in issue history and established
durable ledger, expiry, no competing job, and package hashes. Keep all binding
paths, raw receipts/traces and credential selectors local. No credential content
is inspected by either Python entry point; normal BRN safety guards are unchanged.

After release, run the package's offline preparation with explicit absolute paths:

```text
python3 -B prepare_mac.py --repository EXISTING_REPO --source-dir FRESH_WORKTREE
  --claim-directory EXISTING_DURABLE_LEDGER --cargo-bin VERIFIED_INSTALLED_CARGO_BIN
  --auth-file EXISTING_SELECTED_CODEX_AUTH --output FRESH_EXPERIMENT
  --manifest-sha256 RELEASED_MANIFEST_HASH --authorization-reference RELEASE_COMMENT_URL
```

The source worktree and experiment must not exist. Existing repository refs must
already contain the pinned source from normal authorized Git fetch. Preparation
uses installed rustup `which` before executing proxies, locked/offline helper and
BRN/seeder builds, then `test_deadline.py` offline supervisor acceptance and three pinned evaluator
writing regressions on the Mac.
No model dispatch occurs in preparation. One failed setup consumes this attempt.
The supervisor ledger must end completed/exit0; readiness file alone is insufficient.

The release authorizes the runner to derive local binding and executable hashes
from this successful prescribed build, without another owner approval. Record
those values in local release JSON bound to root's authenticated comment. It is
not an independent authority grant. Required local release fields:

- plan_sha256, package_manifest_sha256, bindings_sha256,
  capability_receipt_sha256, setup_receipt_sha256, authorization_reference;
- runner `brn-mac-relay-01`, live_release true,
  offline002_terminal_and_processes_stopped true,
  existing_codex_subscription true, models_and_medium_verified true,
  offline_build_and_supervisor_passed true;
- candidate_runs16, max_model_calls192, start_before `2026-10-31T00:00:00+00:00`,
  claim_directory (the same established durable ledger);
- binary_sha256 from readiness: BRN_BINARY, BRN_SEEDER, brn-xberg-helper,
  brn-xberg-helper-brn. Bindings use the setup-generated file unchanged.

No release JSON is included. After those exact gates pass, the same job may invoke:

```text
python3 -B launch.py --release LOCAL_RELEASE --bindings FIXED_JOB_BINDINGS
  --setup-receipt FIXED_JOB_SETUP --capability-receipt capability-receipt.json
```

Fixed receipts use job-prefixed `.setup.json`, `.setup-supervisor.jsonl`,
`.bindings.json`, `.readiness.json`, and `.launch.jsonl` in the same ledger. Never
remove/relocate them to retry. A claim without terminal evidence is consumed and
uncertain. Post CLAIMED, RUNNING and terminal separately; posting is not execution.

Stop on setup/credential failure, observed wrong or missing runtime identity,
safety failure, model/runtime failure, capacity exhaustion, deadline or owner stop.
Keep every failed/incomplete attempt and reservation. Remote accepted work after
local cancellation remains uncertain; no refund/replay or success inference.
Launch receipts count local processes, not billable model calls; reconcile native
attempts and reservations separately. Native resends stay visible within limits.

## Results and verification

Use the full frozen suite: missing cases keep overall incomplete/unranked. One
repetition uses fixed arm order, so latency/cache comparisons are preliminary.
No winner or confidence claim. Quality needs blind human rubric adjudication or
separately qualified/frozen semantic grading; no live judge budget is included.
Report quality separately from elapsed time, actual rounds/model calls, observed
input/cached-input/output tokens and failures. Derive uncached only from valid
observations; unavailable usage/cost stays null. Subscription dollars are unknown;
legacy evaluator estimates are not actual charges. No invented monetary cap.

Public summaries contain only technical identities, counts, terminal states and
sanitized minimal failure details. Raw evidence remains local; no wholesale logs
or auth/account identifiers on GitHub. Offline verification is recorded in verification.json; Mac acceptance, including
the writing regressions, is mandatory before first dispatch. The retained
v1 short-timer development failure is documented in task history, not erased.
