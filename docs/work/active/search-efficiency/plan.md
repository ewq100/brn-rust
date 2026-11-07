# Search efficiency restart

Current assignment is the 7 November current request in issue #256 and the owner's explicit four-step chat request.

## Baseline and boundaries

Task checkout is `work/256-evidence-salvage` at remote main `75f3722142bfe5598c3871acc8a8079f991aad9a`. Original checkout has unrelated untracked `.vscode/` and `experiments/` directories. Preserve them, B's release candidate and C's separate work. Release publication is outside this assignment.

## Ordered execution

- [x] Select a small reusable extraction from #269/#271, retaining necessary maintained callers and regression tests. Inspect #270 but preserve its unqualified outcome.
- [x] Council checkpoint one reviews the selected source changes and proposes initial hypotheses. Lead fixes verified defects, runs focused tests and repository gates, obtains current-head code/security review and CI, and merges.
- [x] Verify commit-pinned historical evidence links, close superseded PRs with replacement links while preserving branches. Keep #254/#256 open.
- [x] Fresh product branch from merged main. Freeze Sol 6.1 medium, corpus, compact tuning and reserved confirmation sets, evaluator and runtime identities before the first live request. Verify baseline through grading, revise product and repeat comparisons, including reserved questions.
- [ ] Council checkpoint two reviews final product diff and measured results. Publish separate product PR, merge only confirmed improvement after repository gates, otherwise preserve baseline and report attempts.

## Review and experiment limits

Council has two seats, independent Codex/Sol and owner-selected Copilot/Opus after the initial Kiro quota failure. Kimi excluded. Two checkpoints, at most three rounds total, 30 aggregate wall-clock minutes and at most six seat dispatches including failed seats. Each review receives an eight-minute work allowance. Actual provider HTTP/token use remains unknown unless supplied by the route.

First live dispatch started at 06:24:46 UTC on 7 November, with deadline 10:24:46 UTC. The continuous four-hour window with at most 600 BRN/reader/grader HTTP attempts. Coding/building and grading occur inside it. Failed/uncertain requests count. Reserve repeated final comparisons and independent confirmation before tuning. Score all product input plus output tokens, cached input counted once, divided by fully correct source-supported answers. Unknown usage cannot qualify savings. Count retries and helpers. Preserve per-case completeness, support, abstention and visibility. Report grading/Council separately.

## Checks and acceptance risks

Use synthetic public fixtures and explicit fresh data directories. Evidence must retain failed tool calls and Message/archived records, refuse incomplete traces and unknown usage, and bind grades to actual packets, instructions and runtime. Tests must exercise these concrete regressions. Run relevant Python benchmark tests, formatting, Rust judge tests/lint and current repository CI. Historical scores remain unchanged by collector repairs. No improvement, merge, native observation or owner acceptance is claimed in this preparation record.

## Main reconciliation

Merged main `8b07e035` (#267 shared provider operations) into the salvage branch after its first review. Rust changes auto-merged. Preserved all main task records and the issue256 task, and regenerated the overview. Current-head checks and reviews are refreshed before integration.

## Salvage integration and product preparation

PR #273 merged at `bab8c8766fd30cadb110dfe3c57bbbd67c03ce70`, with all seven PR checks and current-head Codex code/security reviews clean. The integrated production/evaluator tree equals reviewed `90da784f`. Five pinned historical evidence links were verified accessible. Closed #269/#270/#271 as superseded with replacement links, preserving branches. #254/#256 remain open. Resulting-main CI is separately pending.

Product branch `work/256-product-hillclimb` starts from that merge. Fresh synthetic outputs are under `/private/tmp/brn256-20261107`. Offline native preflight passes all six cases. Immutable CLI, seeder and judge artifacts were prepared before live dispatch, with separate build commands and normal CLI feature closure.

Tuning questions are R8, R10, and the original R7 public/private questions projected as two read-only cases R7V/R7P on the same seeded corpus. The R7 writing step is excluded before results. R1/R4 are reserved for final comparison and are not used for tuning. The lead saw R1 during preparation, so no externally unseen-evaluator claim is made. All cases use 1000 filler records and two repetitions. The corpus/question fixture, native evaluator, rubric and Sol/high no-tool grader are frozen before the first live request. Grader deadline is 180 seconds and one HTTP attempt.

At most three product revisions. Reserve 216 conservative HTTP slots for final baseline/candidate comparisons across the four tuning and two reserved cases, two repetitions each. Global 600-slot reservations include product attempts (up to eight HTTP each), grader requests and failures, with no refunds. Actual HTTP and product tokens are reported separately. The window starts at the first live product dispatch and lasts at most four continuous hours, including all subsequent coding/building and grading. B's selected `82657cab` release qualification source is separate. Actual build/provider overlap is coordinated before live dispatch.

The initial live dispatch proceeded headlessly after B reported no build/provider workers and process inspection confirmed its retained app idle at 0% CPU. B normal closure remained pending on screen unlock. This replaces the earlier self-imposed app-closure wait with actual resource-overlap coordination.

## Measured product candidate

Three hypotheses were measured and independently graded. Conditional reformulation and compact reading guidance increased tokens and were rejected. Bounded first-hit exact passages reduced screening tokens 9.34%; the corrected runtime reduced final six-case tokens per fully correct answer 14.03%, with 12/12 fully correct answers in each arm. Both repetitions improved. All 56 product attempts and 56 grader attempts completed, using 254 HTTP attempts and 504 conservative reservation slots. See [product result](product.md) and [machine-readable evidence](summary.json). Council checkpoint two, current-head code/security review, CI and product merge remain.
