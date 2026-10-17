# X2 Office1: DOCX/PPTX common-adapter slice (source-only proposal)

18 October 2026. Branch `work/x2-office-prep-20261018` at `bbf107e370a5c5dc35a7588e97c477d5639401a6`. This is a proposal, not an allocation or code authority. The HTML component at that commit is under independent review elsewhere and is not assumed accepted. Nothing was compiled, resolved or run. Every fact below is a static reading anchored in [source-map.json](source-map.json) (pinned versions, checksums, file and line anchors, file hashes). Scope context: [owner scope](../owner-scope-all-documents.md), [source1b API](../source1b/api.md), [resources](../source1b/resources.md), [next allocation](../source1b/next-allocation.md), [HTML slice](../first-html-slice.md).

## Result

1. Admission is boundable with a maintained seam. `ExtractionConfig.security_limits` (public `xberg::SecurityLimits`) plus ZIP 9.0.5 declared-size reads cap bytes, counts, depth and text before Xberg allocates. Three config pins and one small count-only XML scan are needed. No STOP on admission.
2. The contract mapping is not closable from Xberg 1.3.10 output. Seven evidence classes are unavailable (S1 to S7 below), including byte ranges for figure occurrences. The smallest metadata-only projector that closes them is a projection of the typed BetterOffice models the helper already admits. That projector is close to the size of today's evidence layer, and Xberg would then contribute mainly body text. A behavior-preserving drop-in DOCX/PPTX replacement does not exist in this release.
3. One owner choice is real, not hypothetical (D-OFFICE, section 5). It is needed only before production promotion. It does not hold the first allocation.
4. First allocation O1 is DOCX-only and test-led: independent complete inventories, conformance of the current reader against them, then a real-Xberg differential under a test-only dependency. It adds no production facade. PPTX is O2 and carries extra STOPs.
5. The PDF resource request is one upstream-seam list (section 6). It does not gate Office.

## 1. Field mapping and the named projector

Status words: exact (usable as is), partial, unavailable. "Current" means the code at this commit: schema 1 `helper.rs`, schema 2 `helper/prepared.rs` and `prepared/pptx.rs` (anchors B01 to B07).

### DOCX

| Evidence class | Current contract | Xberg 1.3.10 field | Status |
|---|---|---|---|
| Body text, order | accepted-revision Markdown from `docx_edit` | `ExtractedDocument.content`, `document` nodes (needs `include_document_structure`) | exact order; parity must be proven on inventories |
| Revisions | accepted view only | text follows the accepted convention; `revisions` list (kind, author, timestamp, delta, paragraph index) | exact text, richer audit list; anchor is a paragraph index only (X25) |
| Headings | outline level, capped at 6, source level kept as metadata | style-name derived level | partial: depth above 6 and numbered headings differ |
| Lists | marker label, level | ordered flag, nesting; marker text not emitted | partial |
| Tables | `R{r}C{c}` address, row and grid spans, nested-table gap | `Table.cells` string grid; spans repeated or blanked (X06) | text exact, spans unavailable (S3) |
| Footnotes, endnotes | scoped label, inline `[^label]` at the reference, NOTEREF gaps | definition nodes and `FootnoteReference` relationships keyed `fn{id}`/`en{id}` | text and association exact; inline position and NOTEREF unavailable (S4) |
| Comments | text, id, author, date, ranges, parent, resolved | text only on a Footnote-layer node; reference found by scanning text for `[cmt:` (X04, X05) | unavailable (S2); the text scan is not admissible provenance |
| Links | `href` per inline | `AnnotationKind::Link` with offsets in node text, plus `uris` | exact |
| Headers, footers | "Document section" metadata | `content_layer` Header/Footer | exact; story ids unavailable |
| Figures | `ImageOccurrence` byte range in the Markdown, locator (anchor, rId, part), decoded PNG/JPEG asset | Image node (`image_index`, description), `ExtractedImage.source_path` (target path, not rId), all placeholders `![alt](image)` | ranges unavailable (S1); rId unavailable; part is resolvable from the target path |
| Omitted content | one gap per unsupported element, chart, SmartArt | a few warnings, otherwise silent drops | unavailable (S6) |
| Locator | `story/{story}/block/{block.id}` | content-derived node ids, paragraph index | unavailable as a stable BRN locator (S7 below covers the prefix case) |

