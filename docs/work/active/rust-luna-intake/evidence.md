# Rust Luna intake evidence

Owner assignment: port the conversion kit to Rust, enable dedicated Luna conversion in BRN imports, publish a PR and merge after required checks. The isolated `codex/rust-luna-intake` checkout starts from `1d1555a364ec0f7d8a7e260a0420e9fa587784fe` and reconciles main through `4b41f190` (provider errors/resend). Rust 1.98.1 and the repository lockfile remain authoritative. Only synthetic/public fixtures were used. The owner checkout and separate Python research checkout are preserved.

## Implemented contract

Schema-1 helper behavior remains beside schema-2 prepared evidence. Maintained BetterOffice 0.3.0, mail-parser 0.11.8, inert html5ever/rcdom and pinned Calamine 0.36.1 prepare source structure in the restricted helper; format parser dependencies do not enter SQLite core. Poppler supplies bounded PDF text and previews. Local preparation/check phases use 5 seconds for non-PDF or 20 seconds for PDF, below the existing 30-second hard limit. The native renderer receives a host-written system-font configuration and disables disposable disk font caches; its font set remains in memory. No shell wrapper, external font-directory discovery or broader sandbox access is added.

Rust owns structured-file Markdown, exact values and source/cell bindings. Luna returns a typed plan for table inspection, PDF page Markdown and source-coordinate figure crops. Structured values cannot be rewritten by the model. The checker validates normalized source wording, metadata roles, cell coordinates, heading/list/footnote/table structure, image destinations/occurrences and preview receipts. Validated internal binding comments are removed before persistence. Only linked figures/crops reach storage; originals and full previews remain transient.

PDF preparation preserves Poppler flows. It joins only isolated single-line fragments with complete equal vertical geometry and disjoint horizontal bounds; ambiguous or multiline flows keep extraction order. Luna can request an explicit complete permutation of the indexed nonempty text-layer lines. The candidate retains that order and the checker validates it again against the unchanged source: omissions, duplicates, out-of-range indices, changed values and ordering declarations on empty scans fail. Content conservation does not prove the visual reading order or table associations. Located layout/scan gaps and Partial coverage remain without an independent inventory.

The dedicated production request is fixed `gpt-6-luna` high using explicit existing ChatGPT credentials, with no tools/history, general-settings mutation, provider fallback or automatic resend. Markdown/text remain provider-free and verbatim. The profile bounds prompt text to 256 KiB, candidate text to 1 MiB, images to 32/16 MiB plus decoded-pixel quotas, and one provider request to 180 seconds. Rig's subscription route does not support `max_output_tokens`; no generation-token cap is claimed. Refusal, malformed/truncated output, unavailable credentials/model, timeout and Stop fail before checked persistence. Stop after queued completion also prevents saving. The existing checked service owns protection, source dedup, replay, context boundaries and Undo.

## Source qualification and live measurements

The [independent source comparison](source-qa.md) records source hashes, retained figures, accepted output hashes, known gaps and offline provenance. Five fresh synthetic fixtures cover nested labels, footnotes, email metadata, blank-cell comments, formula/cache roles, leading zeros, headings, scans and figure crops. Earlier Python fixtures and fresh adversarial unit cases supplement these checks; there was no repeated full Sol conversion benchmark.

| Original accepted live typed-plan run | End-to-end seconds |
| --- | ---: |
| nested HTML | 4.036 |
| footnotes DOCX | 4.782 |
| metadata EML | 5.612 |
| blank-comments XLSX | 3.970 |

These measurements belong to the original `qualify-plan-*-02` calls, before the final readability changes. The four current `qualify-rechecked-*-04` outputs are offline rechecks with no new provider calls. Their documented deterministic fixture projection uses the saved candidate's table receipts; prepared source content is JSON-identical to the original. The final PDF `qualify-flow-rechecked-scan-02` also reuses a saved model plan offline, with an explicit configuration-only 10-to-20-second profile projection. Its Markdown and crop are identical to the independently source-checked accepted output. None of these replays constitutes a new latency or model-quality measurement.

A separate actual production CLI import of the three-page text/scan PDF succeeded in 25.211 seconds, showing Reading, Luna high and Checking stages, then a clean protected note and canonical image. General `gpt-6-sol` medium settings were unchanged (`artifacts/live-integrated-02/summary.json`). This was before the final optional line-order prompt extension; that extension was qualified by transport/checker tests, not another live provider call. The three prepared PageText blocks (including the empty scan text block) are byte-identical after the final flow correction.

Actual native picker/import/progress/Save/Read was observed using an isolated application and fresh synthetic data. The final email read view showed named metadata and zero-versus-blank table values without internal comments; records confirmed one protected clean Note, Source and Asset with a canonical image link, and unchanged general settings. Stop during a separate spreadsheet conversion left those counts unchanged. This is targeted native observation, not full product acceptance. A subsequent cold native reopen could not be completed because the capture API repeatedly returned error `-3812`; cold storage reopen and offline retained content are separately exercised by the repository checks. No native cold-reopen success is claimed.

## Verification and independent review

Raw logs, failed attempts and synthetic provider receipts remain in ignored `artifacts/`; they contain no copied credentials. Source code approval by the independent Sol reviewer covers reconciled commit `3eca2f67`, including an independent rerun of all ten PDF tests. Initial numeric corruption, trailing prose, crop-budget timing, inline figure ordering and PDF sorting findings were corrected. Fixture attributes subsequently preserve the nested PDF as binary and email CRLF bytes; `git diff --check origin/main` now passes without changing fixtures.

