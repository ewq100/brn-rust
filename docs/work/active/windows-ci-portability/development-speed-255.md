# Development feedback cycle — #255

## Outcome

No default performance configuration is retained. The package-only AI test
optimization trial made compilation faster but made complete workspace feedback
slower. The second bounded trial (AI codegen-units=512, optimization retained) also
regressed compilation and was rejected early. Product code, tests,
features, toolchain, locks and CI gates are preserved. This PR improves the
measurement/setup guidance and records a reproducible negative result.

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

The accepted comparison dataset is only `baseline-final` / `candidate-final`,
run under a coordinated quiet slot starting 15:44 UTC, with a process guard that
cancels its own process group on competing builds. Both arms start in new
artifact roots and use the same commands, features, helpers and repetition order.
Older trials below are exploratory/excluded, not pooled into headline medians.

Seconds: median (range), three repetitions per warm row:

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

The AI edit followed by the complete workspace gate costs approximately
102.31 → 108.24 s (sum of medians; not a separately timed combined
sample), **5.8% slower**. AI-only focused feedback gets
faster, but this is unsuitable as the default for general development and test
feedback. **Candidate 1 rejected.** No no-op/core/UI speedup is inferred from
small or overlapping results.

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
The owned compiler command included `-C codegen-units=512`. These regressions
justify early rejection; no gain or full-suite qualification is claimed. No
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
next group. Verify no crate/helper/lock/source drift; confirm against a fresh
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
paired outcome, not a guess from normal-library codegen. The execution regression
makes the initial optimization recommendation unsuitable as a default.

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
