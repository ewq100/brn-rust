> Historical investigation/qualification below. Current corrected reference,
> independent review and pending integration/native gates are recorded in
> [INTEGRATION-REVIEW.md](INTEGRATION-REVIEW.md). Original Z1 is unchanged.

# Passage/comment positioning: current BRN editor and viewer

**Handoff completed, 15 October 2026:** the owner selected extending the existing
Read viewer with no Zorite adoption. The [current integration handoff](HANDOFF.md)
records qualified code commit `f64a9d50766428ac162d1e2e2d5d7db4c875bb3c`,
**14 passing headless tests**, exact changed files/patches and reconciliation
against fetched main `8bef439b133e91ad68b0b76792bcd155d3d06237`.
Document-switching, a directly armed first-paint probe and a connected
editor-to-Store dirty/generation/Save/Undo witness complete the missing bounded
checks. Native observation, independent review and production wiring remain
pending. The original 11-test prototype report below describes commit
`26b029756e3f20a3d9f0a7273230ab6cecee756f`; Z1 remains unchanged and separate.

**Result:** a useful missing capability is demonstrated on BRN's existing
Read viewer: a selected comment can be painted at its exact passage's first
rendered text glyph, and comment selection reveals the intended occurrence.
The existing reveal, source mapping and saved-comment checks are reused.
A small passive-layout extension to the current toolkit is needed; no Zorite
code, dependency, editor replacement or second Markdown interpreter is used.

This is an isolated prototype and review proposal, not integrated BRN behavior.
The original Z1 stock-compatibility pass and negative full-editor qualification
remain unchanged on `work/z1-zorite`, commit
`b9bd1a14862fe2efe246790e887797c949f13c50`. In particular this follow-up does not
make Zorite pass gates 1–2 or authorize its native gate 3.

## Baseline and authority

Fetched `origin/main` again for this assignment; snapshot:
**`7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`**. Created
`work/passage-comment-geometry` directly from that ref, in
`/Users/evokessler/Documents/Codex/brn-rust/.worktrees/passage-comment-geometry`.
The originally supplied `/Users/evokessler/repos/brn-rust` was absent; the
registered checkout above has remote `https://github.com/ewq100/brn-rust.git`.
The earlier Z1 implementation commit was evidence, not the integration base.

Read current AGENTS/status, architecture behavior/invariants, development
workflow/verification, writing-polish plan/spec, modular-foundation README,
master plan (B2, experiments, ownership/integration), and writing build prompt.
The stored pack defers experiments until ui-polish and daily-workflow-reliability
land. The owner's 15 October isolated-experiment override and this follow-up
request authorize this prototype only. No canonical C0 active `plan.md` or
C integration ref was present at this snapshot. Neither other stream was
modified or treated as released. Integration must refresh C's ref and ownership.

Only `experiments/passage-comment-geometry/` changes on this branch. Shared
production files, root manifests/lockfile, the main checkout's untracked
experiments, other worktrees and Z1 remain untouched. No provider/model calls,
model downloads, installs, real documents, interactive GUI/computer use,
publishing, push, PR, merge or release occurred.

## What already exists, and the gap

Paths/line numbers below describe the recorded baseline, before the patch.

