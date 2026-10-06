# Current development status

2026-10-06. **Stages 1–4 are implemented, automated verified and integrated.
Stage 5 knowledge and Stage 6 Actions/dashboard foundations are integrated,
with native/live/owner qualification still open.
Stage 7 Inbox is active; Stages 7–16 are incomplete; complete V1
delivery is not claimed.**

The [Product Vision](product/BRN_PRODUCT_VISION.md),
[architecture](architecture/overview.md), [invariants](architecture/invariants.md),
[roadmap](roadmap.md) and [development workflow](development/workflow.md) govern
delivery. Markdown/assets remain durable authority; WorkStore operational state
and the disposable retrieval index retain their existing roles. The owner's
[client-boundary amendment](architecture/overview.md#client-and-protocol-boundary)
permits thin future adapters around the six V1 core crates through workflow/
AppWorker. No MCP, daemon, HTTP service, extra database or remote work is in V1.

Integrated main is **`b5bcb0a7430de437533532f2bfab6a9df751509e` (PR69)**.
Complete retained original review, deterministic saved-body quotes, Rust-owned
Conflict/Knowledge/Action candidate identities, checked full Action replacement
baselines and AI Rewrite metadata protection are integrated. Exact approval and
original replay retain prior evidence and newer reviews. PR64 exact-head
37380762825 attempt1 passed all four protected Mac/shared jobs and Docs; overall
red retains unchanged Windows22 compiler blocks and both summaries matching PR63.
Fresh merged127 AI,11 Action callback/fence tests and52 fixtures passed. Main64
37382000386 attempt1 passed all applicable gates. Overall red retains the same
Windows22/22/14 complete compiler blocks/summaries and three Linux assertions/
backtraces, with raw ordering/thread-ID/duration differences retained.

The pushed immutable **`7c4f668de467721f728f242c8fac8d14606a6e44`** snapshot on
`codex/v1-inbox-recoverable-removal` implements removal/restore and recovery, but is
**unmerged**. Source-preservation correction is integrated: one approved Source proves exact
original preservation, with explicit confirmation still required. Failed/pending
analysis, consequence drafts and later derived edits are independent; item
disposition remains separate. Independent review/shared/native checks and exact-head
protected CI passed; fresh merged9 Store/13 Workflow/6 CLI tests+52 fixtures passed.
Captured-analysis recovery is integrated in the existing proposal-apply family.
Complete independent review and fresh shared1,397/0/8+52/native304 Workflow/models/0/7+285 Desktop/0/0
passed at10e95e2. Exact-head PR37387682980 attempt1 passed all protected checks
and Docs; overall red retains unchanged Windows22 compiler blocks/both summaries.
Fresh merged8 Store/14 Workflow tests+52 fixtures passed with one documented
subprocess ignore. Exact main37389066259 attempt1 passed applicable checks;
unchanged Windows/Linux failures and raw log differences remain visible. Main65 run37385458794 attempt1 passed applicable gates; unchanged Windows/Linux
failures remain visible. The A1 Store prerequisite is implemented on
`codex/v1-original-operation-records`: lean V15 records, byte-preserving legacy
reader, checked compact inventory/atomic streaming/visitor and historical analysis
fence. Complete independent review is clean; fresh shared1,412/0/8+52 and native
305 Workflow/models/0/7+285 Desktop/0/0 with V15 startup/restart checks passed at
48faf3f. PR67 merged normally at1ebff1a6f6099568c339e6b605bf819e1dbf6e6f after exact-head
protected CI/Docs passed; Windows22 complete blocks/both summaries match PR66.
Fresh merged14 Store/1 Workflow tests+52 fixtures passed at unchanged identity.
Main37394115918 attempt1 passed required Mac/shared+Docs and extra Ubuntu UI;
overall red retains Windows22/22/14 matching main66 and the same three Linux
assertions/backtraces, with IDs, duration and terminal-line placement recorded.
Lifecycle workflow/owner CLI is integrated in PR68: full independent review clean;
shared1,441/0/10+52 and native330 Workflow/models/0/9+285 Desktop/0/0,
11 commands/V15 startup/restart passed. Initial required Ubuntu test-import lint
failure was corrected; exact e8ab9de run37398589455 attempt1 passed four protected
Mac/shared checks and Docs, retaining unchanged Windows22 failure. Fresh merged
build/14 Store/15 Workflow/10 CLI tests/0/2+52 fixtures passed at unchanged clean
identity. Main68 run37399517358 attempt1 passed all applicable jobs; overall red
retains unchanged Windows22/22/14 and the same three Linux assertions/backtraces
with raw differences. Mac workspace tests738s versus436s remain recorded.
Native controls are qualified locally on codex/v1-inbox-copy-controls ate522813:
shared1,446/0/10+52, native330 Workflow/models/0/9+292 Desktop/0/0 and11commands/V15
restarts passed; complete/final-delta independent review clean. Documentation
review's legacy pending-retry claim was corrected: inspection/already-performed
settlement is supported; retry requires new-format intents.
PR69 merged normally atb5bcb0a with exact reviewed tree1abaa112 after run37401164589
attempt1 passed all four protected Mac/shared scopes and Docs. Windows22 full
blocks/summaries match PR68; actual red remains. Fresh merged build/5state+2widget
regressions+52fixtures passed at unchanged clean identity (02:06:51–02:06:56UTC).
Main69 run37402543978 attempt1 passed all applicable checks; unchanged
Windows22/22/14 compiler blocks/terminals and Linux3 assertions/backtraces match
main68, with raw order/IDs/timing differences retained. Unlocked owner observation
and Stage8 remain pending. Startup-cost witness preparation is test-only:
independent static review, debug/release compile, Workflow Clippy and default
release CLI build passed. Initial unused-result Clippy failure is retained;
no startup measurements or optimization are qualified. R6 facts are prepared separately.
Broader startup/backup cost remains separate. No semantic
completeness, native/live/owner acceptance or full V1 delivery is claimed. The full
V1 goal remains active. No owner original/private data was inspected or migrated.

The [original-copy record](work/active/text-email-inbox/original-copy-plan.md),
[correction plan](work/active/architecture-review-corrections/plan.md) and
[resumable checkpoint](development/checkpoint.md) retain detailed evidence.

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

Stage 7 integrates text/email intake and processing, selected-Source Action and
Knowledge drafts, exact Current/History supersession pairs, tentative conflicts
with shared Ask/CLI lookup, complete retained original review and fresh removal
preview. Source+confirmation recoverable original removal and exact Restore are integrated;
native controls are integrated after headless qualification; GUI/live/owner acceptance
remain separate. The historical snapshot stays unmerged. Evidence: [Inbox](work/active/text-email-inbox/plan.md),
[supersession](work/active/text-email-inbox/supersession-plan.md),
[conflicts](work/active/text-email-inbox/conflicts-plan.md) and
[original-copy work](work/active/text-email-inbox/original-copy-plan.md).

The CLI is owner-operated and exposes full shared-workflow authority. Its
approval, Save, completion and removal commands are not standing authorization
for an agent. Future external agents use read/propose unless the owner explicitly
delegates more authority. Durable Markdown and managed metadata are described in
the [vault format](architecture/vault-format.md).

## Historical integration evidence

The following results describe earlier checkpoints, not fresh verification of
the current correction branches. Current evidence is in the resumable checkpoint.

[PR55](https://github.com/ewq100/brn-rust/pull/55) merged at
**c5aaa6c4c96007e452151adb167964bf9e2b048a**, exact reviewed tree
`1477a02aed55a8932b23d41ec89540b37948e87c`. Inbox retains tentative unresolved
conflicts with two exact saved body proofs; shared Ask/CLI lookup keeps complete
pages and fresh staleness/ambiguity explicit. Native analysis opens the exact
Needs Review finding through editor recovery guards. No knowledge/Action changes
or original-copy deletion occur. Independent review is clean after reproducing
and fixing malformed managed metadata acceptance.

Final local1315shared/0fail/8ignores,285Desktop/0/0,268optional workflow/0/7,
116capability/0/0,52fixtures/two V14 restarts and relevant builds/Clippy passed.
Automatic exact-head PR run37335653379 attempt1 passed all macOS/shared lanes.
Windows22complete compiler blocks/two summaries and failure sources exactly match
qualified PR54 main. Normal merge satisfied current GitHub requirements. Fresh
merged9worker tests/52fixtures/two restarts passed; merged-main37336763915 attempt1 passed all four applicable Mac/shared jobs.
Overall red retains four baseline-identical informational platform failures. Native/live/owner acceptance stays pending. Evidence:
[conflict checkpoint](work/active/text-email-inbox/conflicts-plan.md).

Previous integrated supersession milestone:

[PR54](https://github.com/ewq100/brn-rust/pull/54) merged at
**eb36b331eb35c9cb5bc5b072060d89c39523ac87**, exact reviewed tree
`c7ab27798a59ab1fef4577baa099ab783083b9b6`. Inbox knowledge can propose a new
Current note plus an exact same-path History predecessor, preserving its identity,
body and provenance. Replay retains original proofs/newer review; stale or
ambiguous authority refuses. Whole-pair recovery, relationship rebuild and Undo
are verified. Source/original intake evidence remains retained.

Independent review is clean after reproducing and fixing exact footer occurrence
qualification. Final local checks passed1282shared/0failures/8ignores,
282Desktop/0/0,259optional workflow/0/7,106synthetic capability/0/0, all relevant
builds/Clippy combinations,52fixtures and two shipping V14 restarts. Exact-head PR
run37325847448 and additional dispatch37325852778 passed allMac/shared lanes.
Their red Windows/Linux diagnostics exactly match qualified PR53 production,
with byte-identical failure sources and CI configuration. GitHub accepted a normal
expected-head merge. Fresh merged39knowledge tests/0/1child ignore,52fixtures and
two V14 restarts passed. Exact merged-main run37327376228 passed all macOS/shared lanes; independent
comparison confirms the same four informational platform failures. Overall CI
remains red; no new supersession defect was found.
Native/live/owner acceptance remains pending. Evidence and next actions:
[supersession checkpoint](work/active/text-email-inbox/supersession-plan.md).

The preceding [PR53](https://github.com/ewq100/brn-rust/pull/53) at ceb3d0f9
qualified complete ordered saved-link targets. Its independent review, macOS/shared
CI, known platform failures and MacBook handoff remain retained in the
[Inbox plan](work/active/text-email-inbox/plan.md), including the
[original safe UI captures](ui/screenshots/2026-10-05/INDEX.md).

Stage6 implementation is integrated through [PR43](https://github.com/ewq100/brn-rust/pull/43).
Dashboard/identified Complete actual observation and eight safe original JPEGs
passed; composition/Rewrite/Ask native/live/owner acceptance remains pending.
Scenarios and integration evidence remain in the
[completed Actions record](work/completed/actions-dashboard/plan.md).

### Earlier Mac mini continuation

This retained handoff predates PR57–60. Its branch names and pending observations
are historical; use the current checkpoint above for continuation.

Owner resumed on Mac mini from verified production and documentation-only74725ad
handoff. Isolated work preserves unrelated checkouts. Complete V1 goal tracking
is confirmed active through trusted-user packaging, without a token budget.
Tooling PR56 is integrated at `03bb83a93c2df88e2699c72fb83e3ec0d78d41c1`:
atomic verification evidence, exact CI summaries, Markdown CI checks and portable
preflight. Four Mac/shared checks enforce strict freshness/admin requirements.
Fresh merged tooling checks pass; main run37341534594 remains pending.
[Resumable checkpoint](development/checkpoint.md) retains the current full goal.

At this handoff, branch `codex/v1-stage7-original-copy` started at this verified merge. Unresolved Inbox conflicts and shared Ask/CLI/native review were integrated.
Exact semantic qualification and recoverable original-copy removal follow.
Independent environment/tooling work is isolated on a separate codex branch;
lead retains architecture and integration.
See the [conflict checkpoint](work/active/text-email-inbox/conflicts-plan.md).
Qualified original-copy deletion follows this slice. BRN's frozen development workflow governs; completed
work/reviews/checks are reused. Historical evidence authorizes no additional live
calls, model downloads or private-data operations.

## Qualification and owner items

- Actual multilingual asset download needs the pending bounded owner permission.
  ONNX compatibility, EN↔ET quality, truncation, scoped restart/rebuild and tool/CLI
  parity remain open.
- The prior bounded Luna-only BRN app/provider qualification round used fresh human Connect,
  at most2 catalog calls+2 logical probes,18 completions maximum. Exact gpt-6-luna;
  no fallback, purchases or existing credential/private-vault inspection. After
  fresh sign-in and normal desktop quit, both permitted catalog calls succeeded,
  but the captured extraction established no usable IDs and did not retain raw
  catalog shape/length. Luna availability remains unverified;0logical probes/
  0completions were made. This round is exhausted; fresh owner permission is required
  for any new live call. Development/review may
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

For continuation: preserve the chat/checkpoints and task-owned branches;
use Apple Silicon macOS/Command Line Tools, pinned Rust1.98.1, locked dependencies,
protobuf, Bash/Python3 and an explicit existing canonical owned TMPDIR. Native
interaction needs an unlocked, awake session; fresh human Connect and additional
live calls need fresh owner authorization. Optional native features and shipping builds need separate
checks. [Verification](development/verification.md) and
[setup](development/setup.md) contain reproducible commands.
