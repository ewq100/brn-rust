# Mac vacation hardening continuation

Updated 19 October 2026. Lead: the current local Codex chat. Baseline:
`bd42c1d99bfb41607c7218addbb52dc99bc340c3` (`origin/main`, merged PR162).
Working branch: `codex/vacation-hardening`, in an isolated managed checkout.
The original checkout's untracked files and all other worktrees are preserved.

## Current owner authorization

The owner selected continued development during a two-week vacation: introduce
coherent reducers/state machines, harden existing behavior, improve usability and
future changeability, remove unjustified test/code burden, and maintain documentation.
The lead chooses useful bounded slices without daily task assignment. This resumes
the existing campaign; it does not restart its clock. Hard expiry remains
**1 November 2026, 23:41:51 Europe/Tallinn / 21:41:51 UTC**.

The owner's 19 October corrections supersede conflicting execution restrictions in
the preserved cloud charter and earlier checkpoint:

- This Mac can remain powered on with Codex running; local implementation and a
  supported chat heartbeat are the selected continuation route. Cloud dispatch is
  not a prerequisite. No custom daemon or cloud/API worker is needed.
- **Merge is authorized**, including in-scope feature improvements, after actual
  independent review and required checks. Verify the resulting main and its checks.
  Merge approval is separate from subjective owner acceptance and release.
- **Mac is the priority.** Linux improvements and full Linux qualification move to
  the back of the queue. Preserve existing Linux behavior and gates; do not make
  cross-platform completion a prerequisite for useful Mac hardening.
- Work may use the existing allowance down to **about 5% remaining** in any applicable
  quota window. Check around work batches, not through token-by-token forecasting.
  Checkpoint before capacity is exhausted; wait and verify renewed capacity after
  the constraining reset. No credit purchase, reset-credit redemption or new billing.

Publication/release/deployment, new paid infrastructure or API workloads, private
vault/data use, credential changes and messaging others remain outside this work.
Preserve supported data, schemas, lockfiles and the checked service contracts.
Consequential changes to product meaning still need an owner decision; isolate those
and continue independent maintenance rather than blocking the whole programme.

## Outcomes and priority queue

Use [the workflow inventory](workflows.md), [test families](test-families.md),
[strategic programming](../../../development/strategic-programming.md) and actual
source as the existing execution system. Do not build a parallel planning framework.

| Order | Bounded outcome | Acceptance and retirement |
|---|---|---|
| 1 | Reconcile the merged campaign and establish local continuation | Exact PR/main checks, current authorization, a registered bounded heartbeat and durable next action; distinguish registration from an observed unattended run. |
| 2 | Make the worker's task/chat/run identity coherent | Run always has its exact chat and run; non-Run work cannot carry Run identity. Retire separate optional identities and repeated reconstruction where replaced. Preserve stream routing, Stop, Stop-and-send, queue/recovery and successor-worker admission. |
| 3 | Expand state ownership where Mac journeys expose real coordination risk | Choose the next domain from actual callers: scoped references/search, navigation during completion, run preview/context, or settings/recovery. Define states/events/effects first; preserve current user behavior and shared service authority. |
| 4 | Harden complete existing journeys | Fresh startup, write/save/reopen, dirty/stale review, Apply/Undo, import failure/cancel, queue/Stop/restart, backup/restore; prioritize reproduced failures and missing consequential witnesses. |
| 5 | Reduce maintenance and feedback cost | Consolidate only demonstrated duplicate/mechanism tests with a retained-guarantee map. Retire obsolete routes with their tests. Measure expensive lanes before optimizing; no test-count, coverage, file-size or deletion quota. |
| 6 | Maintain accurate guidance and handoff | Reconcile current status after each integration, retain historical failures, record a short owner-check list, and leave a runnable main plus explicit remaining qualification at the deadline. |
| Deferred | Further Linux capability/porting and release preparation | Resume only after more valuable Mac/state/reliability work, or a Linux regression affects the current slice's required gate. Existing Linux witnesses remain valid for their recorded source only. |

