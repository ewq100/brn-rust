# Focused P2 BetterOffice API check — 2026-10-07

**Result: use the existing structured/Markdown export API instead of P1's manual
text mapper, conditional on the measured graph cost and explicit fit limits.**
Pinned published 0.3.0 already supplies this API. No new release, parser, renderer,
production adapter, root dependency change, CI change or live/private input was
selected. This directory is a standalone evaluation, not production readiness.
P1 originals and evidence remain unchanged. The parent specification owns adoption.

## Exact API and source basis

Published crates.io source archives (checksums in this directory's Cargo.lock), not
GitHub main claims, were inspected:

- [`betteroffice-docx-edit 0.3.0`](https://docs.rs/crate/betteroffice-docx-edit/0.3.0/source/src/structured/mod.rs),
  `structured/mod.rs:745-789`: `export_docx_structured(bytes,options)`,
  `export_docx_markdown(bytes,options)`, `export_package_structured(wire,parts,options)`.
  `structured/markdown.rs:21` exports `render_docx_markdown(content,options)`.
- [`betteroffice-docx 0.3.0`](https://docs.rs/crate/betteroffice-docx/0.3.0/source/src/document.rs),
  `Document::{open_with_limits,export_structured,export_markdown}` at lines57,216,225;
  facade exports those contracts from `src/lib.rs:19-25`. Its default features are
  empty; raster is optional. The facade nevertheless unconditionally depends on
  edit and layout. A warm facade build and equality with bytes exports passed.
- [`betteroffice-docx-parse 0.3.0`](https://docs.rs/crate/betteroffice-docx-parse/0.3.0/source/src/s9.rs)
  `parse_docx_s9_wire_parts_with_limits(bytes,options,limits)` at line198 returns
  typed `S9WireEnvelope` and retained parts. `Image` in `src/image.rs:109-128`
  already has separate `id/name/alt/title/relationship_id`; the exporter does not
  expose separate picture title/docPr identity.
- [`betteroffice-opc 0.3.0`](https://docs.rs/crate/betteroffice-opc/0.3.0/source/src/lib.rs)
  `unzip_parts_with_limits(bytes,max_expanded_bytes)` at line86 replaces the
  package read loop: actual bounded reads, normalized path/duplicate rejection,
  fixed5000 entry ceiling, caller-tightened expanded byte ceiling. ZIP8.6.0 is
  already in the graph. No bespoke ZIP grammar is needed.

The [fresh sparse index version list](results/published-versions.json) ends at
0.3.0; no newer published facade was available to check for the demonstrated
prefix-resolution gap. The crates.io JSON endpoint returned403; fresh Cargo
registry resolution supplied the pinned source and release list. No source-build
or library survey followed.

## Demonstrated output and limits

[Oracle](results/oracle.json) passes for retained public P1 `harbor.docx`:

- [Structured](results/harbor/structured.json) has ordered body/header/footer,
  table cells (two distinct Pending cells), correct owner/date and budget cap.
  Two image records have different snapshot range anchors and the same resolved
  `word/media/image1.png` part/relationship. Their exact asset bytes are available
  from OPC parts; [package identity](results/harbor/package.json) matches P1 PNG.
- [Markdown](results/harbor/markdown.json) provides readable body/header/footer,
  table and18 marker-to-anchor mappings. It intentionally renders image links with
  empty URLs and emits `image-data-omitted`: bytes must be joined separately.
  Snapshot paragraph/range anchors are not immutable original XML addresses or
  stable identities across later re-extraction. Export IDs are deterministic tree
  paths, explicitly not document identities (`structured/mod.rs:72`). Retain them
  under the exact source hash and extraction snapshot, not as global identity.
- [Synthetic feature variation](results/features/structured.json) preserves a
  `Title` paragraph's text and `styleId=Title`; Markdown leaves it a plain paragraph.
  A second title-only picture exports title as alt fallback. When both descr and
  title exist, only descr is exported in `altText`; separate title is lost at this
  API. A native chart copied from P1's producer PPTX has an unsupported `c:chart`
  inline anchor and specific diagnostic; chart data/rendering is not exported.
  This constructed DOCX chart variation is an API omission witness, not an
  independently rendered Office fidelity fixture.
- [Prefix variation](results/prefix-variation/structured.json) changes only `w` and
  `pic` prefixes in retained body XML; `fixtures.py` asserts namespace-expanded
  element/attribute trees are identical. Both image anchors survive, but their
  relationship IDs become empty and targets unresolved; two specific
  `unresolved-reference` diagnostics appear. Extra `xw:tcPr`/`xw:sectPr` diagnostics
  also appear. Thus0.3.0 is not qualified for arbitrary equivalent namespace
  spelling. Do not guess a replacement association or silently call this complete.

The direct bytes API and `parse_docx_s9_wire_parts_with_limits` →
`export_package_structured` → `render_docx_markdown` route produce exactly equal
exports on all three inputs (assertions in main.rs). The latter can replace
P1's own recursive text mapping without a second semantic parse, and keep the
same parsed package for assets/metadata. ParseLimits in the check tighten XML
bytes/events/depth. There is no public S9 parse entry accepting already-inflated
parts: `parse_s9_package` is private. A separate OPC actual16MiB admission pass
therefore repeats inflation before S9 parsing of the same immutable bytes.
`export_package_structured` seeds a private Yrs editing session; it is not a
lightweight standalone exporter with editing dependencies feature-gated away.

## Remaining bounded responsibilities

| Required outcome | Demonstrated gap | Bounded responsibility / limit |
| --- | --- | --- |
| All selected stories useful | Defaults export body alone | Set explicit Body/Headers/Footers/Footnotes/Endnotes/Comments selection; retain diagnostics/truncation and chosen revision view. No story XML interpreter. |
| Exact useful image bytes at distinct occurrences | Export carries part/relationship and anchors, no bytes; Markdown URLs empty | Join resolved part to retained OPC bytes, validate decoded image/pixel budgets, store exact asset plus distinct snapshot occurrence. Use structured records for evidence display or checked asset URLs; no guessed title/range or same-hash occurrence join. |
| Picture title correctly associated | Structured alt collapses title; lower typed Image has title but exact export-occurrence identity join was not demonstrated | Preserve upstream alt; show separate-title unavailable until an exact typed/source join or upstream export field is qualified. No new general XML walker and no nth-picture/blob guessing. Do not promise P1 title association from this API alone. |
| Equivalent namespace input does not silently misassociate | Prefix-only variant loses relationship IDs/targets | Treat shown diagnostics as partial/unsupported and retain originals. No custom namespace repair or picture inventory/walker is selected by this check. Upstream fix needed for full prefix support; any later guard needs a demonstrated gap and separate qualification. |
| Material omitted chart visible | Chart becomes unsupported inline | Surface diagnostic+snapshot anchor and original inspection. No chart/Word/DrawingML renderer; chart factual content unavailable. |
| BRN <=512members and <=16MiB actual expanded bytes | OPC cap5000; structured bytes API uses default512MiB package ceiling | Maintained ZIP count preflight plus OPC16MiB actual-read admission on immutable bytes. [Guards](results/guards.json) prove513 accepted upstream, inflation/path rejection at OPC16MiB; avoid copying binary ZIP grammar. Duplicate/path normalization belongs to OPC. |
| DTD/XML/output bounded | OPC does not inspect XML; byte export uses default ParseLimits; export limits may truncate | Select parser ParseLimits and package export; selected parsed parts reject DTD. Unparsed parts remain opaque retained bytes, not qualified XML; no blanket custom XML walker is justified by this check. Any later parsing must use qualified upstream safe XML facilities. Export max_bytes/max_blocks and host total-output/time/input caps still required. |
| Source/asset freshness and durable citations | Export anchors snapshot-scoped; no BRN lifecycle | Existing private capture/hash/snapshot/workflow records own binding and exact approval. Not another converter or approval authority. |

[Guard witness](results/guards.json): traversal rejected both paths;17MiB inflation
rejected by OPC16MiB but default structured export reaches XML parsing first;
513members accepted; DTD rejected by DOCX parser. This is not a full adversarial
corpus, complete image-decoder qualification or proof every unparsed XML part is
checked. Helper sandbox/process lifetime and review UI remain P1/spec concerns.

## Total graph and integration cost

[Measured costs](results/costs.json), [enabled features](results/tree-features.txt)
and [metadata/checksums/licenses](results/metadata.json) use arm64 normal registry
packages (optional/target lock-only entries excluded). This standalone leaf check
has86 enabled packages versus P1's53:39 exactpairs new versus P1 (six P1-only
packages removed);14 new exactpairs versus the root lock. Source archives total
9,189,151bytes versus6,082,560; incremental archives versus P1 are4,323,699bytes.
Facade adds one package/60,693archive bytes:87 packages,40 new exactpairs versus
P1 and15 versus root lock. Root comparison is against its lock, not its enabled
graph. Yrs, layout, font shaping/measurement and associated crates are always in
this export leaf graph; calling a free function does not erase them.

Empty-target release build with cached sources took58.90s versus P1's35.805s;
conditions were not controlled for a benchmark. `time -l` failed reading denied
`kern.clockrate` after Cargo reported build success, so build RSS is unmeasured.
Final leaf release is7,511,728bytes; `strip -x` gives6,101,560bytes versus full P1
helper5,156,192/4,297,440bytes. This probe includes extra API equality/guards and
omits P1 MIME/PPTX/sandbox, so these are concrete isolated artifacts, **not** a
same-functionality production link delta or installer estimate. No raster enabled.

P1 main.rs lines17-243 contain227 lines of shared package admission,
XML-picture/relationship/title scanning and recursive text/evidence mapping.
Higher-level export can replace the text mapping and normal relationship/occurrence
resolution; OPC replaces read/path/duplicate mechanics. Required policy checks,
exact assets, omissions and source bindings remain. Probe Rust LOC is recorded in
costs.json, not a production adapter estimate. Total maintenance savings, final
adapter/guard LOC, full BRN binary/installer/update cost and production rendering
remain unmeasured. Fewer hand-written semantic lines cost a materially larger
dependency graph; that tradeoff needs owner selection.

## Reproduce

Use the repository's pinned toolchain and public source cache:

```sh
P2=experiments/architecture-reassessment/p2-api-check
export PATH=/opt/homebrew/opt/rustup/bin:$PATH
export CARGO_HOME=/private/tmp/brn-p1-cargo
export CARGO_TARGET_DIR=/private/tmp/brn-p2-api-target
python3 "$P2/fixtures.py"
cargo build --manifest-path "$P2/Cargo.toml" --locked --offline --release
BIN="$CARGO_TARGET_DIR/release/brn-p2-api-check"
"$BIN" experiments/architecture-reassessment/p1-office-mime/fixtures/harbor.docx "$P2/results/harbor"
"$BIN" "$P2/features.docx" "$P2/results/features"
"$BIN" "$P2/prefix-variation.docx" "$P2/results/prefix-variation"
"$BIN" --guards experiments/architecture-reassessment/p1-office-mime/fixtures > "$P2/results/guards.json"
python3 "$P2/oracle.py"
python3 "$P2/costs.py"
cargo fmt --manifest-path "$P2/Cargo.toml" --check
cargo clippy --manifest-path "$P2/Cargo.toml" --locked --offline --all-targets -- -D warnings
```

Initial `cargo fetch --locked` needs public network. Optional `--features facade`
checks facade equality; raster, rendering, live providers and private inputs are
outside this check. Fixture ZIP timestamps change on regeneration; rerun outputs
and oracle together. Retained P1 fixture bytes are never regenerated here.
