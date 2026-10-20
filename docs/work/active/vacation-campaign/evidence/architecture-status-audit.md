# Architecture opening status consistency audit

Read-only audit at requested checkout `5a181090` (product source equals `origin/main` `9c18bc5f`). I left the pre-existing dirty `campaign.json` and `docs/work/overview.md` untouched.

## Finding

The stale current-status language is in the opening of `docs/architecture/threads-target.md`; `docs/architecture/overview.md` already routes current status to `docs/status.md` and has no pending implementation/merge claim to correct.

- `threads-target.md:1` presents the 18 October local combination as the current snapshot and says combined verification and independent review are pending. That snapshot predates PR #159. Git records the normal merge as `cb3717d0` on 18 October, with `57c710cc` as one parent, and `git merge-base --is-ancestor cb3717d0 origin/main` succeeds. The historical integration record says PR #159 merged after all four required contexts passed on its head; the initial merged-main run was separately incomplete from a 30-minute Mac job timeout, and the later timeout correction/CI results are separately recorded. Thus the blanket pending label is stale, while it would also be inaccurate to silently claim every later CI/native/owner gate passed.
- `threads-target.md:5` says the accepted stock-default/optional-adapted converter direction “is not yet implemented or verified.” That is no longer a current fact: the component work and distribution are recorded in Tasks 1–4, and the combined source is in main through PR #159. This sentence also conflicts with the adjacent `:7` summary that Tasks 1–3 already implemented preferences, Documents controls, and checked routing.
- `threads-target.md:7` retains a Task3-era state phrase saying distribution, full review, and integration are pending. Task4 and the later PR159 integration superseded those pending gates. The useful boundary “Normal stock output is not a claim of complete custom fidelity” remains valid; keep it and all qualification limits in the dated task handoffs.
- `threads-target.md:10` labels a list of October 11-and-earlier merged milestones simply “Status.” Its individual dated merge facts are historical and need not be removed; relabel the paragraph as an implementation-history snapshot and direct readers to the current overview for live execution status.

## Smallest safe disposition

Replace the first status block (`threads-target.md:1–7`) with a short explicit route: current implementation, integration, verification and remaining acceptance gates belong in [the development overview](../../../overview.md); the following X2 Task1–4 records preserve dated component and combined qualification history; representative stock conversion does not establish complete custom fidelity. Preserve links and historical records, including the first merged-main timeout and other failures. In particular, remove the present-tense “pending” claims from lines 1 and 7 and the “not yet implemented or verified” sentence from line 5. Keep native-human and owner acceptance distinct from code, checks and merge.

Change only the label at `threads-target.md:10` from “Status” to “Historical implementation milestones through 11 October 2026,” with one link to the overview for current status. No contract wording or historical PR facts need deletion.

Keep `docs/architecture/overview.md:3` as is: it identifies the page as a navigation summary and points readers through `docs/status.md` to current repository status. `docs/status.md` and `docs/README.md` already route that to the generated overview/current task record, consistent with the development guide. No duplicate execution tracker is needed.

## Evidence bounds

Current campaign overview records PR190 merged and its resulting-main checks pending, plus PR191 checks in flight at its observation time; that is the right kind of changing gate state to keep out of the canonical contract. The PR159 source remains merged in current main. This audit does not claim every historical PR159/160 required-main result, native observation, or owner acceptance is complete; those limits remain in their source-bound records and should not be collapsed into a green architecture-wide status.
