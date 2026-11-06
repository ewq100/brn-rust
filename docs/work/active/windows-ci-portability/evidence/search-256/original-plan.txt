# Search efficiency cycle — issue 256

Agent E; isolated `work/agent-e-256-search`, baseline
`a921e32533dc03134ed8a2001bd82f27194c3302`. This evidence supports the
`search-256` entry in the existing [selected record](record.json).
The owner authorizes a focused PR, headless synthetic execution on the existing
Codex subscription and bounded Council reviews. No merge, release or model-default
change. D's reader, writer and corpus stay unchanged.

## Frozen plan before dispatch

1. Qualify pinned Rust/locked build, evaluator, reservation/deadline and evidence
   checks; freeze CLI/seeder/helpers, inputs and evaluator hashes before live work.
2. Screening: existing R8 long-note retrieval, R10 abstention, and only R7's first
   general-context question (omit its write and private-answer steps; preserve
   private traps, remap safety checks, add no-writes). Two repetitions, fixed
   Codex `gpt-6.1-sol` medium, ordinary subscription tier, helpers off, keyword
   retrieval/no dense assets, scale 1000. No new writer calls: checked seeded
   synthetic records remain frozen in equivalent per-arm clones.
3. Run baseline repetition1 for all screening cases, inspect waste, then freeze one focused candidate initially. Run candidate repetitions1 and2, then baseline repetition2 using the retained unchanged binary (A-B-B-A per case).
   The issue allows at most three revisions, but this fixed allocation funds only one paired
   candidate. Revisions2–3 and reruns are blocked in this cycle. No revision or extra dispatch may consume confirmation capacity.
   Existing section-first guidance and compact catalog listings are already
   implemented; do not reimplement them. Preserve all attempts and patches.
4. Compare baseline/candidate per case/repetition. Require all safety checks,
   no new quality defect, complete correct claims and supporting citations,
   appropriate abstention and zero private exposure. Independent anonymous
   packet review covers every answer (one answer per request, fixed Sol/high/tools[], randomized order with arm labels removed). Mechanical citation resolution checks exact source IDs/versions; literal checks alone are insufficient. The R7 projection retains private traps and all scope fixtures. Its frozen oracle checks original no-writes receipts plus zero mutation-tool calls, zero candidate/operation IDs and unchanged canonical Note/Link/Action records, allowing host Messages/Run bookkeeping.
   Primary population: all six screening case/repetition pairs; input is the sum of provider-reported total input across every request/attempt, including resends and summaries, per run. Report uncached/cached input and output separately. Accept only >=10% reduction in the median of the six paired relative total-input changes, with reduction in both repetitions of an affected case, or >=1 unnecessary model/tool call removed in both repetitions of an affected case. An unnecessary call is an identical search with unchanged scope/query/limit/cursor, an unchanged-version read wholly covered by an earlier read, or an intermediate metadata read that provides no evidence or version absent from the preceding search. Count HTTP requests and tool invocations separately. Require no per-case increase in total input+output or model/tool calls, with no quality regression. Report timing as secondary
   unless contention-free paired repetitions agree. Small N, no significance claim. Missing usage or grader failure blocks acceptance. A failed/uncertain, timed-out or cap-stopped baseline run invalidates its pair and the complete screening criterion; a failed candidate rejects adoption. Never score truncation/completion differences caused by a cap as an efficiency win. All such attempts remain in totals.
5. Two additional existing cases are reserved outside the tuning context.
   Keep their questions, corpus and gold out of tuning Council. Run both arms
   twice only after freezing the candidate; failure rejects adoption. Relevant
   offline visibility, version/full-read and context-resource regressions remain.
6. Bounded Rig diagnostic uses the planned evidence and offline synthetic
   English/Estonian, tools/JSON/history/compaction probes, in shadow mode.
   Compare BRN byte admission, Rig whole estimate and usage-anchored estimate;
   missing real usage stays unknown. Use pinned Rig Prepared::whole_tokens separately from next_request_tokens with a recorded input+output Usage anchor, RequestStart and exact through MessageId. Report anchor validity and fallback separately: a fallback is a whole estimate, not a measured anchored estimate. Rig fingerprints model/prompt/tools/history but not provider/account; the caller must reset on those changes. Prompt/tool/history edits and compaction reset the anchor; missing actual request shape/usage leaves accuracy unknown. No estimates may alter production admission or transcripts. Check route/account/model invalidation.
   Preserve all byte/resource and validated context limits. Accuracy without
   demonstrated product benefit cannot justify adoption; defer is valid.
7. Final bounded Council evidence/code audit, focused checks, separate PR.
   Joint adoption with D requires baseline/knowledge-only/search-only/combined
   checks after D freezes its candidate; pending combination is not adopted.

## Allocation and accounting

Shared upfront escrow: `Documents/Codex/brn-shared-de-20261106/reservations.jsonl`,
1760 = D1592 + E168, no refunds. E's local immutable `reservations.jsonl` under
`Documents/Codex/brn-agent-e-256` reserves72 screening BRN +48 confirmation BRN
+24 independent grader +12 hypothesis Codex Council +12 final Codex Council.
Each BRN event has a hard6 HTTP attempt cap, including resends/summary/helper
attempts, and90 seconds command timeout. This funds12 screening and8 confirmation
runs. Failure/uncertain attempts consume their reservations; no automatic reruns.
Live execution cap5400 seconds; Council wall-time cap1800 seconds across routes,
two rounds, each seat requested <=12 requests/600 seconds per round. Non-Codex
seats capped at24 requested requests per route over the cycle. T3 cannot enforce
the Council per-request cap; retain requested versus observed/unknown separately.
Parent orchestration HTTP usage is outside enforceable BRN accounting; no claim
that this ledger caps the subscription as a whole. Recheck quota on denial or
before another phase; stop on admission failure, without credits or fallback.

