# Council diagnosis: opus

Route: acpRegistry_copilot / claude-opus-5.5
Options: [{"id":"reasoning_effort","value":"high"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3Aaa1436f8-ba63-41e4-a3be-f32116248a11%3Adelegate-task%3A255-diagnosis-opus-1>
Score: no score (diagnosis)

# Review of #255 (read-only, source e210c2c0)

The ranking below rests on two findings. First, a bug in the probe script contaminated the first repeat of the offline-auth and UI probes, so those two baselines need re-measuring. Second, the strongest lever is the test optimization level for the expensive `brn-ai` package. I edited no files and ran no builds or tests; everything below comes from reading the source and the evidence files.

## Issues found

**1. High: probe baseline is contaminated.** In `target/development-speed-255/run-probes.py`, `finally: p.write_text(original)` runs after the restore build has already finished. It rewrites the file with the same bytes but a new modification time, so Cargo treats it as changed.
- **Evidence:** `ai-edit-1` rebuilt `brn_threads_core` and its 115 dependent units, which is the core probe's set; `ai-edit-2` and `ai-edit-3` rebuilt only 86. `ui-edit-1` rebuilt the 86 `brn_ai` units; `ui-edit-2` rebuilt 1.
- **Impact:** the AI and UI medians and ranges are invalid, including the 66.6 s and 14.1 s outliers. The clean AI values are 59.84 s and 38.54 s; the clean UI values are 0.945 s and 1.259 s. The `ui-execute` outlier of 3.19 s may be macOS checking a freshly linked binary on first run.
- **Fix:** in `finally`, rewrite only when the bytes differ. Keep the contaminated logs. Re-run the frozen baseline with the fixed script, and have each run assert that the stale-unit set is exactly the expected one.

**2. Medium: execution baseline is not qualified.** Three `document_refine` tests failed with `Unavailable` because the document helpers were not built.
- **Fix:** build the helpers once into a fixed root outside every arm. Record their SHA-256, export `BRN_XBERG_HELPER` and `BRN_XBERG_HELPER_BRN`, and copy the helpers into each arm's `debug/` folder.
- **Then:** run the workspace suite three times with `--no-run` first, so execution is timed separately from compilation.

**3. Low: cold-build flags differ.** The cold control used `--timings`, while the probes used `--message-format=json`. Keep the flags identical per arm.

## Candidates, ranked

**1. Set `[profile.test.package.brn-ai] opt-level = 0`, keeping debug assertions and overflow checks on.**
- **Why:**
  - The `brn-ai` test unit (116.7 s, built from about 30.5k test lines and 603 tests) is the longest unit on the cold critical path.
  - Code generation is 86% of the normal `brn-ai` library's time (25 of 29 s).
  - CI compiles this unit twice: once in the workspace tests and again in the `capability-spike` step (142 s on Mac).
- **Why it's lower risk than a global change:** the earlier `opt-level = 1` gain (a full-size recovery/Undo test went from 396.8 s to 51.8 s, commit `2421792dd`) targeted JSON/base64 record processing. Those dependencies stay at level 1 under this change.
- **Risks:**
  - `brn-ai` tests run slower, and the deadline/stop tests could become flaky.
  - Downstream test runs that exercise `brn-ai` code (app, desktop, CLI) slow down.
  - The Cargo.toml change alters the CI cache key, so the first run after merge rebuilds from scratch.
- **Accept only if all of these hold:**
  - A cold `--timings` build in a fresh target directory shows a smaller `brn-ai` test unit and total time.
  - Over three clean repeats, the AI-edit and core-edit maxima fall below the re-measured baseline minima.
  - No-op and UI medians are not worse than the baseline maxima.
  - With helpers qualified, the workspace suite has the identical pass/fail/ignored set, and the `brn-ai` suite plus the recovery/Undo test stay within noise over three repeats.
  - Unit features in the timings output are identical to the baseline.
  - The `capability-spike` all-targets test and lint still pass.

**2. Only if candidate 1 passes: `[profile.test] opt-level = 0` with `[profile.test.package."*"] opt-level = 1`.** This drops optimization for workspace crates but keeps it for dependencies; `sha2` stays at 3.
- **Why:** the core-edit probe rebuilds 115 units at 33 s, and code generation is 87% of `brn_threads_core`'s time.
- **Risk:** generic code compiled into workspace crates loses optimization, which could undo the 7.6× recovery gain.
- **Reject if** that test regresses beyond noise. Measure with the same checks as candidate 1.

**3. Lowest priority, speculative: set `opt-level = 0` for the `aws-lc-sys` test build.** This targets cold builds only: its build script took 69.95 s on the critical path. CI restores dependencies from cache, so the benefit is limited to new worktrees. Measure the cold build only.

## Rejected or deferred

- **Splitting Mac CI jobs** (the 224 s document-helper API step, or the spike step): moving steps out of the required `Core and CLI (macos-15)` check would weaken branch protection unless the owner updates it. It also adds Mac runner cost, conflicts with B's PR258 edits to `ci.yml`, and you can't claim a timing gain because claims are local-only.
- **Narrowing the `capability-spike` step to examples only:** it changes the feature lane. That build of `brn-ai` has neither `scripted-runs` nor its gated code.
- **Replacing the crypto provider (aws-lc):** it's a product and dependency change.
- **Moving `brn-ai` tests into a separate crate:** this is the one maintainability boundary in play, and I recommend deferring it. One cold measurement plus a noisy AI probe isn't repeated difficulty. Revisit only if the `brn-ai` test unit still dominates the critical path after candidate 1, across two clean measurements.

