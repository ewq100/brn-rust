# BRN X2 all-document Xberg plan: Copilot Opus review round 3 (read-only)

**Verdict: CHANGES_REQUIRED for the first HTML slice.** The fixes are narrow and doc-level only (Q1–Q3 below). The B3 correction closes at the plan level.

**Binding:** I reviewed the worktree `/Users/evokessler/.t3/worktrees/brn-rust/t3-9527c407` on branch `work/x2-intake-20261017` at HEAD `73ea9ca777e00d348966e37e8402f3eb68ecb20b`, which is clean. The change since `137726e` touches only `first-html-slice.md` (new) and `plan.md` (resequencing). Production `39a67e4`/`bdc0a22` and root main were not touched.

**Method:** I read primary docs, BRN source, Git and the exact upstream registry source (xberg-1.3.10, html-to-markdown-rs-3.18.1, astral-tl-0.8.0). I ran no cargo, metadata, fetch, build, test, network, provider or store commands, made no edits and delegated nothing.

## 1. Verdicts

| Area | Verdict |
|---|---|
| Source audit (source1b) | **Complete** as a static acquisition and source audit: 363 identities checked and the selected library sources are present. This does not cover the root graph, the emitted lock, compilation or runtime. |
| B3 plan correction | **Closed.** Runtime proof is properly left for later (§2). |
| First HTML engineering slice | **CHANGES_REQUIRED.** After Q1–Q3 are fixed in docs and the actual root checksum-ledger audit is done before compiling, it is ready for a precise grant. |
| PDF helper implementation | **NOT READY.** The PDF resource STOP stands: native open, Brotli, default lopdf count, image decode, geometry and font preallocation. Also required later: the helper-local Tokio allocation and requalification of every kind. This is not a new rejection of B3. |
| Production all-document cutover | **NOT READY.** Still open: PDF, XLSX/Office/MIME fidelity, the full rich-HTML gate, wiring the host converter id, and native, owner and main qualification. |

## 2. Prior findings

| Finding | Disposition |
|---|---|
| B1 (package ledger) | Stays closed. The process rule still applies to the real root graph (see Q3). |
| B2 (non-PDF source closure) | Stays closed. Source1b provides the closure. |
| B3 (Tokio runtime for native PDF) | **Closed at plan level.** Details below. |
| B4 (real nonzero `conversion::tests` filter) | Stays closed. |

B3 details:
- **Source facts are recorded correctly.** Under `tokio-runtime`, native PDF calls `spawn_blocking` unconditionally (`pdf/mod.rs:301–310`, `1986/1994`, `2074`), and Excel enables Tokio.
- **The required allocation is right.** It is a helper-local current-thread runtime with these settings:
  - `max_blocking_threads(2)` and 16MiB stacks;
  - a single `block_on`, no timeout (`None`), no URI and no batch;
  - Rayon set to `Some(1)` before the first extraction;
  - the joined 16MiB HTML thread and the cold pipeline init are both counted.
- **No evasion.** Native Excel is neither excluded nor swapped for excel-wasm.
- **Proof is left for later.** All-kind CPU10/NOFILE64, sandbox, thread and shutdown evidence remains mandatory before PDF or Excel enablement.

## 3. Must-fix blockers for the first slice

### Q1 (P1): html5ever-based pre-admission does not bound the parsers Xberg actually runs

**Source.**
- `first-html-slice.md` step 1 uses the html5ever tokenizer with depth 128 and 200000 nodes.
- Xberg's HTML path goes `extractors/html.rs` → `extraction/html/converter.rs` → html-to-markdown `convert_api.rs::convert_inner`, with these stages:
  1. `normalize_input`;
  2. inert-body and script repair, plus an optional html5ever-0.40 repair tree that may continue on the original structure;
  3. `has_head_content_outside_head` (calls `tl::parse`);
  4. the tier1 byte scanner;
  5. tier2 `tl::parse` (astral-tl) followed by a recursive walk.
- astral-tl (`parser/base.rs`) behaves differently from WHATWG/html5ever:
  - It skips whitespace after `<`, so `< img …>` and `< table>` become elements, while html5ever reads them as text.
  - It has no implied closes, so repeated `<p>`/`<li>` nest.
  - An end tag pops only when its raw name exactly matches, so `</DIV>` or `</div >` never pop.
  - It parses tags inside raw-text elements.

