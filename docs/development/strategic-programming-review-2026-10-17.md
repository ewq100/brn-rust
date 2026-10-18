# BRN strategic programming review — 17 October 2026

## Assessment and scope

BRN has a credible shared integrity core, meaningful dependency reuse and established verification. Its main maintainability pressure is in host orchestration and UI state ownership. Stronger engineering rules exist in individual work packs, but are not consistently present in the permanent agent entry point. Stale status/assignment summaries compound the problem.

**Reviewed baseline:** public repository `ewq100/brn-rust`, main [`8f768943964a77711b26d5235df146f8efe1dc6c`](https://github.com/ewq100/brn-rust/commit/8f768943964a77711b26d5235df146f8efe1dc6c), merged PR #157. Review used the source, architecture/behavior/invariant documents, current plans, recent PRs and the main CI logs. Separate reviewers examined architecture and development/quality rules. This is a targeted static assessment, not a complete code audit or a fresh product qualification. No Rust builds, provider runs or native interaction were performed for this documentation change.

Recommendation: apply the existing modular-foundation/CW and Rig-first sequence with clearer design exit criteria. Invest in the shared boundaries that make the next feature easier. The evidence does not justify another wholesale rewrite, a universal state-machine framework or a new development platform.

## Immediate reliability finding

At the reviewed main commit, [CI run 38908335265](https://github.com/ewq100/brn-rust/actions/runs/38908335265) has **three of four required contexts passing**. The required [Mac native UI job](https://github.com/ewq100/brn-rust/actions/runs/38908335265/job/116997504049) built and linted successfully and passed its native-state step, then failed its widget step:

- Test: `threads_ui::writing_tests::an_edit_already_open_elsewhere_offers_compare_and_discard`.
- Assertion at `crates/brn-desktop/src/threads_writing_tests.rs:846`: `edit_open` was still `Some` after simulated Discard confirmation; expected `None`.
- Widget result: 305 passed, 1 failed, 1 ignored; the later headless startup step was skipped.

The [round-3 evidence](../work/active/cli-agent-contract/evidence.md#round-3) records earlier intermittent failures for this test and `a_saved_working_rewrite_offers_the_same_confirmed_stop_in_the_note`, on both baseline and candidate under parallel execution. Those dated repetitions identify a reliability problem; they do not establish its cause or a repair. Optional platform failures are separate and do not explain away the required Mac failure.

Both tests click the shared `consequence_dialog` confirmation, then expect an immediate checked state change. Inspect [the dialog callback](../../crates/brn-desktop/src/threads_components.rs), [writing confirmation](../../crates/brn-desktop/src/threads_writing.rs), [Stop confirmation](../../crates/brn-desktop/src/threads_chat.rs) and their weak-view update results. The test fixtures use separate temporary stores and disable normal polling. A useful first hypothesis is event delivery, readiness or callback execution under parallel UI tests; no toolkit or application root cause has been proved.

**Bounded follow-up:** trace a single attempted click through handler entry, view update, checked store outcome and resulting state. Fix the demonstrated cause using supported deterministic test controls or the specific application transition. Retain the click-through test and all assertions about saved text, preserved local typing, distinct sessions and Stop/queue outcomes. Compare baseline/candidate under the same original parallel conditions for a predetermined bounded set of attempts, retaining every failure; then run the affected tests and required native lane. An unchanged rerun, extra sleep, removed assertion or blanket serialization is not evidence of a repair. A proven test-runtime restriction can justify narrow isolation with an explanation.

## What is already working strategically

| Strength | Evidence at the reviewed baseline | Preserve |
|---|---|---|
| Canonical state and checked mutations | [Core Store](../../crates/brn-threads-core/src/store.rs) keeps the SQLite connection crate-private and applies grouped changes transactionally; [invariants](../architecture/invariants.md) bind authority, versions, replay and Undo. | UI/CLI/runtime must continue through checked operations. A public `Workspace.store` is broad coupling, not evidence that external code can issue raw canonical SQL. |
| Real reuse | [AI manifest](../../crates/brn-ai/Cargo.toml) pins Rig 0.44.0; [intake manifest](../../crates/brn-intake/Cargo.toml) includes BetterOffice, mail-parser, HTML and spreadsheet facilities. | Preserve useful integrations and compare total adapter cost before replacing them. |
| Existing ownership improvements | [ChatCommand](../../crates/brn-threads-core/src/chat.rs), `ContinuePath` in [UI state](../../crates/brn-desktop/src/threads_state.rs), the runtime tool catalogue and shared Markdown rendering policy. | Extend these demonstrated patterns where they reduce repeated knowledge. |
| Meaningful quality evidence | [Quality evidence](../work/active/quality-simplify/evidence.md): the rendering pilot closed three meaningful gaps; the later formatting batch closed fourteen, including two found by independent challenge of equivalent-mutant classifications. | Test actual outcomes and independently review difficult survivor classifications. Counts alone are insufficient. |
| Executable retirement checks | [Provider](../../scripts/check-provider-retirement.py) and [intake](../../scripts/check-intake-retirement.py) checks run in [CI](../../.github/workflows/ci.yml). | Extend this low-cost pattern when a new boundary or cutover needs protection. |

## Main design findings

### 1. Run-start policy still leaks into multiple hosts

`ThreadsUi::start_run` in [threads_ui.rs](../../crates/brn-desktop/src/threads_ui.rs) and CLI `ask`/`provider`/`effort` in [main.rs](../../crates/brn/src/main.rs) separately resolve settings, obtain search, choose credential paths and assemble `ThreadRunRequest`. They already converge on the same `prepare_run` in [the runtime](../../crates/brn-ai/src/threads.rs). This is demonstrated duplication above an existing shared integrity mechanism, not two unrelated runtime implementations.

**Next change:** CW2 should provide one shared request/start operation built on `prepare_run`. The hosts supply intent, scope, valid overrides and event handling; the shared owner resolves common policy. Keep GPUI rendering and CLI signals in their respective hosts. Delete superseded setup paths in the replacing slice.

**Completion evidence:** desktop, CLI and the proposed development host use the same policy; scripted run/invalid-setting/scope/replay cases preserve behavior. Adding a supported request option should not require copying its meaning into each host.

### 2. The writing component needs owned state, beyond separate files

[threads_writing.rs](../../crates/brn-desktop/src/threads_writing.rs) and [threads_document.rs](https://github.com/ewq100/brn-rust/blob/57c710ccec2b007c4d8a25c9f1d1b5c1c539907c/crates/brn-desktop/src/threads_document.rs) extend the same `ThreadsUi`; they are not independent components. The parent owns editor/review state, search, project selection, drafts, workers and notices together. `open_note` also reconstructs writing context from receipt history and falls back to the first linked thread; `ensure_writing_thread` partly owns the selection policy in the UI.

**Next change:** use CW1/CW2 for explicit scoped writing-context resolution and B1a/B1b for a panel with owned state and small typed host requests. Preserve the planned serial ownership of shared interfaces. Extracting the file into a new crate while retaining full-desktop dependencies would not achieve the intended boundary.

**Completion evidence:** B4 can use the real writing component and shared `Workspace` without initializing the full desktop navigation or copying its service. Relevant existing acceptance cases include F04, F18 and F21 in the [foundation plan](../work/active/modular-foundation/proposal/final/01-master-plan.md). Remove the arbitrary first-linked-chat fallback when the shared resolution replaces it.

### 3. Some UI invariants are maintained by compensating fields

`ThreadsUi` holds `notice`, `notice_tone` and duplicate `notice_tone_for` text. Its `set_notice` and `notice_tone` methods compare the strings so a direct notice assignment cannot retain an old tone. This may behave correctly; it nevertheless makes field writers responsible for coordination.

**Next change:** while touching this state, consider one `Notice { text, tone }` value with controlled updates. For a lifecycle with several coordinated fields, consider a small Rust enum and a pure transition function that returns effects. Decide from the actual state relationships. Do not remodel every toggle or create an application-wide reducer.

**Completion evidence:** invalid combinations cannot be constructed through the normal interface, important transitions have contract-based tests, and integration tests still show that the actual widget event reaches the operation. State-machine tests cannot establish pointer hit-testing, focus, geometry or OS behavior.

### 4. Refresh and runtime preparation are investigation hotspots

`ThreadsUi::refresh` loads records/receipts and rebuilds view data and asset images; `show_tile` invokes it. The runtime `begin` combines context/history construction with durable run setup and authority checks. The source demonstrates broad responsibilities. This review measured neither navigation slowdown nor the cost of changing these functions, so it does not establish a performance regression.

**Next change:** C0 should record refresh/navigation as a measurement candidate. Before an optimization is assigned, measure it with representative history/assets in the applicable environment; this is not a new C0 benchmarking or native-interaction gate. During CW2/D1A, give context construction/budget selection an internal owner while preserving the existing durable run and authority contract. Reuse the planned Rig proofs. Remove the fixed-history route when its replacement lands. Do not split large files solely to lower their line counts; the runtime file also contains a substantial inline test module.

### 5. The strongest rules are unevenly discoverable

Before this update, permanent workflow guidance already required reuse, safe boundaries, proportionate review and selective checks. More specific rules about deep modules, replacement/removal and touched-test quality were mostly in the proposed foundation/build-sequence packs. Agents could follow the root guide without encountering them.

The current-status source also described #155–157 as unmerged, while Git showed all three merged. The index and architecture overview still routed new work to pre-release cleanup. These are concrete documentation defects, not evidence that any particular agent actually repeated finished work.

**This documentation update:** promotes a short standing strategic policy, links its details, removes stale cleanup authority from generic routing, reconciles the current baseline/CI state, and labels old receipts as historical. It keeps the original reviewed packs and old test results intact.

## Recommended implementation order

These are recommendations mapped onto existing work. They do not grant implementation or merge authority.

| Priority | Work | Where it fits | Evidence for completion |
|---|---|---|---|
| 1 | Diagnose/fix confirmation-path test reliability | A bounded desktop follow-up, before relying on a green baseline | Original parallel conditions, retained click-through assertions and required Mac lane. |
| 2 | Establish current baseline, owners, relevant test times and supported-data retirement evidence | Combined C0 and time-boxed CL | Accurate baseline; justified removals; no supported recovery loss. |
| 3 | Consolidate common run setup and scoped writing-context policy | CW1/CW2 | Multiple hosts consume one policy; old paths removed. |
| 4 | Extract writing state/operations and use explicit transitions where needed | B1a/B1b/B4 | A real independent host, focused transition tests and native interaction evidence. |
| 5 | Make the new boundaries executable | Alongside the boundary-changing slice | Types/privacy, manifest/dependency checks and real consumer builds; extend existing checks where sufficient. |
| 6 | Separate context/budget policy within the existing runtime | D1A and its approved prerequisites | Existing Rig facilities evaluated, bounded context tests, old fixed-history path retired. |

Potential boundary checks include keeping core free of UI/provider/converter implementations, keeping shared app/core free of GPUI, preserving optional native builds, and preventing an extracted writing component from importing the full desktop shell. Validate the actual dependency graph/features; do not use an inaccurate blanket prohibition on SQLite outside core, since the search index is derived SQLite. No new check, CI requirement, dependency or product feature is implemented by this documentation PR.

## How to judge improvement

Use the small milestone review in the [strategic guide](strategic-programming.md). Compare similar changes: how many unrelated decisions/locations must change; whether old production paths disappeared; whether repeat regressions fell; and whether relevant feedback is reliable and fast. Track deferred debt by consequence and revisit trigger in the owning plan. Do not infer an improving trend from this single review.

The appropriate target is a smaller burden of understanding and fewer repeated policies, with preserved behavior. A deletion quota, mandatory refactor percentage, maximum file size, higher test count or claimed 100% UI correctness would encourage the wrong work.

## Verification of this documentation change

The source/CI findings above are evidence gathered during review. Local documentation checks and independent review results are recorded on the resulting PR. Product code, manifests, lockfile, runtime guide bundle and CI configuration are unchanged. A documentation check does not repair or requalify the failed native widget test.
