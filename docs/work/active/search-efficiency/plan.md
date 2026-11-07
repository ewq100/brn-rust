# Search efficiency restart

Current assignment is the 7 November current request in issue #256 and the owner's explicit four-step chat request.

## Baseline and boundaries

Task checkout is `work/256-evidence-salvage` at remote main `75f3722142bfe5598c3871acc8a8079f991aad9a`. Original checkout has unrelated untracked `.vscode/` and `experiments/` directories. Preserve them, B's release candidate and C's separate work. Release publication is outside this assignment.

## Ordered execution

- [ ] Select a small reusable extraction from #269/#271, retaining necessary maintained callers and regression tests. Inspect #270 but preserve its unqualified outcome.
- [ ] Council checkpoint one reviews the selected source changes and proposes initial hypotheses. Lead fixes verified defects, runs focused tests and repository gates, obtains current-head code/security review and CI, and merges.
- [ ] Verify commit-pinned historical evidence links, close superseded PRs with replacement links while preserving branches. Keep #254/#256 open.
- [ ] Fresh product branch from merged main. Freeze Sol 6.1 medium, corpus, compact tuning and reserved confirmation sets, evaluator and runtime identities before the first live request. Verify baseline through grading, revise product and repeat comparisons, including reserved questions.
- [ ] Council checkpoint two reviews final product diff and measured results. Publish separate product PR, merge only confirmed improvement after repository gates, otherwise preserve baseline and report attempts.

## Review and experiment limits

Council has two seats, independent Codex/Sol and owner-selected Copilot/Opus after the initial Kiro quota failure. Kimi excluded. Two checkpoints, at most three rounds total, 30 aggregate wall-clock minutes and at most six seat dispatches including failed seats. Each review receives an eight-minute work allowance. Actual provider HTTP/token use remains unknown unless supplied by the route.

The experiment has not started. First live preflight starts a continuous four-hour window with at most 600 BRN/reader/grader HTTP attempts. Coding/building and grading occur inside it. Failed/uncertain requests count. Reserve repeated final comparisons and independent confirmation before tuning. Score all product input plus output tokens, cached input counted once, divided by fully correct source-supported answers. Unknown usage cannot qualify savings. Count retries and helpers. Preserve per-case completeness, support, abstention and visibility. Report grading/Council separately.

## Checks and acceptance risks

Use synthetic public fixtures and explicit fresh data directories. Evidence must retain failed tool calls and Message/archived records, refuse incomplete traces and unknown usage, and bind grades to actual packets, instructions and runtime. Tests must exercise these concrete regressions. Run relevant Python benchmark tests, formatting, Rust judge tests/lint and current repository CI. Historical scores remain unchanged by collector repairs. No improvement, merge, native observation or owner acceptance is claimed in this preparation record.

## Main reconciliation

Merged main `8b07e035` (#267 shared provider operations) into the salvage branch after its first review. Rust changes auto-merged. Preserved all main task records and the issue256 task, and regenerated the overview. Current-head checks and reviews are refreshed before integration.
