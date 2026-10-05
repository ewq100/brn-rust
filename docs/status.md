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

[PR45](https://github.com/ewq100/brn-rust/pull/45) merged as
**2a63fafb6f3a3897af5c5fc1d835b3043fe12763**, exact reviewed tree
`f70806f2cc73fd47600d9c5a7180391597963626`. Stage7 now includes the immutable
WorkStore V12 original-copy catalog plus owned exact text/email/Markdown/Teams
capture, private durable mirrors, shared AppWorker capture/read/list and thin CLI
adapters. Originals remain outside the vault/index and outside Current knowledge;
no processing, provider call, proposal or deletion is claimed yet.

Independent whole review of the copy-intake delta is clean. Its exact-head run
`37260989631` passed the applicable macOS Native UI, macOS Native Retrieval and
Ubuntu shared checks; Windows Core/CLI reproduced the known baseline API failures.
Post-merge run `37261533999` passed macOS Core/UI/Native Retrieval and Ubuntu
Core/UI plus Ubuntu Native UI. Ubuntu Native Retrieval has the same three known
synthetic-download failures; Windows Core/UI and Native jobs retain the known
baseline failures. No new shared macOS defect signature was found. The merge tree,
parents and local Git blobs were verified before the normal merge.

Stage6 implementation is integrated through [PR43](https://github.com/ewq100/brn-rust/pull/43).
Dashboard/identified Complete actual observation and eight safe original JPEGs
passed; composition/Rewrite/Ask native/live/owner acceptance remains pending.
Scenarios and integration evidence remain in the
[completed Actions record](work/completed/actions-dashboard/plan.md).

## Active slice and next work

The task-owned branch now advances Stage7 from the merged copy-intake baseline to
a small bounded processing queue. The next slice admits individual or bounded
batch jobs through AppWorker, joins cancellation and restart settlement, and
performs deterministic faithful text/Markdown conversion while retaining originals
and preserving the existing proposal/provenance boundary. It does not add a
provider, write the vault, delete originals or create a second queue framework.
See the short [Inbox plan](work/active/text-email-inbox/plan.md).

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
