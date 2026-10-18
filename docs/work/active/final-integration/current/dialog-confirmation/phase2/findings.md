# Phase2 findings

## Local helper setup disposition fulfilled

Exactly the five failed Documents cases passed5/0/0 on the unchanged phase1 failed binary, SHA256 `b3c7c6aeb28b8111ff8a544b95f5d70dffddd84d3839db4b74ed0a725e174514`, and unchanged `.d` SHA256 `681df80d090741ae576b486a7fe7a063bac488c6377ed02c4c5951c163eb9698`. Its compiled material remains e651; the current documentation HEAD is not a new compiled-source association. This was direct existing-binary execution, no Cargo or helper rebuild.

Both explicit helpers are the previously qualified absolute files under `/private/tmp/brn-final-integration-20261018-target/document-helpers/artifacts/`: stock SHA256 `a2c223701c93399b06acc143c3190a2a908befa411f1c674fc7a596fdb545619`; BRN SHA256 `bc8296e57e18ab63f0388871763c26544a850a9d1de517f26bca3c9cf2674952`. Source/hash proof is in `artifact-bindings.json`. This closes only the setup diagnosis for local300/5/1. That failed suite remains failed; no whole-suite retry or green replacement receipt was produced.

## Correct pinned sources and clock limit

The lockfile selects gpui-kit/base/component0.7.3 and gpui-pre/scheduler0.3.9. Phase1's gpui-component0.7.1 read was an installed older source, not current dependency proof; this phase corrects that attribution explicitly without rewriting its historical report. Actual0.7.3 source retains a250ms default SlideDown entrance and maintained reduced-motion endpoint. `Window.click` computes a center once, dispatches move/down/up at it and renders between events. The one-shot animation uses `scheduler::Instant`, which reexports real web_time::Instant; it calls `state.start.elapsed()`, not the fake scheduler clock. `advance_clock` would not control this entrance. No sleep or clock claim was substituted.

## Controlled red/positive checkpoint

A temporary private helper mirrored the pinned click's single real move/down/up events, with observations after completed frames and unchanged Enter-refusal/confirmation/closed-dialog assertions. It recorded bounds, focus, pointer inclusion, active dialog, actual window dispatch results and callback count. Dispatch observations are not a claim that each event reached the target handler; the separate callback count is the direct effect observation. No direct callback invocation or second click.

Natural control passed1/0/0: target and down remained y121.5, pointer y137.5 stayed inside, callback count reached1 at mouse-up, and dialog closed.

Controlled **after-target** endpoint was red0/1/0 on the original `confirmed.get()` assertion. The target was captured at bounds origin(786,121.5), size121×32, pointer(846.5,137.5). Applying the maintained reduced-motion endpoint after lookup moved the button to y205.5 before down/up. The retained point was outside. Move/down/up were dispatched; callback count stayed0, confirmation false and dialog active. Enter-inert assertions had passed. This is the same assertion symptom through a controlled geometry transition.

Controlled **before-target** endpoint passed1/0/0 on the **same source and same compiled binary**: target origin(786,205.5), pointer(846.5,221.5), pointer inside through down; one callback at up set confirmation true and closed the dialog. Enter remained inert. Binary SHA256 for all three controls is `532b2b610e8b8fd2cd6e912f5223226bee92dc169e7069495fa9fbd3fbb06926`; full `.d`, source/patch/input maps and fingerprint preserved. See raw `mechanism-checkpoint.json` for complete structured events.

This counterfactual establishes that geometry moving after cached target lookup can produce the observed false-confirmation assertion and that a settled target permits the actual callback. It does **not** identify the unique historical CI cause: no hosted geometry/event trace exists, and prior unchanged local seeds0–99 and the dialog case in the context run passed. No high natural failure rate is claimed. Overlay/focus/handler defects are not proved by this probe; the successful stable real click argues against needing a production component change for this controlled scenario.

The initial diagnostic compile failed on a missing InputEvent trait and mutable-context mismatch. Its failed source/command/log is retained as02; it is not a behavioral red. Corrected private instrumentation is03–05. No SDK/vendor/dependency/production change occurred.