**Failure mode.** Refused constructs (`< img`, `< table`) get past an html5ever-token profile check. The real tier2 DOM depth and node count can exceed the html5ever counts, so the "conservative" bound is not conservative.

**Smallest correction.** Restate step 1's criterion: the admitted language must be one where html5ever, astral-tl and the tier1 scanner provably agree. For example, a byte-level allowlist scan that requires:
- every `<` is immediately followed by a lowercase allowlisted name, `/name>` with no whitespace, or `!--`;
- explicit exact-match closing of every non-void element, with no implied-close reliance;
- no raw-text elements (script, style, textarea, template, noscript, math, svg);
- a node bound of at most 2 × count(`<`) + 1;
- depth taken from the balanced scan stack.

Then require differential tests (html5ever versus `tl` versus Xberg output) on adverse inputs. Otherwise apply the slice's own STOP: "missing seam".

### Q2 (P1): upstream defaults and dropped warnings allow silent truncation or content loss

**Sources.**
- **Default depth is 64.** `html-to-markdown options/conversion.rs:12` sets `NATIVE_STACK_SAFE_DEPTH=64`. `main_helpers.rs:29–30` uses `max_depth=None` → 64, and an explicit value is clamped to 1024. Deeper nodes are skipped and only a `DepthLimitExceeded` warning is pushed (`converter/main.rs:~207`).
- **Xberg drops those warnings.** `extraction/html/converter.rs:~150–175` (`convert_single_pass`) keeps only content, metadata, tables and document, and nothing in Xberg's HTML modules consumes `ConversionResult.warnings`. The repair-budget `TruncatedInput` warning is lost the same way.
- **Preprocessing removes content by default.** `ExtractionConfig::default()` (`core/config/extraction/core.rs:591+`) sets `html_options: None`. `resolve_conversion_options` (`converter.rs:34–48`) then enables preprocessing, and `PreprocessingOptions::default()` is enabled with the Standard preset (`preprocessing.rs:84–92`). Standard drops `<nav>`, `<form>`, page headers, and hinted header/footer/aside elements (`preprocessing_helpers.rs:608–660`). Even when options are supplied explicitly, `..Default::default()` keeps preprocessing enabled.
- **Other defaults:** `use_cache: true`, `enable_quality_processing: true`, `extraction_timeout_secs: Some(600)`. The timeout is ignored without Tokio (`bytes.rs:117–129`).

**Failure mode.** The slice admits depth 128, but Xberg's default (64) skips deeper subtrees and reports success with no warning. That violates the slice's own "no truncated-prefix success" requirement. Preprocessing can also silently remove content.

**Smallest correction.** Step 3 must pin the exact config instead of "maintained output/structure configuration".

Exact `ExtractionConfig` settings:

| Setting | Value |
|---|---|
| `extraction_timeout_secs` | `None` |
| `use_cache` | `false` |
| `enable_quality_processing` | `false` (or prove it is a no-op) |
| `max_concurrent_extractions` | `Some(1)` |
| `content_filter` | `None` |
| `output_format` | Explicit |
| `html_options` | `Some(..)`, with the settings below |

Inside `html_options`:
- `preprocessing.enabled = false`;
- explicit `max_input_size = Some(8MiB)`;
- explicit `max_depth = Some(N)`, where BRN's conservative depth (from Q1) is below N and N ≤ 1024. Or keep BRN depth below 64 and use the default.

Admission must make the depth and repair-budget warnings unreachable. Add a test at depth N−1 / N / N+1 showing refusal before Xberg rather than a truncated success.

### Q3 (P2, fix before the grant or the grant stops at step 0): root-lock movement is foreseeable but not dispositioned

**Source.**
- Root `Cargo.lock` has `serde_json` 1.0.151 and `toml` 1.1.6. Xberg 1.3.10's mandatory minimums require 1.0.154 and 1.1.8 respectively; the source1b scratch audit verified exactly those versions.
- It also adds about 15 packages absent from root, including html-to-markdown-rs, astral-tl, html5ever 0.40.1 and markup5ever 0.40, comrak, pulldown-cmark, blake3 and bitvec.
- `first-html-slice.md` says only "report exact delta before compilation", and its checks cover only helper and intake lanes.

