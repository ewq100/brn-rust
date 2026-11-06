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

