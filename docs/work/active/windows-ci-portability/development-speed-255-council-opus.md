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



# Round 2 — final review

Route: acpRegistry_copilot / claude-opus-5.5
Options: [{"id":"reasoning_effort","value":"high"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3A0d444406-0a49-4f34-8b58-7d6d083289dd%3Adelegate-task%3A255-final-opus-2>

# Council review round 2 for #255 (HEAD `2283b8d8`): 5/10, changes needed before merge

**Verdict:** Changes needed. Keeping no performance change is acceptable, but the reason given for rejecting Candidate 1 is contradicted by the PR's own preserved evidence.

This was a read-only review. Nothing was edited, built, tested or run against a provider.

## What checks out
- **Scope:** The diff from `origin/main` to HEAD touches 11 documentation and evidence files only. `crates`, `Cargo.toml`, `Cargo.lock`, `.github`, `agent`, `rust-toolchain.toml` and `.cargo` are byte-identical to main.
- **Final-pair numbers:** Every number in the markdown table matches `results.json` and `final-pair.log`. The derived figures also hold: test executables 44.01 → 67.68 MiB, combined AI-edit-plus-workspace 102.31 → 108.24 s (+5.8%), and 1491/0/9 test counts in all six workspace runs.
- **Candidate 2:** The rows for codegen-units=512 match the observations file, and the two excluded rows are marked as excluded.

## Findings

**1. High — The workspace "regression" behind rejecting Candidate 1 is contradicted by identical binaries.**
- **Where:** `development-speed-255.md` (the 91.79 workspace row, the "5.8% slower" sentence and "Candidate 1 rejected"); `record.json` attempt 4; the overview "Owner attention" line.
- **Evidence:**
  - The `candidate-runtime` workspace runs (observations rows 72–74, around 14:12 UTC, no competing builds) took **69.90 / 67.11 / 67.08 s**, all 1491/0/9.
  - The `candidate-final` runs (around 16:07 UTC) took **148.17 / 91.69 / 91.79 s**.
  - Both arms ran the same opt0 artifacts:
    - `debug/brn` SHA-256 `83f19589…` in both.
    - `brn_ai-0ab1430b85b5c63e` SHA-256 `9c1edc4e…` in both.
    - The ordered list of test-binary names and hashes is identical (same md5).
  - The per-binary `finished in` totals were 52.71 s (earlier opt0), 69.41 s (final opt0) and 52.96 s (baseline). The extra time is in subprocess-heavy suites such as `agent_journeys` (5.9 → 9.6 s), `retrieval_eval` and `user_scale`.
  - So identical binaries ran at baseline speed in one slot and 37% slower in another. The difference comes from the environment or run order, not from the setting.
- **Why it wasn't caught:** The pair always ran baseline first and candidate second, with no alternation. `quiet.py` only detects `cargo`, `rustc` and `cargo-clippy` processes, so any other host load was invisible to it.
- **Same issue, smaller:** The AI suite was 10.81–12.57 s earlier and 12.08–12.12 s in the final run, with the same binary.
- **Disclosure gap:** The markdown mentions "three full reruns" but omits their timings, which contradict the headline.
- **Proposed fix:**
  - Restate Candidate 1 as: compilation clearly faster (AI edit 35.3 → 16.4 s, two cold pairs both faster); execution effect **unresolved or confounded**; not retained because it was not qualified within the budget.
  - Drop "full-suite execution regresses" and "~6% slower" as findings. Cite the 67 s same-binary runs and the time-of-run confound.
  - Make the same wording change in `record.json` and the overview, and add a protocol note to `results.json`.
- **Status:** This is an invalid comparison, not a measured lack of improvement.

**2. Medium — Candidate 2's rejection relies on a single arm with no baseline from the same time slot.**
- **Where:** the Candidate 2 section and `results.json` `.candidate2`.
- **Evidence:** It ran 16:13–16:26 UTC, right after the slot where identical binaries slowed down. It is compared only with baseline numbers taken about 30–40 minutes earlier. Its cold time (263 s) also sits within the cold spread already recorded (214–245 s).
- **Proposed fix:** Describe it as "observed slower in one unpaired arm during a degraded-host window; no gain shown; not retained." Do not say it "regressed compilation."
- **Status:** Missing improvement, weakly evidenced.

**3. Medium — The new measurement guidance misses the lesson this experiment actually taught.**
- **Where:** `verification.md`, "Measure development feedback."
- **Gap:** It covers freshness, helper setup and preserving failed attempts. It does not cover run-order and time-slot effects.
- **Proposed addition (two sentences):**
  - "Alternate arms (ABBA) for execution timings and reuse the same built artifacts; before attributing a difference to configuration, re-run identical binaries across slots."
  - "A process guard limited to Cargo does not prove the host was quiet."

**4. Low — The final-pair script restores the candidate manifest, not the baseline.**
- **Where:** `target/development-speed-255/final-pair.py`, whose `finally` block writes the candidate manifest.
- **Impact:** It is a local, unshipped one-off, and the current `Cargo.toml` hash matches main. It does affect reproducibility, so mention it in the reproduction notes if those scripts are referenced.

## No-change decision
Not retaining a default configuration is defensible. Execution qualification and full-suite identity were required gates, and the finite budget ended. The problem is the stated reason.

As written, the documents present measurement noise as a negative result for Candidate 1. Future cycles could then dismiss a lever that has real compile-time gains: roughly 19 s faster per AI edit and about 80–110 s faster cold builds. The cost is 54% larger test executables.

I am not asking for new performance work. If the owner chooses, an ABBA re-run of the preserved artifacts would be the cheapest way to resolve it.

## Security assessment
No issues found.
- **Changed files:** Documentation, JSON and JSONL only. Shipped runtime prompts under `agent/`, CI and code are unchanged.
- **Secrets and private paths:** A scan of the added lines found no credentials, tokens or private absolute paths. Home paths are rendered as `${USER_HOME}`.
- **Data exposure:** Fixtures are synthetic, with a temp directory outside Git. Raw logs stay local in `target/`.
- **External actions:** None in the diff. No live provider calls, no billing or security setting changes, no check bypass, no release.
- **Authority:** The guard script only signals its own process group and does not grant any authority. The evidence makes no unearned ownership or acceptance claims.
- **Pending gates:** Hosted CI, Codex review and security review are correctly recorded as pending, not passed.

## Score: 5/10
- **For:** The scope is clean, the bookkeeping is honest about failures, provenance and identity checks are excellent, and the setup and freshness guidance is useful.
- **Against:** The main conclusion for Candidate 1 rests on a confounded comparison that the PR's own evidence refutes, and Candidate 2's conclusion inherits the same confound.

The evidence is incomplete for any execution claim. It is adequate for the compile-time claims, which hold on this host and source only.



# Round 3 — corrected evidence recheck

Route: acpRegistry_copilot / claude-opus-5.5
Options: [{"id":"reasoning_effort","value":"high"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3A0d444406-0a49-4f34-8b58-7d6d083289dd%3Adelegate-task%3A255-recheck-opus-3>

**Score: 8/10.** The correction addresses Opus's material objection. It can publish as a no-change, incomplete-qualification result without claiming a speedup or harm.

**What I checked (read-only, no builds or tests):**
- `origin/main` is `7ae7ebf1`. Outside `docs/`, nothing differs from main, so product code, config, CI and lockfiles are byte-identical.
- I hashed the retained binaries myself. `brn` (`83f19589…`), `brn_ai` (`9c1edc4e…`) and `agent_journeys` (`c9b3a0df…`) match between `target/255-candidate-qualified` and `target/255-candidate-final`. `agent_contract` differs, as the docs now say.
- Observations lines 73–75 confirm the earlier 69.90/67.11/67.08 s runs, against 148.17/91.69/91.79 s later.
- No remaining claim that opt0 harms execution or that CGU512 causes slower compilation. Raw numbers are kept and labelled as non-causal. The new guidance covers ABBA alternation, same-binary hash checks across slots, the limits of a Cargo-only guard, and the restore gap in the script's `finally`.

**Remaining issues (none block publication):**
1. **Low, wording:** `development-speed-255.md` lines 11 and 92, and results.json line 394, say "degraded-host window". That states the cause as fact, the reverse of the claims just withdrawn. Fix: say "later slow window, cause unknown".
2. **Low, reproducibility:** `methodological_limits.binary_sha256` doesn't say which artifact roots were compared, and candidate-runtime has no root mapping anywhere in the JSON. Fix: add `target/255-candidate-qualified` and `target/255-candidate-final`.
3. **Low, accuracy:** the compilation gains are also from sequential, non-ABBA runs. "Repeatedly observed" is fine, but add "not ABBA-qualified". Nothing is retained, so this is not material.
4. **Info:** older `record.json` decision entries (lines 162–170) still say "uncontended", "+37%" and "Reject". The appended entry at line 177 supersedes them; optionally mark them "superseded".
5. **Nit:** missing spaces in the `.md` ("is102.31→108.24s", "Candidate1", "configure512CGU", "Round2: Sol9/10").

**Security:** No issues. The change is docs and evidence only, logs use `${REPO}` instead of user paths, no credentials or user data, and no new authority or tool access.

**Limitations of this review:** I didn't re-check every observation row or the timestamps between slots, and I didn't read the round-2 council files in full. Current-head hosted CI, Codex review and security review are still pending as separate gates.

