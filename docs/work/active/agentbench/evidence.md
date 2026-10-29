# AgentBench offline proof evidence

Observed October 28, 2026 in the managed Linux cloud environment. Base main:
`c3d64efa45a3671b6f91e94f1425fcda9fc87690`; isolated worktree
`/workspace/brn-agentbench`, branch `benchmark/agentbench-proof`.
No repository `.agents/skills` was present. Read root AGENTS, development
workflow/strategic programming/verification, current work overview and relevant
native evaluator/seeder/application/CLI source. The first checkpoint changed no
production Rust; the continued milestone adds an opt-in request cap and bounded
wire identity observation. Runtime prompts remain unchanged. Existing branches and PR239–241 were untouched.

## Reuse and responsibility

The existing Python `Sample` and matrix scheduler still own native execution and
one replacement attempt. Native backup/restore and checked owner operations own
state. The new Python adapter only verifies/imports frozen evidence and computes
reports; it does not choose models, alter credentials or invoke a provider.
Alternative page structures and direct solutions remain valid semantic outcomes.
No new production dependency, provider gateway, storage layer, or parallel agent
framework was introduced. Legacy fixtures and result readers remain available.

The eight categories and weights follow the recovered proposal. Criterion
scores are explicit reviewed inputs, not automatic keyword-based semantic
judgments. Source-level checks remain structural diagnostics. Overall scores
stay unranked while coverage, grader qualification or runtime identity is missing.
Unknown usage/cost is never silently estimated in the new reports.

## Checks actually run

- Initial `cargo build --locked --offline -p brn -p brn-threads-app --example
  wiki_eval_seed --bin brn --jobs 2` failed because cached registry metadata only
  had Rig 0.44.0 while current main pins 0.45.0. The same locked build with network
  dependency retrieval succeeded in 65 seconds. No dependency or lockfile changed.
  Rust 1.98.1 and the configured document helper paths were used. No other Cargo
  process was running before the shared target build; all affected local BRN
  crates rebuilt from this worktree.
- `python3 -m unittest discover -s scripts/tests -p test_evaluate_wiki_v2.py -q`:
  101 passed. Initial stricter intake-budget validation failed two legacy DW12
  fixture cases; restricting that new admission rule to checkpoint-enabled cases
  retained compatibility. The legacy declared/sent distinction is now explicit.
- `BRN_AGENTBENCH_TEST_BINARY=/workspace/brn-environment/target/debug/brn python3
  -m unittest discover -s benchmarks/brn_agent/tests -v`: 15 passed, zero skipped.
  Initial native-test setup omitted `arm`, then `trace`, from its sample fixture;
  both setup errors were corrected before the actual native witness ran.
- Native evaluator without `--live`, generated proof fixture and freshly built
  `brn`/`wiki_eval_seed`: all nine cases seeded, warmed, copied, opened and passed
  Knowledge checks. No candidate ask/intake model steps ran. Local receipt:
  `/workspace/brn-agentbench-native-proof/offline.json`.
- Prepare → collect → report CLI smoke succeeded with empty grades. The static
  HTML/CSV/JSON output correctly shows incomplete/unranked, not a model result:
  `/workspace/brn-agentbench-ungraded-report/`.
- Markdown links and `git diff --check` passed before final review.

The native witness creates a fresh synthetic Note, captures a real backup,
restores into a separate probe, runs checked Undo, verifies the probe's restored
state, compares the original records byte-for-byte at the JSON level and checks
the original backup hash. Reusing a checkpoint path is refused.

## Independent review

A separate read-only reviewer inspected the change. Findings addressed:

1. Partial draft proof could rank as complete: eligibility gaps now keep it
   unranked, independently of a numerical quality score.
2. Manifest labels could contradict native bytes: import derives metadata from
   native results and report verifies the entire derived entry.
3. Checkpoint/citation integrity: hash-check backups and records, then resolve
   structured record/version or result-step references.
4. Interrupted folders omitted: preserve and report legacy orphan stubs; the new
   adapter explicitly inventories unindexed attempts and supports their replacement.
5. Wrong fixture could be scored under reused IDs: generated suite binds fixture
   bytes; collection checks the native manifest and freezes its hash.
6. Missing helper traces became zero: enabled-but-unobserved helper usage is null.

