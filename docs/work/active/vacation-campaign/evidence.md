# Local continuation evidence

## Current gate snapshot

PR163's already-qualified worker identity source `823a1544` is merged as
`b5f01b07301dcc94d3d4e273a472d487f569bbc7`. Final-head required contexts,
documentation and Ubuntu Native UI passed in run39201810419; Windows Core failed.
The lead verified exact source digest
`e346235c98c26e9ce1922bfbf92ba6f32d4bd42dcc2c674567cab992ea9e2f88`
against the retained independent review and confirmed the later PR commit was
docs/evidence only. The normal merge used the exact checked head `c86e6d50` after
marking the existing draft ready under the owner's new merge authorization.
Resulting-main checks are pending, not implicitly passed.

PR164 continuation/native-batch policy is being reconciled with that main and
needs updated review/required CI. Initial PR164 run39208310470 was deliberately
cancelled as superseded when PR163 advanced main; its documentation passed and
optional Windows failure remain historical. No failed check was hidden or weakened.
Both local heartbeats are registered ACTIVE, but their first unattended executions
and Mac OS-input qualification remain pending. Historical entries below describe
the setup sequence; this snapshot controls the next action.

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

## Published setup and batched native authorization

Initial reviewed setup commit `584679e402085785a2356369d590a030187b500d` is
published in [PR164](https://github.com/ewq100/brn-rust/pull/164). Hosted run
39208310470 started; documentation/tooling passed, required native/core lanes are
pending. Normal agent merge follows the required checks. Repository auto-merge is
not enabled (the native request was rejected); it was not enabled or bypassed.

The owner subsequently authorized enabled Mac computer use and requested batching.
A separate `brn-batched-mac-interaction-checks` heartbeat was created ACTIVE for
18:00 Europe/Tallinn, through the same fixed expiry. The main heartbeat was updated
with that division of work. Both persisted configurations target this chat and
were checked. No unattended wake or native interaction is claimed. The added
[native queue](native-queue.md) owns ready scenarios and prevents unnecessary reruns.
These additions directly record current owner instructions; the earlier independent
approval covers its pinned candidate, not unseen later material.

The first worker identity implementation is active in its separate owned checkout,
branch `codex/worker-task-identity`, with requested Sol medium and no shared-doc,
push/merge or recursive-delegation ownership. Task-local pinned/locked/offline
native test compilation is running against a cold target. Fixture migration and
qualification remain in progress; no candidate or passing tests are yet claimed.

## Overlap correction and unpublished duplicate

The lead checked current main and campaign records but initially failed to list all
open PRs. PR163 already contained the selected identity slice before this session.
The duplicate implementation was stopped as soon as discovered. It is not adopted,
pushed or claimed as a new fix. Its18-file local patch is preserved through managed
worktree archival; its stopped task-owned dependency cache was moved into the
campaign checkout for possible reuse. Both local compile attempts ended130 with
no executed tests; no native or code qualification is attributed to them. No local
build remains active. Earlier formatting/whitespace passes do not qualify that
unfinished patch. The existing PR163 source is deeper than the duplicate: private
operation plus explicit constructors, coherent identity reads and qualified fixtures.

Future selection must list open PRs, inspect the actual campaign branch tip and
compare existing candidates before dispatching a worker. The lead will reuse valid
source/review/CI evidence and preserve the existing independent findings/failures.

The upstream merge produced conflicts only in current status, campaign state and
checkpoint. Resolution retains the newer owner-authorized local/Mac/merge/quota
policy, carries all PR163 source/evidence files unchanged and links its exact
historical checkpoint. An intermediate pre-merge link check failed because the
upstream worker-identity document was not yet present; this is retained separately
from the complete merged-tree link result. Required CI will run on the reconciled
candidate, not be borrowed from either earlier head.
