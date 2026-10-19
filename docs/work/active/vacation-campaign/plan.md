# Mac vacation hardening continuation

Updated 19 October 2026. Lead: the current local Codex chat. Baseline:
`bd42c1d99bfb41607c7218addbb52dc99bc340c3` (initial fetched main, merged PR162).
Main subsequently advanced to `b5f01b07301dcc94d3d4e273a472d487f569bbc7`
when the lead merged the already-qualified PR163 under current authorization.
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
| 2 (merged PR163) | Reuse the coherent worker task/chat/run identity | Run always has its exact chat and run; non-Run work cannot carry Run identity. Retire separate optional identities and repeated reconstruction where replaced. Preserve stream routing, Stop, Stop-and-send, queue/recovery and successor-worker admission. |
| 3 | Expand state ownership where Mac journeys expose real coordination risk | Choose the next domain from actual callers: scoped references/search, navigation during completion, run preview/context, or settings/recovery. Define states/events/effects first; preserve current user behavior and shared service authority. |
| 4 | Harden complete existing journeys | Fresh startup, write/save/reopen, dirty/stale review, Apply/Undo, import failure/cancel, queue/Stop/restart, backup/restore; prioritize reproduced failures and missing consequential witnesses. |
| 5 | Reduce maintenance and feedback cost | Consolidate only demonstrated duplicate/mechanism tests with a retained-guarantee map. Retire obsolete routes with their tests. Measure expensive lanes before optimizing; no test-count, coverage, file-size or deletion quota. |
| 6 | Maintain accurate guidance and handoff | Reconcile current status after each integration, retain historical failures, record a short owner-check list, and leave a runnable main plus explicit remaining qualification at the deadline. |
| Deferred | Further Linux capability/porting and release preparation | Resume only after more valuable Mac/state/reliability work, or a Linux regression affects the current slice's required gate. Existing Linux witnesses remain valid for their recorded source only. |

This is an evidence-driven order, not a promise to migrate all 25 provisional
domains in two weeks. Update the denominator when source inspection discovers more
routes. Native usability, IME, accessibility and owner acceptance remain distinct.

## Upcoming phases — owner discussion19October

These are priorities, not fixed output promises; defects, review and actual allowance
can change the order. Do not force a refactor to fit a date.

1. **19–20October:** finish notice ownership and outstanding integration/main gates;
   establish the current Mac baseline and queue actual control qualification.
2. **Following days:** harden remaining worker/preview/context transitions where
   source and failure evidence identify a concrete benefit. Prefer one small owner
   over a general state-machine framework; preserve durable shared-service rules.
3. **Middle of campaign:** exercise Save/recovery, stale edits, backup/restore and
   failure paths using synthetic fixtures. Fix reproduced defects and actionable
   recovery wording, with consolidated18:00 Mac sessions when scenarios are ready.
4. **Final days through1November:** finish useful fixes, retire only obsolete
   mechanism tests with replacement proof, reconcile documentation and leave a
   reviewed/verified main plus an explicit remaining-gaps handoff.

Continue useful authorized batches inside the active session. Near5% remaining,
checkpoint and use the recovery schedule around the constraining reset; quota pauses
can move these phases. Daily records track actual outcomes and gaps, not a promise
of features or uninterrupted host availability. No release is included.

## First implementation slice: worker identity

**Already implemented and merged in PR163**, source `823a1544`, main `b5f01b07`.
This records the accepted slice, not a request to reimplement it. Its final source
digest matches the retained independent review; final PR required checks passed.
The next gate is resulting-main verification and continuation-policy integration,
then reconcile the remaining preview/context transitions before selecting new code.

Adapt the existing private `threads_worker::Worker`; use ordinary Rust enums and
the existing `WorkerKind` presentation rules where still useful. Reuse the shared
Workspace stop/queue/run services and existing real-control tests. No external
state-machine dependency, global reducer, event bus or new persistence owner.