This is an evidence-driven order, not a promise to migrate all 25 provisional
domains in two weeks. Update the denominator when source inspection discovers more
routes. Native usability, IME, accessibility and owner acceptance remain distinct.

## First implementation slice: worker identity

Adapt the existing private `threads_worker::Worker`; use ordinary Rust enums and
the existing `WorkerKind` presentation rules where still useful. Reuse the shared
Workspace stop/queue/run services and existing real-control tests. No external
state-machine dependency, global reducer, event bus or new persistence owner.

Inspection found production Runs always construct both identities, while Import,
Export, Backup and Connect construct neither. The current struct nevertheless permits
invalid combinations. There is no reproduced identity bug yet: the intended benefit
is making that invariant structural and reducing repeated caller knowledge.
Do not remove a presentation discriminator used by legitimate headless consumers
merely to satisfy a deletion target. Inspect all production and fixture consumers.

Retain and run existing worker protocol, original-thread stream/navigation,
cancellation capability, stale reference disconnect and replacement-worker journeys.
Compile/test/lint the actual native feature path on pinned Rust 1.98.1, locked
dependencies and synthetic disposable data. Build both current helpers for suites
that require them. Missing setup is a named gate, not permission to weaken a test.
Use native interaction when a control's wiring or visible behavior changes; a
type-only preservation refactor does not newly establish native usability.

## Execution and resource policy

One lead owns shared files, merges and the campaign record. Retain the selected lead
configuration. A bounded source/log/document inspection may use one Luna worker;
implementation and consequential independent review use a suitable Sol configuration
under the invoked development skill. Start with at most one active helper; run a
reviewer after the candidate is ready. No recursive delegation or automatic Astra
escalation. Record requested model/effort and whether the runtime exposes the effective
configuration; do not claim measured savings without usage evidence.

The local heartbeat should wake this chat every two hours through the fixed expiry.
Each run resumes the existing slice, or selects one useful eligible next slice. It
must reconcile Git and other active work before writing; never create a competing
writer, duplicate a PR or dispatch after an uncertain launch. Read actual account
quota at entry and before a substantial new worker/batch. At approximately 5% or an
explicit capacity stop, leave a short checkpoint and use the supported schedule to
back off near the known constraining reset; reread capacity before resuming. Unknown
quota means defer new substantial work and retry later, not assume unlimited credit.
The scheduler itself is not a dollar-denominated spending cap or proof of wake during
exhausted allowance, sleep, app closure or host failure. Do not bypass a user pause.

Prefer coherent reviewable batches and focused checks. Save progress before a long
batch becomes difficult to hand off. After two failed approaches, reassess the
evidence instead of repeating builds or accumulating speculative fixes. Reuse valid
unchanged checks. No live provider activation or model acquisition is needed for
the first slice. At expiry, stop product dispatch and leave the final handoff.

## Gates and integration

Defined: existing behavior and invariants are the requirements; the first identity
slice above is Build-ready subject to local native build prerequisites. Candidate:
actual reviewed changes with applicable checks and residual risks recorded.
Integrated: actual authorized merge plus the relevant resulting-main evidence.

Required hosted contexts remain Core and CLI (Ubuntu and Mac), Mac Native UI, and
Mac Native Retrieval/Combined, as recorded in the verification guide; inspect actual
protection/context names before each merge. Documentation and relevant added Linux
checks also stay visible. Do not weaken or bypass checks. An overall red workflow
must retain its actual optional failures rather than being described simply as green.
Use normal merges and inspect relevant main checks before calling a batch complete.

[Evidence and next action](evidence.md) own the current candidate/check/scheduler
state. The original charter and roadmap remain preserved historical authority;
the dated corrections here own the changed local/merge/Mac/quota policy.
