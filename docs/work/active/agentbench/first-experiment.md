# First bounded wiki experiment — prepared, not authorized to dispatch

Hypothesis: one explicit comparison of useful existing facts against incoming
facts before replacing an article reduces lost qualifiers during overlapping-source
consolidation. Target: `retained_information` on W03 and W05. The
[exact candidate patch](../../../../benchmarks/brn_agent/experiments/wiki-retention-v1.patch)
is prepared and passes `git apply --check`; it is not applied in the delivery branch. A separate experimental checkout contains only this patch for build B. No production behavior was changed. Preserve baseline A and its build; B is built separately from the same app revision plus only this patch. Do not tune on evaluator files.

Candidate: Codex `gpt-6.1-sol`, medium, helpers off, tool rounds 8,
12 actual shared HTTP attempts per event, 60 seconds per candidate command.
Reader: fixed [Sol medium configuration](../../../../benchmarks/brn_agent/wiki_challenge/reader.json),
8 tool rounds, 12 attempts, 90 seconds. Grader: independently owned
[Sol high configuration](../../../../benchmarks/brn_agent/wiki_challenge/reviewer.json),
one bounded response per packet, 45 seconds, no tools except frozen packet input.
Unknown monetary cost stays null. These allocations are proposals; no new live
budget or current model availability has been supplied. Expired pilot release
comments cannot authorize them. No alternate model/account fallback.

| Round-one allocation | Invocations/observations | Maximum HTTP reservations |
|---|---:|---:|
| W03/W05, A and B, 2 repetitions | 24 candidate events | 288 |
| Existing F07/F16, A and B, 1 repetition | 4 candidate + 6 deterministic owner events | 120 conservative slots (actual model-event cap 48) |
| Wiki-only readers: 2 questions × 2 arms × 2 repetitions | 8 | 96 |
| Normal-product reader contrast, W03 final state, one per arm | 2 | 24 |
| Independent grader: 28 candidate checkpoints + 10 reader answers | 38 | 38 |
| **Whole first round** | **28 candidate, 10 reader, 38 grader; 6 owner events** | **566** |

Conservative dry planning counts owner events too; runtime reservations apply only
to actual model invocations. Do not refund or reuse unused slots. Failed/uncertain
calls consume their reservation. No retry, automatic resume or extra adjudication
allocation. A grader protocol fixture does not consume a real grader allocation
or count as observed judgment. All times/calls/tokens remain separate by role.

Hard wall limit: **90 minutes total**, including setup, both runner commands,
readers and review. Stop new dispatch at 89m30s; use the remaining 30s for owned
process cleanup. Reuse `pilots/terra_sol_v3/deadline_guard.py`'s existing `Guard`
under one enclosing job and its inherited session ledger. Do not invoke any old
pilot launcher, release, claim or plan. Do not start live work until that enclosing
receipt, current read-only model availability and a newly approved allocation are
bound. A handoff does not reset its clock. This document prepares commands, not a
new scheduler or authorized live launcher.

## Offline preparation

From the reviewable branch, use new absolute directories for every command.
The following are exact commands; shell variables bind explicitly built artifacts
and fresh local paths. They do not inspect credentials or call a model.

```sh
export AB_WORK=/private/tmp/brn-wiki-round1-preparation
mkdir "$AB_WORK"
python3 benchmarks/brn_agent/bench.py prepare --suite-kind wiki-challenge \
  --cases W03,W05 --repetitions 2 --output "$AB_WORK/target"
python3 benchmarks/brn_agent/bench.py prepare --cases F07,F16 \
  --output "$AB_WORK/regression"
python3 benchmarks/brn_agent/bench.py plan --suite "$AB_WORK/target/suite.json" \
  --configurations 2 --calls-per-event 12 --max-calls 288
python3 benchmarks/brn_agent/bench.py plan --suite "$AB_WORK/regression/suite.json" \
  --configurations 2 --calls-per-event 12 --max-calls 120
git apply --check benchmarks/brn_agent/experiments/wiki-retention-v1.patch
```

The focused suite predeclares its subset/repetitions while preserving the original
fixture/truth bytes. Its result describes only that subset. F07/F16 cannot yield
an existing overall score: missing categories remain null. Keep a full unchanged
baseline preparation for later regression qualification.

Create a local freeze spec with exactly `evaluation_files`, `candidate_files`,
`labels`, `settings`, `isolation`. Evaluation files include both prepared
fixture/suite/gold triples and acceptance rules (this document); the freeze tool
also hashes all benchmark Python, reader/grader/rubric JSON and native runner
code automatically. Candidate files include A/B CLI binaries, seeder, helper
binaries, original guide, candidate guide/patch, Cargo.lock, and a source/build
receipt. Hash retrieval asset files explicitly, never just a directory name.
Labels require `app_source_revision`, `benchmark_revision`,
`build_source_binding`; use `unverified` if no source-bound build receipt exists.
Settings require separate `candidate`, `reader`, `grader`, `retrieval` objects,
including search engine/features, asset digests and scope settings. Declare
`isolation: visible-screening`. No credential file or account identity belongs
in the spec. Hashes record identity, not source/build or isolation attestation.

