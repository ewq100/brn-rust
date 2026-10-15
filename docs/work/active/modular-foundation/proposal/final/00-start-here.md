# Start here — BRN foundation work pack

Prepared 15 October 2026. This pack is a proposed, source-informed implementation assignment, not completed or tested code.

## What is in the pack

| File | Use |
|---|---|
| `01-master-plan.md` | Shared product/architecture contract, work breakdown, ownership, qualification and acceptance. |
| `02-foundation-lead-prompt.md` | Session C: C0 baseline, the shared writing changes (CW), integration and the program-status record. |
| `03-intake-build-prompt.md` | Session A: first intake experiment (X1, Xberg/Poppler) in `experiments/`; cutover is a later decision. |
| `04-writing-build-prompt.md` | Session B: the writing milestone on the current editor (B1a/B1b/B3/B4), plus the separate Zorite experiment (Z1). |
| `05-integrated-review-prompt.md` | Independent final review of the actual combined candidate. |
| `06-native-qualification.md` | Consolidated target-Mac owner/native test outline; lead fills exact setup and evidence. |

## Owner decision, 15 October 2026

`work/ui-polish` and `work/daily-workflow-reliability` are still being worked on. They will be finished and merged into `main` before this program starts. C0, the X1 and Z1 experiments, and all later slices start only after both have landed. The owner-paused path in master plan section 11 stays only as a fallback if this decision changes.

## Recommended launch sequence: three sessions, not three independent rewrites

**1. Start C in Bootstrap mode.** Provide `01-master-plan.md` and `02-foundation-lead-prompt.md` to a fresh session in the actual BRN repository. Use the message below.

C does the following:

- checks current main, unpushed worktrees and their owners, and confirms with you whether overlapping branches are still active;
- installs or reconciles the shared plan at `docs/work/active/modular-foundation/plan.md`;
- records ownership, the section 7.2 contract with its scenario IDs, and the baseline with pre-existing failures listed separately;
- commits an integration ref that adds no new failing required tests.

C0 does not implement CW.

**2. Start the experiments, then B.**

- **Experiments.** Under the owner decision above, X1 and Z1 start after both branches have landed. They may then begin before C0 completes, in their own worktree from an explicitly recorded existing snapshot. They use only an `experiments/` manifest and change no production or shared files. After C0 they reconcile to C's ref (master plan section 11).
- **B.** Start B from C's integration ref once `work/ui-polish` meets the release condition (landed, or owner-paused with its work preserved). B1a needs only C0. B1b and B4 wait for CW2.

Give each agent the shared plan and its role prompt.

**3. Continue C in Build mode.** Return to the same C session with: "C0 is established. Implement CW1, then CW2 once `work/daily-workflow-reliability` meets the release condition. Preserve B's files and integrate only explicit reviewed handoffs." C is the single integration owner, not a separate fourth feature build.

**4. Integrate in small slices.** C reconciles the recorded contracts/manifests/lockfile and reruns relevant cross-component checks. Do not wait for two large final branches before discovering incompatibilities.

**5. Review the combined candidate and run native qualification.** Give `05-integrated-review-prompt.md` to an independent reviewer. C prepares the exact setup for `06-native-qualification.md`. Publish/merge/release only within current explicit owner authority.

With only two concurrent build slots, count only streams that you confirm are building. The order is:

1. Finish and merge `ui-polish` and `daily-workflow-reliability` (owner decision).
2. C0.
3. CW1 alongside B1a.
4. CW2.
5. B1b, B3 and B4.

The experiments are light and can overlap. Never run two heavy native builds or benchmarks at once.

## Copy/paste bootstrap message

```text
Bootstrap mode, C0 only. Use the attached final master plan (01) and
foundation-lead prompt (02) for the BRN repository.

1. Inspect actual current state: branch/HEAD, fetched origin/main, every local
   worktree (including unpushed and uncommitted work), open PRs and running
   processes. The planning snapshot was b139221; do not reset to it. Read the
   current repository AGENTS.md and status. If an injected guide conflicts with
   them, report the conflict and follow current repository guidance and my
   explicit authority here.
2. Owner decision (15 October 2026): work/ui-polish and
   work/daily-workflow-reliability are finished and merged before this
   program starts. Confirm that both have landed on main. If either has not,
   stop and report; do not start C0 work around them. Preserve all other
   unmerged work.
3. Run relevant baseline checks with exact commands and features. Record
   pre-existing failures separately; do not hide, skip or weaken anything.
4. Install the single plan at docs/work/active/modular-foundation/plan.md, linked
   from status. Include the baseline, path-level ownership, the section 7.2
   contract, the section 13 scenario IDs and applicability, and the default-build
   CI rule (F22). Do not implement CW. Do not commit deliberately failing tests;
   failing-first tests stay local evidence inside each implementing slice.
5. Commit a C0 integration ref that adds no new failing or skipped required
   tests, and record it.

Finish with: the factual baseline and pre-existing failures; overlapping-work
dispositions; the integration ref; the ownership table; open questions; and
precise start messages for X1, Z1 and B (B1a).

Local work and local commits only, on synthetic data. No push, merge, release,
real-data use, model download, paid or live provider calls, or computer use.
```

## Operating defaults

One repository; separate worktrees and synthetic data per stream. Keep existing checked state, scopes, dirty guards, exact review, replay and Undo. No mandatory new model or general framework. X1 and Z1 are experiments; adopting either is a separate later decision. Their results (passed, failed, blocked, pending or deferred) do not gate the writing milestone. The writing milestone does not by itself complete the wider foundation objective; master plan section 17 records each area's status.

The pack does not order a NoteGen/Zed fork, production standalone Writer, sync system or database replacement. The later standalone Writer is a small consumer of proven shared components.

GitHub inspection is not a view into active local agents. Reconcile local ownership before starting another writer. Do not merge archive PR #112 as part of this program.
