# BRN CI cost triage (bounded, read-only)

Assessment scope: checkout `/Users/evokessler/.codex/worktrees/vacation-hardening/brn-rust`, branch `codex/agent-first-followthrough`, HEAD `f671bdab8962b19e98d204ae6bc612ea200034d4`; requested baseline `main` `20fcd7383662cd2f67607c3ce04194aed50809b5` (the two inspected run SHAs below are ancestors of the requested baseline). Worktree was clean. No repository files changed and no tests/builds were run.

## Source review

Read repo `AGENTS.md`, the vacation-campaign test-families CI cost section, `docs/development/verification.md`, `.github/workflows/ci.yml`, `.github/actions/setup-rust/action.yml`, `scripts/build-document-helpers.sh`, and `scripts/prepare-xberg-adapted.py`.

The required PR contexts are the Core/CLI Linux and macOS jobs, Native UI macOS, and Native retrieval macOS. CI also runs documentation/tooling and Linux UI checks. Core/CLI includes Windows on PRs; main/manual additionally probes native Windows lanes. The shared Rust setup pins the repository toolchain, restores the existing `Swatinem/rust-cache` cache scoped by OS/architecture/lane/Cargo.toml, fetches locked dependencies, and creates explicit fixture parents. CI prepares stock and adapted helper graphs and uses their artifacts in multiple test journeys. Existing verification notes explicitly protect these helper identity/restriction contracts and reject blanket test retirement.

## Observed run timings

Durations below are GitHub job/step wall elapsed times computed from API `startedAt` and `completedAt`; sub-second steps round to zero. They are not CPU time or summed-body duration. Run 39392977994 was PR event SHA `7225c20877edcb5c4dd20d50d530095682d10c40`, created 2026-10-20 08:22:53Z. Run 39392850688 was the preceding main push SHA `02445b283d85ac09d24f7ee7f111039a87b4dbb5`, created 08:21:50Z.

| macOS lane | Main job elapsed | PR job elapsed | Helper preparation step (main → PR) |
|---|---:|---:|---:|
| Core and CLI | 24m59s | 22m30s | 6m36s → 7m02s |
| Native UI | 38m59s | 33m53s | 12m11s → 12m14s |
| Native retrieval + combined build | 13m42s | 14m05s | 6m36s → 7m08s |

Other notable PR macOS elapsed steps: Core workspace tests 6m01s, helper normal API/restrictions tests+lint 5m16s, scripted CLI route 39s, capability fixture test 1m59s and lint 12s. Native UI reusable writing UI test took 8m39s; native Unix widget tests 1m24s. Retrieval current canonical search test took 2m39s and combined desktop state test 1m18s. The main equivalents for workspace tests/helper tests were 7m17s/6m19s, writing UI tests 10m43s, and retrieval canonical search/combined desktop tests 3m16s/6m03s. This variation across two commits/runs is too large to attribute to a code optimization or stable test cost.

The PR Native UI macOS job started 12m55s after the PR run creation, while the main run's Native UI macOS job started 9s after main run creation. This is only an observed start offset, not a measured queue duration: the API timestamps do not identify runner allocation/queue time separately. No queue estimate is used as optimization evidence.

## Disposition

**No implementation candidate recommended from this bounded sample.** Helper preparation is the largest repeated named step (about 6.5–12.2 minutes per lane), but it contains required locked fetch/materialization/build work. The helper test/lint step uses the same explicit per-mode target directories created by the preparation script, so the workflow already reuses those compilation outputs. All three macOS lanes keep separate scoped caches and each has a distinct tested purpose; removing a lane, broadening feature cache sharing, or shipping helper artifacts from a new shared job would require additional integrity/feature-isolation evidence and workflow design beyond this two-run assessment. The campaign cost record itself says different cache/commit conditions do not establish improvement.

Workspace and native test durations are also significant, but no family-level test timing or guarantee-equivalence evidence in this assessment establishes a safe consolidation. Keep all current required checks and helper guarantees. Better cost comparison needs the workflow's recommended repeated controlled observations (same commit/toolchain/features/platform/cache condition and exact invocation, recording compile/check and body times separately) before proposing an optimization.

Limitations: only two runs, on different SHAs and events, were examined; cache hit/miss metadata and per-test body durations were not collected; GitHub job timestamps do not expose actual queue time. The PR run had optional Windows job failures while the observed macOS required lanes completed successfully; this triage does not diagnose those failures.

Lead reconciliation: the inspected lead checkout had docs-only divergence from requested main20f; its production tree matches. Corrected the original report’s ancestry and Windows-lane wording against Git/workflow source. Original report is preserved losslessly in `ci-cost-triage-original.md.gz`.
