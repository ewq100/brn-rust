# Development feedback cycle — #255

## Outcome

No default performance configuration is retained. AI test opt-level0 repeatedly
showed faster compilation, but its execution effect remains unresolved: earlier
runs of identical core binaries were near baseline speed, while later runs were
substantially slower. The sequential protocol cannot attribute that difference
to optimization. The budget ended without a qualified net improvement. The
codegen-units512 arm also showed no benefit, but lacked a contemporaneous control
in the degraded-host window. Product code, tests, features, toolchain, locks and
CI gates are preserved. This PR records incomplete performance qualification and
corrects measurement/setup guidance; it does not prove these levers harmful.

Scope: [#255](https://github.com/ewq100/brn-rust/issues/255), ordered by
[#253](https://github.com/ewq100/brn-rust/issues/253). Current status/gates live in
task `development-speed-255` in the [existing selected record](record.json).
[Normalized observations](development-speed-255-observations.jsonl) preserve
successful, failed, interrupted and calibration work. [Results and
protocol](development-speed-255-results.json) include individual repetitions,
identity checks and the environment. Local raw logs/compilation artifacts remain
preserved in the task checkout under `target/development-speed-255` and the
separate `target/255-*` roots; these are not shipped build tooling.

## Frozen comparison and results

Source: `e210c2c0fceb6bff4d6c24ee708f95f6ca5ec3df`, tree
`4e31aa801714501571f02c8a6799d657e25aeec2`. Rust/Cargo 1.98.1,
`aarch64-apple-darwin`, Apple M5 Pro, 15 logical CPUs, 24 GiB, Cargo jobs=4,
`CARGO_INCREMENTAL=1`. Registry and OS caches are warm; clean compilation means
an empty, separate Cargo artifact root. No machine-global cache deletion.
Normal background desktop load remains a limitation. Claims apply to this host,
source and feature graph; there is no Linux, Windows, product-runtime or hosted
CI speedup claim.

The final sequential dataset is `baseline-final` / `candidate-final`, starting
15:44 UTC in a reserved build slot. Both arms use new artifact roots, matching
commands/features/helpers and repetition order. The guard detects build/compiler
processes, not all background load, thermal effects or filesystem work. Baseline
always ran first; there was no alternation. Earlier same-artifact execution is
therefore essential counterevidence, not an observation to discard.

Observed seconds: median (range), three repetitions per warm row. Differences
are not causal execution/configuration effects:

| Work | Original test optimization | AI package opt-level=0 |
| --- | --- | --- |
| No-op compilation | 0.94 (0.91–1.08) | 0.31 (0.30–0.31) |
| Core edit compilation | 20.49 (17.96–21.17) | 18.46 (17.91–20.62) |
| Offline-auth edit compilation | 35.30 (35.03–36.63) | 16.45 (15.54–16.63) |
| Headless UI-state edit compilation | 0.97 (0.94–0.97) | 0.99 (0.95–1.01) |
| Complete AI suite execution | 10.11 (10.11–10.74) | 12.09 (12.08–12.12) |
| Complete workspace execution | 67.01 (66.37–102.94) | 91.79 (91.69–148.17) |

Single clean compilation control per final arm: 214.28 s →
133.40 s. Earlier clean controls were 245.46 / 136.93 s;
the spread is preserved, and a single cold pair is not universal performance
proof. The AI test executable grows from 44.01 to
67.68 MiB. Peak RAM was not measured.

The AI edit plus workspace arithmetic is102.31→108.24s (+5.8%), a sum of
medians from this later sequential pair. It is **not a proven regression**. Earlier
`candidate-runtime` workspace runs were69.90/67.11/67.08s (same1491/0/9), near
baseline. The CLI, AI test and journey binaries have identical SHA-256 hashes
across those candidate slots (see results JSON); not every test binary is
identical—`agent_contract` differs. Identical important binaries ran at baseline
speed earlier and slower later, which confounds the execution conclusion.

Compilation gains are repeatedly observed on this frozen source. Execution
qualification and net benefit remain unresolved, so **Candidate1 is not retained
within the budget**. No no-op/core/UI causal speedup is claimed. The no-op ranges
are disjoint, but no configuration is compiled during a no-op; host/cache effects
are a plausible explanation, not an optimization result.

Both final arms passed all three repetitions of the AI suite (598 passed,
2 retained ignores) and full workspace (1491 passed, 0 failed, 9 retained
ignores). The selected tests actually execute: core locator 5, offline auth 1,
headless UI state 20. Compilation rebuild sets match the clean reference for
each edit. All 378 artifact identities/features match; only the AI normal/test
library opt-level changes, with debug assertions and overflow checks retained.
Global test optimization stays 1 and checksum optimization 3 in the trial.
Ignored/zero-test doc summaries are not counted as new behavior proof.

## Candidate 2: codegen units only

Kept the original optimization/checks and changed only AI codegen units to512.
Single clean control: 263.03 s versus214.28 s baseline. Three core-edit
compilations: 28.69, 32.99, 30.38 s (median 30.38),
versus17.96–21.17 s baseline. First auth edit: 56.16 s versus35.03–36.63 s.
The preserved candidate manifest/script configure512CGU and the AI fingerprint
changes with optimization/checks retained; the observed compiler argv was not
preserved as a raw log. These are observed slower timings in one unpaired arm, following the degraded
host window. No benefit was demonstrated, so it was not retained; a causal
compilation regression is not established. No full-suite qualification is claimed. No
third candidate was entered: remaining proposals were broader/riskier and the
finite budget had no justified next trial.

Early-stop housekeeping initially failed to identify a truncated process name;
configuration was restored before the last owned trial had fully stopped. Later
partial auth observations are excluded. The corrected stop targeted only the
verified owned Cargo process group; all trial processes were reaped and every
Rust/configuration byte restored. Original and partial artifacts remain intact.
No performance configuration is retained.

## Reproduction

Use a clean isolated checkout at the frozen source and the repository toolchain
and locks. Reserve the host; do not overlap builds or Council. Build both document
helpers once with the maintained `scripts/build-document-helpers.sh --offline
--jobs 1 --target-root ABSOLUTE_TASK_ROOT`. Use both explicit helper paths in the
results protocol, and stage hash-identical copies beside the built CLI because
some children deliberately clear BRN variables. Stock helper SHA-256:
`114c7955fde6c465363c7ca730a14b7a11efc9aa34a5ee54f9eaa5d7a7d0ffbf`;
adapted:
`e87072a261f0490238a862a93478ea2d801469845b27a1db09930d670d9adaef`.
Fresh synthetic fixtures use a canonical absolute temporary parent outside Git.

Set a different absolute `CARGO_TARGET_DIR` per arm, `CARGO_BUILD_JOBS=4`,
`CARGO_INCREMENTAL=1`. Compile with:

```sh
cargo test --workspace --no-run --locked --offline --timings --message-format=json
```

For candidate 1 only, add `[profile.test.package.brn-ai]` with `opt-level=0`.
Measure a clean control, fully fresh no-op, and three repetitions of each exact
single reversible edit (restoring/compiling between repetitions):

| Probe | File and edit | Executed filter |
| --- | --- | --- |
| Core | `crates/brn-threads-core/src/locator.rs`: first `stored == canonical` → `canonical == stored` | `locator::tests::` |
| AI | `crates/brn-ai/src/codex_auth.rs`: provider inequality operands reversed | `codex_auth::portable_tests::` |
| UI state | `crates/brn-desktop/src/threads_state.rs`: `!settings.maintenance` → `settings.maintenance == false` | `threads_state::tests::` |

Every compilation covers the complete workspace graph. Run the selected emitted
executable with its filter and `--test-threads=4`; use the emitted AI executable
without a filter for the complete AI suite. Run full execution with:

```sh
cargo test --workspace --no-fail-fast --locked --offline -- --test-threads=4
```

Compilation setup is separately recorded (fully fresh Cargo setup about a
fraction of a second); full-workspace wall rows include that setup and process
launch overhead. The first complete execution is slower than the next two and
remains in the range. Source edits must be restored only when bytes differ,
then compiled; check no-op freshness and exact rebuilt artifact sets before the
next group. The one-off final-pair script's `finally` restored its candidate
manifest, not baseline; explicitly restore the original manifest afterward and
verify source/config hashes, as this work ultimately did. For future execution
qualification alternate arms (ABBA), reuse/hash the same binaries across slots,
and control/observe non-compiler host load; Cargo-only guards are insufficient. Verify no crate/helper/lock/source drift; confirm against a fresh
original-baseline run. Do not run these semantic-equivalence probes on a platform
where their function is excluded; this cycle qualified the Unix/Mac paths only.

## Council, bounded work and exclusions

One diagnosis and one final independent review, maximum a third pass only for a
concrete unresolved issue; ten minutes/member/pass. Measurement budget starts
12:26:01 UTC and ends 16:26:01 UTC, including failures/reruns and resource waits;
resumption never resets it. Only two focused candidates were entered. Council
and live coding assistance were outside timed comparisons. No timed
maintainability coding-task comparison was attempted.

Requested/actual seats: fresh Codex `gpt-6.1-sol` / `reasoningEffort=xhigh`;
Copilot `claude-opus-5.5` / `reasoning_effort=high`; Copilot `grok-4.7` /
`reasoning_effort=xhigh`. Grok was initially absent and appeared after the owner
updated configuration, before dispatch; no substitution. Account routes come
from T3 receipts. Diagnosis runtime: 267.8 / 207.9 / 528.9 s (parallel wall about
529 s). Token/cost totals are not exposed; do not infer them. Diagnosis had no
scores. [Sol](development-speed-255-council-sol.md),
[Opus](development-speed-255-council-opus.md),
[Grok](development-speed-255-council-grok.md) preserve the returned findings.

Sol/Opus found that unconditional cleanup rewrote already-restored bytes and
contaminated the next probe. Fixed, with complete fresh/artifact-set checks,
before final measurements. All ranked the narrow AI profile lever; Grok required
more attribution and stricter acceptance. Final attribution is the controlled
paired outcome, not a guess from normal-library codegen. Final review exposed conflicting same-binary execution evidence, so the lead
withdrew the regression claim. The optimization remains unqualified, not disproven.

Retained failures/exclusions: initial AI execution 595 passed/3 helper-Unavailable
failures/2 ignored; missing helper setup fixed before qualification. Initial
probe cleanup and Agent B overlap excluded. Later A and unrelated Codex builds
interrupted/refused trials; guard cancels only owned processes. Owner selected
yield until external work finished; A honored later holds. First candidate
workspace setup had 1483 passes/8 failures/9 ignores because sibling CLI helpers
were missing; hash-identical staging fixed all48 affected tests and three full
reruns. No tests removed, relaxed, self-skipped or hidden.

## CI and maintainability limits

Historical successful CI 42009674068 at `37e8d1d7`: Mac Core/CLI 1227 s,
workspace test step447 s, provider fixtures142 s, helper API/lint224 s. Required
Mac completion1230 s from run creation (about3 s queue/provisioning). These are
investigation leads, not before/after claims. Frozen-source CI42216749186 was
cancelled, with a Windows native failure; not a passing baseline. Local
incremental-on times do not establish CI performance with incremental off.
No jobs, contexts, triggers, cache identities, helpers, platform/feature lanes or
failure propagation were changed. Agent B's updater step remains its ownership.

One hotspot assessed: the large private AI test unit. Defer a crate/test-layout
split: size/compile cost does not show repeated interface/ownership difficulty,
and a split may require new APIs/build work. Wider workspace deoptimization,
crypto/Rig changes, CI sharding/caching and another architecture survey are
outside this cycle. No maintainability speedup is claimed. Lasting guidance now
covers correct freshness, helper identity/staging, failed evidence and comparable
cache/feature conditions. Hosted current-head CI, Codex/security review and normal
integration status are recorded in the existing task/PR; no native or release
acceptance is claimed for this headless work.

## Review disagreement and disposition

Round2: Sol9/10 found no actionable issue; Grok8/10 endorsed rejection with
documentation nits; Opus5/10 found the material time-slot/same-binary confound.
The lead verified identical CLI, AI-test and journey hashes and the earlier
67-second candidate runs. Opus's claim that every test-binary hash was identical
is too broad (`agent_contract` differs), but the verified counterevidence is
sufficient to withdraw causal execution and codegen-regression claims. Scores
and majority agreement do not resolve this evidence problem. Guidance now adds
alternation and same-artifact cross-slot checks. No new experiment is authorized
by this correction; round3 is a bounded recheck of these material corrections.