**Failure mode.** A workspace-wide `serde_json`/`toml` bump is shipped without workspace build, lint or test evidence. Or the grant halts immediately on a delta that was already predictable.

**Smallest correction.**
- Name the expected shared-version moves and require exact-checksum equality with the source1b ledger before any compile.
- Add workspace lanes for the lock move: `cargo build`/`clippy`/`test --workspace`, consistent with CI and `verify-storage.sh`.
- Any other movement stays a STOP.

## 4. Seams verified as acceptable

- **Genuine before-state.** `helper::extract` (`helper.rs:25–117`) handles only docx, pptx and eml; anything else returns "unsupported intake kind".
- **No nested-HTML expansion.** Nested EML dispatch covers only docx/pptx (`helper.rs:~1030–1045`), so a standalone html arm does not add nested HTML.
- **Generic validation.** `Extraction::validate` (`lib.rs`) is generic, so no DTO or schema change is needed. A new HTML converter id is safe because the host checks exact `CONVERTER` (`brn-threads-intake/src/lib.rs:279`) and still refuses Html (`:213`).
- **Defaults off works.** Xberg defaults are `tokio-runtime` + `simd-utf8`, so `default-features=false, features=["html"]` stays Tokio-free.
- **Options are reachable.** `ConversionOptions` is re-exported (`xberg lib.rs:347`), so no direct html-to-markdown dependency is needed.
- **Prepared HTML is untouched.** It keeps its RcDom, depth-128 and 200000-node walk (`helper/prepared/html.rs`); the slice keeps it byte-identical.
- **Sandbox.** The helper sandbox and rlimits (deny-default, CPU10/NOFILE64/FSIZE64MiB) are unchanged. Xberg's `dirs`/cache use is confined to feature-gated embeddings code, though the real no-file proof still has to run.

## 5. Advisories (not blockers)

- **A1 (executor).** `xberg::extract` is the only extraction entry point and it is `async`; the non-Tokio branch compiles without `tokio-runtime`. Specify a private std park/unpark `block_on` executor with no new runtime dependency, and count its thread.
- **A2 (forced options).** `convert_html_to_markdown_with_tables` forces `include_document_structure=true` and `extract_images=true` (`converter.rs:140–141`), and `needs_image_data` triggers a second pass. Image and data-URI refusal must happen before Xberg, using the conservative byte scan from Q1.
- **A3 (feature unification).** Xberg turns on `serde_json/preserve_order`, and html-to-markdown inline-images turns on gif, bmp and webp decoders. This only affects helper-feature builds (`-p brn-intake --features helper`), and BRN's `image_asset` acceptance does not widen. Record it in the feature-union ledger as added attack surface and size.
- **A4 (large inputs).** At 512KiB or more, `run_on_dedicated_stack` copies the HTML into a joined 16MiB thread (`stack_management.rs`). Count both copies in the memory measurements.
- **A5 (UTF-8).** Xberg decodes invalid UTF-8 lossily with a warning. BRN's strict UTF-8/NUL pre-check must stay first.
- **A6 (raw-HTML regex passes).** `recover_mathml_formulas` and `recover_table_captions` (`extractors/html.rs:335/427`) rescan raw HTML. A strict allowlist makes them no-ops; assert that.
- **A7 (wording).** The route is helper-protocol only, with no user path. Docs and status must not imply user-facing HTML capability or all-document adoption.
- **A8 (later fidelity).** For the rich-HTML gate, `map_document_structure` flattens table spans and appends images at the end, which falls short of prepared HTML's span fidelity.

## 6. Next-allocation readiness and limits

**Ready for a precise first-slice code grant once:**
- `first-html-slice.md` is amended for Q1 (agreement-language admission plus differential tests), Q2 (pinned config, depth and warning unreachability) and Q3 (expected lock delta plus workspace lanes);
- the actual root graph and checksum ledger are audited before any compile.

No repeat scope or method approval is needed. PDF, Excel/Tokio, cutover and rich HTML stay gated.

**Limits:**
- Static source review only. I made no runtime, RSS, thread, sandbox or compile claims.
- The root graph versions are inferred from manifest minimums, not resolved.
- This is not owner, main, release or native acceptance.
