# Visual foundations

All values on this page are **implemented** and generated from
[`crates/brn-desktop/src/tokens.rs`](../../../crates/brn-desktop/src/tokens.rs)
by `scripts/design-handbook.py`. Change values there, then run
`python3 scripts/design-handbook.py tokens`; a unit test fails while the
generated files are stale. The theme mapping onto toolkit components is in
[`native/theme.rs`](../../../crates/brn-desktop/src/native/theme.rs).

## Colour

Two schemes, Dark (default) and Light, follow macOS appearance unless Settings
overrides it. Surfaces are near-neutral blue-greys; six **semantic tones** carry
meaning:

| Tone | Token | Means | Examples |
| --- | --- | --- | --- |
| Neutral | `muted` | ordinary metadata, inactive, History | section labels, kind of Inbox copy, History badge |
| Info | `cyan` | focus, links, evidence, the primary action | focus ring, Source badge, *Ask*, *Save*, *Approve* |
| Attention | `amber` | needs a look; changed; waiting | Draft proposals, follow-up, review counts, conflicts warnings in callouts |
| AI | `purple` | AI-generated or provisional | “BRN” label, streaming badge, suggestions |
| Success | `green` | approved current knowledge; saved; done | Ready, Applied, Completed, “Current · editable” |
| Danger | `red` | failure, destructive, overdue | errors, Trash members, overdue, Stop |

Rules:

- **Never colour alone.** Every tone appears with a word (badge text, label) or
  a symbol. Badges always contain text.
- **One filled control per region.** Filled cyan is reserved for the primary
  command; danger fill only for Stop and destructive confirmations.
- **Tints, not fills, for state.** Badges and callouts use the tone at ~14% / 7%
  over the surface so text stays readable in both schemes.
- **Purple is never used for approved knowledge**, and green never for anything
  AI-generated that is not yet approved.

## Typography

- **UI font** (`.SystemUIFont`, SF Pro on macOS): chrome, labels, buttons, chat,
  prose. Default 13 pt; reading text 14 pt with 21–22 pt line height; view
  titles 17 pt semibold; empty-state headline 22 pt.
- **Mono font** (Menlo): only exact identity — paths, UUIDs, model ids, versions,
  byte ranges, counts in metadata lines — and Markdown source editors, where
  exact bytes matter.
- **Section labels**: 11 pt semibold uppercase muted. Use them to name groups;
  do not use bold body text as a heading.
- Keep line length near 760 pt for reading (`size::READING_MAX_WIDTH`).

## Space, size and shape

- 4-point rhythm: 4 / 8 / 12 / 16 / 24. Panes use 16 horizontal, 12 vertical
  padding; groups are separated by a section label rather than borders.
- Rows are 28 pt (40 pt with a detail line); the status strip is at least 26 pt.
- **Square corners** (`radius 0`) and 1 px lines, per the approved direction.
  Accent bars are 2 px (callouts, selected rows in prototypes).
- No drop shadows except the toolkit's modal overlay.

## Icons

Lucide icons from the toolkit's embedded default set (about 100 icons; others
render blank). Use an icon only beside a label or with a tooltip; never as the
only carrier of meaning. Current assignments: Plus (create), LayoutDashboard,
Inbox, Bell (Needs Review), GalleryVerticalEnd (Activity), Settings, RotateCw
(refresh), Network (relationships), Search, ArrowUp (Ask), Chevron (disclosure).
Embedding more icons is a dependency-feature decision; see
[open preferences](open-questions.md).

## Token reference (generated)

{{#include generated/tokens.md}}
