## Review

**Verdict:** `CHANGES_REQUIRED`

**Blocking findings:**

- **plan.md Step 1, bullet 4, and allocation.md X2a root-lock row: the audited sources are not bound to the graph that will be compiled.** The audit uses the X1 lock. The root resolution pins only xberg 1.3.10, xberg-native-pdf 1.3.10 and lopdf 0.45.0, and it stops only for "a newer native transitive" or "an existing package" change. Every other newly added package can resolve to a version nobody audited. That includes the packages the resource gate depends on: decompression, codecs, rendering and fonts (flate2, miniz_oxide 0.8.9/0.9.1, brotli 9.0.0/brotli-decompressor, fax, weezl, jpeg-decoder, tiny-skia 0.12.0, font crates, html-to-markdown-rs 3.18.1). X1 is also not the future feature graph: it locked cfb 0.14.0, while office, excel and email require cfb ^0.15. **Correction criterion:** before any build, the plan requires a table of every package the root lock adds, with name, version and checksum. Each row either matches the audited X1 lock entry, or is pinned to the audited version with a recorded `--precise` decision, or has its exact source fetched and audited before the build. A build is allowed only on a lock whose additions all map to audited checksums.

- **plan.md Step 1, bullet 3, and formats.md feature sequence step 1: the plan says one acquisition can audit all formats, but it cannot.** The plan says the same acquisition will audit the office, excel, email and HTML APIs. The X1 lock contains no calamine, mail-parser, zip, Tokio or outlook-pst, and no cfb 0.15. The xberg crate itself does contain the format modules (`extractors/{docx,pptx,excel,email,html}.rs`), but those modules depend on packages this acquisition does not fetch. The owner's handoff rule says missing pinned sources must be named as a prerequisite. **Correction criterion:** split Step 1 into two parts:
  - **1a:** the X1-lock acquisition, which yields the full xberg crate plus the PDF dependency closure.
  - **1b:** a separately allocated scratch resolution for the office, excel, email and HTML closures. It records exact versions and checksums, including mail-parser 0.11.9 or later and Tokio 1.53.2 or later, and runs before any X2c/X2d contract, compatibility-gap or allocation conclusion.

  State that any non-PDF finding from 1a is partial.

- **The shared adapter's runtime design is not reconciled across slices.** Locations: plan.md Step 2, bullet 2 ("Wake poll"), formats.md feature sequence step 3, and allocation.md (`helper/documents.rs` is "reused" by later families). The published `excel` feature enables `tokio-runtime`, and Cargo features apply to the whole crate. That changes `xberg::extract` for **every** admitted kind, including PDF. In my read-only spot check of the acquired source (`core/extractor/bytes.rs:81–124`), when `extraction_timeout_secs` is set, extraction goes through `tokio::time::timeout`, and `extractors/excel.rs:398–418` uses `spawn_blocking`. The X2a std-Wake adapter would then run Tokio-dependent work with no runtime. The plan only says it will choose an Excel runtime later. **Correction criterion:**
  - X2a sets and records `extraction_timeout_secs` explicitly in the shared configuration. It must be `None`, or a source-justified value.
  - Before `excel` is enabled, X2c either proves a synchronous Excel path without `tokio-runtime`, or allocates a helper-local Tokio runtime. A helper-local runtime needs its threads, CPU limit and sandbox requalified.
  - The acceptance matrix requires that any feature addition requalifies the invocation path, through the real helper, for every kind already admitted (PDF first).

- **plan.md, planned commands, and the Luna row of acceptance.md: the Luna mock gate would run zero tests.** `cargo test -p brn-ai --features capability-spike --lib conversion_tests` filters on a name that does not exist. `conversion.rs:475–477` declares the module as `mod tests` with `#[path="conversion_tests.rs"]`, so the test paths are `conversion::tests::…` and nothing matches. **Correction criterion:** use the filter `conversion::tests`, and require a recorded non-zero count of executed tests, including the fixed Luna/PDF mock case.

**Advisory suggestions:**

- **Corrective criteria for (b), once the source facts arrive.** Allocating X2a production code should depend on the finished source-linked resource table, not on X2a's exit. The table covers `extract`, `PdfRenderSession::open`, `render_page_to_png` and native image decompression: requested limit, first allocation, real enforcement and an adverse witness.
  - If any of those operations allocates without a bound before BRN can check, the next deliverable is the principal disposition: an upstream configuration limit or an enforceable platform mechanism. Do not implement helper hunks that cannot ship.
  - Font discovery that the unchanged sandbox denies is also a STOP. The fix is an explicit, narrow principal decision, not a widened profile.
  - Plan.md's sentence saying the STOP "does not block … helper slice" should be narrowed to match.
