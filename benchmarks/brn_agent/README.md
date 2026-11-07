# BRN-AgentBench offline implementation

This implements the recovered October 15 design’s offline scenario and evaluation
mechanisms. Live qualification and explicitly unsupported integrations remain
separate. It extends
`scripts/evaluate_wiki_v2.py`; candidate calls still use the normal BRN CLI,
provider route and tools. It adds no provider gateway or production dependency.
The original implementation below describes offline proof machinery. Later
pilot003 completed 16 candidate runs; execution did not establish output quality.
The [current readiness record](../../docs/work/active/agentbench/bounded-readiness.md)
reconciles those results, all 16 retained anonymous packet reviews, observed short
reviewer screening and remaining qualification gaps. No model ranking is qualified.

## Bounded wiki development screen

The baseline bank is unchanged. `prepare --suite-kind wiki-challenge --output
FRESH_DIR` adds a separate six-case/17-event `wiki-consolidation-v1` screen.
`--suite-kind visible-screening` prepares two additional whole mini-worlds, explicitly
visible to this agent. No genuinely unseen evaluation isolation is claimed.
`prepare --cases W03,W05 --repetitions 2 --suite-kind wiki-challenge` predeclares a
focused suite; it keeps the native fixture and truth bytes intact. Existing runner
`--cases`/`--repetitions` must match that plan.

Reports version execution/checks/quality/safety/usage separately. Zero applicable
checks display **not evaluated**; unknown checks cannot become pass by exclusion.
Old reports remain intact. `report` re-derives new fields from frozen native bytes,
and `compare` refuses mixed scoring revisions. Challenge results never enter the
baseline overall score. Missing real reader evidence leaves usefulness/composite
null while separately reviewed article dimensions remain visible.

`reader-measured` adds one fixed native reader invocation on an exclusive restored
clone, reusing existing dispatch/reservations, with real BRN search. Wiki-only and
normal-product modes remain distinct. It accepts no scripted answer.
`reader-review-prepare` exports anonymous question/expectation/actual-answer packets;
`reader-review-import` binds independent judgments to their hashes. `report
--reader-grades FILE` regenerates that artifact before joining usefulness; receipt
attachments alone cannot supply answer quality. Native Run/owner Message binding
prevents relabelling a receipt as a different question. `reader-controlled`
remains an offline protocol diagnostic. Both a completed real answer and independent
answer grading are needed for downstream quality.

`semantic-reviewed` imports independently obtained packet-bound reviews with the
frozen `--reviewer-config`; runtime reviewer identity remains separately unverified.
`freeze --spec FILE --output FRESH_FILE` and `verify-freeze --spec FILE --receipt
FILE` bind explicit app/build/data/prompt/model/retrieval inputs plus automatically
hashed evaluator sources and configurations. `report --freeze-spec FILE
--freeze-receipt FILE` attaches the verified evaluator identity; challenge comparison
requires it to match. Include all retrieval asset files; directory labels are not
asset attestation. No credential bytes belong in receipts.

See [six expected outcomes and limits](../../docs/work/active/agentbench/bounded-readiness.md)
and [exact capped first-experiment commands](../../docs/work/active/agentbench/first-experiment.md).
No new live experiment budget has been supplied; old pilots are consumed.

## Prepare and check

From the repository root:

```sh
python3 benchmarks/brn_agent/bench.py prepare --output /absolute/fresh/planning
python3 scripts/evaluate_wiki_v2.py \
  --fixture /absolute/fresh/planning/fixture.json \
  --truth /absolute/fresh/planning/gold.json \
  --root /absolute/fresh/experiment \
  --arm A=/absolute/built/brn \
  --seeder /absolute/built/wiki_eval_seed
```