| Capability | Existing implementation | Finding |
|---|---|---|
| Write source selection | `threads_ui.rs::track_selection` (~1324), current `EditorState::selected_range` | UTF-8 source byte range; no replacement required. |
| Read source selection | Same method; `TextViewState::selected_source_range` | Existing viewer maps rendered selection to original Markdown, guarded by rendered source == editor source. |
| Exact saved comment identity | `threads_ui.rs::comment_on_selection` (~1355), `threads_state.rs::comment_location`, core `writing.rs::exact_writing_range` (~149) | Requires saved note/version, range and exact source quote; mapped metadata is used only when valid, unresolved stays unavailable. Dirty comment creation already requires Save. |
| Read highlights | `threads_document.rs`, `RenderedText::range_for_source`, `set_range_highlights` | Already implemented using the viewer's parser/source map. Delimiters add no rendered text. |
| Comment click reveal | `threads_writing.rs` (~1370) → `threads_ui.rs::reveal` (~1412) | Read maps source range then calls existing `reveal_range`; Write sets current editor selection/focus. Already implemented. |
| Current margin | `threads_writing.rs::margin_column` (~1235–1445) | Cards sort by source start in a separately scrolling column. Their vertical positions do not track passage layout. This is the user-visible gap. |
| Viewer geometry internals | gpui-base `text/{range_highlight,inline,state,text_view}.rs` | Reveal already resolves text leaves and actual shaped glyph boxes. Public `on_reveal` reports a laid-out hidden target to support scrolling; it does not continuously report visible passage geometry. Public `bounds()` is the whole view. |
| Editor geometry | gpui-base `input/base/state.rs` (~626, ~4507) | Hidden/documentation-internal active-caret helpers are not an arbitrary saved-range API. `EntityInputHandler::bounds_for_range` takes UTF-16, searches visible layout, can fall back to the current caret, and forces end onto the start line. Source inspection rules it out as reliable arbitrary-passage positioning; no new native IME claim is made. |

The renderer/link/asset policy is in `threads_ui.rs::safe_markdown` (~253–302):
literal HTML plugins, caller-visible stored assets only, lowercase absolute
http(s) allowlist. The host uses those same plugins/policy with an empty
synthetic asset map. References and proposal review keep their existing viewer;
no production rendering or input policy changed.

## Selected implementation and maintenance cost

1. **Supported reuse:** retain exact core validation, the current source map,
   highlight API, and existing reveal. No reimplementation of those behaviors.
2. **Small extension:** expose the existing viewer's glyph measurement passively.
   [The review patch](patches/gpui-base-0.7.3-range-start.patch) adds 145 lines
   and removes six across four toolkit files. No new parser or layout algorithm.
3. **Selective Zorite reuse:** none is worthwhile for this capability. Zorite's
   caret helper measures its own painted layout and would not give coordinates
   for BRN's current viewer. Copying its scanner/buffer/input/Undo adds maintenance
   and a second interpretation without solving the current surface's geometry.

The experimental API is `TextViewState::track_range_start(rendered_range, cx)`
and `TextView::on_range_start_layout(handler)`. It reports a **passage-start
text glyph box**, with line height, in GPUI **logical window coordinates** after
paint. It is neither an insertion caret nor full-range/line-fragment geometry.
For source spanning formatting, its anchor is the first rendered text glyph
covered by that source range, not the invisible Markdown delimiter.

Only one active probe is supported. It is tied to the exact rendered snapshot,
cleared each frame, never remapped after source changes, and never autoscrolls.
Empty/separator/non-text requests cannot borrow another leaf/block, and a
newline with no glyph cannot fall back to the nearest glyph. Offscreen,
clipped, unlaid-out, changed-source, and max-lines-clamped views yield no
position. Explicit text reveal keeps priority and the passive anchor can be
unavailable while reveal settles; the host must tolerate that.

[src/lib.rs](src/lib.rs) hosts BRN's actual current component types, with a
synthetic toolbar, one active margin card, exact core `Comment` validation and
a derived Read view. It is not a full `ThreadsView`/Workspace integration.
A canvas paints after the viewer in the same frame: subtract margin origin
from window-space glyph origin, then paint at the margin's x and passage's y.
The callback result is consumed once so hidden/skipped viewer paint cannot
reuse the previous frame's position. Unavailable shows an explicit message.
A source observer also invalidates silent programmatic editor changes; it
does not synthesize Change events or increment the host's event counter.

