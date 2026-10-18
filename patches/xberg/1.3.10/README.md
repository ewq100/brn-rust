# Xberg 1.3.10 DOCX native table spans

`0001-docx-table-grid-spans.patch` preserves parsed rectangular DOCX cell spans in the existing semantic native-grid seam. It uses parsed `gridSpan`/`vMerge`, exact source columns and contiguous rows. Previous-row matches use a monotone cursor over source columns, with linear lookup traversal per row. Equal strings and empty covered slots do not create merges. Real empty origins remain cells.

The body and header/footer paths attach the grid to the exact element index. Dense `Table.cells`, Markdown, wording, styles, page/layer and other public output retain their previous values. Missing grids, ragged rows and unmatched vertical continuations retain the legacy projection. This does not resolve nested-table flattening, omitted layout properties, namespace or other Office gaps.

The third hunk removes an empty attribute envelope only after actually removing the private native-grid key. Existing empty maps without that key and unrelated attributes are preserved. JSON/grid validation and fallback remain unchanged. This shared consumer also serves native PDF; PDF execution was outside this Office-only harness.

## Apply and provenance

Start from the exact published crate with SHA-256 `f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d`. From that package root, apply the single patch in the order recorded in [source-manifest.json](source-manifest.json):

```sh
git apply --check /absolute/path/to/0001-docx-table-grid-spans.patch
git apply /absolute/path/to/0001-docx-table-grid-spans.patch
```

The original fresh-archive application is retained. The corrected patch was independently applied to a copy of that verified pristine source and reproduced all three compiled hashes. Only the parser lookup changed from the first candidate. Three blank context lines are normalized for repository whitespace checks; both raw diffs and the old patch remain retained.

The published package declares **MIT**, authored by Na’aman Hirschfeld and Tobias Perlstein. Its exact archive omits `LICENSE*`; its README references `../../LICENSE` outside the package. The manifest records that boundary and the declaration hash. This directory distributes only the small patch and provenance.

## Verification boundary

The standalone path-dependency harness requests only `office`, defaults disabled; upstream transitively enables `notebook` and `html`. Compiled Xberg has `cfg(test)=false`. No root dependency resolution/adoption or registry source edit occurred.

The corrected full target is **7 passed / 1 failed**: all geometry/consumer cases pass, while the unchanged prefix-variation namespace assertion still fails. The new focused case passes with seven tests filtered and checks independently changing mixed-width chains plus 4,096 parallel chains; it is functional evidence, not a benchmark or performance-red claim. Full comparisons pass for 22 DOCX captures and six consumer cases with the original tolerances, and all 28 complete values equal the first candidate. Strict all-target harness Clippy passes; dependency-wide lint is not established. See the [correction handoff](../../../docs/work/active/modular-foundation/x2/office1/spans1/opus1/handoff.md). The [original handoff](../../../docs/work/active/modular-foundation/x2/office1/spans1/handoff.md) retains its actual 1/6 to 6/1 history. Fresh independent round-two review is pending.
