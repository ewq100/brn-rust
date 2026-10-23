# Mac headless real-model precise edit

Selected by the owner's 23 October instruction, separate from the closed vacation
sessions and the cloud admission attempt (zero model calls). Baseline:
`6c86583d709e13e7fc3c5070008d49c824856d07`, including PR224 and Rig0.45.
All four required baseline-main checks passed; optional Windows failures remain.

## Bounds and gates

One initial production CLI request, at most one retry only after a concrete cause
is identified and corrected. Existing stricter limits remain: at most four tool
rounds, 180-second CLI deadline, initial synthetic input at most8KiB, six attempts
per Tallinn day and stop around5% remaining allowance. Tool rounds are not a money
cap. No paid allowance increase, credential copy/change, real vault, native window,
unlock, security/power change, external messages, release or deployment.

Use the existing subscription and available `gpt-6.1-sol`, medium effort, with no
model fallback or helper. Lead owns the isolated branch, build slot, single account
actor and integration. One fresh read-only Sol reviewer will inspect the complete
code/evidence candidate. Stop on uncertain dispatch, missing admission or exhausted
bounds rather than relaunching. The old scheduled campaign remains paused.

## Qualification

Preflight observed the existing ChatGPT login and read-only model catalogue with
medium effort supported. Native account allowance was92% used (8% remaining).
No model turn ran during discovery. The baseline CLI is built headlessly using the
pinned Rust1.98.1 toolchain and locked dependencies. Initial offline build lacked
`rig-harness-protocol`; a locked fetch is preparation, not an upgrade.

The fresh single-scenario [driver](evidence/mac-live-20261023/scenario.py) uses
existing `init`, `configure`, `note-create`, `thread-start`, `ask`, `candidates`,
`operation`, `read`, `apply` and keyed `undo` commands. It does not inject candidates
or mock a provider. Baseline bytes and a separate sentinel are checked after each
phase. Each CLI invocation is a new process. Results and actual attempt reservation
belong to the adjacent session record. Live, review, Apply, Undo, persistence,
independent review, exact-head CI and resulting-main CI remain separate gates.

## Observed result

**Live precise generation and headless CLI Apply/Undo/persistence PASS.** Exactly
one request ran; no retry. The existing subscription's selected model was
`gpt-6.1-sol` at medium effort. Read-only catalogue admission exposed this model
and medium effort. No separate upstream resolved-model field was exposed.

The request completed in10.667seconds with two tool rounds and three model turns:

1. `read_records` read the exact target Note at version1.
2. `prepare_note_edit` used `start:null,end:null`,12 expected UTF-8 bytes and10
   replacement bytes. The runtime's unique-exact-quote route produced one candidate.

The redacted production trace is in [ask](evidence/mac-live-20261023/commands/ask.json).
It intentionally omits raw expected/replacement strings; the candidate request and
independent original/expected fixture bytes establish the exact resulting edit.
The model answer's phrase “no records were changed” is not used as evidence:
Run, messages and candidate bookkeeping were written normally. Both canonical
Notes remained byte-for-byte unchanged at version1 before explicit Apply.

The inspected candidate had one write: the correct Note/version and exact expected
body, with unchanged title, protection, confirmation, import and supersession data.
Apply produced exactly `cobalt boats` → `teal boats`, including unchanged surrounding
Unicode paragraphs, as version2. A new CLI process read that exact result. Undo
restored exact original text as version3; a fresh process read it. Repeating Undo
with the **same owner key** returned the same compensation operation and retained
version3, matching the documented keyed-replay contract. This does not claim that
an unkeyed Undo has the same contract. The separate sentinel remained exact at
version1 throughout. All invocations reopened storage in a separate process.

[Session and independent fixture hashes](evidence/mac-live-20261023/session.json)
and [saved command results](evidence/mac-live-20261023/commands) retain each phase.
Synthetic vault and frozen executable stay outside Git. Binary SHA-256:
`d01f2498f3135ca15cd103a3f61bd5b63c3180b3a4e8f279b322e8e8688114e9`.

Provider-reported usage:19,499 input tokens,246 output tokens,12,544 cached input
tokens; cached input is a reported subset, not added again. Monetary cost is
unavailable. Account allowance before/after was92% used (8% remaining) in the
available primary window, ordinary usage allowed; secondary unavailable. No
allowance increase, reset redemption or additional live request occurred.

## Evidence-script failure and correction

The initial script treated unpaged `candidates` as a paged object. After the live
request and both unchanged-Note checks passed, inspection raised `TypeError`.
No Apply had executed. The corrected script accepts the documented unpaged list;
its separate `inspect` phase resumed from retained output and validated the exact
candidate before Apply. The original failing script hash, cause and recovery are
recorded in the session. This was a qualification-script defect, not a BRN product
failure. No production source, runtime prompt, schema or dependency was changed.

## Commands and gates

The retained scenario is task-specific orchestration of existing shipping commands,
not a new runtime framework. Reproduction needs a fresh root and an explicitly
admitted new live attempt; these commands do not authorize another account call.
Use `EXISTING_AUTH_FILE` only as a path to the already configured login, without
copying it. No token/auth content is retained.

```sh
python3 docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/scenario.py prepare --binary FROZEN_BRN --root FRESH_ROOT --auth-file EXISTING_AUTH_FILE
# Only after separate current allowance/model admission and attempt authorization:
python3 docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/scenario.py live --binary FROZEN_BRN --root FRESH_ROOT
python3 docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/scenario.py inspect --binary FROZEN_BRN --root FRESH_ROOT
python3 docs/work/active/precise-edit-ui-qualification/evidence/mac-live-20261023/scenario.py verify --binary FROZEN_BRN --root FRESH_ROOT
```

The actual shipping request uses `--json --data-dir DATA --timeout 180 ask
--thread THREAD --prompt PROMPT --budget 4 --trace full`; no `--edit-note`
write grant was supplied. Review-first delegation and helper-off are explicit.
Initial combined synthetic note/sentinel/prompt input was under8KiB; packaged
runtime guidance is additional provider input, visible in actual token usage.
The tool-turn budget is not a financial cap. The client deadline was not reached;
no upstream cancellation guarantee is inferred.

Local checks passed:17 precise-edit regressions (zero failures/ignored), strict
all-target CLI/runtime lint, ordinary CLI build, workspace formatting, Python
compilation, Markdown links and overview consistency. [Exact commands/results](evidence/mac-live-20261023/local-checks.json) retain logs and the disclosed initial
offline-build log-retention limitation. Independent review is recorded separately.
Exact-head CI, normal merge and resulting-main CI remain pending. The four required
baseline-main checks passed; all three optional Windows lanes failed and remain
visible. No claim of full-platform success is made.

## Remaining gaps

This qualifies one real-model synthetic precise edit, not general model reliability.
No BRN window, screen/mouse control or unlock was used. Actual Mac buttons, focus,
accessibility, visual review, file-picker behavior and owner UI acceptance remain
unqualified. Existing cloud-controlled native evidence remains separate. No second
live request is needed after a documentation-only integration of these results.
