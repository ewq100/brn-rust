# Proposed architecture change plan — 2026-10-07

## Selected CI repair and supervision — 2026-10-08

The owner asks to “Babysit the CI and fix it.” This selects focused CI diagnosis,
corrections, relevant offline checks, push and supervision of PR87 through a final
result. UX remains deferred; no merge, release, private data or live model action.
Baseline is clean `43ae39f7a325e2eafdc776dc001a362b45f955e3` on
`codex/p2-email-docx-intake`. The lead owns fixes and integration inline; obtain
one bounded read-only independent review of a meaningful final correction.
Stop/reassess a material product/platform change rather than weaken evidence,
credential safety or exact approval to make a check pass.

**Diagnosed Mac cause and focused correction:** diagnostic native job
[113304619387](https://github.com/ewq100/brn-rust/actions/runs/37775332268/job/113304619387)
on `a3fc8b21e82d5482e069522dc0baae3b34d07e26` shows all three approvals
succeeding: validation 0.607 seconds; members 1.005, 8.025 and 3.843 seconds;
three Applied receipts at 13.481 seconds. The test had panicked at 10.147 seconds
while the third approval was running, then worker shutdown drained it successfully.
The workflow finished 389 passed, 1 failed, 15 ignored (986.80 seconds total).
This establishes an aggregate test-budget mismatch, not a stuck or failed member.

The correction gives a validated captured group the existing ten-second test
allowance per sequential atomic approval (30 seconds for this three-member case).
It retains one absolute deadline across unrelated events; invalid groups and
all ordinary operations keep ten seconds. Domain validation bounds membership
to 1–64 before the duration multiplication. No production timeout, approval,
dependency guard, fixture, receipt assertion or test selection changes.
Two native-workflow group-related tests passed (403 filtered, 19.59 seconds);
native-workflow all-target Clippy passed after the correction. One fresh read-only
independent reviewer found no actionable code defects and confirmed the hosted
trace satisfies its diagnosis condition. The lead retains the selected model;
reviewer configuration inherits it, with one active reviewer maximum and no
delegated implementation. The diagnostic run's still-running Mac core job may
be superseded by the fix push; final qualification requires a new exact-head
run on both Mac workflow graphs.

Run 37764888681 completed with docs, Ubuntu and native UI green; both Mac workflow
jobs repeated the single-fixture `ApproveProposalGroup` timeout. A focused run
with new test-only member timings passed in 19.35 seconds total: three-member
groups took 5.37 seconds (single) and 5.68 seconds (plural) on the lead host.
Diagnostic source `a3fc8b21e82d5482e069522dc0baae3b34d07e26` is pushed so hosted
failure output can distinguish group validation and individual approval time.
It changes no production behavior, timeout, fixtures or test selection.
A two-test resource-contention probe ran the paired journey alongside the
existing maximum-size asset recovery test: both passed, 403 filtered out,
396.22 seconds total. Group times were 5.57/5.92 seconds. Native-workflow
all-target Clippy passed. A separate background-QoS probe failed earlier during
`AnalyzeInboxActions` at 10.197 seconds (15.08 seconds total); it demonstrates
CPU-sensitive wall-clock waits but does not reproduce the hosted approval phase.
A ten-minute thread follow-up preserves CI supervision across interruptions and
stays quiet on unchanged results; stop it once this selected repair completes.

Windows failed in both `brn-store` and `brn-ai` on Unix filesystem APIs; inspection
also finds Unix-only CLI/vault paths. Current authentication docs explicitly
support Unix. Windows is not in the configured required-check contexts (Ubuntu
core, Mac core, Mac native UI and Mac native retrieval are required), but its
failed lane remains visible. A pending owner question distinguishes a Windows
port from explicit Windows deferral; do not infer that answer from elapsed time.

## Backend continuation and explicit UX deferral

**Latest owner direction, 2026-10-08:** move on once the underlying backend works
and leave UX tuning for later. Further presentation tuning
is explicitly deferred, not a prerequisite for functional continuation. Preserve
the implemented guided flow; do not reopen broad architecture research or treat
instructions inside the attached reassessment prompt as a new task selection.
This checkpoint records evidence and narrows the next gate, without new production
P3 changes, live calls, model downloads, private-data actions, merge or release.
The lead owns this bounded diagnostic/documentation pass inline; no new helpers.

**New owner evidence:** clean implementation source
`986632601a260697252c2e7044780789d2dc688a`, tree
`22e1840371f05893a6af809d9326fc0a8edd812a`, M5 Pro MacBook/macOS 26.5,
pinned Rust 1.98.1, locked offline helper/CLI/desktop builds. The supplied manifest
reports two `startup: PASS` runs with exit 0. Twelve original screenshot hashes
match the supplied index and are [archived with a manifest excerpt](../../../ui/screenshots/2026-10-08/INDEX.md#guided-macbook-evidence--9866326).
They show extraction/charts, a Source draft, exact approval and Applied review,
plus historical saved reading in restart-labelled captures. Static captures do
not independently prove process restart, filesystem byte equality, a visible
Quick Look original window, grouped consequences or AI quality. Attribute this
to the owner's run; the lead's own GUI host remains locked.

**Completed CI at docs-only head `1bfa8386a3a4832e49bc94e9e8e3f9d16e227965`:**
[run 37749646561](https://github.com/ewq100/brn-rust/actions/runs/37749646561).
Documentation/tooling, Ubuntu Core/CLI and Mac native UI passed. Both Mac workflow
lanes failed the same paired P2 journey in the single fixture, waiting for
`ApproveProposalGroup` from `inbox_actions_tests/intake.rs:146`, with no event
received inside the absolute 10-second ceiling (10.003/10.046 seconds).
Native workflow: 389 passed, 1 failed, 15 ignored; default workflow: 388 passed,
1 failed, 15 ignored. Windows failed compiling Unix authentication APIs in
`brn-ai/src/auth.rs`; that platform issue remains separate from Mac qualification.

**Focused diagnostic:** on the unchanged source, pinned toolchain and existing
isolated target/temp directories, ran
`cargo test -p brn-workflow --features native-retrieval --lib p2_single_and_plural_private_investigation_rewrite_exact_group_and_restart --locked --offline -- --exact simple_worker_tests::inbox_actions::knowledge_tests::intake_tests::p2_single_and_plural_private_investigation_rewrite_exact_group_and_restart --nocapture`.
One paired test passed, both fixture phases completed, 404 filtered out,
19.10 seconds total. Local full parallel suites previously passed too. Resource
contention is a hypothesis, not a demonstrated cause. No production fix,
timeout inflation, skip, fixture shrink or green-CI claim follows from this pass.

**Next functional actions, in order:**

1. Trace grouped approval on the failing Mac CI conditions; distinguish slow
   checked persistence/approval from test scheduling. Verify the actual fix or
   justified test-harness correction on both Mac workflow graphs. Do not bypass
   exact Source/dependency checks or required integration gates.
2. Prepare a small P3 specification for AI investigation of retained evidence,
   including an already approved Source and its checked pictures. The current
   pending-Source-only restriction is a functional capability gap, not UX polish.
   Reuse the existing snapshots, investigation runtime and exact proposal review;
   preserve images and original evidence instead of silently falling back to text.
3. Qualify useful proposed Knowledge/Actions and grouped consequences with explicit
   model selection and a separately selected bounded trial. Owner screenshots
   establish no live AI result. Keep merge/release and full P2 acceptance separate.

Cosmetic spacing, clipped table text, export-comment presentation and the technical
approval layout are deferred. A rendering issue that hides material evidence or
misstates approval effects would still be a correctness defect.

## Selected guided Inbox implementation — 2026-10-08

The owner approved the proposed scope below with “yes” after the explicit next-step
question. Implement it on PR87 from clean head `69cab825d0f49ac56042c4e1a951ea3863854106`;
no new merge/release, private-data, model-download or AI-trial selection is implied.
The lead owns desktop state/native orchestration, integration and documentation.
Two bounded helpers cover exact saved-extraction discovery and safe readable
presentation; CI investigation is read-only before a focused diagnostic/fix.
Maximum two active helpers plus lead; independent review replaces a completed helper.
Reuse the existing helper, snapshots, AppWorker, proposals and approval/recovery.
Stop/reassess only a material data/authority/renderer prerequisite outside this scope.

Execution checkpoints:

- Reproduce import-list staleness; add automatic inventory/selection and bounded
  extraction orchestration with late-reply/input guards.
- Reopen exact retained versions from the item list; render readable, inert email/
  document text and checked images with material gaps visible.
- Replace the default long technical view with focused stages and explicit Source/
  AI actions, proposal review and existing exact approval; disclose details/tools.
- Diagnose Mac CI timeouts, run relevant shared/native/shipping checks, obtain
  independent review, rebuild a fresh local candidate and update PR87 handoff.

Implementation source is `986632601a260697252c2e7044780789d2dc688a`, pushed to
[PR87](https://github.com/ewq100/brn-rust/pull/87). The fresh evidence below is a
local implementation checkpoint; owner usability/P2 acceptance and merge remain
pending. Prior dated P2 and owner MacBook evidence below retains its attribution.

### Guided Inbox implementation evidence and handoff

The default Inbox now occupies the available document area, with retained originals
and focused reading/proposed-note tabs. Import acknowledgement refreshes inventory,
selects the exact import and starts local extraction. A newly imported/selected
item appears at the top of the sidebar even beyond the FIFO page. Back navigation
reads the exact retained item independently of the current inventory page; an
explicitly chosen saved extraction version can be restored without reconversion.
One saved version is labelled historical evidence; multiple versions require a
choice and are never sorted/selected as a latest approval baseline.

The maintained toolkit reader renders decoded adapter text and tables, with named
attachment selection, exact occurrence images and ordinary-language omission
warnings. Markdown/HTML image URLs and all link opening are inert. Actual images
come only from the validated extraction cache. Original inspection names/opens
the selected supported attachment, with no parent-email fallback for a stale node.
Raw Markdown/MIME/limits/hashes live under Evidence details. Paste, copy management
and batch/analysis controls remain secondary; advanced tools use a separate view
with Back. Evidence EditorStates are readonly at creation, including while hidden;
programmatic synchronization remains supported. Source title/path inputs survive
item switching; retained Source edits use its exact review rather than ignored
preparation inputs.

An explicit **Propose notes** gesture can retain a pending extracted Source, load
its private binding and submit one existing investigation with the frozen
provider/model/effort/generation. Late navigation or changed selection prevents
that later submission. **Save as Source only** retains the review draft offline.
Neither reading nor draft preparation approves changes. Source, Knowledge and
Action cards use the existing exact review, comments/Rewrite and group approval;
originals remain retained. No new approval ledger, converter, schema, dependency,
provider route, larger-context behavior or binary cleanup was introduced.

**Supported-state limits:** the guided image-preserving investigation requires a
pending extracted Source. Already approved/rejected/uncertain Sources do not
silently fall back to text-only investigation. Plain-text/Markdown copies can be
saved as Source and use the existing saved-Source tools. The UI explains these
limits before submission. To qualify AI after an offline Source-only approval,
use a distinct public-fixture import and investigate before approving its Source;
this handoff authorizes no new trial call, account change or optional model download.

Fresh checks on the lead's arm64 Mac, macOS 27.0.1, pinned Rust 1.98.1, all offline:

| Check | Result and limit |
| --- | --- |
| Default workspace `cargo test --workspace --locked --offline` | 1,629 passed, 17 ignored. Ran before the final desktop navigation/render repairs; unchanged Store/workflow passed here, and subsequent desktop changes were covered by the final full native suite and focused journeys below. |
| Default and native Clippy | Workspace `--all-targets -D warnings` and desktop native-ui, combined native-retrieval, and combined native-test-support variants passed. |
| Final desktop native suite | `cargo test -p brn-desktop --features native-ui,native-retrieval,native-test-support --locked --offline`: 319 unit/widget + 7 CLI tests passed. Includes hostile visible resource/link tests and genuine toolkit widgets; this is not an unlocked GUI usability observation. |
| Guided real-worker journeys | Five focused tests passed, also included in the native suite: public plural EML import/read, offline Source draft/approval/restart; exact saved version choice; stale/misbound replies and retry; frozen explicit AI admission intercepted before execution; 26th import/back navigation. No live provider command was executed. |
| Native workflow/retrieval | `cargo test -p brn-workflow --features native-retrieval --lib --test models --locked --offline`: 390 unit + 7 model-boundary tests passed, 15 ignored; native retrieval lib/model-download tests: 15 passed. No optional model assets downloaded. |
| Helper and shipping builds | Helper 22 tests passed; locked offline default workspace, native-retrieval CLI and native-ui/native-retrieval desktop builds passed. |
| Fixtures / launcher / tooling | 52 end-to-end assertions, macOS launcher checks, 18 tooling tests and current Markdown-link check passed. |
| Packaged lifecycle | Fresh bundle with the exact tested desktop/helper hashes and isolated empty data/vault: two bundled-binary startup/shutdown runs passed. |
| Independent review | Read-only review found no remaining actionable defect after page/attachment/resource/intent and routing/readonly fixes. Reviewer ran no Cargo or live checks. |
| Native GUI / AI / models | CUA reports execution Mac locked and automatic unlock unavailable; one unlock request sent. No new GUI screenshots/acceptance, provider trial or optional model download. |

Native widget checks first exposed an advanced-renderer stack overflow; separating
the small mode dispatcher from the large guided frame resolved it without larger
stacks or timeouts. The next check exposed unpainted evidence input remaining
editable; initializing readonly state fixed it. Original management tests now
explicitly open the shipping secondary controls; detailed legacy tests exercise
the separate advanced renderer, while new tests cover the default guided view.

The old Mac CI failure did not identify the blocked command. Test-only diagnostics
now report safe command/event variants, operation ID, caller, elapsed absolute
10-second ceiling and last event, with single/plural phase markers. Both local
parallel feature-graph paired P2 checks passed; no timeout inflation, skip,
fixture reduction or speculative resource-serialization fix is claimed.
The later completed final-head results and focused reproduction are recorded in
[the backend continuation checkpoint](#backend-continuation-and-explicit-ux-deferral).
At this earlier source checkpoint, native UI had passed while other checks were
still running and Windows had failed. The source run is
[37748654513](https://github.com/ewq100/brn-rust/actions/runs/37748654513).
Prior Windows Unix-API build failures remain outside this bounded Mac UI scope.

Fresh isolated setup: `BRN-local-builds/P2-guided-9866326-20261008-111609/BRN Guided Inbox.app`
on the execution host, with `README.txt`, `MACBOOK_AGENT_PROMPT.txt`, public
`Fixtures`, hashes in `build-manifest.json` and retained verification logs.
The MacBook prompt pins this exact source commit, builds the helper/CLI/desktop
sequentially, binds new data/vault/log paths and specifies the offline GUI/restart
scenario. No old workspace/account is copied. The owner's next acceptance pass
should report this new commit separately from the older `64db36e` MacBook results;
full P2 acceptance, AI usefulness, release and merge remain ungranted.

## Owner MacBook verification and proposed Inbox UI — 2026-10-08

**Task boundary:** record the owner's verification and screenshot review, and
present a simpler existing Inbox flow before implementation. This is a bounded
native UI proposal, not selection of P3, another converter, a new approval system,
full P2 acceptance or merge/release approval. The lead owns the documentation and
proposal inline, with no additional helpers, provider calls or model downloads.
Stop at the concrete proposed scope until the owner selects implementation.

**Tested candidate and attribution:** [PR87](https://github.com/ewq100/brn-rust/pull/87),
exact commit `64db36ec42d42a9d005797d7c7b2950b187f07e9`; Apple M5 Pro MacBook,
macOS 26.5. The following is the owner's supplied local report, not a fresh lead
execution on that machine. Five inline screenshots were reviewed directly;
their names, provenance and capture-access limitation are recorded in the
[dated index](../../../ui/screenshots/2026-10-08/INDEX.md).

| Gate / observation | Owner result and precise limit |
| --- | --- |
| Local build and packaging | Both locked builds and packaging with the bundled helper passed. |
| Headless lifecycle | Two startup/shutdown runs passed. |
| Native launch and extraction | Actual GUI launch, completed conversion and chart rendering passed. The bundled helper produced 8 source nodes, 1 image asset, 3 occurrences and 4 gaps from `plural.eml`. Source nodes include MIME containers/alternatives; this is not eight attachments. |
| GUI Quit/restart | Both imported copies remained listed after full Quit/restart. This proves the reported list persistence, not reopening retained extraction or completing proposal effects. |
| Usability | Owner could not understand Inbox without step-by-step guidance. Import appeared to leave zero originals until manual Refresh; unrelated stages and technical controls competed for attention. Usable guided intake is not accepted. |
| Still unverified in this run | Retained extraction reopened after restart; Source approval; grouped consequences; AI usefulness. |
| Live/model activity | No AI trials or optional model downloads. Do not reset or infer the earlier campaign's unknown aggregate usage/allowance. |
| Acceptance/integration | Bounded local verification only. Full P2 acceptance and merge approval were explicitly withheld. |

**Screenshot review and code check:** the supplied views show a raw Markdown
editor with upstream export comments and asset hashes, full quota/usage numbers,
unnamed MIME containers with disabled preview buttons, repeated occurrence
locators, conversion gaps, batch selection, cleanup/Restore and Source-path inputs
in one long view. The full-window capture gives a large empty generic chat panel
space while the intake content and next step remain crowded. The chart and
conversion counts are visible; the screenshots do not establish later approvals,
restart extraction retrieval or model usefulness.

The manual-refresh report is consistent with the current state code:
`inbox_state.rs::apply_inbox_event` handles a validated `InboxCaptured` event by
updating `capture_result` and the notice, without refreshing/updating the inventory.
`refresh_inbox` obtains a new page but also invalidates current selection/preview.
The proposed fix must show/select the committed import without losing unrelated
input or accepting a stale inventory reply. This is a code-supported explanation,
not an additional runtime reproduction on the MacBook; no fix is made here.

**Recommended layout:** a persistent Inbox item list plus one focused content area
with readable stage labels and one primary next action. Opening Inbox uses the
available work area; the generic chat composer is not permanently shown beside
the reader. Evidence can remain open beside a selected proposal when useful.
Back navigation and switching items preserve settled work and unfinished input;
the stages describe progress rather than forcing a one-way wizard.

| Stage | Primary view and action | Details available on demand |
| --- | --- | --- |
| Import | One **Import email or document** button. A successful retained capture immediately appears and is selected; local extraction starts with readable progress and Cancel. No mandatory checkbox/batch/Refresh sequence for one file. | **Paste text** retains existing text/Markdown/Teams entry. **Select multiple** exposes existing bounded batch processing. Retry appears for an actual failed operation. |
| Read email/attachments | Decoded mail fields, readable body, named attachments and actual images in their qualified source/occurrence context. DOCX opens as reflowed readable text/tables/images. A visible summary says, for example, **forecast.xlsx retained, not read** and **Some document visuals/layout may be missing**. Primary action: **Propose notes and actions**. | **Open original** for supported exact retained originals; **Evidence details** for raw extraction, full gaps, MIME tree, IDs, hashes and selected/consumed limits. No invented attachment names or image associations. |
| Review proposed notes | Separate cards for **Original email (Source)**, proposed knowledge (including any Current→History change), and related Action. Show complete proposed content, destination, reasons and clickable supporting evidence. Title/path suggestions are visible and editable before preparation; the owner need not type UUIDs or a path just to read. Comments/Rewrite/Reject use existing proposals. Primary action: **Review selected changes**. | Source-preservation prerequisites are shown in plain language. Exact raw/diff/asset inspection remains accessible. No broader split/retarget/regroup editor or new destination inference engine is included. |
| Approve | A compact summary names every selected note, asset and Action, including required Source preservation, with complete before/after inspection available. **Approve selected changes** submits the exact displayed versions through existing group approval. Completion links to saved notes/Actions and shows what remains pending. | A conflict or partial application shows applied/stopped/remaining members and links to Activity/recovery. A changed version requires renewed review; the sequence is not presented as all-or-nothing. |

Selecting **Propose notes and actions** is an explicit AI request using the configured
provider/model/effort and existing budgets. Preparing/retaining the Source draft
is orchestration of existing commands, not Source approval. If AI is unavailable,
reading remains usable, **Connect AI to propose notes** explains the next step,
and **Save as Source only** offers the existing offline Source-review path. No
automatic sign-in, fallback, provider call or fabricated AI draft is introduced.

**Progressive disclosure:** raw MIME nodes, hashes, locators, schema/version labels,
quota counts and Markdown source move into Evidence details. Unprocessed attachments,
missing meaningful visuals, failure reasons and blocked approval remain visible in
ordinary language. A small status indicator exposes Cancel while extraction/AI is
active; hitting a budget surfaces a relevant explanation and the detailed limits.
Copy cleanup lives under the selected item's **Manage original** menu, with the
existing eligibility checks and explicit confirmation. Repair controls appear
when unfinished work requires attention and remain reachable through Activity;
they are not a permanent bank of disabled buttons. Approval still retains originals.

**Implementation scope if selected:** adapt desktop Inbox state/orchestration and
native Inbox/analysis/review presentation, reusing the shared AppWorker commands,
immutable extraction records, exact occurrence joins, images/original-preview
facility, comments/Rewrite and group-approval machinery. Prefer the existing
toolkit's maintained readable text/Markdown facilities where qualified (the pinned
toolkit already provides `TextView::markdown`; its untrusted-content/image behavior
still needs qualification). A narrow read-only workflow/Store lookup may be needed
to expose a captured item's already-saved batch/snapshot references to the item list:
the present reader accepts a known snapshot UUID, not an Inbox-item discovery query.
Match the exact retained capture identity, keep separate imported copies distinct,
and expose saved extraction versions without automatically reconverting or silently
choosing a new approval baseline. This is discovery over existing records, not a
new schema or evidence store. Display
untrusted content inertly, block remote loads and arbitrary local-file links,
and resolve image bytes only from the checked snapshot. No Word/DrawingML/HTML
renderer, semantic mapper, converter/schema migration, second receipt ledger,
general chat/tool overhaul, additional format, binary cleanup qualification or
larger-context P3 work is part of this proposal. If readable mail requires data
not exposed by the maintained result, identify the narrow prerequisite before
expanding the boundary; do not recover it with another parser or guessed slicing.
In particular, decoded email headers currently arrive as adapter-authored source
text, not separate typed header fields. The bounded first reader renders that
existing metadata and body safely; a conventional custom From/To/Date header would
need an explicitly scoped maintained-adapter projection rather than parsing the
rendered Markdown back into guessed fields.

**Acceptance for a selected UI change:** import appears without manual Refresh;
the owner can read the email, named DOCX, inline/repeated chart and unsupported
XLSX warning and identify the next action without guidance. Required evidence and
full proposed changes remain inspectable. Source-only offline review works;
configured AI, comments/Rewrite, exact selected dependencies, conflicts and partial
application retain current authority. Full Quit/restart reopens the same retained
extraction and settled review without conversion/provider reexecution. Targeted
state/widget checks cover successful capture, failed capture, late replies, item
switching and input preservation; relevant native/shared checks and actual owner
observation follow the repository verification guide. Real-model usefulness remains
a separate bounded campaign gate. No product code or UI implementation is changed
by this evidence/proposal record.

## Selected P2 implementation — 2026-10-07

The owner now selects the accepted P2 specification and authorizes production code,
dependencies, lockfile, tests, documentation, bounded paired live validation and an
implementation PR. This supersedes earlier pause/finalization-only restrictions for
P2; the dated evidence below remains historical. No merge, release, private data,
global configuration, new account/billing or P3 work is authorized.

Baseline: PR86 final head `4e6c5a294582ad765e011cf2708fc000fac3b185`, merged
at `47f12c05dadcabd899f015f4d8e2b904f12d8557`, verified against fetched
`origin/main`. Clean task branch `codex/p2-email-docx-intake`, worktree
`/Users/evokessler/repos/brn-p2-email-docx-intake`; all existing work preserved.
One lead integrates; two bounded Sol High helpers isolate maintained adapter and
snapshot persistence. Reassess only on a material containment, association or
recovery blocker; use task-owned targets and preserve unknown usage as unknown.

Execution milestones from the accepted specification:

- [x] Retain versioned extraction, original/attachment/image/occurrence collections
  and exact hashes; verify old-record readability and proof-based legacy Restore.
- [x] Integrate pinned BetterOffice structured/Markdown/package exports and MIME in
  a helper restricted before input; validate untrusted bounded output, cancellation
  and failures. Replace converter reconstruction with retained snapshot validation.
- [x] Admit private snapshot investigation through Rig, exact citations and Source
  prerequisites; prepare/revise knowledge, History transition and related Actions.
- [ ] Expose evidence, images, gaps, reasons and exact selected changes in shared
  CLI/native grouped review; demonstrate simple/plural cases and quit/restart.
- [ ] Run appropriate integrated/storage/native gates, bounded paired Sol/Luna
  scenarios, independent actual-tree review, fix validated defects, commit and PR.

Checkpoint (implementation present; review handoff qualification): the
maintained adapter/helper, V16 immutable snapshot/mirror recovery, private Rig
investigation, exact Source dependencies and CLI/native collection review are in
the task-owned tree. The old DOCX package/XML/Word interpreter is removed; saved
markup/proof readers remain. The complete default workspace gate passed 1,613
tests, with 17 documented ignored entries, and 52 end-to-end fixture assertions.
This gate preceded the final retained-extraction viewer and Source-form corrections;
those changes receive fresh targeted and native checks below.

The retained snapshot reader now reopens extraction without an original, queue row,
converter or model. Focused workflow intake checks pass 10 tests, including the
readonly restart witness, single/plural private investigation, comment/Rewrite,
exact group approval and restart, shared-image imports, repeated preservation of a
snapshot, original/asset freshness, short valid EML and legacy renewed-review/partial
Restore. The ignored private crash entry is exercised by its parent test. Adapter
qualification passed 22 tests, including native restrictions and process cancellation.

Independent actual-tree review prompted fixes for Applied dependency freshness,
shared-image filename collisions, lost refusal categories, ambiguous identical
source-text ownership, selected prerequisite group membership, stale retained
extraction display and the native multi-asset Source form. Re-review found no
remaining actionable static defect. The real plural email now passes native state
qualification through Source draft creation with all validated image assets.
Source UUID namespaces edit only exact qualified image destinations; citations map
back to immutable text ranges, and ambiguous ownership refuses. Old generic
literal-Email cleanup fixtures now use Text; real MIME fixtures test EML behavior.
Raw-versus-materialized filename assertions were updated openly. Meaningful
freshness/recovery assertions remain. The snapshot bootstrap's oversized-inventory
startup regression was fixed and its existing witness passes.

Live campaign usage ledger/numeric remaining allowance was not found in committed
records; an owner clarification is pending. Protected `brn ai status` on the
standard BRN-simple credential route reports ChatGPT and Copilot disconnected,
no selected model or effort. No token/cache contents were inspected. Session
provider completion calls are zero, prior campaign usage/remaining allowance
unknown. Actual Sol/Luna identifiers, supported efforts/tools/images and usefulness
remain unqualified; no fallback, login, new account/billing or allowance reset occurred. Public Harbor QuickLook thumbnail showed text, table,
images and footer; the feature-rich variation stalled and was cancelled. Full
native review/restart acceptance remains a separate gate.

Native checkpoint: strict default-workspace and all three desktop feature Clippy
variants passed. Native desktop tests passed 308 unit/widget + 7 integration tests;
native retrieval passed 13 library + 2 download tests. The complete native workflow
library passed 389 tests with 15 documented exclusions, and its unfiltered
model-contract integration suite passed 7 tests. Across these native commands,
726 tests passed, none failed and 15 were ignored. The maximum-size asset crash/
recovery test completed in the 440.84-second library run. These are offline synthetic
checks; real model assets were not loaded. The 15 ignored entries comprise 13
private subprocess entries exercised by parent witnesses and two preexisting
explicit expensive witnesses: aggregate-over-64-MiB mirror recovery and synthetic
copy startup cost. Those two were not run. Shipping desktop (`native-ui,native-retrieval`, without test support) and native
CLI builds passed; the shipping desktop passed two fresh AppWorker startup/shutdown
runs against the same new V16 database. The final fixture-only run passed 52
assertions and the launcher check passed. CUA attempted to open a unique task-owned local app and
reported the Mac locked; automatic unlock failed. An unlock request is pending.
No BRN GUI interaction or full GUI quit/restart is claimed. The earlier public
Harbor Quick Look capture is in the [screenshot index](../../../ui/screenshots/2026-10-07/INDEX.md).

Verification environment: Apple Silicon macOS, pinned Rust 1.98.1, locked/offline
Cargo, sequential task-owned target `target/intake-ui`, incremental compilation
disabled, and existing private `TMPDIR=/private/tmp/brn-p2-fixtures`. The complete
shared gate was `bash scripts/verify-end-to-end.sh`; the fresh native commands were
all three desktop Clippy feature variants from the verification guide, desktop
`native-ui,native-retrieval,native-test-support` tests, retrieval `native` library/
model-download tests, workflow `native-retrieval --lib --test models` **unfiltered**,
and the shipping builds. The final native Rust source matches the implementation
handoff tree; only documentation changed afterward. Python tooling passed 18 tests,
all shell scripts passed `bash -n`, retirement checks passed, Markdown validation
passed 48 files / 560 local links, and `git diff --check` passed. The upstream
`block 0.1.6` future-compiler warning remains; no real ONNX, corpus or live-model
qualification is inferred from synthetic/native tests.

The helper copied into that unsigned local app is byte-identical to the tested
helper (SHA-256 `8a7dae346aac826c2178c1e7666f1db6e52effa5288836a1fabb4a9ebc4bde71`).
With an empty environment it parsed the real 111,544-byte plural EML into 8 sources,
1 deduplicated image, 3 distinct occurrences and 4 explicit gaps. Its native sandbox
denied an existing synthetic file and a live loopback listener; its actual owned
helper/child/grandchild process group was cancelled and joined. An initial ad-hoc
qualification assertion incorrectly expected two unique assets; inspection confirmed
the fixture intentionally shares one image across the three occurrences, and the
corrected check passed. This qualifies the copied helper, not release distribution.

Remaining acceptance is explicit: unlock the Mac for actual BRN review/original
inspection and full GUI Quit/restart; restore the existing subscription route and
provide the prior campaign usage/remaining allowance for equivalent paired Sol/Luna
trials. The smallest next step is those existing-route/fixture acceptance runs,
not another parser or renderer. Picture titles remain unavailable where exact
mapping is unqualified; charts/SmartArt/shapes, layout and unresolved-prefix images
have visible gaps. JPEG is reviewable but the existing AI transport carries PNG
only. The existing 50 KB wrapped analysis-input limit remains; larger context is P3.
Ambiguous duplicate-node citations refuse instead of guessing. Non-macOS helper
profiles fail closed; hard RSS and signed distribution are unqualified. No P2
owner acceptance, merge/release or P3 continuation is claimed. The implementation
is [draft PR87](https://github.com/ewq100/brn-rust/pull/87), with these external
qualification gates pending. Tested implementation source is
`55ef38840bc08f2acf42a380a911cade9dbc7214`; subsequent handoff edits are documentation
only. The branch is pushed, independent review is clean, and no merge/release or
P3 continuation is authorized. Hosted checks are separate from the local results
above; inspect the PR's exact current attempt before any later integration.

Current seam disposition verified against baseline: keep capture no-follow proofs,
atomic proposal/apply/repair receipts, canonical historical readers and Rig. Adapt
Source bindings, private analysis and grouped review. Replace synchronous DOCX
reconstruction and literal EML conversion; remove Word/package interpreter and its
entry points/defaults/fallbacks after legacy transition witnesses. Snapshot payloads
are stored together in one canonical hashed SQLite transaction, so backup includes
manifest and bytes and a receipt cannot outlive an external blob. This avoids a
second filesystem transaction engine; absence/damage blocks evidence use.

### Local owner scenario (P2 implementation checkout)

Use only public fixtures and a fresh explicit workspace. Build the sibling helper
and shipping native executable (pinned toolchain 1.98.1):

```bash
cd /Users/evokessler/repos/brn-p2-email-docx-intake
export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
export CARGO_TARGET_DIR="$PWD/target/intake-ui"
cargo build -p brn-intake --features helper --bin brn-intake-helper --locked --offline
cargo build -p brn-desktop --features native-ui,native-retrieval --locked --offline
p2_owner="$(mktemp -d /private/tmp/brn-p2-owner.XXXXXX)"
mkdir "$p2_owner/data" "$p2_owner/vault"
"$CARGO_TARGET_DIR/debug/brn-desktop" --data-dir "$p2_owner/data" --vault "$p2_owner/vault"
```

In Inbox, **Import EML or DOCX file** selects
`experiments/architecture-reassessment/p1-office-mime/fixtures/plural.eml`.
Include it in the batch, **Process checked originals**, then **Inspect full
converted preview**. Inspect decoded email, Harbor/appendix attachments, the
retained unprocessed XLSX, inline imagery and repeated document images; use
**Inspect exact original with Quick Look** and review every stated gap.
Prepare the Source input at a new `.md` path and open its review; it remains
unapplied. Private investigation requires an explicitly connected subscription,
model and effort. Select that pending Source for investigation, request knowledge
and a related Action, comment that inspection and the EUR 4,000 cap must remain,
then Rewrite and review the exact versions. Select the Source prerequisite and
wanted consequences in grouped review, approve, fully Quit and reopen the same
workspace. Verify Source, Current, History where a predecessor was selected,
Action and retained evidence. Repeat with `single.eml` in the same Source folder;
shared image bytes use independent Source filenames.

This scenario distinguishes available implementation from acceptance: offline
synthetic Rig hooks establish lifecycle/authority, not model quality. The local
standard account route is disconnected and prior campaign usage unknown. Live
paired validation and owner acceptance remain pending. Full feature-rich original
preview also remains unqualified: a second bounded Quick Look thumbnail attempt
returned no image before cancellation at20seconds; Harbor rendered successfully.

## PR86 finalization and canonical fresh-agent handoff

**Latest owner decision, 2026-10-07:** the revised [P2 specification and retirement inventory](p2-conditional-spec.md) and maintained DOCX/MIME direction are accepted. Finalize and merge [PR86](https://github.com/ewq100/brn-rust/pull/86) only after final-head required checks and applicable merge policy, then stop. P2 production implementation, new probes/experiments, live-model calls, release, other merges and unrelated cleanup are excluded from this session. The older decisions below are dated history, superseded where this later decision differs; the broader P3–P13 queue remains proposed.

- **Accepted specification:** separate historical saved-record readers from conversion; preserve notes/assets/receipts and recover unfinished work through retained originals, exact proofs, explicit Restore where safe, new extraction and renewed review. Retire the complete bespoke DOCX converter from production, including defaults/fallbacks and old-draft approval/Finish dispatch. New format mechanics belong in the maintained adapter/helper; Store persists evidence. Preserve all V1/cardinality/approval decisions. Correct image/source/occurrence associations and honest omissions are required; separate authored picture titles may be explicitly unavailable without a qualified exact mapping. Title support never requires a custom XML scanner.
- **Baseline and integration receipt:** preparation began at PR85 merge `3d59cdd4d6379b526fed19f6a339cfd2dce2dead`; reviewed clarification head is `f8791335cc520f0020c110bf5f0b2ba908fa06ce` on `codex/p1-office-mime-evaluation`. PR86's finalization receipt records the final pushed/checked head, exact hosted run, policy, merge SHA and post-merge verification. Its GitHub merge commit is the merged baseline for continuation; acceptance alone is not proof of merge. Re-fetch main and read that receipt before taking responsibility. Preserve existing dirty work, especially the unrelated `brn-removal-boundary` worktree, and all other PRs/checkouts.
- **Evidence and review:** [P1 evidence](p1-evidence.md) and the focused API record retain completed synthetic results, costs, prior clean independent reviews and limits. This finalization changes only title acceptance and documentation/handoff status; the existing substantive reviews remain applicable. Lead diff/link/scope checks cover this bounded clarification; no new independent experiment or product qualification is claimed. Final-head hosted checks and integration identity belong to PR86's durable receipt.
- **Remaining P2 integration:** obtain an explicit P2 implementation selection, refresh affected production seams against merged main, then follow the specification's dependency order. Implement the legacy-work transition and saved-record/recovery witnesses before converter removal; integrate versioned snapshots/assets/occurrences and maintained MIME/DOCX structured/Markdown/package APIs in the constrained helper; adapt private analysis, exact evidence/quote bindings, Source dependencies, revised knowledge/Actions and grouped CLI/native review. These are remaining tasks, not implemented or Build-ready work selected by PR86. No repeat broad survey or unchanged probe is needed.
- **Upstream gaps and unsupported behavior:** the 0.3.0 export graph is larger (86 enabled packages versus P1's 53), pulls edit/layout/font/Yrs internals and has no export-only feature; BRN owns no collaboration lifecycle. OPC admission/parser reinflation overlaps, BRN policy budgets need helper qualification, and no hard RSS/shipping bundle qualification is established. Blank image URLs require exact exported anchor→resolved part→validated asset binding. Prefix-only variants can leave unresolved references; separate title identity mapping is unqualified and titles may be unavailable. Keep diagnostics/partial status and meaningful original inspection, never guessed joins or custom OOXML interpretation. Charts/SmartArt/shape rendering, layout fidelity and revisions remain unqualified. PPTX production, PDF/layout selection, full XLSX and standalone-image intake are outside P2; XLSX attachments remain retained/unprocessed. Binary cleanup and unsafe recovery are not authorized.
- **Remaining acceptance:** demonstrate the EML→DOCX/meaningful PNG→private investigation→revised knowledge plus related Action→exact grouped approval→restart/history story, plural attachments/repeated occurrences and the visual-loss case under the clarified title rule. Complete production safety/freshness/cancellation/crash/Restore/compatibility tests, packaged-helper restrictions, actual native image/original review and separately authorized provider usefulness/owner acceptance. Retained originals alone do not prove useful visual review. PR86 merges evaluation/specification, not production or native/live acceptance.
- **Existing bounded Sol/Luna authorization:** the [runtime comparison](#later-solluna-runtime-comparison) remains available to a fresh agent for Sol 6.1 Medium versus Luna Medium and bounded supported Luna effort variations, with the same public/synthetic scenarios and predeclared aggregate request/token/cost/time ceilings. Verify real provider IDs/efforts and prerequisites, retain evidence, allow no fallback/private uploads/new paid accounts/unrestricted spend, and record unavailable capabilities honestly. This does not authorize P2 implementation and no calls occur in this finalization session.

## Historical planning and selection record

**P2 clarification after PR86:** the owner settles maintained adapters and removal of the full bespoke DOCX interpreter. The [updated specification](p2-conditional-spec.md) distinguishes historical record reading from conversion, identifies actual unfinished approval/Finish dependencies and defines explicit retained-original/Restore/renewed-review transitions. Its focused higher-level BetterOffice API check replaces the proposed manual mapping/relationship mechanics with upstream export/package APIs and named bounded guards/gaps. No implementation or merge is selected by this clarification.

**Later owner selection, 2026-10-07:** P1 is selected for isolated Office/MIME evaluation, with a conditional P2 specification explicitly authorized for review. The [P1 record](p1-evidence.md) owns fresh baseline, scope, resource/check/review evidence and recommendation; the [conditional P2 specification](p2-conditional-spec.md) owns affected production paths, retirement inventory and acceptance. Production adoption/implementation and any merge of this result still need the owner decision. PR85 is confirmed merged at `3d59cdd`; the older baseline and finalization-only restrictions below describe that prior session. The broad audit and separate live-model campaign are not restarted.

**P1 result:** bounded useful extraction/adaptation, parent/occurrence associations, omission inspection, native containment/cancellation and measured isolated costs are demonstrated; the complete result/specification passed independent Sol High review. Next unsatisfied gate is the owner's bounded adoption/P2 implementation selection. Full product packaging/native/live/owner qualification remains separate. Continue from the current P1 record rather than rerunning the broad audit or tiny probes.

**PROPOSED, pending owner acceptance and implementation selection.** The owner authorizes focused documentation/planning finalization and merge of PR85 after required checks, then stop. No new probe or live call occurs in this session. A fresh agent has the bounded runtime authorization below; production implementation, other PR merges/releases and private-data operations remain unselected/unauthorized. Baseline `origin/main@67818388112c400802097763b9eeed8aa87bb465`. The [reassessment](../../../audits/BRN_ARCHITECTURE_REASSESSMENT_2026-10-07.md) owns findings, sources, decisions and evaluation limits; the [Product Vision amendment](../../../product/BRN_PRODUCT_VISION.md#owner-amendment--2026-10-07) owns requirements. This plan replaces the old H1–H5 queue as the proposed selection map; it does not silently resume V1.

## Ordering and decision gates

Complete the review/planning handoff first. Then, if selected, qualify a narrow adapter and demonstrate one product flow early. Broaden the shared draft tools and remaining formats incrementally; do not implement a large universal conversion/document model or recovery rewrite before a useful demonstration. Independent slices may proceed without unrelated qualifications, but product correctness dependencies remain explicit.

**P1 unresolved adoption question:** can BetterOffice0.3.0 plus a small BRN adapter and dedicated MIME parsing produce useful narrow Office/email extraction with correct source/asset-occurrence associations and visible gaps, inside a constrained cancellable Apple Silicon helper, at lower total maintenance/deployment cost than extending the custom interpreter, without recreating a second full parser or approval system? A bounded adoption choice or concrete blocker must be backed by the endpoint evidence in P1 below; perfect formatting and all-Office coverage are not required.

| Slice | User-visible outcome / dependencies | Modules and reuse / code or constraints retired | Meaningful verification / owner acceptance | Independent? |
| --- | --- | --- | --- | --- |
| P0 completed by this task | Canonical clarified requirements, current Git/PR evidence, one reassessment and proposed queue | Existing docs; supersede freeze, mandatory stage ordering, six-crate assumptions, H4/H5 unconditional non-adoption and approval-every-persistence interpretation | Documentation links/diff; isolated probes separately recorded; independent recommendation challenge | Yes; no product change |
| P1 focused adoption probe, first selected technical step | Demonstrate useful Office extraction/assets with one common adapter plus real MIME parent/children; establish support and known gaps before production choice | Standalone BetterOffice0.3.0 Office challenger; rdocx0.15.0 DOCX comparator; mail-parser0.11.8 MIME; dedicated PDF vs Docling only in an independent later PDF/layout probe, not a prerequisite for Office/MIME selection. Reuse upstream models and small raw-field extraction. No product dependencies yet | Producer/public benign DOCX + PPTX, EML+Office attachment, hostile budgets/path/DTD; measure adapter size, enabled dependency graph, cold/warm wall/RSS and install/update size on arm64. Inspect rendered original/output and omissions. Terminating process must stop converter children; no network/vault/credentials. Owner can read the comparison. Endpoint: useful extraction/adaptation, correct source/asset associations, visible gaps, relevant containment/cancellation evidence and measured integration cost, ending in a bounded adoption choice or concrete blocker. Perfect formatting and complete support for every Office feature are not prerequisites; an optional missing field alone does not force rejection | Yes; before dependent production adapter design |
| P2 first recommended implementation slice, conditional on P1 and owner selection | **Real EML with one selected DOCX attachment with one known PNG → preserved original/children → decoded email + useful Office content/visual → editable grouped proposals → exact approval → restart/history.** Keep a second visual-rich Office fixture in acceptance | Workflow Inbox/file intake and shared DTOs; Store extraction/evidence snapshot binding; AI task input; CLI/native review. Reuse mail-parser and P1-selected Office adapter, existing asset/apply/recovery engine. First bounded demonstration: one EML, one DOCX, one known PNG, one revised proposal and a related Action; this is not a permanent schema/application limit. Planned structures support collections of attachments and visual occurrences, including repeated assets. The later P2 specification must identify custom code replaced/removed from new imports, necessary shared safeguards retained, and readers retained only for historical records; do not write that implementation specification during this handoff. General member/path restructuring waits for P3. Retire literal-MIME-as-email claim, avoid extending single-image-specific records, remove approved-Source prerequisite for investigation and technical-only asset review in this slice. Historical records/readers remain; they do not justify a competing legacy path for new imports | Raw/decoded headers and original timezone, no invented IDs/authentication/thread from subject, text+HTML/CID, exact attachment bytes and parent relationship; unknown attachment visibly unprocessed; a small second case has two attachments (one unsupported XLSX retained/unprocessed) and two occurrences of the same PNG in the supported document, preserving each parent/occurrence association without complete Office support; source/extraction/AI/gaps/reasons distinct; revise/reject/approve current displayed version; changed source/late result cannot apply; restart/old-DB recovery uses saved snapshots without converter/LLM reexecution. Owner navigates email, attachment visual and proposed knowledge/action in native and headless interface. Stubbed AI proves mechanics; later authorized real-model observation proves usefulness separately | No: P1 selection plus bounded snapshot/effect spec. Does not wait for all-format/recovery rewrite |
| P3 broader investigation and structural draft revision | Normal chat can inspect retained/unclassified/long evidence, create/split/retarget/regroup proposals, revise after comments and recommend conflict resolution under visible budgets | brn-ai tools/behavior/Rig hooks, workflow BoundProposal/context, Store draft revision/versioning, CLI/native progress/review. Keep one runtime, exact apply. Replace fixed8/50k/no-range limitations with task budgets and range reads; remove Inbox-only knowledge drafting and blanket no-winner recommendation ban. Retain host-minted identity, managed transitions and original immutability | Read tail of near1MiB note using snapshot/hash/offsets; raw metadata-invalid evidence clearly noncurrent; budget exhaustion/cancel/progress and bounded invalid-tool correction; no silent account/model fallback. Split/retarget draft preserves unchanged before bindings; new targets get host-captured proofs; changed external target requires explicit reconciliation and a new before/after, never a silently refreshed baseline. Revision changes the approval version, late result preserves owner edit, new assets inspectable. Owner reorganizes a proposal without approving intermediate work | Some safe ranged reads/budget UI independent; structural revision depends on P2 snapshot seam and small settled spec |
| P4 vault conflict and maintenance demonstration | Conflicting new email vs current knowledge yields evidence-backed preferred resolution/alternatives, reviewable update/history and follow-up; launch-due maintenance finds stale/neglected work | Existing Findings, Actions, current/history tools, same P3 draft work; small due/run state using existing SQLite. No semantic rules engine/general scheduler/second agent. Replace “report only/refuse reasoning” with recommendations; closure still not knowledge application | Opposing originals visible; stale/missing/unknown evidence explicit; deny silent truth resolution; dedupe findings without hiding revised evidence; cancellation/resume records no duplicate consequences. Approve displayed update/history/action group, reject another; owner sees why recommendation was made | Offline detector independent of web; full demo depends P3, web checks depend P6 |
| P5 complete required format profiles incrementally | Practical DOCX/PPTX/PDF and supplied-URL intake; complete V1 email/Teams/Markdown/text workflows; embedded meaningful visuals and inline email images remain in scope. XLSX and standalone-image import are excluded, with no assumed partial XLSX conversion | Same adapter/result seam. PDF/layout source/component selection is a separate later decision, independent of P1; optional Docling subprocess for layout only if it wins. Supplied HTML reused by EML/URL with safe display. Stop extending full handwritten XML engines. Old conversion snapshots stay replayable; no hidden dual-engine fallback | Per-format expected content/assets inventory and readable originals; scanned PDF separates OCR from source text; slides/notes/charts and exact quote locators; known omissions remain prominent. Partial usable review allowed; no cleanup completeness inference. Owner accepts each declared format profile, not “all Office supported” | Each format independent after seam; extraction first, AI-enhancement depends P3/provider qualification |
| P5b qualified original-copy cleanup | Owner can explicitly Remove/Restore a fully qualified binary intake copy while approved evidence/original artifacts remain available | Existing original-operation family plus immutable extraction/source proof from P2/P5; reuse host file/receipt machinery. No automatic purge; no new semantic-completeness engine from confidence | For a declared complete profile, owner reviews retained original/evidence and exact cleanup target; incomplete/changed/lost evidence refuses cleanup, cancellation/crash/old-DB restore retains proof and recoverability. Qualify each profile separately. Owner can Restore the exact removed copy; partial conversion remains retained | Conditional on meaningful-content/asset qualification, not required for useful partial review or P2 |
| P6 attributable web research | AI follows selected URLs/searches, captures evidence and revises durable-capture drafts within budget | Rig existing provider/native web capabilities or explicit chosen fetch/search adapter, workflow evidence, same proposals; no second research runtime. Keep converter networking disabled; fetch is explicit evidence operation | Cite captured URL/time/source text, disambiguate web vs vault/current vs inference, redirects/content caps/cancel, saved output restart without web rerun. Offline synthetic transport first; bounded actual route request only separately authorized. Owner investigates and approves capture | Can follow P3 independently of other format profiles; supplies P4 web detectors |
| P7 quality and operational gaps | Search works on realistic EN/ET corpus; useful Action compensation/Undo, outbound sent-version capture and safe during-session backups | Keep FTS5/FastEmbed/ONNX candidate until quality probe; indexed projection only on measured need. Existing typed Action changes/revision checks, SQLite online backup reused. Bind explicit “sent it” to actual sent version as a Markdown Source/thread and identified completion; drafting/approval leaves the Action open. Retire startup-only backup policy; preserve completion history | Corpus known answers/long-tail/Source-History scopes; real-model assets require separate permission. Backup after meaningful operational work then restore old DB + receipts with chats/drafts preserved to checkpoint. Action inverse refuses newer edits/completion ambiguity. Owner approves a reply, edits/sends externally, confirms actual sent version and identified completion; saved source/thread survives session deletion. Owner understands compensation and recovery limits | Backup/Action inverse can be independent; search quality independent of conversion, bounded Sol/Luna runtime comparison is authorized below; model-asset acquisition remains separately scoped |
| P8 project/person and graph views | Readable current context, communications/Actions/relationships; graph navigates existing evidence | Existing identity/link/index/workflow queries, native widgets. No graph database or persisted duplicate profiles-as-projections; approved profile notes remain Markdown | Missing/ambiguous IDs visible, current/history distinction, links reach evidence; headless parity for domain queries. Owner navigates project→email→action→source and graph | Read-only views independent after foundations; no dependency on full maintenance or graph for cleanup |
| P9 sessions/preferences | Reversible Archive/Restore,30day inactivity policy, warning before Delete, durable outcomes survive session deletion, approved preferences | Existing sessions/timestamps, Store/workflow/CLI/native; SQLite transactions and same proposals for preferences. No daemon. Delete/retention exact small spec required | Offline due archive, restore, deletion warning based on actual uncaptured work, saved source/provenance survives, stale command no loss; preference changes need review. Owner archives/resumes and understands Delete | Independent of converters; coordinate snapshot references before destructive spec |
| P10 bounded helpers | AI delegates useful investigation with shared total budget/evidence/cancellation and no independent authoritative writes | Check pinned Rig runtime/companion capability before extending current lane. Small task evidence return; no agent hierarchy/framework by default. Existing exact proposals accept results | Synthetic child cancellation/join, total spend accounting, evidence identity, untrusted-instruction containment and late result, no fallback/escalation. Separately bounded live comparison only if needed | Independent after P3 budget/evidence contract; not prerequisite for useful single-agent work |
| P11 existing-vault cleanup | Reviewable metadata/organization/supersession batch without graph prerequisite | P3 structural drafts + P4 evidence, existing identity/history/apply/Undo. Reuse Markdown AST for links; investigate lossless metadata editing rather than blanket YAML roundtrip. Retire fixed-path drafting limitations; no auto-migration or private vault access here | Synthetic mixed vault: unmanaged/malformed metadata, duplicates, conflicting claims, archive targets, changed external files. Uncertain identity permits draft but blocks unsupported application. Batch preview shows moves/link repairs/history; exact recovery/Undo. Actual vault operation separately selected | Does not depend on graph; depends structural changes and affected effect contracts |
| P12 measured machinery simplification | Lower maintenance/latency without weaker integrity; isolate only affected unresolved objects where provable | Conditional idiomatic clap migration; smaller shared effect/file helpers or affected-object fences only after measured probe. Keep SQLite/platform/Rig rather than ORM/workflow engine. No format/general IR replacement in this slice | CLI validates before workspace and keeps domain/JSON semantics; allow documented help/order changes. Recovery substitution preserves crash/stale/link/restore/foreign-object witnesses and read-only investigation of unaffected evidence. Measure full adapter/maintenance savings, not LOC alone | CLI independent; recovery/fence consolidation late and conditional; not blocker for P2–P11 |
| P13 trusted-user delivery | Apple Silicon installation, upgrades/recovery, usable V1 and scoped portability honestly qualified | Existing Rust/macOS packaging plus selected converter/runtime/license notices; pinned artifacts and explicit optional model install | Clean machine no inherited caches/credentials/network dependency surprises, offline launch/read, install/update/rollback, representative corpus RSS/time/disk, native/IME/accessibility/chooser/restart, logs/privacy, backup recovery, qualified selected-provider semantics. Signing/accounts/distribution need separate authorization. Existing Windows/Linux failures remain visible | Final depends declared V1 profile and acceptance, not speculative universal portability |

## Every remaining V1 stage retained

This mapping changes sequence/approach, not the complete product goal:

| Original stage | Disposition in proposed plan |
| --- | --- |
| 0 guidance | P0 owner amendment and reconciled active docs |
| 1 Save / 2 removal | Already integrated foundations retained; P12 changes only with equivalent evidence, never resurrect legacy paths |
| 3 provider capabilities | Keep explicit choice; P3/P6/P10/P13 name fresh offline vs live qualification; old permission is not reused |
| 4 Proposal Core | Keep exact snapshots/application; P2 reviewability and P3 structural drafting; P7 Action inverse gap |
| 5 knowledge | Keep UUID/provenance/scopes; P3 evidence access and P7 multilingual quality; P8 views |
| 6 Actions | Keep real state/direct completion; P2/P4 semantic proposals, P7 inverse/recovery; P7 adds actual sent-version/thread capture with explicit completion acceptance |
| 7 Text/email | EML now explicit; P2 completes actual MIME workflow, preserves parent/children and interpretations; pasted content still supported |
| 8 Office/URLs | P1 decision, P2 vertical demo, P5 profiles and conditional P5b recoverable cleanup; no automatic binary cleanup or universal-format claim |
| 9 web | P6 same agent/evidence/draft path |
| 10 Needs Review/maintenance | P4 offline and P6 web-dependent checks; all current staleness/conflict/neglected-action goals retained |
| 11 project/person | P8 composed context/profile outcomes |
| 12 helpers | P10 bounded shared runtime, no independently authoritative writes |
| 13 sessions/preferences | P9 reversible lifecycle/capture warning/preferences |
| 14 graph | P8 graph as view |
| 15 cleanup | P11, may precede graph; actual private data operation not authorized now |
| 16 delivery | P13 upgrade/recovery/clean-machine/native/provider acceptance, no release in this task |

## Open PR and local work disposition

| Work | Proposed disposition | Reason / action after handoff |
| --- | --- | --- |
| [PR84 H4](https://github.com/ewq100/brn-rust/pull/84), head0f3e277 | **REVISE** result framing; retain pinned fixtures/observations | Preserve loss/resource evidence, qualify corpus/oracle limits and remove mandatory custom-expansion conclusion. Link this assessment; optionally merge a historical evaluation only under later authority. No dependency adoption or merge now. |
| [PR80 H5](https://github.com/ewq100/brn-rust/pull/80), headbe59a7a | **REVISE** conclusion; retain legacy-contract experiment | Label non-adoption conditional on old compatibility policy. P12 idiomatic probe may supersede choice; do not extend emulation merely to preserve wording. |
| [PR82 H1 record](https://github.com/ewq100/brn-rust/pull/82), head8018755 | **RETAIN**, refresh current status/links before publication | H1 is already merged, historical checks/integration evidence useful. Avoid overwriting H2/current reassessment status with old baseline. |
| H1/H2 merged code | **RETAIN** | Useful small reuse improvements; H2 integration record is corrected in this documentation task. H3 remains unexecuted; fold into P3/P6 offline native-schema probe. |
| Dirty `brn-removal-boundary@4a6446f` and old unmerged review work | **HOLD**, preserve | Do not apply its old gates over current lean witness/main. If later selected, inventory exact dirty patch and reconcile against integrated code, not assume newest worktree. No reset/delete/merge. |
| Other old feature/trial refs and local worktrees | **HOLD as history** | No demonstrated pending V1 dependency; none cleaned up/migrated by this task. |

## Settled scope and remaining proposed choices

- **Settled owner requirements:** real EML ingestion; preserved originals/parent relationships and meaningful embedded/inline visuals; useful qualified extraction and editable drafts before exact approval; collections of attachments/visual occurrences. Full XLSX and standalone-image import are **OUT OF SCOPE for V1**, including assumed partial XLSX conversion. Unsupported XLSX attachments may remain retained/unprocessed. Research already completed remains evidence, not active V1 work.
- **Still proposed:** architecture/adoption direction, selected Office adapter, P1 execution and its adoption outcome, then bounded P2 production scope. PDF/layout selection is independent and later. Production implementation requires explicit task selection; merging this documentation does not select it.
- Model-asset acquisition, clean-machine/distribution and actions beyond the bounded runtime campaign below need their applicable authorization. No release or other PR merge is authorized.

## Later Sol/Luna runtime comparison

**Owner authorization, for a fresh agent:** evaluate useful BRN runtime tasks with GPT-6.1 Sol at Medium reasoning and GPT-6 Luna at Medium, with bounded experiments at other supported Luna efforts. The question is whether Luna performs the same useful BRN work as Sol, not whether to repeat this architecture audit. This authorization supersedes earlier blanket no-live/fresh-permission statements for this campaign only. No calls or new experiments occur during PR85 finalization.

Carry forward the existing comparison outline: email/Office investigation, meaningful visual review, and conflict/maintenance recommendations. Use the same public/synthetic inputs, task instructions, available BRN tools, evidence and review criteria for paired Sol/Luna runs. Score useful grounded output, source/asset associations, visible omissions/uncertainty, proposal revision, approval discipline and task completion; inspect outputs, not only parser/transport success. Compare other Luna efforts only on a named bounded question. If a scenario requires unimplemented capabilities, record that prerequisite or use an honestly labeled existing-runtime scenario; this campaign does not authorize implementing it or treating a direct model prompt as BRN end-to-end evidence.

**Assistant-proposed ceilings, not owner-selected numbers:** retain the earlier at-most-three demonstration runs per explicitly selected model/effort condition, one condition per run, with no silent fallback. Before calls, write a small run matrix and aggregate request/token/cost/time ceilings covering Sol Medium, Luna Medium and any selected Luna effort variation, using available account limits/prices. Bound total conditions and retries, expose progress and cancellation, and stop at the first applicable ceiling; do not multiply three runs into an open-ended campaign or silently widen budgets. Numerical ceilings remain experimental proposals, not an unlimited spending authorization or a reason to re-ask the settled model choices.

Verify actual provider model identifiers and supported effort settings before use; display labels and historical catalog results are not proof. Use only the explicitly selected available account/provider; report unavailable routes/efforts without substitution. Preserve prompt/input hashes, runtime/tool/evidence identities, selected model/effort, outputs, errors, request/retry counts, latency and available usage/cost information. Mark missing token, reasoning/cache or billing metrics as unavailable; label estimates and their source rather than inventing measured costs. Use disposable public/synthetic data, no private uploads/credential inspection, no new paid accounts, no unrestricted spending, and retain exact approval boundaries. Model choice is not permission for authoritative writes or model downloads.

## Task verification and durable fresh-agent handoff

- **Entry points:** this plan, [current status](../../../status.md), and [owner requirements](../../../product/BRN_PRODUCT_VISION.md#owner-amendment--2026-10-07). PR85 branch `codex/architecture-reassessment-2026-10-07` began finalization clean at `c913e4c6d85c4bd2d31501ba5f70a473c2872710`, against main `67818388112c400802097763b9eeed8aa87bb465`. The [PR](https://github.com/ewq100/brn-rust/pull/85) records final head, required checks and integration identity. Preserve all other branches/worktrees and dirty work.
- **Completed evidence:** the [reassessment](../../../audits/BRN_ARCHITECTURE_REASSESSMENT_2026-10-07.md) and standalone probe records retain executed pinned synthetic BetterOffice/MIME checks, measured tiny-fixture costs, known losses and source-only comparators. Title scanning is unbound; XLSX was from parts, not file ingestion; CID bytes were a non-image sentinel. No production workflow, broad fidelity, helper containment/cancellation, packaging or live-model/native usefulness is established. Do not repeat these unchanged probes or the whole audit.
- **Focused existing-code follow-through, next agent only:** verify the affected Inbox/MIME and `crates/brn-store/src/work/inbox_source/docx.rs` / `docx/` paths, shared evidence/asset records, workflow/AI draft restrictions and CLI/native review seams against current merged code. Identify what a selected adapter replaces on new imports, which safety checks stay shared and which readers serve historical records only. Retire literal-MIME-as-email, single-image-specific new-record coupling, approved-Source prerequisites and technical-only review within the selected slice; avoid parallel default converters or approval systems. This handoff does not supply or select that implementation specification.
- **Next recommended action:** after the owner selects P1, re-fetch/recheck main, PR heads and dirty state, read these settled requirements, and execute only the focused adoption question above. Completion is a useful bounded result with correct source/asset associations, explicit gaps, relevant containment/cancellation witnesses and measured full integration cost, followed by an adoption choice or concrete blocker. No perfect-formatting/all-Office gate. A later P2 specification and production implementation require selection after that result; P2 starts with the narrow demonstration plus the small plural/repeated-occurrence case.
- **Separate later campaign:** the Sol/Luna comparison is authorized within the bounded conditions above, but is not started by this handoff. Verify actual routes/efforts and runtime/fixture prerequisites, then declare the comparison matrix and experimental ceilings. Missing product capabilities remain limitations, not an implementation authorization.
- **Verification/integration gate:** focused changes are documentation/planning only. Run `git diff --check`, the current Markdown gate and explicit outgoing links for the amended audit. Verify all four protected CI checks against the final PR head before ordinary permitted merge; informational Windows/Linux failures stay visible. Final head/check/merge results are accessible in PR85. No production Rust/UI, root manifests/locks or CI changes, no new probes, no live calls. Stop after merge verification; no cleanup or automatic V1 resumption.

Local PR85 finalization verification: `git diff --check` passed; `python3 scripts/check-markdown-links.py` passed44files/524local links/0failures; explicit amended-audit outgoing check passed1file/53local links/0failures. These are documentation checks, not fresh production/probe/native/live qualification. Protected hosted checks must still pass for the final pushed head before merge; their immutable run results and merge SHA are recorded by PR85.
