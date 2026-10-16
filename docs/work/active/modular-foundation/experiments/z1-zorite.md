# Z1 — Zorite editor qualification result

Recorded 16 October 2026 from the pinned experiment sources below.
The owner authorized isolated qualification before the broader program started.

**Status:** gate 1 (stock compatibility) passed; gate 2 (headless checks) failed qualification;
gate 3 (native) not run.
**Verdict: not adopted, no adaptation. The current editor is retained; production is unchanged.**

## Gate 1 — passed

Unmodified `zorite-editor 0.11.2` and `gpui-kit 0.7.3` compiled together under Rust 1.98.1,
sharing exactly one `gpui-pre 0.3.9` graph. No patches, pin upgrades or Zorite application
dependency were used. The experiment started from snapshot `7e9d0376`; all 85 changed files
are under `experiments/z1-zorite/`.

## Gate 2 — failed qualification

The suite has 11 passing characterization tests (none failed or ignored). Several deliberately
assert defects, so a passing suite does not mean gate 2 passed.

- **Byte offsets: passed.** Keyboard selections mapped exactly on repeated paragraphs,
  Estonian diacritics, combining marks, emoji/ZWJ and CRLF fixtures.
- **Read-only enforcement: failed.** A key/action wrapper blocked typing and Cmd-V, but
  input-handler paste, marked-text callbacks and mouse checkbox interaction still changed text.
- **Rendered/source agreement: failed.** Pointer selection included hidden closing emphasis:
  `formatted**` at `19..30` versus BRN Read's `formatted` at `19..28`; similarly `40..48`
  versus `40..46` on the repeated Estonian fixture.
- **Lossless loading: failed.** `before $$x^2$$ after` gained newlines without a Changed event.
- **Change notification:** `replace_range`, Undo and Redo changed text without Changed;
  `set_text` cleared both histories. BRN draft/generation and Undo integration would need a
  separately qualified adapter.

## Against the named capabilities

| Capability (master plan section 9, B2) | Outcome |
|---|---|
| (a) Range geometry for margin comments | Useful zero-width caret anchor that follows resize/rewrap; not range geometry, and unavailable before paint. |
| (b) Live WYSIWYG Markdown | Failed exact mapping and source-fidelity qualification; would add a second Markdown interpreter (`zorite-markdown`). |
| (c) macOS IME/composition | Marked-text/input-handler API checks passed; real native IME behavior not evaluated (gate 3). |

New dependencies were `zorite-editor 0.11.2`, `zorite-markdown 0.10.0` and `gpui-bidi 0.1.1`, all MIT,
with notices retained. No mandatory GPL/AGPL dependency was introduced.

## Follow-up: passage/comment positioning on the current viewer

The owner then chose to extend BRN's existing Read viewer instead. A reviewed prototype paints one
selected comment beside the first rendered glyph of its exact passage. It needs a small passive-layout
`gpui-base` 0.7.3 patch, and is preserved as **draft PR [#149](https://github.com/ewq100/brn-rust/pull/149)
only; the owner instructed not to merge it**. Production wiring, native qualification and the
toolkit-patch route are slice PC and decision T in the [forward build sequence](../../build-sequence/README.md).

## Not established

Editor adoption, production integration, native acceptance, full Markdown parity, or complete
geometry, clipboard and IME behavior. Gate 3 is not recommended as an adoption gate while the
source-identity and read-only failures stand.

## Pinned sources

- [REPORT.md](https://github.com/ewq100/brn-rust/blob/b9bd1a14862fe2efe246790e887797c949f13c50/experiments/z1-zorite/REPORT.md)
- [Qualification tests](https://github.com/ewq100/brn-rust/blob/b9bd1a14862fe2efe246790e887797c949f13c50/experiments/z1-zorite/tests/qualification.rs)
- [Evidence (commands, outputs, dependencies, notices)](https://github.com/ewq100/brn-rust/tree/b9bd1a14862fe2efe246790e887797c949f13c50/experiments/z1-zorite/evidence)
- [Positioning prototype report (#149)](https://github.com/ewq100/brn-rust/blob/5f7612c970e1c650e66cb524e6062782ac46e896/experiments/passage-comment-geometry/REPORT.md)
