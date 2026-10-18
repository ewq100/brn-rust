## Review

**Verdict:** `CHANGES_REQUIRED`. One narrow correction to the documents is needed. Source1b can still run now.

This review covered the frozen `cd2da52c` documents and the source1a evidence. The documents rework the plan from PDF-only to all documents. Source1b work that started during this review is out of scope.

**Checks performed (all read-only):**
- `cd2da52c:crates` and `65684dcb:crates` are both `bdc0a22a`. Both diffs touch only documents.
- The xberg, native and lopdf archive hashes match the documents. So do the X1 manifest and lock hashes.
- Several facts in the plan match current source:
  - **Lock and helper pins:** Tokio is locked at 1.53.1, and the helper pins mail-parser at exactly 0.11.8.
  - **Line references:** the converter string `CONVERTER` is at `lib.rs:20`, its host check at `threads-intake lib.rs:279`, and the test module at `conversion.rs:475–477`.
  - **PDF resource gaps:** Brotli ignores its output cap. Xberg calls `lopdf` `load_mem` without bounds at `pdf/mod.rs:238` and `:264`. The render DPI floor is 72, and `render_page_fit` exists.
  - **PDF tool references:** `pdf_tools.rs` is 137 lines, and `BRN_PDFTOPPM` appears at `prepared.rs:419`.

**Blocking finding:**

- **B3 residual: a source contradiction about Tokio and the PDF path.**
  - `xberg-1.3.10/src/extractors/pdf/mod.rs:301–310`: when the `tokio-runtime` feature is on, `spawn_pdf_blocking` always calls `tokio::task::spawn_blocking`. There is no batch-mode check.
  - It is called from `extract_core_native` (`:2074`). That is the default native backend for every single-input PDF (`:1986/:1994`).
  - In xberg's `Cargo.toml`, `excel` enables `tokio-runtime`. `pdf`, `office`, `email` and `html` do not.
  - So any helper built with both `pdf` and `excel` must run PDF extraction inside a Tokio runtime. The planned std-Wake poll would panic.
  - The documents say otherwise: formats.md step 3 ("not unconditional runtime necessity"), resources.md ("runtime is not proven unavoidable"), and opus1/response.md's B3 row, which checked only the DOCX, PPTX and Excel batch branches.

  **Correction criteria:**
  1. Record this source fact in resources.md (feature-unified section), formats.md step 3, plan Step 2 bullet 2, and the "Feature-unified runtime" row of acceptance.md.
  2. State that enabling `excel` alongside PDF has a known prerequisite: a separately allocated, helper-local runtime. Its flavor, blocking-pool thread limit, stack size, thread creation under CPU/NOFILE limits and sandbox behaviour must all be requalified. The alternative is an explicit decision to sequence or exclude Excel. Using `excel-wasm` to avoid this is not acceptable, and no runtime is needed now.
  3. Add to source1b's scope: audit every `tokio-runtime`-gated call reachable from each admitted single-input path.

**Advisories:**
1. **Stale handoff.** `x2/handoff.md` is unchanged since `98b` and still says X2a, including the PDF projections, is ready. It also still describes the sources as missing from the cache. When the file is unfrozen, add a dated "superseded" note pointing to `source-audit/handoff.md` and `opus1/readiness.md`, keeping the original text.
2. **Retirement script.** `check-intake-retirement.py` has no Poppler checks today, so "extend" means adding them. It needs tokens for `PdfTools`, `BRN_PDF*` and all three tool names. It must also cover `package-macos-preview.py:77` and the `ci.yml:77–78` exports, because it currently scans only `crates/*.rs`. It also bans `roxmltree::` and the OOXML namespace strings in production code. Future X2c projectors must comply, or get a reviewed exception rather than a weaker check.
3. **Cache moved after `cd2`.** cfb 0.15.0, mail-parser 0.11.9, outlook-pst 1.2.0, Tokio 1.53.2 and zip 9.0.5 appeared at 23:14:45, after the `cd2` commit at 23:10. Being in the cache does not mean they have been audited. The "missing" statements in `cd2` remain dated history.
4. **HTML closure.** `pdf-native` already includes `html`, and the X1 lock already contains html-to-markdown-rs 3.18.1. An HTML-only slice therefore adds no packages beyond X1 and no Tokio. Say so explicitly when HTML is planned.
5. **Thread accounting once a runtime exists.**
   - Keep the extraction timeout set to `None`; the host deadline stays authoritative.
   - Pipeline initialization also calls `spawn_blocking` when a runtime is present (`initialization/mod.rs:436`). Count those threads.

**Spec coverage:**

| Item | Status |
| --- | --- |
| Full owner scope supersedes the PDF-only plan | Covered (plan header, owner-scope, modular plan link) |
| B1: every root-lock addition mapped by checksum | Closed (plan 1c, allocation, acceptance) |
| B2: source1a vs source1b; non-PDF findings partial; no old-version substitution | Closed |
| B3: explicit `None` timeout, requalification of every admitted kind | Partly closed; residual blocker above |
| B4: `conversion::tests` filter, non-zero count with PDF/Luna mock | Closed |
| Resource table before helper code, no hard-RSS claim, honest font STOP | Covered |
| Acquisition vs root resolution vs build kept separate; mail-parser provenance and host check; image codecs/rayon; Tokio features compiled vs used | Covered |
| All 8 kinds and the extra-family map; Verbatim Text/Markdown | Covered |

**Readiness:**
- **Executable now:** source1b. That is the scratch resolution, fetch and audit for office/excel/email/html, with no root adoption and no build. Add the Tokio call-site audit to its scope.
- **Not ready (conditional):** the helper implementation. It waits on the PDF resource STOP, the root ledger (1c) and the runtime decision for the Excel combination.
- **Not qualified:** the all-document cutover.

This review grants no activation, release, main, product, owner, native-human or remote approval. Still open: the original INT-D unknowns and closed windows, plus the SDK, widget, PDF, platform, strict-Copilot, INT-W, main, owner and program gaps.
