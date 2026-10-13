# Ordinary wiki notes UI prototype

Throwaway primary source for the 13 October 2026 ordinary-wiki assignment. No dependencies, persistence, model calls or BRN connection. All content, availability states, changes and drafts are synthetic.

Run from the repository root:

```sh
python3 -m http.server 8768 --bind 127.0.0.1 --directory crates/brn-desktop/prototypes
```

Open [the prototype](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=B). The floating arrows switch three structures; left/right keys do the same outside text inputs. `?variant=A`, `B` and `C` survive reload. Drafts and mock Undo stay in memory until reload or Reset demo.

| Variant | Question | Tradeoff |
|---|---|---|
| A — Read with context | Can evidence and changes remain reachable beside readable knowledge? | Reading-focused alternative; context takes a third column at wide sizes and stacks below at narrow sizes. |
| B — Subject overview | Does a visible evidence chronology make disputed/older evidence easier to understand? | Useful for history-heavy subjects; more vertical space before the account. |
| C — Connected reading | Does navigating local evidence and related notes clarify what supports a page? | Useful optional context mode; navigation takes space from reading. |

Owner selection, 13 October 2026: **B — Subject overview with evidence chronology** guides the later native UI pass. Native UI has not changed. A and C remain alternatives in this primary-source prototype. The owner then asked how hundreds of notes should be navigated and requested research into graphs and Obsidian-style categories.

The drawer's Edit control only exercises draft coordination. The accepted native design remains always editable, with the existing Save/Discard and exact review. Purpose labels illustrate writing conventions; they propose no persisted role field. Source availability labels describe explicit demo states, not inferred freshness or truth. The mock Undo illustrates a new compensating revision; actual content restoration must use the checked receipt service.

Browser preview on 13 October 2026 checked all variants at 1440×1000 and 720×1000, evidence passages, unavailable original, protected note navigation, conflict thread, dirty draft preservation, arrow keys inside a textarea and disabled Undo while dirty. Default viewport restored afterward. This is browser prototype observation, not native app qualification.

Desktop screenshots: [A](screenshots/variant-A.jpg), [B](screenshots/variant-B.jpg), [C](screenshots/variant-C.jpg). Narrow screenshots: [A](screenshots/variant-A-narrow.jpg), [B](screenshots/variant-B-narrow.jpg), [C](screenshots/variant-C-narrow.jpg).

B now replaces its horizontal note strip with a bounded library browser containing 500 synthetic entries (four worked examples and 496 navigation placeholders). Search title/text, inspect snippets, open a result with Enter, revisit Recent, browse recorded demo relationships or protected notes, and filter All A–Z by starting letter. Twelve results are shown per page with exact counts. ⌘K opens the browser. Search uses simple literal matching; neither its relevance nor native large-workspace performance is qualified by this demo.

Follow-up browser checks cover paging (1–12 / 13–24), searching a remembered detail (Jaan), opening with Enter, ⌘K, linked/protected views, letter filtering and no results. New screenshots: [search](screenshots/variant-B-search.jpg), [library](screenshots/variant-B-library.jpg), [narrow library](screenshots/variant-B-library-narrow.jpg). The 500-entry library was inspected at the default browser size and at 720×1000; viewport override reset afterward.

Independent follow-up review found that abbreviated search summaries missed text such as Tuesday and Medical emergencies in the worked comparison. The demo now indexes full worked note bodies and displays a matching-context snippet separately; both remembered-detail searches and Enter opening were browser-verified after correction.
