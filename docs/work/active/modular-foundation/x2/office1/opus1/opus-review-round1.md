# Office1 review, round 1: DOCX/PPTX plan and source claims

**Verdicts**
- **O1 DOCX development plan** (inventories, current-reader parity, real-Xberg differential): **CHANGES_REQUIRED.** The fixes are bounded and docs-only. The test-first direction is right. Several source claims are wrong or overstated, though, and some proposed pins would narrow inputs BRN accepts today.
- **Production Office promotion**: **CHANGES_REQUIRED, not ready.** There is no executed differential evidence. D-OFFICE is framed as a false binary, and the projector design does not hold up. The HTML component is not approved: its first review returned CHANGES_REQUIRED, so I did not treat it as an approved base.

**Binding:** I confirmed HEAD `adc841a274dc7f9029ce61f876259638c25d931c`, tree `cfda69ae9d6ee827b9f5ca9ecdf49474f46579a1`, branch `work/x2-office-prep-20261018`, base `bbf107e3`. The diff adds only the three office1 docs (968 lines).

**What I ran:** static checks only. That means reading the pinned registry sources (xberg 1.3.10, zip 9.0.5 and 8.6.0, betteroffice-opc 0.3.0, quick-xml 0.42.0, lopdf 0.45.0), the BRN sources, tests and CI, plus `unzip -l/-p` inspection of fixture XML and two read-only Python text checks on the JSON ledgers and the lock. I ran no Cargo, build, test, extractor, network, GUI or helper processes and no child agents, and made no edits. Every "predicted" item below comes from reading source; none was executed.

## Must-fix corrections (priority order)

### 1. The `SecurityLimits` mapping is wrong in three places
- **"Remaining" is computed after admission.** `admit_package` adds this package's parts and expanded bytes to the budget after unzipping (`helper.rs:395-415`). If `max_archive_size` and `max_files_in_archive` are "remaining" after that step, Xberg refuses any package larger than half the remaining budget. Correction: pin both to the pre-admission remaining values, the same allowance `admit_package` used.
- **BRN counts actual bytes; Xberg counts declared bytes.** BRN admission measures inflated bytes through zip 8.6.0 and opc `take(remaining+1)` (`opc lib.rs:108`); zip 8.6.0 has no declared-size limit. Xberg checks declared sizes (`security.rs` `ZipBombValidator`), and zip 9.0.5 errors once a member exceeds its declared size (`types.rs:104-109`, `crc32.rs` `count`).
  - Declared larger than actual: Xberg refuses a package BRN accepts today (narrowing).
  - Declared smaller than actual: Xberg's read fails, and optional parts are skipped silently through `if let Ok` (comments, numbering, footnotes, endnotes, styles, theme; `parser.rs:2266-2320`). That is a silent drop.
  - source1b `resources.md` already requires rejecting "inconsistent members"; the plan dropped that.
  - Correction: add a declared-equals-actual size check per member in admission, and record the malformed-input behaviour explicitly. Add T2 fixtures for both lies; none of the hostile fixtures has one.
- **The proposed values can narrow; finite values that cannot bind exist.**
  - `max_compression_ratio` is a `usize` compared as `f64`. Set it to the same value as `max_archive_size`. Each compressed size is at least 1 and the total is capped at that value, so no finite ratio can exceed it. A member with compressed size 0 still gets an infinite ratio and is always refused; that needs a boundary test.
  - `max_content_size` should not be 8 MiB. Xberg's DOCX budget is one per document, and it counts body text plus `instrText` plus deleted text plus notes and comments (`parser.rs:2918-2955`, `SecurityBudget::from_config`). That can exceed 8 MiB while the accepted output stays under it. Use the pre-admission expanded bytes for both `max_content_size` and `max_entity_length`.
  - `max_table_cells` is an aggregate per document. BRN's 2048×256 is a schema-2 limit per table (`prepared.rs:249-266`), so "at least 2048×256" can narrow multi-table documents. Use the expanded-bytes bound: a counted cell needs a closing `</w:tc>`.
  - `max_iterations`: propose expanded bytes plus part count. Every non-EOF quick-xml event consumes at least one byte. This is a candidate, not a fact: verify at coding time how many passes each part gets and add a boundary test.
