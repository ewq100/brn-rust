# Selected direction: one selected passage comment in Read

Owner selected this direction on 15 October 2026: extend BRN's existing Read
viewer, with no Zorite adoption. The demonstrated capability is **one selected
comment anchored to the first rendered text glyph of its exact passage in Read**.
The box uses the glyph's horizontal span and line height. It is not a caret,
full-range geometry, Write-mode comments or collision layout.

For the short production-owner handoff, use [OWNER-HANDOFF.md](OWNER-HANDOFF.md).

## Exact deliverable and reconciliation

| Identity | Exact commit/ref |
|---|---|
| Original Z1 qualification, unchanged and separate | `b9bd1a14862fe2efe246790e887797c949f13c50` on `work/z1-zorite` |
| Initial follow-up prototype | `26b029756e3f20a3d9f0a7273230ab6cecee756f` |
| Qualified follow-up code, including missing bounded checks | **`f64a9d50766428ac162d1e2e2d5d7db4c875bb3c`**, descendant of the initial prototype, on `work/passage-comment-geometry` |
| Follow-up branch baseline | `7e9d0376d677ed2a46bdbcd8933f55f9e1746a02` |
| Fetched main for this handoff | `8bef439b133e91ad68b0b76792bcd155d3d06237` |

[CHANGED-FILES.txt](CHANGED-FILES.txt) lists all 99 initial prototype paths and
all 19 qualification-completion paths, including evidence; every path is under
`experiments/passage-comment-geometry/`. The implementation files are
[Cargo.toml](Cargo.toml), [src/lib.rs](src/lib.rs), [src/main.rs](src/main.rs),
and [tests/geometry.rs](tests/geometry.rs). Support files are the standalone
lockfile, checksum-verified bootstrap, audit/runners, notices and documentation.
The final documentation commit contains this handoff and its review artifacts;
the qualified code commit above identifies the executable/tested tree.

Two separate review artifacts:

- **Toolkit extension:** [gpui-base patch](patches/gpui-base-0.7.3-range-start.patch),
  SHA-256 `9bca1efb86ea0ba263fec58cad59a7ae39b43772cfa79259fa374f14dd83ff9f`.
  This patch is unchanged from the initial follow-up prototype.
- **BRN synthetic prototype source:** [source review patch](patches/brn-prototype-source.patch),
  SHA-256 `8205e08d27f9d8fbbd1f369adcf49ffaa3d749421f6623263d312e6fb1acbb49`.
  Exact diff of the four implementation files above from baseline to the
  qualified code commit. This is a review extract, not an applied production
  patch or a standalone bootstrap; the lockfile/bootstrap remain in the commit.

The [baseline audit](evidence/handoff-baseline.stdout) records Git object IDs.
Between baseline and fetched main, only `docs/status.md` and
`docs/work/active/quality-simplify/evidence.md` changed. The entire desktop,
core and app crate trees, root Cargo files, toolchain, AGENTS, writing-polish
and modular-foundation guidance are identical. The qualified tests therefore
use the same relevant production code/pins as fetched main. No rebase or
production integration was performed. Main's current status was read; no
canonical C0 active plan/integration ref was present. C must record the actual
integration ref and confirm live ownership/release conditions before wiring.

## Toolkit extension: upstream, not BRN workflow

The implementation must live **inside gpui-base's TextView layout pipeline**;
the shaped glyph/layout information is private there. In upstream
`longbridge/gpui-kit`, revision `67265a36e260ffe03c5aaebdcd8d654dbf892eab`,
the paths are under `crates/base/`:

| Upstream file | Exact responsibility |
|---|---|
| `src/text/range_highlight.rs` | Resolve a passive range to its first covered text leaf; reuse reveal's glyph-report channel with scrolling disabled; no nearby-leaf/block fallback. |
| `src/text/inline.rs` | Measure the actual glyph during layout/paint; no nearest-glyph fallback for a missing glyph; report only a visible box; passive requests never autoscroll. |
| `src/text/state.rs` | Add `track_range_start(range, cx) -> Result<(), RangeHighlightError>` and `clear_range_start(cx)`; retain one probe bound to the rendered snapshot; never remap it after source edits. Explicit text reveal has priority. |
| `src/text/text_view.rs` | Add `on_range_start_layout(Fn(Option<Bounds<Pixels>>, &mut Window, &mut App))`; clear each frame's report and call after descendant paint in logical window coordinates. |

