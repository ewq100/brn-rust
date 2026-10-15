# X1 — Xberg PDF qualification, gates 1–2

**Decision: proceed to gate 3, limited to integration design inside the existing restricted helper.** Gates 1 and 2 pass for the two maintained synthetic PDF fixtures. Xberg provides local text, page rasters and embedded images without pdfium or model/native downloads. Its raw text order differs on one table; an explicit complete source-line permutation is needed to preserve the independently checked row associations. This report does not authorize or implement production integration.

| Gate | Status | Evidence |
|---|---|---|
| 1 — dependency graph/build | **passed** | Defaults disabled; locked offline release build with network denied; native-source/build-script audit; no ORT/OCR/pdfium/model graph; linked-library, size and license records. Build timings are **provisional** because a separate Z1 build overlapped. |
| 2 — PDF prepared evidence | **passed, bounded to these fixtures** | Five page-bound text outputs and rasters; both embedded images pixel-identical to Poppler; independent prose/table/figure/scan source inspection. Process page 2 needs a complete unchanged-line reordering receipt. |
| 3 — integration design | **pending** | Outside this assignment. No production helper, schema, provider, app or root dependency changes. |

## Authority, snapshot and isolation

- Owner override: **15 October 2026**, X1 gates 1–2 may run before the stored overlapping-branch release condition. No C0 or broader program start is claimed.
- Fetched `origin/main` and worktree starting SHA: **`7e9d0376d677ed2a46bdbcd8933f55f9e1746a02`** (PR147). Required current AGENTS/status/pack reads were completed at this SHA; master-plan sections 8/A2 and 11 and the final intake prompt governed the qualification.
- Branch: **`work/x1-xberg`**, created from `origin/main`. **Current worktree: `/Users/evokessler/repos/brn-rust/.worktrees/x1-xberg`**, in a fresh clone of `https://github.com/ewq100/brn-rust.git`, following the owner's location choice. The original qualification ran at `/Users/evokessler/Documents/Codex/brn-rust/.worktrees/x1-xberg` while the location question was pending. Its local commit **`f6092462065a6d614fe337f7a2639ca2e75b55a4`** was fetched into the new clone without pushing; all 132 experiment files transferred byte-for-byte. Historical absolute paths in captured evidence intentionally retain the original execution location. The old checkout is preserved. See the [relocation record](evidence/relocation.md).
- All task files are under **`experiments/x1-xberg/`** in that worktree. The experiment has its own Cargo.toml, Cargo.lock and empty `[workspace]`; the root manifest/lock and production sources are unchanged. The main checkout's untracked experiment material was not used. Neither named overlapping branch nor its worktree was modified.
- Build artifacts: **`CARGO_TARGET_DIR=/private/tmp/brn-x1-target`**, absent before the cold build. No shared BRN/Z1 target was used or cleaned.
- Host: macOS **27.0.1 (26A434)**, `aarch64-apple-darwin`. Existing pinned **Rust 1.98.1**, rustc `48a229ceaefd4985c50990b14116b6d856af0985`; LLVM 22.1.8. System Poppler **26.10.0** (`pdftotext`, `pdfimages`, `pdftoppm`, `/opt/homebrew/bin`). No Poppler installation was needed.
- Environment deviation: a preflight `rustup toolchain list` outside the repository unexpectedly auto-installed the missing default `stable` toolchain (1.99.0). The command was meant to list toolchains. All experiment compiler/Cargo invocations instead used the already-installed **1.98.1** paths; no repository pin was upgraded. No other installation was performed.
- Only tracked synthetic inputs and synthetic canaries were used. **Zero provider/model calls, model downloads, real user-data reads, UI/computer use, pushes, PRs or merges.** Active work stayed within the approximately three-hour timebox.

Exact commands and reproduction are in [COMMANDS.md](COMMANDS.md). The [plan](PLAN.md) received independent read-only approval; [review evidence](evidence/plan-review.md).

## Gate 1 — graph, build and native acquisition

### Exact selection