Final bounded re-review found no remaining blocking issue in the implemented
surface. The reviewer ran the Python contract tests with its native witness
skipped; the explicit native execution above supplies that separate evidence.
The normal tooling test lane now discovers these tests through
`scripts/tests/test_agentbench.py`.

## Blocker and remaining implementation

The parent supplied the recovered original plan text. Library search resolved
`BRN-AgentBench-Consolidated-Plan.md` and `BRN-AgentBench-Planning-Pack.zip`.
Current bundled Library download helpers were materialized into a fresh helper
directory and invoked with the complete structured search result and selection
000001. The helper failed before transfer with:
`hosted apps tools/list request failed with HTTP status 401`.
No bytes downloaded, no retry, no alternate URL or credential workaround. This
was an authentication failure, not an automatic approval-review rejection.
Original twelve helper cases and twelve calibration pairs are pending; they were
not reauthored or represented as integrated.

The benchmark README lists remaining implementation independently of this
transfer blocker; the continued milestone below closes several earlier gaps. No
claim that phases 1–3 or the full recovered specification are complete. No live
runs, cost incurred for model calls, private user documents, Mac sessions,
external dataset redistribution, PR approval, merge or release occurred.


## Continued offline milestone

Implemented opt-in `--model-call-limit` in the real ask/useful-intake runtime.
The atomic shared counter covers parent, helper, retry and summary dispatch;
unsupported model endpoints refuse under a cap. Intake refuses a cap with AI
conversion enabled or non-useful intent. Traces retain bounded sent model/effort
and returned model IDs, never auth headers or prompt bodies. Scheduler reservations
are locked, persisted and never refunded after uncertain execution. `--max-spend`
refuses before work because supported subscription routes expose no enforceable
monetary charge bound. It does not pretend estimated credits are measured cost.

Controlled owner thread/edit/discard/apply/dismiss/Undo events use actual CLI
handlers. A stale edit witness correctly left a dirty recovery buffer; the
initial Undo expectation failed, then the scenario was corrected to assert the
refusal, explicitly discard, and retry Undo. No host protection changed.

Added checkpoint-scoped anonymous grading, explicit wiki artifact allowlists,
blind pair-order calibration and absolute defect accounting. Added an optional
Inspect 0.3.286 frozen-output adapter, installed only into an isolated verification
venv. Initial Inspect logging attempted a read-only home data directory; the test
now directs XDG data/cache to its disposable workspace. The parity probe observed
one preserved output and zero model usage. Inspect emitted a non-failing AnyIO
resource warning. No candidate or independent grader was executed.

Local LongMemEval transformation preserves dates/full histories, strips oracle
labels, hides original IDs, groups overlapping sessions and requires explicit
provenance admission. Upstream README, cleaned MIT card and metadata were checked
October 28, 2026: revision 98d7416c24c778c2fee6e6f3006e7a073259d48f.
MemoryAgentBench metadata revision 7ea066982b140a19337e17e60d45d4076e042faf also
reported MIT, but no adapter/data admission is claimed for it. No corpus bytes
were downloaded or redistributed. Cleaned MIT metadata does not settle embedded
upstream provenance; actual external admissions remain empty.

Verification for this milestone:

- Locked native binary/seeder build passed. Strict Clippy for brn-ai/brn all targets
  passed after replacing a large transport-error return with a narrow IO error
  at the observation boundary. No dependency lockfile changed.
- Existing evaluator suite: 101 passed. Explicit native plus isolated Inspect
  benchmark suite: 29 passed, zero skipped (final rerun).
- CLI specification suite: 11 passed. Atomic wire-observation tests: 2 passed.
  In-memory capped transport test: 1 passed, one real scripted HTTP dispatch,
  refusal before the next dispatch, sent/returned identity observed.
- Existing scripted helper suite: 31 passed, 6 failed. Credential-backed tests
  reported helper unavailable/credentials unsafe before their expected provider
  exchange. An unchanged cached-input baseline also failed with UnsafeCredentials.
  No security check was weakened; in-memory transport tests are narrower evidence,
  not proof of credential-backed provider integration.
- Fresh native offline preparation: all 9 cases passed at
  `/workspace/brn-agentbench-final-native/offline.json`. Static incomplete/unranked
  report exported at `/workspace/brn-agentbench-final-report/`.