- `strict_parse` has no effect on Office; it is consumed only at `core/extractor/lenient.rs:58`. Don't cite it as protection.

### 2. "No `processing_warnings`, else refuse" narrows real inputs
Xberg warns on ordinary documents: an internal link to a missing bookmark (`docx.rs` comment beside `pending_anchor_links`), or a comment reference with no definition (`parser.rs` after `parse_comments`). Correction: map each Xberg warning class to a BRN gap, or prove it corresponds to an existing BRN refusal. Never refuse on warnings generically.

### 3. T2 contradicts "no production source in O1"
T2 tests `office_admission` code, but `office_admission.rs` is listed as a promotion hunk. Correction: O1 T2 tests the existing `admit_package`, plus a `SecurityLimits` built inside the test from the mapping. Promote it to production only at the gate.

### 4. The test-only dev-dependency silently changes the tested helper graph
Under resolver 2, `cargo test -p brn-intake --features helper --all-targets` (`ci.yml:76`) builds the `brn-intake-helper` bin with dev-dependency features unified. So `restricted_helper`, including the HTML cases, would run an office-compiled binary, while the shipped build at `ci.yml:74` stays HTML-only. `cargo test --workspace` (`ci.yml:91`) would also compile xberg with office for every brn-intake test build.

Smallest correction: use a non-default feature, for example `office-differential = ["helper", "xberg/office"]`, as `required-features` on the new test, and run it with an explicit command. That leaves the existing CI graph unchanged. If the dev-dependency is kept, state the unification and requalify HTML on both graphs.

### 5. The fixture survey is wrong, and the inventory method repeats the defect it diagnoses
- `prefix-variation.docx` is recorded as `"counts": {}`. Its `word/document.xml` actually has 15 `xw:p`, 1 `xw:tbl`, 9 `xw:tc` and 2 `xw:drawing`. The survey counted only the literal `w:` prefix.
- Its header and footer use the `w:` prefix, so the prediction is "empty *body*, header and footer text present". "Empty text" is not quite right.
- Correction: inventories must not be produced by a new raw-XML generator script as the only oracle. Derive them from authored inputs: `make-quay.py` for the quay files, hand-authored reviewed JSON for the rest. A namespace-resolving script may serve as a cross-check only.
- Add `harbor-process.docx` to T1/T3; it is the only DOCX that already has a complete inventory.

### 6. PPTX "slide order exact" is false on an existing fixture (O2, but fix the claim now)
`parse_presentation_rels` and the fallback both sort by slide filename number (`pptx/parser.rs:878-910`, `container.rs:143-173`). `quay.pptx` has presentation order `rId7` (slide9) then `rId8` (slide2) in `sldIdLst`, so Xberg puts slide2 first. Combined with `hidden: None`, `quay.pptx` is a ready red witness for both order and hidden state.

