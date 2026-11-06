# Cloud feedback Cycle 2 (#255)

**Outcome: no workflow or performance setting retained.** Earlier relevant test
feedback was observed, but the prescribed repeated, matched comparisons and
fresh confirmation were not qualified. The cycle stopped on a cache-validity
failure within its original allocation. This is an incomplete performance
qualification, not proof that focused-first is universally slower.

Cycle 1 remains complete through PR262, merge `6897fe56`, with its
[original findings](development-speed-255.md) unchanged. This separate cycle
changes documentation/evidence only. Production source, Cargo configuration,
test selection, required CI and B's updater step are unchanged by C.

## Source, environment and existing workflow

Frozen measurement source: `6897fe56ce9457fa7d60d0354b0ba4627d5049bb`.
The initial cloud checkout was stale at `aebfa12b` and was left untouched.
Task checkout: `/workspace/brn-cycle2`, branch
`agent-c/development-speed-255-cycle2`. Later updater/runtime main changes were
integrated only after measurements and probe restoration; these observations
are not relabelled as measurements of that later runtime.

Cloud Linux x86_64; Rust/Cargo 1.98.1; committed root/standalone locks; 4-CPU
cgroup quota, 16 GiB memory limit, no swap; 4 Cargo jobs; incremental disabled;
unchanged default/test profiles. Synthetic TMPDIR was
`/var/tmp/brn-cycle2-fixtures`. Helpers used the maintained one-job builder.
No owner Mac, native interaction, model assets or live product-provider calls.

Focused checks were **already supported** by developer guidance and prior task
practice. The maintained bulk scripts nevertheless build/lint before tests.
This experiment used that actual scripted entry point as its comparator; it
does not claim to invent focused-first development or benefit a developer who
already ran the relevant tests.

Both workflows completed these unchanged commands:

```sh
bash scripts/verify-storage.sh
bash scripts/verify-end-to-end.sh --fixtures-only
```

They cover formatting, locked helpers/staged siblings, workspace build, strict
all-target lint, workspace tests, scripted-document tests/lint, startup,
retirement checks, offline fixtures and the CLI journey. Hosted CI supplies its
additional platform/production coverage; no hosted speed comparison was made.

Candidate 1 prepended the existing package-only core test command. Candidate 2
prepended the matching workspace target kind: library tests for core/AI, binary
tests for headless UI state. The focused exit status was retained and the full
gate still invoked after a focused failure. No new runner was shipped.

## Protocol and finite allocation

Before candidate results: at most 60 minutes setup, then 90 minutes comparisons,
16 complete comparison workflows including failed/excluded attempts, plus the
two controlled-failure observations. One diagnosis and one final Council review
were allocated at most 30 minutes each; neither could run. At most three
candidates were allowed; two were examined, and no third was selected.

The trial window began 19:45:47 UTC on 6 November 2026, with an unchanged
21:15:47 UTC deadline (23:15:47 Tallinn). After baseline validation, thresholds
were frozen at at least 20% and 5 s earlier first relevant feedback per edit,
at most 10% median full-gate wall overhead, 15% CPU overhead and 20% RSS/disk
overhead (75 MiB artifact allowance). Coverage/failure behavior had to remain.

| Probe | Equivalent temporary edit | Intended executed filter |
| --- | --- | --- |
| Core | Reverse first locator equality operands | `locator::tests::` (4 tests) |
| Offline AI | Reverse first provider inequality operands | `codex_auth::portable_tests::` (1 test) |
| Headless UI state | Equivalent paused-maintenance `concat!` literal | `threads_state::tests::` (20 tests) |

The plan used no-op controls, A–B–B–A per probe, separate roots A/B (later B2),
original-source restoration, feature/freshness messages, helper/test hashes and
direct monotonic timing through both scripts. Root copies preserved mtimes and
used independent files, not hardlinks. Cache resets never occurred inside a
workflow. Repetition was later interleaved across probes without changing each
probe's intended A–B–B–A order, cap or deadline. It was **not completed**.

## Observations and decisions

These are direct complete-workflow observations, not sums of unrelated medians.
Each row below is a **single pair**, not a repeatable performance result.

| Observation | First relevant A / B | Complete gate A / B | Decision |
| --- | --- | --- | --- |
| Package-only no-op control | 64.90 / 26.36 s | 75.64 / 105.30 s | Reject unconditional prepend: +39.2% wall, +73.2% child CPU; isolated package resolves an additional graph. |
| Fixed-environment core edit | 226.84 / 77.28 s | 255.67 / 298.93 s | Unqualified: +16.9% wall, +4.8% child CPU; no balanced repeat/confirmation. A 45.3 s helper rebuild stays included in B. |

Candidate 2's original-source full/library/binary Cargo messages were fresh and
all common unit feature sets matched. AI retained `scripted-runs`, desktop
`default`. Its two core test executables had the identical SHA256
`fdd4d6e6950816ff44e558d57a24981391ad0c815cf2ffbd430cc767c8c6707e`.
That verifies artifact identity, not cache equivalence or a speedup.

The UI pair is excluded from performance comparison: A rebuilt core/AI while
only desktop source was edited. AI's 273.38 s complete baseline is unpaired.
All three edited paths executed nonzero intended tests and passing complete
verification; the UI focused sequence and the later AI focused/full correctness
check also passed. These correctness results do not complete the timing design.

## Failures, validity correction and early stop

- A missing `/usr/bin/time` caused one failed launch before any gate. The
  disposable observer instead used monotonic output timestamps and OS counters.
