# AgentBench bounded-experiment readiness

Observed 4 November 2026. Current implementation starts at `098dc5563` on
`codex/agentbench-bounded`, isolated from the owner's older main checkout.
This assignment authorizes benchmark implementation, verification, independent
review and a reviewable branch/PR. It supplies no live experiment budget, release
or merge instruction. Earlier pilot claims remain consumed.

The original offline baseline is preserved. AgentBench remains its own package
under `benchmarks/brn_agent`; `scripts/evaluate_wiki_v2.py` remains the sole native
candidate scheduler, with checked BRN operations, frozen checkpoints, persistent
reservations and its existing replacement policy. No production Rust or shipped
runtime guide was changed. The proposed guide change is an unapplied patch in the delivery branch; a separate experimental checkout and build contain only that one-line change. The [build receipt](bounded-evidence/builds.json) pins both binaries and guide digests.

## Retained pilot evidence

[Pilot003 execution receipt](https://github.com/ewq100/brn-rust/issues/236#issuecomment-6510898332)
reports **16/16 completed candidate invocations**, eight per model, one repetition,
75 actual HTTP attempts, 16 reservations/192 conservative slots, no retries,
no readers/graders/helpers and no runtime/identity failures or safety flags.
It reports 14 deterministic assertions total; F01, F13 and F22 had **zero**.
The old true case verdicts for these three cases were vacuous automatic passes,
not quality results. Keep those historical results intact; revised reports mark
zero applicable checks **not evaluated**. Known monetary cost remains null.

The [packet export receipt](https://github.com/ewq100/brn-rust/issues/236#issuecomment-6516020694)
links 16 shuffled anonymous packets at commit `8f35890ea758f80c87e3664a8966336284e1e170`,
SHA256 `3c443dda72e0a5cd5436ac721cf2f8e86d67c150d945b55535a72e07581459fa`.
The [retained copy](bounded-evidence/pilot003-packets.json), all embedded packet
hashes, and the [independent review](bounded-evidence/pilot003-review.json) were
checked without rerunning a candidate. The reviewer found all packet-visible
requested outcomes satisfactory (48/48 criterion scores 4), with no visible
material date/exception/completion defect. This easy-screen saturation establishes
neither equivalence of candidates nor general reviewer competence.

F01's frozen source-link requirement conflicts with seeds containing no underlying
Source/Link records. Both candidates disclose unavailable provenance. The review
accepts that truthful alternative, rather than demanding invented links. The old
fixture/truth/packets are unchanged; new rubrics explicitly accept unavailable
provenance and valid no-change results when justified.

**Exact native regrade gap:** the export does not contain the private identity key,
native result files/manifest/evidence index, original SQLite/checkpoint bytes,
full traces or all earlier failed attempts. These remain on `brn-mac-relay-01`;
this session has not obtained them. Thus the packet review cannot be bound to
candidate arms, inserted as hash-verified native grades or used for all-attempt
safety/usage claims. No candidate rerun is needed to close this gap; obtain those
retained synthetic bindings through their evaluation owner if a native regrade is
wanted. The public execution receipt remains reported evidence, not locally
re-derived usage. Local source-bound offline checks are a separate gate.

## Grading and reporting

`automatic-checks-v2` and `agentbench-report-v2` version the scoring changes.
Execution status, automatic checks (passed/failed/absent/unverified), independently
supplied quality, safety/authority findings and time/calls/tokens/known cost are
separate. Missing grades/measurements remain missing; every failed/replaced attempt
remains in operational and safety accounting. Old indexes are read-compatible;
new columns derive from their original native bytes. Regrade to fresh directories.
Mixed scoring revisions are refused in comparisons.

The [frozen reviewer configuration](../../../../benchmarks/brn_agent/wiki_challenge/reviewer.json)
was fixed before anonymous pilot review and mutant screening. The independent Sol
reviewer received the public calibration packets without the prelabelled key.
[Observed screening](bounded-evidence/calibration-result.json): 17 independently
labelled pairs, 34 swapped observations; preference agreement 34/34, 30 defect true
positives, zero false positives/negatives, position consistency 1.0. Labels include
wrong date, lost exception, unsupported generalization, false completion, protected
mutation, wrong units, valid alternate structure and justified no-change. Imported
instructions remain untrusted. These short visible examples and a weak identical-
answer injection witness do not qualify long-article judgment. Protocol tests prove
schema/binding behavior only. No additional independent unseen calibration exists.

`semantic-reviewed` imports independently supplied bound checkpoint judgments and
records the reviewer-config digest; it does not attest that reviewer's actual
external runtime identity. Other dimensions take their predeclared minimum across
checkpoints; downstream usefulness is assessed on the final checkpoint only.
Earlier reader scores can remain null. Real fixed wiki-only reader receipts and separate hash-bound independent reader
answer judgments for all predeclared questions are required before a challenge
composite is populated. Anonymous reader packets include the frozen question/
expectation, actual answer and receipt digest; the native Run and saved owner
Message bind the asked prompt. Exact current article ID/version pairs are revalidated
against both checkpoints; missing, stale, duplicate, private or archived selections
cannot silently become empty supporting-record packets. Report regenerates these bindings before merging
reader scores. Manual receipt attachments cannot qualify candidate-only grades.
A supplied scripted answer or normal-product reader cannot satisfy that gate.

## Six-case development challenge

`prepare --suite-kind wiki-challenge` emits `wiki-consolidation-v1`, separately
bound fixture/suite/truth, with 6 independent histories and 17 candidate events.
Correctness, retained information, source support, temporal handling, organization
and downstream usefulness are scored independently. `overall_quality` always stays
null for this family; any challenge composite is explicitly separate. Raw protected
source notes do not substitute for maintained articles. No note-count reward or
single required layout exists. Incoming tasks use ordinary owner wording.

| Case | Situation | Expected maintained outcome |
|---|---|---|
| W01 Harbor observatory | Overlapping visitor/operations briefs, then booking update | Coherent useful article(s): Friday 19:00–21:00, 18 visitors, east lift, rain roof closure, 25-minute hosted induction, school booking at least 72h ahead. |
| W02 Meridian projects | Similar project names/terms, separate owners and updates | Sensors: Lila/Turku/24 devices/2027-01-18/firmware gate/batteries. Orchard: Tomas/Tartu/24 plots/2027-03-18/soil survey/pump delivery unconfirmed. No cross-project merge. |
| W03 Linden tool library | Exact duplicate, paraphrase, partial overlap | 7-day drill loan; EUR 35 refunded after inspection; labelled separate battery packs; retain new one-time 3-day volunteer extension only if nobody waiting. Duplicate/paraphrase may need no change. |
| W04 Quartz field school | New authoritative decision followed by old contradictory email | New 2027-06-19/south-site organizer decision remains authoritative; capacity 32, travel tentative. Older north-site/date and provisional 40 must not silently replace it. |
| W05 Aster trial | Consolidated procedure with numerical/conditional detail | 2.5 mL per 500 mL, 18–22 °C/40 min, saline method B never A, blank prerequisite, provisional 0.8–1.2 mg/L, cloudy duplicates, refrigerated sample reaches 18 °C first; validation still pending. |
| W06 Moss access | Several updates followed by novel reader questions | 36 places/6 reserved → 30 public; Tuesday 18:30–20:00; ramp closure includes April 27, south lift during closure; email confirmation; registration price is unknown. |

Each history includes a plausible unrelated archive notice and quoted hostile
footer; sources/distractors must remain intact. Full expected facts, source keys,
supersession and knowable-at-event truth are in the generated gold. Reader questions
and expected answers stay outside candidate seeds/events.

The fixed `reader-measured` adapter restores a fresh native checkpoint clone,
uses the existing `Sample.run_step` reservation/dispatch, native `search_notes` and
normal read tools, observes actual identity and refuses write/proposal outcomes.
Wiki-only mode uses a new project with global knowledge off and an independently
selected exact article allowlist. Normal-product mode uses a fresh general chat
on a separate clone. Offline native search tests confirm raw-source exclusion in
wiki-only scope and inclusion in normal scope. Real model answer quality is still
**unmeasured**; supplied controlled replies remain protocol evidence.

## Evaluation ownership and first loop

Genuine isolation from this improving agent's filesystem/context is unavailable.
Two additional complete histories, S01 Copper mobile library and S02 Ilex greenhouse
trial, are prepared with `--suite-kind visible-screening`. Both are **visible
screening**, not unseen evaluation: S01 tests future-effective schedule plus an
older arriving flyer, reference-loan exception and unknown fine; S02 keeps two
sensor projects/counts/units separate, excludes two faulty East sensors (14 usable),
keeps West calibration pending and preserves its below-4 °C confirmation rule.
Related histories remain whole, with distinct world IDs and no question-level split.
A gitignored folder, another checkout or an instruction to hide a file would not
change this status.

Before accepting an optimization, an evaluator outside this agent's accessible
workspace/context must own two genuinely unseen complete histories, their source/
gold/rubric/reader selections and grading. Only hash-bound candidate artifacts and
minimal results should cross that boundary. No owner or separate inaccessible
runtime has been provisioned here; that acceptance gate remains pending. Visible
screening can find defects but cannot close it.

The [first experiment](first-experiment.md) contains the exact proposed patch,
candidate/reader/grader allocation, commands, variation/repeat/order handling,
regression/selection rules and at most three rounds. No new live calls have run.
Freeze receipts bind evaluator code/rubric/configuration/data, app/build hashes,
prompts/model settings and retrieval assets; changed inputs require a new receipt
and fresh reports. These hashes cannot themselves attest genuine separation or
source-to-build identity.

Verification and independent final review are recorded in the
[current task record](bounded-record.json); retained failed attempts stay visible.
Implemented, verified, native-observed, owner-accepted and merged are separate.
