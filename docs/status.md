# Current development status

2026-10-04. The owner has authorized sequential BRN v1 delivery under the frozen
[product vision](product/BRN_PRODUCT_VISION.md), [architecture](architecture/overview.md#frozen-target),
[invariants](architecture/invariants.md), [roadmap](roadmap.md) and
[development workflow](development/workflow.md). **Stages 1–4 are implemented,
automated verified and locally integrated. Stage 5 qualification/publication continues; Stage 6 Actions/dashboard is active.
Stages 5–16 remain unfinished. Complete BRN v1 delivery is not claimed.**

The owner's 2026-10-04 [client-boundary amendment](architecture/overview.md#client-and-protocol-boundary)
establishes BRN as a headless platform: six current V1 core crates with permitted
thin future adapters through workflow/AppWorker. Independent implementation audit
found no desktop/CLI domain bypass; existing scoped knowledge/proposal/activity
commands are retained. Read-only local stdio MCP is future work; no daemon,
network service or added V1 stage is authorized by this amendment.
Workflow-owned search evidence now keeps SQLite passage IDs inside retrieval.
The real row-reallocation regression and independent correction review passed;
exact quotations/hashes/scopes remain stable across disposable-index rebuilds.
Merged-main CI37197827195 passed Mac3+UbuntuCore/UI; existing non-Mac exclusive
install/Unix API gaps leave overallCIred, without a shared macOS defect.

Manual Save/recovery (`6609442`) preserves exact UTF-8, generation-bound recovery
and file/parent/root identities. Copies install exclusively; missing originals
are not recreated. Acknowledgement establishes recoverability. Guarded navigation
and Quit drain admitted work; system termination can lose unacknowledged typing.
[Stage 1 evidence](work/completed/simple-save/plan.md) retains checks and native
acceptance. Stage 2 (`a5ec4ae`) removes legacy production paths, `brn-core` and
`brn-flow`. The six-crate workspace refuses old/mixed markers before SQLite without
migrating original data. [Stage 2 evidence](work/completed/legacy-removal/plan.md)
records the retired baseline and qualification.

Stage 3’s scoped live round is complete (`d40e0a1`): both fresh human connections
succeeded, using **10 logical probes / 14 completion attempts** within the 11/22
cap. ChatGPT `gpt-5.5` Responses passed low/high read tools and the tiny image.
Copilot `gpt-5.3-codex` Responses passed read tools but reversed image colors.
Both observed hosted web search with correct official SQLite URLs; native citation
metadata remains unqualified. Copilot `gpt-5.5` Chat refused with
`unsupported_api_for_model`; production reports `ModelRefused` without fallback.
[Stage 3 evidence](work/completed/provider-capabilities/plan.md) records limits.
That completed round authorizes no further account calls or model downloads.

Stage 4 Proposal Core is implemented through sequential reviewed slices. Complete
typed Create/Replace/Trash drafts preserve exact before/source bindings and full
review text. Temporary comments retain uncertain quotes without guessing anchors;
edits, comments, rejection and owned Rewrite use one exact review version. Rewrite
shares the owned chat lane, captures explicit provider/model/effort and settles
validated output atomically; restart interrupts without retry. Fresh Ask also
requires explicit low/medium/high effort, while older unknown-effort history stays
readable and replayable offline.

Exact individual/captured-group approval, whole-file application and ordinary
proof recovery are shared by AppWorker/CLI and native controls. Fresh pre-effect
refusal preserves review work; unknown/partial effects stay fenced. Historical
replay never repeats installation and preserves later files and typing. Applied
cleanup removes covered temporary annotations. Paged Activity loads only the
identified full snapshot. Full Undo, original-index Trash restoration and explicit
Finish/Restore repair freeze exact previews/attempts; workflow rechecks eligibility
and retains errors/outcomes without automatic retry. Native initial composition
creates full one-note proposals; CLI supports multi-member requests/groups.
Completed acknowledged answers may explicitly prefill session-bound review work;
real seed drafts/comments/owned Rewrite provide AI writing. Unsubmitted form input
is transient, copyable and guards leaving; only acknowledged creation recovers.
Knowledge changes only after exact approval.

[Stage 4 evidence](work/completed/proposal-core/plan.md) retains each baseline,
independent reviews, technically verified fixes and reproducible manual scenarios.
The final creation slice’s independent review found no actionable defects and
passed **6 default / 6 native creation tests, 4 source-worker tests and 1 widget
test**. Fresh root macOS arm64 / Rust 1.98.1 locked/offline gates passed **684
workspace tests, 0 failed, 2 ignored**, **52 end-to-end assertions**, retirement,
format/build/all-target Clippy with warnings denied and **147 native tests**,
shipping native build and native Clippy. Default desktop passed **131**; workflow
**186**. The two ignored private crash entry points are exercised by subprocess
matrices. Earlier slice counts remain in the evidence, not current gate claims.

Stage 5’s [active plan](work/active/knowledge-foundations/plan.md) records sixteen
integrated slices: managed Markdown identities; fresh duplicate/incomplete
inspection; current/source/history/all retrieval and native read-only browsing;
durable exact provenance and native source inspection; reliable session/turn
timestamps; saved CommonMark links; disposable relationships; exact stable-link
preparation; native saved-link/relationship inspection; and native exact link preparation. Fresh whole-byte observations detect retained-size/mtime changes. UUID links survive moves without
path/title guesses. Saved citations retain their exact historical quotes and
source uncertainty independently of sessions or disposable indexes.

Relationship pages distinguish explicit links from provenance-based candidates,
retain full endpoint hashes and exact proofs, filter both endpoints before
pagination and report duplicates/incomplete inspection. Healthy index V2 upgrades
additively to V3, preserving passages/vectors. No AI or durable write occurs during
reconstruction. Independent review reproduced a valid reference-proof boundary;
the corrected 8,192-proof cache preserves the accepted 4,096-link extractor. An
introduced absent-metadata update regression was also reproduced and corrected.
That slice’s final review passed **45 tests** with no actionable finding, plus
actual CLI probes for **8,192 exact proofs** and **5,000 notes / 4,999 edges**.
The two scale queries took **6.108 s / 10.026 s** on this Mac; each relationship
request currently rederives the saved vault observation.

Stable-link preparation returns full additive Replace review input with both
consumer/target source bindings and no admission or vault write. It preserves the
entire byte prefix, escapes literal labels and verifies parser placement. Fresh
approval checks new UUID targets after edits/Rewrite against exact saved evidence
or same-draft reviewed target bytes. Historical links, Undo and completed replay
retain their authority. Broader tests caught and fixed an unrelated legacy-layout
regression. Independent public probes also reproduced and verified fixes for
case-sensitive aliases hiding duplicates and opaque metadata supplying false old
link authority. Final review passed **62 tests / 0 failed / 1 ignored**, and the
ignored real App case separately passed on fresh owned case-sensitive APFS.

Native inspection shows one selected link/edge and persistent exact read-only
proof, complete identity/hash details, target uncertainty and inspection issues.
Scoped 25-edge pages replace the prior observation; stale/closed replies cannot
replace current state. Review reproduced Save Copy changing Unique to Ambiguous
and stacked panels placing controls outside a 480×480 window. Corrected
invalidation/scrolling passed **22 independent state/widget tests**, with no
remaining actionable finding. Unsaved typing and original evidence remain intact.

Fresh final Rust 1.98.1/macOS arm64 locked/offline verification passed **950
workspace tests / 0 failed / 3 ignored**, **52 end-to-end assertions**, retirement,
format/build/all-target Clippy, **213 native desktop tests / 0 failed / 0 ignored**,
test-support and shipping native Clippy, and the shipping native desktop build.
Fresh native retrieval fixtures passed25;embedding metadata/vector guards10;
missing-model loader1 (both real-model tests compiled and stayed unexecuted).
Focused native workflow passed155/0failed/2ignored;capability fixtures passed
86library+1example,with feature Clippy and shipping native CLI build. Two ignored private crash
entry points remain exercised by subprocess matrices; the third case-sensitive
regression was explicitly qualified in the prior unchanged adapter slice.
Fresh shipping headless startup/restart passed twice with exact synthetic vault
bytes and zero credential files. The prior synthetic prepare/Create/Approve/rebuild
scenario is retained. No providers, downloads or actual ONNX inference were used.
The [native manual scenario](../crates/brn-desktop/README.md) is reproducible;
unlocked GUI/owner acceptance remains pending.

Native link preparation uses the guarded initial Replace form, exact captured
consumer and a separately inspected target UUID/hash. It retains both full source
bindings and allows title/body edits while fixing path/kind; explicit Create and
Approve remain separate. Empty/exact-capture body guards prevent discarding authored
replacement text. Stale operations/form/input/document generations and changed full
consumer proofs preserve current input. Archived target moves follow UUID. Separate
proposals retain complete bindings under fresh UUIDs. Independent reviews passed
**7 form + 6 preparation state + 6 existing creation + 3 new / 1 existing native
widget tests**, with no actionable defects. A real worker prepare/Create/Approve
scenario verified unchanged vault until approval; fresh shipping startup/restart
passed twice with exact synthetic bytes and zero credential files.

Basic findings now persist strict immutable source proof in operational schema
V9. Duplicate UUIDs and unresolved saved links supply fresh deterministic capture;
exact closure changes queue state only. Original quotes/full fingerprints survive
restart, closure and source drift. Inspection reports new proof separately and
refuses foreign-vault substitution. Independent review reproduced and fixed
case-folding distinct paths; CLI qualification fixed local/global --version
parsing. Final findings suites passed **14 Store + 8 workflow + 4 process tests**;
independent review passed **20 Store/CLI + 7 workflow tests**, no remaining
actionable findings. Native Needs Review now offers bounded filtered pages, complete retained proof
and separate fresh inspection, direct exact Resolve/Dismiss, saved-issue capture
and explicit same-request retries even without a selection or available vault.
Independent state/widget reviews passed **8 + 4 tests**, with no unresolved
defect. A reproduced refused-navigation bug was corrected before integration.
The explicit installer now pins multilingual MiniLM-L12 quantized assets at
`2c4055b12046f11709e9df2c122e59ffbdc2f900` (135,392,488 bytes). Recognized new
ONNX requires all five exact assets before initialization; mean/384/max128/Static
and the full bundle produce a distinct identity. Saved/explicit legacy directories
retain their previous behavior and identity. Fresh defaults use a separate folder;
consent is scoped to this revision and never executes automatically. Independent
review reproduced and fixed the CLI legacy-folder mismatch. Fresh **28 native
retrieval +137 native workflow tests**, feature Clippy/builds and two shipping
startup/restart runs passed. A prepared ignored six-query bilingual model smoke
test compiled but was not run. Actual ONNX compatibility, EN↔ET quality, long-tail
truncation, scope/restart/rebuild and tool/CLI parity remain unqualified. The
bounded asset-download question is still pending. Ask now instructs the selected
model to normally use the current question's language, honor explicit language
requests and preserve original source quotes. Independent review passed two
actual Rig synthetic transport tests across three explicit routes and both Ask
APIs; no extra call or Rewrite protocol change occurs. Fresh adapter/capability
fixtures passed **86 library +1 example tests**. Actual response-language compliance
remains unqualified. Safe later implementation continues; Stage 5 is not complete.

Stage 6’s [active plan](work/active/actions-dashboard/plan.md) now includes checked
V10 Action reads and typed stored Create/Replace review members using the existing
exact lifecycle. Full replacement baselines/IDs remain immutable; completed work
cannot be reopened. Combined budgets and omitted empty fields preserve old
Markdown hashes. Workflow creation/apply/owned Rewrite explicitly refuse Actions
until whole application/recovery is qualified. Independent review passed71tests
and6synthetic CLI refusal probes with no Action/provider effects. A separate
recovery defect was reproduced and fixed: complete Action prevalidation skips
malformed backup candidates while refusing semantic-invalid mains. Independent
52tests plus genuine Action B-tree corruption recovery passed; V9 Findings
prevalidation/regressions are retained. Joined Store application now captures
exact Action after-state and atomically settles CAS writes with the whole receipt,
review and comment cleanup. Recovery checks real Replace baselines before importing
after-state, preserves newer/Completed work and refuses equal-version forks.
Independent review reproduced the missing before-fork check; its meaningful RED
then corrected regression passed, with no remaining findings. Final Store266/0/0
and independent56/0/0 passed. Workflow ordinary recovery now accepts checked
vaultless Action snapshots,imports terminal Applied state and settles unfinished
zero-file intents NotApplied. A meaningful regression reproduced vacuous Applied
classification; foreign Action IDs/full baselines/order now also prevent temporary
retirement. Final independent25/0/0 passed with no finding. Fresh998workspace/
0failed/3ignored+52fixtures,148focused native workflow/0failed/2ignored,
213combined-native/0failed/0ignored,both native Clippy configurations,shipping
desktop/CLI builds and two startup/restart checks passed with exact synthetic
bytes,V10 and zero credentials. Local integration/publication is recorded in the
plan. Shared full Action reads now go through workflow/AppWorker and CLI with
typed validation/NotFound,current-evidence fences,all-state/25-entry pages and
immutable creation-time/UUID cursors. Independent review's valid DEL/C1 display
defect was reproduced and corrected; review then found no remaining defect.
Fresh post-correction1010workspace/0failed/3ignored+52fixtures,153focused native
workflow/0failed/2ignored,213combined-native/0failed/0ignored,native Clippy/builds
and startup2 passed,V10,exact bytes,zero credentials. Native gates cover unchanged
desktop/workflow code; final CLI feature Clippy/build reran after the display fix.
Workflow creation/apply/Rewrite remain guarded until references and mixed
execution/recovery qualify; no real Action producer,dashboard or Stage6 completion is claimed.

## Qualification still open

Luna computer-use qualification on0243c5d passed fresh synthetic Current/Source/
History search and saved-evidence views, explicit Unicode Save with BOM/CRLF
preservation, and acknowledged unsaved-buffer recovery after verified full quit/
restart. AX labels and owned file/record evidence were observed; this is native
qualification,not owner acceptance. Screen capture failed(-3811/-3812),so visual
layout remains unobserved. No provider or model download occurred.

Broader GUI/IME/accessibility, chooser, native review/approval/Undo/
repair/creation usability, live Rewrite/effort usability and owner acceptance are
pending. Synthetic state/crash/widget tests do not establish physical power-loss
durability, other-volume support, actual inference or release readiness. These
items do not block later safe implementation. The completed provider round leaves
Copilot GPT-5.5 Chat unsupported, Codex vision accuracy and native citations
unqualified. Upstream `block v0.1.6` retains a future-compiler warning.

Reviewed PRs16–21 publish Stages1–4; their completed plans retain exact checks.
Stage5A [PR22](https://github.com/ewq100/brn-rust/pull/22) mergeda9f8382 after
exact4895916/run37191800152. Stage5B [PR23](https://github.com/ewq100/brn-rust/pull/23)
mergedd48654098f79c8b4a6b13982650c258245b2d800 after exact2d0993f/
run37193690701. Stage5C [PR24](https://github.com/ewq100/brn-rust/pull/24) merged
2483b31f38a2b941ab71b7fba5449519da600e82 after exact716aede/run37194454005.
Each exact latest head passed macOSCore/UI/Retrieval+UbuntuSharedCore; Windows
Unix APIs failed,leaving overallCIred. Independent review and relevant local
verification passed before merge. PostB14CLIprovenance/Storetimestamp tests,
postC10CLIlinks/relationships/preparation tests,52fixtures at each checkpoint,
merged-tree equality and two startup/restart checks each passed with exact bytes
and zero credential files. GUI/owner acceptance stays pending. PR25 merged8295326 after exact90de25e/
run37197453154 passed the same four applicable lanes; Windows Unix APIs failed,
overallCIred. Merged tree equality,3focused tests+52fixtures and two shipping
startup/restart checks passed with exact bytes,V8 and zero credentials. Stage5D
[PR26](https://github.com/ewq100/brn-rust/pull/26) mergedb0e93fb after exactfc98b0d/
run37199082097 passed Mac3+UbuntuShared;Windows Unix API failures leave overallCIred.
Merged tree equality,28focused tests+52fixtures and two startup/restart checks
passed with exact bytes,V9,zero credentials. Main37199659960 passed Mac3+UbuntuCore/UI;
Ubuntu native retrieval failed6pass/3fail on unchanged unsupported exclusive-install
expectations,Windows3failed,overallCIred. Independent analysis found no sharedMac
defect. Stage5 language [PR27](https://github.com/ewq100/brn-rust/pull/27) merged
42722526009b6c986271afc75680828bc5287c02 after exactcee0144/run37201040507 passed
the same four applicable lanes;Windows failed before tests on Unix APIs,overallCIred.
Merged tree equality,24focused profile/model/Ask tests+52fixtures and two shipping
startup/restart checks passed with exact bytes,V9,zero credentials. Main37201599486
passed Mac3+UbuntuCore/UI; Ubuntu native retrieval failed10pass/3fail on unchanged
unsupported exclusive-install expectations,Windows3failed,overallCIred. Independent
source/log analysis found no sharedMac/language defect. Stage6 storage/review/
joined-Store [PR28](https://github.com/ewq100/brn-rust/pull/28) merged5d0e9f5 after
exacta1cc9b4/run37202673215 passed Mac3+UbuntuShared;Windows failed before tests,
overallCIred. Merged tree equality,35focused tests+52fixtures and two shipping
startup/restart checks passed,V10,exact bytes,zero credentials. Main37203224513
passed Mac3+UbuntuCore/UI;unchanged Ubuntu native installer10pass/3fail and Windows3
Unix build failures leave overallCIred,with no shared Action defect. The next
workflow recovery [PR29](https://github.com/ewq100/brn-rust/pull/29) merged0243c5d
after exact9e785a5/run37204244962 passed Mac3+UbuntuShared;Windows Unix APIs failed,
overallCIred. Merged tree equality,25focused tests+52fixtures and startup2 passed,
V10,exact bytes,zero credentials. Main37204848736 passed Mac3+UbuntuCore/UI;
unchanged Ubuntu installer10pass/3fail and Windows3Unix failures leave overallCIred.
Independent actual-log analysis found no shared Action defect. Shared Action
reads are published in [PR30](https://github.com/ewq100/brn-rust/pull/30),exact
1298ee78e6fc788e04e6b76c3634ae1c0c5f1a22 with identical reviewed90e9193 tree.
Run37208267069 found an Ubuntu test expectation error for macOS-only recovery;
the corrected test retains shared fences and explicit non-Mac refusal. Independent
review confirmed no production defect. Corrected exact-head CI/merge remain pending;
GUI/model/provider qualification,wholeStage5 and wholeStage6 remain pending.

The owner authorized Luna-only BRN app/provider testing on 2026-10-04; development
and review may use Sol/Luna,never Astra. A fresh synthetic connection and bounded
exact `gpt-6-luna` qualification are planned. The existing ChatGPT adapter currently
refuses Luna locally; additive maintained-list support and offline review are
needed first. No new account calls have run; prior credentials remain untouched.
This does not authorize model-asset downloads or private data inspection.

Release/public distribution, other live calls/model downloads, purchases and
original/private-data inspection or migration still need applicable owner
permission. No original data was migrated or inspected.

Older records in [completed evidence](work/completed/README.md) and the
[earlier Rig notes evidence](work/active/simple-rig-notes/evidence.md) are history;
their specifications, process assignments and permissions are not execution plans.
