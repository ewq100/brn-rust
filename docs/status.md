# Current development status

2026-10-05. **Stages 1–4 are implemented, automated verified and integrated.
Stage 5 foundations are integrated, with live/native qualification still open.
Stage 6 Actions/dashboard implementation is integrated, with acceptance open.
Stage7 Inbox is active; Stages 7–16 are incomplete; complete V1
delivery is not claimed.**

The [Product Vision](product/BRN_PRODUCT_VISION.md),
[architecture](architecture/overview.md), [invariants](architecture/invariants.md),
[roadmap](roadmap.md) and [development workflow](development/workflow.md) govern
delivery. Markdown/assets remain durable authority; WorkStore operational state
and the disposable retrieval index retain their existing roles. The owner's
[client-boundary amendment](architecture/overview.md#client-and-protocol-boundary)
permits thin future adapters around the six V1 core crates through workflow/
AppWorker. No MCP, daemon, HTTP service, extra database or remote work is in V1.

## Integrated behavior

Stages 1–2 deliver exact Markdown Save, generation-bound unfinished-edit recovery,
conflict/exclusive-copy protections and removal of legacy production architecture.
Legacy/original data is untouched. Evidence:
[Save](work/completed/simple-save/plan.md) and
[legacy removal](work/completed/legacy-removal/plan.md).

Stage 3's completed authorized live round made **10 logical probes / 14 completion
attempts**. Fresh human connections succeeded. ChatGPT gpt-5.5 passed effort/read
tools/tiny image; Copilot gpt-5.3-codex passed read tools but reversed image colors.
Both observed hosted web search with official SQLite URLs; native citation
metadata remains unqualified. Copilot gpt-5.5 Chat explicitly refused; there is
no fallback. This historical round authorizes no additional calls. Evidence:
[provider capabilities](work/completed/provider-capabilities/plan.md).

Stage 4 delivers typed whole proposals, exact before/source bindings, full edits,
temporary comments, owned Rewrite, individual/captured-group approval, recoverable
application, Activity, practical Undo/Trash and explicit Finish/Restore repair.
AppWorker/CLI and desktop share those rules. Late work preserves newer typing;
unknown effects fence current evidence. Knowledge changes only after exact
approval. Evidence and manual scenarios:
[Proposal Core](work/completed/proposal-core/plan.md).

Stage 5 delivers managed UUID identities, exact provenance, Current/Source/History/
All retrieval, reliable session timestamps, saved links, derived relationships,
stable-link proposal preparation, native scoped/evidence browsing and basic Needs
Review findings. Current is default. Retained evidence survives source/index/
session changes; duplicate/incomplete identities stay explicit. Search clients
consume semantic workflow evidence rather than SQLite passage IDs. Independent
reviews reproduced and verified corrections to proof bounds, case-sensitive
identity/alias handling, stale native captures and scoped paging.

The multilingual MiniLM-L12 profile pins five exact assets at revision
2c4055b12046f11709e9df2c122e59ffbdc2f900 (135,392,488 bytes) in a separate fresh
folder. Synthetic bundle/vector/installer tests passed; actual download/inference
and EN↔ET quality remain unqualified. Ask preserves source quotations and requests
the question's language; actual language compliance remains unqualified.
Relationship requests currently rederive the saved vault observation; the retained
scale probes took 6.108 s / 10.026 s for 8,192 proofs / 5,000 notes. Evidence:
[knowledge foundations](work/active/knowledge-foundations/plan.md).

Stage 6's integrated checkpoints deliver checked V10 Actions, immutable approved
origins/full replacement baselines, exact stored review members, joined Store
settlement/recovery, shared paged Action reads and explicit reference/dependency
validation. Dependencies and parent graphs are separate checks; new knowledge
references require captured or same-draft proof. Completed records cannot reopen.
Evidence: [Actions/dashboard record](work/completed/actions-dashboard/plan.md).

## Latest integrated checkpoint

[PR49](https://github.com/ewq100/brn-rust/pull/49) merged at
**fab3da64275f02c5498006701b8bb289e27e9a96**, exact reviewed tree
`3285fdd6e779323676f0c6b4d44797e045f222ad`. Stage7 now includes private exact
capture, V13 bounded conversion, immutable Source preparation/approval/recovery,
and native Inbox capture, full original/preview inspection, processing controls
and binding-preserving Source review. Originals remain retained. Semantic
knowledge/Action/link consequences and safe deletion are still incomplete.

Independent complete read-only review is clean after reproduced consumption,
retry and readonly-review fixes. Fresh default verification passed 1,210 tests
(seven ignored helpers), format/build/all-target Clippy and startup; native checks
passed 229 workflow/model tests (six ignored helpers), 273 desktop tests, native
Clippy, 52 fixtures, shipping builds and two V13 startup/restarts. Exact-head run
`37287735015` passed all three macOS lanes and Ubuntu shared. Windows Core's full
normalized diagnostic blocks match PR48; overall CI remains red under the owner
platform policy. Fresh requirements permitted normal expected-head merge without
bypass. Merge parents/tree and fast-forward were verified; fresh nine Inbox
state/widget tests, 52 fixtures and two shipping startup/restarts passed,
preserving exact vault bytes and zero credentials. Main run `37288829290` at the exact merge passed all three macOS lanes
and Ubuntu Core/UI. Ubuntu Native retains three prior installer failures
(10 passed / 3 failed); Windows Core/UI/Native retain the prior 22/22/14 compiler
errors plus two summaries each. Complete failed-job logs match PR48 after
incidental normalization; overall CI remains red. Computer use reported the Mac locked; Inbox native observation,
screenshots and owner acceptance remain pending.

PR48's test-only CLI cancellation repair remains integrated and qualified;
its main run passed all macOS lanes and Ubuntu Core/UI with the known remaining
Ubuntu Native/Windows failures. The active Inbox record retains historical source
and race-reproduction evidence.

Stage6 implementation is integrated through [PR43](https://github.com/ewq100/brn-rust/pull/43).
Dashboard/identified Complete actual observation and eight safe original JPEGs
passed; composition/Rewrite/Ask native/live/owner acceptance remains pending.
Scenarios and integration evidence remain in the
[completed Actions record](work/completed/actions-dashboard/plan.md).

## Active slice and next work

The Action-analysis slice is implemented and independently reviewed on
`codex/v1-inbox-action-analysis`, from merge `fab3da6`. One approved saved Inbox
Source uses the existing owned Ask and exact proposal boundary. V14 retains a
whole Source/selection capture; generation is transient and replay never repeats
a retained turn. At most20 separately reviewable Action proposals bind the exact
Source; real effects still require approval. CLI exposes full Source proof,
analysis submission and retained inspection. No original deletion/completeness
is inferred. Fresh default checks passed 1,228 tests (seven ignored helpers);
native workflow/models 237 (six ignored), desktop273, default/native Clippy,
52 fixtures, shipping builds and two V14 startup/restarts passed. The broader
workspace run exposed an old five-field receipt-test expectation; its corrected
ordinary/grouped shapes and all workflow tests passed freshly. Hosted CI and
integration are pending. Manual/live/native acceptance remains pending; no new
account/model calls occurred. Knowledge/link/replacement consequences and safe
deletion follow. See the
[Inbox plan](work/active/text-email-inbox/plan.md).

## Qualification and owner items

- Actual multilingual asset download needs the pending bounded owner permission.
  ONNX compatibility, EN↔ET quality, truncation, scoped restart/rebuild and tool/CLI
  parity remain open.
- Luna-only BRN app/provider qualification is authorized with fresh human Connect,
  at most2 catalog calls+2 logical probes,18 completions maximum. Exact gpt-6-luna;
  no fallback, purchases or existing credential/private-vault inspection. After
  fresh sign-in and normal desktop quit, both permitted catalog calls succeeded,
  but the captured extraction established no usable IDs and did not retain raw
  catalog shape/length. Luna availability remains unverified;0logical probes/
  0completions were made. No further calls in this round. Development/review may
  use Sol/Luna, neverAstra.
- Prior unlocked native checks passed synthetic scoped reads, Unicode Save and
  acknowledged buffer recovery after full quit/restart. Safe original UI captures
  are retained in the [screenshot index](ui/screenshots/2026-10-04/INDEX.md).
  Fixed Settings and its optional-model dialog were observed; three safe
  originals are retained. Download was declined without a model request.
  Human sign-in and updated-link/code acceptance passed on the unlocked Mac.
  ScreenCaptureKit intermittently prevents computer-use capture/interaction;
  safe screenshots are retained when available. Visual/native Action review, live inference,
  broader IME/accessibility/chooser and owner acceptance remain open.
- Pinned Rig0.43's streaming invalid-tool branch unconditionally prints its partial
  assistant choice to stderr before hooks, observed with synthetic tool arguments.
  No existing hook/log setting suppresses it; rejected model arguments can appear
  in technical logs. This concrete upstream logging gap needs a qualified correction
  before trusted-user packaging. No live/private data was used in this witness.
- Synthetic crash/widget tests do not establish physical power-loss durability,
  other-volume support or release readiness. Windows Unix API and non-Mac native
  installer gaps remain visible. Upstream block0.1.6 has a future-compiler warning.
- Release/public distribution, additional live/download scope, purchases and
  original/private-data inspection/migration still require applicable permission.

Pending acceptance does not block later safe implementation when it is not a
dependency. No original data was inspected or migrated.

For transfer to the Mac mini: preserve this chat/checkpoint and task-owned branches;
use Apple Silicon macOS/Command Line Tools, pinned Rust1.98.1, locked dependencies,
protobuf, Bash/Python3 and an explicit existing canonical owned TMPDIR. Native
interaction needs an unlocked, awake session; fresh human Connect is only for the
bounded live round. Optional native features and shipping builds need separate
checks. [Verification](development/verification.md) and
[setup](development/setup.md) contain reproducible commands.
