# Knowledge refresh seam reassessment

Read-only assessment of `/Users/evokessler/.codex/worktrees/architecture-status-route/brn-rust`, branch `codex/knowledge-check-refresh`, clean HEAD `b8141b576dd9cb9e30f3215620b9122327effb63`; base `f79f6540276c00155ab3bc0d7f019f4ea5d00c9e`. No builds, tests, native interaction, network/account actions, nested helpers, or source edits were performed. This report is the only file written. The bounded three-attempt run loop remains stopped.

## Verdict

There **is a concrete supported observation seam**, established by both pinned dependency source and existing runtime evidence. Do not describe this as “no supported deterministic seam.” Use the already-observed accessible pending label on the real `knowledge-run` control, followed by the registered `knowledge-summary` and `knowledge-review` controls. No new API or test-only state injection is needed for that observation.

However, **none of the three saved runs reproduced the application defect**: each stopped on a harness assertion before the pending summary/Review checks. No production fix is qualified or recommended for immediate integration. Preserve the stopped run limit; this report does not authorize another attempt. A lead-selected continuation could use the concrete seam below, or defer production work pending that qualification.

Minimal result retirement at the start of an explicit replacement check is a coherent, narrowly scoped implementation of “a new check retires the displayed old findings.” It is supported by the source mechanism, but it remains a selected behavior change rather than an already passing regression fix. Neither the existing snapshot design nor the Connections behavior independently requires invalidating every knowledge result whenever workspace state changes.

## What the saved failures actually establish

- `initial-red.log` (commit `041e9a3320e22b809fd6b386e6402485b8193cb8`): missing `knowledge-finding-0`. Production assigns the finding div an ID but does not opt it into observation (`threads_connections.rs:451–457`). The failure does not demonstrate missing displayed finding text.
- `corrected-red.log` (commit `b0480c3b97ba67e9b582f2f7fc4dd012f7253d00`): the immediate frame after the real second click returned `Some("Check knowledge, unavailable: a check is running")`; the test expected visible text `Some("Checking…")`. This is useful positive runtime evidence that the existing click/immediate-frame sequence reached the running render before completion. It is not a stale-result assertion failure.
- `final-baseline.log` (HEAD `b8141b5`): `disabled()` returned `None`, expected `Some(true)`. Again this stops before the summary and Review absence assertions. It establishes neither enabled input nor completed background work.

All three commands are preserved beside their raw logs under `/tmp/brn-knowledge-refresh-qualification/`. The HEAD diff is test-only: one file, 82 additions/2 deletions. The original first Check, DuplicateTitle, seeded review-message and no-worker assertions remain before the extension. The fresh-result and durable-fixture assertions after the failed pending assertion have not executed successfully in these runs.

## Actual pinned button and observation semantics

The inspected worktree pins GPUI Kit/Base/Component **0.7.3** (`Cargo.lock`, `crates/brn-desktop/Cargo.toml`). The primary checkout has different dependency source; it must not be used to infer this candidate's API.

The real route is:

1. `threads_ui.rs:6–19` imports `gpui_kit::component::button::Button`, with the shared `button()` builder imported through `brn_writing_ui::native::*`.
2. `crates/brn-writing-ui/src/native.rs:54–59` creates that Component Button and applies presentation defaults.
3. Pinned `gpui-component-0.7.3/src/button/button.rs:192,247` owns/creates `gpui_base::Button`. At `748–751`, an explicit accessibility label wins over the visible label; at `796,817–818`, disabled state and the chosen accessibility label are forwarded to Base Button.
4. Pinned `gpui-base-0.7.3/src/button.rs:41` observes its root through `.test_support()`. Its render at `202–263` withholds activation/focus for disabled controls and stops left mouse-down propagation. It does **not** set an accessibility disabled property.
5. Pinned `gpui-base-0.7.3/src/test_support.rs:106–115` explicitly documents that `disabled() == None` means no native disabled flag was exposed, and absence does not prove input acceptance. `label()` is the accessibility label, which can intentionally differ from visible text. At `317–318`, disabled observation derives only from `node.is_disabled().then_some(true)`.

Dependency source root: `/Users/evokessler/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

Pinned `gpui-kit-0.7.3/src/test.rs:28–37,143–160,213–220` documents snapshots as facts from a completed frame; `window.click` performs native pointer dispatch and frame renders without calling `run_until_parked()`. The repository's ordinary `click()` helper (`threads_tiled_tests.rs:67–72`) drains afterward. The extension deliberately bypasses that helper for the second click. The corrected-red runtime label independently confirms that this sequence observed running state on that execution. This is a supported immediate-frame seam; no assertion here claims it has now been rerun or shown repeatable across multiple schedules.

## Source cause and policy boundary

`threads_connections.rs:361–380` sets `running = true` and clears the error but retains `knowledge.result`. Success replaces the result; failure records an error while retaining the previous result. `knowledge_panel` at `438–470` renders summary/findings/Review solely from that retained result, with no running condition. `review_knowledge` at `383–387` clones the same result and seeds a chat without a running guard. Thus retained old findings and Review during replacement/failure are a direct source consequence, independently of the broken test assertions.

The result is an owner-triggered **general-knowledge snapshot**, not a note-bound neighbourhood: `KnowledgeView` has no note identity or generation; `Workspace::knowledge_check` (`crates/brn-threads-app/src/knowledge.rs:144–156`) scans current general records. `seed_knowledge_review` (`521–526`) creates a chat and posts the supplied findings; it does not run a worker or silently repair notes. The current behavior contract (`docs/architecture/threads-behavior.md:143–154`) treats these as structural evidence, not truth or mandatory repair. This is not evidence of a checked-write or authority bypass.

Connections has a different identity policy: `threads_connections.rs:60–117` replaces note-specific requests, clears old results, cancels/fences prior work by generation, and refuses late publication. Knowledge is global and explicitly triggered. Copying Connections' entire retirement/generation policy would be an unjustified expansion. Clearing only `knowledge.result` when starting the owner's replacement check is the minimal coherent change if that selected UX policy is retained. It also ensures a failed replacement cannot leave the old Review actionable. It would not promise automatic currentness after every unrelated note edit, nor require a new persistent workflow, result timestamp, worker, generation subsystem, or provider call.

## Concrete alternative for lead selection, not an authorized retry

Keep the existing synthetic archive and real second-click sequence. Replace the unsupported pending marker with the exact already-observed accessible label:

`window.find("knowledge-run").label() == Some("Check knowledge, unavailable: a check is running")`

Then assert the absence of **registered** `knowledge-summary` and `knowledge-review`. Both have positive preconditions in the current extension; summary registers explicitly at `threads_connections.rs:445–446`, Review registers through Base Button. Remove the `knowledge-finding-0` absence claim: because that div never registers, `try_find(...).is_none()` is vacuous on both correct and incorrect application behavior. It adds no defect evidence. Summary and Review absence cover the same result-rendering branch.

This alternative is grounded in actual source and a saved runtime value, unlike the previous visible-label and disabled-property guesses. It has not been executed by this assessor. The baseline is expected from source to fail on retained summary/Review after the exact pending label succeeds, but that remains an expectation until an admitted run reaches it.

Remaining qualification, if separately selected: a genuine baseline failure on a supported result/Review assertion; minimal production retirement; successful replacement result without DuplicateTitle; exact durable synthetic-record preservation and no worker; appropriate focused checks and formatting; explicit treatment of replacement-error behavior if that outcome is claimed. Native usability, live usefulness, owner acceptance, integration and release remain separate and unperformed. Do not silently convert this source assessment into a green baseline or permission to exceed the selected attempt cap.
