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
Reader questions/results remain independently supplied diagnostics.

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
transformation, not native execution or an official dataset score. It whitelists
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

## Verification

```sh
python3 -m unittest discover -s scripts/tests -p test_evaluate_wiki_v2.py
BRN_AGENTBENCH_TEST_BINARY=/absolute/built/brn \
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
supported local transfer returned HTTP 401. They have not been reauthored or
claimed as integrated. Controlled helper reports still need to traverse the real
child read ledger and validator; live helpers remain separately authorized.

The real child-ledger and helper-report validator are exercised by existing
`brn-ai` scripted research tests; this does not integrate the unavailable original
helper cases. Six credential-backed tests encountered `UnsafeCredentials` in this
cloud environment; security checks remain unchanged. The in-memory transport cap
witness and 31 scripted helper tests passed independently.

Remaining implementation/qualification: original pack integration, controlled
mid-request interruption/resumption scenarios, protected-edit and skills coverage,
semantic grader qualification with independently reviewed held-out mutations,
fixed-reader execution, native external-history intake mapping, and enforceable
monetary bounds if a supported route exposes them. MemoryAgentBench conversion and
the broader 24-case/three-world suite remain deferred. No live model or reader has
run, and no model ranking is asserted.