This is smaller than replacing the editor but still creates toolkit maintenance:
review/rebase four internal-layout touchpoints on an upgrade. Prefer upstreaming
a supported API; do not quietly install this local fork in production. Exact
upstream revision, files, archive checksum and Apache-2.0 notice are in
[UPSTREAM.md](UPSTREAM.md). No additional external crates or GPL/AGPL code were
introduced. This is not a fresh license audit of every existing BRN dependency.

## Verified behavior

Initial-prototype [headless output](evidence/tests-final-12.stdout): **11 tests passed**,
zero failed/ignored; tests execute GPUI's test platform on this Mac, not a real
Mac window/text system. Numeric measurements below are test-platform results.

| Check | Evidence / outcome |
|---|---|
| Repeated exact occurrence | Two identical paragraphs; second stored source range maps to second rendered occurrence. First/second glyph y = 56/97 logical pixels. Quote search is used only to construct synthetic fixtures, never to resolve a stored comment. |
| Unicode and formatting | Estonian õ/ÕÄÖÜ/šž, combining `ä`, emoji/ZWJ/skin tone, CRLF, emphasis, link text and inline code. Source bytes unchanged; rendered range matches the expected text. Also reuses Café/formatting fixtures from BRN document/format tests. |
| First layout | No invented initial position. Registration clears prior output before layout. Current geometry comes only from actual shaped glyph reporting. |
| Resize, wrapping, font | Preceding prose wraps: target y 227 → 435 after width 1000 → 620; larger text gives y 737. Shorter viewport removes the card; explicit reveal yields current y 247. |
| Coordinate conversion | Nonzero document/margin origins; assert local point = glyph origin − margin origin, card window y == glyph y, and card x == margin x, in the same paint. |
| Scroll and reveal | Passive second/offscreen target returns None and leaves list offset `(0, 0)` unchanged. Actual comment-2 button reveals later occurrence: offset `(22, 56px)`; selecting comment 1 returns toward start `(1, 0)`. Scrolling it offscreen clears card and anchor. |
| Markdown layout routes | Nested quote/list, code block, table cells and literal block HTML policy. Equal table-cell text has distinct x coordinates (29/327); correct source occurrence is preserved. |
| Stale invalidation | Source edit/reparse invalidates even if old highlights can remap. Reverting source still requires validated re-registration. Real editor typing and silent `set_value` both invalidate. Dirty text refuses geometry/reveal. |
| Invalid/unavailable | Wrong note, version, quote/range, unresolved comment, source-only delimiters, empty/separator/newline requests all give unavailable. No fuzzy relocation or active-caret fallback. |
| No text/Undo side effects | Repeated comment display/reveal leaves editor value and Change-event counter unchanged. Pristine Undo stays empty. A real typing edit retains normal Undo/Redo across Read/comment operations. Silent programmatic loading produces no fabricated draft event. |
| Checked Save and recovery | Real synthetic `Store`: records, receipts, recovery buffer and generation unchanged by display/resize. Explicit stale-generation Save is refused. Only explicit checked Save remaps second occurrence 6..10 → 13..17; overlapping Save marks it unresolved. Geometry contains no storage writes. |

The host event counter is a deterministic witness, **not** a replacement for
BRN's persisted draft-generation implementation. The synthetic Store is checked
alongside the host; the full desktop's Save/recovery/Apply integration is pending.
Source identity, checked Apply and durable Undo implementations were not changed.

## Versions, commands and status

Rust **1.98.1**; gpui-kit/gpui-base/gpui-component **0.7.3**; one gpui-pre **0.3.9**
graph; current viewer `markdown` **1.0.0**, editor `ropey` **2.0.0-beta.1**.
The experiment starts with a copy of root Cargo.lock. Resolution prunes unused
packages and changes gpui-base's source to the isolated patched path; every
shared package keeps its root version. The only new package identity is the
local experiment. See [audit](evidence/scope-final.stdout) and
[GPUI tree](evidence/gpui-graph.stdout).