- **Readiness wording:** Step 1 mixes acquisition, root-lock resolution and a heavy slot. Acquisition needs no heavy slot and no lock change; keep root resolution as a separate later allocation.
- **Acquisition provenance:** acquisition populated the shared default `~/.cargo/registry`, not an isolated `CARGO_HOME`. Source-map's "absent" inventory is now historical. Record this so `--offline` results are not read as cache-independent.
- **Converter provenance:** `CONVERTER` (lib.rs:20) embeds `mail-parser-0.11.8`. Any mail-parser pin change must update the converter provenance and the host's exact-match check together.
- **Image graph:** xberg's `image` dependency turns on webp, bmp, tiff, gif, pnm and rayon in the shared image 0.25.10. BRN's explicit PNG/JPEG allowlist (helper.rs:198–207) keeps admission unchanged. Still record the extra threads, code size and the new decoders as graph facts.
- **Excel's Tokio features:** Tokio's target dependency for `excel` also brings in `process`, `fs` and `rt-multi-thread`. Include them in the X2c audit.
- **Retirement checks:** `scripts/check-intake-retirement.py` already exists, so say "extend". `package-macos-preview.py:77` lists only pdftotext and pdfimages; the retirement check must also cover pdftoppm through the renderer route in process.rs.
- **Line ranges:** `pdf_tools.rs` is 137 lines, so the "4–75" range is imprecise.

**Coverage summary:**

| Requirement | Coverage |
| --- | --- |
| All 8 kinds, current schema-1/schema-2 differences, extra-family map | formats.md tables; plan Step 4. Accurate against lib.rs:207–215 and prepared.rs:23–35. Covered. |
| Text/Markdown verbatim | The existing `plain()` path, with converter `"UTF-8 verbatim"`, needs no helper. The explicit Verbatim mode is a named identity contract, not a hidden parser or fallback. It is justified by byte identity and by the helper being macOS-only. Covered. |
| Typed Office/MIME/table/formula/caption provenance, no Markdown reconstruction | X2c/X2d and formats.md compatibility rule. Covered, but the closure source prerequisite is blocking finding 2. |
| xberg =1.3.10 exact, defaults off; mail-parser and Tokio conflicts | Covered precisely. Source-to-lock binding is blocking finding 1; the cross-kind runtime is blocking finding 3. |
| Unchanged helper restrictions and limits; no hard-RSS claim; real STOP gate; no pre-parser, allocator or relaxation | Resource STOP gate; matches helper.rs:83–139. Covered; sequencing is in the advisories. |
| Two PDF projections: binding, geometry, inventories, PNG/crop, no reorder, warnings never count as success | Step 2 and acceptance rows. Covered. |
| Host deadlines, cancellation, cleanup; Luna, account, scope and Undo untouched | X2b and acceptance lifecycle/host rows. Covered, apart from the test filter in blocking finding 4. |
| Retirement of all 3 Poppler tools, rollback, packaging, notices | X2b and packaging allocation. Covered. |
| File hunks plausible in current source | Spot-checked. Constructors, dispatch, tests and fixture hashes match. Covered. |
| Meaningful per-stage tests; missing sources named as prerequisites | Mostly covered; blocking findings 2 and 4. |

**Readiness:**

- **(a) Acquisition and source audit: READY.** It is safe with the original X1 manifest and lock, no build scripts and no lock edits. **Provenance:** the untracked `x2/source-audit/` was written by the parallel worker, not by this review:
  - At 22:59 it ran `cargo +1.98.1 fetch --locked` with the scratch X1 manifest and lock, for target aarch64-apple-darwin, and it exited 0.
  - The manifest and lock hashes equal the X1 Git blobs, and all three archive SHA256s match. I recomputed both.
  - It contains acquisition and hash evidence only, not an audit. I did not rely on it as passing any source gate.
- **(b) Helper implementation: NOT READY.** It needs blocking findings 1–3 resolved and the completed resource table.
- **(c) All-document cutover: NOT READY.** Unresolved: the resource, sandbox and graph gates, the non-PDF closure sources, the mail-parser and Tokio dispositions, and real fidelity/host evidence.

This review approves no implementation, cutover, build, provider call or shipping.