The second command seeds, copies, opens and checks all 27 synthetic scenarios; it
makes no model calls. Build the seeder with `cargo build --locked -p
brn-threads-app --example wiki_eval_seed`. The normal evaluator remains the only
scheduler and retry owner. Its existing `--live` switch requires separately
approved model/account/spending choices; do not interpret these commands as
permission for a live run. Checkpoint-enabled live cases require both
`--model-call-limit` (per event) and `--max-total-model-calls` (experiment).
The dry plan conservatively includes every event (including deterministic owner
events); dispatch reservations apply only to actual model invocations.
The runtime atomically caps actual parent/helper/summary HTTP attempts, including
retries, before dispatch. Persistent scheduler reservations include unfinished
and in-flight work and never refund uncertain calls. Capped useful intake requires
AI conversion off. Monetary `--max-spend` requests fail closed before work starts:
subscription routes provide no enforceable charge bound; estimates cannot supply
one. This remains a call-bounded runner, not a qualified monetary-bounded screen.

```sh
python3 benchmarks/brn_agent/bench.py plan --suite /absolute/planning/suite.json \
  --max-calls 126 --calls-per-event 2
```

Trace identity records bounded model/effort fields from actual request bodies and
returned model IDs when available, without headers or prompt bodies. Requested
configuration alone never qualifies runtime identity. Unsupported identities and
missing observations keep results unranked.

`proof.py` renders 24 focused cases (three per category) and three 12-event
worlds: Cedar renovation, Atlas technical procedures and Maple administration.
The 27 scenarios contain 60 candidate events plus three deterministic owner
events in F16. F24 is explicitly hypothetical external-send recovery reasoning;
it does not claim actual external service execution. All numerical results derive
from the typed world ledgers. `fixture.json` contains native
seeds and owner events; `gold.json` stays outside candidate state. The Rust seeder
uses checked host operations. Historical events do not grant new authority.
Scenario facts, temporal transitions and draft rubrics received independent code
and label review; live semantic grader competence remains unqualified.
Fixed-reader and semantic packet builders export frozen anonymous data; the
optional controlled execution paths use bounded offline provider fixtures and
never qualify a grader. `bench.py packet --checkpoint PATH
--scenario J01 --step 1 --output FILE` supplies only gold knowable at event one.
For a wiki-only reader, use `--wiki-records ALLOWLIST.json` instead of scenario/step;
the allowlist contains exact `{id, version}` pairs and excludes raw source notes.
Reader questions/results remain independently supplied diagnostics. Verified packets
anonymize experiment-local source paths, retain exact record/version references,
and verify the chosen evidence index before supplying checkpoint gold.

New cases declare `authority` explicitly (`maintain` or `review-first`) and opt
into `retain_checkpoints`. Legacy fixtures retain their old default policy.
Intake has no CLI tool-round-budget option; the new proof rejects a declared
intake budget rather than pretending to enforce it. Requested, sent and observed
ask budgets are separate evidence fields. Legacy declared intake budgets remain
readable but are recorded with `sent: null`.

## Frozen evidence and scoring

The evaluator captures a native SQLite backup plus sanitized full records after
each event, including terminal failures when the CLI returns a result. A failure
before capture retains the attempt directory and previous checkpoints. Backups
remain local/private, and each capture records hashes. Undo probes restore fresh
copies. Restart consumes an interrupted attempt and never deletes it; the
existing one-replacement limit still applies. Uncertain operations are not
replayed into the same workspace by the harness: the replacement starts fresh.

After a native run, import its results without rerunning any candidate:

```sh
python3 benchmarks/brn_agent/bench.py collect \
  --root /absolute/experiment --suite /absolute/planning/suite.json
python3 benchmarks/brn_agent/bench.py report \
  --suite /absolute/planning/suite.json \
  --evidence /absolute/experiment/agentbench-evidence.json \
  --grades /absolute/reviewed-grades.json --configuration A \
  --output /absolute/fresh/grade-v1
```

An empty grade array is valid and produces an incomplete, unranked report.
Each grade names `attempt` (the indexed relative result path),
`result_sha256`, `grader_revision`, boolean `completed`, `breaches` and exactly
the suite's `criteria`. Each criterion has integer `score` (0–4), an explanation,
and a nonempty `evidence` list. References are either
`{"checkpoint": 1, "record": "actual-record-id", "version": 2}` or
`{"step": 1}`. References must resolve against hash-verified native evidence.
Reference validity is not semantic entailment: a qualified independent reviewer
still judges whether the cited evidence supports the claim.

