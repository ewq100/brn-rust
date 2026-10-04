# Current development status

2026-10-04. **Stages 1–4 are implemented, automated verified and integrated.
Stage 5 foundations are integrated, with live/native qualification still open.
Stage 6 Actions/dashboard is active. Stages 7–16 are not implemented; complete V1
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
Evidence: [Actions/dashboard plan](work/active/actions-dashboard/plan.md).

## Latest integrated checkpoint

[PR37](https://github.com/ewq100/brn-rust/pull/37) merged
**3af4526f7795f749855f4c6165c0351facd56f38**, reviewed source tree2cb6175c.
Exact direct Complete now runs through App/AppWorker and CLI, with immutable
ordinary recovery, checked startup/retry reconciliation, uncertainty fences and
shutdown draining. Independent whole review found no blocking defect. Final
1093workspace/0failed/6ignored+233native/0failed/0ignored,52fixtures,both native
Clippy modes,shipping builds and startup2 passed. Post-merge28focused/0failed/
1ignored+52fixtures+startup2 passed,V11/exact bytes/zero credentials. Exacthead
bc2b737/run37225342866 passedMac3+UbuntuShared. Windows22 Unix errors exactly
match baseline logs and unchanged failing source; overallCIred, no shared Mac
defect. Fresh head/mergeability/rules guards passed; no requirement bypass. Main
37225862216 completed5success/4failure: Mac3+UbuntuCore/UI passed. All four actual
failed logs match baseline21442bf: Windows22/22/14 Unix errors before tests,
UbuntuNative10pass/3fail ExclusiveInstallUnavailable/TargetOccupied. Failing
source is unchanged; no shared Mac defect. Temporary stages after failed publication are bounded
individually but can accumulate; they cannot authorize completion. Native
Dashboard/Complete and owner Action acceptance remain pending.

[PR36](https://github.com/ewq100/brn-rust/pull/36) merged
**21442bfb0ff9e15705208f9c1fc224c7de90a5d1**, reviewed source tree8f0a2372.
The checked V11 identified-Complete Store foundation binds the full unfinished
Action/operation, preserves origin/content and settles the immutable receipt
atomically after its required exact publisher under Immediate write exclusion.
This checkpoint opens no client Complete producer. Independent complete review
found no defect. Fresh1072workspace/0failed/5ignored+233native/0failed/0ignored,
both native Clippy modes,shipping builds,52fixtures and startup2 passed,V11/exact
bytes/zero credentials. Post-merge12public tests+52fixtures+startup2 passed.
Exactheadb991/run37221597895 passedMac3+UbuntuShared; Windows22 unchanged Unix
errors remain informational and overallCIred. Main37222463218 completed5success/
4failure: Mac3+UbuntuCore/UI passed; actual logs repeat UbuntuNative10pass/3fail
ExclusiveInstallUnavailable and Windows22/22/14 Unix gaps. Source comparisons
found no shared Mac defect; no GitHub requirements were bypassed.

[PR35](https://github.com/ewq100/brn-rust/pull/35) merged
**9609b7fce0dd99bf3614a117213436a684485a92**, reviewed source tree98a9d606.
The sign-in URL opens on click; its read-only code supports selection/keyboard
Copy and an explicit Copy code button. Exact transient operation/prompt guards
refuse stale events. Independent complete/join reviews found no defect. Fresh
joined233native tests,both Clippy modes,shipping builds,52fixtures and startup2
passed; post-merge4widget tests+52fixtures+startup2 passed,V10/exact bytes/zero
credentials. The qualified replacement bundle now runs on the unlocked Mac.
The owner reports BRN connected and the link/code controls work: this changed
interaction has actual owner acceptance. Safe startup/Settings originals are
retained; authentication screens/codes are excluded.

Exact head62ed653/run37219579216 passed Mac3+UbuntuShared. Windows22 unchanged
Unix errors keep overallCIred; logs/source compared, no shared Mac defect.
Merged-main37220191827 completed5success/4failure: Mac3+UbuntuCore/UI passed;
actual logs repeat Ubuntu installer10pass/3fail and Windows22/22/14 Unix errors.
No GitHub requirement was bypassed. Environment remains AppleSiliconmacOS/CLT,
pinnedRust1.98.1,lockeddependencies,protobuf,Bash/Python3,canonical ownedTMPDIR.

[PR34](https://github.com/ewq100/brn-rust/pull/34) merged5f9033b9/tree3978cedd.
Whole typed Action Create/Replace and mixed drafts use exact proposal approval,
including source-free vaultless work. Full-record CAS protects file effects and
Applied recovery authority; joined settlement remains atomic. Native full-field
review preserves incomplete typing and captured grouped approval. Activity counts
approved Actions. Final1059workspace+229native+52fixtures and post22focused tests+
52fixtures+startup2 passed. Its exacthead Mac3+UbuntuShared and merged-main Mac3+
UbuntuCore/UI passed; the same four platform gaps remained. Native Action/owner
acceptance is pending; reproducible scenarios remain in the
[CLI](../crates/brn/README.md#manual-action-acceptance) and
[desktop](../crates/brn-desktop/README.md#manual-action-review-acceptance) contracts.

Earlier PRs16–33 are integrated with evidence in their plans/PRs. The actual fixed
Settings/model-consent dialogs were observed; safe original screenshots retained.

## Active slice and next work

codex/v1-dashboard-query is based on3af4526. The shared snapshot query implements
global state/date counts, filtered pages and exact dependency observations
through workflow/AppWorker and CLI. Active defaults to Open/Waiting/Blocked;
date comparisons use one explicit resolved civil date, frozen across pagination.
No action state/priority is inferred or mutated. Focused Store7/workflow3/CLI
process5+CLIunit7 tests passed; whole independent read-only review found no defect,
and reviewed source hashes remain unchanged. Fresh final verification passed
1106workspace/0failed/6ignored+52fixtures,233native/0failed/0ignored, both native
Clippy modes, shipping Desktop/CLI builds and two V11 startup/restart checks
(exact bytes, zero credentials). Exact-head CI/integration is next.
Acceptance and evidence remain
in the [Actions plan](work/active/actions-dashboard/plan.md).

Next: qualify/integrate shared Dashboard queries, then thin native Complete/new
approved follow-up controls and Stage6 AI/read tools within the same proposal
boundary. Inbox follows in roadmap order. WholeStage5/6 and V1 remain unfinished.

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
