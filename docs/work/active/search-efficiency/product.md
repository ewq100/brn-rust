# Search efficiency product result

BRN can answer from a bounded exact passage carried by its first search hit. The passage uses the existing checked Note reader, with the same scope, version and delivered-read guarantees. This can remove a separate model request to read that passage. Longer or additional evidence remains available through the existing reader.

## Final comparison

Six fixed synthetic questions, two repetitions each, fixed Codex subscription Sol 6.1 medium, keyword retrieval and the frozen Sol/high no-tool grader. A fully correct answer requires all four quality criteria at 4, completed execution, source support, appropriate abstention/visibility and unchanged knowledge records.

| Measure | Baseline | Corrected candidate |
| --- | ---: | ---: |
| Fully correct answers | 12/12 | 12/12 |
| Input tokens, including cached | 419,643 | 360,691 |
| Output tokens | 4,146 | 3,635 |
| Input + output per fully correct answer | 35,315.75 | 30,360.50 |
| Model HTTP attempts | 47 | 41 |
| Recorded tool calls, including host reads | 57 | 74 |
| Automatic checked passage reads | 0 | 24 |
| Cached input, already included above | 346,880 | 288,384 |

The six-case comparison measured **14.03% lower** tokens per fully correct answer. Repetition totals fell from 202,777 to 176,395 and from 221,012 to 187,931 tokens, reductions of 13.01% and 14.97%. Screening of the third hypothesis separately showed 9.34% lower tokens with 8/8 correct answers. These are small synthetic samples, with visible variation in query choices. The comparison supports this bounded lookup improvement. Latency and subscription-charge savings are unqualified.

| Case | Baseline tokens/answer | Candidate tokens/answer | Fully correct |
| --- | ---: | ---: | --- |
| R8 | 27,343 | 15,494 | 2/2 → 2/2 |
| R10 | 41,492 | 27,511 | 2/2 → 2/2 |
| R7V | 42,772 | 46,333 | 2/2 → 2/2 |
| R7P | 23,040 | 15,369 | 2/2 → 2/2 |
| R1 | 41,265 | 43,419 | 2/2 → 2/2 |
| R4 | 35,984 | 34,037 | 2/2 → 2/2 |

The reserved R1/R4 pair used 77,248.5 baseline tokens and 77,456 candidate tokens per pair, a **0.27% increase**. Quality held on both reserved cases. The efficiency benefit comes from tuning cases, especially the repeated single-passage R8/R7P lookups. Baseline itself moved from 30,097.88 to 33,661.50 tokens per answer on the same four tuning cases between screening and final measurement, an 11.8% drift. Sequential phases and two repetitions leave the effect size on multi-search questions unproven. The mechanism and repeated one-passage savings support the bounded change; the result does not establish a general 14% saving.

R8 covers a long-note section, R10 abstention, R7V general visibility and R7P authorized private visibility. R1/R4 were reserved for final confirmation and excluded from tuning. The lead saw R1 during preparation. There is no external unseen-evaluation-owner claim. The R7 fixture was projected into its original public/private read-only questions before results, excluding its writing step.

## Attempts and limits

| Phase | Product attempts | Fully correct | Input + output tokens | Model HTTP |
| --- | ---: | ---: | ---: | ---: |
| baseline | 8 | 8 | 240,783 | 28 |
| candidate1 | 8 | 8 | 267,246 | 30 |
| candidate2 | 8 | 8 | 275,043 | 28 |
| candidate3 | 8 | 8 | 218,283 | 24 |
| final-baseline | 12 | 12 | 423,789 | 47 |
| final-candidate | 12 | 12 | 364,326 | 41 |


The single window began **06:24:46 UTC on 7 November 2026**, with deadline **10:24:46 UTC**. Last grading completed at **2026-11-07T06:54:01.969556+00:00**, 29.3 minutes after first live dispatch. Product revisions, builds, measurements and grading occurred inside it. Initial reference artifacts and offline fixture checks preceded the first live request.

All 56 product attempts completed, with no product retries, model helpers, failed product attempts or unknown product usage. All runtime identities were verified from retained native and wire evidence. There were **198 product HTTP attempts plus 56 grader attempts = 254**, below 600. Conservative reservations consumed 504 slots and were never refunded. The score includes all product input/output in each compared configuration, including host-assisted execution. Cached input is included once inside reported input. Host reads add no separate model HTTP request. Their trace calls carry `via=search_notes`; parent search bytes account for the delivered JSON, avoiding duplicate byte totals.

