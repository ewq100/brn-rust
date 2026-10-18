# Merged-main macOS job budget

18 October 2026. Program [PR 159](https://github.com/ewq100/brn-rust/pull/159) merged as `cb3717d09f15eea9af4eb9a238d0306a352a02f1` after all four required contexts and documentation passed on head `cf06a402`. The merged tree exactly equals that head's tree, `66a70b83a86f65f92d83f9eba26cebd842b0de8b`.

The first [merged-main run 39149776090](https://github.com/ewq100/brn-rust/actions/runs/39149776090), attempt 1, is **failed/incomplete**, not green. Linux Core/CLI, macOS Native UI, macOS retrieval/combined, and documentation passed. macOS Core/CLI job 117740852808 was cancelled with GitHub's explicit annotation: “The job has exceeded the maximum execution time of 30m0s”. Capability tests were running when cancellation arrived; later capability lint, CLI help and offline journeys were skipped. No assertion failure was reported in the cancelled step. The initial result remains preserved.

## Evidence and bounded disposition

| Observed step | Successful identical-tree PR | First merged-main run |
| --- | ---: | ---: |
| Helper preparation | 397 s | 571 s |
| Workspace tests | 366 s | 559 s |
| Workspace lint | Not run on Mac PR | 58 s, passed |
| Capability step start after job start | 21 m 13 s | 28 m 34 s |
| Job duration including cleanup | 22 m 54 s, passed | 31 m 7 s, cancelled |

The timing evidence establishes exhaustion of the cumulative job budget before the complete lane finished. It does not identify a parser or test performance defect, nor prove why individual runner steps took longer. Queue-capacity notices are not treated as execution time. A passing unchanged rerun would not explain the original cancellation.

The lead therefore allocates a bounded **45-minute macOS Core/CLI job limit**, retaining 30 minutes for Linux and Windows. macOS performs both helper build/test graphs and main also performs workspace lint; all existing commands, assertions, step conditions, required names and failure behavior remain unchanged. The finite additional margin lets those same checks complete without a cache-speed assumption. No production source, dependency, cache key, local runtime deadline, test limit or release policy changes. A further timeout or a substantive test failure must be investigated rather than automatically raising the limit again.

Full watch, job logs, annotations, exact timing comparison, source/tree and process-release evidence are preserved under `/private/tmp/brn-pr159-merged-main-ci-20261018`; its sealed manifest SHA-256 is `fb3f6e3a4afa7f373d2276828487b5781aae997311207440731fe54e545433e3`. Existing optional Linux/Windows retrieval and Windows Auth failures remain red and source-qualified separately. Native-human, owner and live-provider qualifications remain unchanged.

This correction must pass independent bounded review, its own required PR checks and actual merged-main verification. The program implementation is already merged; CI completion is still pending. No retry or passing result is claimed here.

Local precommit checks: nine existing CI-summary and intake-retirement/workflow tests passed, Markdown links passed (529 files, 2433 local links), and whitespace passed. No Cargo or application command was needed for this workflow/documentation-only delta. These static checks do not establish hosted execution of the new job limit.
