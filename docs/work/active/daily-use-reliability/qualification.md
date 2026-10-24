# Daily-use batch qualification

Status: focused and independent gates passed as recorded below; local full deterministic provider-fixture qualification is blocked by the cloud fixture environment. Draft [PR226](https://github.com/ewq100/brn-rust/pull/226), not merged. Production source under test: `aa5d53cea4b88586a64480232b240e74d173d665`, based on main `f30f37e0dc50fdcf91e97a77bc7cda9acb678d78`. Later test-only fixes enable the existing Linux document witness and prevent three fixture barriers from hanging after early worker termination; CI also enables the existing Linux document/CLI gates. Runtime production behavior is unchanged by those follow-ups. The frozen test/CI correction head is `3393d4e010f673f78f3c9b4eb2bfd98d8e8e898e`; subsequent report/evidence edits are documentation-only.

## Delivered behavior

- Existing Stock/BRN Xberg routes retain bounded native PNG and JPEG figures, including reference-style images. Source bytes/hashes, provenance and content-addressed assets are preserved. GIF/WebP/SVG and noncanonical references remain explicit gaps; no remote image fetching was added.
- The `figures-v1` identity separates new figure-capable conversions from historical imports without rewriting their provenance. Current-profile duplicate/replay handling remains checked.
- Existing daily workflow evaluation gains 12 authored semantic scenarios and 8 composed scripted service journeys. Correctness, grounding, safety and recovery remain separate from cost. Scripted results and marker checks are not real-model semantic qualification.
- Linux verification now includes the existing scripted document route, including its formerly Mac-only test module and CI step. Three async test fixtures fail clearly if their worker ends before signaling the expected barrier, preserving their successful-path assertions. Documentation is reconciled with code; the [single decision backlog](decision-backlog.md) preserves blank owner choices and links its curated source map.
- The [calculator/chart/report proposal](artifact-proposal.md) is design-only. No worker or general execution capability is implemented or enabled.

## Completed focused checks

| Evidence | Result | Limit |
|---|---|---|
| Stock and BRN helpers, frozen source `2c33838d609cf76c55d8cd253d3ba12bada6d483` | 19 active tests each, strict all-target Clippy and formatting pass | Each has one intentionally ignored child entry point exercised by its parent test |
| Independent final Stock PDF raster oracle | Exact 2×2 red/green/blue/white pixels pass using Pillow | Synthetic fixture, not broad PDF fidelity |
| Shared intake validation on integrated source | 8 tests pass | Hostile header/hash/quotas and bounded decoder behavior |
| Figure-profile reimport on final test head | 1 test passes | Historical note/provenance preserved; current variant deduplicated |
| Source tools on integrated source | 2 tests pass | JPEG/PNG provenance, retention and replay; no live provider |
| Existing daily workflow test binary | 9 tests pass, including 8 new composed journeys | Scripted model fixtures; fresh service reopen is not a process-crash or native UI test |
| Wiki seeder | 6 tests pass, including all 12 new authored inputs | Validates fixture admission and seed state, not model answers |
| Integrated Python tooling suite | 199 run: 198 pass, 1 Apple Silicon packaging skip | Linux cloud environment |
| Default workspace format/build and strict all-target Clippy | Pass | Linux cloud, default features |
| Final all-target Clippy with `brn/scripted-document-tests` | Pass | Includes Linux document witness and repaired barrier bodies |
| Independent core/intake/retrieval/writing crates | 307 tests pass; 1 subprocess-only child entry point ignored and exercised by its parent | Explicit subset, not a substitute for the full provider suite |
| Existing offline end-to-end gate | Pass: 108 CLI journey steps, 1,266 checks, plus Save/recovery/search/export/backup/startup/refusal fixtures | Headless, no provider or native UI claim |
| Independent source/evidence review | No material blocker in intake, workflow, backlog, later test and CI fixes | Exact final-head CI remains separate |

Helper artifacts were rebuilt and SHA256-checked. Integration compares their production source bytes to the frozen qualification commit before reuse; it does not infer executable identity from timestamps. The final helper hashes are recorded in the [artifact receipt](evidence/intake-helper-artifacts.json), with a [source manifest](evidence/intake-helper-source-manifest.json) and [independent pixel result](evidence/intake-independent-pdf-pixels.json).

## Environment and failed-attempt accounting

Pinned Rust 1.98.1, locked/offline Cargo graphs, Linux x86_64 cloud environment; heavyweight builds serialized. Both qualified helper binaries are supplied explicitly and copied beside the CLI for process tests that deliberately clear environment overrides. No Mac access occurred.

The container's PID1 does not reap killed grandchildren. The first Python run caught the resulting zombie in a process-lifecycle check; an existing process-local subreaper wrapper then ran the unchanged suite successfully. This changes the test host's child-reaping behavior, not application security or assertions. Original failed output is retained.

The first offline workflow build found a missing cached dependency; the unchanged locked dependencies were fetched before subsequent offline runs. Workflow fixture checks caught an empty-topic fixture admission mismatch and an incorrect bare-body expectation (the seeder intentionally includes the note title). These were corrected and rerun. An intermediate helper identity build race was caught, rebuilt at the frozen source, and requalified. Original failed attempts remain evidence, not successful checks.

The first integrated source-tool command used a module filename as its filter and ran zero tests. A mistyped overview test-discovery filename similarly ran zero tests; its corrected filename ran all 16 overview tests successfully. It is excluded from proof. The corrected `cargo test -p brn-ai --lib useful_intake_model_tools --locked --offline` ran both intended tests successfully.

Normal supported Codex app-server initialization with task-local SQLite state exited 1 with a read-only-filesystem error before model admission. No credentials or security settings were changed. The [admission receipt](evidence/live-admission.json) records the failure. Model calls: **0**; tokens, cost and remaining allowance: **unavailable**, not estimated. No live semantic claim or repeat of PR225's narrow qualification is made.

## Full-suite environment blocker and CI

The full local workspace test run exposed a distinct cloud limitation: the sandbox presents a read-only `.git` marker at every granted writable root. BRN's existing `Auth::validate_ancestors` correctly refuses credential storage beneath any Git ancestor, including the synthetic caches in these tests. The diagnostic `stub_ask_completes` fails with `UnsafeCredentials` before contacting its loopback stub. A narrowly granted fresh `/var/tmp` fixture root received the same marker and reproduced the refusal; no markers or authentication guards were removed, and no unsandboxed retry was attempted.

The original no-fail-fast workspace run recorded failures and then blocked in three fixtures awaiting middleware events that admission could never reach. Its owned Cargo/test processes were terminated. It is **not a completed or passing full-suite run**. The three test-only repairs detect early worker completion and retain the previous successful-path assertions. [Failure-path diagnostics](evidence/barrier-diagnostics.json) confirmed all three now return explicit `UnsafeCredentials` failures in 0.047–0.087 seconds rather than hanging. These are expected failing diagnostics, not passing behavioral tests; success paths still require compatible final-head CI. The [environment receipt](evidence/environment-blocker.json) records the attempted actions and outcome.

The newly enabled Linux scripted-document module compiles and runs one test (not zero), but its synthetic auth file is subject to the same refusal. This result is blocked qualification, not a passing provider test. Independent non-provider checks continue separately; no broad skip is represented as full acceptance.

Normal GitHub CI at `aa5d53ce` passed Linux core/CLI, Linux native build/state, Mac core/CLI and documentation. Linux core included the default workspace tests, helper qualification and provider capability fixtures. The [Linux core job receipt](evidence/ci-aa5-linux-core.json) binds those results to that exact head. That run skipped the then-Mac-only scripted-document and offline-E2E steps. The final CI change enables both on supported Linux; **earlier success does not qualify these new steps**. Final-head CI is required before considering merge. Optional Windows core failed during workspace build; detailed log retrieval returned HTTP403, so its cause is unverified and not assumed identical to a baseline failure.

## Remaining acceptance and authority

Actual native interaction and final-head CI are separate from the completed local checks. Actual Mac focus, rendering, accessibility, file-picker, review/Apply/Undo and owner acceptance remain the [single later checklist](baseline.md#later-consolidated-native-acceptance).

JPEG validation establishes bounded decoder acceptance, not an entropy-integrity certificate: maintained decoders can recover some truncated entropy streams with a forged end marker. The regression records that limitation. SDK image settings/post-extraction quotas are not a universal preallocation cap; existing Linux process limits apply, and hard Mac memory containment is not claimed.

Calculator implementation/activation awaits its separate decision. This batch has no merge, release or deployment authority. Optional Windows implementation remains out of scope.

## Integrated slices

All remain in draft PR226; none is merged by this batch.

| Commit | Slice |
| --- | --- |
| [`e78f3ff4`](https://github.com/ewq100/brn-rust/commit/e78f3ff45f20f9dd3c0a0072629b0132b0bb8854) | Bounded PNG/JPEG intake, figures-v1 provenance and fixtures |
| [`fffc3414`](https://github.com/ewq100/brn-rust/commit/fffc34141789ced004528e9c6b8a3758e08aa79f) | Composed daily workflows and real seeder qualification |
| [`ef9c14ad`](https://github.com/ewq100/brn-rust/commit/ef9c14ade4cb65861a2e7c52efbe939b330a2d6b) | Evaluation dimensions and 12 authored semantic cases |
| [`2d81fb04`](https://github.com/ewq100/brn-rust/commit/2d81fb04d3387b6bb15c9b00106e72a55f827f9d) | Canonical chat-source/backlog reconciliation |
| [`97d1ea28`](https://github.com/ewq100/brn-rust/commit/97d1ea28064e2ca5ee9601c4b3f7439701edddd5) | Design-only constrained artifact proposal |
| [`f0c5cf99`](https://github.com/ewq100/brn-rust/commit/f0c5cf99a94cdcc19f4ab8befd4349f41aa1128e) | Stable decision-entry navigation |
| [`c0a86f90`](https://github.com/ewq100/brn-rust/commit/c0a86f90ad1dc4fa20b77c31ed638714b26e8e2b) | Actual Linux scripted-document test module |
| [`42e4f4f2`](https://github.com/ewq100/brn-rust/commit/42e4f4f2d5667da59cef05912801c784a0806214) | Fail-fast fixture barriers |
| [`3393d4e0`](https://github.com/ewq100/brn-rust/commit/3393d4e010f673f78f3c9b4eb2bfd98d8e8e898e) | Linux CI document and offline-CLI gates |
