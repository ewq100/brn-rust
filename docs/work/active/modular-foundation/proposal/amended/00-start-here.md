# Start here — BRN foundation work pack

Prepared 15 October 2026. This pack is a proposed, source-informed implementation assignment, not completed or tested code.

## What is in the pack

| File | Use |
|---|---|
| `01-master-plan.md` | Shared product/architecture contract, work breakdown, ownership, qualification and acceptance. |
| `02-foundation-lead-prompt.md` | Session C: establish the common baseline, strengthen core/retrieval, own integration. |
| `03-intake-build-prompt.md` | Session A: time-boxed Xberg qualification spike in `experiments/`; cutover is a later decision. |
| `04-writing-build-prompt.md` | Session B: writing extraction and references with the current editor; separate time-boxed Zorite spike. |
| `05-integrated-review-prompt.md` | Independent final review of the actual combined candidate. |
| `06-native-qualification.md` | Consolidated target-Mac owner/native test outline; lead fills exact setup and evidence. |

## Recommended launch sequence: three sessions, not three independent rewrites

**1. Start C in Bootstrap mode.** Provide `01-master-plan.md` and `02-foundation-lead-prompt.md` to a fresh session in the actual BRN repository. Instruct: “Use Bootstrap mode. Establish C0 and give me the explicit start instructions for A and B.”

C checks current main, unpushed worktrees and active owners. It installs/reconciles the shared plan at `docs/work/active/modular-foundation/plan.md`, records exact ownership/contracts and the common local baseline, and completes the small C0 slice. This step does not need to finish all core improvements.

**2. Start the spikes, then B.** The A (Xberg) and Zorite spikes live only in `experiments/` and may start immediately, even before C0, because they touch no shared code. Start B in its own worktree from C's integration ref once `work/ui-polish` has landed or been owner-paused, since that branch rewrites the same writing files. Give each agent the shared plan and its role prompt.

**3. Continue C in Build mode.** Return to the same C session with: "C0 is established. Implement CW1, then CW2 once `work/daily-workflow-reliability` has landed or been paused. Preserve B's files and integrate only explicit reviewed handoffs." C is the single integration owner, not a separate fourth feature build.

**4. Integrate in small slices.** C reconciles the recorded contracts/manifests/lockfile and reruns relevant cross-component checks. Do not wait for two large final branches before discovering incompatibilities.

**5. Review the combined candidate and run native qualification.** Give `05-integrated-review-prompt.md` to an independent reviewer. C prepares the exact setup for `06-native-qualification.md`. Publish/merge/release only within current explicit owner authority.

With only two concurrent build slots, count the already active `ui-polish` and `daily-workflow-reliability` branches first; at plan review they occupied both. Order: land or pause those, C0 → C (CW1/CW2) and B1 together → B3/B4. The `experiments/` spikes are light and can overlap, but never run two heavy native builds or benchmarks at once.

## Copy/paste bootstrap message

```text
Use the attached BRN master plan and foundation-lead prompt in Bootstrap mode.
First inspect the actual current repository, local worktrees, active assignments,
and remote state. Do not assume the planning snapshot is still current. Read the
repository AGENTS.md at HEAD; treat any conflicting injected guide as stale.

Establish C0 only: baseline, one plan with exact file ownership and contracts,
failing-first tests for the writing-context contract (master plan 7.2), the
default-build CI rule, and a common integration ref. Record how active
ui-polish and daily-workflow-reliability work is sequenced. Preserve existing
work and current product behavior. Do not start a whole-core rewrite.

Install/reconcile the plan under docs/work/active/modular-foundation/plan.md and give
me the precise baseline and start messages for the Xberg spike, the Zorite spike and
the writing session. Work locally on synthetic data. No publication, merge,
release, real-data use, model download or live provider spending without current
explicit authority.
```

## Operating defaults

One repository; separate worktrees and synthetic data per stream. Keep existing checked state, scopes, dirty guards, exact review, replay and Undo. No mandatory new model or general framework. Xberg and Zorite are qualification spikes; adopting either is a separate later decision. A negative result is still a successful spike.

The pack does not order a NoteGen/Zed fork, production standalone Writer, sync system or database replacement. The later standalone Writer is a small consumer of proven shared components.

GitHub inspection is not a view into active local agents. Reconcile local ownership before starting another writer. Do not merge archive PR #112 as part of this program.