```toml
xberg = { version = "=1.3.10", optional = true, default-features = false, features = ["pdf"] }
```

The experiment's `candidate` feature enables this dependency and the serialization utility `serde_json =1.0.154`, already present transitively. Its `baseline` binary has no enabled dependencies. Xberg's effective feature closure is **`html`, `image-encode`, `pdf`, `pdf-native`**. The published `pdf` selects `pdf-native`, which selects **`xberg-native-pdf 1.3.10`, `lopdf 0.45.0`, image/flate2, HTML and image encoding**. Defaults (`tokio-runtime`, `simd-utf8`) are **unnecessary**: the public async extraction API runs on the no-Tokio path with a standard-library waker, and rendering uses `PdfRenderSession` directly.

No `ocr`, `paddle-ocr`, `pdf-pdfium`, `embeddings`, `full`, other inference feature or Kreuzberg dependency was enabled. **`ort` and `ort-sys` are absent.** The package lock checksums pin acquisition; manifests/metadata and [effective features](evidence/features.txt) record exact resolved versions. Metadata contains **337 all-target nodes / 303 target-filtered macOS nodes**, including the local harness. PDF-only still carries a substantial HTML/image-codec graph.

| Requested comparison | X1 | BRN root Cargo.lock at snapshot | Result |
|---|---|---|---|
| quick-xml | **0.42.0** | **0.41.0** | An additional version would be needed in a combined product graph. |
| zip | **absent** | **8.6.0** | Expected ZIP 9 duplication does **not** occur with this feature set. |
| ort / ort-sys | **absent** | ORT **2.0.0-rc.13** | No ORT links conflict demonstrated by this isolated selection; no combined-product build was attempted. |
| image | **0.25.10** | **0.25.10** | Same recorded version. |
| xberg / xberg-native-pdf | **1.3.10 / 1.3.10** | absent | Isolated candidate only. |

`cargo tree -d` is retained [in full](evidence/duplicates.txt). Distinct-version duplicate families are: `alloc-no-stdlib` 2.0.4/3.0.0; `alloc-stdlib` 0.2.4/0.3.0; `brotli-decompressor` 5.0.3/6.0.1; `fax` 0.2.7/0.3.0; `getrandom` 0.2.17/0.3.4/0.4.3; `miniz_oxide` 0.8.9/0.9.1; `pastey` 0.1.1/0.2.3; `syn` 2.0.119/3.0.9; `weezl` 0.1.12/0.2.1. Cargo also repeats same-version host/target build units; those are not distinct-version conflicts. The complete [root-lock comparison](evidence/root-lock-comparison.tsv) covers every target-filtered package. No root resolution or pin change was made.

### No automatic native/model acquisition

Normal `cargo fetch` succeeded; every subsequent build used `--locked --offline`. The whole cold build additionally ran inside **macOS `(deny network*)`**, covering arbitrary build-script networking as well as Cargo itself. Offline acquisition/build had no cache miss or network blocker.

The [native audit](evidence/native-audit.md), [41-script inventory and hashes](evidence/build-script-inventory.json), inspected [script sources](evidence/build-scripts.txt), and actual [native link directives](evidence/native-link-build-output.txt) establish:

- Xberg's build script emits cfg/build-ID metadata and invokes only local Git inspection commands.
- `xberg-native-pdf` has **`build = false`** and renders with Rust/tiny-skia. `blake3` compiles bundled NEON C using the installed compiler; no native source/library is fetched. The AV1 scene-change/encoder assembly paths are disabled (`asm` is not enabled).
- Enabled remaining scripts generate local tables/metadata or probe the existing compiler. No enabled native/model download path was found. No ORT/Tesseract/Leptonica/pdfium/model manager or HTTP-download client is in the selected graph.
- Both fixture extraction/render runs also succeeded under the deny-default [runtime profile](runtime.sb), with explicit input/output access and system fonts/libraries only. A [same-profile witness](evidence/sandbox/result.json) verifies denied excluded-file reads and denied connection to a listening loopback socket, while owned scratch writes succeed. Cache and OCR are explicitly disabled. There is no runtime native/model fetch.

