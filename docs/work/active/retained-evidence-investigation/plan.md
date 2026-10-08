# Retained evidence investigation: approved Sources

## Selected outcome and baseline

On 2026-10-08 the owner selects moving to retained-evidence AI investigation,
including already approved Sources, with further UX tuning deferred. Implement
this bounded first P3 slice through existing workflow/AppWorker and native
Propose notes. Baseline `255dcf1889e15e6236513ebe53292992a59e0fad` passed all four
required CI checks plus documentation in PR87 run37777829649. The informational
Windows Unix-portability failure remains separate and unresolved. PR87 is draft
and unmerged; this follow-up uses `codex/retained-evidence-investigation` and a
stacked PR based on `codex/p2-email-docx-intake` to preserve its checked candidate.
The existing linked worktree is reused; no other checkout or data is reset.

## Scope and behavior

- Explicitly select a retained email/document reading and its exact Source.
  Both Draft and Applied extracted Sources can initiate the existing collection
  investigation with a selected provider/model/effort. Reading alone never calls AI.
- An Applied Source binds to the exact successful approval journal for its
  current review stamp and full draft, retaining the original approved Draft
  stamp expected by the existing dependency validator. Missing or ambiguous
  matching receipts, rejected/applying/uncertain states and changed evidence
  refuse rather than invent a binding or choose the latest receipt.
- Retain the complete selected extraction, image assets and distinct occurrence
  mapping. Never silently substitute text-only saved-Source investigation.
- New Knowledge and Action proposals remain independent review work. An already
  Applied Source is not created or approved again. Existing group approval can
  approve just the dependent proposals after checking their Source prerequisite.
  New admission and application recheck exact originals, Source text/assets and
  dependency receipts; old approval does not make changed files fresh evidence.
- Restart/replay opens retained work without reconversion or another model call.
  Existing frozen selection, navigation, cancellation and late-reply guards stay.

This slice does not add large-context/range reads, structural proposal revision,
new formats, converter/runtime/dependencies, schema changes, Windows support or
UX polish. Existing explicit input/resource limits and extraction gaps remain.
No live provider trials, optional model downloads, private vault access, merge or
release are selected by this task; offline model hooks verify mechanics.

## Reuse and approach

**Reuse/adapt:** `App::intake_analysis_binding`, immutable `IntakeSnapshot`,
`InboxIntakeBinding`, checked `proposal_apply` journals, the existing
`validate_intake_dependency`, Rig-backed owned investigation lane, proposal tools
and exact application/recovery. Inspection shows the downstream dependency
validator already accepts exact Applied Source receipts; the binding loader and
guided UI currently reject that state. Extend those entry points rather than
add another analysis mode, store or authority path. Existing toolkit widgets
only need their functional enabled-state/message updated.

The lead owns implementation and documents inline with the selected model.
One fresh read-only independent reviewer checks the complete change before PR;
no delegated implementation or additional helpers. Reassess a material evidence
or receipt mismatch rather than weaken existing guards to broaden eligibility.

## Acceptance and verification

1. Real worker: import plural EML, retain/approve Source only, restart, mint its
   exact binding, investigate with deterministic hooks and preserve one image /
   three occurrences. Prepare useful Knowledge/Action drafts without changing
   Source bytes, assets or creating a duplicate Source.
2. Review and approve just the dependent proposals as one group; assert both
   Applied receipts, expected note/Action content and unchanged Source evidence.
   Restart and replay retained analysis with a hook that panics if called again.
3. Changed Source bytes, image bytes or original, missing/mismatched receipt and
   noneligible Source state refuse before new inference or authoritative effects.
   Existing pending-Source journeys remain supported.
4. Native state: after offline Source approval/restart, one explicit gesture
   loads an image-preserving binding and emits one request with frozen selection.
   Changed selection/navigation or repeated/late replies do not submit it.

Run focused red/green worker/native-state tests, affected all-target Clippy,
default/native workflow and full native desktop tests as appropriate once on the
final candidate. Reuse unaffected helper/converter evidence; no converter changes.
Run documentation/format checks and inspect final hosted checks for the stacked
PR. GUI observation and actual model usefulness remain separate pending gates;
UX deferral does not qualify either. Record final identities/results in this
plan and the PR receipt.

