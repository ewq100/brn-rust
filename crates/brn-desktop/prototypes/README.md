# Ordinary wiki notes UI prototype

Throwaway primary source for the 13 October 2026 ordinary-wiki assignment. No dependencies, persistence, model calls or BRN connection. All content, availability states, changes and drafts are synthetic.

Run from the repository root:

```sh
python3 -m http.server 8768 --bind 127.0.0.1 --directory crates/brn-desktop/prototypes
```

Open [the prototype](http://127.0.0.1:8768/ordinary-wiki-notes.prototype.html?variant=A). The floating arrows switch three structures; left/right keys do the same outside text inputs. `?variant=A`, `B` and `C` survive reload. Drafts and mock Undo stay in memory until reload or Reset demo.

| Variant | Question | Tradeoff |
|---|---|---|
| A — Read with context | Can evidence and changes remain reachable beside readable knowledge? | Best default; context takes a third column at wide sizes and stacks below at narrow sizes. |
| B — Subject overview | Does a visible evidence chronology make disputed/older evidence easier to understand? | Useful for history-heavy subjects; more vertical space before the account. |
| C — Connected reading | Does navigating local evidence and related notes clarify what supports a page? | Useful optional context mode; navigation takes space from reading. |

Recommendation: A as the default, with B's chronology available in evidence details and C's local connections as an optional view. This is an engineering recommendation, not owner acceptance. No winner has been selected or native UI changed.

The drawer's Edit control only exercises draft coordination. The accepted native design remains always editable, with the existing Save/Discard and exact review. Purpose labels illustrate writing conventions; they propose no persisted role field. Source availability labels describe explicit demo states, not inferred freshness or truth. The mock Undo illustrates a new compensating revision; actual content restoration must use the checked receipt service.

Browser preview on 13 October 2026 checked all variants at 1440×1000 and 720×1000, evidence passages, unavailable original, protected note navigation, conflict thread, dirty draft preservation, arrow keys inside a textarea and disabled Undo while dirty. Default viewport restored afterward. This is browser prototype observation, not native app qualification.

Desktop screenshots: [A](screenshots/variant-A.jpg), [B](screenshots/variant-B.jpg), [C](screenshots/variant-C.jpg). Narrow screenshots: [A](screenshots/variant-A-narrow.jpg), [B](screenshots/variant-B-narrow.jpg), [C](screenshots/variant-C-narrow.jpg).