```sh
python3 benchmarks/brn_agent/bench.py freeze --spec "$AB_WORK/freeze-spec.json" \
  --output "$AB_WORK/frozen-inputs.json"
python3 benchmarks/brn_agent/bench.py verify-freeze --spec "$AB_WORK/freeze-spec.json" \
  --receipt "$AB_WORK/frozen-inputs.json"
```

## Execution after a new allocation and enclosing deadline are approved

Bind `AB_A`, `AB_B`, `AB_SEEDER`, `AB_AUTH` to explicit existing
absolute paths; auth is passed to normal BRN configure, never copied/read/logged.
B is a separate built app with the proposed guide patch; evaluator checkout stays
fixed. The prepared default-feature CLI builds use native BM25 retrieval with no embedding assets. Both arms and the fixed reader use that same configuration. Dense/native-retrieval feature qualification is outside this first screen; changing it requires a new frozen experiment.

```sh
python3 scripts/evaluate_wiki_v2.py --live \
  --fixture "$AB_WORK/target/fixture.json" --truth "$AB_WORK/target/gold.json" \
  --cases W03,W05 --repetitions 2 --root "$AB_WORK/target-run" \
  --arm "A=$AB_A" --arm "B=$AB_B" \
  --seeder "$AB_SEEDER" --provider Codex --model gpt-6.1-sol --effort medium \
  --auth-file "$AB_AUTH" --model-call-limit 12 --max-total-model-calls 288 --timeout 60
python3 scripts/evaluate_wiki_v2.py --live \
  --fixture "$AB_WORK/regression/fixture.json" --truth "$AB_WORK/regression/gold.json" \
  --cases F07,F16 --root "$AB_WORK/regression-run" \
  --arm "B=$AB_B" --arm "A=$AB_A" \
  --seeder "$AB_SEEDER" --provider Codex --model gpt-6.1-sol --effort medium \
  --auth-file "$AB_AUTH" --model-call-limit 12 --max-total-model-calls 120 --timeout 60
```

The existing runner rotates A/B then B/A across target repetitions. Baseline A's
two independent runs measure variation; show both scores and their range, not just
a mean. Regression starts B/A. Do not restart failed work. Stop the enclosing job
on credential, identity, authority/safety, runtime/transport failure, deadline,
ledger uncertainty or capacity exhaustion. Preserve every directory and receipt.

For each root, collect and export all anonymous checkpoints using the existing
commands (replace the target paths with the regression paths for that root):

```sh
python3 benchmarks/brn_agent/bench.py collect --root "$AB_WORK/target-run" \
  --suite "$AB_WORK/target/suite.json"
python3 benchmarks/brn_agent/bench.py semantic-prepare --root "$AB_WORK/target-run" \
  --suite "$AB_WORK/target/suite.json" --truth "$AB_WORK/target/gold.json" \
  --evidence "$AB_WORK/target-run/agentbench-evidence.json" --output "$AB_WORK/target-packets"
```

Independent evaluation ownership selects maintained article IDs/versions from each
final checkpoint, excluding raw sources. Use a fresh clone per question/mode.
Bind `AB_CHECKPOINT` and `AB_ARTICLES` to that checkpoint and exact allowlist; store
reader outputs below the candidate experiment root so report references stay local.
This is the W03 reader command (repeat the predeclared W05 question separately):

```sh
python3 benchmarks/brn_agent/bench.py reader-measured --live \
  --checkpoint "$AB_CHECKPOINT" --wiki-records "$AB_ARTICLES" \
  --question 'Can a volunteer extend a drill loan when someone is waiting?' \
  --mode wiki-only --binary "$AB_A" --auth-file "$AB_AUTH" \
  --output "$AB_WORK/target-run/readers/W03-A-r1-wiki"
```

Use the same reader binary A for both candidate arms. For the two diagnostic
normal-product calls, change only mode/output; do not combine their scores with
wiki-only usefulness. No controlled answer argument exists on this path. Successful
retrieval and a real answer do not establish answer correctness: the independent
grader must assess it against the frozen question expectations.

Use `semantic-reviewed` with the same prepare arguments, `--prepared` pointing to
`private-result.json`, `--replies` to independently obtained bound candidate replies,
`--revision wiki-reviewer-v1`, and `--reviewer-config` to the packaged reviewer JSON.
Candidate checkpoint replies use null for downstream usefulness; they cannot grade
reader answers they have not received. The importer records independently supplied
review origin, not attested external runtime identity.

Prepare a private `reader-index.json` list. Each row has exactly `attempt` (indexed
relative native result path), `question_id` (gold ID, for example W03-Q1), and
`measurement: {path, sha256}` (relative real-reader receipt and exact hash). Use a
separate invocation for every attempt/question. The adapter verifies the actual
native owner Message/Run prompt, current fixed reader configuration, final candidate
checkpoint, article scope and retrieval trace; relabelling a receipt cannot fill a
missing question. Reused native invocation IDs are refused.

