# Retained evidence investigation: approved Sources

## Owner continuation: repair empty ChatGPT discovery — 2026-10-08

After normal product sign-in and the offline GUI qualification recorded in PR88,
the owner selects continuing until model discovery works and the bounded live
qualification can proceed. Baseline is merged main `90d20909de8794dbe574322dfc68e2b5a5363047`;
reuse its clean linked checkout/target on `codex/fix-chatgpt-model-discovery`.
Both GUI and ordinary CLI discovery return zero selectable models. A temporary
metadata-only diagnostic confirms zero models before filtering; no credential,
header, token or raw provider body is printed or copied. Determine whether the
request route/version causes the empty server catalog, then add a focused failing
transport witness, correct the smallest boundary and verify existing AI/auth tests,
shipping builds and actual GUI discovery. Preserve pinned Rig, ordinary product
authentication, explicit model choice and no-fallback authority. No optional
asset download, private data, account switching, UX work or release.

The selected lead retains its current configuration and owns the coupled repair;
one fresh read-only reviewer will independently check a meaningful final candidate.
At most one reviewer is active. Reassess after contradictory evidence rather than
stacking speculative fixes. Discovery diagnostics are limited to six additional
metadata requests, each with a 30-second deadline, and zero completion probes.
The existing single Luna Medium investigation / nine model turns / eight tool
rounds / 180-second cancellation deadline remains the only live usefulness trial.
No inferred IDs, route fallback, hidden retry or extra effort condition is allowed.
Token/spend telemetry remains unknown. Record exact gates and final observations
in the repair PR and existing qualification receipt.

Diagnosis: the same normal account/transport returned zero raw models at BRN
version `0.1.0`, one hidden model at protocol `0.99.0`, and ten raw/seven visible
models (including exact `gpt-6-luna`) at published Codex protocol `0.161.0`.
Omitting the version failed; the newer public SIWC endpoint refused this existing
Rig Codex OAuth route and is not adopted. No new sign-in, route fallback or
credential export was used. The root cause is coupling the catalog compatibility
query to BRN's independent package version. Reuse Rig's authenticated Models
wire and transport; change only that query value to an explicit qualified
protocol constant, preserving account/caller headers and strict validation,
visibility/order, cancellation and safe error policy. Temporary metadata diagnostics
are removed from the candidate. A focused synthetic-transport regression failed
against the old request URL before the correction (one failed, 140 filtered).
The first test build exposed a HeaderValue assertion API mistake; corrected the
test before observing that meaningful failure. Broader AI tests and live final
candidate qualification follow; no inference has started.

Final local correction checks: `cargo test -p brn-ai --lib --locked --offline`
passed 140 with one existing ignore (including the new regression, auth/catalog
failures/cancellation and exact Luna Responses/tool/image contracts). AI all-target
Clippy with `-D warnings`, Cargo formatting, whitespace and 590 local Markdown
links passed. Fresh shipping combined native desktop and native CLI builds passed
without test support. Final ordinary CLI discovery returned the seven visible
models including `gpt-6-luna`, with no diagnostics compiled in. Only the isolated
qualification bundle was updated; its previous main executable is retained locally.
Independent review, hosted gates, actual GUI model discovery/selection and bounded
live usefulness/consequence review remain pending at this candidate checkpoint.

## Owner continuation: CI performance, integration and live qualification — 2026-10-08

The owner now selects fixing the long Mac CI run, merging the eligible dependency
and follow-up in order, then continuing GUI and live-AI usefulness qualification.
This supersedes the earlier no-merge/no-live gates for this bounded continuation.
PR87 is the dependency at `255dcf1`; PR88 is the reviewed retained-evidence slice
at `936dff4`. All four required checks plus documentation passed on both heads.
Main protection requires those four contexts and an up-to-date branch, including
administrators; preserve that policy. Windows remains a known informational
Unix-portability failure, outside this performance correction.

The lead owns the coupled fix/integration inline with the selected configuration;
one fresh read-only reviewer checks the complete performance correction before
integration. Reuse the existing linked checkout/target and runtime/fixtures.
No additional implementation helper, dependency, store schema, recovery-format
change, stress-test removal or workflow suppression is selected. Profile the
existing complete 16 MiB recovery/Undo test, compare its unchanged assertions
under the smallest supported test-build correction, then run affected checks
and inspect exact-head CI before merging. Reassess if profiling contradicts the
hypothesis or required checks fail; do not mask failures with retries/skips.

