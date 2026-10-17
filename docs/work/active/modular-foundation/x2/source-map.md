# Current source reconciliation (65684dcb; all-document scope)

[All-document route/feature map](formats.md) is controlling; PDF rows below detail the first engineering slice. All line references are to the recorded base, not X1's7e9d0376 snapshot. [Binding](binding.json) verifies334 compiled inputs/3INT-D scripts unchanged and ancestry. [X1 evidence](x1-evidence.json) hashes all135 objects at65715ee, including the original DESIGN/REPORT/manifest/lock/probe/native audit. These read-only observations are not executed conversion qualification.

| Current path/symbol | Exact seam and consequence |
| --- | --- |
| `crates/brn-intake/Cargo.toml:8–35` | defaults=[], helper optional parser dependencies; binary requires helper. Add only optional exact Xberg/pdf edge and dep:xberg activation. DTO default consumers do not call a parser. |
| `brn-intake/src/bin/brn-intake-helper.rs:11–18,53–80,83–139` | restrictions before input, empty environment, one bounded HelperRequest, normal Extraction or --prepare PreparedDocument. macOS deny-default permits system runtime reads, process-fork/sysctl, dev/null only; CPU10/NOFILE64/FSIZE64MiB; other platforms refuse. Unchanged profile is mandatory. Serialization currently uses to_vec before output length check; output quota is not a general allocation limit. |
| `brn-intake/src/lib.rs:9–20,25–121,198–265,399–407` | existing limits/usage/request/Extraction, schema1 root original hash/bytes and collection validation, helper feature module, bounded PNG validation. No public Xberg type or DTO schema change needed. |
| `helper.rs:25–116,194–245` | extract(HelperRequest)->Result<Extraction,String>, input/admission/usage/output finalization; currently standalone DOCX/PPTX/EML only. image_asset bounds PNG/JPEG decoding; this cannot bound earlier native PDF decompression. Add separate pdf dispatch without admitting PDF MIME attachments. |
| `helper/prepared.rs:19–43,44–152,168–184,225–280` | prepare(HelperRequest)->Result<PreparedDocument,String>, format dispatch, measured aggregate usage/serialized size and validate. Add pdf adapter before common finalization; preserve other formats. |
| `src/prepared.rs:5–94,165–323`; `prepared/formatting.rs:437–447,828` | schema2 PROFILE, PageText/PreparedPage, <=32pages/assets and2048blocks, exact preview/hash/geometry validation and typed complete source-line permutation receipts. Reuse, do not weaken or hardcode X1 permutation. |
| `brn-threads-intake/src/lib.rs:38–77,80–98,183–235` | import_limits(Pdf)=20s; read_input_file(path,max,&AtomicBool) bounded/cancellable; Converter{helper_path,pdf_tools,temp_root,limits}; convert(ConversionRequest,&AtomicBool)->Result<ConversionResult,ConversionError>. Final cancellation/deadline and explicit cleanup. |
| `lib.rs:242–347` | document helper schema1 request over pipes; validates exact existing converter/source/limits/usage; projects only readable coverage/assets, discarding original SourceNode.bytes. Generalize explicit kind to pdf and select PDF constant only there. |
| `lib.rs:375–575`; `pdf_tools.rs:4–75` | current schema1 PDF writes input.pdf; pdftotext-layout, pdfimages-list/png; page:N text and image gaps; explicit tool discovery/overrides. All this is live, not CL dead code. Replace only after helper qualification. |
| `prepared.rs:11–77,270–395,418–440,459–797` | schema2 PDF exception bypasses helper, writes input.pdf, bbox text and pdftoppm preview; bbox geometry/isolated-row logic. Non-PDF helper path already has exact source/limits/input binding. Route PDF through that path, add PDF-specific complete page validation, then retire exception/bbox/renderer discovery. |
| `prepared.rs:81–268,798–911` | finish_candidate(request,PreparedDocument,ConversionCandidate,&AtomicBool) retains linked assets/crops, removes full-page previews, checks crop coordinates/limits/deadline/cancel/inventory. Keep behavior and receipt checks. Prepared gaps currently become Limitations: missing substantive page/glyph/render evidence must refuse upstream. |
| `process.rs:21–217` | run(executable,args,input,temp,ProcessBudget,cancel,restrict)->Result<Vec<u8>,ConversionError>; absolute canonical executable/env_clear/PGID/bounded pipes/deadline/quota/joins/group reap. Helper=false means helper self-restriction, not an unrestricted parser API. |
| `process.rs:45–70,111–129,228–311` | renderer/restricted external-tool profiles, Homebrew and fonts.conf handling are Poppler-specific replacements to retire. No importing these broader allowances into helper restrictions. Nonzero helper currently maps budget-like reason to Budget, otherwise Invalid. |
| `transient.rs:12–117` | private UUID/0700 directory, bounded file entries/bytes/symlinks, explicit cleanup+Drop and24h owned-name expiry. Keep unchanged; no source-path input or raw files required for new worker. |
| `brn-ai/src/conversion.rs:26–41,131–218,330–409` | MODEL=gpt-6-luna, fixed High, Chatgpt client from explicit host auth_file/credentials_dir,180s provider deadline, max1model turn, no tools/history/fallback. Local prepare then model then finish_candidate with cancellation between stages. General chat settings do not select this model. No change proposed. |
| `brn/src/main.rs:922–935,1515–1572` | constructor discovers PdfTools (remove just field/discovery), ConversionJob uses explicit Settings auth references; cancellation checked immediately before Workspace.attach_full/import_full. Useful selection continues through normal scoped runtime. Keep flow. |
| `brn-desktop/src/threads_ui.rs:1–3,3310–3345,3570–3581` | import/conversion constructor mechanical PdfTools removal; completed conversion rechecks both cancellation flags before existing checked attach/import. Preserve worker/queue/writing/navigation logic. |
| `brn-threads-app/src/intake.rs:143–177,245–398` | import_full/attach_full/import_full_scoped use shared Workspace checked operation; asset digest, scope/source identity/protection/replay retained. No app production edit proposed. |
| `scripts/package-macos-preview.py:54–105`; `make-macos-app.sh:54–64,116` | matches helper/app architecture/minOS/system links/signing, bundles helper; packaging currently advertises external_pdf_tools. Coherent later removal plus notices and real relocated helper PDF test, no release now. |
| `.github/workflows/ci.yml:68–79,203–219` | Poppler installation/export in Mac core lane; helper build and native/retrieval combined lanes. Retire only obsolete Poppler setup after replacement qualified, keep helper and meaningful build/test jobs. |

