# X1 gate 3 — Xberg inside the existing restricted intake helper

**Written integration design; no production implementation or cutover authorization.** Recommend a bounded adaptation of Xberg for PDF only, conditional on the integration gates below. Gates 1–2 passed on the maintained fixtures; their [prerequisite confirmation](REPORT.md#gate-3-prerequisite-confirmation--15-october-2026) includes the observed reading-order difference. This design does not claim raw Poppler/Xberg line equivalence or complete PDF support.

**Production integration waits for `work/daily-workflow-reliability` to land, completion of C0, C's allocation of exact files at its integration ref, and explicit owner approval.** The owner's 15 October 2026 permission extends X1 to this written gate only. Preserve the stored overlapping-branch conditions, including `work/ui-polish` where applicable; neither branch nor its worktree is part of this assignment.

## Authority and baseline

| Item | Recorded state |
|---|---|
| Production source snapshot | `7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`, fetched `origin/main` used by gates 1–2 |
| Qualification commit | `f6092462065a6d614fe337f7a2639ca2e75b55a4`; transfer/location record at `93994ff5780b39f3375d7602c8ee0ccb26c2002e` |
| Design location | `work/x1-xberg`, `experiments/x1-xberg/` in `/Users/evokessler/repos/brn-rust/.worktrees/x1-xberg` |
| Current changes | This document and REPORT.md only; standalone experiment manifest/lock, root files and production code unchanged |
| Governing contracts | [Master plan §8/A2–A5 and §11](../../docs/work/active/modular-foundation/proposal/final/01-master-plan.md), [intake brief](../../docs/work/active/modular-foundation/proposal/final/03-intake-build-prompt.md), [invariants](../../docs/architecture/invariants.md), [Threads behavior](../../docs/architecture/threads-behavior.md) |

Reconcile all paths, pins, contracts and qualification results to C0's recorded integration ref before implementation. This snapshot cannot justify overwriting newer workflow work. No C0 completion or overlapping-branch landing was verified by this design task.

## Existing boundary and reuse decision

The current [helper executable](../../crates/brn-intake/src/bin/brn-intake-helper.rs) activates native restrictions **before reading stdin**, requires an empty environment and parses one bounded `HelperRequest`. Its normal mode returns schema-1 `Extraction`; `--prepare` returns schema-2 `PreparedDocument`. Parser implementations belong to the `brn-intake` **helper feature**. Default consumers use the DTOs and validators.

PDF is currently the exception in [host preparation](../../crates/brn-threads-intake/src/prepared.rs): it writes a transient input PDF, runs `pdftotext -bbox-layout` and `pdftoppm`, then builds source lines and previews. The [deterministic conversion route](../../crates/brn-threads-intake/src/lib.rs) instead uses `pdftotext -layout` and `pdfimages` for page text and linked embedded figures. [pdf_tools.rs](../../crates/brn-threads-intake/src/pdf_tools.rs) discovers those executables. Schema-2 preparation does **not** consume `pdfimages`; its retained figures come from checked preview crops.

Reuse the helper protocol, [process ownership](../../crates/brn-threads-intake/src/process.rs), [transient lifecycle](../../crates/brn-threads-intake/src/transient.rs), [prepared DTO/validator](../../crates/brn-intake/src/prepared.rs) and [formatter/receipt checker](../../crates/brn-intake/src/prepared/formatting.rs). Adapt the PDF dispatch and add a small helper-only Xberg adapter with two projections into the existing schemas. Do not introduce a parser service, second worker protocol, generic extractor registry or new PDF parser. Keep the qualified Office/MIME/HTML readers and Markdown/text behavior.

Proposed flow: host bounded source read → existing owned helper process and pipes → helper-only native PDF adapter → existing validated schema-2 evidence → unchanged explicit Luna conversion → existing deterministic candidate/crop checks → existing checked host save. The deterministic `Converter::convert(Pdf)` route also uses the helper, with the schema-1 projection; it makes no provider call.

## Exact dependency selection and cost

Add only to `crates/brn-intake/Cargo.toml`, activated by `helper`:

```toml
xberg = { version = "=1.3.10", optional = true, default-features = false, features = ["pdf"] }
# Add "dep:xberg" to the existing helper feature.
```

Keep Rust **1.98.1**. The qualified effective Xberg features are **`html`, `image-encode`, `pdf`, `pdf-native`**; defaults `tokio-runtime`/`simd-utf8` are unnecessary. Use the native backend with locked **xberg-native-pdf 1.3.10 / lopdf 0.45.0**. Do not enable `ocr`, `paddle-ocr`, `pdf-pdfium`, `embeddings`, `full`, crawling, remote extraction or extra formats. Do not introduce Kreuzberg. The demonstrated no-Tokio extraction path and `xberg::pdf::PdfRenderSession::{open,page_count,render_page_to_png}` suffice; do not add a runtime merely to poll extraction.

C owns root lockfile resolution. Preserve qualified dependency versions where compatible, review the complete root lock/feature diff, and requalify any changed resolution; do not copy the standalone lockfile over the root lock or silently accept a newer native engine. Keep the dependency optional in default DTO consumers and ensure PDF parsing/rendering is called only in the restricted executable. Check workspace feature unification as well as individual package graphs.

| Gate-1 evidence | Integration consequence |
|---|---|
| X1 `quick-xml 0.42.0`; root `0.41.0` | Both versions remain unless an independently justified change is approved. Removing host bbox XML parsing can remove the host crate's direct quick-xml dependency; Office readers still need the root pin. |
| X1 has no `zip`; root `8.6.0` | No anticipated ZIP 9 duplicate with this selection. Preserve qualified Office ZIP handling. |
| No X1 `ort` or `ort-sys`; root uses ORT `2.0.0-rc.13` | Do not change the embedding runtime. A combined native-retrieval build must confirm no new links/feature conflict. |
| Both use `image 0.25.10` | Reuse BRN's pinned helper image adapter for PNG normalization/resizing; inspect added codec features after unification. |
| Internal duplicate families | `alloc-no-stdlib` 2.0.4/3.0.0, `alloc-stdlib` 0.2.4/0.3.0, `brotli-decompressor` 5.0.3/6.0.1, `fax` 0.2.7/0.3.0, `getrandom` 0.2.17/0.3.4/0.4.3, `miniz_oxide` 0.8.9/0.9.1, `pastey` 0.1.1/0.2.3, `syn` 2.0.119/3.0.9, `weezl` 0.1.12/0.2.1. Recheck the combined graph; no deduplication upgrades in this slice. |
| Release probe versus minimal binary | +36,980,776 bytes; after identical system `strip -S`, **+36,957,760 bytes (~35.25 MiB)**. Actual helper/app incremental size is unmeasured because existing code/codecs may overlap. Measure before approval. |
| Cold release builds | 0.234 s baseline / 126.336 s candidate, **provisional** due to another build. Not a product latency or uncontended build benchmark. |
| Native/notice inventory | CoreFoundation, libiconv and libSystem; bundled static BLAKE3 NEON C. No external PDF/OCR library. MIT engine, `option-ext 0.2.0` MPL-2.0 and vendored font/fontdb notices require packaging review. |

Sources: [feature/graph summary](evidence/graph-summary.json), [duplicates](evidence/duplicates.txt), [root comparison](evidence/root-lock-comparison.tsv), [native audit](evidence/native-audit.md), [sizes](evidence/binary-sizes.json), [license inventory](evidence/licenses-macos.tsv). The no-download evidence applies to this locked graph and exercised local route. Repeat the audit/offline network-denied build for the integrated graph.

## PDF adapter and prepared evidence

One helper-only PDF reader accepts bytes and validated `IntakeLimits`, never a path or URL. Select plain native content, per-page extraction and `keep_native_content`; explicitly disable OCR, quality rewriting, table reconstruction, cache use, image OCR and placeholder injection. Use native images only for the deterministic image projection; schema-2 needs text and previews. Do not export arbitrary Xberg metadata, source filenames, errors or hierarchy as trusted BRN content.

For `--prepare` with `kind="pdf"`:

1. Admit bounded nonempty bytes and page count. Require one extraction result, complete uniquely numbered consecutive pages, matching extraction/render page counts and finite positive per-page dimensions from native page metadata. Reject encryption requiring a password; do not prompt, fetch, repair with another tool or silently truncate. Account for rotation/CropBox and the renderer's coordinate convention before accepting any crop geometry.
2. Emit one `PageText` block **for every page, including empty scans**, with stable page-based IDs/locators and the native page content in its emitted order. Preserve literal values, Unicode, punctuation and line boundaries used by the existing receipt checker. Do not regroup using the retired Poppler bbox rules, sort columns, infer tables or hardcode fixture permutations.
3. Render with `PdfRenderSession` inside the helper. The qualification used 150 DPI followed by a 1600-pixel-long-edge resize; implement that bounded resize/PNG encoding using the existing Rust `image` dependency, with predicted intermediate and final geometry/pixel/byte budgets checked before allocation. No Python or subprocess becomes a shipping requirement. Return source-page dimensions in PDF points and the actual validated PNG dimensions/hash; retain the existing maximum 1600-pixel preview edge. The 1236 versus 1237 pixel rounding difference is accepted only with correct source-to-preview crop mapping.
4. Return schema **2**, unchanged `PROFILE="brn-prepared-v2/luna-high-v1"`, `format="pdf"`, original byte hash, echoed limits, measured usage, pages, preview PNG assets and blocks. Deduplicate identical PNG bytes while retaining each page association. Require exactly one source-text block and valid preview association per page in the PDF adapter/host checks, beyond the generic validator. Derive `needs_transcription` from **`text.trim().is_empty()`**, never the inconsistent Xberg `is_blank` summary.
5. Any extraction error, missing page, substantive processing/render warning or incomplete preview is a refused preparation. Install/drain native render diagnostics per job; no successful result with dropped glyphs or unqualified render damage. A static general fidelity disclaimer can remain a located limitation. Do not put lost content into a generic prepared gap and treat that as complete: `finish_candidate` currently maps prepared gaps to `Limitation`.
6. Compute usage/output size through the existing finalization and run `PreparedDocument::validate()` before stdout. Host preparation repeats validation, format/source hash/limits/input-byte binding and post-completion cancellation/deadline checks. No original PDF is part of this response.

The observed process page-2 native order puts Marker/Depth rows before Status. The existing optional **complete permutation of unchanged nonempty source lines** remains the only reflow mechanism. Its demonstrated permutation in [REPORT.md](REPORT.md#per-page-findings) is a fixture witness, not production logic. Keep duplicate/missing/out-of-range-index rejection and exact source/hash/preview binding. Independent row-tuple and preview checks must verify `M-0042 / 0 / Dry at intake`, `M-0108 / 12.5 / Outlet flowing`, and `M-0110 / -- / Not observed`; finding the individual tokens is insufficient.

Prepared PDF evidence continues to contain **page previews plus PageText**, with retained figures supplied by checked crops. Do not add embedded-image Figure blocks/provider inputs to this lane merely because Xberg exposes them. That would change the model-facing evidence and asset budget. Embedded-image extraction replaces `pdfimages` in the deterministic route below; gate 2 already checked both images independently of the preview route.

## Deterministic PDF conversion and complete Poppler retirement

Preserve the public deterministic PDF capability; do not leave its old executables behind while switching only `prepare`. Add top-level `kind="pdf"` to normal helper extraction, using the same PDF reader with images enabled and **without rasterizing page previews**. Project into existing schema-1 `Extraction`: original root, page source nodes, bounded literal page text in safe fences, normalized PNG assets and page-bound `ImageOccurrence`s. Record a PDF-specific exact converter identifier, proposed `brn-intake-v1/xberg-1.3.10/pdf-native`; host validation selects this constant only for PDF, retaining `CONVERTER` for existing formats.

Validate embedded image dimensions, decoded pixels, type, page association and masks before retention. Reject masks/unsupported composition until separately qualified. Deduplicate asset bytes but keep every image occurrence and page locator; preserve the host's `page:N` source convention for this route. Link extracted images even on an empty-text page, while recording an explicit text-transcription omission so an image-only scan cannot become a complete readable note. This removes the old unlocated-image case without pretending to OCR it. Preserve the general complex-layout/vector-art/scan limitation and inventory-based Partial/Unchecked semantics; distinguish that fixed disclaimer from substantive omissions when adapting host gap mapping.

Schema 1 currently requires the original root bytes to match `original_sha256`. Reusing it therefore echoes the bounded original over the private pipe **transiently**; it must be discarded with the extraction envelope, just as the current host discards other source bytes when creating `ConversionResult`. Account for root/base64/JSON overhead, decoded child bytes, retained-byte quota and stdout quota; refuse overflow. No original bytes enter the knowledge store, logs, checkpoints or backups. No schema change or PDF-attachment format expansion is implied; this branch admits standalone PDFs only.

Remove all shipping `pdftotext`, `pdfimages` and `pdftoppm` discovery, invocation, environment overrides and renderer-specific fontconfig setup once both routes pass. No alternate backend flag or hidden Poppler retry. Keep the experiment's recorded Poppler evidence as a comparison oracle; it is not a runtime dependency or fallback. Do not remove generic process/cancellation/isolation checks with the retired renderer mechanism.

## Process isolation, budgets and lifecycle

Run the existing packaged `brn-intake-helper` by canonical absolute path with `env_clear`, bounded stdin/stdout, discarded stderr, one owned process group and the existing private host scratch directory as cwd. The helper activates its **current** macOS deny-default profile before reading/parsing: inherited pipes only, system runtime/font reads already allowed under `/System/Library` and existing runtime paths, no input/temp/home/vault/credential allowance, no network, and no disk writes except `/dev/null`. Keep fail-closed refusal on unqualified platforms or restriction activation failure. Xberg may not load a model, execute an external converter or read file/URL references from a PDF.

**Do not substitute the experiment's runtime.sb for the helper profile.** The experiment permitted explicit fixture/output paths and `/Library/Fonts`; the production helper permits neither. Actual PDF extraction/rendering with empty environment and the exact production profile is a mandatory pending gate. Missing system fonts or denied optional font/cache reads must produce qualified output or refusal, never ambient HOME/user-font access, cache writes or a silent sandbox expansion.

| Existing boundary | Required handling |
|---|---|
| Input 16 MiB; request envelope `MAX_REQUEST_BYTES = 4 × 16 MiB + 1024` | Validate before parsing; no source-path reopening or URL input. Keep bounded source-file reading on the host. |
| Text 8 MiB; schema-1 retained sources 32 MiB; decoded/expanded defaults 16 MiB | Bound aggregate text/decoded objects and serialized output. Count actual usage; unused package/MIME fields stay zero, not fabricated evidence of parser memory limits. |
| Prepared pages/assets 32; sources/occurrences/blocks 2048 | Preserve prepared page bound 32 and image/occurrence caps. The schema-1 projection has no preview-page cap; its page nodes must fit `MAX_SOURCES` including the root. Set Xberg `SecurityLimits.max_pages` to the applicable mode's bound, check complete native counts and reject over-limit input, never accept a truncated prefix. The experiment's 20-page/100-images-per-page settings are not production policy. Requalify any effective support reduction from the schema-1 source-envelope bounds before cutover. |
| Images 16 MiB aggregate; per-image 32 × 1024² pixels; aggregate 64 × 1024² pixels | Respect tighter request limits and bounded image decoder dimensions/allocation. Check intermediate rasters/rotation/resize and repeated occurrences as work, not only deduplicated retained assets. Bound native decompression before processing; post-result validation alone is insufficient. |
| Output 64 MiB maximum | Use tighter configured output cap, bounded writer/reader, and final size accounting. `RLIMIT_FSIZE` does not cap pipe output; the protocol/host caps do. |
| PDF host wall profile 20 s; hard configurable maximum 30 s | One deadline covers request writing, extraction, rendering and pipe completion. Keep the current process polling/kill path; Xberg's timeout is additional cooperation, not enforcement. Do not import the experiment's 30 s extraction timeout over the host deadline. |
| Helper CPU 10 s, file descriptors 64 | Preserve existing `RLIMIT_CPU`/`NOFILE` and resource-activation refusal. Do not raise limits to make fixtures pass. |
| Memory | No hard RSS ceiling is currently enforced by the helper. Library page/time controls are not a memory sandbox. Pre-allocation/expansion enforcement and measured peak RSS on adversarial inputs require qualification; if the pinned API cannot bound an operation, that operation cannot ship without a separately reviewed enforceable limit or narrower accepted input profile. |

Cancellation/timeout/output overflow/process death use the existing process-group kill, wait/reap and writer/reader joins. Reap descendants on ordinary completion too. Do not rely on an async task cancellation flag to stop native parsing. Check cancellation after helper completion and again at existing conversion/save boundaries so a late result cannot persist after cancellation.

The new worker writes no raw input, preview or image files. Preserve host scratch ownership, 0700 permissions, symlink refusal, storage/entry monitor and explicit checked cleanup on every exit plus Drop fallback. Keep startup/pre-conversion cleanup of owned UUID directories older than 24 hours, without following links or deleting unrelated/external files. Crashes must leave no retained original payload. No store handle, model credentials, conversation context or data directory is supplied to the worker.

## Luna and checked host behavior remain unchanged

[brn-ai/src/conversion.rs](../../crates/brn-ai/src/conversion.rs) continues to call local preparation, then the dedicated fixed `gpt-6-luna` high-effort conversion through the existing explicit ChatGPT account, with no tools/history/store access and existing provider timeout/prompt/candidate budgets. No general-chat settings, new client, automatic OCR, provider/model substitution or extractor failure retry enters that stage. Empty scan text still means explicit visual transcription from the page preview; uncertainty remains visible.

Keep exact-candidate/source/preview receipts, full source-line permutations, checked crop bounds and retained PNG/hash checks. Full-page previews remain transient; only accepted readable content and linked figures are retained. Full import must not become a summary. Inventory and uncertainty determine coverage independently of model success. Existing useful/full-note intent, protected save, scope, version checks, idempotent replay, Undo and backup/restore stay with checked host services. Do not edit `brn-threads-app/src/intake.rs` for this adapter; reconcile its newer workflow behavior after that branch lands.

## Failure behavior

| Condition | Visible result; no save/provider request from a failed preparation |
|---|---|
| Missing/non-executable helper; unsupported platform or failed native restriction activation | Refused/unavailable local conversion. Never select Poppler or relax the restriction. |
| Malformed/encrypted/truncated PDF, missing pages/geometry, unsupported mask, processing/render warning or native panic/death | Refused invalid preparation, with a bounded non-content reason. No partial envelope accepted as success. |
| Page/text/image/expansion/output/CPU bound or wall deadline exceeded | Budget/timeout refusal as appropriate; kill/reap owned processes and clean scratch. |
| Source/limits/schema/profile/preview/hash mismatch, older helper not supporting PDF, corrupt/oversized response | Protocol/compatibility refusal. Ship matching host/helper artifacts; no schema guessing or compatibility fallback. |
| Cancellation, including a completed result queued before save | Cancelled; no retained note/assets; process and scratch cleanup completes before return. |
| Valid empty native scan text; uncertain visual content | Prepared preview with `needs_transcription=true`; existing explicit Luna/coverage behavior. Deterministic conversion records missing transcription. Never synthesize native text or claim full coverage from a blank flag. |

Use the existing bounded `{"error": "..."}` envelope. Add only stable allowlisted PDF failure reasons (for example `pdf_budget`, `pdf_unavailable`, `pdf_invalid`) and exact host classification where needed; do not pass through Xberg errors, filenames, document text or diagnostics. The current host recognizes budget keywords and otherwise maps child errors to Invalid, so richer categorization needs a deliberate small change, not a claim that it exists today. Unrecognized/nonzero output still fails closed. No source payload in logs or new telemetry.

## Proposed production file ownership (later assignment only)

All paths below are relative to the repository root. Existing paths were inspected; **new** paths are proposals. C must allocate shared paths after revalidation. None is edited by gate 3.

| Files | Proposed change |
|---|---|
| `crates/brn-intake/Cargo.toml`; root `Cargo.lock` (C-owned) | Optional exact Xberg dependency/helper feature; reviewed combined resolution only. Root workspace manifest need not change. |
| `crates/brn-intake/src/helper/pdf.rs` **new**; `src/helper/prepared/pdf.rs` **new**, within that crate | Shared bounded PDF reader and schema-1 projection; schema-2 page/preview projection. Reuse existing image/usage helpers. |
| `crates/brn-intake/src/helper.rs`; `src/helper/prepared.rs` within that crate | Standalone PDF dispatch and module wiring; existing schemas' usage/finalization paths. |
| `crates/brn-intake/src/lib.rs`; `src/bin/brn-intake-helper.rs` within that crate | PDF converter constant, bounded error reasons if needed. Preserve schemas and restrictions/startup order. No Xberg types in public DTOs. |
| `crates/brn-threads-intake/src/{lib,prepared,process}.rs`; `Cargo.toml` | Both PDF routes through the helper; strict result/converter/page validation; remove bbox parsing, Poppler invocation and only-obsolete process profiles/fontconfig. Remove direct host quick-xml dependency when its last caller is gone. |
| `crates/brn-threads-intake/src/pdf_tools.rs` | Retire file and `PdfTools` API after caller search; no executable discovery/override route remains. |
| `crates/brn/src/main.rs`; `crates/brn-desktop/src/threads_ui.rs` | Remove `PdfTools` construction only, reconciling current owners' newer code. |
| `crates/brn-ai/examples/qualify_conversion.rs`; `crates/brn-ai/src/conversion_tests.rs` | Update converter construction and fixture/mock expectations only; no change to the Luna stage. |
| `crates/brn-intake/tests/{restricted_helper,protocol,maintained}.rs`; `crates/brn-intake/src/prepared/tests.rs`; helper PDF module tests; `crates/brn-threads-intake/tests/qualification.rs` and prepared/process unit tests | Add PDF helper evidence/negative cases; preserve meaningful source binding, coverage and lifecycle checks. Existing fixture files remain authoritative. |
| `.github/workflows/ci.yml`; `scripts/package-macos-preview.py` | Keep helper build/test/package checks; retire Poppler install/export requirement and `external_pdf_tools` manifest entries after verified cutover. Verify `scripts/make-macos-app.sh` still bundles the matching executable; no change expected to its bundling contract. |
| `docs/development/{setup,verification,preview-release}.md`; `docs/architecture/{dependencies,overview}.md`; `crates/{brn-intake,brn-threads-intake,brn-desktop}/README.md`; C-owned status/plan records | Update dependency, build, installed-tool, size/license and qualification instructions together. Preserve historical backend evidence/readers. |

No planned database/schema migration, core/store/retrieval change, new provider feature or broad workspace refactor. Keep `brn-ai/src/conversion.rs`, formatter/checker behavior and host save implementation unless C finds a separate demonstrated integration defect requiring its own ownership decision.

## Acceptance and verification for the later approved slice

These are **pending implementation gates**, not results of this written task. Preserve existing qualified non-PDF cases. Use deterministic synthetic fixtures and mocked provider plans in CI; provider/native UI runs require their own current authority.

| Seam | Keep/add evidence and acceptance |
|---|---|
| Restricted executable, not just in-process parser tests | Extend `restricted_helper.rs` to run both maintained PDFs in both modes with empty environment under the exact helper profile; keep restriction-before-input, existing forbidden synthetic file/live-loopback witness, environment refusal and child/grandchild reaping. No scratch/font/cache write permission added to pass. |
| Prepared source evidence | Harbor's existing 19-item inventory and process source QA/exact [inventory](source-inventory.json): complete prose, all 12 row-associated cells including `--` versus `0`, leading zeros, nesting, references, scan's nine units and complete vector diagram. Compare every page's native lines/order, not keywords. Keep raw process-table difference explicit. |
| Receipts and preview crops | Keep source/hash/preview mismatch tests, complete permutation acceptance and duplicate/missing/out-of-range rejection, unchanged values and crop bounds/hash checks. Replace Poppler bbox mechanism tests with native evidence tests; keep delayed-label/footer association assertions. Retest five legible previews, page geometry, rotated/CropBox pages, correct crop scaling and empty scan flag. |
| Embedded images | Both fixture images' page associations/dimensions and exact decoded RGB hashes from gate 2; full PNG decode. Add repeated-image occurrences across pages, image-only pages, masks, corrupt/unsupported images and missing associations. Native vectors stay represented through previews/crops. |
| Host qualification/failure lifecycle | Keep `qualification.rs` ordinary PDF/DOCX inventories, FullNote/useful intent, Partial/Unchecked/omitted-figure coverage, no original payload, bounded reads, malformed input, budgets/timeouts, worker death and cleanup. Replace `cancellation_after_pdf_copy...`'s disk-copy trigger with a deterministic helper-handoff barrier; test cancellation while parsing/rendering and after completion before save. |
| Resource abuse | Truncated/password-protected PDFs, excessive pages/objects/images, huge/nonfinite/rotated geometry, decompression expansion, malformed streams, output overflow, CPU/wall deadline and allocation/pixel boundaries before render/decode. Record peak RSS and exact enforcement; unsupported enforcement remains a cutover blocker. Restart cleanup preserves unrelated files and never follows symlinks. |
| No hidden old path | Run qualification without Poppler installed/available and with synthetic canary executables/obsolete override values. Assert no converter invocation. A missing/old helper must refuse; it cannot select another backend. Search shipping callers and packaging/docs for retired requirements. |
| Graph/platform/package | Locked combined helper + native UI + native retrieval graphs; no Xberg OCR/pdfium/ORT addition or native/model downloader. Network-denied offline build/run, native script/license audit, `otool -L`, actual helper/app size, uncontended build and per-fixture latency/RSS. Verify matched helper bundling, launch and target architecture/signing. Other platforms retain explicit refusal. |
| Unchanged model/host seam | Existing mocked fixed Luna/account/high-effort, tool-free plan tests; deterministic plans through actual candidate checker and checked save. General/private/project scope, protected note, replay after lost response, cancel-before-save, Undo, backup/restore and offline content after original removal must pass at C's new baseline. No original binary persisted. |

After authorized implementation, the initial focused commands at the inspected baseline would be:

```sh
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo test -p brn-intake --features helper --all-targets --locked --offline
BRN_INTAKE_HELPER="$PWD/target/debug/brn-intake-helper" cargo test -p brn-threads-intake --all-targets --locked --offline
cargo test -p brn-ai --features capability-spike --all-targets --locked --offline
git diff --check
python3 scripts/check-markdown-links.py
```

Use the pinned toolchain, controlled target directory/build slots and C's current CI/verification commands; adapt `BRN_INTAKE_HELPER` to an explicit target directory if used. Require actual fixture execution, not a skipped/zero-test pass. Broaden to relevant native combined-build/host checks after focused gates pass. A separately authorized live Luna comparison must establish full-note source fidelity with the new previews/line receipt behavior; mock success and gate-2 raster inspection are not that evidence. No live/provider/UI qualification was run for gate 3.

## Rollback, open risks and recommendation

Before cutover, C records a clean qualified rollback commit and ships host/helper as one matched artifact. Revert the whole later PDF cutover, root lock and accompanying packaging/docs together if mandatory fidelity, restriction, resource or host gates fail. Restoring a Poppler-based release restores its explicit installation prerequisite; it is an intentional release rollback, never an automatic failed-job fallback. Retry an interrupted conversion from an explicitly supplied external original; do not reuse receipts with changed previews/line order or replay an already applied import as a new mutation.

Existing persisted notes/assets, converter strings, replay receipts and Undo history remain readable; no migration or reconversion of stored knowledge is planned. Preserve any readers needed for historical results. Discard/reprepare in-flight evidence when switching versions. Ordinary checked Undo remains the route for reversing an accepted import, not backend rollback.

| Residual risk | Required disposition before shipping |
|---|---|
| Production restrictions differ from experiment; font substitution already visible | Prove both fixtures inside the unchanged helper profile, investigate denied font reads without broadening access, qualify representative missing fonts/scripts and retain visible uncertainty/refusal. |
| Native renderer returns PNG after internal work; library limits do not establish hard RSS | Verify actual decode/render allocation and expansion boundaries plus process containment. C/owner must resolve any enforcement gap; no unsupported hard-memory claim. |
| Two PDFs/five pages, unusual layouts unqualified | Extend targeted adverse/rotated/layout evidence; retain bounded support, strict diagnostics and independent inventory coverage. No universal math/scan fidelity claim. |
| Table-column order and new raster hashes | Keep current receipt grammar/checker; verify actual later Luna output against full source inventory. Do not alter the prompt/model stage or embed fixture-specific reordering to make a test pass. |
| Broader codec/dependency graph, startup/size and notices | Recheck combined feature/link graph and platform build, actual helper footprint/latency/RSS and distribution notices. No dependency deduplication/pin upgrade bundled into adoption. |
| Existing workflow changes and no C0 ref yet | Reconcile source/tests/ownership at C0 after workflow landing; rerun affected evidence and obtain explicit owner approval for the separate integration slice. |

**Recommendation: bounded adaptation, design complete; keep current production behavior until the separate approved slice satisfies these gates.** Xberg can supply the required local PDF evidence without pdfium/OCR/downloads on the qualified fixtures. Adoption remains conditional on exact-helper containment, resource/fidelity checks, unchanged Luna/checked-save behavior and a coherent removal of all three Poppler executables. This document authorizes no production edits, builds, provider calls, downloads, push, PR or merge.
