# X2 Office1: corrected DOCX/PPTX implementation slice

18 October 2026. Source-only correction of `adc841a274dc7f9029ce61f876259638c25d931c`, on the same isolated `work/x2-office-prep-20261018` checkout. Original HTML candidate/base: `bbf107e370a5c5dc35a7588e97c477d5639401a6`; HTML is **not assumed approved**. The [full round-1 review](opus1/opus-review-round1.md) is retained verbatim; [response](opus1/response.md) answers all nine corrections. Original proposal remains in Git.

**Goal:** preserve all current schema-1/schema-2 behavior while moving all eight current document kinds to the owner-approved Xberg boundary. **Architecture:** keep the existing restricted helper, BRN DTO validation and checked host services. First measure DOCX current-reader and real-Xberg behavior against independent inventories; production projection follows measured gaps and a separate exact allocation. **Stack:** repository Rust/lock authority, Xberg 1.3.10, BetterOffice 0.3.0, BRN ZIP 8.6.0 versus Xberg ZIP 9.0.5. **Spec:** [owner all-document scope](../owner-scope-all-documents.md), [API](../source1b/api.md), [resources](../source1b/resources.md), [next allocation](../source1b/next-allocation.md), and the canonical [target](../../../../../architecture/threads-target.md), [behavior](../../../../../architecture/threads-behavior.md), [invariants](../../../../../architecture/invariants.md).

Only docs are authorized now. No Cargo/resolution/build/test/provider/network/GUI work; no delegation or heavy slot. O1 coding requires HTML acceptance, heavy availability and the parent's separate allocation. No new owner choice is ready or asked. The old D-OFFICE binary/recommendation to retain BetterOffice as the all-document solution is withdrawn. Existing production routes stay until qualified replacement; that sequencing grants no permanent fallback.

## 1. Actual result mapping; parity remains unproven

[Source map](source-map.json) records versions, file hashes and anchors. `ExtractedDocument.content`, optional `document`, `images`, `pages`, `uris`, `revisions`, `processing_warnings` are the public result path. Request `include_document_structure = true`. Public field existence and the intrinsic mappings below are source facts; **every behavior/parity claim is predicted/unproven until executed inventory comparison**. No parsing flattened Markdown to recover provenance.

| DOCX evidence | Current BRN schema 1 + 2 | Xberg 1.3.10 mapping and limit |
|---|---|---|
| Text/order/revisions | Accepted-revision export; ordered blocks and story/block locators | `content`, semantic Heading/Paragraph/ListItem/Table/Image nodes; `revisions` with kind/author/time/delta and paragraph-index anchor. Accepted-view and order parity unproven; empty paragraphs drop, notes/comments inject markers. No BRN story/block identity implied by content-derived node IDs. |
| Headings/lists | Level, source-level metadata; marker label/nesting | Style-derived heading level, ordered flag/nesting, annotations; source numbering/level parity unproven. |
| Tables | Addresses, typed rows, grid/row spans, nested-table gaps, `block_id` binding | `Table.cells` and semantic grid from strings. Parser knows `gridSpan`/`vMerge`, but DOCX does not pass spans into public `GridCell.row_span/col_span` (S3). Text/placement unproven. |
| Notes | Scoped fn/en labels and inline reference; NOTEREF cached value plus explicit unresolved-association gap | Footnote definitions and relationships keyed `fn{id}`/`en{id}`. Inline `[^id]` exists but collides between fn/en and with literal text; per-paragraph association excludes table references. Multi-paragraph notes join with spaces. NOTEREF handling unknown (S4). |
| Comments | Text + id/author/date/ranges/parent/resolved in section context | Dedicated Comment nodes/references, text in Footnote layer. Parser `Comment` has id/author/paragraphs; author not emitted, other metadata not parsed. Reference builder scans `[cmt:` text, so literal collisions are possible (S2). |
| Links | Per-inline href and exact visible wording | Link annotations with node-text offsets and `uris`; external/internal relationship mapping exists. Wording, offsets and scope parity unproven. Missing bookmarks are ordinary warnings, not automatic refusals. |
| Headers/footers | Document section metadata and story locators | `content_layer` Header/Footer; paragraphs join with newlines. Relationships plus conventional orphan header/footer sweep differ from current stories. Story identity and parity unproven. |
| Figures/captions | Repeated occurrence identity, Markdown byte range, anchor/rId/part locator, alt/caption and decoded PNG/JPEG | **Occurrence identity already public:** Image node `image_index` (drawing index), `attributes.image_uri`, `ExtractedImage.image_index/source_path/description`. Target path, not rId. No byte range into `content`; caption association and BRN source locator unproven (S1). |
| Completeness/location | Located omitted-element/chart/SmartArt/unsupported gaps and exact source occurrences | Sparse warnings, silent optional drops, paragraph-index revision anchors. No complete unrepresented inventory (S6). Namespace handling is S7, below. |

