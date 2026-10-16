> Historical qualification/handoff at `38e0c95`. The integration-review pass
> reproduced five additional gaps and corrected the isolated reference. Use
> [INTEGRATION-REVIEW.md](INTEGRATION-REVIEW.md) and
> [PROPOSED-INTEGRATION.md](PROPOSED-INTEGRATION.md) for current review status,
> patch identity, source/ownership blockers and pending production/native gates.
> No C exists yet (owner reply); no production integration is approved.

# To the current writing-module owner (B)

Selected direction: **one selected comment anchored to the first rendered glyph
of its exact passage in BRN's existing Read viewer**. No Zorite adoption,
full-range geometry, Write-mode comments or collision layout.

Qualified prototype: `f64a9d50766428ac162d1e2e2d5d7db4c875bb3c` on
`work/passage-comment-geometry`, from baseline
`7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`. Initial follow-up is `26b029756e3f20a3d9f0a7273230ab6cecee756f`;
original Z1 remains separately unchanged at `b9bd1a14862fe2efe246790e887797c949f13c50`.
Fetched main `8bef439b133e91ad68b0b76792bcd155d3d06237` has identical relevant
production trees/pins; only two docs changed. C must confirm live ownership
and record the actual integration ref. No production checkout was patched.

Use [the four-file upstream gpui-base patch](patches/gpui-base-0.7.3-range-start.patch)
for `track_range_start`/`clear_range_start` and `on_range_start_layout`.
Upstream/source-release approval belongs to the toolkit maintainer and C.
Do not vendor the whole toolkit into BRN. The ignored experiment extraction is
only build input; no toolkit tree is tracked or proposed for production.

B's current seams are `threads_writing.rs` (Read view/margin/click),
`threads_ui.rs` (private selection and note/version lifecycle) and
`threads_document.rs` (derived Read source/mode/highlights), plus tests.
If extraction has moved them, reconcile to C's new paths. Reuse exact saved
range/quote checks and source mapping; honor the existing clean-session rule.
Consume each paint result once, convert window to margin coordinates, and clear
on source/note/version/mode/comment changes, including identical-text notes.
An unavailable Read mapping stays unavailable in Read; do not take the generic
reveal helper's fallback into Write. Geometry calls no Save/draft/Undo path.

**Evidence:** 14 headless tests pass, including exact repeated/Unicode/formatted
passages, scroll/reveal, resize/rewrap, armed first paint, same/different-document
switches, source/revert invalidation, unchanged editor Undo, and a connected
editor-to-Store clean/dirty/generation/explicit Save/durable Undo witness.
Build, strict own-target Clippy and formatting pass. This is not a full desktop
integration run. [Detailed handoff, patch identities and evidence](HANDOFF.md).

**Before production acceptance:** obtain independent review of the toolkit API
and actual B/C candidate; rerun BRN writing/document/formatting plus checked
Apply/recovery/Save/Undo gates. Native checks remain pending: glyph/card alignment
with real fonts/Retina, split/resize/scroll, document/version switches and rapid
reveal races, actual dirty/recovery indicators, selection/focus/accessibility,
and unchanged reference/proposal rendering/link/asset policy. No IME behavior
is changed or newly qualified. Nothing has been published, merged or released.
