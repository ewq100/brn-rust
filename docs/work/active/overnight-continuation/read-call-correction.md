# One bounded correction of malformed read-tool JSON

Selected independent next P3 slice while citation review fixes/qualification
continue in the lead checkout. Baseline main6a63173fb6e33b4ba239ad6aade23bde976ee051;
reuse /Users/evokessler/repos/brn-p3-work-budgets, branch
codex/p3-read-call-correction. Owner authorizes continuous small complete V1 work
until05:00UTC; at most2 helpers, no recursion, one Cargo across checkouts. Lead
retains selected model/effort. No live calls needed for this behavior; shared
campaign stays14/16(7each), one Luna/one Sol remaining. No GUI/private data/accounts/
paid fallback/download/port/release. All interactive acceptance stays in the single
morning task owned in the lead checkout, not a new UI checklist.

## Outcome and reuse

An ordinary Ask or retained Inbox investigation can recover from ONE syntactically
non-JSON call to an allowlisted read tool within its existing model-call, tool-round,
deadline and cancellation limits. No extra response allowance is added. Typed
schema/domain/backend failures keep existing normal tool-result handling. Unknown
or malformed proposal/mutation tools refuse. Strict Rewrite and standalone visual
completion remain unchanged. Earlier valid drafts/partials remain recoverable.

Inspect/reuse pinned Rig0.43 InvalidToolCallContext/Action::retry, existing RoundHook,
ToolRounds and normal streaming collector. The pinned streamed rollback path
abandons the complete malformed response before tool dispatch and synthesizes
"not executed" results for valid peers; prove this with real-Rig mixed-batch tests
in both orders. Do not edit the vendored one-line credential/error safety patch,
add a second loop, manual JSON repair, automatic transport/top-level retry,
provider/model fallback, dependency or schema. Native web capture remains parked:
probe annotations lack real source text; pinned hosted-call budgets are not ready.

## Fixed behavior

- Apply to standard investigation modes, including proposal-capable normal Ask/
  Inbox paths (normal product Ask supplies DrainedProposals and is ActionReview).
  A proposals=None-only change would not deliver the product outcome.
- Allow at most1 correction per entire investigation, only reason MalformedArguments
  (non-JSON wire arguments), tool name among the seven registered read tools and
  membership in both available_tools/allowed_tools. UnknownTool never corrects.
- Static feedback only: explain valid JSON/schema and that no tools in the rejected
  response executed. Never echo raw args, parser error, history, provider IDs,
  evidence or supplied unknown names into feedback/events/errors/logs.
- Keep max_turns=max_tool_rounds+1. Malformed response consumes an existing call;
  it admits no tool round and dispatches no sibling read/proposal. Second malformed
  response, malformed final-slot call or cancellation permits no correction request.
- Final response slot must be tool-free even when malformed input left unused
  admitted rounds. Refuse before dispatch using existing ToolLimitReached mapping;
  do not create a last-slot draft/read with no remaining final answer allowance.
- Make the consumed call visible without claiming provider/token telemetry: count
  **model requests admitted** at existing on_completion_call, after cancellation
  guard, from pinned HookContext.turn/CompletionCallEvent.turn (verify one-based
  semantics). Keep field model_turns for compatibility but update documented/runtime
  copy from completed responses to admitted requests; emit same cumulative count
  again when valid tool-round admission changes. This counts admission to Rig's
  completion boundary, not HTTP requests, provider reasoning turns, tokens or spend.
  Keep counts monotonic and <=max_tool_rounds+1. Existing workflow initial0 event,
  stale/generation/frozen-budget checks and owner UI buffers remain unchanged.
- Check cancellation at completion boundary, retain outer workflow deadline/drain
  and unchanged strict visual/Rewrite policies. No late provider request/effect.

## Acceptance and ownership

One bounded implementation helper owns brn-ai chat hook/policy and meaningful
provider_formats/provider_stderr tests, plus directly affected budget documentation/
Desktop budget label/state tests. Root owns shared status/morning task, review,
composition, final gates/CI/integration. No helper Cargo until explicit root grant.

Use real Rig with distinct queued mock SSE and full request interception across
three existing routes: malformed read -> valid read -> final (3calls/1read); second
malformed fails (2calls/0read); unknown/malformed mutation/Rewrite/visual unchanged;
mixed valid proposal+invalid read in both orders has zero dispatch for rejected
response, correction can then complete, earlier-turn draft retained exactly; typed
schema errors ordinary continuation; partial text once; unique secret markers absent
from feedback/stderr/local events/errors; limit1 malformed->final succeeds but
malformed->read refuses before dispatch, limit2 malformed->read->final succeeds;
final-slot invalid/no further request; cancel/deadline before correction; progress
includes malformed admission, stays frozen/monotonic and counts actual admitted calls.

