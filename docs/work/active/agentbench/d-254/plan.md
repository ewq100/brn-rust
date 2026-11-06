# Agent D #254 execution plan

Authority: owner handoff and [issue254](https://github.com/ewq100/brn-rust/issues/254),
including the same-machine withheld-evaluator clarification posted there.
Status lives only in selected `windows-ci-portability/record.json` task
`knowledge-quality-254`. [Design history](design-history.md) and
[raw independent reviews](design-reviews.json) preserve earlier proposals/findings.
No old pilot allocation, release or model default change is authorized.

## Baseline and qualification

Isolated checkout `work/agent-d-254-comparison` at planning main
`a921e32533dc03134ed8a2001bd82f27194c3302`; retain owner main9b47b285 and unrelated
`.vscode/`/`experiments/`. Rig0.46, pinned Rust1.98.1 and Cargo.lock. Default-feature
CLI/BM25, no embedding assets. E search changes and C provider refactor excluded.
Owner hardware correction (254 comment6696700518): B uses Macmini; D/E use MacBook. B acknowledgement is not required. Serialize D/E heavy builds and latency on MacBook with separate targets/data. E takes first baseline build, D next; B can work concurrently on Macmini.

Before candidate dispatch: qualify source-bound CLI/seeder/helper artifacts,
actual model availability and medium support on the existing subscription,
runner/checkpoint/evidence/native-reader checks, request reservations, deadline
cleanup and freeze verification. Keep failed attempts. Optional capability-gated
no-tool grader is evaluator support only, absent from the measured default-feature
app. Build exact production runtime from the qualified baseline before tuning.

Read-only discovery: ordinary subscription usage allowed, weekly29% used, no
credits/spend control reached; remaining request capacity UNKNOWN. Four metadata
RPCs, zero generation requests. Reobserve admission before each phase/confirmation.
No copied credentials, purchases, resets or provider/account/model fallback.

## Frozen settings and finite limits

Both model arms use the same binary and inputs. Only main model changes:
Codex `gpt-6.1-sol` vs `gpt-6.1-terra`, medium, helpers off, AI conversion off.
Common enforced HTTP model-attempt cap6/event; requested ask tool-round budget8
is separate. Intake has no tool-round-budget flag; sent/observed rounds remain
unset/unknown. Candidate command40s; timeout is censored latency at40s plus a
completion failure, never silently omitted. No priority service tier requested;
report observed identity/settings or unknown.

Fixed reader unchanged `wiki-reader-v1`: Sol medium,8tool rounds,12HTTP,90s,
real native search/read on a new checkpoint clone, exact article ID/version
allowlist excluding raw source notes. Fixed independent grader unchanged
`wiki-reviewer-v1`: Sol high, no tools; anonymized packet input only,45s/request,
exactly1HTTP admitted using production CallBudget, no retry. Same-family bias
is disclosed; deterministic checks and non-Sol blind review of disagreements
supplement it. Missing measurements remain null/not evaluated.

Shared immutable escrow `/Users/evokessler/Documents/Codex/brn-shared-de-20261106/`:
1760 = D1592 + E168; no unassigned capacity. E168 includes120BRN (48confirmation
INSIDE120),24grader,24Codex Council. D subdivisions are non-additive projections
of its original1592, never independent allowances:

| D role | Maximum reservations | Time cap |
| --- | ---: | --- |
| Phase1 incl fixed readers and withheld confirmation |600|60min|
| Phase2 development |264|part of90min Phase2|
| Phase2 broad visible regressions |432|protected before development|
| Phase2 fresh withheld confirmation |144|protected before development|
| Independent grading and evaluator setup |120|20min INSIDE phase/cycle clocks|
| Codex Council design and final |24|12requested/10min per seat|
| Discovery/failed/uncertain contingency |8|inside original clock|
| Total |1592|180min whole cycle;150min live phases|

Whole-cycle clock began2026-11-06T21:30:06.983387Z, ends2026-11-07T00:30:06.983387Z.
Build/wait/review time after that start counts; no clock restart. Phase2 retains
840 total and576 broad/fresh confirmation protected slots. Earlier unused328/512
children were superseded before BRN dispatch, receipt preserved; no consumed call
refunded. Evaluator setup12 comes from grader120, leaving108 for actual grading.

Reuse `limits.Reservations` and v3 `deadline_guard.Guard` via experiment
`experiments/d254/supervise.py`; native v2 remains sole candidate scheduler.
Role debit precedes native local reservation; uncertain/local-refused attempts
keep that debit. Guard checks current in-memory step and previous operation
receipts before another CLI launch. Missing terminal outcomes, rate/transport
failure, accounting uncertainty, identity mismatch or confirmed safety defect
stop further dispatch. Expected tool-budget/invalid-tool behavior remains a
scored model failure under the existing v2 terminal taxonomy, not a success.
No automatic resume/retry; each operation/root/identity is exclusive.

Before each block require enough original remaining time for its frozen minimum,
grading and30s cleanup; otherwise record not-run. These finite time boundaries
can curtail worst-case work: Phase1 planned generation worst case56min before
grading; Phase2 worst case96min before grading, beyond its90min cap. Report any
curtailment; do not discard failures, extend time or claim complete confirmation.
Prioritize protected confirmation over extra tuning. Genuine observed faster
completion may fit; no worst-case completion guarantee is made.

Council: two funded independent rounds (design/final), within personal maximum3.
Seats Sol/Codex, Opus/Kiro, Kimi/Copilot; only named policy fallback after failure.
Kiro failed monthly limit in design; Opus/Copilot fallback responded. Across all
routes at most8seats including failures,12requested calls/seat,96requested total,
30min total Council wall window. T3 cannot enforce/observe actual provider turns;
report unknown actuals, debit requested ceiling conservatively, never claim HTTP
accounting complete. Parent orchestration/unrelated actor usage also lies outside
BRN enforcement; observe quota and stop on denial. Council is outside measured
runs and never replaces fixed reading/grading.

## Phase1: same-product comparison

Visible W03/W05 two paired repetitions:24model events*6=144.
F01 creation/F04 writing/F07 privacy-scope/F16 stale-save recovery two paired
repetitions:16model events*6=96 (owner events deterministic, zero model HTTP).
W03/W05 final reader question for every run:8*12=96. Visible total336.
Fresh evaluator pack:2one-event histories*2arms*2repetitions=8events*6=48,
8fixed-reader calls*12=96, total144. Phase1 planned480<=600;120 remains for
failed/uncertain attempts, not adaptive tuning. Rotate A/B then B/A; fresh
synthetic equivalent workspaces. Preserve all actual attempt/usage outcomes.

Decision tolerances BEFORE results: each W criterion on0–4 averaged equally over
W03/W05 then equally over repetitions; F categories reported separately. Terra
must have no new material correctness/grounding/protection defect, completion
>=95%, and no criterion mean more than0.25 below Sol. Require either >=15% lower
paired median latency, >=10% lower observed token usage, or >=0.5 higher
`downstream_usefulness`. Both repetitions must favor Terra on the efficiency
metric used; partial/missing usage in either arm cannot meet that rule. Show
ranges/censored counts, no statistical superiority claim. Fresh withheld results
are required for a limited engineering recommendation; scope it to this frozen
setting/case sample. No broad model-superiority claim or automatic default change.
Mixed/missing evidence gives insufficient evidence and provisional Sol medium.

## Phase2: actual single-factor improvement

Fix chosen model/effort and every other setting. Preserve original baseline;
rerun it within Phase2 rather than substitute Phase1 outputs. Inspect development
failures first, then test the prepared `wiki-retention-v1.patch` as candidate1.
Patch provenance: authored for public PR246/W03/W05 development before this task;
it is not unseen evidence. First check against current guide, build separate
candidate with only that guide factor changed, retain exact patch/build receipt.

One full candidate is funded (owner permits up to3 but no second complete cycle
fits while preserving confirmation). W03/W05 baseline/candidate2paired repetitions:
24events*6=144 plus8readers*12=96, development240, spare24 for failures.
Target retained_information mean gain >=0.5/4, positive in BOTH paired repeats,
and greater than original-baseline repetition range (absolute difference of
repetition1/2 means across W03/W05). Correctness/source support/temporal handling/
organization/reader usefulness cannot decline; no material authority/protection/
preservation defect. Baseline retention>=3.5 is saturated for this target gain:
stop without accepting. Quality and efficiency remain separate.

Freeze final candidate; compare original baseline vs candidate on visible W01–W06
one paired repetition:34events*6=204, all8questions*2arms*12=192 (W06 has3),
plus F04/F07/F16 one paired regression:6events*6=36. Broad total432.
Distinct fresh evaluator Phase2 pack:8events*6+8readers*12=144.
Phase2 planned240+432+144=816<=840, leaving24. Do not tune on confirmation.
If no qualifying improvement, confirmation fails/is missing, or safety regresses,
retain accepted original baseline and preserve rejected/screening artifacts.

## Withheld evaluation and grading

Fresh role `d254-evaluation-only-setup`, Codex/Sol high/default, no tuning history
or participation, prepared/froze two distinct2history packs before model runs.
Files owned in external task `evaluation-only/`; lead/tuning Council do not inspect
contents. Return aggregate results/failures/digests/artifact references first;
full evidence stays available for later review. New owner clarification permits
procedural withholding on this same machine; filesystem access is NOT technically
isolated. Revealed/tuning-used cases become development for future cycles.

Pre-execution manifest SHA256:
`23e90f48332c70fb0939160895b11f830a44cd91b9876f9d1f817fc8c4aad33a`.
Preparation-complete manifest:
`d78912c33322634d3b7608da997cff7180d8f6df1f5c7d820816170278ca3248`.
Validation receipt:
`cf485017133ca43339382d24e7549620d94a14369136de2712cbd9a70d2a51ef`.
Reported8preparation toolcalls,0providers/builds, all4histories seeded through
existing checked seeder; source qualification for measured runtime stays separate.
Actual setup provider turns/usage unknown; requested12 debit retained.

Grading unit is each declared candidate-event checkpoint and actual reader answer;
at most8 independently shuffled packets/request, each with exact hash/reference
schema. Export only through maintained semantic-prepare/reader-review-prepare;
import via semantic-reviewed/reader-review-import. Keep raw replies/packets/order,
all event-time truth and failed/missing grades. Plan includes168packets across
phases (64Phase1,104Phase2), up to21base grader requests, plus one batch of4
shuffled repeat packets for consistency. This fits remaining108grader slots.
No candidate self-grading; no grading candidate-only downstream usefulness.

## Delivery

Dedicated comparison PR with binary/configuration/case freeze, real saved-state/
fixed-reader evidence, separate quality/completion/time/usage metrics, all failed
attempts, actual Council identities/scores/unknowns and scoped recommendation.
Separate knowledge-improvement PR with actual focused change, measured keep/reject,
broader/fresh confirmation, preserved baseline/rejected attempts, full accounting.
Finish applicable checks and bounded final Council evidence/code review. No merge,
release/publication, app default change or inference to Copilot rollout.
