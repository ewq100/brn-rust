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
permission for a live run. This proof does not implement spend caps or certify
wire-observed model/effort identity, so it is not ready for a controlled model
screen.

`proof.py` deterministically renders eight focused cases and one eight-event
Cedar renovation world from a small truth ledger. `fixture.json` contains native
seeds and owner events; `gold.json` stays outside candidate state. The Rust seeder
uses checked host operations. Historical events do not grant new authority.
Gold labels and semantic rubrics are drafts requiring independent validation.
The fixed-reader packet builder and semantic packet builder are interfaces only;
neither invokes a model nor qualifies a grader.

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

Inspect integration is deferred until a parity probe proves it neither reroutes
candidate requests nor duplicates scheduling. External dataset adapters and
license/version admission are not implemented; no third-party corpus was
redistributed. Missing host-event adapters include controlled edits/rejections,
stale-input injection, interruption and resumption. Full semantic/link/retention
mutation grading, fixed-reader execution, calibrated independent grading,
verified wire identity and scheduling cost caps remain required before model
selection. The broader 24-case/three-world suite is not implemented.