This conclusion covers the locked local PDF route. The standalone harness does not qualify the production helper's complete isolation, hard memory enforcement or cancellation lifecycle.

### Footprint

Sequential **cold release** builds, four jobs; target initially absent. These are one-run wall times and **provisional**: [process samples](evidence/processes-xberg-probe.txt) include compilation in `/private/tmp/brn-z1-target`, summarized in [contention.json](evidence/contention.json).

| Measurement | Minimal binary | Xberg extraction + rendering probe | Delta |
|---|---:|---:|---:|
| Cold build seconds, provisional | 0.234 | 126.336 | +126.101 |
| Cargo release bytes | 469,208 | 37,449,984 | **+36,980,776** (~35.27 MiB) |
| After identical system `strip -S`, bytes | 434,184 | 37,391,944 | **+36,957,760** (~35.25 MiB) |

[Build commands/results](evidence/builds.json), [candidate log](evidence/build-xberg-probe.log) and [size/hash record](evidence/binary-sizes.json) are retained. This measures the usable extraction/render harness, including JSON/filesystem glue, rather than a product executable's incremental linked cost. No root/product binary was built.

Rust's bundled `rust-objcopy` emitted a **debug-strip warning** because its `libLLVM.dylib` could not be located; the build exited successfully. The separately recorded system-strip comparison avoids silently treating the warning as a clean toolchain environment.

`otool -L`: [minimal](evidence/otool-baseline.txt) links `libSystem.B.dylib`; [candidate](evidence/otool-xberg-probe.txt) links **CoreFoundation, libiconv.2.dylib and libSystem.B.dylib**. No external PDF/OCR/ONNX library is dynamically linked. The bundled BLAKE3 C archive is statically linked. Existing compiler tools and system fonts remain prerequisites.

### Licenses

Cargo metadata license expressions are recorded for the [whole graph](evidence/licenses.tsv) and [macOS graph](evidence/licenses-macos.tsv); [summary counts](evidence/graph-summary.json) preserve every expression. Xberg and its native PDF engine declare **MIT**, lopdf **MIT**. The graph includes MIT/Apache alternatives, BSD-2/3, Unicode-3.0/Unicode-DFS-2016, Zlib, Unlicense, CC0 and optional alternative BSL/LLVM-exception expressions. **`option-ext 0.2.0` declares MPL-2.0**. The sole missing macOS license field belongs to the local unpublished experiment, not a third-party dependency. All-target metadata also records `libfuzzer-sys`'s NCSA expression; it is absent from the target-filtered selection.

Cargo metadata does not inventory vendored/bundled content. The native engine's [attributions](evidence/licenses/xberg-native-pdf-attributions.md) record its PDFOxide lineage, vendored MIT fontdb and font licenses. The [DejaVu license](evidence/licenses/DejaVu.txt) is retained; CJK/emoji feature-gated fallback assets are not enabled. Packaging/notice review belongs to any later distribution decision; no general legal clearance is claimed.

## Gate 2 — source evidence versus Poppler

### Baseline contract and fixture authority

Source trace at the snapshot:

| Consumer | What it actually consumes |
|---|---|
| `pdf_tools.rs` | Discovers explicit native `pdftotext`/`pdfimages`; authoritative overrides refuse invalid tools. `prepared.rs` separately resolves native `pdftoppm`. |
| `prepared.rs::prepare_pdf` | `pdftotext -bbox-layout -enc UTF-8`, preserving page geometry and flow/block source lines. Only isolated single-line fragments at identical vertical bounds and disjoint horizontal bounds are regrouped. One PageText block per page, including empty scans. |
| Same schema-2 preparation | `pdftoppm -f 1 -l N -scale-to 1600 -png` supplies transient previews, source page dimensions, preview hashes and `needs_transcription = text.trim().is_empty()`. |
| `lib.rs` legacy PDF route | `pdftotext -layout`, `pdfimages -list` and `pdfimages -png` supply source text and embedded figures. **Schema-2 PDF preparation does not call pdfimages**; it retains preview-based figure crops after candidate checking. Embedded-image extraction was nevertheless compared as explicitly requested. |
| `brn-intake` prepared checker | Exact source/hash/preview bindings; complete, unique, in-range zero-based nonempty-line permutations may reflow a page. Empty scan text cannot claim line-order evidence. These rules preserve words/values; independently checked previews/inventories establish table/figure associations. |

