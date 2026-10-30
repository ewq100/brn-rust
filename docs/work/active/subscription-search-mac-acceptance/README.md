# Subscription search and Mac acceptance — issue 243

Authority and owner-facing progress: [issue #243](https://github.com/ewq100/brn-rust/issues/243).
This record is source-bound engineering evidence, not additional permission.

## Baseline and ownership

Started from main `89e6221afc8d6b036b7b5a1bcfe4b7cc23871583` on 30 October 2026.
The four required contexts passed in CI40990750144; seven supported jobs passed
and three optional Windows jobs failed. This was not an all-green workflow.
The working checkout is isolated; unrelated local changes and open PRs remain
untouched. PR239 stays paused. The retained cloud search commit `38e9baa86d7c47f25a4e0da8dd8977a5c0468982`
and Label commit `f6d39ff8` were unavailable in checked local/remote refs; no recovery is claimed.

One existing Sol lead owns integration and the live ledger. Terra6.1/medium
workers own bounded provider and desktop implementation; a fresh Sol/high reviewer
checks meaningful changes. Normally at most two helpers run, including review.
Shared builds are serialized. Early builds made while source is changing are
provisional; final affected checks must run on the frozen candidate. Stop dependent
live work on safety, identity, capacity, permission or deadline failure. No new
merge, release, paid route, credentials or real-data access is granted here.

## Completion gates

- [x] Current main, CI, ownership, open PRs, privacy and allowance inspected.
- [x] Subscription-only search and explicit-query desktop flow implemented (qualification pending).
- [x] Effective installed-version isolation verified with zero model turns before the first query.
- [ ] Login/provider, cancellation/deadline, malformed/unsafe/empty/cited paths tested.
- [ ] Native Label/focus candidate reviewed and affected tests passed.
- [ ] Real Mac search, keyboard/chat, Capture Cancel/Escape and dismissal observed.
- [ ] Synthetic document → knowledge → referenced draft journey, including a negative path.
- [ ] Complete final candidate independently reviewed; actionable findings resolved.
- [ ] Formatting, strict lint, applicable builds/tests and exact-head required CI passed.
- [ ] Scoped draft PR published and remote head verified.
- [ ] Source-bound evidence distinguishes passed, failed, blocked and unrun work.

## Observations and limits

- The initial Label change supplies AccessKit Label values matching the existing
  visible labels. Independent source review found no actionable defect; this is
  not a native interaction claim.
- A permanent search-rejection stub was an intermediate research checkpoint,
  not a functioning search implementation. Independent review found a feasible
  App Server construction on installed `0.162.0-alpha.17.2`: discover configured
  MCP/plugin/skill entries, explicitly disable them, re-read effective metadata,
  and create an ephemeral environment-free thread with no instruction sources.
  Empty/default overrides alone were insufficient. `unified_exec` opt-out being
  ignored does not itself prove command exposure; command controls are separate.
- Native control discovery reported the Mac locked and exposed no native apps.
  No further access attempt, security/power change or native acceptance is claimed.
- Existing normal subscription only. Search has one initial harmless public-query
  attempt, a 60-second application deadline and no automatic retry. A new local
  ledger reserves at most 12 top-level live turns across this issue and a single
  30-minute live interval, counting failed/uncertain launches. Hidden provider
  calls are not an invented monetary cap. Pilot003 remains closed and separate.
- No live turn has been launched at this checkpoint. Cost is unavailable.
- Zero-turn standalone-search metadata preflight accepted the exact version's
  `standalone_web_search` setting while preserving zero enabled skills, MCP
  tools/resources, explicit plugins and environment/instruction sources. Native
  structured results remain a live qualification gate; narrative URLs and page
  open requests are not invented source evidence.
- Independent source review found and corrected literal-key configuration
  overrides, benign lifecycle notification handling and a selected-account
  fallback. Desktop preparation refuses missing/unsupported saved account routes;
  settings changes invalidate pending results. Real stalled-process deadline,
  cancellation and process-group cleanup tests are added but execution is pending.
- Provisional page tests passed. Capture tests initially refused the macOS `/var`
  temporary-directory alias as a symlink; this original failure is retained locally.
  The corrected check uses a canonical task-owned `TMPDIR`, without weakening the
  data-folder guard. All final affected checks still require the frozen candidate.

## Headless search reproduction

Build the `brn-ai` example `subscription-search` with `--features capability-spike`
using the pinned toolchain/lockfile. Executing
`subscription-search --live-public-query EXACT_MODEL PUBLIC_QUERY` is a live turn:
reserve it first in the task ledger. It intentionally uses the normal managed
login, explicit medium effort, no workspace data, one request and no retry.
Neither building it nor the offline fixtures make a model request. A fresh
allowance/admission check is required before execution. Do not reuse historical
pilot or vacation drivers.

Raw process/account metadata, diagnostics, logs and the live reservation ledger
stay local. Only sanitized receipts and synthetic fixture reproduction belong in
this repository. Update this record at meaningful gates; historical campaign
instructions do not resume their old actors or allocations.
