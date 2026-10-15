# Session A — modular intake and extractor qualification

You own intake for the BRN modular-foundation program. Read the supplied master plan and the current canonical `docs/work/active/modular-foundation/plan.md`, contracts, baseline, ownership and coordination records. Session C owns shared interfaces/integration; Session B owns writing/editor work.

## Objective

Strengthen the existing intake boundary, qualify a maintained Rust Xberg/Kreuzberg extractor against BRN's actual current route, and adopt it only if mandatory gates pass. Deliver maintainable, independently testable intake consumed through the existing checked import service. Do not rewrite BRN or build new parsers.

## Preflight and scope

Read `AGENTS.md`, status, architecture invariants, workflow/verification, current intake and Rust/Luna conversion plans, source and tests. Inspect the actual branch/HEAD, remote and local ownership. Start from C's explicit common ref in your isolated worktree. The planning snapshot is not permission to discard newer work.

Confirm exact owned paths with C. Default focus: `brn-intake`, `brn-threads-intake`, their fixtures and agreed dedicated AI-conversion files. Shared schemas/types/root manifests/lockfile/app-runtime dispatch belong to C. Record a concise shared-file change request when needed. Local compile-only patches must be disclosed for C's integration.

Use separate synthetic data/scratch and task outputs. Work locally, test, review, document and commit within scope. No publication/merge/release, real user data, model spending or model downloads without current explicit authority. Use current compiler/lockfile and selected model. Do not silently substitute provider/account/model. No UI use on a locked Mac.

## Work in reviewable slices

### A1 — current route and comparison harness

Trace every supported user-facing import path, including prepared/Luna conversion. Inventory formats, fidelity expectations, hard bounds, source/asset mapping, dedicated model behavior, process isolation, cancellation, persistence, replay, Undo and cleanup. Do not mistake a legacy `convert` switch for the entire supported product.

Reuse/evolve current `ConversionResult`, assets, coverage and source contracts. The extractor returns normalized knowledge and diagnostics; the host decides where/when to save it. No canonical store access or project policy in the extractor. Preserve Markdown/text verbatim behavior.

Build candidate-versus-baseline fixtures with independently established source inventories. Preserve current prepared-reader/value/table/figure bindings and failure tests. Keep the comparison harness outside the permanent production selection path.

### A2 — exact candidate qualification

Check current official Xberg/Kreuzberg lineage, release, Rust package, features, license/provenance and supported integration interface. Legacy docs use a different version narrative from current Xberg's rebranded line; pin actual qualified artifacts rather than guessing a version.

Use a minimal-feature native Rust integration in an isolated bounded worker. Inspect transitive/native requirements, PDF backends, OCR/model assets, package size, build cost and ONNX/embedding-runtime interactions. Preserve existing GUI/compiler/Rig/storage pins unless a separately justified shared change is accepted by C.

Do not enable optional web crawling, embeddings, entity extraction, transcription, remote LLM calls or extra formats by default. Native extraction precedes any selective explicitly enabled OCR; do not OCR files with usable extracted text. Do not broaden scope into a universal renderer or math-layout system.

Prove available isolation rather than calling any subprocess a sandbox: permitted input/scratch only, no canonical data or credentials, local extraction without network, validated output, time/size/page/memory bounds, child cancellation and cleanup/reaping after crashes. Record residual gaps.

### A3 — preserve or explicitly supersede dedicated conversion behavior

The baseline includes owner-directed prepared/Luna behavior. Compare actual full-note outcomes and source fidelity, not only token/latency savings. Preserve deterministic checks. Decide whether Xberg supplies prepared evidence to that route, replaces a demonstrably unnecessary step, or fails mandatory support.

Do not silently delete the dedicated conversion model, repurpose chat settings, fabricate full coverage, change exact source values or use another provider after failure. A necessary semantic/visual stage stays explicit and tool-free with the existing authority/budget rules.

### A4 — end-to-end evidence

Preserve every supported user-facing baseline format. Test DOCX/PDF/email/Markdown/text and any active supported HTML/spreadsheet/presentation cases with source inventories. Include nested lists, footnotes, duplicate figures, tables and associations, leading zeros, numeric values, supported formulas/caches/comments, Unicode and exact locators.

Test malformed/truncated/password-protected sources, unknown coverage, missing native tools, oversized outputs, worker death, timeouts, cancellation after completion but before save, restart and temp cleanup. A returned result is not proof of complete extraction.

Through real checked host services verify import intent, project/private scope, retained assets, idempotent replay, Undo, backup/restore and offline reading when the original is unavailable. Original binaries remain external; do not retain hidden raw copies in logs/checkpoints. Do not delete external owner files.

Use scripted/mocked model tests in CI. Live provider tests use only currently authorized models/accounts, synthetic/public sources and explicit authority. Report conversion-source QA separately from model success.

### A5 — decide, integrate, retire

Record adopt/keep-current/bounded-adaptation with mandatory pass/fail evidence, exact pins/features, fidelity, runtime/memory/build footprint and maintenance/glue cost. C accepts shared integration patches.

For adoption, switch one production route and remove replaced parsers/orchestration/helpers/defaults/dependencies coherently after caller/data-reader checks. Keep rollback evidence and needed historical result readers, not a hidden alternate converter. Preserve current schema/data compatibility or submit a separately qualified C-owned change.

If the candidate fails, deliver the sound boundary/tests with the current backend and a precise reason. Do not force adoption or expand into an unbounded fork.

## Handoff to C

Provide commit/ref, contract revision, exact owned/shared paths, supported behavior and gaps, source/version/license/native dependency inventory, benchmark fixtures/results, test commands/features and failures, independent review disposition, rollback/data compatibility, native/provider checks still needed and proposed integration order. Keep current docs and evidence concise and authoritative. Do not claim merged, native-observed or complete when those gates have not occurred.
