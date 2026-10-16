# Session A — modular intake and extractor qualification

You own intake for the BRN modular-foundation program. Read the supplied master plan and the current canonical `docs/work/active/modular-foundation/plan.md`, contracts, baseline, ownership and coordination records. Session C owns shared interfaces/integration; Session B owns writing/editor work.

## Objective

Run the first intake experiment (X1): a time-boxed qualification of `xberg =1.3.10` against BRN's demonstrated intake weakness. The PDF path depends on external Poppler executables, and complex PDF fidelity is unqualified. X1 is not the whole long-term intake objective; other intake areas stay deferred with triggers in master plan section 17. Use three staged gates (master plan section 8, A2) and deliver a decision report. The extraction boundary already exists (#144: restricted helper, schema-2 prepared readers, store-free Luna stage, checked host save). Do not re-modularize it, rewrite BRN or build new parsers. Production cutover is a separate later owner decision.

## Preflight and scope

Read `AGENTS.md`, status, architecture invariants, workflow/verification, current intake and Rust/Luna conversion plans, source and tests. Inspect the actual branch/HEAD, remote and local ownership. Start in an isolated worktree from C's integration ref. Before C0 exists, start from an explicitly recorded existing snapshot (for example fetched `origin/main` and its SHA), with no production or shared-file changes. After C0, record C's ref and state whether your results still hold or need rerunning. The planning snapshot is not permission to discard newer work. Do not use the untracked `experiments/` folder in the owner's main checkout.

Confirm exact owned paths with C. During the experiment, own only `experiments/xberg-*`, with its own manifest and lockfile per repository rules, plus its evidence file. Do not edit `brn-intake`, `brn-threads-intake`, `brn-ai/src/conversion.rs` or `brn-threads-app/src/intake.rs`; the last is being changed by `work/daily-workflow-reliability` until it meets the master plan section 11 release condition. Shared schemas, types, root manifests, the lockfile and app-runtime dispatch belong to C. Production integration after gate 3 needs C's ownership allocation and owner approval.

Use separate synthetic data/scratch and task outputs. Work locally, test, review, document and commit within scope. Normal acquisition of pinned crates into the experiment's own lockfile is permitted within current authority. Automatic build-time or runtime native or model downloads are not. No publication/merge/release, real user data, model spending or model downloads without current explicit authority. Use current compiler/lockfile and selected model. Do not silently substitute provider/account/model. No UI use on a locked Mac.

## Work in reviewable slices

### A1 — current route and comparison harness

Trace every supported user-facing import path, including prepared/Luna conversion. Inventory formats, fidelity expectations, hard bounds, source/asset mapping, dedicated model behavior, process isolation, cancellation, persistence, replay, Undo and cleanup. Do not mistake a legacy `convert` switch for the entire supported product.

Reuse/evolve current `ConversionResult`, assets, coverage and source contracts. The extractor returns normalized knowledge and diagnostics; the host decides where/when to save it. No canonical store access or project policy in the extractor. Preserve Markdown/text verbatim behavior.

Build candidate-versus-baseline fixtures with independently established source inventories. Preserve current prepared-reader/value/table/figure bindings and failure tests. Keep the comparison harness outside the permanent production selection path.

### A2 — exact candidate qualification

Pin `xberg =1.3.10`. Do not use Kreuzberg 4.10.4: its `ort` rc.12 conflicts with fastembed's rc.13 through `ort-sys links`, and older v4 releases were ELv2.

- **Gate 1:** `pdf` feature only, acquired as pinned crates. Record duplicates (`quick-xml` 0.42/0.41, `zip` 9/8.6), build time, binary delta and native requirements. Show that this feature set triggers no automatic build-script or runtime native/model download.
- **Gate 2:** produce the schema-2 PDF prepared evidence (line-ordered text, page rasterization for Luna previews, embedded images) on existing fixtures and inventories versus Poppler. If rasterization needs `pdf-pdfium`, record it as a separate decision.
- **Gate 3:** design integration inside the existing restricted helper only after gates 1 and 2 pass.