## Implementation and review evidence — 2026-10-08

The candidate adapts the existing binding loader, dependency validator and guided
entry point. Applied binding uses the checked receipt's approved Draft stamp;
admission initializes the checked file adapter after restart. Fresh unique
identity validation reuses the internal inventory proof scan, including during
admitted application when public Current reads remain fenced. The owned runtime,
store schema and converter are unchanged.

Focused red/green exposed three concrete boundaries: the original entry point
refused Applied Sources, fresh restart had no initialized Source-file adapter,
and a public identity resolver correctly refused reads during application.
The fixes reuse the checked adapter and internal proof scan rather than weaken
the public fence. Four new worker tests passed, covering retained images and
occurrences, dependent-only approval, replay, tampering, duplicate identity,
missing receipts and ineligible states. The final broad gate rechecks these
after adding the explicit one-Source/two-consequence inventory assertion.

The native restart/approval state journey now also checks the Applied binding,
frozen selection/effort, changed-generation refusal and duplicate-reply refusal.
Combined desktop/native widget tests passed 319 tests plus seven CLI tests with
`native-ui,native-retrieval,native-test-support`; the intercepted provider command
makes no live call. These are headless results, not GUI qualification.

One fresh read-only reviewer checked the complete dirty candidate and both new
files against baseline `255dcf1`, including exact receipt binding, proof scans,
recovery/replay and native intent guards, with no actionable findings. A later
Clippy-only nested-condition collapse preserves the same short-circuit matching.
The first lint attempt caught that style warning; the corrected default workspace
all-target Clippy passed. Broad final gates and hosted exact-head results follow
in this plan/PR receipt; they are not implied by the focused checks.

Owner scenario after provider qualification: import/read an email, save and
approve its Source offline, quit/restart, select that retained item and explicitly
Propose notes. Review the proposed Knowledge/Actions with retained pictures and
originals, then approve only the new consequences. The Source must remain single
and unchanged. Live usefulness and GUI observation for this new path are pending;
further UX tuning is explicitly deferred. No merge/release is authorized.

### Gate receipt and next action

Lead host: arm64 macOS 27.0.1, pinned Rust 1.98.1. All Cargo calls are sequential
in this checkout's `target/intake-ui`, with `CARGO_INCREMENTAL=0` and private
synthetic `TMPDIR=/private/tmp/brn-p2-fixtures`, using `--locked --offline`.
The dirty candidate is reviewed against `255dcf1`; the follow-up PR pins the
published commit and owns final exact-head hosted evidence without introducing
a documentation-only rerun solely to record its own final CI identity.

| Gate | Observed result before draft publication |
| --- | --- |
| Four new real-worker tests (`cargo test -p brn-workflow --features native-retrieval --lib approved_retained_source --locked --offline`) | Passed; all four also passed in the still-running default workspace suite, including final no-duplicate inventory assertion |
| Combined desktop (`cargo test -p brn-desktop --features native-ui,native-retrieval,native-test-support --locked --offline`) | Passed: 319 unit/widget + 7 CLI |
| Default Clippy (`cargo clippy --workspace --all-targets --locked --offline -- -D warnings`) | Passed after one style correction |
| Format, diff whitespace, Markdown links, provider/intake retirement | Passed |
| Tooling (`python3 -m unittest discover -s scripts/tests -v`) | Passed: 18 |
| Default workspace (`cargo test --workspace --locked --offline`) | In progress: new four tests and existing paired P2 journey passed; maximum-size asset recovery still active |
| Default/shipping native builds, native workflow/models, native Clippy, end-to-end fixtures | Pending final sequential local batch and hosted CI |
| Independent complete-candidate review | Passed: no actionable findings |
| Native GUI observation and actual model usefulness | Pending qualification; no live calls or downloads |
| UX tuning | Explicitly deferred |
| Merge/release | Not authorized |

Publish the stacked draft while final local checks and hosted checks execute in
parallel on separate hosts. Finish the local batch, inspect exact-head hosted
results, fix relevant regressions if any, and update the PR receipt with the
final gate state. Windows portability remains separate; do not suppress its red
job or claim full P3 acceptance.
