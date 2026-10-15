# Start here — BRN foundation work pack

Prepared 15 October 2026. This pack is a proposed, source-informed implementation assignment, not completed or tested code.

## What is in the pack

| File | Use |
|---|---|
| `01-master-plan.md` | Shared product/architecture contract, work breakdown, ownership, qualification and acceptance. |
| `02-foundation-lead-prompt.md` | Session C: establish the common baseline, strengthen core/retrieval, own integration. |
| `03-intake-build-prompt.md` | Session A: intake boundary and Xberg/Kreuzberg qualification/cutover. |
| `04-writing-build-prompt.md` | Session B: writing extraction, references, and Zorite qualification/cutover. |
| `05-integrated-review-prompt.md` | Independent final review of the actual combined candidate. |
| `06-native-qualification.md` | Consolidated target-Mac owner/native test outline; lead fills exact setup and evidence. |

## Recommended launch sequence: three sessions, not three independent rewrites

**1. Start C in Bootstrap mode.** Provide `01-master-plan.md` and `02-foundation-lead-prompt.md` to a fresh session in the actual BRN repository. Instruct: “Use Bootstrap mode. Establish C0 and give me the explicit start instructions for A and B.”

C checks current main, unpushed worktrees and active owners. It installs/reconciles the shared plan at `docs/work/active/modular-foundation/plan.md`, records exact ownership/contracts and the common local baseline, and completes the small C0 slice. This step does not need to finish all core improvements.

**2. Start A and B in separate worktrees/sessions.** Give each the shared plan and its role prompt. Use the baseline/ref C supplies, not a guessed historic commit. A is the intake build; B is the writing/editor build. Their isolated research can start before C0, but production integration waits for C0.

**3. Continue C in Build mode.** Return to the same C session with: “C0 is established; A and B are running. Continue Build mode within your ownership. Preserve their files and integrate only explicit reviewed handoffs.” C performs core/retrieval work while A and B build. It is the single integration owner, not a separate fourth feature build.

**4. Integrate in small slices.** C reconciles the recorded contracts/manifests/lockfile and reruns relevant cross-component checks. Do not wait for two large final branches before discovering incompatibilities.

**5. Review the combined candidate and run native qualification.** Give `05-integrated-review-prompt.md` to an independent reviewer. C prepares the exact setup for `06-native-qualification.md`. Publish/merge/release only within current explicit owner authority.

With only two concurrent build slots: complete C0, run A and B together, then continue C's remaining hardening and integration. Read-only research and fixture work can overlap without shared-code changes.

## Copy/paste bootstrap message

```text
Use the attached BRN master plan and foundation-lead prompt in Bootstrap mode.
First inspect the actual current repository, local worktrees, active assignments,
and remote state. Do not assume the planning snapshot is still current.

Establish C0 only: baseline, minimum contracts, exact file ownership, shared
fixtures, and a common integration ref. Preserve existing work and current
product behavior. Do not start a whole-core rewrite.

Install/reconcile the plan under docs/work/active/modular-foundation/ and give
me the precise baseline and start messages for the intake and writing sessions.
Work locally on synthetic data. No publication, merge, release, real-data use,
model download or live provider spending without current explicit authority.
```

## Operating defaults

One repository; separate worktrees and synthetic data per stream. Keep existing checked state, scopes, dirty guards, exact review, replay and Undo. No mandatory new model or general framework. Adopt Xberg/Zorite only after qualification. A negative dependency decision can still produce a successful modularization.

The pack does not order a NoteGen/Zed fork, production standalone Writer, sync system or database replacement. The later standalone Writer is a small consumer of proven shared components.

GitHub inspection is not a view into active local agents. Reconcile local ownership before starting another writer. Do not merge archive PR #112 as part of this program.
