# Search efficiency cycle — issue #256

**Decision: no production search change adopted.** Candidate 1 batched independent
search reformulations. It reduced input and model requests, but failed the frozen
rule against increasing tool calls in any case. The original query description is
restored. Rig estimator adoption is deferred. Nothing is merged or released and
the app's model default is unchanged.

This is the bounded first cycle authorized in [#256](https://github.com/ewq100/brn-rust/issues/256),
coordinated through [#253](https://github.com/ewq100/brn-rust/issues/253). The current
`search-256` task lives in the [selected record](record.json).

## Measured result

Each arm completed six read-only runs: R7's general-context question, R8's long
handbook lookup and R10's abstention check, each repeated twice. R7's writing and
private-answer steps were excluded; its private trap and scope fixtures remained.
All twelve runs completed with no transport failure, retry, timeout, cap stop,
mutation attempt, pending proposal or canonical knowledge change.

| Observed usage, six runs per arm | Baseline | Candidate 1 |
| --- | ---: | ---: |
| Input tokens | 270,923 | 217,690 |
| Cached input, included above | 190,976 | 161,792 |
| Uncached input | 79,947 | 55,898 |
| Output tokens | 2,645 | 2,583 |
| HTTP model requests | 26 | 20 |
| Tool invocations | 39 | 40 |
| Tool rounds | 20 | 14 |
| Sum of native run elapsed time | 105,883 ms | 88,065 ms |

The median of the six paired relative input changes is **−22.7338%**. Aggregate
input changed by −19.6486%; these are different populations. Each pair used one
fewer model request. **R10 nevertheless increased from five to six tool calls in
both repetitions**, violating the frozen per-case gate. We reject the candidate
without changing the criterion after seeing results.

Timing is secondary, small-sample evidence with ordering and prompt-cache limits.
It establishes neither broad latency superiority nor subscription-charge savings.
Money cost remains unknown.

The fixed independent grader produced twelve complete, hash-bound judgments:

- Eleven answers scored 4/4 on correctness, completeness, citations, abstention
  and visibility.
- Candidate R7 repetition 2 scored correctness 3/4 for an auxiliary statement
  about saved discussion. The grader packet omitted that Message's contents and
  version. Its central abstention, citations, completeness and visibility scored
  4/4. We retain the finding and packet-coverage limit; we do not claim an overall
  absence of quality regression.
- Thirteen grader HTTP attempts include one response that mistyped its packet
  hash by one character. The original response was rejected and retained; the
  unchanged packet was deliberately retried within the original failed-grader
  reserve. Valid low grades were never rerun.

The grader used fresh requests without parent tuning history on the same machine.
This supports a **limited engineering conclusion**, not technical isolation or
calibrated broad semantic competence. Transport fixtures do not qualify semantic
judgment accuracy.

## Freeze and execution

Baseline revision: `a921e32533dc03134ed8a2001bd82f27194c3302`, after response
hardening #264 and portability #265. E used an isolated worktree, its own fresh
Cargo target and synthetic data. Production Rust was unchanged for the baseline;
the added estimator test is test-only.

The CLI and seeder were built with exactly:

```sh
cargo build --locked --offline -p brn -p brn-threads-app \
  --bin brn --example wiki_eval_seed
```

The same target selection was used for the candidate to preserve Cargo feature
unification. A preliminary CLI-only candidate build was discarded before live
calls; its log remains retained. Rust 1.98.1, the lockfile and default features
were fixed. No native retrieval or dense assets were selected. Requests could
still specify hybrid mode; trace results reported semantic coverage `none`.

| Immutable artifact | SHA-256 |
| --- | --- |
| Baseline CLI | `5497781af9bd72d85ea7dd5d22083972de047aaa58b385140b03b05f0e9ed94d` |
| Seeder | `4a23729e1706e74c210d0ef6d870632585b5a05d5934d4459f212a44325c44c2` |
| Candidate CLI | `235c10a2c1a83ec66d4a4475daba498a4e1d88ed7a494154829a7a25a913e0a7` |
| Fixed no-tool grader, qualified by D | `dc54b46c3f083b9b146e68ae76d977d5073202b03035e81c0ff672980164bdc6` |

The fixed candidate model was Codex `gpt-6.1-sol`, medium, ordinary subscription
tier, helpers off. Account discovery reported ordinary usage allowed, 29% weekly
usage, no spend-control denial and no credits; remaining request capacity was
unknown. The model and evaluator stayed fixed throughout.

The frozen corpus used 1,000 filler notes per case and checked seeded records;
there were no live writer calls. Each arm copied the same canonical IDs, versions
and derived index. Stable files were hashed after offline opening checks;
transient SQLite WAL/SHM files were excluded. Two independent clones returned the
same fixed-query page. The R7 trap returned zero general hits and one private hit.

The [original plan](evidence/search-256/original-plan.txt),
[criteria](evidence/search-256/criteria.json),
[freeze receipt](evidence/search-256/freeze-receipt.json) and
[candidate freeze](evidence/search-256/candidate-freeze-receipt.json) preserve the
pre-result choices. Baseline repetition 1 preceded tuning; candidate repetitions
1 and 2 followed; retained baseline repetition 2 completed A–B–B–A ordering per
case. Candidate 1 changed only a query-schema description to batch independent
reformulations while preserving later dependent searches and pagination. Existing
direct passage reads already worked and were not reimplemented.

## Finite accounting

Shared upfront escrow: D 1,592 + E 168 = **1,760 reservations**, with no refund or
reset. E's fixed split was:

| Role | Reservations | Actual known HTTP |
| --- | ---: | ---: |
| Screening BRN | 72 | 46 |
| Reserved confirmation BRN | 48 | 0 |
| Independent grader | 24 | 13 |
| Codex Council | 24 | unknown |

Each BRN event had a hard six-attempt cap and 90-second command timeout, including
resends and compaction/helper attempts. The grader had a hard one-HTTP cap,
45-second internal deadline and no automatic resend. There were no BRN failures;
the one grader binding failure remains in the thirteen-attempt total.

Known BRN plus grader usage is **59 HTTP attempts**, 737,526 input tokens and
14,356 output tokens. Grading alone used 248,913 input and 9,128 output tokens,
with 226,256 ms elapsed across all thirteen attempts. It is outside measured BRN
execution. Parent orchestration and Council actual HTTP/token use are not fully
observed or enforceable by the BRN ledger and remain unknown.

The original live deadline was 22:07:34–23:37:34 UTC on 6 November 2026 and never
reset, including waits. Council had a separate 30-minute wall-time allocation;
per-seat request/time limits were requested, not hard-enforced by T3. The final
requested 22:30 cutoff was exceeded: Opus completed at 22:30:14, Sol at 22:30:16
and Kimi at 22:31:08. The latest overrun, 68.8 seconds, is retained. Cancellation
and readback reconciled all three as completed before the explicit handback.
D did not start its next models until that handback.

B runs on the Mac mini; D/E run on the MacBook. The earlier B-slot wait was an
incorrect shared-machine assumption, corrected by the owner before execution.
D/E then serialized heavy builds and provider measurement windows with separate
targets, binaries and data.

No second candidate was funded. Confirmation's 48 reservations remain unused and
unrefunded because candidate 1 was rejected. No D knowledge was imported. Reserved
and baseline/knowledge-only/search-only/combined checks were not run; they remain
required before any later joint adoption.

## Rig estimator recommendation: defer

The whole-context counter and usage-anchored next-request estimator are distinct.
Pinned Rig 0.46 synthetic English, Estonian and JSON probes compare BRN serialized
byte estimates, `Prepared::whole_tokens()` and explicitly synthetic usage anchors.
They verify model, prompt, tool, history-replacement and compaction invalidation,
unchanged transcript bytes and fallback semantics. `Prepared` has no provider or
account fields: a caller must reset across either boundary. Fallback to the whole
estimate is never labelled observed anchored accuracy.

All 46 BRN HTTP attempts have actual usage, but production traces retain bounded
identity/tool summaries rather than full prepared requests. Consequently there is
**no matched live-request estimator prediction-error comparison**. Synthetic
accuracy fields remain null. The diagnostic's source and coverage limits are
explicit; it does not replace the completed search loop.

Estimator accuracy alone is not an efficiency gain, and admitting more context
could increase usage. No admission rule, validated context window, hard byte or
resource guard, session store or default payload recording changes. A justified
adoption needs a separate measured product benefit.

## Verification and review limits

- Baseline build and five offline corpus cases passed.
- AgentBench: 97 tests passed with the baseline CLI; six optional/configuration
  witnesses skipped. Evaluator: 105 tests passed. Deadline guard: 11 passed.
- Full AI tests: 608 passed, four failed because helper overrides were absent,
  one ignored. With verified helper paths, all nine document-refinement tests
  passed. Original failures remain retained. An initial type-inference error in
  the new test was fixed before execution and its log retained.
- Search regressions: nine passed. Rig shadow/wire selection: 23 passed.
- Final lightweight admission/oracle tests: twelve passed. Post-run hardening
  re-derived all twelve attempt objects with exact equality.

Council used two rounds with three model families. Kiro/Opus hit its monthly
limit in round 1; the installed named fallback was Copilot/Opus. Actual routes and
raw receipts are retained.

| Seat | Hypothesis | Final |
| --- | --- | --- |
| Codex / Sol high | 7/10 | no score; partial, output truncated |
| Copilot / Opus high | 6.5/10 | 7.5/10; partial |
| Copilot / Kimi high | 7/10 | 8/10; partial |

Sol could not independently validate the core final calculations/code because
its output was truncated. Opus checked portions of arithmetic, rejection and
unknown-usage handling; it did not inspect grader reconciliation or the archive.
Kimi checked arithmetic, the patch and synthetic-anchor assertions; it did not
fully reread result/freeze receipts. **No complete final Council pass is claimed.**

Low findings prompted post-run robustness fixes for missing summary fields,
unknown parent usage, ledger headers and active-sample lookup, plus a precise test
name. The suggestion that `except (CycleStop, Exception)` is redundant was rejected:
CycleStop deliberately subclasses BaseException to escape native sample recovery.
Only fixed host stop reasons are recorded; arbitrary exception payloads remain
omitted for privacy. These small fixes were not independently re-reviewed; no
further model round is funded.

Executed source snapshots match the pre-dispatch hashes. Historical freeze
receipts were verified before dispatch; current future-runner hardening changes
those path-based source hashes and does not silently replace measured code.
Sanitized synthetic answers, retrieved evidence, traces and all grader responses
are in [the compressed evidence](evidence/search-256/screening-evidence.json.gz).
Native SQLite backups remain local; their hashes were verified before export.
[The result record](evidence/search-256/cycle-result.json) carries per-pair usage,
failures, judgments and accounting.

Implemented: bounded experiment controls and test-only diagnostic.
Verified: the checks and limited measurements above.
Native-observed: not assigned or claimed.
Owner-accepted: experiment authority and criteria, not an adopted product change.
Merged/released: no.