- The first cold gate failed after 832.68 s: container PID 1 retained a killed
  descendant as a zombie. The existing cloud subreaper launcher fixed the
  environment, with assertions/exit propagation unchanged. The successful
  qualification's fixtures-only child initially inherited older external helper
  overrides; comparisons then explicitly pinned the newly built helper bytes.
- The first core pair is excluded: nested environment activation appended native
  paths twice. Cargo's verbose receipt explicitly reports `PKG_CONFIG_PATH`
  changed for `aws-lc-sys`. Single activation, a fixed native-environment digest
  and fresh full/library/binary/helper preflights corrected that setup.
- One helper preflight stopped before starting a sample. Warm and subsequent
  fresh helper checks were separated. Helper rebuilds later still occurred;
  their cause remains unresolved and their time was never subtracted.
- **Final protocol failure:** separate targets shared one source checkout.
  Restoring a core edit and warming only B changed the source mtime seen by A.
  Its subsequent UI-only edit therefore included unintended core/AI rebuilds.
  Identical source/binary bytes did not establish matched starting caches.

Scheduling stopped at 20:51:45 UTC; the active AI workflow finished normally at
20:53:16 UTC. Nine of sixteen comparison attempts were used. All remaining
performance trials were cancelled: completing the required matched repetitions
and fresh confirmation was no longer justified within the unchanged allocation.
No trial allowance was reset or extended. Graceful controller stops occurred
after active observers completed, and no in-flight source was changed.

A future independently allocated comparison needs separate source checkouts, or
fresh original-source verification of **every affected root immediately before
each arm/probe**, including untouched earlier-probe dependencies and helpers.
An initial no-op or restoring only the active root is insufficient. This cycle
does not spend unused candidate slots or launch that follow-up.

## Work, resources and preserved evidence

Twelve instrumented complete workflows comprise three setup gates and nine
comparison attempts; eleven exited zero and the cold setup gate failed. Their
direct wall totals are 3,242.39 s and child CPU totals 6,989.46 s. These sums are
experiment work, not a synthetic end-to-end latency metric.

Across the full observed 87m17s window, container CPU counters increased by
10,758.79 CPU seconds, including intervening preflights, copies, restorations
and diagnostics. The 67m29s trial window accounts for 8,898.34 CPU seconds.
Quota throttling was 45.24 s over the full window. Raw counters preserve scope;
they are not candidate-only costs. Maximum-child RSS and block I/O are recorded
per workflow; RSS is not aggregate container peak memory. Retained targets after
verification occupied about 11 / 7.5 / 9.5 GiB (A / rejected B / B2), including
experiment setup and receipts, not normal-workflow artifact overhead.

[Normalized results](development-speed-255-cycle2-results.json) classify every
complete observation, with commands, raw-log references, tests, wall/CPU/RSS/I/O,
hashes and exclusions. The [raw archive](evidence/development-speed-255-cycle2/raw.tar.gz)
contains original logs, Cargo feature messages, preserved failed invocation lists,
the pre-closeout protocol and correctness receipts. Historical drivers are audit
material; their known shared-checkout flaw is not a recommended benchmark tool.
Cargo summaries include setup with compilation/linking; harness execution and
process-launch events remain separate in raw records. No pure-linker or
universal/Mac speed claim is made.

## Coverage and controlled failure

The harmless test-only probe failed exactly once in the focused command and
once in the unchanged full gate. Both exited 101; the combined focused/full
pipeline also exited 101 and invoked the full gate after the focused failure.
No other test failed in that probe. Original bytes were then restored and the
4 core tests passed. Formatting passes; all three probe file hashes match the
frozen source. No Rust, Cargo, runtime-guide or CI change remains from C.

The additional untimed AI driver initially omitted the same required subreaper
and reproduced the container-zombie failure. That failed receipt is preserved;
the corrected full gate and fixtures passed under the existing launcher,
including 108 CLI steps / 1,266 checks. This is correctness evidence after the
performance stop, not a replacement performance trial.

Local documentation validation: 329 tooling tests passed / 31 skipped (360
total); Markdown links, generated overview, shell syntax, whitespace and all
105 archived entry hashes passed. Platform-specific/native fixture skips are
not passing native evidence. The [local tooling output](evidence/development-speed-255-cycle2/local-tooling.log.gz)
is retained separately from performance observations.

## Review, integration and next stage

Council is **incomplete**: no installed Council skill was found in cloud/executor
catalogs or accessible skill paths, and no Council/T3/Kiro/Copilot delegate route
was exposed. Zero Council sessions ran; actual score is **no score**. This was
[reported once in coordination](https://github.com/ewq100/brn-rust/issues/253#issuecomment-6693449661).
No Mac sessions or credentials were copied. Ordinary Codex review is not Council.

Normal hosted platform CI, current-head Codex code/security review and unresolved
thread checks are tracked by the Cycle 2 PR linked from
[#255](https://github.com/ewq100/brn-rust/issues/255). Missing Council remains a
required gate; no waiver or completed merge is claimed by this report.

[Stage 2 (#263)](https://github.com/ewq100/brn-rust/issues/263) has a separate
read-only caller/policy map, outside these measurements. No implementation branch
or PR exists yet. A264 merged as `98802606`, and selected A265 is now merged as `a921e325`.
C still needs explicit release of shared provider/runtime files on that
reconciled runtime, plus Stage 1 closeout. B's first-alpha candidate,
publication window and frozen #254/#256 builds remain separate. The refactor's
maintainability evidence is not counted as a Cycle 2 result.
