# Xberg 1.3.10 DOCX native table spans

`0001-docx-table-grid-spans.patch` preserves parsed rectangular DOCX cell spans in the existing semantic native-grid seam. It uses parsed `gridSpan`/`vMerge`, exact source columns and contiguous rows. Equal strings and empty covered slots do not create merges. Real empty origins remain cells.

The body and header/footer paths attach the grid to the exact element index. Dense `Table.cells`, Markdown, wording, styles, page/layer and other public output retain their previous values. Missing grids, ragged rows and unmatched vertical continuations retain the legacy projection. This does not resolve nested-table flattening, omitted layout properties, namespace or other Office gaps.

The third hunk removes an empty attribute envelope only after actually removing the private native-grid key. Existing empty maps without that key and unrelated attributes are preserved. JSON/grid validation and fallback remain unchanged. This shared consumer also serves native PDF; PDF execution was outside this Office-only harness.

## Apply and provenance

Start from the exact published crate with SHA-256 `f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d`. From that package root, apply the single patch in the order recorded in [source-manifest.json](source-manifest.json):

```sh
git apply --check /absolute/path/to/0001-docx-table-grid-spans.patch
git apply /absolute/path/to/0001-docx-table-grid-spans.patch
```

A fresh independent extraction/application reproduced all three compiled source hashes; the remaining 761 files were unchanged. Three blank context lines were normalized for repository whitespace checks; the original raw diff is retained in the external evidence root.

The published package declares **MIT**, authored by Na’aman Hirschfeld and Tobias Perlstein. Its exact archive omits `LICENSE*`; its README references `../../LICENSE` outside the package. The manifest records that boundary and the declaration hash. This directory distributes only the small patch and provenance.

## Verification boundary

The standalone path-dependency harness requests only `office`, defaults disabled; upstream transitively enables `notebook` and `html`. Compiled Xberg has `cfg(test)=false`. No root dependency resolution/adoption or registry source edit occurred.

Six focused geometry/consumer tests pass. The unchanged seventh test retains the known prefix-variation empty-content failure: the final full target is **6 passed / 1 failed**, not all green. Full public-output comparisons pass for 22 DOCX captures and six consumer cases. Strict all-target harness Clippy passes; it does not establish that dependency source was itself linted. See the [source-associated handoff](../../../docs/work/active/modular-foundation/x2/office1/spans1/handoff.md) for exact commands, artifact custody and remaining gates.