Setup commands (repository root, no production edits):

```sh
git fetch origin
git worktree add -b work/passage-comment-geometry .worktrees/passage-comment-geometry origin/main
cd .worktrees/passage-comment-geometry
cp Cargo.lock experiments/passage-comment-geometry/Cargo.lock
```

Build/test/bootstrap commands and runner instructions are in [README.md](README.md).
Exact executed argument vectors, environment, UTC timestamps, durations, exit
codes and build contention snapshots are in [commands.jsonl](evidence/commands.jsonl).
Final successful commands:

```sh
python3 experiments/passage-comment-geometry/prepare-toolkit.py
cargo build --locked --manifest-path experiments/passage-comment-geometry/Cargo.toml
cargo test --locked --manifest-path experiments/passage-comment-geometry/Cargo.toml -- --nocapture
cargo clippy --locked --manifest-path experiments/passage-comment-geometry/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path experiments/passage-comment-geometry/Cargo.toml -- --check
cargo tree --locked --manifest-path experiments/passage-comment-geometry/Cargo.toml -i gpui-pre
python3 experiments/passage-comment-geometry/audit.py
```

All use the installed pinned compiler and
`CARGO_TARGET_DIR=/private/tmp/brn-passage-geometry-target` through `run.py`.
The patch was applied successfully to a fresh checksum-verified source copy,
then compiled/tested. The final whitespace-clean patch also reproduces all four qualified source files byte-for-byte; see [source checksums](evidence/qualified-toolkit-sources.json) and [reproduction check](evidence/reproduced-sources.txt). First cold compilation attempt took 49.08 seconds and
failed on a missing host trait import. Successful rematerialized-toolkit build
used the cached target (12.37 seconds); the final host build is incremental.
These are compilation observations, not UI/performance benchmarks.

Development failures remain in earlier logs: host imports/test type errors,
GPUI's eager test-frame scheduling, synthetic Store ID/revision prerequisites,
and a mistaken assumption that `insert` is silent (the final silent-load test
uses documented `set_value`). They are superseded by the final 11-test run.
Cargo reports the existing `block 0.1.6` future-incompatibility warning. No pin
was upgraded or warning suppressed to remove it.

| Status | Result |
|---|---|
| Prototype implemented | Passed: real existing components, passive single-card positioning + existing reveal. |
| Deterministic verification | Passed: 11 headless tests, build, strict own-target Clippy, format and graph/scope audit. |
| Stock full-editor Z1 | Preserved: gate 1 passed, gate 2 failed qualification; no adoption/adaptation rerun. |
| Native Mac geometry/selection/IME | Pending; no app launch or native observation. |
| Production desktop integration / independent review / owner acceptance | Pending; isolated patch and wiring host only. |
| Publication / merge / release | Not performed. |

## Recommendation and remaining limits

Run a bounded **native qualification of this current-viewer extension** after
toolkit/writing-owner review. It is worth doing: it closes a real margin-position
gap with exact current source identity and existing reveal, without changing
input, Markdown policy or storage. This does not recommend reopening Zorite's
full-editor gate 3 based on the previous negative qualification.

The prototype demonstrates one active card in Read. It does not solve multiple
card collisions, all visible comment positions, offscreen estimated anchors,
Write-view arbitrary-range geometry, accessibility/focus polish or general
outer-scroll/virtualized-container integration. Large-document performance,
real font shaping/fallback, Retina scale, partial clipping, rapid reveal/edit
races and native selection still need observation. A max-lines view deliberately returns no geometry.
Do not treat a reveal request's acceptance as proof that native reveal completed.

Live WYSIWYG editing remains a separate future capability; no editor/scanner
transplant is warranted by these results. macOS IME remains on BRN's existing
input handler and has no new native qualification here. The writing-owner
[handoff](HANDOFF.md) identifies owned paths, dependency approval, integration
prerequisites and one consolidated native/regression checklist.
