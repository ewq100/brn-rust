# Final independent review: Models Settings draft isolation

Checkout: `/Users/evokessler/.codex/worktrees/mac-test-roots/brn-rust`, branch `codex/settings-draft-isolation`.
Final reviewed candidate: `fde0477385639913aa9e8dcf39981f95c0aff8d2`.
Baseline: `9e2a81c0e3d5fbc1b617fe54bdb4b103f607b822`.
Previous reviewed candidate: `3d25183ba134b75a5d290d33182a651aafc3bd7f`.
The checkout was clean at inspection. Reconciled the complete final four-file candidate with the prior full-candidate review and inspected every intervening changed line and both resulting affected journeys.

## Findings and disposition

No remaining actionable findings. The prior P2 helper Off/On compatibility finding is **valid/fixed in final source**.

The final selector (`threads_tiles.rs:1639-1644`) clears the helper provider and resets helper effort to High only when Off is selected. Enabling or directly switching Codex/Copilot sets only the provider, preserving effort for direct enabled-provider switches. This restores the old Off→On reset without changing canonical settings ownership, other form fields, or the Save boundary.

The existing helper Save journey now clicks Low → Off → Codex before its original Save. Its unchanged complete HelperSettings expectation still requires `effort: "high"`, along with the original provider/model/auth binding. This observes actual persisted behavior, does not derive its expected result from ProviderForm or the selector, and removes/weakens no assertion. Existing visibility/tested-note/credential and worker-disabled checks remain. No new test case or test framework was introduced.

Read `/tmp/brn-settings-draft-isolation-qualification/helper-reset-negative.log`: the augmented existing test actually failed with saved Low versus expected High (0 passed, 1 failed, 342 filtered) before the selector correction. The lead associates this run with `29a02b4f`; the log itself does not embed that commit. The concrete observed failure supports the new event sequence's effectiveness rather than treating source reasoning as a behavioral red.

`threads_state.rs` and `threads_ui.rs` are byte-identical to the previously reviewed candidate. The only later changes are the three helper-test clicks and the narrow Off reset. The complete final diff remains four files, 118 additions/44 deletions. Thus the previously reviewed separation of editable segments from active canonical settings, startup/refresh reset semantics, explicit Save overlay, unrelated toolbar/connection isolation, checked configure/run fencing and offline fixture ownership remain intact. No new draft-retention policy, production provider/account action, schema, dependency or authority change is present.

## Qualification and limits

`git diff --check 9e2a81c0 HEAD` passed. Final file SHA-256 values:

- `threads_state.rs`: `593871ef16669669b4877d098b084a835401156651401afe93beee626d0d7e86`.
- `threads_tiles.rs`: `7722d90fe3b2c342505d5d65edc4bc3578794966d5fa2a2fdaaa0a2dfa14e937`.
- `threads_ui.rs`: `e51ee31214b0334c68a2614be8763efb0cbcb6128e0f8d4873e3162f00ececb0`.
- `threads_pages_tests.rs`: `4bfd5120b10d9769c8bb18902734bb31ebcb388f34ccc11baf39a5bd9cf4ec00`.

The lead reports the original `3d25183b` focused test, full desktop suite (342 passed/1 existing ignored), strict lint, ordinary build and formatting passed. Those earlier source-bound passes do not qualify the final correction. Final candidate checks are lead-owned and pending for this review; no final pass is inferred. This source review closes the P2 code finding but does not replace the final positive witness/full-check gate.

This reconciliation retains the coverage/limitations in `/tmp/brn-settings-draft-isolation-review.md`: synthetic widget/state evidence, a current connection/effort regression and an existing helper Save/reset journey; source-reviewed toolbar/render/failure/stale paths; no real sign-in, credential read, provider/network call or physical Mac interaction. Native usability and owner acceptance remain explicitly deferred and distinct from normal integration.

No source/shared-record edits, builds/tests, account/native/network actions or nested work were performed by this reviewer. Only this requested report was written. Lead owns completed final checks and integration; no release or native/owner acceptance is claimed.
