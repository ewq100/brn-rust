# Handoff to the writing-module owner

Keep the current editor and viewer. BRN already selects exact source ranges,
validates saved comments, highlights them in Read, and reveals them on click.
The missing piece is passive passage position for cards: today's margin is an
independently scrolling sorted list.

The prototype demonstrates one active Read-view card aligned with the first
rendered text glyph covered by the comment's exact source range. It uses the
current viewer's parser, source map and shaped glyph boxes. The proposed API
is `track_range_start` + `on_range_start_layout`, supplied as a reviewable
four-file gpui-base patch. It returns a window-coordinate glyph box after
paint, or `None`; it does not return a caret or full-range rectangles. Consume
the measurement once per frame and paint the margin after the viewer. Keep
an explicit unavailable state before layout and for hidden/stale content.

No Zorite dependency or copied Zorite code is justified here. The original
Z1 failure remains preserved on `work/z1-zorite` at `b9bd1a14862fe2efe246790e887797c949f13c50`.
The fresh integration baseline for this prototype is
`7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`, not the Z1 implementation commit.

## Proposed integration, not applied

| Owner | Paths/dependency | Required work |
|---|---|---|
| Toolkit maintainer, coordinated by C | gpui-base `src/text/{range_highlight,inline,state,text_view}.rs` | Review passive API/lifetime, add upstream regression tests, prefer an upstream supported release to a permanent fork. No extra dependencies. |
| B, after ui-polish releases writing paths | Snapshot paths `crates/brn-desktop/src/threads_writing.rs`, `threads_ui.rs`, `threads_document.rs`, relevant writing/document tests; or their C0 extraction destinations | Wire only the active card first; reuse `comment_location`, source mapping and existing `reveal`. Clear on note/version/source/mode change. Convert window coordinates to the margin's local coordinates. Preserve `safe_markdown`, caller-scoped asset map, saved-comment and dirty-draft rules. |
| C | Future root Cargo.toml/Cargo.lock only if an approved toolkit source/release is chosen | Own dependency approval and lockfile change; record C0 integration ref and applicable F-suite. No root edit was made here. |
| Owner / integration lead | Native and integration acceptance | Run the checks below on the recorded integration tree; then decide whether multiple visible cards and collision handling merit a separate slice. |

Do not turn geometry notifications into `InputEvent::Change`, a Save, a new
draft generation, Apply or Undo. Source edits use existing checked Save/core
comment mapping; overlapping edits stay unresolved. Never locate a comment
by searching for its quote. Do not reuse the editor's IME `bounds_for_range`
as arbitrary passage geometry: it can fall back to the active caret.

The experiment override does not release production ownership held by
ui-polish/daily-workflow-reliability. The stored C0 prerequisites remain for
integration. Refresh the baseline after they land and C records an integration
ref. No canonical C0 `plan.md` or integration ref was present at this snapshot.

## Consolidated pending qualification

1. Integrate in the owner's writing checkout after ownership release. Re-run
   these 11 tests plus BRN document/writing/formatting tests and required native
   build/lint, then shared Save/recovery/checked Apply/durable Undo regressions.
2. On an unlocked Mac, use a fresh synthetic data directory. Select the second
   duplicate comment, including Estonian, combining marks, emoji and formatted
   text. Verify the actual highlighted occurrence and card placement at normal
   and Retina scale, including proportional/fallback fonts.
3. Resize/split the Note tile, rewrap, scroll both directions, switch Read/Write,
   navigate to another note, reopen, and edit then Save/Undo. No old card may
   remain on screen; offscreen/unmapped/dirty comments show unavailable. Include
   rapid successive comment clicks and edits while an earlier reveal is pending.
4. Exercise long/wide code, tables, nested lists, literal HTML, figures,
   malformed Markdown and long documents. Confirm clipping, explicit reveal
   settling, no external asset reads, and no change to link allowlisting.
5. Verify reference/proposal viewers remain their existing read-only surfaces,
   input focus/selection/accessibility are usable, and no new typing, marked-text
   or paste behavior appears. Native IME remains a separate check of the current
   editor; this geometry patch changes no input handlers.

Live formatted editing is a separate future capability with source identity,
draft/Undo, input and Markdown-policy costs. This prototype neither implements
it nor supplies evidence to replace the editor for it.