S7: Xberg's body parser matches the qualified element name literally (`w:p`, `w:tbl`; X30). A package whose main namespace has another prefix is not recognised. `prefix-variation.docx` uses `xw:`. The source reading predicts empty body text with no gap. The current reader handles it and discloses the image join honestly (`maintained.rs:111`). This is a silent-loss witness, to be executed in O1 step 3.

### PPTX

| Evidence class | Current contract | Xberg 1.3.10 field | Status |
|---|---|---|---|
| Slide order | presentation order, part path | slide order from presentation rels; per-slide `PageContent` with `pages` config | order exact, part path unavailable |
| Hidden slides | "Slide N (hidden)", hidden shapes | `PageInfo.hidden = None`; text included silently (X21) | unavailable |
| Shapes, groups, ids, omitted inventory | per shape id and path, omitted-element gaps | none; structure is rebuilt from slide Markdown (X22) | unavailable |
| Text, bullets | per paragraph, bullet char or autonumber declaration | text exact; marker and declaration lost | partial |
| Tables | rows with spans | strings via Markdown re-parse | text exact, spans unavailable (S3) |
| Notes | body of the notes page; "other notes shapes" gap | `PageContent.speaker_notes` | body exact, other-shapes gap unavailable |
| Comments | id, author id, x/y EMU, slide part, text | `DocumentRevision{Comment}`: slide index, author name, date, text (X24) | partial: id, x/y, part unavailable |
| Pictures | bound to the shape by rId and part, decoded asset | bytes in visual order, `source_path = None` (X21) | binding unavailable (S1/S5); Xberg re-reads each referenced image on every slide (X19) |
| Charts, SmartArt, math | gap: "factual content unavailable" | chart text, diagram text, OMML to LaTeX | new content class: needs located provenance and a gap-wording decision, never silent |
| Layout, master, background | located gaps | none | unavailable |

### Named projector

`OoxmlEvidence` (`DocxEvidence`, `PptxEvidence`). Metadata only. It projects the typed models the helper already links and admits: `docx_parse` wire plus `docx_edit::structured` export, and `pptx_parse::PptxPackage`. It carries ids, anchors, spans, comment metadata, note reference positions, picture part and rId, hidden state, omitted-element and unsupported diagnostics. It carries no text, renders no Markdown, adds no XML reader, never parses Xberg's Markdown and is never a fallback. It binds to Xberg output only by ordinal alignment checked against byte-equal paragraph or cell text. Any mismatch refuses the request.

Seams that no projector can close, because they need Xberg to emit data it drops:

- S1: unique image identity or byte ranges into `content`. The current renderer injects one token per image and reads ranges from the match. Xberg nodes carry no ranges (X26) and every placeholder is identical.
- S2: DOCX comment date, resolved flag, parent, anchor ranges.
- S3: table spans (DOCX and PPTX).
- S4: footnote reference position inside a paragraph, NOTEREF.
- S5: PPTX hidden state, shape and part identity, picture rId, comment position, omitted inventory.
- S6: a completeness inventory of unrepresented content.
- S7: namespace-prefix independence.

The smallest upstream seam that removes S1 to S7 together is a rich conversion result: source part and id per node, content byte ranges, span-bearing grids, comment and revision metadata, and an unrepresented-element list. The HTML plan reached the same conclusion for its table and image ambiguity ([next allocation](../source1b/next-allocation.md)). Not proposed here: a BRN-side Markdown renderer over Xberg nodes, or any text scan. Both would be a second reader.

## 2. Pre-operation admission

Order, all before `xberg::extract`:

1. Exact kind. BRN passes exactly two MIME strings (`...wordprocessingml.document`, `...presentationml.presentation`) and no filename. `ooxml_opc::detect_package_kind` (O02) must return Docx or Pptx on the admitted parts. Xberg itself also accepts macro-enabled, template and slideshow strings (X09, X10); BRN refuses them. Legacy OLE conversion needs the legacy MIME (X28), which BRN never passes.
2. Existing `admit_package` (B02), unchanged: entry count against `max_package_parts`, then `ooxml_opc::unzip_parts_with_limits` with the remaining `max_expanded_bytes` (O01): safe paths, duplicate normalised paths, at most 5000 entries, inflation with `take(remaining+1)`, no prefix success. Its parts map is kept. BRN, not Xberg, reads image bytes from it and dedupes through the existing `image_asset` and `add_asset`.
3. Existing typed parse under the existing `ParseLimits` (B03, B04): 200,000 events, depth 128, DOCTYPE refused (O03, O04), shape, paragraph, run and comment caps. It runs first because the projector needs it, and it admits every part it reads.
4. The only new admission code: a count-only scan of the XML parts that Xberg DOM-parses and step 3 may not read (`docProps/core|app|custom.xml`, `word/theme/theme1.xml`, `word/styles.xml`, PPTX comment authors, chart and diagram parts; X13, X14, X23). It uses `quick-xml` 0.41.0, already a helper dependency, counts events and depth, and refuses DOCTYPE. Estimated 80 lines, no namespace strings (checker B12). It exists because roxmltree, as Xberg calls it, has no node cap. It must not narrow today's accepted envelope: boundary fixtures just under and over the ceiling are tests, not assumptions.
5. `SecurityLimits` mapping, every value from the request's `IntakeLimits`, none from Xberg defaults:

| Field | Value | Reason |
|---|---|---|
| `max_files_in_archive` | remaining `max_package_parts` | same count semantics as step 2 |
| `max_archive_size` | remaining `max_expanded_bytes` (default 16 MiB) | declared total, checked after every entry (X16); ZIP 9.0.5 then refuses reads past the declared size (Z01, Z02) |
| `max_compression_ratio` | very large | BRN has no ratio gate today. The 100:1 default (X16, whole-archive check at line 417) would refuse inputs accepted today |
| `max_xml_depth`, `max_nesting_depth` | 128 | existing |
| `max_content_size`, `max_entity_length` | `MAX_TEXT_BYTES` (8 MiB) | the 1 MiB entity default would narrow long text runs |
| `max_table_cells` | at least 2048 x 256 | the 100,000 default would narrow large tables |
| `max_iterations` | above the scan ceiling | backstop only; the scan is authoritative |
| `max_pages` (PPTX) | remaining `max_package_parts` | slide count, enforced before per-slide work (X20) |

6. Config pins: `max_archive_depth = 0` (default is 3 and recurses into `word/embeddings` or `ppt/embeddings`; X07, X08; `harbor.pptx` carries `Microsoft_Excel_Sheet1.xlsx`, a ready witness), `images = None`, `strict_parse = true`, plus every HTML-slice pin (B09: no cache, no timeout, concurrency 1, post-processing off, OCR off, no URL or local-file input). DOCX adds `include_document_structure = true`. PPTX adds `pages = Some(extract_pages)` for per-slide notes.
7. After return: exactly one result, no errors, no `processing_warnings`, no `children`; text at most `MAX_TEXT_BYTES` before append.

Exact maintained seam. The fixed 100 MiB constants (X12, X18) are inert once `max_archive_size` is at most 16 MiB: no entry can declare more, and ZIP 9.0.5 errors past the declared size. DOCX also refuses any member declared above 100 MiB before it reads (X11). This is a pre-read gate, not a post-hoc page, RSS or timeout claim. The prior "100 MiB read can truncate" statement applies only to a caller that leaves the 500 MiB default (corrections list in the source map).