No parser, text buffer, input handler or Undo changes. No additional external
crate. gpui-kit already reexports these types. Versions remain kit/base/component
0.7.3 and one gpui-pre 0.3.9 graph. Source/license/checksum details are in
[UPSTREAM.md](UPSTREAM.md); gpui-base is Apache-2.0, with its notice retained.

**Maintenance approach:** submit/review the narrow upstream API and regression
coverage, then use an approved supported toolkit source/release through C's
shared dependency ownership. Recheck the four layout/reveal touchpoints when
the toolkit changes. Until an approved source is available, production wiring
remains pending. Do not vendor the toolkit into BRN, copy private layout code
into the writing module, or introduce another Markdown interpreter. No toolkit
or root dependency change is applied by this handoff.

The ignored `.vendor/gpui-base/` is disposable experiment build input extracted
from the pinned, verified crate archive so the patch can be compiled headlessly.
**Zero toolkit tree files are tracked or proposed for production.** The only
maintained toolkit deliverable here is the four-file patch (145 added/six
removed lines), plus its source checksums and tests using the real component.

## BRN-specific integration: current writing owner B

The BRN layer owns selected-comment identity and presentation, not glyph
measurement. Bind the selected comment to comment record ID, note ID, saved
version, exact source range/quote and current rendered snapshot. Reuse
`comment_location`/`exact_writing_range` and `range_for_source`. Require both
BRN's clean-session rule and source equality; a recovery session is not made
clean merely because its visible text matches saved text.

On validated selection, register the passive probe and existing Read highlight;
request existing Read reveal only after source-to-rendered conversion succeeds.
The generic `ThreadsUi::reveal` can fall back to Write if Read conversion fails:
the new selected-Read-comment path must instead show unavailable and stay in
Read for an unmapped range. Do not fabricate a position from quote search,
whole-view bounds, the active caret, or the editor's IME bounds callback.

Consume the paint callback once per frame. Paint the selected margin card
after the viewer; subtract the margin's window origin to obtain local coordinates.
Before layout/offscreen/hidden/stale/unmapped means unavailable. Clear the
selected anchor on note/version/source/mode changes and comment removal or
resolution, **even when two notes have identical Markdown**. The prototype's
`clear_comment_anchor` demonstrates invalidating presentation without loading
text, saving or touching input history. Existing document loading remains a
separate action; its existing `set_value` behavior is not a geometry operation.

| Owned paths at both recorded main refs | Integration seam |
|---|---|
| B: `crates/brn-desktop/src/threads_writing.rs` | Read viewer construction and `margin_column`/comment click: render only the selected aligned card; retain the existing comment list as navigation. |
| B: `crates/brn-desktop/src/threads_ui.rs` | Private selected-anchor state; invalidate on `open_note`, checked `reload_open_note`/version changes and note close; keep the existing selection/reveal and editor event rules. Do not call `note_changed` or `save` from geometry. |
| B: `crates/brn-desktop/src/threads_document.rs` | `readable_document`, `sync_read_text`, `set_note_reading`, `sync_read_marks`: hook passive measurement and clear stale presentation. Preserve `safe_markdown`, caller-scoped stored assets and link allowlisting. |
| B: writing/document tests | Add these cases to actual `ThreadsUi` fixtures; keep existing formatting, request/review/Apply, recovery and Undo tests. |
| C: root Cargo.toml/Cargo.lock, dependency integration ref | Approve and reconcile an upstream supported toolkit source. No core/app/schema change is needed for geometry. |

If C0 extraction moves these seams into `brn-writing-ui`, B ports to the recorded
new destinations rather than patching old desktop ownership. The experiment
exception does not release another builder's paths. No owned production file
was modified here.

## Evidence: 14 deterministic tests passed

[Latest output](evidence/handoff-headless-01.stdout),
[build](evidence/handoff-build.stderr),
[strict own-target Clippy](evidence/handoff-clippy.stderr),
[format](evidence/handoff-fmt.stderr) and
[exact commands/environment](evidence/commands.jsonl). All tests execute GPUI's
headless test platform with synthetic data; no native window was launched.