Inspection found production Runs always construct both identities, while Import,
Export, Backup and Connect construct neither. The prior struct permitted
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
configuration. The owner clarified on19October that Luna may also implement small,
clearly specified changes, as well as inspect code/logs, maintain docs and triage
tests. Choose it when the bounded task and verifiable outcome suit it; reserve Sol
for consequential decisions, difficult changes and meaningful independent review.
Helpers may use nested Luna helpers when the lead explicitly assigns useful bounded
delegation. Count all helpers, including nested agents and reviewers, in the same
normal maximum of two; prefer one, with no allowance multiplied per parent. Start
with at most one active helper; run a
reviewer after the candidate is ready. No automatic Astra escalation or unassigned
recursive delegation. Record requested model/effort and whether the runtime exposes the effective
configuration; do not claim measured savings without usage evidence.

The owner clarified on19October: **continue actively through successive useful
batches while authorized work and credits remain**. Completing a batch does not
cause a two-hour wait or voluntary handoff. Keep implementation, review, checks and
integration moving, then select the next useful eligible slice. Renew context from
the durable checkpoint instead of treating one context/batch as the programme limit.
The two-hour heartbeat is a recovery/resumption fallback after actual pauses,
interruptions, capacity stops or resets, not the pace of work. When a constraining
reset is known, use supported scheduling near it and recheck capacity. End active
work only at the quota floor, fixed deadline, explicit owner pause, or when no useful
authorized independent work remains. The lead must reconcile Git and other active
work before writing; never create a competing
writer, duplicate a PR or dispatch after an uncertain launch. Read actual account
quota at entry and before a substantial new worker/batch. At approximately 5% or an
explicit capacity stop, leave a short checkpoint and use the supported schedule to
back off near the known constraining reset; reread capacity before resuming. Unknown
quota means defer new substantial work and retry later, not assume unlimited credit.
The scheduler itself is not a dollar-denominated spending cap or proof of wake during
exhausted allowance, sleep, app closure or host failure. Do not bypass a user pause.

The owner additionally enabled and authorized Mac computer use on 19 October,
requesting batched sessions. A separate heartbeat at **18:00 Europe/Tallinn** owns
one consolidated native session when scenarios are ready; its registration is not
an observed execution. Keep the main engineering loop primarily headless and use
[the native queue](native-queue.md) to avoid replaying unchanged journeys. Prefer a
30–45 minute block, defer for active builds/writers or unavailable desktop access,
and use immediate computer interaction only for a specific blocking diagnosis.
Both heartbeats share this chat/lead and the same quota floor and fixed expiry;
neither may launch a competing writer or bypass a pause.

Prefer coherent reviewable batches and focused checks. Save progress before a long
batch becomes difficult to hand off. After two failed approaches, reassess the
evidence instead of repeating builds or accumulating speculative fixes. Reuse valid
unchanged checks. No live provider activation or model acquisition is needed for
the first slice. At expiry, stop product dispatch and leave the final handoff.

## Gates and integration

Defined: existing behavior and invariants are the requirements; the first identity
slice above is merged with source/PR evidence; resulting-main checks remain separate. Candidate:
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

## Selected next slice: notice state ownership

Baseline source is merged PR163; open PRs were reconciled before selection and
none already implements this slice. The current UI stores notice text, Tone,
`notice_tone_for` and last-work tuple separately; shell and UI repeat restoration
and text-equality rules. Adapt the existing native UI boundary using a small private
Rust owner and the existing Tone type. Preserve the public product behavior and
the current `ThreadsUi` inform/confirm/warn/fail/report routes. No external framework,
new crate, persistent state, worker policy, label or new control.

Files: the notice owner beside `threads_state.rs`/`threads_worker.rs`, its native
module wiring in `threads_ui.rs`, rendering/controls in `threads_shell.rs`, and
actual fixture consumers. Lead owns shared documentation; implementation owns only
desktop source/test consumers. Retire independent notice/tone/guard/last-work fields,
the string-guard fallback and duplicate restore coordination in the same slice.

Acceptance:

1. Text and Tone are one payload; unsupported direct writes cannot bypass the owner.
2. Ordinary notices leave the latest completed work available. Work completion
   records and displays the same payload atomically. Dismiss/clear hides current
   text without deleting the latest work status.