The repository contains exactly two tracked `.pdf` fixtures. The older external research `layout.pdf` is referenced in historical QA but is not a maintained tracked fixture in this checkout and was not used.

| Fixture | SHA-256 | Independent inventory |
|---|---|---|
| `tests/fixtures/harbor-study.pdf` | `f5739675823b6900a212bdd10e27423f7fc29038c03fb7d103fa78430abd3fe0` | Existing `harbor-study.inventory.json`: 19 substantive items and one 720×200 figure. |
| `tests/fixtures/luna/process-scan.pdf` | `ebc90965a73b6dec328cb7355176c22edb842afa1b74684f2c711ef318e28244` | Existing `docs/work/active/rust-luna-intake/source-qa.md`: 11 headings, eight list items, 12 table cells, nine scan units and a complete three-box/two-arrow diagram. [source-inventory.json](source-inventory.json) adds exact row/prose/scan transcription from the source-page visual inspection. |

The [runner](compare.py) preserves raw Poppler bbox XML and layout output, a small projection of the existing prepared-source-line rule, Xberg's exact configuration/result/page text, line diffs, images and previews. The projection is not a production-helper invocation. [Inventory checks](evidence/inventory-checks.json) compare full ordered substantive sequences, complete row tuples and image pixels; they do not infer success from isolated word presence.

### Per-page findings

| Fixture/page | Poppler / Xberg nonempty source lines | Text and source associations | Raster/visual inspection |
|---|---:|---|---|
| Harbor 1 | 19 / 13 | Same complete substantive sequence. Xberg groups table cells into three full lines. Both rows remain `Check first / 12 / 1`, `Test first / 19 / 3`, in their header order. All 13 page inventory items pass. | Title, findings, three labelled boxes/two arrows, caption and limits legible and correctly placed. |
| Harbor 2 | 6 / 6 | Identical stripped line sequence; both references, appendix definition and observation method preserved. All six page inventory items pass. | All prose and references retained. |
| Process 1 | 16 / 14 | Same complete substantive sequence; three diagram labels form one Xberg line. Marker `0042`, ruler `R-0006`, nested step label and separate-permit qualification retained before footer. | All headings/steps, three box borders/labels, both complete arrows and caption retained. Native bold headings use a visibly different serif fallback. |
| Process 2 | 22 / 18 | **Raw order differs:** Xberg emits Marker/Depth header and rows first, then Status and its values. A complete unchanged-line permutation restores all 12 cells: `M-0042 / 0 / Dry at intake`; `M-0108 / 12.5 / Outlet flowing`; `M-0110 / -- / Not observed`. No value or identifier is rewritten. | Row alignment, literal `--` versus `0`, table note, disposition, reference and footer are visibly correct. |
| Process 3 | 0 / 0 | Image-only page stays empty in both native text outputs; transcription remains required. No fabricated/OCR text. | All nine scan units remain readable, including `06:40`, CARD 09, M-0042, R-0006, nesting and the safety qualification. |

Process page 2's tested Xberg source-line permutation is **`[0,1,2,3,4,8,5,9,6,10,7,11,12,13,14,15,16,17]`**. It uses every nonempty line once. The reflowed full substantive sequence equals the independent inventory; the raw-order check intentionally remains false in the evidence. This is a deterministic, hand-checked compatibility witness for the already-existing receipt contract, not a new sorting algorithm or proof a model will always choose the right receipt. The actual production checker/Luna route was not run.

