# Knowledge Check triage qualification

Observed 2026-10-27 in the isolated `codex/knowledge-triage-20261027` worktree,
based on reviewed rehearsal `f19538c`. Only this slice's new commit is intended
for transplant onto actual main; rehearsal ancestry is not publication authority.

## Delivered behavior

Home's existing Knowledge Check now filters its eleven finding kinds, pages the
bounded retained findings, reports retained versus total counts, and inspects a
selected finding. Current affected Notes open through the existing editor;
Sources, Links and unavailable references are shown as read-only context.
An eligible existing pending proposal opens the exact shared Changes review.
Selection does not apply, refresh, retire or generate a proposal.

The replacement **Draft follow-up in chat** action creates a selected, unsent
composer draft. It posts no owner Message and starts no Run. Sending later is an
explicit owner action under current delegation settings; this slice adds no new
semantic repair generation or execution authority. The old broad Home seed
control is retired, while its existing CLI/application API remains supported.

Snapshots bind exact observed records, known missing references and the complete
pending operation/hash/request. Action-time validation rejects changed findings,
retired/replaced proposals and context outside General. Whole-candidate inspection
includes current and historical inputs, expected target revisions and supported
typed references. Thread inputs, host markers and managed assets conservatively
require the proposal's original scoped Changes view; no new historical visibility
exception is introduced.

The existing composer recovery writer now has a narrow checked create-if-absent
entry point. Its SQLite writer transaction rechecks exact observations, absence,
active candidate identity and destination General privacy, including archived
chat-owned Notes. Replay preserves edited and emptied drafts. UI callbacks
revalidate after delivery and preserve newer visible/local recovery text. No
schema, runtime request protocol, scheduler or second maintenance engine was added.

## Qualification

Synthetic, deterministic checks only; no live model calls or token/cost claims.
The headless GPUI harness is not actual Mac UI or owner acceptance.

- `cargo test -p brn-threads-app --test topics_knowledge --locked --offline --jobs 1`:
  **19 passed**, including eight new triage witnesses. Coverage includes unsent
  persistence/reopen/replay, stale/private evidence, known-missing references,
  canonical chat privacy without a Thread-version change, complete candidate
  membership/retirement, private historical revisions and scoped Thread inputs.
- `cargo test -p brn-desktop --features native-test-support --locked --offline --jobs 1 knowledge_ -- --nocapture`:
  **4 passed**. Actual harness controls cover filter/paging counts and semantic
  Label role, selected draft loading into the existing composer, exact pending
  review without Apply/Run, retired delayed review, and delayed response preserving
  newly edited or emptied composer text. Replacement checks still retire old UI
  results. The final rerun also covers the ID-only Open response representation.
- `cargo clippy -p brn-desktop -p brn-threads-app -p brn-threads-core --all-targets --features brn-desktop/native-test-support --locked --offline --jobs 1 -- -D warnings`:
  passed. Formatting and `git diff --check` passed.

Logs remain under `/workspace/brn-next-tracks-evidence/knowledge/`:
`app-tests-2.log`, `native-tests-4.log`, and `clippy-2.log` are the final checks.
App execution took 0.10s; final native execution took 0.32s. Build
costs are separate: dependency/cache setup dominated (first native compile 7m07s,
subsequent desktop-only compiles approximately 1m35s). The first app attempt used
only the base environment; remaining native/lint runs consistently sourced both
`/workspace/brn-batch-evidence/environment.sh` and
`/workspace/brn-environment/native/activate-native.sh`. Tests used the explicit
synthetic `/tmp/brn-knowledge-triage-fixtures` directory.

## Retained failures and corrections

- `app-tests.log`: E0603 because the existing `prepared_on` reader was module
  private. The reader is now `pub(crate)` so the same transaction can compare the
  full existing Prepared value. No new public persistence API was created.
- `native-tests.log`: three passed, pagination label assertion failed. The
  metadata helper rendered text without an accessible parent label. The summary
  now exposes that exact visible text with a non-focusable `Label` role; exact
  range/count assertions remain, plus a semantic-role assertion. Intermediate
  label-only and final-role reruns are retained as `native-tests-2.log`/`-3.log`.
- `clippy.log`: large response enum and needless test borrows. Open now delivers
  only its record ID alongside the already captured selection guard; test borrows
  were removed. Strict lint passed, followed by the final native rerun.

## Remaining gates

Independent frozen-commit review, exact-main transplant/CI and reviewed draft
publication belong to the integration owner. No push, merge or release was
performed by this lane. Actual native usability/accessibility and Mac owner
acceptance remain separate; no provider, Mac, credential or security workaround
was attempted. Existing scoped proposals rejected by this conservative General
entry point remain reviewable through their original scoped workflow.
