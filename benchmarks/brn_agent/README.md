# BRN-AgentBench offline proof

This is a partial implementation of the recovered October 15 design. It extends
`scripts/evaluate_wiki_v2.py`; candidate calls still use the normal BRN CLI,
provider route and tools. It adds no provider gateway or production dependency.
No model has been evaluated, no judge has been calibrated, and no model ranking
is qualified by this change.

## Prepare and check

From the repository root:

```sh
python3 benchmarks/brn_agent/bench.py prepare --output /absolute/fresh/planning
python3 scripts/evaluate_wiki_v2.py \
  --fixture /absolute/fresh/planning/fixture.json \
  --root /absolute/fresh/experiment \
  --arm A=/absolute/built/brn \
  --seeder /absolute/built/wiki_eval_seed
```

The second command seeds, copies, opens and checks all nine synthetic cases; it
makes no model calls. Build the seeder with `cargo build --locked -p
brn-threads-app --example wiki_eval_seed`. The normal evaluator remains the only
scheduler and retry owner. Its existing `--live` switch requires separately
approved model/account/spending choices; do not interpret these commands as
permission for a live run. Checkpoint-enabled live cases require both
`--model-call-limit` (per event) and `--max-total-model-calls` (experiment).
The runtime atomically caps actual parent/helper/summary HTTP attempts, including
retries, before dispatch. Persistent scheduler reservations include unfinished
and in-flight work and never refund uncertain calls. Capped useful intake requires
AI conversion off. Monetary `--max-spend` requests fail closed before work starts:
subscription routes provide no enforceable charge bound; estimates cannot supply
one. This remains a call-bounded runner, not a qualified monetary-bounded screen.

```sh
python3 benchmarks/brn_agent/bench.py plan --suite /absolute/planning/suite.json \
  --max-calls 32 --calls-per-event 2
```

Trace identity records bounded model/effort fields from actual request bodies and
returned model IDs when available, without headers or prompt bodies. Requested
configuration alone never qualifies runtime identity. Unsupported identities and
missing observations keep results unranked.

`proof.py` deterministically renders eight focused cases and one eight-event
Cedar renovation world from a small truth ledger. `fixture.json` contains native
seeds and owner events; `gold.json` stays outside candidate state. The Rust seeder
uses checked host operations. Historical events do not grant new authority.
Gold labels and semantic rubrics are drafts requiring independent validation.
The fixed-reader and semantic packet builders export frozen, anonymous data;
neither invokes a model nor qualifies a grader. `bench.py packet --checkpoint PATH
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
remain readable. Outputs are `report.json`, `scores.csv`, and escaped static
`report.html`. Reports may include synthetic record text; do not share raw
backups or use private owner workspaces.

Scores average criteria, repetitions, scenarios and then the eight frozen
category weights. Missing mandatory slots/categories make overall quality null;
they are never silently dropped. A complete numerical score is still unranked
while coverage, grader qualification or model/effort identity is unverified.
Confirmed safety breaches remain disqualifying even after a replacement.
Completion and all-attempt operational counts remain separate. Token/cache
measurements are nullable; output tokens are not inflated by separately adding
reasoning tokens. Unknown cost stays null. The legacy evaluator's historical
credit estimates are not imported as measured cost.

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
Normal-product mode uses general visibility. Both use fixed supplied reads,
not model-selected retrieval. The only scripted tool is native `read_note`;
pre-existing clone records and the original checkpoint must remain unchanged.
Each mode uses its own copy. Credential/helper/connector-configured snapshots
are refused; the controlled probe never loads credentials or contacts providers.
Its supplied answer and synthetic usage are explicitly unqualified diagnostics.

`semantic-prepare --root ROOT --suite SUITE --evidence INDEX --output FRESH_DIR`
exports anonymous final-captured-state packets separately from private bindings.
It supplies checkpoint-scoped gold, the fixed 0–4 rubric, and actual event evidence.
`semantic-controlled` takes the same arguments plus `--prepared PRIVATE_RESULT
--replies FILE --revision ID`; replies bind exact packet hashes and must contain
all mandatory criteria, references, completion and safety flags, or an explicit
grader error. Missing replies and grader outages remain incomplete. Imported
`grades.json` can feed the normal report command; regrading never invokes a
candidate. This validates the grading protocol, not semantic competence. Earlier
checkpoint packets remain available through `packet` for temporal diagnostics.

## Verification

```sh
python3 -m unittest discover -s scripts/tests -p test_evaluate_wiki_v2.py
BRN_AGENTBENCH_TEST_BINARY=/absolute/built/brn \
BRN_AGENTBENCH_READER_BINARY=/absolute/built/examples/agentbench_reader \
BRN_AGENTBENCH_RECOVERY_BINARY=/absolute/built/examples/agentbench_recovery \
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
is a separate existing native scripted contract, not a fake live outage. H07–H12
controlled reports are consumed directly by the native scripted test producer:
actual seeded note IDs/versions and UTF-8 citation offsets are resolved at runtime,
real read_note handlers populate the child ledger, and submit_findings uses the
production validator. Stale versions and untrusted report text remain visible;
exact quotations do not prove semantic entailment or correct parent assessment.

`bench.py check-host --checks helper-authored-reports helper-parent-read
helper-unavailable helper-read-only --output FRESH_DIR` retains these controlled
host receipts. They are never model scores. H01–H06 natural routing, paired
operational costs and H07–H12 live parent assessment remain unobserved.

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
  and any model comparison. No live quality result is present.
- Real external datasets require explicit provenance/allowed-use admission; only
  synthetic adapter witnesses have run. No external corpus was redistributed.
- Monetary scheduling requires an enforceable charge bound from a supported
  route; current monetary requests refuse before execution. Call caps are enforced.
- Publication requires authorization. This branch remains local.

The later 24-case/three-world expansion and native UI qualification are separate
plan phases, not claims made by this initial offline implementation.
