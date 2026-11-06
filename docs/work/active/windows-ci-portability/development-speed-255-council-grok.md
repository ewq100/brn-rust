# Council diagnosis: grok

Route: acpRegistry_copilot / grok-4.7
Options: [{"id":"reasoning_effort","value":"xhigh"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3Aaa1436f8-ba63-41e4-a3be-f32116248a11%3Adelegate-task%3A255-diagnosis-grok-1>
Score: no score (diagnosis)

Read-only diagnosis for #255. No speedup is claimed. No second or third implementation candidate is justified.

## Bottleneck

The cold workspace test compile ends when one rustc unit ends. `brn_ai "lib" (test)` (`scripted-runs`) runs from 128.57 s to 245.29 s, 116.72 s of the 245.455 s wall. `aws-lc-sys` (69.95 s, ends 84.60) and `rig-core` (38.65 s, ends 128.58) are only the prefix. Jobs=4 cannot hide that unit.

Incremental workspace `--no-run` is the same graph, not test execution. The no-op rebuilds nothing (median 0.407 s). A UI edit rebuilds one artifact (median 1.259 s). An AI edit rebuilds 86 artifacts (median 59.840 s, range 38.535–66.612). A core edit rebuilds 115, including `brn-ai` (median 33.104 s). Targeted execution is 0.162–0.692 s. The AI-edit range is 28 s wide, so a small win is noise.

## Opt-level narrowing: reject on this evidence

Rank it alone and do not implement it yet. The 116.72 s versus 29.01 s gap is confounded. The normal library at opt-level 0 is 3.96 s frontend and 25.05 s codegen. The test unit has no section split. It also compiles the `#[cfg(test)]` majority of about 60k lines, `cfg(any(test, capability-spike))` modules, and dev-dependencies. `opt-level = 1` is only one of those factors. Package-scoped opt-level 0 is the only reversible knob on that unit that keeps assertions, overflow checks, the `sha2` opt-level 3 override, features, and cache identity. It is not a demonstrated win.

Spend a slot only after this gate: one warm AI-edit and one warm core-edit with the same probe and `--timings`, jobs=4, incremental=1, locked, offline. Reject without editing the profile if the `brn-ai` test unit is not the majority of that incremental wall, or if a clean timing shows codegen under 40% of that unit. If sections are still missing, do not guess.

If that gate passes, the only change is:

```toml
[profile.test.package.brn-ai]
opt-level = 0
```

Do not change the global test profile, `sha2`, debug assertions, overflow checks, features, CI cache keys, `CARGO_INCREMENTAL`, helper jobs, or `scripts/ci-workspace-tests.py`.

Accept only if all of these hold, on the same machine protocol, three warm repeats, one clean control:

- The clean `brn_ai "lib" (test)` duration drops by at least 30 s from 116.72 s, and the wall drops by at least that unit delta minus 5 s. One cold sample cannot prove a smaller effect.
- The AI-edit maximum is below the baseline minimum, 38.535 s. An overlapping range is a reject, not a partial keep.
- The core-edit median stays at or below 33.636 s and the no-op median stays at or below 0.786 s. The UI median must not exceed 14.097 s.
- Helpers are qualified before any runtime comparison. Build both helpers, export `BRN_XBERG_HELPER` and `BRN_XBERG_HELPER_BRN`, and copy them beside the test binary as `docs/development/verification.md` requires. Then the `brn-ai` lib suite must show no new failures and wall time at most 37.6 s (3 times 12.535 s). The three probe filters must still pass. If helpers are still unqualified, revert. Do not keep an execution-unchecked profile change.
- The diff stays that profile block. Do not claim a CI speedup from it.

The risk is slower `brn-ai` test execution. Other crates stay at test opt-level 1, which exists for recovery JSON/base64 loops. If the AI-edit bar fails, the incremental wall is the 86-artifact fan-out, not this knob. Revert. Do not add a second factor.

## Maintainability

Defer a crate or test split. The repeated cost is one `cfg(test)` rustc unit plus metadata fan-out from `pub(crate)` async code in `codex_auth.rs`. That is not a demonstrated ownership or correctness failure. A new crate would add build cost and cannot see private tests without an API change. No architecture campaign in this cycle.

## Concrete issues

| Severity | Location | Evidence | Proposed fix |
| --- | --- | --- | --- |
| High, feedback | `brn_ai "lib" (test)` in `baseline-units.json` | Sole cold-wall end, 116.72 s; AI-edit rebuilds 86 artifacts | Measure the split above. No code change until the gate passes. |
| Medium, invalid runtime baseline | `baseline-ai-suite-1.log` | 595 passed, 3 failed, 2 ignored, 11.49 s tests, wall 12.535 s, exit 101. Failures are `document_refine` `Unavailable` / `Intake(Unavailable)` at `document_refine_tests.rs:147`, `:187`, and `:424`. `helper-setup.log` stops mid-compile. | Preserve this failure. Qualify helpers before any runtime comparison. |
| Rejected leads | `.github/workflows/ci.yml`, `setup-rust/action.yml`, `scripts/build-document-helpers.sh` | Historical Mac run 42009674068: workspace-test 447 s, helper prepare 215 s, helper API/lint 224 s, capability fixtures 142 s, job about 1227 s. Frozen run cancelled. | Not causal. Do not change them in this cycle. |

## Do not spend the other slots

- Global test opt-level 0. That is broader than the unit and fights the recovery-loop reason for opt-level 1.
- Rig, rustls, or aws-lc changes. Those are forbidden runtime changes, and aws-lc is off the incremental path.
- `CARGO_INCREMENTAL=1` in CI, or `cache-workspace-crates: true`. Clean CI builds do not restore workspace crates. Cache identities stay.
- Helper jobs above 1. The helper script accepts only jobs=1.
- Parallel helper and workspace steps, dropping `cargo build`, nextest, or sccache. These are either unmeasurable here or they change failure propagation. `ci-workspace-tests.py` must keep streaming output and non-zero exit.
- Removing `scripted-runs` from host manifests. Both the 29 s and 117 s units already enable it, so it does not explain the gap. Feature identity stays.
- A package-filtered inner loop as a substitute for workspace `--no-run`. An edit in `brn-ai` still dirties the downstream test graph.

Leave the PR258 macOS updater step untouched. Historical CI only says the same test profile sits on the old Mac critical path. It is not a before/after result.



# Round 2 — final review

Route: acpRegistry_copilot / grok-4.7
Options: [{"id":"reasoning_effort","value":"xhigh"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3A0d444406-0a49-4f34-8b58-7d6d083289dd%3Adelegate-task%3A255-final-grok-2>

Round 2 review, Grok 4.7. Read-only. HEAD `2283b8d8` versus `origin/main` `7ae7ebf1`. Diff is docs and task routing only. Product crates, `Cargo.toml`, `Cargo.lock`, `.github`, and `agent/` match main. Working tree is clean. I did not rebuild, retest, or treat pending CI, Codex, or security review as passed.

## Score: 8/10

The no-change decision is justified by a valid experiment, not by a failed protocol. Headline numbers match the raw final-pair logs, suite output, artifact profiles, lock hash, and Rig 0.45 fingerprints. Two low documentation nits keep this from 9. They do not reopen measurement.

## Findings

**Low — outcome sentence misstates the no-op result.** `docs/work/active/windows-ci-portability/development-speed-255.md`, Candidate 1 close. It says no no-op/core/UI speedup is inferred from small or overlapping results. Core (`17.96–21.17` vs `17.91–20.62`) and UI overlap. No-op does not: `0.905–1.076` vs `0.299–0.310`. Say no-op compilation was faster, core/UI overlap, and none of that is a retained default because full-suite feedback regressed.

**Low — codegen flag observation is not in the preserved logs.** `development-speed-255-results.json` `flag_evidence`, and the same sentence in the task doc. I found no saved `rustc` argv containing `-C codegen-units=512`. What is preserved is `Cargo.candidate-two.toml`, `candidate-two.py`, opt-level still 1 with assertions and overflow checks, and a changed `brn_ai` fingerprint (`fa4d4d0228d2a525` vs baseline `bbfa16938722e9ba`). Reword to that. Do not rerun.

**Low — task routing is one minute behind HEAD.** `record.json` / generated `docs/work/overview.md`, observed `16:26:54Z`, still say the next action starts with reconciling main. `2283b8d8` at `16:28:01Z` already contains `origin/main`. Update the record and regenerate the overview: main is reconciled; remaining work is this review and normal PR follow-through. Do not mark hosted CI, Codex, or security review passed.

## Decision

Rejecting both candidates is correct. Opt0 makes AI compilation faster (`35.298` to `16.449` s, no range overlap) and the single cold control faster (`214.283` to `133.402` s), but AI suite execution is slower (`10.113` to `12.094` s, no overlap) and the stable workspace repeats are slower (`66.37/67.01` vs `91.69/91.79`). The disclosed median sum, `102.31` to `108.24` s, is about 5.8% slower. The record’s “+6%” is rounding; the task doc has 5.8%. That is enough to refuse a general default.

Codegen-512 is also a valid early reject: cold `263.028` s, all three core edits `28.691/32.987/30.379` s versus baseline max `21.17`, first auth edit `56.157` s. Completed second auth edit `54.851` s is marked excluded and is also slower. The interrupted restore (`exit -2`) is the partial sample. No contradictory win was dropped.

This is a missing improvement, not an invalid experiment. Contaminated trials were preserved and kept out of the headline pair. Do not start another candidate, crate split, CI change, or architecture survey.

## Consistency

Final-pair commands, jobs=4, incremental=1, and `competing: []` match the protocol. Each arm: workspace `1491/0/9` summed from the logs; AI suite `598/0/2`; probes 5, 1, and 20, all nonzero. Stale sets match across all six repeats: core 115, AI 86, UI 1. Cold logs have 383 artifact messages and 378 unique identities. Only `brn_ai` normal/test opt-level changes, 1 to 0; debug assertions and overflow checks stay on. Test executable `46151312` to `70966688` bytes. Frozen lock SHA-256 matches `e210c2c0`. Both cold logs are `rig-core`/`rig` 0.45.0. Current main lock is 0.46.0 and is not used as a performance claim. Probe sources and committed `Cargo.toml` are restored. CI still sets `CARGO_INCREMENTAL: '0'`.

## Guidance

`docs/development/verification.md` is the useful deliverable. It records the failures that actually happened: restore only on byte change, fresh no-op before the next group, compare artifact sets, separate compile from execution, qualify both helpers and stage them beside the CLI, keep incremental-on local times distinct from CI, and preserve failed attempts. That is enough. No shipped benchmark runner is needed.

## Objections

Workspace full ranges overlap only because the first-run outliers were retained (`102.94` and `148.17`). The doc says so. Stable repeats and the AI suite do not overlap. Do not rerun for a combined edit-plus-suite sample or a second cold pair. Non-AI cold units also moved (`aws-lc-sys` `63.96` to `48.98`, `rig-core` `38.7` to `28.36`), which is why one cold pair cannot be a universal claim. The doc already limits it that way.

## Security

No finding. Changed files do not touch the checked writer, credentials, provider transport, MCP dispatch, helper restrictions, or `agent/` prompts. Record limits still forbid production data, live provider calls, billing or security changes, check bypass, force-push, and release. Committed observations use `${REPO}` and contain no home paths, tokens, or credential material. Local logs stay untracked. The auth probe was an operand swap, tests passed, and the source was restored. No new authority or external action.



# Round 3 — corrected evidence recheck

Route: acpRegistry_copilot / grok-4.7
Options: [{"id":"reasoning_effort","value":"xhigh"}]
Thread: <t3-thread://v2/18810396-d052-461d-8c90-3993aed1f189/thread:delegated-task:command%3Amcp%3A0d444406-0a49-4f34-8b58-7d6d083289dd%3Adelegate-task%3A255-recheck-grok-3>

**9/10. Publish the no-change / incomplete-qualification result. Do not publish a speedup, an execution-harm finding, or a causal codegen-units regression.**

HEAD `ac00b931` versus `7ae7ebf1` is docs, evidence, and routing only. Product, manifests, lockfile, CI, and `agent/` match main. The working tree is clean. Probe sources and `Cargo.toml` match the restored baseline, not the trial profiles.

## Material objections

Opus’s high finding is valid and corrected. I re-hashed the retained candidate binaries. The CLI `83f19589…`, the AI test `91edc4e…`, and the journey binary `c9b3a0df…` are identical across the earlier and later slots. The earlier workspace runs were 69.90 / 67.11 / 67.08 s, all 1491/0/9. The later runs were 148.17 / 91.69 / 91.79 s. The same important binaries were near baseline earlier and slower later, so the sequential pair cannot attribute execution to opt-level 0. `agent_contract-8eef83669256bae3` differs (`9638fc81…` versus `fddd6a09…`), so Opus’s “every test binary” claim was too broad. That does not restore a causal slowdown.

The current outcome, gates, overview, and `results.json` withdraw both causal claims, keep the raw numbers, and state that execution is unresolved. Compilation stays an observation only: the AI edit is 35.30 s to 16.45 s, with two clean controls, and no net default is qualified inside 12:26–16:26 UTC. Codegen-units 512 remains an unpaired slower arm with no gain and is not retained. The no-op ranges are disjoint, and the text correctly refuses a configuration-speed claim.

Guidance now requires ABBA, same-artifact hashes, and states that a Cargo/compiler guard does not prove the host was quiet. `final-pair.py` restores the candidate manifest in `finally`; the reproduction note says to restore and verify afterward. The compiler argv was not saved. `Cargo.candidate-two.toml` and `candidate-two.py` set 512 units, and the AI artifact id differs (`fa4d4d02…` versus `bbfa1693…`). Main is reconciled. Hosted CI, Codex, and security review stay pending.

## Remaining issues

No high or medium issue remains. No further timing is required.

- **Low, non-blocking:** `record.json` attempts 4–5 still say +37% / +6% and “reject early.” Attempt 6 and the current gates supersede them. Do not cite 4–5 as the conclusion.
- **Low, non-blocking:** The outcome lede does not name Rig. The frozen lock `39366eba…` is e210 / Rig 0.45. HEAD is Rig 0.46. No current-main speed claim is made.
- **Low, non-blocking:** The differing `agent_contract` hashes are verified locally but not copied into `results.json`. Clean-wall deltas also include non-AI units, and the doc already says that one cold pair is not universal proof.

## Security

No issue. The diff does not touch the checked writer, credentials, provider transport, MCP dispatch, helper restrictions, or shipped prompts. Committed evidence uses `${USER_HOME}` and `${REPO}`. No tokens or private payloads. Local logs stay under `target/`. No new authority, external action, or check bypass. Pending gates are not marked passed.

## Limits of this pass

I did not rebuild, retest, retime, or run docs, CI, Codex, or security review. Identity checks used existing local binaries, which are not in the PR. Preserved round-2 reports still contain the withdrawn causal wording. They are reviews of `2283b8d8`, not the current conclusion.

