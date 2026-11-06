# Terra and Sol: bounded BRN comparison

Conclusion: insufficient evidence to replace Sol as BRN's main agent. The later
Phase 2 used provisional Sol medium. Terra was effective on this visible sample,
with fewer requests and lower reported time/token totals, but the saved-state
quality gate is unmeasured and independent unseen comparison was not admitted. No default-model change is proposed.

## Observed results

One pinned CLI, Codex subscription, medium effort, helpers/conversion off,
6 HTTP attempts/event and 60 seconds/command. Fresh synthetic workspaces,
two paired repetitions, rotated order. W03/W05 cover creation, consolidation,
updates, duplicate handling and knowledge retention. F01/F04/F07/F16 add creation,
writing, privacy scope and stale-save recovery. Each history's elapsed value below
is the sum of reported native model-operation times, excluding setup and readers.

| Visible sample | Sol | Terra |
| --- | ---: | ---: |
| Completed candidate events | 16/20 | 20/20 |
| W03 median seconds/history | 79.377 | 57.724 |
| W03 median reported input+output tokens/history | 181,617.5 | 147,884.5 |
| W05 median seconds/history | 83.279 | 46.946 |
| W05 median reported input+output tokens/history | 140,118 | 106,586 |
| W03/W05 independently graded reader answers | four 4/4 | four 4/4 |
| Frozen saved-state semantic dimensions | unknown | unknown |

Sol reached the tool budget on both F01 creation repetitions and both F04 writing
repetitions. F01 has no applicable automatic checks, so its mechanical pass is
null. F04 failed the required note-update check in both Sol repetitions. Terra
completed these events and passed F04's mechanical check. Both models passed the
F07 scope and F16 recovery checks. Automatic checks establish their named
properties; they do not establish answer completeness, grounding or prose quality.

All eight fixed readers used the original Sol-medium reader binary on fresh
checkpoint clones, with native retrieval and a versioned article allowlist that
excluded protected raw sources. A separate no-tool Sol-high invocation graded
each anonymous answer against frozen expected answers. All eight returned valid
hash-bound scores of 4/4. Same-family grading bias remains a limitation.


The paired efficiency observations below show both repetitions; Terra was lower
on both measures in each W03/W05 pair. This does not resolve the missing quality
gate or unseen comparison.

| Case/repetition | Sol seconds | Terra seconds | Sol input+output | Terra input+output |
| --- | ---: | ---: | ---: | ---: |
| W03/1 | 73.247 | 57.544 | 181,279 | 147,717 |
| W03/2 | 85.506 | 57.904 | 181,956 | 148,052 |
| W05/1 | 83.843 | 43.886 | 146,464 | 106,443 |
| W05/2 | 82.715 | 50.006 | 133,772 | 106,729 |

All eight reader grades are at the 4/4 ceiling. Two visible reader questions
cannot distinguish the models' full knowledge quality. The planned four-packet
blind consistency check was not run; no grading consistency result is claimed.

## Missing evidence and failures

Three saved-state grader attempts failed within the finite allocation: an outer
deadline initially allowed only 30 seconds, then two requests hit the unchanged
45-second internal grader limit. Their original packets, responses/empty output,
deadline receipts and conservative reservations remain available. No quality
scores were inferred. The output controller subsequently used one reader answer
per grading request, without changing the reader, rubric or valid responses.

A reader setup attempt failed before any CLI/model dispatch because its receipt
parent directory did not exist. Its request and explicit zero-dispatch disposition
are retained. The replacement has a new identity. No candidate was rerun.

The evaluator prepared distinct withheld packs for both phases before execution.
Phase 1's pack was not admitted. The clock started at 22:11:10Z and would end at
23:11:10Z; Phase 2 started at 23:09:19Z, with 1 minute 51 seconds still remaining.
The phase clocks therefore overlapped, although native model runs did not. The
remaining time could not admit the confirmation block. The last visible native
block ended at 22:42:51Z. The following 26 minutes 28 seconds went to fixed readers,
179 seconds of direct grading, output-controller fault containment, preparation
and lead work. Prioritizing those tasks left the unseen pack unrun; it was not
simply dispatched after an already-expired clock. No broad recommendation follows.
Phase 2 subsequently attempted a distinct fresh pack, outside tuning.

## Reproduction and qualification

The [frozen plan](plan.md) defines the pre-results tolerances. The
[W03 summary](evidence/phase1-W03-summary.json),
[W05 summary](evidence/phase1-W05-summary.json) and
[representative summary](evidence/phase1-representative-summary.json) bind each
retained native result to its SHA-256 digest. Their adjacent manifest files index
the exported synthetic evidence. Original SQLite checkpoints and immutable
binaries remain in the task artifact directory recorded in those manifests.

The runtime baseline is main `a921e32533dc03134ed8a2001bd82f27194c3302` plus
capability-gated evaluator support. CLI SHA-256 is
`35f686fb8e6b1539477e0fe5439c2f6f5c9a3e891d17e63b76db5a30dd365f44`.
The exact combined CLI/seeder build has CLI features `[]`, app features `[]`
and transitive brn-ai features `[scripted-runs]`; this is not a pure library-default
closure. Ordinary CLI transport used actual Codex calls without test injection.
E's search candidate and C's provider refactor are excluded. Helpers are pinned
but optional AI conversion and agent helpers are off. No native desktop UI,
owner acceptance, release or merge is claimed.

Reported usage is subscription transport usage, not a monetary charge. Missing
usage and outside orchestration requests remain unknown; estimated pilot credits
in retained legacy output are not observed charges. The [final accounting](accounting.md) and [Council findings](review-followups.md)
accompany the two PRs. The post-run audit-status fix was tested offline; the exact
executed supervisor is archived with its original hash. It did not replace any
measured source or alter a retained result.