D's sanitized account discovery reports ordinary usage allowed,29% weekly used,
no spend-control denial or credit balance, Sol medium advertised. Remaining
request capacity is unknown; percentage is not an inferred request allowance.
Heavy builds/latency wait for B; D/E latency measurements serialize.

## Current evidence

- Evaluator105 tests passed. AgentBench88 tests passed,11 native/configuration
  witnesses skipped; no build/native/live qualification inferred from skips.
- Independent reviews, binary/input freeze, baseline/candidate measurements,
  diagnostic and final decisions are pending. No improvement claimed.

Council hypothesis round1: Sol/Codex7/10 (four tool calls; model request count/usage unknown); Kiro/Opus failed monthly quota; authorized Copilot/Opus fallback6.5/10 (about5 model requests reported, tokens unknown). Kimi/Copilot7/10 completed21:31:57Z, about5 model requests self-reported,tokens unknown. The completion notification was not delivered; cancellation at21:39:26 returned completed and retained the review. No extra run occurred. Methodological findings above are addressed before baseline; candidate selection still awaits traces. ASCII serialization restored unrelated record escapes.

Offline read-only projection oracle:5 tests pass, covering expected host bookkeeping, unapplied proposals, refused mutation attempts, changed/missing knowledge and missing trace. Deadline guard11 tests pass. E reuses D's capability-gated no-tool Sol/high judge after its compilation/qualification; no duplicate provider adapter. Native build and fixed grader hash remain pending B's slot handoff.

Kimi findings: retain provider-reported usage as the sole token acceptance source (no estimator proxy/circularity); unknown usage blocks token-based adoption. D's separate no-tool grader caps each invocation at1HTTP, so24 reservations cover20 single-answer packets plus4 failed/uncertain attempts, not four6-call BRN runs. Cap-free live rehearsal is rejected: it would violate the finite allocation; scripted path checks plus explicit cap-failure rejection suffice. Keyword/no-dense findings do not establish semantic-model performance. New shadow diagnostic is test-only, uses explicitly synthetic usage for anchor protocol checks and cannot establish actual provider accuracy. Production admission/transcripts are unchanged.

Resource checkpoint from A/D: A confirms B's slot remains held since20:08Z without an acknowledgement/release. No qualifying CLI/seeder exists at the selected source. E will not use A's mixed target. Same-input helpers `/private/tmp/brn-rig046-helpers` may be reused after exact hash/input checks; they do not qualify the CLI. Headless live execution is pending the slot, not alpha publication. The live5400-second deadline has not started because no live phase was dispatched.

## Evaluation separation clarification

Owner clarification: a fresh evaluation-only role on the same machine, with reserved questions withheld from the tuning context, supports only a limited engineering conclusion. This is organizational separation, not technical isolation or evidence of broad model/search superiority. E's two reserved existing-case questions remain unopened by the tuning lead and tuning Council; their packet evaluation must use a fresh role without parent tuning history. D's newly authored whole histories and changing writer/corpus stay separate from E's frozen existing retrieval corpus. E's168 ceiling and48 confirmation BRN reserve are unchanged. No additional evaluator setup dispatch is inferred from D's allocation. Build/live remain pending B's explicit handoff.

Operational admission follow-up: the E phase wrapper stops on a missing prior terminal receipt, failed deadline, orphan sample, failed in-progress event or unknown identity/accounting before another CLI launch. It audits both persisted results and the active in-memory run_step result, because native v2 reads the Run before publishing that event. A CycleStop persists the active sample and bypasses per-sample recovery. Phase order is fixed;9 offline oracle/admission tests pass. No model dispatch or new allocation.

## Hardware correction and resumed qualification

Owner correction: B runs on the Mac mini; D/E run on the MacBook. B's slot is not an E dependency and no acknowledgement is needed. The earlier wait was based on an incorrect shared-machine assumption. E resumes locked offline builds in its own target, coordinating MacBook heavy-build and timed slots directly with D; original168 allocation and benchmark requirements remain unchanged.

Baseline qualification: default-feature CLI and seeder built in83s from a fresh E target at a921e325. Exact hashes in `evidence/search-256/build-baseline.json` when copied for delivery. Five corpus fixtures opened successfully; R7 BLUEFIN has0general/1private hits, and two R8 corpus/index clones return the same fixed-query page. Stable SQLite files are hashed; transient WAL/SHM files are excluded after closed-process offline qualification. Initial Rust-test compile failed on ambiguous collect in the new test-only probe; fixed by passing the iterator, preserving the failed log. FullAI608passed/4failed/1ignored because document-helper overrides were absent; all9 document-refinement checks then passed with explicit verified helpers. Rig shadow/wire23 checks pass. AgentBench97tests pass with baselineCLI,6remainingnon-applicable/configuration skips. E released the MacBook heavy-build slot to D; no live call yet.
