# HTML review-2 correction handoff

18 October 2026. Material pin **`fa0bb1221224db3f8e6710a6647b6d4c2f3592d6`**, branch `work/x2-html-20261017`, literal checkout `/Users/evokessler/.t3/worktrees/brn-rust/t3-ad771af6`. Baseline material `695f750af5103550239830a541b3673e107c624c`; clean entry evidence HEAD `ccbcf422eaa97c73694001410e238844dc0eace8`. This handoff's evidence-only commit leaves the material crates tree unchanged.

The single href-serialization finding is implemented and locally verified. [Full review](opus-review-round2.md) is verbatim (SHA-256 `f1d1639f3834ce1d42ad86015974c4ec6ae0a13f47be3d7b9e0e0728d62dd620`); [response](response.md) records both genuine before failures and the bounded fix. [Check index](check-index.json) holds exact commands, source/artifact associations, raw/stored log hashes and limitations.

`documents.rs` is the only changed source file: pin autolinks off through the exposed option, and remove one serializer-added `&amp;` layer for href comparison. Word checks, returned backend Markdown, monotonic order/multiplicity and `O(M + S + H)` work remain. Literal named/numeric URL contents remain distinct. No grammar narrowing, entity table/parser framework, dependency/lock/host/prepared-HTML/runtime/other-format/CI change.

| Check | Actual result |
| --- | --- |
| Genuine fresh 695 before helper build | pass |
| Identical new before witnesses | 0 passed / 2 behavioral failures, both after real Xberg output |
| Corrected helper build first | pass |
| New focused witnesses/supplements | 4 passed |
| HTML/admission tests | 27 passed |
| Helper all targets | 95 passed / 0 failed / 0 ignored (54/0/17/12/6/6) |
| Helper strict all-target Clippy | pass, `-D warnings` |
| Formatting | pass |

The unchanged rich prepared-HTML regressions and restriction/cancellation canaries pass in all-targets. Existing literal/backslash assertions, negatives and 219-pair corpus are unchanged. The original 37-log path transformation is now explicitly qualified in the original report/index; every old raw hash/gzip byte remains intact, and all 37 expanded copies match their original raw digests. New opus2 gzip logs reproduce raw bytes exactly without path rewriting.

New before source `/private/tmp/brn-x2-html-r2/before-src` uses only `/private/tmp/brn-x2-html-r2/target-before`; candidate uses its existing `/private/tmp/brn-x2-html-r1/target`. Fresh `/private/tmp/brn-x2-html-r2/tmp-before` and `tmp-candidate`, Rust 1.98.1, locked/offline/jobs1. No source edit during Cargo. Existing target executables/dep-info were copied before rebuilding. Before and final helper/test executables, `.d`, full source snapshots and both complete `695..pin` / `781..pin` binary diffs remain outside targets in `/private/tmp/brn-x2-html-r2/artifacts`. Nothing was cleaned up. No historical deleted artifact was reconstructed. Full `695..pin` includes intervening opus1 evidence commits; the separately retained source-only diff isolates this one module correction.

The prior 91-test helper and 35-pass/1-PDF-failure/3-ignored host results remain qualified to 695; no full host/PDF/workspace/native/retrieval repeat or new-pin pass is claimed. No unexpected code/test/lint failures or retries occurred; the two expected before failures are retained as failures. The documentation link checker exits 1 with exactly three machine-local `file:line` links in the verbatim review. The owner explicitly requires this report copied verbatim; its bytes are preserved rather than editing its links or the checker. This is a recorded documentation qualification, not a code-check pass. No other link failure was reported.

[Release receipt](release.json) records the precise empty heavy inventory and completed sessions. Sole heavy and **all allocated paths are RELEASED on this terminal handoff**, after the final evidence-only commit. No worker process/writer remains running and no review/new-family work will be started here.

Parent owns the focused final precision review and component acceptance; neither is claimed. All-eight-kind adoption, rich HTML cutover, PDF resource resolution, Office/XLSX/MIME/runtime/platform/native/owner gates and integration remain pending. No Kiro call/probe, delegation, download, Auth/live/provider/GUI/mutant/PR/push/merge occurred.
