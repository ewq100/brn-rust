# Resumable V1 checkpoint

Lead-confirmed 2026-10-05: full frozen V1 goal **active**, without a token budget.

- **Integrated baseline:** original-review PR57 at `c07cc19e4cc9e4aeed84aa86a0a6d6551231458d`, then AI-authority guidance PR58 at `3984e7cba9eb038ab0571604aa09becfebf3b4bd`. Exact reviewed trees were retained by normal merges.
- **Current:** narrow AI maintainability follow-up on `codex/v1-ai-behavior-boundary`: static agent instructions/capability policy in `brn-ai`, typed captured Inbox/Rewrite input in `brn-workflow`. Original prompt bytes, enabled tools and deterministic authority are preserved. This is not a roadmap stage.
- **Verification:** clean independent read-only review confirmed byte-equivalent instructions/input assembly and unchanged capabilities. Final format/build/all-target Clippy, 1,326 workspace tests / 0 failures / 8 ignores, 52 fixtures and 303 local links passed. Atomic gate retained terminal exit0 and an unchanged dirty-snapshot identity. PR58's exact-head four protected Mac/shared checks and documentation passed; platform failures remain visible. Merged-main run37348746479 passed all applicable gates. Windows22/22/14 full compiler blocks/summaries match the baseline; Linux retains the same three assertions/backtrace frames, with an earlier interleaved Cargo terminal-error line absent. Full raw logs/differences are retained.
- **Preserved work:** independently reviewed removal qualification preview remains in its isolated `codex/v1-inbox-removal-qualification` checkout, now based on PR58; four Store/seven Workflow regressions and Clippy passed after three validated fixes. Final broad gate and integration remain pending.
- **Pending acceptance:** native/live/owner acceptance, real multilingual model assets and trusted-user packaging. No new live provider calls or model downloads are authorized.
- **Next:** finish this maintainability follow-up, integrate the removal preview, then exact owner attestation and recoverable original-copy removal/recovery under the [current plan](../work/active/text-email-inbox/original-copy-plan.md).
- **Environment:** Darwin arm64 Mac mini, pinned Rust1.98.1, locked/offline dependencies, canonical owned `TMPDIR=/private/tmp/brn-mini-synthetic-20261005`, empty native model setting, separate checkout/target directories and synthetic data only.

## Earlier tooling checkpoint

Lead-confirmed 2026-10-05, full V1 goal **active**, without a token budget.

- **Integrated:** tooling PR56 at `03bb83a93c2df88e2699c72fb83e3ec0d78d41c1`,
  exact reviewed tree `9377f6d8d06db28e2240ca5d0d09ceb929085f95`.
- **Behavior:** conflicts/supersession remain integrated. Portable preflight,
  atomic gate evidence, exact CI summaries and deterministic Markdown checks
  are available. Four Mac/shared PR checks now require strict freshness and
  administrator enforcement; existing platform failures remain visible.
- **Verification:** exact-head PR run37340063138 attempt1 passed four required
  Mac/shared checks plus Documentation/tooling. Windows22 complete compiler
  blocks/two summaries match qualified main. Clean independent re-review after
  four validated fixes; fresh merged16 tooling tests,3 offline provider-example
  tests,292local links and atomic retirement gate passed with unchanged identity.
- **Pending:** merged-main run37341534594; native/live/owner acceptance, model
  assets and trusted-user packaging. Raw discovery observations need a scoped
  Auth interface; built-in synthetic catalog evidence is not live qualification.
- **Next:** original review evidence on `codex/v1-stage7-original-copy` at this
  merge, then exact owner attestation and recoverable removal/recovery. See the
  [current plan](../work/active/text-email-inbox/original-copy-plan.md).
- **Environment:** Darwin arm64 Mac mini, Rust1.98.1, locked/offline, canonical
  owned `TMPDIR=/private/tmp/brn-mini-synthetic-20261005`, empty native model
  setting, isolated checkout/target; synthetic data only. No new live calls,
  model downloads, private-data operations or public release authorized.

## Earlier product checkpoint

Lead-confirmed 2026-10-05 checkpoint; update this short record at integration.
Historical observations remain in the linked plans and [status](../status.md).

- **Mission:** complete the frozen V1 through trusted-user packaging. The lead
  confirmed the full goal active without a token budget at 15:44 UTC. This helper
  does not replace, complete or alter that goal.
- **Integrated:** [PR55](https://github.com/ewq100/brn-rust/pull/55) merged at
  `c5aaa6c4c96007e452151adb167964bf9e2b048a` on 2026-10-05 15:55:37 UTC,
  retaining reviewed tree `1477a02aed55a8932b23d41ec89540b37948e87c` and
  parents `eb36b331eb35c9cb5bc5b072060d89c39523ac87` /
  `569e654d5c99899037db1cc7d83d30d00d30ae42`.
  [Conflict plan](../work/active/text-email-inbox/conflicts-plan.md)
  retains the acceptance scenarios and detailed evidence. Paired supersession
  was previously integrated through PR54 at
  `eb36b331eb35c9cb5bc5b072060d89c39523ac87`.
- **Behavior:** exact tentative opposing quotations and a shared Ask conflict
  read. Frozen six crates, shared AppWorker and exact proposal approval remain
  the boundaries.
- **Verification:** lead reports clean independent review; final local
  1,315 shared tests / 0 failures / 8 ignores; 285 native Desktop / 0 / 0;
  268 optional native Workflow / 0 / 7; 116 synthetic capabilities / 0 / 0;
  52 fixtures; two V14 shipping restarts; relevant builds, Clippy and format
  passed. Exact-head automatic PR CI run `37335653379`, attempt 1, passed all
  four applicable Mac/shared checks. Windows Core remained failed with 22
  complete compiler error blocks and two summaries matching the qualified
  baseline. Overall PR CI remains red. At exact merge `c5aaa6c`, the lead's
  nine conflict tests, 52 fixtures and two V14 shipping restarts passed.
  Automatic merged-main CI run `37336763915` completed with all four applicable
  Mac/shared checks passed. Overall main CI remains failed from four informational
  platform failures: Windows Core/UI/Native retained 22/22/14 complete compiler
  error blocks and matching summaries; Linux Native retained three matching
  assertion/backtrace blocks by test name. The lead confirmed normalized full
  diagnostics and failure-source/CI/setup blobs match the qualified PR54 baseline.
- **Pending:** native/live/owner acceptance, multilingual assets and trusted-user
  packaging. Full V1 remains incomplete. Next bounded product slice: qualified
  safe original-copy removal.
- **Environment:** Darwin arm64 Mac mini, Rust 1.98.1, locked offline dependencies,
  canonical owned `TMPDIR=/private/tmp/brn-mini-synthetic-20261005`, empty
  `BRN_NATIVE_MODEL_DIR`; isolated checkout/target. No live provider calls,
  model downloads or private/original-data operations are authorized here.

Run the [preflight](../../scripts/development-preflight.py) on another checkout;
use the [verification guide](verification.md) and [workflow](workflow.md).
Recheck the commit, working-tree identity and current goal before continuing.
