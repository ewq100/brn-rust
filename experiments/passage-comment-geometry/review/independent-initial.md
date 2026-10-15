# Independent review — initial candidate

Read-only reviewer `read_geometry_review`, requested GPT-6.1 Sol/high;
requested configuration is not runtime model telemetry. Reviewed clean commit
`38e0c95fd46e8ee96f5397d7f66e08662315e974`, including qualified implementation
`f64a9d50766428ac162d1e2e2d5d7db4c875bb3c`, against fetched main
`8bef439b133e91ad68b0b76792bcd155d3d06237`. Toolkit patch SHA-256:
`9bca1efb86ea0ba263fec58cad59a7ae39b43772cfa79259fa374f14dd83ff9f`.
The reviewer edited nothing and did not execute new repros or rerun builds.
The lead subsequently executed all five repros (see review record).

## Returned findings

1. **P1 — synthetic host: removing/reordering comments can attach another
   comment's text to the old passage.** `src/lib.rs:90`, render validation
   at 280–286 and label lookup at 299–302. The host stores `Option<usize>`;
   removing comment 0 after selecting it moves comment 1 into that index.
   The new comment has a valid saved range, so the label changes while the
   probe still addresses the first passage. Bind measurement to stable record
   ID plus validated document/version/range/quote/snapshot; clear when the
   identity disappears or changes. Add removal/reordering and resolution tests.
2. **P2 — lifecycle: clearing selected presentation does not cancel its pending
   explicit reveal.** `src/lib.rs:168`, selection reset at 189–192, source
   invalidation at 157–160; generated toolkit `state.rs:1052–1088,695–698`.
   Select an offscreen comment with reveal, then switch to an identical-source
   document before the next frame. `set_text` is a no-op and the old pending
   reveal scrolls the new document despite the cleared selection. A failed
   replacement selection has the same risk. Add supported cancellation on
   invalidation/navigation/mode changes and rejected replacement selection.
   The old settled/passive identical-source test does not cover this race.
3. **P2 — toolkit: async source change/revert can revive the previous probe
   without registration.** Patch line 206; generated `state.rs:707–712,1121–1128`.
   Validity compares a committed parse snapshot and source bytes; mutation
   neither clears the probe nor records mutation revision. Above 4 KiB,
   change then revert in one update before either background parse commits:
   both guards match the original snapshot again. Clear synchronously on every
   effective mutation or bind the mutation revision too. The existing small,
   settled synchronous edit/revert test does not establish this guarantee.
4. **P2 — synthetic host: Read → Write → Read restores the selected anchor.**
   `src/lib.rs:273`. The toggle flips `show_write` without clearing the anchor.
   A clean round trip restores the card without a new selection. Clear on
   every transition and test without introducing an edit that masks the gap.
5. **P2 — toolkit: a passage beginning at a hard line break misses its first
   covered glyph.** Patch lines 13 and 113. `new_probe` retains only the
   resolved piece's start; passive Inline rejects a non-glyph at that byte.
   `hello  \nworld`, source passage `\nworld`, must find the covered `w`;
   a newline-only passage must still not borrow `w` outside its range.
   Preserve enough exact covered-range information to locate that glyph.
   This finding was source reasoning, not an executed reviewer repro.

## Returned verdicts and limitations

**Toolkit:** changes required (findings 3 and 5) before accepting this exact API
patch for supported integration. The normal uncached TextView frame pipeline
appears internally consistent: clear report before descendant prepaint, measure
against current content mask, suppress passive scroll, prioritize explicit
reveal, invoke consumer after descendant paint. No additional concrete defect
found in that ordinary path. Arbitrary cached ancestors, native shaping/fallback
and outer-scroll behavior remain unqualified.

**BRN proposal:** reasonable integration direction, not production candidate
approval. Named seams match current source; proposal correctly requires stable
comment identity, clean-session/source guards, fail-closed mapping, identical
source document invalidation, retained navigation and rendering policy. It
correctly avoids generic `ThreadsUi::reveal`'s Write fallback. It needs explicit
pending-reveal cancellation, and the synthetic lifecycle gaps fixed. Actual B/C
implementation, source approval, integration ref and ownership remain pending.

The reviewer read the full patch, host, all 14 tests, production seams and
recorded build/lint/headless evidence. Those tests establish settled-frame
geometry and editor/Store witnesses; they do not establish removal/resolution,
stable record binding, pending-reveal cancellation, rapid async revert, or full
actual desktop Save/recovery/Apply/Undo integration. Native acceptance pending.
