# Knowledge retention candidate: measured, not adopted

Keep the original baseline. Candidate 1 was implemented, compiled and measured,
but it did not establish the frozen retained-information gain. Confirmation is
incomplete. This report preserves an actual improvement attempt; it does not
recommend a production guide change.

## One focused change

The first hypothesis was that checking each supported fact, quantity, unit,
condition, exception and unresolved point before replacing a maintained account
would reduce W03/W05 information loss. The existing visible development patch
adds one sentence to `agent/skills/maintain-notes/SKILL.md`. Its provenance predates
this cycle and includes the public W03/W05 cases. No holdout informed the patch.

The separate candidate checkout is `work/agent-d-254-improvement`, source commit
`45610287`; the exact [patch](evidence/candidate-1.patch) is retained here. Its CLI
SHA-256 is `49e8b1f6ef114c8c521ce3fb8c53e9c8640e6704b80443bc7de981c9a103f317`.
The baseline remains `35f686fb8e6b1539477e0fe5439c2f6f5c9a3e891d17e63b76db5a30dd365f44`.
Both use the same combined CLI/seeder feature closure (CLI/app `[]`, brn-ai
`[scripted-runs]`), actual Codex transport, Sol medium, helpers/conversion off,
6 HTTP/event and 60 seconds/command. The only runtime-guidance difference is the
retention sentence. Trace guide digests differ, and maintain-notes was actually
loaded in all eight development histories. The accepted baseline is preserved;
the publication branch contains the experimental patch as evidence only.

## Development observations

Fresh equivalent stores, two paired repetitions, rotated order, baseline A and
candidate B. Original Phase 1 outputs were not substituted for Phase 2 baseline.

| Observation | Original baseline | Candidate 1 |
| --- | ---: | ---: |
| Candidate events completed | 11/12 | 12/12 |
| Mechanical checks | four histories pass | four histories pass |
| Observed HTTP attempts | 57 | 55 |
| Reported input tokens | 625,329 | 611,325 |
| Reported output tokens | 9,926 | 9,446 |
| Cached input tokens (subset of input) | 522,368 | 508,672 |
| W03 seconds/history, repetitions 1/2 | 70.261 /68.220 | 69.368 /68.015 |
| W05 seconds/history, repetitions 1/2 | 86.734 /76.144 | 77.684 /64.367 |
| Fixed-reader answer quality | unmeasured | unmeasured |
| Frozen saved-state quality dimensions | unmeasured | unmeasured |

Baseline W05 repetition 2 ended its second event with ToolLimitReached. Its
mechanical checks still passed; this is not reliable completion. The candidate
completed both W05 repetitions. These observations do not establish retention,
grounding or a quality gain. [Per-run evidence and hashes](evidence/phase2-development-summary.json)
preserve terminals, reported usage and actual saved records. One candidate was
funded; no additional tuning was attempted or silently substituted.

## Unblinded saved-state inspection

After the final Council, the lead inspected all eight exported final development
articles. Both arms retained the visible W03 loan period, deposit/refund condition,
separate battery labels and conditional one-time extension. Both retained W05's
recorded quantities, hold conditions, saline method restriction, blank gate,
cloudy-sample duplicates, refrigerated-sample prerequisite and provisional
validation status. The baseline W05 repetition 2 article had already saved its
update before its final ToolLimitReached terminal; its completed saved content
explains why the mechanical checks passed despite incomplete agent execution.

The [W03 baseline snapshot](evidence/phase2-development/runs/W03-0/A-r1/checkpoints/4/records.json)
and [interrupted W05 baseline snapshot](evidence/phase2-development/runs/W05-0/A-r2/checkpoints/2/records.json)
are examples; the manifest binds all eight histories. This is a post-run,
unblinded lead observation, outside the frozen independent grading process and
not reviewed by Council. It assigns no rubric score or saturation result and
does not establish full grounding, safety, reader usefulness or a retention gain.
The independent dimensions above therefore remain unmeasured.

## Fresh confirmation and stop

An independent evaluation-only owner authored and froze a distinct two-history
Phase 2 pack before candidate execution, without tuning conversation, patch or
Council access. Configurations and binaries were fixed before confirmation.
The contents stayed outside the tuning lead and Council's context. This was
procedural withholding on the same MacBook; no technical access isolation exists.
The permitted conclusion would have been limited engineering evidence only.

A nested 10-minute coordinator window was introduced during development and
fixed before any fresh results; it was absent from the initial frozen plan.
It included preflight and left only 275 seconds for native execution. Worst-case
candidate plus reader commands require 20 minutes (8 x 60 + 8 x 90 seconds), before
grading, so they could not all fit that window. The nested window, not the original
cycle deadline, limited execution: about 61 minutes of the whole cycle remained
at the stop. T3 coordination itself finished in 10 minutes 18 seconds, exceeding
the requested soft window; native cleanup had already completed. No elapsed limit
was silently extended to resume the interrupted measurement.

Confirmation reached that coordinator time boundary before completing the pack:
seven of eight candidate attempts started; two Completed, four ToolLimitReached,
and one in-flight outcome remained uncertain. Six observed terminals had verified
Sol-medium identity. Six mechanical protection checks passed and one is unknown.
These are distinct fresh histories and only a partial set of outcomes; development
completion rates do not transfer to them. Case difficulty may differ, but this
cycle cannot attribute the gap to a cause.
36 HTTP attempts are observed, total unknown, bounded by 42 nonrefundable
reservations. Cleanup had no errors and left no owned live sessions. No replay,
refund or further BRN dispatch occurred. There were zero readers and zero graders.

The [aggregate](evidence/phase2-fresh-aggregate.json),
[artifact manifest](evidence/phase2-fresh-artifact-manifest.json) and
[command reference](evidence/phase2-fresh-command-reference.json) preserve this
failure without exposing case contents. Complete evidence remains at their
recorded local paths for later review. These cases have been used for confirmation
and must not serve as fresh cases in another cycle.

The lead changed the planned order: after development candidate runs, fresh
confirmation ran before development readers and broad regressions. The reason was
to prioritize protected fresh capacity after earlier grading faults consumed time.
This was an execution deviation: development acceptance had not been established,
and it consumed the fresh cases on an unqualified candidate. Protected broad
W01-W06/F04/F07/F16 regressions and development readers were consequently still
pending when the uncertain outcome stopped further dispatch. Unspent capacity is
not a refund or a new allocation. Retained-information gain, baseline saturation, repeat-range
threshold, no-decline dimensions and broader/fresh acceptance remain unknown.
The frozen rule therefore rejects adoption. Baseline remains accepted and final
quality confirmation remains pending a separately authorized fresh cycle.

## Checks and limits

Source-bound candidate combined build passed after forced local-crate rebuild;
original and candidate immutable binaries, source/patch/configuration hashes and
usage survive in task artifacts. Existing baseline qualification includes native
reader, reservation, deadline, runtime HTTP and no-tool judge wire fixtures.
Initial post-run native AgentBench: 96 passed, one optional Inspect skip. After
final-review corrections, 99 of 100 tests passed with the same optional skip.
The earlier no-binary run (86 passed, 11 skips) remains preserved. Grader Clippy
passed with capability-spike/scripted-runs after two command-only target/feature
errors. The supervisor exit-status correction and additional containment tests
were offline only; exact executed source remains archived. No model call replaced
a missing measurement. [Final review](review-followups.md) and [accounting](accounting.md)
are in the comparison layer.
No default-model change, desktop qualification, owner acceptance, merge or release.
