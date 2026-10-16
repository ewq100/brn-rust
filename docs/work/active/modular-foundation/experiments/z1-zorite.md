# Z1 — Zorite editor qualification result

Recorded 16 October 2026 from the pinned handoff sources below. The owner authorized isolated qualification before the broader program started.

**Status:** gate 1 (stock compatibility) passed; gate 2 (headless mapping and read-only checks) failed qualification. Gate 3 (native) was not run.

**Verdict: not adopted, no adaptation. The current editor is retained and production is unchanged.**

## Against the named capabilities

| Capability (master plan section 9, B2) | Outcome |
|---|---|
| (a) Range geometry for margin comments | Not provided by Zorite for BRN's surfaces. Its caret helper measures only its own painted layout. Pursued instead on the current Read viewer (below). |
| (b) Live WYSIWYG Markdown | Not adopted. It would add a second Markdown interpreter beside BRN's consolidated renderer policy. |
| (c) macOS IME/composition | Not evaluated; gate 3 was not run. The report does not recommend reopening it after the negative gate 2. |

## Follow-up: passage/comment positioning on the current viewer

The owner then chose to extend BRN's existing Read viewer instead of adopting Zorite. A reviewed prototype paints one selected comment beside the first rendered glyph of its exact passage and reuses the existing reveal, source map and saved-comment checks. Evidence:

- 21 geometry tests passing;
- 150 existing desktop tests passing;
- independent review complete.

It needs a small passive-layout patch to `gpui-base` 0.7.3 (+145/−6 lines across four toolkit files). Toolkit source ownership is not approved.

It is preserved as **draft PR [#149](https://github.com/ewq100/brn-rust/pull/149) only; the owner instructed not to merge it**. Production wiring, native qualification and the toolkit-patch route remain separate decisions. The [forward build sequence](../../build-sequence/README.md) tracks them as slice PC and decision T.

## Pinned sources

- [Prototype report, including the Z1 status table](https://github.com/ewq100/brn-rust/blob/5f7612c970e1c650e66cb524e6062782ac46e896/experiments/passage-comment-geometry/REPORT.md)
- [Integration handoff](https://github.com/ewq100/brn-rust/blob/5f7612c970e1c650e66cb524e6062782ac46e896/experiments/passage-comment-geometry/HANDOFF.md)
- [Independent re-review](https://github.com/ewq100/brn-rust/blob/5f7612c970e1c650e66cb524e6062782ac46e896/experiments/passage-comment-geometry/review/independent-corrected.md)

The original Z1 qualification is commit `b9bd1a14862fe2efe246790e887797c949f13c50` on `work/z1-zorite`. That branch was not on `origin` when this was recorded. Publish it from the machine that holds it to make the gate details directly linkable.
