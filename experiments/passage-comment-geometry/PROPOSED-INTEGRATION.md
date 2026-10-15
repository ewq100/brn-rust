# B/C integration proposal — awaiting authority and supported toolkit source

This is an implementation handoff for review, **not a production patch**.
One selected saved comment may align beside the first rendered glyph of its
exact passage in Read. Keep the current editor, parser/viewer and comment list.
No live Write comments, full-range geometry or collision layout.

## Dependency and ownership gates

C must provide the actual integration ref and released owner paths. Fetched
main `8bef439b133e91ad68b0b76792bcd155d3d06237` has no writing extraction; it
is evidence of the repository's visible state, not confirmation from C.
If extraction has moved code, use the new shared-writing module paths. C must
approve a supported toolkit source and root manifest/lock reconciliation.
No production files or dependency pins have been changed in this review.

The extension belongs inside gpui-base, upstream directory `crates/base`, in
`src/text/{range_highlight,inline,state,text_view}.rs`. The narrow patch adds:

- `TextViewState::track_range_start` and `clear_range_start` for a passive,
  source-mutation-invalidated exact rendered passage probe;
- `TextView::on_range_start_layout` for this painted frame's visible glyph box
  in logical window coordinates (`None` means unavailable);
- `TextViewState::clear_reveal` to cancel a pending explicit reveal when its
  host-owned context disappears. It does not reverse already completed scroll.

Maintain this as a reviewed upstream API and regression suite. Once available,
use a C-approved supported toolkit revision/release; explicitly review any
manifest/lock changes and revalidate shared versions. Do not commit the ignored
experiment extraction, copy private layout logic into BRN or introduce an
unapproved fork. The old stock 0.7.3 source does not expose these APIs.

## Proposed BRN state and transitions

Use the actual saved comment **record ID**, note ID, saved note version, exact
range/quote and rendered snapshot as the selected binding. Use the existing
writing-comment addressed/resolution/archival state; `Comment.unresolved` means
an unmappable passage, not a resolved conversation. Neither a list index nor
text equality establishes identity. Keep the existing clean-session guard as
well as source equality; a recovery session with matching text is still a
session. Reuse `comment_location`/`exact_writing_range`, `range_for_source` and
the current source-mapped Read highlights. No quote search or fuzzy relocation.

On explicit comment selection: cancel the prior comment-owned reveal, clear
its probe/highlight/measurement, validate the binding against current saved
state, convert its exact source range, and only then register the probe and
existing Read reveal. On a rejected/unmapped selection, stay in Read, with no
pending old reveal and no position. Do not use generic `ThreadsUi::reveal`'s
fallback into Write. Passive measurement must not request reveal or scrolling.

On every document/version/source/mode change, close/reload, selected comment
removal, addressed/resolution/archival change, or payload/range/quote change:
clear the binding, probe, highlight and frame result. Cancel its pending
comment-owned reveal too. Document/mode invalidation cancels any reveal belonging
to the invalidated document/mode, including outline requests. Identical Markdown
in different documents must clear. A later revert/reopen requires new selection.

BRN's existing outline navigation already calls `ThreadsUi::reveal` through
`threads_document.rs`; both outline and comment navigation reach the same
TextView. `clear_reveal` cancels unconditionally. Coordinate all Read reveal
requests at that shared boundary with host-owned owner/document/generation
state (or a supported request token). Track which request superseded which.
Clearing a comment must not cancel a newer outline request. An invalid comment
selection cancels its prior comment request but must leave an unrelated outline
request alone. A valid explicit comment selection may supersede an outline
request. The toolkit currently keeps one pending explicit reveal per view.
This ownership rule belongs in BRN's integration, not in the passive glyph API.

Consume each callback once after the viewer paints; paint the margin card
later in that frame. Convert coordinates by subtracting the margin's window
origin. Clip to the margin. Never retain a prior-frame box if hidden, offscreen,
unmapped, stale or before first layout. Cached ancestors and outer scrollers
need explicit acceptance tests; ordinary headless paint success does not qualify
them. Keep the comment list for navigation and unavailable/history entries.

## Current source seams (reconcile with C before editing)

- `crates/brn-desktop/src/threads_ui.rs`: private selected binding; `open_note`,
  `reload_open_note`, close/version transitions and existing editor Change
  subscription. Geometry must not call `note_changed`, `save`, buffer APIs or
  emit editor Change events.
- `crates/brn-desktop/src/threads_document.rs`: `readable_document`,
  `sync_read_text`, `set_note_reading`, `sync_read_marks`; hook passive report
  and invalidate even when `read_source` bytes are unchanged across documents.
- `crates/brn-desktop/src/threads_writing.rs`: viewer/margin painting and saved
  comment click/refresh; retain comment-list navigation; clear on addressed/
  removal/resolution. Do not alter compose/rewrite/review/Apply paths.
- Actual writing/document/formatting fixtures, currently
  `threads_writing_tests.rs`, `threads_document_tests.rs`, `threads_format.rs`,
  plus affected focus/recovery/checked Apply/Undo fixtures. If extraction moved
  them, port to the current owning shared-writing and desktop test locations.
- Root `Cargo.toml`/`Cargo.lock`: C only, after source approval. No core/app/
  schema changes are expected for geometry.

Preserve `safe_markdown()` and caller-scoped stored-asset resolution, literal
HTML and allowed http(s) link handling for notes, references and proposals.

## Acceptance evidence to port, then review independently

Port the experiment's exact repeated/Unicode/CRLF source mappings, visible glyph
and coordinate conversion, offscreen passive no-scroll, explicit reveal,
resize/rewrap, first paint, same-text document switch, version switch, remove/
reorder/addressed/payload mutation, clean mode round trip, failed replacement
selection, pending reveal cancellation and large async change/revert checks.
Add outline → comment and comment → outline supersession tests, including clear
and rejected selection before the first reveal frame; only the request still
owned by the invalidated context may be cancelled. Use actual Store record IDs and actual document/navigation transitions.

Observe the actual note's text, clean/dirty session, generation/recovery record,
Save eligibility and version, editor Undo/Redo and checked durable Undo before
and after positioning/reveal/clear. Include positive real-input and explicit
Save controls. Rerun existing request/review/Apply, stale/version refusal,
recovery and Undo suites. Experiment editor/Store witnesses are not substitutes
for actual ThreadsUi integration tests. Independent review of that exact
candidate, including C's dependency changes, remains required.

On an accessible unlocked Mac, one consolidated synthetic run must check real
font/fallback/Retina alignment, resize/splits/outer scroll, rapid reveal and
same-text document/version/removal/resolution/mode transitions, indicators,
Save/recovery/Undo, selection/focus/accessibility and preserved link/asset/
reference/proposal behavior. This changes no IME handler or IME qualification.
Until these gates settle, retain the current comment list/reveal main writing
build. No publication, merge or release is assigned.