| PPTX evidence (explicitly O2) | Xberg mapping and limit; all parity unproven |
|---|---|
| Presentation order / source part | `PageContent`/slide pages exist, but parser and fallback sort by numeric slide filename, **not** `sldIdLst`. `quay.pptx`: rId7→slide9 then rId8→slide2; source predicts reversed order. |
| Hidden slides/shapes / shape IDs | Public `PageInfo.hidden` is hard-coded `None`; parser does not read `show`. Shape IDs/omitted inventory are not supplied. Quay slide9 `show="0"` plus order is a future O2 witness, not an executed red result (S5). |
| Text/bullets/tables | Slide Markdown; common structure rebuilt from it (`include_structure: false`). Bullet declarations/shape identity/spans not preserved by string-grid path; text/order unproven. |
| Notes/comments | `speaker_notes`; Comment revisions with slide index/author/date/text. Other-notes-shape gaps, comment ID/author ID/x-y/part missing; notes parity unproven. |
| Pictures/captions | Public image index and page fields; `ExtractedImage.source_path` exists but is `None`. Shape/rId/part/caption binding unproven. Disable image reads; preserve BRN's admitted image path only after identity proof. |
| Charts/SmartArt/math/layout/master | Some chart/diagram/LaTeX text is emitted. No complete located omitted inventory; preserve current gap policy and source evidence rather than silently treating new text as complete. |

S1–S7 correction registry: S1 lacks content ranges, **not image occurrence identity**; S2 comment metadata is a genuine parser/output gap; S3 spans have public fields but DOCX does not populate them; S4 inline note position is ambiguous, not absent; S5 hidden/source-path public fields exist but are unpopulated, other shape/comment facts are missing; S6 complete unrepresented inventory is absent; S7 prefix loss is a source prediction. The plain quick-xml `Reader` matches raw qualified `w:` names. In `prefix-variation.docx`, namespace-resolved `word/document.xml` has 15 `xw:p`, 1 `xw:tbl`, 9 `xw:tc`, 2 `xw:drawing`; header/footer use `w:`. Predict **body** loss while header/footer text may remain. Existing `maintained` prefix test proves only its image-gap assertion, not complete current-reader text parity.

The proposed metadata-only `OoxmlEvidence`/`DocxEvidence`/`PptxEvidence` projector is withdrawn from O1. Byte-equal alignment needs the full BetterOffice wire parse **and** structured export. Fingerprints do not remove that reader. Injected note/comment markers, dropped empty paragraphs, joined stories/notes and orphan parts make that alignment invalid on ordinary inputs. No full current reader may hide behind a facade and count as Xberg completion.

Bounded routes to assess **after O1 observations**, with principal-owned exact allocation/review:

- DOCX spans through existing `NATIVE_TABLE_GRID_ATTRIBUTE` into public `GridCell` fields (small).
- PPTX presentation order and populated public hidden/source-path fields (small).
- Structured note/comment offsets or annotations instead of ambiguous markers (small–medium); comment metadata (medium).
- Namespace-resolved matching (mechanical but wide); per-node content byte ranges (medium).
- A BRN renderer over Xberg semantic nodes can produce schema blocks/Markdown/occurrence ranges without rereading the package. It is not automatically a second reader; source completeness and output semantics still require an exact reviewed design. No such renderer is granted here.

These are maintained/upstream or limited audited patch seams, not a proposed framework/fork. Network/publication/patch authority is separate. A missing DTO field alone does not force quality loss or an owner choice. O1 measurements select the next code path.

## 2. Pre-operation limits: candidates, remaining STOPs

O1 T2 exercises **existing** `helper::extract(HelperRequest)` and `helper::prepared::prepare(HelperRequest)`; both reach private `admit_package`. The real executable uses the same JSON `HelperRequest`, with `--prepare` selecting schema 2 (`bin/brn-intake-helper.rs:59–83`). No public test API, production `office_admission` or cfg-test registration is needed. Construct the proposed `SecurityLimits` **inside the differential test**. Do not change current admission in O1.

For later promotion, snapshot **before** `admit_package`:

```text
E = limits.max_expanded_bytes - budget.expanded_bytes
P = limits.max_package_parts - budget.package_parts
```

Use checked/prevalidated allowances. `admit_package` subsequently charges actual bytes/parts (`helper.rs:395–415`); recomputing remaining after that would double-charge this package. Input ≤ request maximum; E ≤ 16 MiB, P ≤ 512 under current hard bounds. Existing bounded OPC inflation/path/duplicate checks stay reused, and admitted parts supply BRN's image validation/deduplication. No duplicate full text conversion is proposed for the final path.

| Candidate field/pin | Exact proposal and evidence obligation |
|---|---|
| `max_archive_size`, `max_files_in_archive` | E and P from the **pre-admission** snapshot. Xberg counts central-directory declared sizes; BRN ZIP8.6/OPC measures actual inflation via `take(remaining+1)`. They are not equivalent. |
| `max_compression_ratio` | Finite E. For positive compressed size and declared total ≤ E, member/aggregate ratios cannot exceed E. Source excludes zero-uncompressed members from its per-member ratio branch: empty valid members must pass; nonzero uncompressed + zero compressed gives infinity and refusal. Test these boundaries; do not describe every zero compressed member as malformed. |
| `max_content_size`, `max_entity_length` | E, **not** rendered 8 MiB. DOCX accounts body/instructions/deleted text/notes/comments before rendering. Prove pass/count semantics and near-boundaries; final BRN rendered text still ≤ `MAX_TEXT_BYTES`. |
| `max_table_cells` | E as a conservative aggregate bound: each counted source cell consumes XML bytes (closing `w:tc`), not BRN's per-table 2048×256. Verify nested/multi-table and any repeated parses before promoting this derivation. |
| `max_iterations` | Candidate checked E + P (event bytes + EOF per part). Count actual budget calls, repeated passes, note/comment nested parsers and shared header targets at coding/fixture boundary; this is **not a proven bound**. A required multiplier needs evidence/review, not automatic quota raising. |
| XML/nesting | Candidate 128, current parse depth. Existing 200,000-event policies have different per-document/per-part pools; boundary fixtures must establish equivalent acceptance, not silently impose a new aggregate gate. |
| `max_pages` (O2) | Candidate P; slide-count check precedes per-slide work. No post-hoc page count, RSS or timeout claim. |
| recursion/images | `max_archive_depth=0`; `images=None`, `captioning=None`, `qr_codes=Some(false)`, embedded-image OCR off. `needs_image_data()` must stay false; placeholders remain enabled. Source shows no Office image read/clone then; PPTX repeated-image work is avoided, not fixed. `from_bytes` still copies input. |