Grading overhead was **687,356 input and 12,731 output tokens**, 56 one-request judgments with no failures or retries. It is excluded from product scores. Council/build-agent actual HTTP and tokens are unavailable. Council has a separate finite 30-minute/six-seat allowance. Subscription charges are unknown.

## Rejected revisions and implementation correction

1. Conditional reformulation/stopping guidance used 267,246 tokens, 11.0% above the original 240,783-token screening baseline. All eight answers were graded fully correct. Rejected.
2. Compact reading guidance used 275,043 tokens, 14.2% above screening baseline. All eight answers were fully correct. Rejected.
3. Inline exact passage reads used 218,283 screening tokens with eight fully correct answers. Before confirmation, the implementation gained a combined-output guard so undelivered passages cannot grant Note-read coverage. Confirmation uses a separately pinned corrected binary. The screening and corrected runtime hashes and exact source patches are retained separately.

The accepted runtime adds at most a 1 KiB exact read of the first hit for ordinary parent searches. Existing writing-mode and research-helper reading paths retain their own behavior. The reader enforces chat/project visibility, live-run fences, exact versions and UTF-8 boundaries. Every attempted automatic read is traced, including failures. A stale/unavailable passage is explicit. The combined encoded search result is checked before the read ledger records delivered coverage. Direct read_note callers retain their entry point.

## Evidence and verification

[Compressed operational evidence](evidence.json.gz) contains the fixed fixture and criteria, evaluator freeze, every product result and judgment packet/receipt, runtime/build identities, rejected patches and executed source snapshots. [Machine-readable totals](summary.json) retain every attempt and per-case usage. Native SQLite/checkpoint backups and immutable binaries are additionally retained locally under `/private/tmp/brn256-20261107`, with a durable copy at `/Users/evokessler/repos/brn-256-evidence-20261107`. Opaque record IDs differ across freshly seeded roots. Fixture text, seed, versions, questions, scope and evaluator are fixed.

Local verification currently passes 52 search/research tests, nine read-ledger tests, 28 summary tests, three guide tests and 13 writing-evidence tests, plus strict all-target capability-spike Clippy. The 94 Python benchmark tests pass with 11 optional skips. Final current-head Codex code/security and hosted CI remain pending. Qualification is headless and synthetic.

Salvage [PR #273](https://github.com/ewq100/brn-rust/pull/273) merged as `bab8c876`, and its four required main contexts passed. Optional Windows main lanes failed in untouched desktop/skills paths, including missing native methods/imports and unused command-description helpers. [Main CI](https://github.com/ewq100/brn-rust/actions/runs/42345387252) preserves those failures. [Historical outcomes](outcomes.md) keep the old Terra/Sol comparison inconclusive, #270 unqualified and the old search candidate rejected under its original rules. PRs #269/#270/#271 are closed as superseded, with branches preserved.

B's selected release source `82657cab` and C's separate work are preserved. Trials ran headlessly after B reported no build/provider workers and its retained app was observed idle at 0% CPU while screen-locked. Release publication and hands-on owner qualification remain separate. Issues #254/#256 remain open.

## Final Council disposition

Two focused rounds used Codex GPT-6.1 Sol high and Copilot Claude Opus 5.5 high. Selection scores were 6/10 each. Final scores were Sol 9/10 and Opus 7.5/10, before the following report/test corrections. Kiro's initial selection dispatch failed on quota, then the owner selected Copilot/Opus. Five seat dispatches used 13.1 cumulative seat-minutes, including that failure, within the 30-minute allowance. [Council receipts](council.json) record the actual routes and timing. Kimi was excluded.

Sol recomputed every phase and verified 56 native-result hashes, packet/receipt bindings, freeze hashes and binary hashes. Opus independently reconciled the totals and runtime guards. Both found no runtime correctness/security defect. Opus requested narrower claims, the reserved-set/drift disclosure, an updated behavior contract and explicit writing/helper omission assertions. These corrections are included. Its tool-description copy-edit and richer failure-reason metadata are follow-ups. The measured prompt text and failure protocol remain the qualified runtime inputs. Failed automatic reads are retained as error attempts and `Unavailable` grants no read coverage.