All five native rasters rendered at **150 DPI (1275×1650)** through **`PdfRenderSession` with `pdf` only**. The experiment downsamples them to a 1600-pixel long edge (1236×1600); Poppler emits 1237×1600 due to different rounding. Source geometry is retained as 612×792 PDF points. All five page pairs were visually inspected. Mean absolute RGB differences are 0.71–2.30/255; these are diagnostics, not correctness thresholds. Font substitution and antialiasing differ, while the inventoried substantive information remains visible. No render/processing warnings were returned.

**Rasterization does not require pdfium in this pinned release.** The separate-decision stop for `pdf-pdfium` was therefore not triggered, and that feature was never enabled. Manual preview suitability is established on these inputs; no Luna visual-response quality is claimed.

### Embedded images

| Fixture | Poppler / Xberg | Source association and pixel evidence |
|---|---|---|
| Harbor | 1 / 1, 720×200 | Page 1, three-gate CHECK → TEST → RELEASE figure. Both decoded RGB SHA-256: `efd7dd3591a80a54b90074718d964dbd672850ea37bc8ab439c5450219ee30ab`. Xberg exposes its page number/bounds. |
| Process | 1 / 1, 1650×2100 | Page 3, complete scanned card. Both RGB SHA-256: `1a07326d9feba32a035cdbcf4117f6115f1b2947f15ef45edf1c83cb3409547e`. Page-1 diagram is vector content and correctly comes through rasterization, not as an embedded raster. |

Encoded PNG bytes/sizes differ because compression differs; decoded dimensions and every RGB pixel agree. Full names, encoded hashes, page/bounds metadata and comparisons are in [comparison.json](evidence/comparison.json) and the fixture `xberg/result.json` files. Page preview hashes/line diffs and five visual pairs are under [Harbor evidence](evidence/harbor-study/) and [process evidence](evidence/process-scan/).

## Residual risks and disposition

| Risk or unqualified area | Consequence / next requirement |
|---|---|
| Reading-order/table-column difference | Preserve the complete source-line receipt contract and page previews; raw content cannot silently claim row-order equivalence. Any gate-3 design must account for this observed case. |
| Metadata ambiguity on the scan | Xberg document metadata marks page 3 blank in one summary, while the per-page result reports nonblank and `scanned_pages=[3]`. Existing BRN empty-source-text transcription semantics remain the reliable fixture witness. Do not derive completeness from a generic blank flag. |
| Font discovery/fallback | Runtime source walks font directories. This experiment denied user font contents and allowed system fonts. Native heading font substitution is visible; other fonts/scripts/complex layouts are unqualified. |
| Containment and adversarial lifecycle | The network/file canary and local fixture runs are passing evidence. Production helper compatibility, hard RSS/page/raster limits, malformed/encrypted/truncated PDF behavior, worker death, cancellation/reaping and cleanup still need their own qualification. No broad sandbox certification is claimed. |
| Small fixture corpus | Two synthetic PDFs/five pages do not establish universal PDF/scan/math fidelity. Independent inventory remains necessary; native empty scan text is not complete conversion. |
| Model and host behavior | No Luna calls, full-note model conversion, checked save, replay, Undo, scope/privacy persistence or native application interaction were exercised. Existing behavior must remain explicit in a later approved slice. |
| Cost and licenses | ~35.25 MiB stripped harness delta, a broad transitive graph, MPL-2.0 package and bundled-font notices merit later packaging review. Provisional timings require an uncontended benchmark if used for a performance decision. |
| C0/new baseline | C0 does not yet supply an integration ref for this experiment. Reconcile root graph, source contracts and fixtures against that ref before relying on these results for integration. |

**Recommendation:** proceed to **gate 3 design only**, preserving the current restricted helper and schema-2 source/preview/receipt contracts. These deterministic results justify investigating removal of the external Poppler prerequisite; they do not justify cutover, expanded parser scope, an optional PDF/OCR/model feature or any pin upgrade. The writing milestone and overlapping work remain independent.

Formatting, strict experiment Clippy and deterministic source-inventory checks passed; the [final verification](evidence/verification.md) and [independent evidence review](evidence/final-review.md) record the results and resolved findings. Local commit only; pushing requires the owner's separate instruction.