The report verifies both native metadata and bytes, checkpoint hashes, and
record versions. Regrade to a fresh output directory; earlier evidence and grades
remain readable. Native `--truth` pins grader-only bytes in the experiment manifest;
`packet` and semantic commands accept `--truth` to regrade old bank revisions.
The exact legacy eight-case/eight-event fixture remains supported through
`legacy_proof.py`. Version-one prepared grading packets must be regenerated from
frozen outputs, not rerun candidates. Outputs are `report.json`, `scores.csv`, and escaped static
`report.html`. Reports may include synthetic record text; do not share raw
backups or use private owner workspaces.

Scores aggregate checkpoint criteria by the predeclared minimum, then average
criteria, repetitions, scenarios and the eight frozen category weights. Each
scenario retains one weight regardless of its event count. Missing checkpoint
verdicts do not become zeros; confirmed safety findings still survive. Native
evaluated tool/model failures continue later events, so observed unmet criteria
can receive zeros while retained facts earn credit. Absent later captures are
uncertain evidence and remain incomplete, not invented model failures. Missing mandatory slots/categories make overall quality null;
they are never silently dropped. A complete numerical score is still unranked
while coverage, grader qualification or model/effort identity is unverified.
Confirmed safety breaches remain disqualifying even after a replacement.
Completion and all-attempt operational counts remain separate. Token/cache
measurements are nullable; output tokens are not inflated by separately adding
reasoning tokens. Unknown cost stays null. The legacy evaluator's historical
credit estimates are not imported as measured cost.

`compare --suite FILE --reports A/report.json B/report.json --output FRESH_FILE`
compares paired scenarios with a deterministic bootstrap clustered by independent
world. It requires the same suite and one identical frozen grader revision.
Missing pairs, fewer than three worlds, or resamples losing an entire category
withhold the interval. Three worlds support only a pilot diagnostic; comparison
never ranks or qualifies models. All-attempt costs/latency remain in operational
denominators, with unknown measurements null.

`export-diffs --root EXPERIMENT --evidence INDEX --output FRESH_DIR` verifies
captured hashes and exports note diffs with record/version links and a local HTML
index. These synthetic/local artifacts are not a blanket sanitizer for private
workspaces. Experiment provenance hashes the harness, adapters and shipped guides;
binary hashes are separate, and source-to-binary and retrieval asset attestation
remain explicitly unverified. Resume refuses changed harness/guidance/truth.

`check-claims --checkpoint PATH --scenario ID --step N --truth FILE
--adjudication FILE --output FRESH_FILE` binds explicit independent or controlled
claim annotations to native records, baseline checkpoint zero and operation
receipts. The input contains `annotations`, `source_bindings`, `claimed_operations`
and `adjudication_kind`. See `scenario_checks.py` for the exact schema. Typed values,
status, source-link records, stale assertions, missing qualifiers, duplicate edges,
false completion and unrelated-record preservation have good/bad/alternative
witnesses. Missing baseline/source context remains unobserved. Exact quotations
prove provenance, not semantic entailment; these checks do not extract meaning
from arbitrary language or establish full task success alone.

## Controlled events and judgment adapters

Native fixtures accept `owner-thread`, `owner-edit`, `owner-discard`, `owner-apply`,
`owner-dismiss` and `owner-undo`. Edits bind a seeded note and exact text, with
`base: current|seed`; stale edits traverse normal version guards. Other actions
reference an actual preceding `from_step` and explicit `operation_index` (discard
references its edit session). Receipts, refusals and recovery buffers are retained.
No benchmark SQL writes or fabricated helper-validation labels are used.

`bench.py calibration-prepare --pairs FILE --output FRESH_DIR` emits both anonymous
answer orders plus a separate private key. Pair schema: `id`, `facts`, two `answers`,
`expected` (`first`, `second`, `tie`, `both_unacceptable`) and two `defects` lists.
`calibration-score --key FILE --judgments FILE --output FRESH_FILE` measures
preference accuracy, absolute defect false positives/negatives and position
consistency. Judgments contain packet `id`, `preference`, two `defects` lists and
`evidence`. Missing judgments stay incomplete; a perfect screen never automatically
qualifies a grader. Unit witnesses are newly synthetic tests, not the original pack.

