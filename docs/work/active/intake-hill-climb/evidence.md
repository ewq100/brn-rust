# Intake hill-climb evidence

One candidate was kept for speed. Mean fidelity did not improve. The single held-out evaluation confirmed the speed result under the predeclared spread rule. No newly lost expected element, lost unmanifested content or false added structure was found.

The [plan](plan.md), [frozen scorer](../../../../tools/intake-score/README.md), [candidate ledger](candidates.json) and [council record](council.md) preserve the protocol and decisions.

| Split | Helper | Fidelity baseline → final | Cold ms baseline → final | Warm ms baseline → final |
| --- | --- | --- | --- | --- |
| Tuning | Stock | 80.3707% → 80.3707% | 34.13 → 25.08 | 25.99 → 15.90 |
| Tuning | BRN layer | 82.0752% → 82.0752% | 34.10 → 25.30 | 25.98 → 16.80 |
| Held-out | Stock | 82.2240% → 82.2240% | 33.49 → 24.85 | 25.87 → 16.47 |
| Held-out | BRN layer | 85.7955% → 85.7955% | 34.33 → 26.20 | 25.89 → 15.97 |

These are medians of per-file medians. Raw samples, per-file spreads, exact missing elements and per-format summaries are in [tuning.json](tuning.json) and [held-out.json](held-out.json). Cold means fresh host/helper, not evicted OS caches. Warm uses a persistent host and fresh restricted helper. Both include source read and checked conversion; SQLite publication and native interaction are excluded.

| Candidate | Hypothesis and diff | Fidelity | Paired speed gains / spread threshold | Guardrails | Decision |
| --- | --- | --- | --- | --- | --- |
| 1 | Wake `process::run` through its bounded output channel; wait for successful process exit after EOF. | Unchanged on both splits | Tuning 8.40–9.62 ms / 2.17–5.06 ms; held-out 6.78–9.15 ms / 4.40–7.58 ms. Every mode/temperature passes. | All pass | Kept |

All eight formats and both helper modes imported successfully. Existing losses remain visible, including nested-table structure, PDF heading/order distinctions and some PPTX/EML content. No new PDF, Office or MIME fidelity repair is claimed. One candidate was tried; no candidate was rejected.

The external [tuning](tuning-guardrails.json) and [held-out](held-out-guardrails.json) keep gates are authoritative. The scorecards capture metric eligibility before those separate receipts and do not themselves authorize keeping a candidate.

The native local unsigned apps measured with `scripts/report-sizes.sh` were 138,472 KiB baseline and 138,476 KiB final, growth 0.0029%. See [size output](app-sizes.txt). These task-owned apps were assembled without launching or publishing them. Stock helper sources, all dependencies/locks and accepted patches are unchanged.

Timing used `/tmp/brn-timing-window.sh acquire intake 30`. It waited about 12 minutes for its two sub-100% CPU checks, admitted at 09:29:36 UTC, and released at 09:30:06 UTC. The immediate post-admission status read 108% CPU. Load varied despite the shared window; this is local timing evidence, not continuous machine isolation. The closest held-out margin was BRN warm, 8.57 ms gain versus 7.58 ms spread. The requested ABAB order retains possible position bias. Full admission/load/output/release evidence is in [timing-window.log](timing-window.log). The [single-use ledger](held-out.started.json) records the only held-out invocation.

Validation passed: 20 intake crate tests, 36 helper tests, 46 app intake tests, 5 CLI document-mode tests including 16 imports, 1 scripted in-process route test and 9 scorer controls. One existing ignored child-process witness is invoked by its parent cancellation test; no substantive check self-skipped. Strict intake and both helper Clippy lanes, formatting, local Markdown links and native release build passed. [Hashed receipts](test-receipts.json) link compressed logs. The first CLI command lacked required helper environment variables and failed setup before imports; [that failure](logs/cli-tests.log.gz) is preserved, followed by the correctly configured passing run.

Conversion scoring used no provider or external network. Each exact helper passed live-loopback network denial and forbidden-file denial witnesses. All hostile fixtures kept the same Invalid/Budget refusal category in both arms. Encrypted PDF remains deferred and unmeasured, with no new support or refusal claim.

Native observation, owner acceptance, broader timing/fidelity generalization and release are open. This work does not change #253/#268. The PR is not merged.

Final council round 1 scored Sol 9/10 and Opus 8/10, with no blocking findings.
Receipt source bindings and separate formatting/link/whitespace receipts address
the evidence finding. The [council record](council.md) lists the three accepted
nonblocking limits. Production and frozen scorer bytes have not changed since the
held-out measurement. The [post-council tuning recheck](post-council-tuning.json) and its
[keep gate](post-council-guardrails.json) passed. Held-out was not converted or
scored again.

The tuning recheck measured from 09:49:21 to 09:49:41 UTC and released immediately. Its [window log](post-council-window.log) records admission, source hash and load. The fresh formatting/link/whitespace/intake test receipts capture HEAD and process-source hash before each command.

The post-council paired gains were 6.81-7.69 ms against spread thresholds of 2.53-4.92 ms. Its fidelity scores and all loss/refusal/size/source gates remained unchanged. The [timing comparison](timing-comparison.html) shows this final tuning recheck and the single held-out evaluation.