**Inconsistent ZIP members:** source1b requires rejecting truncated/inconsistent members. O1 must measure central-declared uncompressed size versus actual inflated size in **both directions**, for required and optional XML/image parts, preserving valid CRC where possible to isolate the size defect. BRN ZIP8.6 may accept these malformed packages; ZIP9 only bounds reads above declaration and Xberg may silently skip failed optional reads (`if let Ok`: comments/numbering/notes/styles/theme). Larger-than-actual declarations can also trigger Xberg total gates despite BRN accepting actual bytes. Proposed later correction: compare every admitted non-directory member's actual length with its central declaration before Xberg; handle directory declaration semantics explicitly in the boundary cases. This is a future hunk, not present behavior. Capture actual baseline before claiming narrowing or non-narrowing. Refusing previously accepted **malformed** packages implements the stated consistency contract; no evidence yet proves valid-input acceptance unchanged. Local-header zero sizes with a valid data descriptor/central directory are valid ZIP and are a separate positive test, not a central mismatch.

**XML before upstream allocation:** `SecurityLimits` does not budget every XML parser. A later narrow count-only quick-xml scan must cover the complete selected Xberg read set from admitted bytes before XML Strings/DOMs: DOCX document, document relationships, numbering/comments/footnotes/endnotes, relationship-targeted headers/footers **and conventional orphan `word/headerN.xml`/`footerN.xml`**, styles, theme, docProps core/app/custom; O2 presentation/rels, slides/rels, notes/rels, comments/authors, reachable chart/diagram XML and metadata. Resolve targets using maintained OPC facilities. Count actual expanded bytes/parts/relationships, events/depth, reject DTD/entity expansion and escaping relationships; external links stay inert evidence, not fetched. Do not parse/re-render text. Exact scan coverage and acceptance equivalence need boundary tests and a separate production allocation. `strict_parse` is consumed by `core/extractor/lenient.rs:58`, **not Office protection**. Library comments/defaults are not runtime proof. Existing image signature/dimension/decoded-byte/pixel/aggregate caps must precede decode/retention, not follow Xberg copies.

The fixed 100 MiB reads are statically inert **if** all central declared totals ≤ E ≤ 16 MiB, consistency has been established, and security validation runs on every handle before its first read (DOCX constructor, metadata reopen, PPTX from_bytes do). This corrects the earlier broad truncation claim; it does **not** close resource qualification. ZIP central-directory allocation occurs before entry-count checks, bounded by input geometry rather than P. The count scan itself needs already inflated parts. Existing admission, current-reader wire/export, Xberg parse and metadata reopen inflate separately; repeated reads/shared references multiply work. This is bounded-work analysis to be measured, never a hard heap/RSS guarantee. roxmltree has no configured node cap in these calls. PPTX OMML uses `with_defaults()` (1 MiB entity/1024 depth), an O2 potential narrowing seam.

**Concrete promotion STOPs:** prove scan read-set/pass counts and same valid-input envelope; pre-admit inconsistent members; classify silent optional-read losses; close roxmltree/preallocation proof and real restricted-helper CPU/NOFILE behavior. Where a maintained call cannot carry the required bound, request its narrow caller-budget/error propagation patch (or a demonstrated platform enforcement decision) before promotion. No general ZIP/XML framework, global allocator, sandbox relaxation, or claim that size caps alone close these STOPs.

**Warnings:** no generic `processing_warnings` rejection. O1 records exact `(source,message)` classes and the source item, then compares the current BRN gap/refusal policy. Known source templates (X39/X41) start this classification:

| Source / exact message template class | Proposed current-policy mapping to verify in O1 |
|---|---|
| `relationships` / `{total} cross-reference target(s) could not be resolved and were dropped from the document structure: {listed}{suffix}` | Missing bookmark/reference gap; ordinary document remains usable/partial. |
| `docx` / `Comment reference id {id} has no matching entry in comments.xml; the comment text was dropped` | Missing-comment-definition gap, not a new generic refusal. |
| `docx` / `Could not map w:sym character code {code:?} in font {font:?}; inserted a placeholder` | Text-loss gap; no invented symbol. |
| `docx` / `A DOCX field instruction (w:fldChar begin) was never closed with a matching separate/end; trailing body content may have been treated as field instruction text and dropped` | Field/trailing-content gap, subject to current malformed-input rules. |
| `docx` / `{title} relationship target '{path}' could not be read; that {lower} was dropped` | Dropped-story gap retaining target path (`title` Header/Footer, `lower` header/footer). |

Carry the source location actually available; missing required location evidence is a promotion gap, never guessed. These mappings require current-reader comparison; refuse only when an existing BRN malformed/limit/validation rule warrants it. Sparse warnings cannot prove completeness. Unknown/unmapped warning classes block **promotion**, not silently pass or create a new generic production refusal. Result errors, unexpected children, text/output/DTO budget failure remain explicit gates.

## 3. Exact writable scope to request later

O1 manifest hunk (not edited now), retaining the existing optional Xberg HTML row:

```toml
[features]
# add beside the existing default/helper entries
office-differential = ["helper", "xberg/office"]

[[test]]
name = "office_differential"
path = "tests/office_differential.rs"
required-features = ["office-differential"]
```

No unconditional Xberg dev-dependency: resolver 2 unifies its features into helper test binaries under ordinary `--all-targets` and workspace tests. The nondefault test feature leaves ordinary CI/shipped helper's HTML graph unchanged; it is **not** a user-facing backend option/fallback. Explicit differential builds compile Office into the helper graph, so requalify HTML both in the ordinary HTML-only graph and the explicit union. Feature presence is not runtime route replacement.

Root lock remains principal-owned. The old **33 identities are a rough ledger-derived upper bound**, not 33 additions. Nine are possible compatible movements of existing names: libc .2.189→.2.190; either 1.18→1.19; equivalent 1.0.2→1.0.3; smallvec 1.16.2→1.16.3; unicode-ident 1.0.26→1.0.27; uuid 1.26.1→1.28; deranged .5.8→.5.9; powerfmt .2.0→.2.1; syn **3.0.6→3.0.9**. Preserve locked versions when lower bounds permit (Xberg libc .2.189, either/uuid `1`; nix, if reachable, libc .2.186). Any further existing-package move beyond the six shared HTML moves is a STOP before compile. Syn **2.0.119** coexists with syn **3.0.6**; strum_macros .27.2/.28.0 requires syn **2.0**, not syn 3. Nix .31.3 is possible over-approximation, not established Office reachability. ZIP9/quick-xml .42/cfb .15 coexist with root versions; other additions depend on actual resolution. Source map labels every candidate; copied ledger checksums are provenance references, not an independent audit.

Before future compile: exact root resolution and effective features against helper/default/native/retrieval graphs; preserve the six audited HTML moves; match **all** actual additions to audited package sources/checksums and disclose newly enabled source. `typst-utils .15.1/build.rs` can invoke `git rev-parse HEAD`; strum_macros .27.2 (possible new) and .28.0 (already locked) are proc macros. Nix has a build script **if actually reachable**. Use existing source1b audit; inspect activated build-script/proc-macro/flate2/ZIP union, not just new package names. No metadata/tree/compile now. Email mail-parser .11.9/.11.8 and Excel Tokio 1.53.2/1.53.1 stay separate gates.

