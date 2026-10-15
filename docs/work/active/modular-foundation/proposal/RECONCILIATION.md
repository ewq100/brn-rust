# Final reconciliation: BRN reviewed work pack

**Pass:** one bounded document correction, 15 October 2026. No architecture re-review, implementation, install, commit or provider use.

**Unchanged inputs:** `original/` (byte-identical to the supplied pack), `amended/` and `amendments.patch` (the first review), and `REVIEW.md`.

**Outputs:**

| File | Contents |
|---|---|
| `final/` | Reconciled seven-file pack |
| `final.patch` | Original → final; 35 hunks |
| `delta-from-amended.patch` | First amended pack → final; 38 hunks |
| `RECONCILIATION.md` | This note |

## Verification performed

- Applied `final.patch` to a disposable copy of the originals: `diff -r` against `final/` shows the files are byte-identical.
- Applied `delta-from-amended.patch` to a disposable copy of `amended/`: byte-identical to `final/`.
- Originals: re-compared against the supplied attachments with `cmp`; unchanged.
- Acceptance IDs: F01–F22 are each defined once in master §13. Every ID used in any file exists, and the applicability registry assigns all 22.
- Slice IDs used consistently across all files: C0, X1, Z1, CW1, CW2, B1a, B1b, B3, B4, INT.
- Every "section N" reference resolves to an existing master-plan section.
- Stale terms are gone: `F01–F20`, deliberately failing C0 tests, "landed or paused" without a definition, "conversation selection", "not available in this conversation", "treat … as stale", "this program's completion".
- The repository working tree is unchanged (only the owner's pre-existing untracked `.vscode/` and `experiments/`).

## Short delta from the first amended pack

| Correction | Resolution | Where |
|---|---|---|
| 1. C0 usable baseline | C0 records the §7.2 contract, scenario IDs, baseline and pre-existing failures, then commits an integration ref with no new failing or skipped required tests. Failing-first tests are local evidence and land with their implementing slice. C0 does not implement CW. | 01 §6 tasks/exit; 02 Bootstrap 3/5/8; 00 step 1 and launch message |
| 2. Writing context | Writing context = (Note, scope) under the existing `visible()` scope model. The setup (brief, references, purposes) is scoped and never shared automatically across contexts. The simplest representation is preferred; no record layout is pre-approved. The active context is resolved by the shared service and shown in the tile; there is no first-linked-thread fallback. F21 extended to setup, brief and metadata, UI previews, request capture, and private material applied to a shared note. | 01 §5, §7.2, §9 B3; 04 B3; 05 items 1 and 4; 06 step 3 |
| 3. Apply safety | Owner-review Apply visibility is a **mandatory CW2 test-and-fix gate**, explicitly marked unverified. After access loss, no earlier approval or automatic retry applies a candidate. The binding table separates working-note/comment edits, setup edits, ordinary reference edits (Historical, "changed since"), required-current material (Current, Refresh) and access loss. Refresh is defined. v1 request, saved-candidate, replay and restart compatibility is preserved. | 01 §3, §7.2, §10 CW2, F10; 02 Build; 04 B3; 05 item 3; 06 step 10 |
| 4. Dependencies and IDs | B1 split into B1a (UI-only extraction; desktop keeps its `start_run`; nothing copied) and B1b (shared-run wiring after CW2). B4 depends on B1a + CW2. F01–F20 → F01–F22. One applicability registry (owner, required-for, evidence type) is referenced by every prompt and checklist; native steps are tagged with F-IDs. | 01 §9, §12, §13; 02; 04; 05 item 8; 06 |
| 5. Milestone vs program | §17 now defines the **writing milestone** on current components. X1/Z1 are recorded as passed, failed, blocked, pending or deferred and do not gate it. A per-area status table (writing, checked core, retrieval/context, provider/runtime, intake, editor) records starting dispositions and reopen triggers; it is not a new audit. X1 is the *first* intake experiment, not the intake objective. Experiments may start before C0 from a recorded snapshot and reconcile afterwards. | 01 §1, §8, §17; 00; 03 Objective/Preflight; 04 B2 |
| 6. Honest reading | Three separate statuses: import coverage, retained text read (ledger) and requirements reported as checked (a claim, with verified quotes). No claim of full original-file coverage or that all requirements were verified. Retrieval checks are selected by affected behavior (scope, filtering, run context, tools or search wiring need scope/tool tests even if ranking is untouched). | 01 §7.2, §7.3, §10, F08; 02 Context; 04 B3; 05 item 5; 06 step 7 |
| 7. Operational wording | Overlapping branches were refreshed at 16:23: both unpushed, last change 08:40, no matching build process. Branch existence is not treated as proof of an active agent or occupied slot. The "release condition" defines landed vs owner-paused: work preserved, not on the baseline, and a pause is not a merge. Pinned crate acquisition is distinguished from automatic native or model downloads, and an offline cache miss is an environment blocker. Zorite stock compatibility is kept separate from bounded adaptation. Stale-guide handling now reads current guidance and explicit authority, reports conflicts, and does not override genuinely higher-priority instructions. | 01 §3, §8 A2, §9 B2, §11; 00; 02; 03; 04 |

## Remaining executable or native questions (not resolved by documents)

1. ~~Overlapping branches~~: resolved by owner decision (below); C0 confirms that both have landed.
2. Owner-review Apply visibility for writing candidates (CW2 gate; code reading only so far).
3. The simplest scoped-setup representation, and whether it needs a schema increment (CW1).
4. Whether comment visibility on shared notes matches the intended private-context rule (recorded open question; not a CW change).
5. Z1: does unmodified `zorite-editor 0.11.2` compile against `gpui-pre 0.3.9`?
6. X1: can `xberg-native-pdf` rasterize pages without pdfium?
7. Native IME, selection and geometry on the target Mac (F20; open while the Mac is inaccessible).
8. Baseline check results and pre-existing failures at the C0 ref.

## Exact next launch message — C0 only

Also in `final/00-start-here.md` under "Copy/paste bootstrap message". Attach `final/01-master-plan.md` and `final/02-foundation-lead-prompt.md`.

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

## Addendum: owner decision, 15 October 2026 (16:46)

The owner confirmed that `work/ui-polish` and `work/daily-workflow-reliability` are still being worked on. Both will be finished and merged into `main` before the program starts. The final pack records this in:

- `00-start-here.md`: the decision section, the launch order, and C0 launch-message step 2;
- master plan §3 and §11 (release condition = landed; the paused path is only a fallback) and the §12 dependencies for C0, X1 and Z1;
- prompt 02, Bootstrap step 1.

The launch message above is the updated version. `final.patch` and `delta-from-amended.patch` were regenerated and re-verified after this change.