Stop at the first failed qualification gate and report the result as passed, failed, blocked or pending. An offline cache miss or unavailable network is an environment blocker to record and retry, not a negative qualification. Do not upgrade pins to make a gate pass.

Use a minimal-feature native Rust integration in an isolated bounded worker. Inspect transitive/native requirements, PDF backends, OCR/model assets, package size, build cost and ONNX/embedding-runtime interactions. Preserve existing GUI/compiler/Rig/storage pins unless a separately justified shared change is accepted by C.

Do not enable optional web crawling, embeddings, entity extraction, transcription, remote LLM calls or extra formats by default. Native extraction precedes any selective explicitly enabled OCR; do not OCR files with usable extracted text. Do not broaden scope into a universal renderer or math-layout system.

Prove available isolation rather than calling any subprocess a sandbox: permitted input/scratch only, no canonical data or credentials, local extraction without network, validated output, time/size/page/memory bounds, child cancellation and cleanup/reaping after crashes. Record residual gaps.

### A3 — preserve or explicitly supersede dedicated conversion behavior

The baseline includes owner-directed prepared/Luna behavior. Compare actual full-note outcomes and source fidelity, not only token/latency savings. Preserve deterministic checks. Decide whether Xberg supplies prepared evidence to that route, replaces a demonstrably unnecessary step, or fails mandatory support.

Do not silently delete the dedicated conversion model, repurpose chat settings, fabricate full coverage, change exact source values or use another provider after failure. A necessary semantic/visual stage stays explicit and tool-free with the existing authority/budget rules.

### A4 — end-to-end evidence

*During the experiment, run only the extractor-level cases (malformed, truncated, password-protected, limits, Unicode, tables and figures against inventories). The host-service cases below belong to a later approved integration slice.*

Preserve every supported user-facing baseline format. Test DOCX/PDF/email/Markdown/text and any active supported HTML/spreadsheet/presentation cases with source inventories. Include nested lists, footnotes, duplicate figures, tables and associations, leading zeros, numeric values, supported formulas/caches/comments, Unicode and exact locators.

Test malformed/truncated/password-protected sources, unknown coverage, missing native tools, oversized outputs, worker death, timeouts, cancellation after completion but before save, restart and temp cleanup. A returned result is not proof of complete extraction.

Through real checked host services verify import intent, project/private scope, retained assets, idempotent replay, Undo, backup/restore and offline reading when the original is unavailable. Original binaries remain external; do not retain hidden raw copies in logs/checkpoints. Do not delete external owner files.

Use scripted/mocked model tests in CI. Live provider tests use only currently authorized models/accounts, synthetic/public sources and explicit authority. Report conversion-source QA separately from model success.

### A5 — decide (integration and retirement are a later assignment)

Record adopt/keep-current/bounded-adaptation with mandatory pass/fail evidence, exact pins/features, fidelity, runtime/memory/build footprint and maintenance/glue cost. If the owner later approves adoption, that separate slice follows the retirement rules below and C integrates it.

For adoption, switch one production route and remove replaced parsers/orchestration/helpers/defaults/dependencies coherently after caller/data-reader checks. Keep rollback evidence and needed historical result readers, not a hidden alternate converter. Preserve current schema/data compatibility or submit a separately qualified C-owned change.

If the candidate fails, deliver the sound boundary/tests with the current backend and a precise reason. Do not force adoption or expand into an unbounded fork.

## Handoff to C

Provide the starting snapshot and its reconciliation to C's ref, commit/ref, X1 result status for master plan section 17, exact owned/shared paths, supported behavior and gaps, source/version/license/native dependency inventory, benchmark fixtures/results, test commands/features and failures, independent review disposition, rollback/data compatibility, native/provider checks still needed and proposed integration order. Keep current docs and evidence concise and authoritative. Do not claim merged, native-observed or complete when those gates have not occurred.