```sh
python3 benchmarks/brn_agent/bench.py reader-review-prepare --root "$AB_WORK/target-run" \
  --suite "$AB_WORK/target/suite.json" --truth "$AB_WORK/target/gold.json" \
  --evidence "$AB_WORK/target-run/agentbench-evidence.json" --index "$AB_WORK/reader-index.json" \
  --output "$AB_WORK/reader-packets"
```

Give the fixed independent reviewer only the anonymous reader packets, containing
the frozen question/expectation, actual answer, supporting article versions and
native receipt digest. Replies contain `id`, `packet_sha256`, integer `score` 0–4,
`explanation`, `evidence: [{reader_sha256: EXACT_DIGEST}]`, and `breaches`; an outage
uses `id`, `packet_sha256`, `error`. Do not supply scripted judgments as observations.

```sh
python3 benchmarks/brn_agent/bench.py reader-review-import --root "$AB_WORK/target-run" \
  --suite "$AB_WORK/target/suite.json" --truth "$AB_WORK/target/gold.json" \
  --evidence "$AB_WORK/target-run/agentbench-evidence.json" --index "$AB_WORK/reader-index.json" \
  --prepared "$AB_WORK/reader-packets/private-result.json" --replies "$AB_WORK/reader-replies.json" \
  --revision wiki-reviewer-v1 --output "$AB_WORK/reviewed-readers"
```

Report re-derives this entire bound reader-review artifact before merging the
minimum across all predeclared final questions into usefulness. Candidate and reader
must use the identical frozen reviewer revision. Missing questions/judgments stay
unmeasured; confirmed safety findings survive incomplete reviews. Manual receipt
attachments cannot manufacture a usefulness score. Normal-product and supplied
scripted answers cannot fill this wiki-only requirement. Preserve every raw reply,
failure and the previous report directories.

```sh
python3 benchmarks/brn_agent/bench.py report --suite "$AB_WORK/target/suite.json" \
  --truth "$AB_WORK/target/gold.json" --evidence "$AB_WORK/target-run/agentbench-evidence.json" \
  --grades "$AB_WORK/reviewed-grades.json" --reader-grades "$AB_WORK/reviewed-readers/private-result.json" \
  --configuration A \
  --freeze-spec "$AB_WORK/freeze-spec.json" --freeze-receipt "$AB_WORK/frozen-inputs.json" \
  --output "$AB_WORK/A-report-v2"
```

Run B report to a different directory. Input grades must contain only the requested
configuration. Use `compare` on matching reports to show paired deltas. Two worlds
are insufficient for an interval. Keep quality and efficiency separate; unknown
usage/cost remains null. Never rerun candidates to repair report format or grading.

## Selection and remaining rounds

Round one is screening only. A promising result requires target retained-information
mean to improve by at least 6.25 points (0–100 scale), both paired repetitions to
favor B, and improvement greater than the observed A repetition range. Correctness,
source support, temporal handling and organization must not decline on either target.
F07/F16 must have no new material defect or safety/approval finding. Report all
exceptions; do not trade quality for latency or call count. Saturated baseline or
missing measurements means stop without accepting a change.

Maximum **three rounds total**, no automatic next round:

1. Above screen: 566 reservations / 90 minutes. No model allocation yet.
2. Only if promising, same candidate on fresh full six-case histories, 2 paired
   repetitions plus existing regressions and all wiki-only questions: ceiling
   1500 reservations / 210 minutes. Requires a new allocation and unchanged
   evaluator/acceptance lock. S01/S02 are additional **visible screening** only;
   separately owned genuinely unseen histories remain required for acceptance.
3. Only to resolve a concrete disagreement or obtain a third paired repeat on
   W03/W05: ceiling 220 reservations / 45 minutes, fresh outputs and unchanged
   hypothesis. No second parameter search or expensive multi-model grading without
   a concrete unresolved judgment disagreement.

Accept only after target improvement, important baseline behavior, safety/approval
rules and fresh independently owned evaluation all pass. If genuine unseen
separation remains unavailable, preserve the best **verified screening version**
without claiming accepted improvement. Keep A available; never overwrite the best
verified build or outputs. Stop after the cap, any safety breach, repeated semantic
regression, no target improvement, uncertain effects, missing authority or owner stop.

## Prepared local artifacts

The source-bound [build receipt](bounded-evidence/builds.json) records A at
`/tmp/brn-agentbench-bounded-build/debug/brn`, B at
`/tmp/brn-agentbench-retention-build/debug/brn`, and the current seeder at
`/tmp/brn-agentbench-bounded-build/debug/examples/wiki_eval_seed`.
Fresh target/regression plans are retained at `/tmp/brn-round1-final-target` and
`/tmp/brn-round1-final-regression`. The local input spec and frozen receipt are
`/tmp/brn-round1-final-freeze-spec.json` and
`/tmp/brn-round1-final-frozen-inputs.json`; the sanitized receipt is retained in
`bounded-evidence/frozen-inputs.json`. Verify them again before any authorized
run. These disposable paths are this machine's artifacts, not repository binaries
or live authorization. B's only source difference is the recorded guide patch.
