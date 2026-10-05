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

[PR50](https://github.com/ewq100/brn-rust/pull/50) merged at
**fd24dee7dd640505a51d0978df4e8886fb9da47a**, exact reviewed tree
`441f1de50b0ec741cbc9c35fc687c54d065d65e9`. Stage7 includes private exact intake,
owned conversion, native Inbox capture/preview/processing and exact Source review,
then one complete saved Source analyzed through the existing owned Ask lane into
separate Action drafts. V14 retains whole Source/selection capture; generation
is transient and replay never repeats a retained turn. At most20 independently
reviewable drafts bind the exact Source. CLI exposes full Source proof, analysis
submission and retained inspection. No Action takes effect without exact approval;
originals stay retained. Knowledge/link/replacement/conflict and deletion remain
incomplete.

Independent complete read-only review is clean after a verified transient-generation
replay fix. Fresh default qualification passed 1,228 results/0failed/7ignored;
workspace format/build/Clippy passed. An old maximal receipt assertion in the
broader script was corrected, then all398 Workflow results passed freshly; prior
830 unchanged package results remain valid. Native workflow/models 237/0/6,
desktop 273/0/0, native Clippy, shipping builds, 52 fixtures and two V14 startup/
restarts passed. PR run37294640953 passed all three macOS lanes and Ubuntu shared;
Windows 22 compiler errors/two summaries exactly match PR49. Normal expected-head
merge satisfied fresh GitHub requirements without bypass. Verified merge parents/
identical tree and fast-forward, then eight worker tests, four CLI Inbox tests,
52 fixtures and two shipping V14 startup/restarts passed freshly with exact vault
bytes and zero credentials. Main run37295871888 at the exact merge passed all
three macOS lanes and Ubuntu Core/UI. Ubuntu Native retained 10 passed/3 prior
installer failures; Windows Core/UI/Native retained 22/22/14 compiler errors plus
two summaries each. Complete failed diagnostics match PR49 main; overall CI
remains red. Native Inbox observation/screenshots and owner acceptance remain
pending; no native analysis controls or live inference are claimed.

PR49 native Inbox and PR48 test-only CLI cancellation repair remain integrated
and qualified; their historical results remain in the active Inbox record.

Stage6 implementation is integrated through [PR43](https://github.com/ewq100/brn-rust/pull/43).
Dashboard/identified Complete actual observation and eight safe original JPEGs
passed; composition/Rewrite/Ask native/live/owner acceptance remains pending.
Scenarios and integration evidence remain in the
[completed Actions record](work/completed/actions-dashboard/plan.md).

## Active slice and next work

`codex/v1-inbox-knowledge-analysis` continues from merged `fd24dee7`. Extend the
same immutable capture with explicit knowledge_and_actions purpose, preserving
old Actions canonical bytes/questions. A thin opt-in protocol prepares one
Current knowledge Create per call with a new UUID and exact selected-Source
quotes; the existing typed proposal protects these bindings through review and
exact approval/apply/Finish. Reuse the owned WorkTurn and shared 20-draft group
limit. Independent review is clean after validated orphan-binding and recovery
identity corrections. Fresh default checks passed 1,254/0/8; native Workflow/
models 247/0/7 and Desktop 273/0/0 passed, with Clippy, shipping builds, 52 fixtures
and two exact-byte V14 startup/restarts. Seven default skips are crash helpers;
one needs a case-sensitive APFS fixture. Published-head CI/integration and manual
acceptance remain pending; the complete Stage7 is not claimed.
Following Stage7 deliverables are links, replacement/history/conflict resolution,
safe copy deletion and native semantic controls. See the
[Inbox plan](work/active/text-email-inbox/plan.md). No new live/download scope or
private/original data operations are being used.

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