| Requirement | Demonstrated check | Boundary |
|---|---|---|
| Exact passage identity | `exact_second_occurrence_unicode_formatting_and_crlf`: second stored byte range maps to second rendered occurrence; Estonian, combining marks, emoji/ZWJ, CRLF, bold, links and code. Wrong-note/version/range/quote and unresolved cases refuse; no quote relocation. | Actual native shaping/selection remains pending. |
| Scrolling/reveal | `passive_offscreen_none_then_comment_click_reveals_second_without_text_change`: passive query leaves scroll offset unchanged; actual comment clicks reveal later then earlier occurrence. Offscreen clears card. | Native scroll settling and rapid reveal/edit/navigation races pending. |
| Resize/rewrap/coordinates | `resize_rewrap_font_scroll_and_coordinate_conversion_use_current_frame`: y 227 → 435 on narrowing, y 737 with larger text; short viewport makes unavailable, reveal gives y 247. Nonzero origins and card y == glyph y asserted. Nested lists/quotes, code and table-cell routes pass. | Retina/fallback fonts, arbitrary outer containers and long-document performance pending. |
| First layout | `armed_passage_has_no_report_before_first_paint_then_measures`: exact probe registered before window creation has no report; first painted frame supplies a visible glyph box. Existing host also clears previous output before layout. | Direct toolkit first-paint check uses default Markdown configuration; host policy fixtures separately test BRN's configuration. |
| Document switching | `document_switch_identical_text_and_changed_text_clear_old_anchor`: same editor/viewer entities and same parsed snapshot across identical-source note A → B; clear old output, reject A comment, select B. Changed-source C also starts unavailable. | Test-only navigation seam; production `ThreadsUi::open_note` wiring is pending. |
| Stale layout | Source edit/reparse and revert cannot reuse a probe; real typing and silent programmatic loading invalidate. Empty/separator/non-glyph ranges give no borrowed box. | No stale geometry carried as an estimate; full desktop lifecycle still needs integration regression. |

## Text, dirty state, Save, draft generations and Undo

`positioning_and_reveal_do_not_modify_current_editor_undo` verifies text and
Change counter stay unchanged during repeated display/reveal. Pristine Undo
stays empty; real typing Undo/Redo survives Read/comment operations. This tests
the current editor's history, not a replacement Undo engine.

The new `coupled_editor_store_witness_preserves_clean_dirty_generation_save_and_durable_undo`
connects actual current-editor Change events to actual core Store recovery APIs:

- Clean display/reveal/resize/anchor clear creates no edit session or Change
  event and changes no record, revision or receipt; saved text stays identical.
- Real typing is the positive control: it creates a persisted recovery session
  at generation **6** for the six simulated input characters.
- Dirty display/reveal refusal/resize/anchor clear preserves that exact text,
  session, generation, open/dirty recovery state, records and receipts. No Save.
- Explicit Save at an older generation is still refused; explicit current Save
  succeeds; replay returns the same receipt; durable checked Undo restores the
  original saved text.

The earlier real-Store mapping test still verifies checked Save remaps an
unaffected second passage and marks an overlapping edit unresolved. Geometry
has no storage calls or event emission. The connected witness is deliberately
small and is **not** the full desktop's `note_changed`/Save wiring. Production
clean-session/dirty indicators, per-note recovery, checked Apply, Save and Undo
must be rerun after the actual writing integration; they are not claimed passed
for an unapplied production change.

## Review and remaining native checks

Independent review is **pending**, required before production integration.
Review the exact toolkit patch and B's eventual candidate against C's recorded
ref: cached paint/report lifetime, document/version binding, reveal priority,
source mapping, fail-closed invalidation, no passive scroll, no notification →
draft/Save/Undo path, and preserved rendering/link/asset policy. Add upstream
regression tests for the passive API; this handoff's tests are consumer tests.
Then run existing BRN document/writing/formatting and checked Apply/recovery/Undo
suites, relevant native build/lint and required integration gates on that tree.

Native observation is **pending**. One consolidated synthetic qualification:

1. On an unlocked Mac, select the second repeated passage comment, including
   Unicode/formatting; confirm the actual glyph and aligned selected card at
   normal/Retina scale with proportional and fallback fonts.
2. Resize/split/rewrap/scroll; switch between identical-text and different notes,
   reload/version-change, resolve/remove the selected comment, and rapidly
   select comments while a previous reveal is pending. No old card may remain;
   unavailable stays explicit and Read does not switch to Write on a failed map.
3. Check actual dirty indicators, saved text/version, draft recovery generation,
   explicit Save/refusal and editor/durable Undo across those interactions.
4. Verify references and proposal review retain existing rendering/read-only
   behavior, stored assets/link restrictions, selection, focus and accessibility.
   This patch changes no macOS input/IME handlers and adds no IME qualification.

No Zorite reevaluation, live Write comments, multi-comment collision layout,
publishing, merge or release is part of this handoff.