| Check | Command/result |
| --- | --- |
| PDF flow, coordinates, scan/crop binding and complete line permutation | `cargo test -p brn-threads-intake --lib prepared::tests --locked --offline`: 10 passed, `pdf-flow-order-final.log` |
| Dedicated provider mocks | `cargo test -p brn-ai conversion --locked --offline`: 9 passed; strict library/test Clippy passed. Covers source-line metadata, default/explicit ordering, fixed model/effort/no tools, refusal/truncation, timeout/cancel and payload-free errors. |
| Restricted reader/formatter helper | Strict Clippy passed in `helper-clippy-integrated.log`; the final helper all-target rerun passed 64 tests on source tree `3eca2f67` in `helper-final-07.log`. |
| Final integrated default gate | `BRN_PDFTOPPM=<qualified native renderer> bash scripts/verify-storage.sh`: passed on source tree `3eca2f67`; `storage-final-06.log`. Includes formatting, helper/default build, strict workspace lint, 669 passing workspace tests (four ignored) and fresh headless startup. |
| Final native widgets | Strict `native-ui,native-test-support` Clippy and parallel `threads_ui::` tests: 145 passed (2 tests outside the filter), on source tree `3eca2f67`; `native-final-07.log`. Earlier parallel 147-test gates passed twice after a reduced-motion fixture correction. |
| Combined native/retrieval desktop | Build, strict all-target Clippy and 54 state tests passed before final flow/provider reconciliation (`combined-final-*.log`); required PR CI qualifies the final integrated tree. |
| End-to-end synthetic storage | `bash scripts/verify-end-to-end.sh --fixtures-only`: passed Save/recovery/search/export/backup/startup/foreign-data refusal and retirement checks, `end-to-end-integrated.log`. |
| Documentation/tooling | Markdown links passed; 133 tooling tests passed, `docs-final-02.log` / `tooling-final-02.log`. Final documentation links are checked again after this record. |
| Renderer/transient witnesses | External reads and loopback network denied, owned scratch writes allowed, descendant cancellation, byte/entry quotas and vanished Fontconfig cache-entry handling passed. |

Early full-Markdown responses failed binding/list/escaping checks and were not saved. PDF wrapper/font failures, the prior 10-second timeout under concurrent builds, invalid plans, invented arrow glyphs and a literal `--` Setext-checker false positive are preserved. A concurrent cache rename witness reproduced and fixed a narrow metadata `NotFound` race; the original generic integrated I/O failure is not conclusively attributed to that race. Earlier native modal timing failures remain beside the successful parallel reruns. These are qualification history, not passing evidence.

## CI compatibility correction

[PR #144](https://github.com/ewq100/brn-rust/pull/144) was published after local qualification. The first required Mac Core/CLI check failed in the public three-page PDF preparation test. Static phase errors narrowed it to rendering; they do not retain source or helper diagnostics. Subsequent public-fixture probes in CI ran both unrestricted and restricted-profile replicas successfully with all three valid PNGs. The replicas omit the live scratch monitor and differ in some process-resource details, so they do not prove the exact failing entry or system call.

The focused correction disables disposable Fontconfig disk-cache writes by setting the single explicit cache destination to `/dev/null`. Fontconfig retains a nonempty cache-directory list, avoids adding ambient default paths and keeps the scanned font set in memory when disk-cache writing fails ([initialization source](https://raw.githubusercontent.com/fontconfig/fontconfig/2.17.1/src/fcinit.c), [scan source](https://raw.githubusercontent.com/fontconfig/fontconfig/2.17.1/src/fcdir.c)). The existing file-only scratch monitor and sandbox remain unchanged. Identical local native runs with and without disk caches produced the same three PNG SHA-256 values; actual monitored library/qualification tests passed 25 + 11 tests with three subprocess witnesses ignored, and strict intake Clippy passed. The previously failing fixture and required Mac Core/CLI job passed on `5a0cb22e` in CI run `38630444342`; `pr-144-macos-no-cache-job.log` records the successful real monitored preparation. Disabling disk-cache writes resolves this reproduced CI failure. A specific `.LCK` race was not captured and is not claimed as proven.

The local staged native renderer reports Poppler 26.05.0; `26.08.0` is merely the staging directory label. The observed CI renderer is Poppler 26.09.0. The diagnostic source and logs remain in ignored artifacts, but the temporary unrestricted/restricted shell probes are removed from the final workflow. Final CI still runs every original check.

The optional Windows Core/CLI job fails in existing Unix-only `auth.rs`/`codex_auth.rs` code; both files are unchanged from main. Required Linux and both native Mac checks passed in the original run. No all-platform success is claimed.

## Integration status and limits

This is the pre-merge qualification snapshot. Implemented, source-checked, independently code-reviewed and selectively native-observed; local final gates passed. Protected final-HEAD CI, merge and merged-main verification are pending at this snapshot; see [PR #144](https://github.com/ewq100/brn-rust/pull/144) for current integration status and receipts. The user authorized this scope and merge; this record does not claim separate owner acceptance of every native journey. DOCX cached note association/chart graphics, legacy spreadsheet drawings, complex layout/scan interpretation and independently unverified coverage remain explicit limitations. No universal Sol-quality parity, cost reduction, all-platform success or release is claimed.