Residual, stated without a memory claim: the zip crate allocates the central directory before any caller count check (BRN's own `ZipArchive::new` does too); roxmltree DOM size follows from the step 3 and 4 event ceilings, not from a hard limit.

STOP, with the smallest decision: PPTX figure bytes through Xberg. It re-reads and clones each referenced image on every slide with no aggregate cap (X19); `max_images_per_page` is PDF-only. Resolution inside this plan: Xberg never reads Office image bytes (`images = None`); BRN reads them from the admitted parts. That keeps S1/S5 as the open item rather than a resource gap. No custom ZIP or XML framework, fork, global allocator or sandbox relaxation is proposed.

## 3. Writable hunks to request later (no edits now)

Promotion hunks, used only after D-OFFICE. O1 uses a test-only dependency instead.

| Path | Hunk |
|---|---|
| `crates/brn-intake/Cargo.toml` | O1: `[dev-dependencies] xberg = { version = "=1.3.10", default-features = false, features = ["html", "office"] }` and one `[[test]] required-features = ["helper"]`. Promotion: the optional row `features = ["html"]` becomes `["html", "office"]`. No new direct dependency; `quick-xml`, `zip`, `image`, `libc` are already helper rows |
| Root `Cargo.lock` (principal) | Up to 33 added identities, all with checksums equal to the source1b ledger (zero mismatches, statically) and listed in the source map. New beside old: zip 9.0.5 beside 8.6.0, quick-xml 0.42.0 beside 0.41.0, cfb 0.15.0 beside 0.14.0. Probably already locked (the scratch graph was seeded from X1): libc, smallvec, either, itertools, unicode-ident, uuid, syn. The figure is an upper bound. The principal's offline metadata on the exact manifest gives the real set |
| Existing-package moves | None beyond the six approved HTML moves (serde_json, serde_spanned, toml, toml_datetime, toml_parser, toml_writer). Anything else is a STOP before compile. Record effective feature unions (flate2 backends, zip features) with `cargo tree -e features` before the first build |
| `helper/documents.rs` | Split `html_config()` into a shared pinned base plus the HTML overrides; add `office_config(limits)`; share `block_on` and `BACKEND_CALLS` |
| NEW `helper/documents/office_admission.rs` | MIME table, `detect_package_kind` check, `SecurityLimits` mapping, count-only XML scan |
| NEW `helper/documents/docx.rs` | pinned call, private `DocxRead`, `DocxEvidence`. PPTX is a later sibling file |
| `helper.rs`, `helper/prepared.rs` | DOCX arm `if source_base == 0` only. Nested EML attachments stay on the current reader until the EML gate, as the HTML arm does. Prepared DOCX reads the same adapter |
| `lib.rs`, host `threads-intake/src/lib.rs` | The host accepts exactly `CONVERTER` (B10). Any new provenance string is a coordinated host hunk. Stored receipts keep their old strings |

Office aggregate. `office` is the only gate that contains DOCX and PPTX; there is no narrower feature (pins in the source map, X29). It also compiles DOC, PPT, RTF, ODT, ODP, EPUB, FictionBook, LaTeX, RST, Org, OPML, Typst, BibTeX, DBF and Jupyter. They are reachable only by MIME. BRN admits two strings and registers nothing custom. A test passes other MIME strings (`docm`, `pptm`, `dotx`, `ppsx`, legacy OLE) and asserts zero backend calls through the existing call counter.

Separate gates, unchanged: email (mail-parser 0.11.9 against the locked 0.11.8, `CONVERTER` string, host matcher) and Excel (tokio 1.53.2 against 1.53.1). The static closure shows neither is pulled by `office`. The six shared HTML moves stay audited. Any further root checksum or source work is mapped before compile. HTML uses the same private adapter and boundary: no second service, protocol or backend flag. Retirement checker (B12): the adapter source may not contain `roxmltree::` or the OOXML namespace strings. Xberg's own transitive roxmltree is not scanned.

## 4. Test order, filters, retirement criteria

Counts must be nonzero in the log. Baselines recorded for HTML at this commit (not rerun by me): `documents` lib filter 15 passed, `brn-intake --features helper --all-targets` 83 passed in 6 results ([check index](../html1/check-index.json)).

| Step | What | Filter or command (fixed names) |
|---|---|---|
| T0 | Independent complete inventories, authored before any code from raw OPC XML by a committed script (precedent: `make-quay.py`). DOCX: `harbor`, `features`, `prefix-variation`, `footnotes`. PPTX (O2): `harbor`, `quay`. Existing complete inventory: `harbor-process.inventory.json` only | script plus JSON, reviewed against the fixtures |
| T1 | Current reader against the inventories through the real restricted helper. May expose existing gaps; those are findings, not failures of the plan | `--test maintained`, `--test pptx`, `--lib helper::prepared::tests`, `-p brn-threads-intake --test qualification` |
| T2 | Admission only, no Xberg: MIME and kind refusal, ceiling boundaries for every mapped limit, the four hostile fixtures (`hostile-dtd`, `-inflate`, `-members`, `-path`), duplicate and alias members, relationship counts, event pool | new `office_admission` tests |
| T3 | Real `xberg::extract` per fixture against the inventories. Full ordered sets, never word or keyword presence. Executes S1 to S7 as assertions. Embedded-object witness on `harbor.pptx` with depth 0 against the default. Prefix witness on `prefix-variation.docx`. Hidden-slide witness on `quay.pptx` (`show="0"`) | new `office_xberg_*` tests |
| T4 | Real restricted executable with the adapter compiled: no file, network or child canaries, bounded pipes and process group, cleanup, CPU 10 s and NOFILE 64 headroom with two parses | `--test restricted_helper` additions |
| T5 | HTML requalification on the union: the three HTML cases in `restricted_helper`, `protocol::html_converter_provenance_is_distinct_and_validates_generically`, `documents` filter, all-targets, all at least at the recorded counts | as listed |
| T6 | Host and retirement, only after parity | `scripts/check-intake-retirement.py`; `-p brn-ai conversion::tests` nonzero including the PDF and fixed-Luna mock |

Existing fixtures have no tracked changes, no comments, no `vMerge` or `rowSpan`, and no DOCX embedding (survey in the source map). T3 builds those synthetically with `ooxml_opc::rezip_parts`, as `pptx.rs` already does. Repeated figures use `harbor.docx` (two drawings, one media) and `quay.pptx`. Notes and `gridSpan` use `footnotes.docx`; tables use `footnotes.docx` and `quay.pptx`. Fixed Luna and the checked service stay unchanged.

Retire the current DOCX or PPTX reader only when all hold:

1. Every inventory item, including S1 to S7, is satisfied by the replacement and executed.
2. The schema-2 route keeps its block identity, table binding by `block_id`, replay and Undo (verified by the host qualification suite).
3. Nested attachments are served by the replacement.
4. The retirement checker is extended in the same change.

None holds today, so nothing retires in O1.

## 5. First coding allocation

O1, DOCX only. PPTX is explicitly O2.

- Step 1 (useful under every outcome): T0 and T1. Files: 4 inventory JSON, 1 generator script, extensions to `tests/maintained.rs` or a new `tests/docx_inventory.rs`, synthetic-fixture helper.
- Step 2: T2 plus the O1 manifest hunk and the principal's lock reconciliation.
- Step 3: T3 under the test-only dependency. Its output is a table of executed facts and one yes or no per class S1 to S7.
- Gate: promotion hunks from section 3 and T4 to T6 run only if D-OFFICE resolves to a path that keeps the contract. If not, O1 ends with Steps 1 to 3 merged and nothing else. This avoids an empty facade and an unused or narrowing production DOCX path.

Estimated scope: about 8 test and fixture files, roughly 600 to 800 test lines, one manifest hunk, no production source before the gate, zero changes to Luna, the checked service, the host or the CLI. Promotion adds two new source files and three small hunks (section 3). O1 should start from the reviewed HTML commit rather than `bbf107e3`, because the manifest line and `documents.rs` may change in that review.

D-OFFICE, the one consequential owner choice: given S1 to S7, may BetterOffice 0.3.0 (already a pinned, maintained OOXML adapter) remain the Office adapter inside the shared document boundary while Xberg serves HTML, XLSX, EML and PDF, with Xberg Office revisited when a rich-result seam exists? The alternative is to require Xberg as the sole DOCX/PPTX source, which waits on the upstream seam in section 1 or accepts a stated narrowing of the evidence list. My recommendation is the first option. It adds no bespoke projector, 33 packages or duplicate major versions, and it loses nothing. The owner scope asks for a maintained adapter and no silent quality loss; BetterOffice meets both and Xberg Office currently does not. Decide before promotion. Steps 1 to 3 give the measurements either way.

## 6. PDF next resource (from the existing audit only)

One request, not code. Ask upstream (xberg and xberg-native-pdf) for a caller-bounded profile:

1. The page-count path passes `LoadOptions.max_decompressed_size` through lopdf's existing `load_mem_with_options` (xberg `extractors/pdf/mod.rs:238`).
2. Native Brotli honours `max_output_bytes` (`decoders/brotli.rs:11`).
3. `PdfRenderSession::open` and `NativeDocument::from_bytes` accept parser options (`pdf/render.rs:958`, `pdf/native/mod.rs:81`, `document/open.rs:22`).
4. The session exposes page geometry and float `render_page_fit` (`rendering/mod.rs:436`).
5. No eager image fallback after enumeration failure (`pdf/native/images.rs:44,640`).

Decision needed: pursue that upstream patch, or name an enforceable platform bound. I did not verify any macOS memory rlimit and make no claim about one. This STOP does not block Office, XLSX or EML work.
