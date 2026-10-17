Rendered review copy: three machine-local source links are relative here. [Exact verbatim report](opus-review-round2.txt) retains the original bytes.

**CHANGES_REQUIRED** for the corrected first HTML engineering component at `695f750af5103550239830a541b3673e107c624c`.

Review performed by **Codex GPT-6.1 Sol**, read-only. No edits, builds, tests, Cargo, network, provider calls, GUI operations or delegation occurred.

The pinned checkout is clean at evidence HEAD `ccbcf422eaa97c73694001410e238844dc0eace8`. Its crates tree matches the material pin: `0670fb3198fde3ece1bb392a3bc272d8ce3edc40`. The correction changes only the two allocated production modules, their tests and evidence documents. Lock/manifests, host, prepared HTML, core, AI, CLI and other formats remain unchanged.

**One blocking correctness finding: admitted hrefs can still be falsely refused after valid upstream serialization.**

At [documents.rs:356](../../../../../../../crates/brn-intake/src/helper/documents.rs), integrity searches for each decoded source href verbatim in Markdown with backslashes removed. That normalization does not cover two representations emitted by the pinned backend:

- `<p><a href="mailto:ops@example.test">ops@example.test</a></p>` is admitted. `ConversionOptions` inherits `autolinks=true` and `default_title=false`. Upstream `LinkData::is_autolink` selects the email autolink, and `emit_autolink` emits `<ops@example.test>`, omitting `mailto:`. The word check succeeds; the href check returns `INTEGRITY`.
- `<p><a href="https://x.test/?v=&amp;copy;">go</a></p>` is admitted. The scanner and upstream attribute decoder produce `https://x.test/?v=&copy;`. Upstream `append_url_destination` calls `escape_entity_ampersands`, emitting the destination as `https://x.test/?v=&amp;copy;`. Removing backslashes does not restore the searched string, so integrity again refuses valid conversion.

These are source-traced findings, **not newly executed failures**. Relevant pinned upstream anchors are:

- `html-to-markdown-rs-3.18.1/src/options/conversion.rs:301`
- `src/converter/handlers/link.rs:372`, `:388`, `:516`
- `src/converter/inline/link/link_url.rs:202`, `:323`
- Xberg `src/extractors/html.rs:615`, `:666` and `src/extraction/derive/mod.rs:675`, which preserve the converted Markdown.

The source grammar permits both witnesses at [html_admission.rs:355](../../../../../../../crates/brn-intake/src/helper/documents/html_admission.rs). Existing tests use labels such as `mail`/`go` and an ordinary `&c=2` query, missing these serialization interactions.

**Smallest acceptance criteria:** handle both representations within the existing adapter/configuration boundary and add real-Xberg regression cases for them. Preserve returned Markdown, href order/multiplicity, missing/reordered-target negatives and the aggregate linear bound. Pinning autolinks off is a bounded option for the email case. Entity-shaped destinations still need comparison consistent with their Markdown serialization. No grammar narrowing, quota increase, whole-output per-link scan or parser framework is needed.

**Prior blockers**

- **B1 — performance defect closed; href compatibility remains unresolved as above.** [advance/integrity](../../../../../../../crates/brn-intake/src/helper/documents.rs) uses two independent monotonic cursors. For normalized output length `M`, source-text bytes `S` and aggregate href bytes `H`, normalization, pattern setup and the current Rust substring-search strategy give aggregate `O(M + S + H)` integrity work. Searches stop at their first match or failure; successful matches advance past the complete pattern. The counter honestly measures consumed prefixes/failure suffixes, not CPU comparisons or complete offered suffixes. The retained before witness demonstrates the former repeated scanning; the benchmark is supplementary evidence.
- **B2 — closed for the allocated literal-backslash defect.** Normalization is symmetric and only affects comparison. Returned Markdown remains untouched. Tests check decoded non-code text and exact fenced/inline code bytes, including escaped quotes and doubled backslashes. Original truncation and reordered-word negatives remain. Removing all backslashes creates a deliberately lossy equivalence; it remains a truncation defense, not fidelity proof.
- **B3 — closed for the requested deterministic structural coverage.** The independently stated content-model table covers 219 admitted immediate pairs plus disallowed pairs. Known source trees supply preorder tags, parent indexes, raw text and attributes; astral-tl is compared directly with those facts. Scanner structure, authored decoded entity/href expectations and maintained-tokenizer corroboration provide separate checks. Every admitted corpus case invokes real Xberg and integrity. Repeated-li/raw-text divergence witnesses and pre-backend refusals are present. The new href finding needs two focused semantic cases, not a corpus redesign.
- **B4(a–f) — closed with the documented qualifications.** Earlier source hashes remain unknown; pre-escape failure and original SIGTERM evidence remain prose-only; deleted historical targets/binaries/dep-info remain hashes-only. Tagless routing is corrected and tested. Silent sandbox logging remains inconclusive. Native blake3 NEON C and the empty-name diff artifact are disclosed. New before/current artifacts and source bindings are retained separately.

**Advisories**

- **A1 closed:** the one-body guard is the only grammar change; duplicate bodies refuse before backend.
- **A2 retained limitation:** substring matching and symmetric normalization cannot establish complete semantic fidelity.
- **A3 closed:** the tautological check is removed; `lt_count` is test-only.
- **A4 closed:** documentation distinguishes pre-backend input/node/decoded-text limits from post-conversion Markdown/protocol limits. No hard RSS guarantee is claimed.
- **A5 closed:** the simple Pending → wake → Ready test exercises the executor without the long timeout.
- **A6 requested cases added:** literal block starts, code fences, NBSP and specified anchor forms are covered. The two href variants above remain a concrete gap.

One nonblocking evidence clarification would help: historical `raw_log_sha256` fields identify original scratch logs, while repository gzip copies shorten `/Users/evokessler/` to `~/`. All **37 original scratch logs** match their recorded hashes; every repository copy matches after that path expansion. Label this transformation explicitly rather than implying decompressed repository bytes have the original raw digest.

**Evidence assessment**

- All **62 correction log entries** and **34 retained external artifacts** match their recorded hashes and sizes.
- Final command source hashes match the material pin. The intermediate 22-test source is correctly distinguished.
- Retained final logs confirm **23 focused tests**, **91 helper tests passing**, strict helper Clippy and formatting success.
- Threads-intake remains **35 passed / 1 carried PDF `Unavailable` / 3 ignored**, command exit 101. Prepared HTML precedes the PDF failure; dedicated rich-HTML regressions also pass.
- The genuine before witness is **0 passed / 2 behavioral failures**. The interrupted build and E0434 fixture error are separately recorded and are not counted as behavioral reds.
- Large-request timing/RSS and friendly success remain measurements. Missing exact runtime PGIDs, validator failures, completed-operation waiting and the separate release inventory are honestly qualified.
- Historical broad workspace/native/retrieval results remain associated with `bbf107e3`. They are not correction-pin passes.

The existing restriction-before-input implementation and deterministic canaries remain adequate for this engineering gate. The silent logging control proves no absence of syscall attempts. I found no reason to reopen the dispositioned shared lock or require another acquisition/checksum campaign.

There is **no separate standards blocker**. The one blocking spec/correctness issue is the href serialization mismatch.

This verdict covers only standalone schema-1 `html-textual-1`. Rich prepared HTML remains unchanged; host `Converter::convert(Html)` remains unavailable. PDF resource resolution, Office/XLSX/MIME fidelity, helper-runtime unification, rich-HTML cutover, hosted/platform qualification, native/owner acceptance, all-eight-kind completion and final integration remain separate pending gates. No component publication, PR or merge is approved.