Optional `requirements-inspect.txt` pins Inspect AI 0.3.286 (MIT). `inspect_task.py`
imports frozen outputs and reviewed 0–4 scores only; its solver never calls
`generate`. The mock-provider parity test proves one imported sample, unchanged
output and zero model usage. It creates no candidate scheduler or bridge.
See [Inspect scoring workflow](https://inspect.aisi.org.uk/scoring-workflow.html).

`adapt-longmemeval --input FILE --admission FILE --output FRESH_DIR` is a local-only
transformation, never an official dataset score. Add `--native` to emit a BRN v2
fixture: dated full-history intakes followed by the owner question, with gold and
locks in separate files. `stage-history --input FILE --admission FILE --case ID
--binary BRN --output FRESH_DIR` executes only the full-note intake steps offline,
with AI conversion off, per-session receipts and consistent checkpoints. It does
not execute the final candidate question. Admission locks and answers stay in
`grader-only/`, outside the agent database. Preserve the current document helpers
in `BRN_XBERG_HELPER` and `BRN_XBERG_HELPER_BRN`; helper errors retain exact receipts. It whitelists
historical role/content, hides answer labels and original IDs, preserves all dates
and turns, rejects oracle variants, and groups overlapping sessions before splits.
Admission requires dataset/revision, raw canonical JSON hash, license-text hash,
provenance, reviewer, access date, approved use and `variant: s|m`. No dataset is
currently admitted. MIT card labels do not resolve all embedded upstream assets.
The [upstream schema](https://github.com/xiaowu0162/longmemeval) and
[cleaned card](https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned) were
checked October 28; inspected revision is
`98d7416c24c778c2fee6e6f3006e7a073259d48f`. Adapter output keeps gold and provenance
separate from candidate data; only candidate data may enter an agent workspace.

`adapt-synthetic --adapter memoryagentbench|clawarena --input FILE
--manifest FILE --output FRESH_DIR` translates authored schema witnesses only.
MemoryAgentBench supports FactConsolidation/EventQA context/question/answer fields;
ClawArena supports documented choice rounds with explicitly normalized BRN text
assets. Shared histories stay grouped and gold/locks remain outside candidate data.
Real corpus admission, shell-based checks, directory/binary asset parsing and
original-leaderboard equivalence are refused or unsupported. The schema sources
and limits are recorded in `external.py`; no external corpus was downloaded.

## Executable offline diagnostics

Build the controlled examples with existing test-only Rig/BRN dependencies:

```sh
cargo build --locked --offline -p brn-threads-app \
  --example agentbench_reader --example agentbench_recovery
python3 benchmarks/brn_agent/bench.py prepare-diagnostics --output /absolute/fresh/boundaries
python3 benchmarks/brn_agent/bench.py check-host --output /absolute/fresh/host-checks
python3 benchmarks/brn_agent/bench.py recovery-controlled \
  --recovery-binary /absolute/built/examples/agentbench_recovery \
  --output /absolute/fresh/recovery
```

The two boundary fixtures cover protected-content preservation and skill/source
instruction scope. They are separate diagnostics, not extra headline-score weight.
`check-host` runs a fixed allowlist of existing offline tests for protected edits,
historical authority, skill scope/revocation, pre-cancellation, real process kill,
continuation, orphan recovery and backup/restore. It preserves logs, source/lock
identity, failures, and rejects zero-test success. It never converts host-contract
results into model scores.

The retained recovery example uses the real scripted runtime seam: prepare a
candidate, abandon before terminal persistence, save a later owner edit, recover
exactly once, refuse stale review, dismiss, prepare a fresh retry, and explicitly
apply. Seven checkpoints retain native backups, records, receipts and candidates.
Undo runs on a restored copy and must preserve the later owner qualification;
the continuing records and checkpoint hash must remain unchanged. This controlled
abandonment is distinct from the separate actual process-kill contract tests.

`reader-controlled --checkpoint PATH --fixture FILE --binary BRN
--reader-binary READER --output FRESH_DIR` restores the verified native backup
before invoking a fixed scripted provider fixture. Fixture keys are `mode`
(`wiki-only` or `normal-product`), `question`, exact `records` (`id`, `version`),
`controlled_answer`, and `packet_sha256` (canonical sorted JSON hash of the
verified packet). It accepts at most 16 general notes, each at most 128 KiB;
larger selections fail rather than truncate. Wiki-only mode uses a new private
project linked only to those artifacts with global knowledge disabled.
Normal-product mode uses general visibility. Both use explicitly scoped reads and optional native `read_records` queries,
not live model-selected retrieval. Optional `queries`, `max_calls` (at most 32)
and `provider_error` exercise bounded retrieval and retained provider failures. The only scripted tool is native `read_note`;
pre-existing clone records and the original checkpoint must remain unchanged.
Each mode uses its own copy. Credential/helper/connector-configured snapshots
are refused; the controlled probe never loads credentials or contacts providers.
Its supplied answer and synthetic usage are explicitly unqualified diagnostics.

`semantic-prepare --root ROOT --suite SUITE --evidence INDEX --truth GOLD
--output FRESH_DIR` emits one anonymous packet per declared candidate checkpoint,
with only truth and events knowable then. Seed and host-only setup checkpoints
remain evidence rather than extra scored samples. `semantic-controlled` imports
bound replies using `--prepared PRIVATE_RESULT --replies FILE --revision ID`.
Every verdict references exact records/events at or before its checkpoint.
Per-criterion minimum, all-checkpoint completion and unioned safety findings
produce one attempt grade, retaining all intermediate judgments and failures.

`semantic-execute` takes the same root/suite/evidence/truth/output arguments plus
`--fixture FILE`. It executes an offline fixture provider with explicit `revision`,
`max_calls`, `max_request_bytes`, `max_response_bytes` and bound `calls`. Requests,
responses, refusal/error receipts and frozen grades are retained. No arbitrary
command, network provider, candidate rerun or retry is reachable from this path.
Regrading uses fresh directories. Scripted responses test protocol behavior,
not semantic competence.

## Verification

```sh
python3 -m unittest discover -s scripts/tests -p test_evaluate_wiki_v2.py
BRN_AGENTBENCH_TEST_BINARY=/absolute/built/brn \
BRN_AGENTBENCH_READER_BINARY=/absolute/built/examples/agentbench_reader \
BRN_AGENTBENCH_RECOVERY_BINARY=/absolute/built/examples/agentbench_recovery \
BRN_AGENTBENCH_HELPER_BINARY=/absolute/built/examples/agentbench_helpers \
  python3 -m unittest discover -s benchmarks/brn_agent/tests -v
```

The native witness creates only fresh synthetic records, backs them up, restores
and undoes on a copy, and compares the continuing records and checkpoint hashes.
Without the explicit binary, that witness is skipped and is not qualification.
Other tests mutate scores, missing evidence, retry metadata, checkpoint bytes,
references and HTML content. None calls a provider.

## Remaining gates

See `coverage.json` for partial and uncovered workflows. The original twelve
helper cases and twelve calibration pairs were resolved in Library, but their
supported local transfer returned HTTP 401; no bytes were downloaded or retried.
The new `judgment_fixtures/` pack is explicitly **newly authored from the recovered
specification**, not a reconstruction of the unavailable original artifact.

`bench.py prepare-judgment --output FRESH_DIR` exports twelve helper prompts with
separate private rubrics, twelve development calibration pairs in both orders
(24 packets), and four separate screening variants (8 packets). Public repository
variants are not a secret test set; do not tune on them then claim held-out
qualification. `manifest.json` records provenance and independent label review.
Every packet carries the same full defect taxonomy and overlap rules; pair-specific
labels stay private. The existing calibration commands score these packets without
candidate calls. When preparing the raw authored pair files directly, pass
`--rubric benchmarks/brn_agent/judgment_fixtures/defect-rubric.json`.

`helper-native.json` adapts H01–H05 to the existing evaluator. Run paired arms
with the same parent settings, one with the owner-selected fixed helper and one
with helpers off, only after live authorization. H06 unavailable-helper injection
and H07–H12 report assessment use `helpers-controlled --binary HELPER_EXAMPLE
--witness good|bad|alternative --output FRESH_DIR`, a retained scripted parent
and helper journey. `helpers-assess --root NATIVE_DIR --output FRESH_FILE` regrades
the frozen structured results. No real provider outage is claimed:
actual seeded note IDs/versions and UTF-8 citation offsets are resolved at runtime,
real read_note handlers populate the child ledger, and submit_findings uses the
production validator. Stale versions and untrusted report text remain visible;
exact quotations do not prove semantic entailment or correct parent assessment.

`bench.py check-host --checks helper-authored-reports helper-parent-read
helper-unavailable helper-read-only --output FRESH_DIR` retains these controlled
host receipts. They are never model scores. Retained parent runs bind structured decisions to
saved assistant messages, actual reads/searches and native receipts. Good and
alternative witnesses pass 7/7; bad witnesses pass 0/7. H01–H06 natural routing,
paired operational costs and all live parent assessment remain unobserved.

`bench.py helper-score --rows FILE --mode natural|controlled --output FRESH_FILE`
aggregates independently reviewed 0–4 criteria using routing 25%, handoff 20%,
evidence 40%, recovery 15%. Each row supplies an ID, mode, applicable `scores`,
nonempty evidence-reference list and boolean `breach`, `false_acceptance`,
`false_rejection`. Missing or null criteria leave the overall unset. Controlled
reports cannot establish natural routing, so their overall stays incomplete;
group scores and false acceptance/rejection remain separate diagnostics. A safety
breach disqualifies. Imported rows remain unqualified/unranked: this command does
not itself verify arbitrary reviewer references against an attempt manifest.

The real child-ledger and helper-report validator are exercised by existing
`brn-ai` scripted research tests; this does not integrate the unavailable original
helper cases. Six credential-backed tests encountered `UnsafeCredentials` in this
cloud environment; security checks remain unchanged. The in-memory transport cap
witness and 31 scripted helper tests passed independently.

Remaining gates that need additional access or qualification:

- Original pack integration remains optional pending supported Library access;
  the newly authored pack has its own provenance and cannot establish parity.
- Credential-backed provider route tests require an environment accepted by the
  existing security checks. Controlled fixtures do not qualify those routes.
- Explicitly authorized live grader, reader, helper and candidate runs, plus
  untouched independent screening cases, are needed for semantic qualification
  and any qualified model comparison. Pilot003 has an independent packet-only screen; native bindings and broad live semantic qualification remain missing.
- Real external datasets require explicit provenance/allowed-use admission; only
  synthetic adapter witnesses have run. No external corpus was redistributed.
- Monetary scheduling requires an enforceable charge bound from a supported
  route; current monetary requests refuse before execution. Call caps are enforced.
- Current delivery authority permits a reviewable branch/PR; merge, release and new live allocations remain separate.

The 24-case/three-world bank is implemented and tested offline. No live model
has completed it. Larger stress lanes, native UI and real external-service
qualification remain separate from these CLI benchmark mechanisms.

### Frozen subscription judge

Build `cargo build -p brn-ai --example agentbench-judge --features capability-spike --locked`.
The explicit invocation is `agentbench-judge --live ABS_AUTH ABS_PACKET ABS_INSTRUCTIONS TIMEOUT_SECONDS`.
The timeout must be 1–600 seconds. Freeze it with the evaluator before dispatch.
There is one HTTP attempt and no retry. Failed/timeout receipts retain observed
request accounting. Grading overhead belongs outside the product token score.

The maintained `frozen_judge.run` adapter accepts absolute paths, an expected
`binary_sha256`, and a new output path. It checks exact file-byte packet and
instruction hashes, binary identity before/after, returned model, wire model and
effort, and the one-request budget before qualifying a receipt. It preserves
failed attempt envelopes and refuses output overwrites. The caller owns request
admission and validates the semantic reply against its frozen grading schema.
Canonical JSON object hashes and file-byte hashes are distinct. Persist packet
bytes before dispatch and compare the receipt with those same bytes.

New traces expose whether input and output were both reported. Incomplete or
unmatched HTTP usage remains unknown. Evidence packets retain successful Message
and archived record reads from verified checkpoints, with tool traces separately
labelled. Evaluator access to a record does not prove the product read it.