After implementation obtain one fresh complete independent read-only review,
validate/fix findings, final affected AI/workflow/native/CLI/Clippy/shipping gates,
actual required CI, normal protected merge and resulting-main verification. Compose
with citation review without replacing the authoritative morning task. Keep pending
GUI acceptance separate from implemented/verified/merged.

## BLOCKED / NOT QUALIFIED — 2026-10-09 pinned wire evidence

The isolated candidate at HEAD5219761c90b5f9973ed50f281d0009b4eba7353d plus
unstaged changes **must not merge or ship**. Implementation stopped when the
required real-provider mixed-batch witness falsified the reuse assumption. The
candidate hook/policy, tests, runtime copy and README edits are preserved only for
reviewable continuation; they do not establish implemented/qualified behavior.
No independent review, affected Clippy/full AI/native/workflow gates or CI ran.
No live calls, dependency/vendor edits, custom parser, staging, commit, push or PR.
Cargo was released after both bounded test processes exited; no helper processes
remain. Lead owns preservation and next independent slice selection.

Pinned rig-core0.43.0 source under the local registry
`src/providers/openai/responses_api/streaming.rs:762–800` closes a completed
FunctionCall at line799 with `out.close_pending(index, IfMalformed::Drop)`.
The response-end path at903–907 also drops pending malformed arguments. Thus a
syntactically non-JSON completed read never reaches AgentRun's
`InvalidToolCallReason::MalformedArguments`/RoundHook seam on Responses. A valid
proposal peer survives and dispatches in either ordering. The ChatGPT Responses
owner Ask path and Copilot Responses path both reproduce this defect; Copilot
Chat uses `IfMalformed::Fail` except explicit Length in
`src/providers/openai/wire/chat.rs:1311–1317` and does reach the rollback hook.
The registry source is unmodified; vendored rig-agent's credential/error safety
patch is unchanged.

The existing AgentRun mechanism itself is reusable: `run/mod.rs:901` increments
the model-call index before `CallModel`, engine `on_completion_call` receives that
one-based index, and streamed Retry abandons the surfaced invalid response before
dispatch. It cannot supply the required three-route product outcome while the
Responses decoder discards the invalid call first. The first surfaced invalid
call controls the existing hook; later peers in an abandoned stream are drained
without separate classification. No bespoke pre-parser or second loop was added.

Reproduction uses the granted pinned environment (recorded with the logs),
`CARGO_TARGET_DIR=$PWD/target/budgets`, existing physical synthetic TMPDIR and no
provider network/account work:

- `cargo test -p brn-ai --lib --locked --offline read_call_correction_tests`:
  exit101,0passed/7failed. Required mixed proposal/non-JSON-read witness observed
  both the earlier retained draft and the rejected peer draft. Other Responses
  cases completed with empty text instead of entering correction/refusal.
  [First output excerpts](read-call-correction-failed-test.log) explicitly retain
  the separate unqualified pending-stream cancellation-fixture failure; that
  failure is not evidence of the cancellation boundary.
- `cargo test -p brn-ai --lib --locked --offline pinned_wire_malformed_observation -- --ignored --nocapture --test-threads=1`:
  exit0,1manual diagnostic passed (155filtered). The diagnostic had begun before
  the final park instruction. Both Responses routes dispatched1proposal in both
  orders; Chat dispatched0. All used2mock requests. This confirms incompatibility,
  **not candidate acceptance**. [Exact diagnostic output](read-call-correction-wire-observation.log).
- Direct pinned rustfmt completed after an initial PATH-only `rustfmt` lookup
  failed; `git diff --check` and Markdown links passed before the park note.

Next dependency decision: qualify a maintained Rig provider release/fix that
surfaces completed malformed Responses tool arguments to the existing hook, or
explicitly authorize a separately reviewed narrow upstream/provider correction.
Re-run unchanged mixed-peer witnesses in both orders across all three routes
before treating this slice as Build-ready. Do not ship Chat-only recovery or
admitted-request copy as a substitute for normal owner Ask. Repair the pending
cancellation test transport before claiming that acceptance criterion. The lead
must separately decide whether to resume or replace this parked slice.
