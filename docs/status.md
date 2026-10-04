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

[PR33](https://github.com/ewq100/brn-rust/pull/33) merged
**7fed13295a4c9c2e633ef020e981042840f2246c**, source tree47f89319 equal to the reviewed
candidate. Exact latest head0882a30/run37214585531 passed MacCore/UI/Retrieval and
UbuntuShared. Windows repeated22 unchanged Unix API errors, so overallCI is red;
no GitHub requirement was bypassed. An initial Ubuntu test assumption about
macOS-only source coordination was independently reviewed and corrected without
changing production behavior.

Fresh post-merge **16 reference tests, 52 fixtures and two shipping startup/restart
checks passed**, V10/exact BOM/CRLF/Unicode bytes/zero credential files. Merged-main
run37215168696 completed **5 success / 4 failure**: Mac3+UbuntuCore/UI passed;
actual logs repeat Ubuntu native installer10pass/3fail and Windows22/22/14 Unix
errors in unchanged production paths. No shared macOS defect was found.

Earlier reviewed PRs16–32 are integrated; their plans/PRs retain exact CI and
verification evidence. PR32 fixes the actual missing Settings/dialog render layer.
Its automated widgets, build, startup and applicable CI passed; fixed Settings/
human login has not yet been exercised on the unlocked Mac.

## Active slice and next work

Branch codex/v1-action-application is based on7fed132. Whole typed Action
Create/Replace and mixed note+Action drafts are implemented locally through the
existing proposal boundary. Source-free work needs no vault. Full Action CAS is
checked before file effects and Applied recovery authority; joined settlement
retains transactional CAS. Mixed drift stays Uncertain/fenced. Native review
retains all fields, incomplete input and exact grouped captures; independent UI
review's valid group-approval refusal was reproduced and corrected.

Focused checks passed:12 recovery tests/0failed/1ignored crash child,23 exercised
crash points,2 mixed Finish/Restore tests/0failed/1exercised crash child,3 real
CLI+2 AppWorker scenarios,222 native unit/widget+7 CLI tests and native Clippy.
Complete independent review found no actionable defect and privately passed29
focused tests. A final real CLI regression reproduced Activity omitting Actions
(1pass/2fail); the shared summary now counts approved Create/Replace Actions.
Independent correction review passed7tests with no finding. The pre-correction
whole gate passed1059tests/0failed/5ignored+52fixtures. Fresh corrected gates
passed the same counts,178focused-native-workflow/0failed/4ignored,
229combined-native-desktop/0failed/0ignored,both native Clippy configurations,
shipping builds and startup2(V10/exact bytes/zero credentials). New application
tests qualify macOS ordinary receipts; an explicit non-Mac pre-admission refusal
witness awaits Ubuntu CI. Fresh Mac5application tests and test-targetClippy passed.
This candidate
is implemented and locally verified; publication and exact-head applicable CI
precede integration. Manual Action scenarios are in the
[CLI](../crates/brn/README.md#manual-action-acceptance) and
[desktop](../crates/brn-desktop/README.md#manual-action-review-acceptance) contracts.

Next: identified direct Complete, dashboard and related follow-up creation, then
the remaining Stage6 AI/read-tool path within the same approval boundary. Inbox
follows in roadmap order. Independent implementation/test work can run in parallel
behind fixed interfaces; integration follows dependencies. WholeStage5/6 remain
unfinished.

## Qualification and owner items

- Actual multilingual asset download needs the pending bounded owner permission.
  ONNX compatibility, EN↔ET quality, truncation, scoped restart/rebuild and tool/CLI
  parity remain open.
- Luna-only BRN app/provider qualification is authorized with fresh human Connect,
  at most2 catalog calls+2 logical probes,18 completions maximum. Exact gpt-6-luna;
  no fallback, purchases or existing credential/private-vault inspection. No new
  catalog/inference call has run. Development/review may use Sol/Luna, neverAstra.
- Prior unlocked native checks passed synthetic scoped reads, Unicode Save and
  acknowledged buffer recovery after full quit/restart. Safe original UI captures
  are retained in the [screenshot index](ui/screenshots/2026-10-04/INDEX.md).
  The unlocked Mac now shows fixed Settings and its optional-model dialog; three
  safe originals are retained. Download was declined without a model request.
  Fresh human sign-in is pending. Visual/native Action review, live inference,
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