After integration, verify actual main identity/checks and use an isolated
synthetic GUI/data workspace. Exercise import/read, offline Source approval,
Quit/restart, retained-evidence investigation, review/approval and another
restart. Use existing product authentication only; never export credentials or
upload private vault data. No optional model download, account switching, new
paid account or release is selected. Begin usefulness qualification with one
approved-Source `plural.eml` scenario on the already authorized Luna Medium
condition if its actual route is available; the broader historical paired
comparison is separate. Aggregate ceiling for this fresh qualification is one
top-level investigation, at most nine model turns/eight tool rounds through the
existing runtime, a 180-second CLI cancellation deadline, no retry/fallback and
no additional effort conditions. Model token/spend telemetry is not currently
exposed as an enforceable aggregate cap; record it as unknown rather than
claiming a numerical token/cost limit or an unlimited campaign. Record
unsupported/auth/locked-desktop conditions honestly; continue independent
headless work without claiming GUI or usefulness qualification from builds.

Initial profile of the unchanged maximum-asset test shows its active thread in
repeated journal JSON/base64 encoding/decoding and unoptimized byte/slice loops;
it is CPU-bound rather than waiting for a provider. SHA-256 is already separately
optimized in the test profile. A five-second sampling record is retained locally
at `/private/tmp/brn-ci-perf-sample.txt`; durable before/after measurements and the
actual correction follow here and in PR88's receipt. Existing hosted run 37783868867
records 27m core Mac (22m29s default tests), 22m45s combined/native (16m36s workflow
step) and 4m34s UI. No assertion or feature coverage is being reduced.

Fresh exact-name baseline on the same host/checkout/target passed one complete
maximum-asset approval/recovery/Undo test in **396.82 seconds**, with 408 other
entries filtered. It was sampled on the active CPU thread while processing exact
journal JSON/base64; the main test thread merely waits for completion. Test-only
`opt-level=1` is the single experimental correction, with explicit
`debug-assertions=true` and `overflow-checks=true`. This optimizes the generic
byte loops across both the Store and workflow's envelope codecs, preserving
their source, format, checks and full-size witnesses. Production/dev profile,
lockfile, dependencies, runtime and CI test commands stay unchanged. Compare the same
focused test first, then qualify the final test profile broadly and in hosted CI.
The identical focused test with optimization level 1 passed in **51.84 seconds**
(7.65 times faster, 86.9 percent less test execution time), after 66 seconds of
one-time compilation. Verbose rustc commands confirm optimization level 1 and
debug assertions on for the Store and workflow test executable. One fresh
read-only reviewer found no actionable correctness/scope defect; the advisory
stale checksum-only comment was corrected. Fresh default workspace tests passed 1,635 with 17 existing ignores; native
workflow passed 401 with 15 existing ignores; native retrieval passed 15.
Combined native desktop/widgets passed 319 tests plus seven CLI tests. Default
workspace and combined native all-target Clippy passed with `-D warnings`;
formatting, whitespace, 590 Markdown links and 18 tooling tests passed. All
commands used the pinned locked/offline toolchain and one sequential Cargo target.
Shipping builds at source `936dff4` remain relevant because runtime sources and
the dev profile are unchanged; final hosted CI qualifies the new test profile.

The synthetic GUI/live workspace is
`/private/tmp/brn-retained-qualification-bi3q58kf` with separate data/vault/results
directories. Local-only `ai status` through the standard protected
`BRN-simple.credentials` route reports ChatGPT and Copilot disconnected, with no
selected model/effort; no cache contents were printed/copied. GUI tooling reports
the remote Mac locked and unable to unlock automatically. An owner unlock
request is pending. Normal product sign-in and actual model discovery are live
prerequisites; neither is inferred from Codex's own connected account. Continue
the CI correction/integration while those prerequisites are unavailable.

PR87 was merged with the ordinary permitted merge strategy after freshly
confirming its exact head, all four required checks and strict protected-main
policy. Result: `e65cd510988568c8f0df3e0a9ac1bc7a5afe535e`; its tree is identical
to checked source `255dcf1889e15e6236513ebe53292992a59e0fad`. Main push run
[37790769005](https://github.com/ewq100/brn-rust/actions/runs/37790769005) is pending.
PR88 will be brought onto that main before its final exact-head hosted gates.

Hosted follow-up at `075b65a` found an additional cache defect before merge.
The native workflow test step passed in 7m38s versus 16m36s historically, but UI
spent 8m58s + 3m04s compiling test executables (tests passed in 5.18s, 0.34s and
2.63s). The cache restored an old exact key and ended `Cache up-to-date`, so it
did not save the new-profile artifacts. Add the root `Cargo.toml` hash to the
existing platform/architecture/lane key using GitHub's built-in `hashFiles`.
Reuse the action's existing compiler/toolchain/lockfile handling and cache save;
no custom cache mechanism or test command/coverage change. This makes profile
changes invalidate the primary key; the first refreshed build still pays its
compilation cost. Recheck the cache correction independently and inspect the
new exact-head hosted run before merge. Reuse the already verified unchanged
Rust/profile tree and shipping executable evidence. A fresh read-only review
confirmed the exact omission/save behavior against the pinned action source and
found no actionable issue. New explicit key hashes also change fallback prefixes:
first builds are cold, then dependency artifacts can save. Workspace test
executables still rebuild under the existing policy; do not promise otherwise.

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