### 7. Lock section: "33 additions, no other moves" is internally inconsistent
- Nine of the 33 are higher semver-compatible versions of packages already locked: libc 0.2.190 (0.2.189 locked), either, equivalent, smallvec, unicode-ident, uuid, deranged, powerfmt, and syn 3.0.9 (3.0.6 locked).
- The requirement lower bounds I checked are already satisfied by the locked versions: xberg needs libc 0.2.189 and either/uuid "1", nix 0.31.3 needs libc 0.2.186, and strum_macros needs syn 2.0, not 3.
- If any of these appears raised in the lock, it is an existing-package move and a STOP. If not, it is not an addition. Re-label them.
- "Zero checksum mismatches" is circular: the checksums were copied from the ledger.
- nix appears to be an over-approximation: no crate whose manifest I checked depends on it.
- Build items to disclose: build scripts in typst-utils (and nix, if it's real); strum_macros is a proc-macro.

### 8. The projector claim (Q3) cannot hold as written
"Metadata only, no text, bound by byte-equal paragraph or cell text":
- **It is the full reader.** Byte-equal checking needs BetterOffice text, which means running the full `docx_parse` wire parse plus `docx_edit` structured export on every request, i.e. today's whole reader. Bounded fingerprints would not change that dependency.
- **It would refuse ordinary documents.** Xberg injects `[^id]` and `[cmt:id]` into run text (`parser.rs:2835-2873`). It drops empty paragraphs, joins header and footer paragraphs with newlines, joins multi-paragraph notes with spaces, and sweeps orphan `word/headerN.xml` parts (`parser.rs:3322-3341`). Byte-equal alignment would refuse essentially every document with notes or comments, including `footnotes.docx`.
- Filling the remaining gaps from BetterOffice would be BetterOffice behind a shared facade, which is not acceptable as all-document completion.
- Correction: keep the projector out of O1 entirely. Restate it as unresolved design, not "smallest projector".

### 9. D-OFFICE is a false binary, and the recommendation must be withdrawn
Retaining BetterOffice as the production Office adapter changes the owner's goal, so it cannot be the principal's default or a recommendation now. There is a third route, a bounded set of upstream (or limited audited) patches, and several of them only populate public fields that already exist (next section).

## S1–S7 against the complete public result path

| Class | Plan says | What the source shows |
|---|---|---|
| S1 image identity/ranges | identity and ranges unavailable | **Overstated.** Per-occurrence identity *is* public: Image node `image_index` (the drawing index), `ExtractedImage.image_index`/`source_path`, node `attributes.image_uri` (`docx.rs` Drawing arm; `derive/mod.rs:215`). Only byte ranges into `content` are missing; `DocumentNode` has no range. |
| S2 DOCX comment metadata | unavailable | **Correct, a real parser drop.** The `Comment` struct holds only id, author and paragraphs (`parser.rs:227`); no range, date, done or parent parsing found. Author is parsed but not emitted. |
| S3 table spans | unavailable | **Correct today, small seam.** The public `GridCell` already has `row_span`/`col_span` (`document_structure.rs:549-561`). The PDF path fills them through `NATIVE_TABLE_GRID_ATTRIBUTE` (`types/internal.rs:61`, `pdf/extraction.rs:74`). DOCX passes a string grid even though its parser already knows `gridSpan`/`vMerge` (used by `to_cell_grid`). |
| S4 note reference position | inline position unavailable | **Overstated.** The position *is* in-band as an injected `[^id]` marker, but it is ambiguous: footnote 1 and endnote 1 both render `[^1]` in `footnotes.docx`, and literal author text can collide. Structured association is per paragraph only, and missing for references inside tables (`table_stack.is_empty()` guard). NOTEREF is unknown, not asserted. |
| S5 PPTX hidden/part/rId/comment position | unavailable | Hidden state and source path exist as public fields but are hard-coded `None` (`pptx/mod.rs:302,351`). The parser never reads `show`. Shape ids, comment x/y and omitted inventory are not parsed. |
| S6 unrepresented inventory | unavailable | Correct: warnings are sparse and drops are silent. |
| S7 prefix independence | silent loss | **Plausible, not executed.** quick-xml 0.42 `QName::as_ref` is the raw qualified name, read through a plain `Reader` with no namespace resolution, and I found no canonicalization pass in the DOCX path. Keep it as a prediction only. The current test (`maintained.rs:111`) asserts only the image-join gap, not that the current reader's text is correct. |

The "exact" cells in the DOCX/PPTX tables (body order, links, headers/footers, footnote association, revisions) must all be re-labelled "predicted; unproven until inventory parity". Header/footer and note handling differ statically (see must-fix 8).

## Resource questions (Q2)

- **`max_archive_depth = 0`:** correct. It disables embedded recursion cleanly with no error (`docx.rs:1323`, `pptx.rs:582` guard `> 0`).
- **`images = None`:** avoids Office image reads and clones at every internal call site I found:
  - DOCX reads image bytes only when `needs_image_data()` is true (`docx.rs`, image loop).
  - PPTX calls `get_slide_images` only under `extract_images = needs_image_data()` (`pptx/mod.rs:406`, `extractors/pptx.rs:491`).
  - Condition: captioning stays `None`, `qr_codes` is not `Some(true)`, and embedded-image OCR is off.
  - Placeholders are still emitted (`inject_placeholders` defaults to true).
  - So the PPTX repeated-image concern is avoided, not "fixed". `from_bytes` still copies the whole input (`container.rs:78`).
- **The 100 MiB truncation correction:** holds statically, under two conditions. The declared total must be at most 16 MiB, and the validator must run on each handle before its first read; it does: `DocxParser::new` (`parser.rs:2227-2228`), the metadata re-open (`docx.rs`), and PPTX `from_bytes`. Archive-mode reads set `size_unknown = false`. This does not close the upstream resource STOP. What remains is silent skipping of optional parts on read error (must-fix 1) and DOM size.
- **Central directory:** capacity is bounded by `directory_start`, entries are parsed one at a time, and the input already exists (`zip_archive.rs:246-298`). It is bounded by input size, not by a parts count before allocation. BRN's own `ZipArchive::new` comes first. The plan's residual wording is accurate.
- **Unbudgeted roxmltree:** styles/theme (`styles/mod.rs:192`, `theme.rs:102`), docProps, and all PPTX XML. The step-4 scan list must be the complete Xberg read set: add orphan `word/header*/footer*.xml`. The PPTX OMML sub-parse uses fixed `with_defaults()` limits (`pptx/parser.rs:712`; 1 MiB entity cap, depth 1024), which is an O2 narrowing item.
- **Aggregate decompression:** BRN admission, the BetterOffice wire parse, Xberg's parse and Xberg's metadata open each inflate separately. Peak work is a multiple of 16 MiB, not "bounded before Xberg allocates". State this; don't claim a heap bound.

## Bounded upstream seams (Q4)

| Seam | Size |
|---|---|
| DOCX spans through the existing native-grid attribute | small |
| PPTX `PageInfo.hidden`, `ExtractedImage.source_path`, presentation (`sldIdLst`) order | small; fields already exist |
| Note/comment references as structured offsets or annotations instead of text markers | small to medium |
| Comment metadata | medium |
| Namespace-resolved matching for S7 | mechanical but wide |
| Content byte range per node | medium |

A BRN-side Markdown renderer over Xberg nodes does not re-read the package. Whether it counts as forbidden duplication is a principal call; it is not automatically a "second reader". None of these routes needs a fork or a framework. Pursuing any of them needs network/publication authority outside this review.

## O1 scope (Q5/Q7)

The filters exist with nonzero test counts: `maintained` 17, `pptx` 12, `restricted_helper` 6, `qualification` 11; `protocol::html_converter_provenance_is_distinct_and_validates_generically`, `helper::prepared::tests` (`prepared.rs:957`) and `brn-ai conversion::tests` (`conversion_tests.rs`) are present. Write the full commands with `-p` and `--features`.

The fixture-absence claim is **correct** (prefix-agnostic check): no `ins`/`del`, comments, `vMerge`/`rowSpan` or DOCX embeddings in any fixture. 8 files and 600–800 lines is proportional.

T3 should keep **permanent assertions of observed gaps** (an executed yes/no per class) without failing tests that are always required. Promotion stays blocked by the retirement criteria, not by red tests.

## PDF (Q8)

The list is advisory and consistent with source. `extractors/pdf/mod.rs:238` uses plain `load_mem`, lopdf 0.45.0 has `LoadOptions.max_decompressed_size`, and the Brotli decoder ignores `max_output_bytes` (`brotli.rs:12-18`). There is no RLIMIT claim. The proposal is valid, not a resolution, and does not block Office.

## Next allocation and owner choice

**Allocation:** once the HTML component is approved and the doc corrections above land, allocate O1 DOCX only:
- T0 inventories authored from inputs;
- T1 current-reader parity;
- T2 on the existing admission path with test-built limits;
- T3 differential behind the explicit test feature.

The differential writes no production source and changes no lock moves without a principal STOP.

**Owner choice:** none is ready. After T3 there may be one, but only if the evidence shows that neither the upstream seams nor an approved node renderer can preserve the contract. Keeping BetterOffice, or a stated narrowing, would then be the owner question.

**Status:** not implemented, verified, native-observed, owner-accepted or merged. Only the static reading above was done.
