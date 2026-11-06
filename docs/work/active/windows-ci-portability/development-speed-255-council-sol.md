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



# Round 2 — final review

Route: codex / gpt-6.1-sol
Options: [{"id":"reasoningEffort","value":"xhigh"},{"id":"serviceTier","value":"default"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3A0d444406-0a49-4f34-8b58-7d6d083289dd%3Adelegate-task%3A255-final-sol-2>

**No actionable findings. Score: 9/10.** I reviewed HEAD `2283b8d8` against `origin/main` at `7ae7ebf1`, including the evidence, raw logs and probe scripts. Standards and task-scope review pass.

- **Scope:** All 11 changed files are documentation, evidence or routing. Production crates, manifests, lockfile, CI and runtime prompts match main.
- **Evidence:** Recorded timings, medians and ranges reconcile with observations. All 378 artifact identity/feature keys match; only the two AI optimization profiles differ, retaining assertions and overflow checks. Probe freshness, exact rebuild sets, nonzero test execution and helper/sibling hashes check out.
- **Decision:** Rejecting both candidates is justified. Opt0 improves focused compilation but worsens complete feedback; CGU512 has sufficient cold/core/auth regression evidence for early rejection. Missing optimization is an acceptable bounded outcome, not an invalid experiment. Excluded failures and partial samples remain visible.
- **Guidance:** The `docs/development/verification.md:79` capture useful corrections: source restoration, freshness, helper staging, feature identity and compilation versus execution. Deferring a crate/test split is proportionate.

My methodological reservation is limited confidence beyond this experiment: one cold control per arm, sequential arm order and ordinary host noise cannot establish universal performance. The reported 5.8% regression is explicitly a sum of medians, not a directly measured combined sample. These limits are disclosed and do not undermine rejection. Frozen Rig 0.45 results must remain separate from integrated Rig 0.46 claims.

**Security assessment:** Against the threat model, I found no introduced data exposure, credential disclosure, external-action path or authority expansion. Changed evidence normalizes local paths; shipped credential, provider, filesystem, checked-writer and cancellation enforcement remains unchanged. The disclosed cleanup failure is excluded historical evidence, with no mechanism shipped.

Evidence is sufficient for the bounded no-change conclusion. Current-head hosted CI, Codex/security review and integration remain pending; this review does not establish those gates or native/owner acceptance.

Read-only throughout: no edits, builds, tests, provider calls, delegates or watchers.



# Round 3 — corrected evidence recheck

Route: codex / gpt-6.1-sol
Options: [{"id":"reasoningEffort","value":"xhigh"},{"id":"serviceTier","value":"default"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3A0d444406-0a49-4f34-8b58-7d6d083289dd%3Adelegate-task%3A255-recheck-sol-3>

**Score: 9/10.** The material corrections support publishing a **NO-CHANGE / incomplete-qualification** result, subject to the separate hosted gates. No High or Medium findings remain.

Reviewed `ac00b931` against `7ae7ebf1` / local `origin/main`. All 11 changed files are documentation or evidence; the checkout remains clean.

The corrected account preserves the necessary distinctions:

- Faster compilation was observed; execution effects and net benefit remain unresolved.
- Earlier candidate workspace readings reconcile with the observations and contradict treating the later slowdown as a configuration effect.
- Hash identity is expressly limited to three binaries; `agent_contract` differs.
- CGU512 is described as an unpaired slower observation with no demonstrated gain.
- Guidance now covers alternation, repeated execution of identical artifacts, limited process-guard coverage, and explicit manifest restoration.
- Raw results, exclusions, and historical review disagreements remain visible.

One **Low, nonblocking accuracy issue** remains at `docs/work/active/windows-ci-portability/development-speed-255.md:178`: “Final attribution is the controlled paired outcome” retains the superseded framing. The following sentence withdraws the regression claim, limiting its impact. Replace it with: “The final sequential pair provides observations; earlier same-artifact runs prevent causal execution attribution.”

**Security assessment:** No introduced security issue identified against the invariants and threat model. Runtime authority, credentials, provider dispatch, checked writes, and cancellation enforcement are unchanged. Inspection found no credential material or private absolute paths in the changed files.

This recheck reconciled the correction with retained evidence; it did not independently rehash local binaries or reproduce measurements. Current-head hosted CI, Codex review, and security review remain pending. No edits, builds, tests, provider calls, delegation, publication, or watchers were performed.

