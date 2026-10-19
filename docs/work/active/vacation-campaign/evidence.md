# Local continuation evidence

## 19 October 2026: baseline and assignment

The original local `main` was `18f3891e29e57d4459c663967d26115f0e57787d`,
1,187 commits behind the freshly fetched remote. Its untracked `.DS_Store` files
and `.worktrees/` were preserved. An isolated managed worktree starts at
`bd42c1d99bfb41607c7218addbb52dc99bc340c3`; branch `codex/vacation-hardening`.
No pre-existing checkout or trial data was reset or removed.

Environment: macOS 27.0.1, Darwin arm64. Repository pin and installed toolchain:
Rust 1.98.1. This initial continuation batch changes campaign documentation and
execution state, not Rust production code, dependencies or tests.

The owner explicitly confirmed this Mac can remain on with Codex running, authorized
merges after checks, deprioritized Linux and selected roughly 5% remaining allowance
as the work admission floor. See [the current plan](plan.md), which supersedes the
old no-merge/cloud-only/quota-reserve instructions without extending campaign expiry.

## Fresh hosted evidence

Read using `gh pr view 162 --json state,mergedAt,mergeCommit,headRefOid,statusCheckRollup`
and `gh run view 39200470515 --json jobs` after fetching `origin`.

- [PR162](https://github.com/ewq100/brn-rust/pull/162) is merged, at
  2026-10-19 04:38:18 UTC, as `bd42c1d99bfb41607c7218addbb52dc99bc340c3`.
  Its exact final head is `aebfa12b0d5da5f5bd91163f82f9cbc07a8edb3a`, including
  worker code `c24da6af`. Earlier pilot-only CI is no longer the latest evidence.
- Final PR required Core/CLI Ubuntu+Mac, Mac Native UI and Mac Native Retrieval
  all **passed**; Ubuntu Native UI and documentation also **passed**. Optional
  Windows Core/CLI **failed**. These are the reported job conclusions; this batch
  does not claim a new test-body audit or reinterpret counts from earlier logs.
- [Resulting-main run39200470515](https://github.com/ewq100/brn-rust/actions/runs/39200470515)
  on exact `bd42c1d9` completed. All four required contexts and documentation
  **passed**, as did Ubuntu Native UI and Ubuntu Native Retrieval/Combined.
  Three optional Windows lanes **failed** (Core/CLI, Native UI, Native Retrieval).
  The overall workflow is **failure**, not an all-platform pass. Failure causes
  were not newly diagnosed in this documentation batch.
- A preview workflow also reports success for this main. It was pre-existing and
  read-only observed; this work did not dispatch it or authorize publication.

Existing source/native observations in [worker lifecycle](worker-lifecycle.md) and
[verification](verification.md) remain tied to their original sources. Passing
hosted checks and an observed merge do not establish owner/native acceptance.

## Bounded inspection and next code slice

One read-only Luna helper was requested with model `gpt-6-luna`, effort `medium`,
fresh bounded context and no edits/builds/delegation. Effective child model/effort
is not independently exposed by the agent-status tool. It inspected the worker
identity boundary and its consumers; the lead confirmed the struct, production
construction, stop-and-send and writing/status adapters. No bug was reproduced.

Selected next task: the [worker identity slice](plan.md#first-implementation-slice-worker-identity).
Existing protocol/GPUI integration tests are the primary witnesses; no new mirror
test suite is planned. The previous campaign's 25 domains remain provisional,
two lifecycle pilots plus partial worker protocol ownership; no full-coverage claim.

Quota read via the native account-usage tool at setup reported 68% used / 32%
remaining in the seven-day Codex window, with no credit balance or reset credits.
This is account-wide, temporary telemetry, not a per-campaign or currency budget.
Future dispatch must read it again; current authority permits work to roughly 5%
remaining. Do not carry the original campaign's unavailable-telemetry claim forward
as a statement about this local execution surface.

## Candidate and continuation gates

- Implementation: initial plan/status reconciliation active; worker identity pending.
- Documentation checks: `python3 scripts/check-markdown-links.py` passed (546 files,
  2,478 local links, zero failures); `git diff --check` passed. No Rust suite rerun
  for this documentation-only batch.
- Independent review: **approved, no actionable findings**, fresh Sol reviewer
  requested as `gpt-6.1-sol` / high. Complete staged candidate digest
  `83ee8ea14c1447bdce0411e97f7cd61c981189992e778cb902345680a3e5e40a`.
  Requested/effective configuration distinction is retained; review did not
  independently repeat hosted queries or claim native verification.
- Integration: approved documentation candidate on `codex/vacation-hardening`;
  commit/push/PR and required-check integration are the next gates.
- Scheduler: **registered ACTIVE** as `brn-mac-vacation-hardening`, targeting this
  chat every two hours through the original expiry. Native creation, successful
  view and persisted local TOML were checked. Exact next-fire time is not exposed;
  no unattended run or future host uptime is proven.
- Native/owner acceptance: remains open; no fresh GUI interaction in this batch.

Next action: finish/review this continuation record, publish the candidate, observe required CI and normally
merge under the new owner authorization. Then implement the worker identity slice
with actual Mac native-feature checks; keep Linux enhancements deferred.

## Native implementation preflight

`PATH=/opt/homebrew/opt/rustup/bin:$PATH cargo +1.98.1 metadata --manifest-path
Cargo.toml --features brn-desktop/native-test-support --locked --offline
--format-version 1` failed before compilation: the local registry has no matching
`rmcp` package. This is a missing dependency-cache prerequisite, not a production
failure or a Rust test result. Before the worker slice, fetch the pinned locked
Mac dependencies through normal Cargo, then build the real native feature path.
No lockfile change, model acquisition or live-provider call is needed. The initial
continuation documentation checks remain independently passing.

Fresh `gh api repos/ewq100/brn-rust/branches/main/protection` confirms exactly the
four required contexts named in plan.md, strict up-to-date checks and enforced
administrator protection. No GitHub-required human review count is configured;
independent candidate review remains the development workflow's separate gate.

The subsequent `cargo +1.98.1 fetch --locked --target aarch64-apple-darwin`
**passed** through normal dependency fetching. Repeating the identical locked/offline
native metadata command **passed**. This resolves dependency resolution only;
actual compilation, helpers and native behavioral checks remain for implementation.
No manifest or lockfile changed. Rust tests have not been run in this batch.

Independent review approved the complete five-file candidate without actionable
findings. Later changes to this evidence/checkpoint only record that approval and
the successful dependency preflight; they do not expand the reviewed policy.