Only after O1 and an exact promotion grant: `helper/documents.rs` shared pinned config/executor, a bounded `documents/office_admission.rs` and actual DOCX adapter/renderer or approved upstream seam; schema-1 `helper.rs` and schema-2 `helper/prepared.rs` route hunks, converter provenance/host exact matcher, real restricted tests and retirement checker. These are a **scope map, not a ready projector design**. PPTX adapter/prepared hunks follow O2 evidence. Exact DOCX/PPTX MIME + `detect_package_kind` admission; never macro/template/slideshow/legacy OLE strings or custom registry. The `office` aggregate also compiles DOC/PPT/RTF/ODT/ODP/EPUB/FictionBook/LaTeX/RST/Org/OPML/Typst/BibTeX/DBF/Jupyter paths; compilation grants no dispatch. Production namespace/`roxmltree::` bans stay in the existing retirement checker. Same private adapter/boundary, no second service/protocol/backend flag. Luna, checked services and saved receipts stay fixed.

## 4. First coding allocation: substantive DOCX differential (O1)

After HTML accepted + heavy free + separate grant, use the accepted HTML pin (this proposal's manifest/config anchors may need rebasing). No production Office switch. Approximately eight test/fixture files and 600–800 Rust test/support lines plus exact manifest/root-lock reconciliation:

| Step | Exact files and action |
|---|---|
| T0 | Four **hand-authored, reviewed** JSON inventories in `crates/brn-intake/tests/fixtures/office/`: `harbor.inventory.json`, `features.inventory.json`, `prefix-variation.inventory.json`, `footnotes.inventory.json`. Derive from authored inputs/known fixture content, not new extractor output. Reuse the existing `crates/brn-threads-intake/tests/fixtures/harbor-process.inventory.json` and DOCX. Namespace-resolving raw XML script is a cross-check only, never the sole oracle. O2 uses `make-quay.py` authored inputs. |
| T1 | New `tests/docx_inventory.rs` (helper-gated): `current_docx_inventory_*` compare ordered complete schema-1/2 wording, blocks/stories, table cells/spans, links, notes, comments/revisions, figures/locators/assets/gaps, including **harbor-process**. Public `helper::extract`/`helper::prepared::prepare` only. Complete inventory assertions, not keyword hits. |
| T2 | `current_office_admission_*` in that same test uses existing public helper entrypoints, tightened request limits, existing four hostile DOCX fixtures, synthetic duplicates/aliases/escape/DTD/expansion/part/relationship/event cases. `office_limits_*` in `office_differential.rs` constructs candidate limits in test and observes real Xberg boundaries, declared/actual lies both directions, zero compressed/valid empty/data-descriptor members, aggregate multi-table/content/pass cases. No new production admission or public testing facade. |
| T3 | New `tests/office_differential.rs`: `office_xberg_docx_*` calls **real `xberg::extract`**, bytes + exact DOCX MIME, pinned config and proposed limits, compares the same full inventories including **harbor-process**. Executes S1–S7 classifications with permanent assertions of **observed** gaps and captures warning classes/optional-read loss. Test support: `tests/support/office.rs` for bounded synthetic OPC fixtures and small explicit comparison helpers. No framework. |
| Registry | `tests/fixtures/office/parity-registry.json` records per-fixture/class fulfilled/unmet promotion criteria and observed classifications. Gap assertions can pass with an unmet criterion; they do not claim parity. Update deliberately with later fixes; never add always-red required CI then weaken/delete it. Current-reader fidelity failures are preserved/reported without production fixes under O1; compiler errors are not behavior-red evidence. |

Existing fixture source hashes/inventories travel. Namespace-aware cross-check confirms no revisions/comments/`vMerge`/`rowSpan`/DOCX embeddings in the surveyed fixtures. Add bounded synthetic authored cases for accepted/deleted revisions, comment metadata/ranges, same-ID fn/en plus literal markers/table refs/NOTEREF, empty and orphan stories, merged/nested/multiple tables, repeated identical figure captions/targets. Existing repeated figures: harbor DOCX (two drawings/one media); notes/gridSpan: footnotes DOCX. O2 keeps harbor/quay PPTX including embedded XLSX, hidden state and presentation-order witness. Class presence alone is not fidelity proof.

### Future command order (none executed here)

Run serially with task-owned targets and repository toolchain after root/source audit. Ordinary HTML-only helper must precede host tests. These existing filters are source-present/nonzero: maintained 17, pptx 12, restricted_helper 6, qualification 11 (static counts, not results); lib prepared/protocol/AI filters are source-present. For new filters require a logged count >0 and inspect named tests; no zero-test success.

```sh
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo test -p brn-intake --features helper --test maintained --locked --offline
cargo test -p brn-intake --features helper --test pptx --locked --offline
cargo test -p brn-intake --features helper --lib helper::prepared::tests --locked --offline
cargo test -p brn-intake --features helper --test docx_inventory current_docx_inventory --locked --offline
cargo test -p brn-intake --features helper --test docx_inventory current_office_admission --locked --offline
cargo test -p brn-intake --features helper --test restricted_helper --locked --offline
cargo test -p brn-intake --features helper --test protocol html_converter_provenance_is_distinct_and_validates_generically --locked --offline
cargo test -p brn-intake --features helper --lib documents --locked --offline
cargo test -p brn-intake --features helper --all-targets --locked --offline
cargo build -p brn-intake --features office-differential --bin brn-intake-helper --locked --offline
cargo test -p brn-intake --features office-differential --test office_differential office_limits --locked --offline
cargo test -p brn-intake --features office-differential --test office_differential office_xberg_docx --locked --offline
cargo test -p brn-intake --features office-differential --test restricted_helper --locked --offline
cargo test -p brn-intake --features office-differential --test protocol html_converter_provenance_is_distinct_and_validates_generically --locked --offline
cargo test -p brn-intake --features office-differential --lib documents --locked --offline
cargo test -p brn-intake --features office-differential --all-targets --locked --offline
```

O1 union helper still runs the **current** production Office route; its no-access success does not qualify Xberg Office runtime. It does requalify actual Xberg HTML on the feature union. After rebuild of the exact intended helper graph and explicit `BRN_INTAKE_HELPER` binding, host regression commands are `cargo test -p brn-threads-intake --test qualification --locked --offline` and `cargo test -p brn-ai --lib conversion::tests --locked --offline` (nonzero, actual PDF/fixed-Luna mock and carried Poppler/environment qualifications preserved). Do not equate mocks with live Luna proof.

Later T4–T6 promotion: actual Xberg Office inside the unchanged restricted executable, both schemas, synthetic file/live-loopback denial, no extra access/child route, bounded pipes, PGID cancel/reap/cleanup and CPU10/NOFILE64; all prior HTML graphs; host table `block_id`, source/occurrence/location/asset binding, private/protected save, versions, replay/Undo, originals-removed offline retention. Retirement only after **every** independent inventory item and unmet registry gate closes, nested EML attachments use the qualified replacement, and `scripts/check-intake-retirement.py` is extended/run in the same allocation. Do not retire/assert cutover from new API types or class-presence tests. No reader retires in O1. Further code path/patch/renderer allocation remains principal-owned; owner choice arises only if evidence rules out behavior-preserving routes.

## 5. PDF advisory, independent of Office

Existing audit only; unchanged five-seam resource request, **not a resolution or owner question**:

1. Page-count `extractors/pdf/mod.rs:238` plain `load_mem` should pass lopdf 0.45.0 `LoadOptions.max_decompressed_size` through `load_mem_with_options`.
2. Native Brotli decoder must honor `max_output_bytes` (`decoders/brotli.rs:12–18`).
3. Caller parser options through `PdfRenderSession::open`, `NativeDocument::from_bytes` and document open.
4. Expose session page geometry and float `render_page_fit` for caller pixel admission.
5. Avoid eager image fallback after enumeration failure.

Pursue an exactly allocated maintained patch or demonstrate platform enforcement before PDF promotion; no memory-rlimit claim. This advisory STOP does not block independent Office. No new PDF research, compile or upstream request here.
