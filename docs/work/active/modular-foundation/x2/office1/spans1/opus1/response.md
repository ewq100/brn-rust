# P2 correction and round-two actual-code review brief

The [full raw review](opus-review-round1.txt) found one P2 complexity defect and no additional correctness/security/scope blocker. The allocated correction is implemented; fresh independent round-two review and principal acceptance remain pending. [Requirements](requirements.md), [source pin](source-pin.json), [checks](check-index.json) and [handoff](handoff.md) preserve exact scope and evidence.

## P2: previous-row lookup

Only `src/extraction/docx/parser.rs::Table::to_native_grid` changes from compiled candidate `c111e78`. A row-local cursor replaces `previous.iter().find`. It skips preceding source columns, then requires the exact current column and width before continuing the same origin. It consumes that matched entry and never moves backward. The existing `previous = active` replacement preserves immediate-row continuity. Checked column/span arithmetic, origin text, row spans, header/style/empty-cell behavior and the other two production files are unchanged.

Current cell columns increase because spans are positive. Previous active entries were appended in that same order. Let P be previous active entries and Q current continuations: each cursor advance consumes one entry and there are at most P advances per row; each continuation adds constant peek/match work. This lookup is O(P+Q), instead of restarting a full scan for every continuation. This is a structural source proof of the local lookup bound, not a bound on complete parsing/rendering or heap usage. No sorting, HashMap, dependency, parser-policy, quota/iteration change or instrumented/copied algorithm was introduced.

The review’s 4,096-column / 8,390,656-comparison example is retained as an **analytical old-operation count**, not measured timing. The 1024 limit applies to one gridSpan, not table column count. No performance-red, benchmark, timing or RSS threshold is claimed.

## Permanent functional coverage

One test is appended without changing any original test byte. Its mixed-width source has five simultaneous chains over four rows, with widths 2/1/2/1/2, independent ends/restarts and a heading-styled origin. The expected public grid has exactly 12 authored origins, including two distinct three-row chains. The same test includes an unstyled 4,096-column control over three rows, with exactly 4,096 three-row origins. Exact typed row/column/span/content/header/style fields and complete coverage are asserted. Complete public legacy Table JSON checks dense cells, Markdown, page and style values; node attributes remain absent. Expected geometry/legacy values come from authored source patterns, not extraction output.

The focused test passed **1/0, 7 filtered**. The full harness ran once: **7 passed / 1 failed, 0 ignored / 0 filtered**. Only the retained prefix-variation empty-content assertion fails. Original orphan/mismatched/interrupted, span-edge, empty-origin, body/header/footer and native-consumer assertions remain unchanged. The new functional case is not claimed to fail under the old quadratic algorithm.

Original-tolerance complete comparison still passes for **22 DOCX + six consumer outputs** against the retained pristine captures. Additionally, **all 28 complete public values, including grids, equal the first patched candidate**; inputs match byte-for-byte. The two new focused/full captures also agree. Old 1/6 -> 6/1 receipts, logs, sources, binaries, dep-info and patch remain at their original identities, without rerun or relabeling.

Strict all-target harness Clippy `-D warnings`, direct owned rustfmt, independent patch application and static checks pass as indexed. This does not establish dependency-wide lint. The corrected three-file distributable patch reproduces compiled selected source hashes; only parser lookup differs from the first patched production source.

## Custody, scope and limits

Fresh genuine Git source root `/private/tmp/brn-x2-office-s3-r1-20261018/final`, separate fixed `final-target`, and real 0700 `tmp-final` avoid overwriting or aliasing original roots. The copied lock/manifest are byte-identical. All three commands use Rust 1.98.1, locked/offline/jobs1 and serial tests; source hashes are unchanged across commands. Emitted 239 registry identities/features exactly equal the approved first candidate, with Xberg normal `cfg(test)=false` and `html,notebook,office`. No new metadata reconciliation, dependency/source acquisition or full audit was performed.

The raw report is copied verbatim and hashed. New source/artifact receipts pin actual compiled Git before execution; final repository commit is recorded after freeze. Original patch/provenance copies and artifact hashes were preserved/verified before the new build. No previous report/receipt, BRN Rust/test/oracle/registry/manifest/root lock, global registry/cache or PDF checkout was edited.

The shared native-grid attribute consumer and extractor paths are byte-identical to the first candidate; their global native-PDF reach stays qualified and real PDF consumer execution remains unexecuted. Complete XML/DTD/read-set/resource and repeated-pass bounds, namespaces, nested tables, notes/annotations/locators, renderer associations, PPTX O2, restricted runtime, root adoption, reader retirement and all-eight promotion remain open. All prior PDF/SDK/widget/platform/INT-W/native-human/owner/program/live/main qualifications remain. No paid/provider/Auth/network/GUI/mutants/PR/push/merge or review delegation.

Round-two review should inspect the cursor’s monotonic bound and exact-match semantics, unchanged original assertions, independently authored parallel-chain geometry/legacy values, complete compatibility checks and compiled-source/patch custody. This response does not self-accept the component.
