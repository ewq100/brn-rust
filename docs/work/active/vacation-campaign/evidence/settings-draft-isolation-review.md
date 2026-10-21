# Independent review: Models Settings draft isolation

Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/settings-draft-isolation`.
Reviewed frozen candidate: `3d25183ba134b75a5d290d33182a651aafc3bd7f`.
Verified base/merge base: `9e2a81c0e3d5fbc1b617fe54bdb4b103f607b822`.
The checkout was clean throughout inspection. Reviewed the entire four-file candidate diff, including test-only regression commit `7462f0a04b5e195a4618cdd27af2a4fe7281c293`, whose regression test file is unchanged in the candidate.

## Finding

**[P2] Preserve the helper effort reset when Off is selected — `crates/brn-desktop/src/threads_tiles.rs:1639-1640`.**

The old helper selector assigned `settings.helper = None` on Off. Re-enabling from None constructed a fresh helper with `effort: "high"`; direct switches between enabled providers preserved effort. The new selector only clears `provider_form.helper_provider`, leaving `helper_effort` retained. Consequently, choosing Helper Low or Medium, Off, then Codex/Copilot now reselects and eventually saves Low/Medium rather than the existing High default. The same change occurs when a saved non-High helper is toggled Off and On. The new Save builder persists this retained value (`threads_ui.rs:3559`).

This is an unrequested retention behavior introduced while separating form ownership; restoring the old reset does not need a new product decision. Reset `provider_form.helper_effort` to `high` when Off is selected (or on a None-to-enabled transition), while preserving effort on direct enabled-provider switches. Extend the existing positive helper Save journey with Low → Off → On and an explicit High-selection expectation; its existing saved-High assertion can then also verify persistence. This finding was sent promptly to the lead. It is statically verified by comparing the full old/new selector and Save paths, not runtime-reproduced by this reviewer.

**Lead disposition supplied during review:** valid, correction selected. Preserve the frozen `3d25183b` qualification attempt and its actual results, then make the narrow reset correction and extend the existing helper Save journey without adding another test case. Requalify the final changed candidate before integration; an initial passing run does not qualify that later source. No reviewer source edits were made.

No other actionable finding was identified.

## Ownership and correctness review

The core separation is appropriate: `ProviderForm` contains only editable provider/effort/helper-provider/helper-effort segments; existing model/credential text inputs remain their own InputState entities. Canonical `self.settings` remains the source for active chat model/effort display, toolbar menu selection, explicit toolbar mutations, Copilot connection completion, startup model discovery, document/MCP/skill projections and unrelated configuration fields. All production direct writes to the old segmented fields were inspected; they now target the form rather than the canonical cache.

Startup initializes both snapshots from the same checked workspace settings. `refresh` reloads form segments alongside the canonical cache at the former cache-reset point. This retains the pre-existing refresh/navigation-reset policy rather than selecting a new draft-preservation guarantee. Text-input refresh behavior is unchanged. The form is a small local DTO, not a new persistence framework, schema or authority layer.

Explicit Save now builds a local settings value from the canonical cache, overlays the form and existing text fields, and uses unchanged `Workspace::configure`. It does not transiently install unsaved selections in the active cache before validation. Auth/credentials text trimming/emptiness behavior and provider-specific helper credential selection match the old Save logic. Applied, refused, failed or stale results still use the existing outcome reporting and refresh path. The existing Save/Connect disabled states while any local worker exists, explicit toolbar worker guards, checked configure authorization/run fencing and worker drain/stale-event handling are unchanged.

Unrelated toolbar actions clone the canonical cache, so selecting a toolbar model or effort cannot carry the form's provider/helper/other effort selection into persistence. CopilotConnected clones the canonical cache, changing only its intended provider/directory fields, so it no longer persists an unsaved Models effort. The connection itself intentionally uses the displayed credentials-directory input as its chosen destination; that existing explicit-action behavior is not an accidental form commit. Its final event/refresh and stale-worker controls remain unchanged.

The candidate still uses the existing canonical snapshot/cache model for concurrent external changes; it does not introduce or claim a new settings-version conflict protocol. Navigation retention and all-account/provider qualification remain outside this correction.

## Evidence and witness effectiveness

Read AGENTS, installed independent-review/resource/Rust guidance, relevant architecture/behavior/invariant and desktop README/verification contracts, the documented Save setup workflow, the full actual diff, all resulting changed ownership/Save/render code, and relevant initialization, refresh, toolbar, worker/connection, import/startup and shared configure callers. Inspected the complete affected regression journey, its fixture/window helpers, existing helper positive Save/worker-disabled test, and existing trailing-stale-worker connection witness.

The baseline log at `/tmp/brn-settings-draft-isolation-qualification/baseline.log` records a real one-test failure: saved effort was `high` where `medium` was expected (0 passed, 1 failed, 342 filtered). The lead associates this run with regression-only `7462f0a0`; that test's source is identical in the reviewed candidate. It first clicks the actual High segment, proves canonical effort remains Medium before any save, then delivers a synthetic current connection event through the owned Worker::connect receiver and checks the intended provider/directory update while effort remains Medium. These are independent public persistence expectations rather than assertions of ProviderForm internals.

The fixture creates a fresh canonical synthetic root, installs `offline-fixture`, clears credential locations and disables polling before the scenario. The connection event carries only an absolute synthetic path string; it invokes no sign-in, account/network operation, credential-file read or real model discovery. Existing helper Save evidence checks explicit persistence and worker-disabled affordances. The new regression covers current connection/effort isolation; it does not directly cover toolbar isolation, every form segment, failure/stale/worker branches, or the newly found Off/On reset interaction. Those paths were assessed against actual callers; the selected helper reset correction needs its final witness/check results.

## Qualification and limits

`git diff --check 9e2a81c0 HEAD` passed. Final inspected file SHA-256 values:

- `threads_state.rs`: `593871ef16669669b4877d098b084a835401156651401afe93beee626d0d7e86`.
- `threads_tiles.rs`: `2c11eeea20fde1252d166c5986ebe5a8f2ef5b5f037721c73d2d258075616c4d`.
- `threads_ui.rs`: `e51ee31214b0334c68a2614be8763efb0cbcb6128e0f8d4873e3162f00ececb0`.
- `threads_pages_tests.rs`: `f0113963f2f76d0f6530ffccf226c33c29a2698ab5a5f67ea0249bcd9427ab93`.

The frozen candidate's concurrent lead-owned qualification was still compiling at observation. No candidate pass, compilation success or full-suite result is inferred here. This reviewer ran no builds/tests and performed no source/shared-record edits, account/native/network actions, credential reads or nested work. Only this requested review artifact was written.

Native interaction remains explicitly deferred due the unavailable screen; widget/source checks do not establish physical-host usability or owner acceptance. Lead owns final correction, affected review verification, completed checks and normal integration. This report reviews exact `3d25183b`; the known narrow reset/test correction must be verified against its final diff and requalified before merge. No release is included.
