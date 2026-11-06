# Council diagnosis: sol

Route: codex / gpt-6.1-sol
Options: [{"id":"reasoningEffort","value":"xhigh"},{"id":"serviceTier","value":"default"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3Aaa1436f8-ba63-41e4-a3be-f32116248a11%3Adelegate-task%3A255-diagnosis-sol-1>
Score: no score (diagnosis)

Recommend measuring the narrow `brn-ai` profile change first, after correcting the probe harness. No speedup is established.

**Concrete finding — P2: probe transitions include unintended rebuilds.** At `target/development-speed-255/run-probes.py:34`, `finally` rewrites the original source after its restore build, invalidating Cargo freshness again.

Evidence confirms the effect:

- AI edit 1 rebuilt 115 units, including core; subsequent AI edits rebuilt 86.
- UI edit 1 rebuilt 86 units, including `brn-ai`; subsequent UI edits rebuilt only desktop.

Proposed fix: restore only when bytes differ, then verify a fully fresh compilation before each probe group. Preserve existing results and repeat the original baseline with the corrected harness. The current ranges describe mixed workloads.

1. **Highest priority: override only `brn-ai` test optimization to level 0.**

   Add `[profile.test.package.brn-ai] opt-level = 0` in `Cargo.toml:14`, retaining global level 1, `sha2` level 3, debug assertions and overflow checks.

   Evidence supports an experiment: the AI test unit runs from 128.57–245.29 s and finishes last. Normal AI compilation spends approximately 25 of 29 s in code generation. However, the test build contains substantially more code; its 116.72 s duration does **not** prove optimization accounts for the difference.

   Risks: slower fixture execution, deadline-sensitive failures, larger unoptimized code, and changed costs for the normal AI library built under the test profile. This override is package-wide within that profile.

   **Acceptance:** one empty-target compilation per arm; corrected three-repeat no-op/core/auth/UI probes; separately timed, helper-qualified complete AI and workspace execution. Also retain the separate `capability-spike --all-targets` test/lint lane. Compare compile-plus-execution time, preserving every assertion, ignored case and failure. Reject gains offset by execution regressions or indistinguishable from baseline variability. Confirm retained results against a fresh original-baseline arm.

2. **Lower priority: measure focused compilation for earlier local feedback.**

   The existing probes compile every workspace test before running 5 core, 1 auth or 20 UI tests. Measure package/target-focused compilation followed by those same tests, while retaining the complete workspace gate afterward.

   This could shorten time to the first relevant result, especially for core edits that currently invalidate 115 units. It does not demonstrate faster completion of equivalent full-workspace work.

   **Acceptance:** three repeats with the same edits and selected test identities; record time to focused result **and** time through the subsequent complete gate. Preserve workspace-equivalent features where required—the baseline AI artifact enables `scripted-runs`. Compare artifact feature identities explicitly. Use existing documented commands; a new orchestration layer is unnecessary.

**Defer CI restructuring.** The historical 447 s workspace, 142 s provider-fixture and 224 s helper-verification steps identify investigation targets, not causal savings. Before changing scheduling, partition compilation from execution on the same source and record cache state, queue/provisioning, required completion time and aggregate runner work. Local incremental-on results cannot establish CI performance with `CARGO_INCREMENTAL=0`.

Do not combine default/scripted and capability feature configurations merely because test names overlap. Preserve helper source/target associations, stock/adapted identities, qualified one-job builds, cache keys, failure propagation, all platform lanes, and B’s additional updater step.

**Maintainability: explicitly defer a `brn-ai` crate or test-layout split.** The large compilation unit exposes cost, but this evidence does not establish repeated boundary difficulty or justify production-interface changes. Revisit only if the profile experiment leaves a measured bottleneck and a bounded split demonstrates lower total build and maintenance cost.

Reviewed frozen HEAD `e210c2c0`; no edits, builds, tests, provider calls or delegation performed.