- Independent review fixed malformed owner-event references, scenario/gold
  mismatch and modified checkpoint export, and malformed calibration defect sets.
  Regression tests cover each issue, including canonical checkpoint directory names
  so an unverified `01` directory cannot substitute for captured step `1`.

Remaining gaps are explicit in README/coverage: original pack integration,
controlled mid-request interruption, protected/skill scenarios, qualified semantic
and fixed-reader execution, native external-history intake mapping, and any future
supported monetary bound. No claim that the complete benchmark is finished.

Final bounded independent re-review confirmed the canonical checkpoint guard and
found no remaining findings in the reviewed surface. It reran 10 evidence tests;
full native/Inspect and Rust validation above was performed by the implementation
owner. This is not a statement that unimplemented coverage is qualified.


## Final authorized offline scope (October 29)

Added two native boundary fixtures (protected content and skill/source authority),
a fixed host-contract runner, the controlled reader example, the retained recovery
example, native LongMemEval history mapping/staging, and the bound semantic grading
protocol. These remain diagnostic machinery and synthetic witnesses, not semantic
model qualification. The original 12+12 planning-pack fixtures remain inaccessible;
no replacement originals or live results were invented.

The reader restores a hash-verified checkpoint into an exclusively owned directory.
Wiki-only mode creates an isolated project, links the explicit general-note
allowlist, and disables global knowledge for that project through normal checked
services. The fixed script can only call native read_note; it cannot request writes.
Normal-product visibility is tested separately on another copy. A raw-source canary
is absent from wiki-only model-visible history and present when explicitly read in
normal-product mode. All pre-existing records and original backup/record hashes
must remain unchanged. The fixed supplied response and token counts are labelled
synthetic; credential/helper/connector-configured snapshots are refused.

The recovery example retains seven native checkpoints, each with records, backup,
receipts and candidates plus hashes. It abandons a prepared edit before terminal
persistence, saves a later owner qualification, recovers the Run exactly once,
checks immutable candidate retention and stale refusal, dismisses the stale
candidate, creates a distinct retry, and applies only through explicit review.
Undo runs on a restored copy, preserving the later owner text and the continuing
workspace. The real-process-kill tests remain a separate reused contract lane.
Observed result: `/workspace/brn-agentbench-recovery-observed/recovery-result.json`.

Native history staging imports all dated historical turns through normal full-note
intake with conversion AI off. First checkpoint contains only the first session;
second includes the later revision. Answer/oracle canaries never enter records.
Selected dataset locks, admission provenance and gold are retained under
`grader-only/`, outside candidate state. Successful synthetic receipt:
`/workspace/brn-agentbench-history-observed/stage-result.json`.
No real external history was admitted or redistributed.

Grading preparation emits anonymous latest-captured-state packets separately from
private attempt bindings. Actual events, exact references, checkpoint-scoped gold
and the fixed rubric are included. Custom evidence indexes are honored. Local
Source/import/citation paths are anonymized because native arm-directory paths can
otherwise identify a candidate. Controlled replies bind exact packet hashes;
wrong versions, changed packets, missing criteria, malformed scores and explicit
grader outages are rejected or recorded incomplete. Earlier checkpoint packets
remain separately available. Empty inputs never qualify a calibration/grading run.

Failures encountered and retained:

- Preinstalled document helpers failed native intake (initial missing-path IO,
  then Protocol mismatch). Rebuilt both current helpers with the repository's
  locked/offline script into `/workspace/brn-agentbench-document-helpers`, with
  separate stock/adapted targets and receipts. Builds took 4m26s and 4m18s. Native
  intake then passed. No helper protocol/security check was weakened.
- The initial controlled reader used an incomplete read_records argument and did
  not actually observe note text. Its witness failed. Replaced it with exact-version
  native read_note and now assert both positive wiki text and negative raw canary.
- Example builds caught a Result alias collision and non-serializable outcome;
  corrected local example types/diagnostic serialization. Strict lint caught a
  needless borrow, corrected without changing behavior.
- Broad existing skills tests: 7 passed, 3 credential-backed tests failed (the
  barrier case explicitly returned UnsafeCredentials). Steering: 5 passed, 1
  credential-backed test failed with UnsafeCredentials. These failures remain
  unqualified; no security checks or credentials were altered.