3. Restore requires an empty current notice, no active worker and a remembered work
   status; it neither restarts work nor replaces a newer notice/error.
4. The owner records whether current presentation is the remembered work status;
   identical text in an ordinary later notice does not establish work identity.
   This follows the existing newer-status preservation contract; no production
   bug reproduction is claimed at selection.
5. Keep five real outcome-tone cases (Stale, refresh follow-up, Applied, NeedsReview,
   failure), recorded-run/terminal severity, failed banner restore, repeated actual
   Dismiss/Layout restore, active/newer-status refusal and new-run clear. Extend
   an appropriate retained test for equal-text ordinary-notice provenance.
6. Retire only the sixth direct-field-write fallback case in the old outcome-tone
   test once private ownership makes that write unsupported; migrate other fixtures
   to owner operations and retain their behavioral assertions. No blanket deletion.

Use focused owner transition/sequence tests for these actual rules, retained GPUI
control/service journeys, native-feature strict lint/format and the relevant final
desktop suite. Qualify missing helper/build prerequisites separately; no zero-test
run or source-only inspection satisfies a test gate. Independent plan and complete
candidate review, required CI/normal merge/main verification remain separate gates.

Plan gate: independent Sol high review approved the complete plan at SHA-256
`e8bc9208e5ab6f87170213d9664c2d470f2cfdc2bde40656f1b96c559d2e8e17`
against baseline `37c87985`, with no actionable findings. A fresh Sol medium worker
owns desktop source/tests in the managed `notice-state` worktree, using the shared
task-owned partial build cache with one build at a time. The lead owns docs and
integration; normal allowance is one active helper. Stop and reassess after two
failed approaches, a disk/capacity limit or a consequential scope ambiguity. No
native, provider or owner-acceptance result follows from this plan approval.

## Next bounded candidate: worker drain fairness

The Luna source inspection found that `Worker::drain` loops until Empty/Disconnected,
even after its first terminal. A bounded channel limits queued slots, not cumulative
reads while a concurrent producer refills it. No production stall, post-terminal
production path or latency measurement was reproduced; this is proactive hardening.

Adapt the existing private worker owner, without another abstraction or dependency:
limit each drain to128 events (the largest current production channel capacity) and
return Pending when that budget ends before terminal. Stop reading immediately after
the first terminal; UI must consume it and retire its receiver before successor work,
as it already does. Remaining preterminal events wait for the next80ms poll; they
must not be dropped or marked complete early. This is an event-count bound, not a
wall-time guarantee for arbitrary payloads. Keep blocking content/terminal sends,
lossy progress, cancellation, disconnect partial persistence and queue policy.

Acceptance: a deterministic prefilled channel larger than the budget proves the
first batch has at most128 events and Pending, subsequent batches preserve every
payload/FIFO order and eventually admit terminal once. Retain first-terminal payload,
no-producer-disconnect completion, disconnected partial, Stop/Steer/successor and
Converted-to-new-Run/trailing-event integration cases. No sleeps or racing live
producer is needed to claim this defined event-count invariant; do not claim an
old native freeze reproduction. Existing worker/GPUI tests, native-feature strict
lint/format, applicable final desktop checks and independent review remain required.

If independent plan review approves, a Luna medium worker can own this small,
specified source/test change. Lead owns docs/integration. One writer/build per
boundary; wait until the notice build releases the shared target before testing.
Normal maximum2 helpers includes this worker and any reviewer/nested helper; no
nested helper is useful for this tiny task. Reassess after two failed approaches or
an event-order/persistence scope ambiguity. No implementation is authorized by this
paragraph until its independent plan review is recorded.

Plan review approved clean `7bbd7d6e`, plan SHA-256
`32c4ad53d668f534fea4738f1e264889a690a98e59b41d397895191de4c4dca3`,
with no actionable findings. A Luna medium worker owns the implementation in the
managed worker-drain-budget worktree. It may edit statically while the notice
worker finishes production qualification; local tests wait for the lead to release
the single shared build slot. Implementation review/checks are not yet satisfied.
