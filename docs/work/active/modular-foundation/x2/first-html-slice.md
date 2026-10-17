# Principal first HTML/common implementation proposal

17 October 2026. Review proposal, **not yet an implementation/heavy grant**. Owner has already authorized Xberg for all documents; no repeated scope approval is needed. This is the next independently testable component, not all-document adoption or a narrowed replacement for existing imports.

## Outcome and boundary

Add a real Xberg-backed **standalone schema1 HTML helper capability**. `helper::extract` currently rejects kind=html; `Converter::convert(Html)` remains Unavailable and the richer prepared HTML reader remains unchanged in this slice. The new helper branch must be a normal exercised implementation, not an unused facade, cfg(test)-only extractor or backend-selection flag. Do not accidentally enable nested EML HTML dispatch. Host wiring and prepared rich HTML cutover receive their later exact fidelity allocation.

Use xberg1.3.10, defaults false, ONLY html. This introduces neither PDF operations nor Excel/Tokio/mail-parser pin movement. The same private documents boundary will serve the later format families. Text/Markdown remain byte-exact Verbatim with no helper/provider requirement.

## Exact proposed ownership after review

- `crates/brn-intake/Cargo.toml`: add optional exact xberg dependency and helper feature activation only.
- Root `Cargo.lock`: lead-controlled actual graph reconciliation. No scratch-lock copy. Every new package/version/checksum maps to source1a/source1b or a separately audited exact source before ANY build. Preserve existing versions/checksums; if resolver demands movement, report exact delta before compilation. Record effective normal/helper/default/native/retrieval features and distinguish metadata from emitted units.
- `crates/brn-intake/src/helper.rs`: private documents registration plus standalone HTML dispatch and existing finalization/accounting reuse. Existing DOCX/PPTX/EML behavior remains unchanged.
- NEW `src/helper/documents.rs`, NEW `src/helper/documents/html_admission.rs`: private BRN-owned extraction result/config/admission. No public Xberg DTO, parser registry, new service or protocol.
- `crates/brn-intake/src/lib.rs`: only an HTML-specific stable converter identifier if needed; no schema/limits/validation change and no replacement of the legacy converter constant.
- Existing `tests/restricted_helper.rs` / `tests/protocol.rs` and narrowly new module tests: actual executable positive/adverse cases. Existing prepared HTML source/tests are read-only; use their rich inventories as future cutover criteria.
- X2 docs plus existing notices/graph receipt documentation only. No CI/packaging/host/core/AI/app/CLI/dependency pin expansion in this slice without an exact need.

## Admission and actual backend

1. Validate existing request limits/nonempty bytes, strict UTF8/no NUL and <=existing8MiB HTML ceiling before backend work. Use maintained html5ever tokenizer for a bounded pre-admission pass before Xberg DOM allocation. Bound source bytes, depth128, tokens/nodes200000, cumulative text/attributes/entities and request expanded/decoded/output accounting with checked arithmetic. Do not build a second full text converter. Source token limits must conservatively handle malformed/implied-close structure; reject ambiguity rather than accepting an underestimated DOM.
2. First engineering profile admits only the explicitly tested textual HTML constructs (headings, paragraphs, lists, quotes, code, Unicode and inert links). Tables/images/embedded resources or richer semantics outside that profile refuse explicitly before Xberg; never silently drop them or route to the old extractor. This refusal is acceptable only on the previously unsupported new schema1 route. Existing prepared HTML tables/figures/spans up to10000 remain intact.
3. Call actual `xberg::extract` on exactly one bytes input with exact text/html MIME. Explicit None timeout, no file/URI/batch/child auto-extraction/cache/OCR/model/rewrites, limited concurrency Some(1) before first extraction. Use maintained output/structure configuration and actual returned content/semantic nodes; do not synthesize expected text around a no-op backend call.
4. Project into existing Extraction1/source/usage DTOs with exact original hash/bytes and honest converter provenance. Retained output is bounded and validated; no source-supplied file reads/network. Static safe errors only. Source/output integrity and no truncated-prefix success must hold.
5. Actual helper restrictions, CPU10/NOFILE64/FSIZE and external deadline/PGID cleanup remain unchanged. No hard RSS claim. HTML >=512KiB uses the upstream joined16MiB-stack thread; prove useful restricted operation and measure threads/RSS/latency. Source-inspected Rayon target1 is not an OS-enforced memory/thread cap.

## Meaningful checks and stop conditions

- Genuine before: new HTML request through the actual old restricted helper fails unsupported. A missing Rust API/compile failure is not behavioral red.
- Actual Xberg nontrivial textual positive, exact ordered Unicode/source facts and integrity, plus >=512KiB joined-thread path. Stable output contract tests should assert relevant complete facts, not a few keywords or fixture-specific reorder.
- Explicit table/image/embedded-resource refusal; current prepared rich HTML regression remains passing and byte-identical source. UTF8/NUL/depth/token/entity/attribute/output/adverse quotas, malformed structure, no retained partial success.
- Real unchanged restricted executable/env_clear: no file/network/child effects; host-owned process bounded pipes/deadline/cancel/reap checks. Preserve original safety assertions, no relaxed sandbox/quota.
- Build helper first after graph audit; all helper targets/protocol/restricted tests and affected strict lint/fmt, then relevant existing prepared/intake regression. Use actual nonzero filters and source-bound artifacts/logs. No live provider/native/model download/mutants.
- If pre-admission cannot conservatively bound the actual DOM/output operation, stop that operation with the exact missing seam. Do not trade the PDF unsafe path for an unexamined HTML one.
- Freeze a substantive candidate for fresh independent actual-diff review. Existing PDF resource STOP, Office/XLSX/MIME fidelity gates, full rich HTML cutover and all-document completion remain explicit future work.

## Prior findings and preserved limits

Read complete Opus1/Opus2 and responses. B1/B2/B4 were closed by Opus2; verify B3 runtime correction against source1b/runtime.md without reopening unrelated items. Source1b is acquired/verified, not a compile/runtime pass. PDF Brotli/open/default count/image/geometry gaps are still blockers to PDF reader hunks. Native Excel requires the later bounded helper-local Tokio allocation and requalification of every previously admitted kind. No exclusion of Excel or permanent HTML-only scope.

Earlier PDFUnavailable, unexplained SDK/widget failures, platform/hosted/native-human/INT-W/main/owner/program qualifications remain. No new paid start or source-quality acceptance is inferred.