- First curated host report marked the process-kill group failed despite 16 passing
  tests because it contains one intentionally ignored manual UI-store exporter.
  The runner now explicitly records only that named manual export as excluded;
  unexpected skips and zero-test success still fail. The first report is preserved
  at `/workspace/brn-agentbench-host-contracts/host-checks.json`.

Independent review addressed source-path identity leakage, discarded admission
provenance, and accidental use of the default evidence index. It source-reviewed
the retained recovery example and found no remaining blocking issue after fixes.
Explicit native execution is separate from that review. Final counts and lint are
recorded in the task record once completed. No live calls, model spending, push,
PR, merge, release, Mac access or original Library-transfer retry occurred.


Final verification of the October 29 increment:

- 35 benchmark tests passed, zero skipped, with explicit CLI, controlled reader,
  controlled recovery and isolated pinned Inspect interpreter.
- Existing evaluator suite: 101 passed. Both new boundary fixtures passed native
  seed/copy/open/Knowledge checks without model calls.
- 48 curated native tests passed across all 11 host-contract groups. Exactly one
  named ignored manual UI-store exporter is recorded as excluded, not tested.
  Final receipt: `/workspace/brn-agentbench-host-contracts-final/host-checks.json`.
- Both controlled examples built under the locked offline toolchain and passed
  strict Clippy. Markdown links and diff whitespace checks passed.
- Final independent bounded review found no blocking issue; it checked the manual
  export exclusion accounting and its regression. It did not run Cargo/live calls.

The local implementation is reviewable. Mandatory original judgment-fixture
integration remains blocked specifically on supported Library access. Credential
route qualification, independent/live semantic qualification, any model spending,
external corpus admission and publication remain separate explicit gates.

## Newly authored judgment fixtures — 2026-10-29

Parent authorized new, explicitly labeled fixtures from the recovered specification
without retrying the failed Library transfer. Added H01–H12 helper scenarios,
C01–C12 calibration pairs (24 anonymous order-swapped packets), four separate
screening variants (8 packets), and a common public defect taxonomy. These are
new synthetic work, not original ZIP contents. Public variants cannot establish
secret/untouched qualification after tuning.

H01–H05 export through the existing native evaluator; five offline seed checks
passed in `/workspace/brn-agentbench-authored-seed/offline.json`. H06 uses the
existing no-substitution/unavailable-helper scripted contract. The new H07–H12
native test resolves real note IDs, versions and UTF-8 ranges, reads via actual
child handlers, and submits through the production validator. It checks retained
summaries, nonvacuous citation counts, exact/stale classification, complete scoped
reads and untrusted framing. It does not score parent semantic judgment. Three
other existing contracts check parent full-read ownership, unavailability and
read-only helpers. No production helper policy changed.

`helper-score` keeps natural and controlled evidence separate, uses declared
25/20/40/15 groups, leaves missing/unobserved slots incomplete and disqualifies
actual breaches. Imported reviewer references are not automatically authenticated;
results remain unqualified and unranked. Costs stay unknown. H01–H06 paired natural
runs and H07–H12 live parent report assessment remain unperformed; the latter's
producer is currently a native test fixture, not a live CLI injection adapter.

Independent reviewer `review_agentbench` approved all H01–H12 expectations and
C01–C12/V01–V04 draft labels after two corrections: publish a full taxonomy with
primary-defect overlap rules in every packet, and clarify V02 asks only the
approved repair cost, making rejected-proposal detail optional. Reviewer reran
four Python judgment tests, checked diff hygiene and found no remaining blocker.
This is independent draft-label review, not live grader qualification.

Verification: 39 complete benchmark Python tests passed with native binaries and
Inspect (`/workspace/brn-agentbench-authored-python-final.log`, zero skips). Four
native helper contracts passed (`/workspace/brn-agentbench-authored-host/`), and
the strengthened report assertions passed again in `authored-host-final/`.
Initial manually typed exact Cargo filter selected zero tests; it was not counted
as a pass and was replaced by the fixed fully-qualified host runner. Strict Clippy
initially rejected one needless borrow in the new test; the borrow was removed.
Markdown links: 687 files, 3274 local links, zero failures. Final verification is
recorded in the task record. No credential-backed test, provider call, original
pack transfer, push or PR was attempted in this increment.

