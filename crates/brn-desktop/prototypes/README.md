# Ordinary wiki notes UI prototype

Throwaway primary source for the 13 October 2026 ordinary-wiki assignment. No dependencies, persistence, model calls or BRN connection. All content, availability states, changes and drafts are synthetic.

Run from the repository root:

```sh
python3 -m http.server 8768 --bind 127.0.0.1 --directory crates/brn-desktop/prototypes
```

Open [B's prototype](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=B). The floating arrows switch the three structures; left/right keys do the same outside text inputs. `?variant=A`, `B` and `C` survive reload. Drafts and mock Undo stay in memory until reload or Reset demo. Start flow resets navigation and tile layout while retaining drafts.

## Accepted B flow

The owner selected **B — subject overview with evidence chronology**, then **topic pages and search as primary navigation, with a local graph under Connections**. The owner requested the tiled UX of [Omarchy](https://omarchy.org). Its [official navigation manual](https://omarchy.org/manual/navigation/) describes adjacent tiles, focus, moving/resizing and maximization. B applies those interactions using BRN's existing square borders, compact monospace chrome, readable prose and Sammal/Virmalised palettes.

B has three independently scrolling tiles: **Topics & Search**, **Topic page / Subject overview**, and **Evidence / Connections / Changes**. Drag a header to swap tiles, drag a gap to resize, use the header buttons to focus/maximize/close, and Open tiles to restore them. Double-click a header to maximize. ⌘[ / ⌘] focus visible tiles; ⌘⇧Enter maximizes the focused tile; ⌘K reveals search. Narrow windows scroll the tile row, selecting a note reveals its tile, and maximizing provides the available width.

Try these paths:

1. Start on **Harbor project** → **Harbor · current delivery plan**. Read the working account beside evidence dated 1, 8 and 12 October. Inspect a retained passage or the unavailable-original observation.
2. Search **Jaan** → press Enter. The matching context opens the same subject. Search indexes full worked-note text; twelve results per page and exact counts keep All 500 notes bounded. Recent revisits opened notes.
3. Open **Connections** → **Loading requirements**. The graph recenters on that note. Include Sources or expand to depth 2; at most eight neighbors appear, with exact shown/available counts. A readable list preserves recorded directions and relation strings. A source node opens its retained passage.
4. Use the topic breadcrumb or Previous note to return. Open Changes for mock Undo; Write and chat exercise separate memory-only drafts. Context/navigation/tile changes preserve them; a dirty note disables Undo.

Topic pages are ordinary linked Notes, so a subject can appear on several topic pages. There are seven worked Notes (three indexes and four subject/reference pages) and 493 clearly marked placeholders, totaling 500. The graph renders the explicit synthetic links; it supplies no similarity, inferred category, truth or whole-corpus graph. Search is literal matching, with no measured native relevance or performance claim.

The Writing tile exercises draft coordination. The accepted native note remains always editable, with existing Save/Discard and exact review; this does not propose a Read/Edit mode switch. Purpose labels illustrate writing conventions and add no persisted role field. Availability labels describe explicit recorded demo states. Mock Undo illustrates compensation as a new revision; real restoration must use the checked receipt service.

## Preview evidence

Browser checks on 13 October 2026 covered desktop 1440×1000, narrow 720×1000 and the default 374×853 viewport: topic → subject → evidence, remembered-detail search/Enter, graph recentering, source inspection, depth/source controls and the eight-neighbor bound, protection, dirty-note draft preservation, separate thread replies, Undo guards, tile resize/swap/close/reopen and maximized focus. Temporary viewport overrides were reset. This is browser observation, not native app qualification or a usability study.

Independent review found two B state defects: a thread reply could disappear on rerender, and keyboard focus could select an invisible tile while another was maximized. Replies are now retained by destination and focus cycles only visible tiles; scoped re-review found no remaining material findings. Earlier review also corrected incomplete body-text indexing; Tuesday and Medical emergencies find the worked comparison.

Current captures: [topic page](screenshots/variant-B-topic-flow.jpg), [subject and evidence](screenshots/variant-B-subject-evidence.jpg), [local graph](screenshots/variant-B-local-graph.jpg), [search](screenshots/variant-B-tiled-search.jpg), [narrow maximized Connections](screenshots/variant-B-tiled-narrow.jpg), [default topic navigation](screenshots/variant-B-tiled-default.jpg), [default maximized subject](screenshots/variant-B-default-subject.jpg).

## Earlier alternatives

A and C remain comparison alternatives. Earlier captures preserve the preceding sketches, including B's replaced strip/library overlay.

| Variant | Question | Tradeoff |
|---|---|---|
| A — Read with context | Can evidence and changes remain reachable beside readable knowledge? | Reading-focused alternative; context takes a third column at wide sizes and stacks below at narrow sizes. |
| B — Subject overview | Do topic navigation and evidence chronology keep maintained subjects understandable? | Adjacent navigation/context consumes width; tiles can be resized or maximized. |
| C — Connected reading | Does navigating local evidence and related notes clarify what supports a page? | Useful optional context mode; navigation takes space from reading. |

Earlier desktop captures: [A](screenshots/variant-A.jpg), [B](screenshots/variant-B.jpg), [C](screenshots/variant-C.jpg). Narrow: [A](screenshots/variant-A-narrow.jpg), [B](screenshots/variant-B-narrow.jpg), [C](screenshots/variant-C-narrow.jpg). Replaced B library overlay: [search](screenshots/variant-B-search.jpg), [library](screenshots/variant-B-library.jpg), [narrow library](screenshots/variant-B-library-narrow.jpg).