## Upstream API evidence and gaps

Git-pinned `src/bin/xberg_probe.rs:49–86` actually built configuration/ExtractInput::from_bytes and xberg::extract;117–128 actually built `xberg::pdf::render::{install_pdf_render_diagnostics,take_xberg_native_pdf_render_warnings}` and `PdfRenderSession::{open,page_count,render_page_to_png}`. It used150DPI, then Python downsizing in comparison tooling. That is **not** a shipping Rust resize or preallocation proof. No new wrapper should guess that PdfRenderSession exposes dimensions/allocation callbacks not shown here.

Historical native-audit lists `xberg/src/pdf/render.rs`, `pdf/native/images.rs`, engine/{mod,extract_impl}.rs and native rendering/text_rasterizer.rs/vendor/fontdb/system_fonts.rs. Their current pinned implementations are UNVERIFIED because source/archive is absent in documented src/cache roots. Read them after exact acquisition before deciding enforceable decompression/object/page/raster/font limits. The X1 runtime.sb allowed explicit fixture/output, /Library/Fonts,/usr/share and random device reads; current helper does not. X1 pass cannot replace exact-helper evidence.

## Dependency acquisition prerequisite

Registry: crates.io. Exact archive SHA/checksums from pinned X1 lock:

| Package | Version | SHA256 |
| --- | --- | --- |
|xberg|1.3.10|f9859ca06d9ee171c65c91cfbb0a25287a45622e7628f1b45ba5353b473b6f0d|
|xberg-native-pdf|1.3.10|c574e362d731d6c66c47275ec6b16e3acccef3c2fd465da973bcf424cd6fa1fc|
|lopdf|0.45.0|bfffda0fe1ab0157e1a13c14bebd3f28671f2fccb7922f0722ec53926e6922d3|

CARGO_HOME is unset; recorded X1 metadata paths all point into ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f. Both default src/cache registry directories were enumerated for these exact names; no match. This is a bounded documented-location inventory, not a claim that no other disk/cache exists. No downloads/acquisition/resolution/compilation occurred. All transitive acquisitions and checksums must follow principal-approved combined graph reconciliation, not copying the old experiment lock into root.