Final four native contracts passed on the corrected source in `/workspace/brn-agentbench-authored-host-reviewed/host-checks.json`, including the helper fixture SHA256. Strict all-target brn-ai Clippy passed after the single borrow fix. Final reviewed pack: `/workspace/brn-agentbench-authored-judgment-reviewed/`. No tests were ignored in this increment.

## Full authored bank and offline execution increment — 2026-10-29

The completion audit found that the earlier implementation stopped at the initial
proof. Parent explicitly authorized continued offline implementation and bounded
parallel work. This increment adds the full authored bank: 24 focused cases, three
per category, plus Cedar/Atlas/Maple twelve-event journeys. There are 60 candidate
events and three deterministic owner operations in F16. F24 is explicitly
hypothetical uncertain-send reasoning, not a claim of external-service execution.
Original ZIP bytes were neither needed nor retried.

Each scenario has versioned checkpoint truth, typed scope exceptions, numerical
derivations, superseded facts and predeclared grading checkpoints. Gold is outside
native seeds. `--truth` binds its bytes to the experiment; explicit truth files and
exact legacy fixture identities support frozen regrading after code revisions.
All candidate checkpoints are graded with local truth/event prefixes. Criterion
minimums produce one grade per attempt without weighting longer journeys more.
Missing evidence remains incomplete; confirmed safety breaches are retained.
Native evaluated tool failures continue through later events; missing later
captures are uncertain evidence, never invented zeros for future model behavior.

The invariant checker consumes explicitly controlled or independently reviewed
claim annotations, exact note/version/quote references, real source links,
baselines and native receipts. It checks known fact/status values, exceptions,
stale assertions, missing source edges, duplicates, false completion and unrelated
record preservation. It does not extract or certify prose meaning. Positive,
negative and alternative-valid witnesses cover all 27 scenarios, including valid
paraphrases and a correct no-change outcome.

Retained H06–H12 native parent/helper execution now uses actual scripted BRN
search/read/report validation and saved parent messages. Good and alternative
witnesses each pass 7/7; deliberately bad witnesses pass 0/7. H10 performs a real
inadequate search followed by parent recovery; H12's unauthorized private read is
blocked. The assessor checks actual saved answers, current-version reads,
checkpoints, receipts and fixture revision, not an answer supplied as an outcome
label. Artifacts: `/workspace/brn-agentbench-parent-helper-bounded-good/`,
`/workspace/brn-agentbench-parent-helper-bounded-alternative/`, and
`/workspace/brn-agentbench-parent-helper-bounded-bad/`.

Bounded semantic fixture execution retains requests, responses, failures and
call/byte accounting over frozen packets without network calls. The reader uses
native scoped queries and reads on clones, with call-limit refusal, injected
provider failure and unchanged originals checked. All fixture answers and usage
remain explicitly controlled; no grader or model competence is inferred.

Reports now support paired comparisons clustered by world, a single frozen grader
revision, incomplete-pair handling, all-attempt cost/latency denominators, failure
classes and evidence-linked note diffs. Fewer than three independent worlds or
cluster resamples losing an entire category withhold intervals. Three worlds are
only a pilot. Checkout/guidance/adapter hashes are retained; source-to-binary and
complete retrieval asset attestation remain explicitly unverified. Resume rejects
changed harness/guidance/truth rather than attributing work to old provenance.

Synthetic-only adapters cover MemoryAgentBench FactConsolidation/EventQA schemas
and ClawArena choice rounds with explicitly normalized BRN text assets. Gold and
split groups remain separate. No external corpus is admitted or redistributed;
Claw directory/binary assets and executable shell checks remain unsupported.
Primary schema documentation/code was checked by the adapter agent; upstream
commit pinning is not claimed for these synthetic witnesses.

Independent review (`review_agentbench`) challenged and verified corrections for:
- mixed grader revisions hidden by a set comparison;
- changed provenance on resume;
- old proof regrading after the bank version changed;
- source links with wrong relations or nonexistent targets;
- absent baselines incorrectly treated as new saved work;
- latest-only grading hiding intermediate defects.
Final review found no must-fix issue in the offline increment. The reviewer ran
selected Python checks; native tests were separately executed by implementation
agents and the integrated root run. No qualification is claimed from review alone.

Verification on final code:
- 74 benchmark tests passed, zero skips, with all four explicit native binaries and
  pinned Inspect: `/workspace/brn-agentbench-complete-python.log`.
- 101 existing evaluator tests passed:
  `/workspace/brn-agentbench-complete-evaluator.log`.
- 27 native offline seed/open checks passed:
  `/workspace/brn-agentbench-complete-native/offline.json`.
- Reader and helper examples passed locked offline builds and strict Clippy.
- Seven scenario checks include actual native stale-edit refusal and preservation
  of the later owner qualification, without candidate provider calls.
- Markdown: 687 files, 3274 local links, zero failures; diff hygiene passed.
- Frozen no-model reporting correctly returns 27 planned scenarios, incomplete,
  unranked, null overall: `/workspace/brn-agentbench-complete-report/report.html`.

Preserved failed checks: the first combined run had 73 passes and one obsolete
limits-test expectation (32 calls from the old proof versus 126 conservative
reservations for the new bank); the arithmetic witness now uses an independent
explicit fixture. A new oversized-helper-input test caught an intended file-size
guard that had not inserted; the guard was fixed and retested. Earlier diagnostic
artifacts were not overwritten. Final planning bundle:
`/workspace/brn-agentbench-complete-planning/`.

No live model/provider run, credential change, restricted-source retry, corpus
admission/download, push or PR occurred. Publication remains pending. Live semantic
qualification, credential-backed route acceptance, actual measured spending and
separate native UI/external-service qualification remain gates; they are not
substituted by these offline passes.

## Publication — 2026-10-29

Evo approved pushing the reviewed branch and opening a draft PR. Reviewed commit
`bbc86dde73ca80404e2e4253ea570183e749e4db` was pushed normally and verified against
the remote branch and [draft PR #242](https://github.com/ewq100/brn-rust/pull/242).
The base remained `c3d64efa45a3671b6f91e94f1425fcda9fc87690`. CI monitoring is
pending; publication does not authorize merge, live models, spending or changes
to credential/security settings.

PR CI initially failed in Documentation and tooling, job `123219795601`, run
`40804414933`: Python 3.12 nested unittest discovery changed the outer loader's
root and raised `AssertionError: Path must be within the project`. AgentBench's
`load_tests` now uses a separate `TestLoader`; independent review found no issue,
and direct discovery retained the outer root while loading all 74 benchmark tests.
The full local tooling command ran 294 tests with 11 explicit optional/native
skips; its sole failure before and after the fix was the pre-existing process-group
cleanup witness. This container retained killed `sleep` children as PID-1 zombies
(observed `Z` state), so the unrelated test was not weakened or altered. Logs are
retained as `/workspace/agentbench-ci-discovery-before.log` and
`/workspace/agentbench-ci-discovery-after.log`. Exact-head remote CI remains the
next verification gate; no all-green result is claimed.

## Terra/Sol pilot supporting publication (29 October 2026)

Owner requested the existing-subscription Terra6.1-medium/Sol6.1-medium comparison.
Mac read-only discovery confirmed both exact IDs and medium at
2026-10-29T04:20:33.066650Z in issue236 comment6487835954; zero model calls.
The candidate source remains12251dfc78bc6ff9aa0eaf6809448886b0aa794c, whose CI
finished six Linux/macOS/tooling passes and one optional Windows compile failure
in unchanged Unix credential code. No merge or live dispatch occurred here.

The original local v1 package manifest15a39c16c6dabe790af1cb5fdf7c15f4ac0128e9ff34d42432a6cc19ac5ac5da
was verified before preparing explicitly versioned supporting package
`benchmarks/brn_agent/pilots/terra_sol_v2`. It retains the same cases/truth and adds
an installed-toolchain, locked/offline isolated build and Mac supervisor gate.
Launch time is deducted from one60-minute budget starting at the durable setup
claim. The fixed one-attempt job has16 candidate invocations,12calls/event192total,
300second command limits,59m30 dispatch stop and30second cleanup reserve.

Independent review corrected nested-session cleanup and accounting failure paths.
Eleven offline controls tests pass, including outer timeout during a nested
supervisor and cleanup with a truncated ledger; the existing101 evaluator tests
also pass. Markdown links/diff checks pass. Mac build/supervisor acceptance remains
an execution gate. Supporting publication is authorized; root owns review/posting
of the authenticated release. No paid fallback, account changes, raw private
logs or credential data are included. No model ranking is qualified.
