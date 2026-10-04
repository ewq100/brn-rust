# Actions/dashboard — roadmap Stage 6

Baseline main@8657637173777c4e143fa5ea6b5b265b470d4e07; preserve owner AGENTS.md.
Vision §12, frozen architecture/invariants and the existing proposal lifecycle
remain authoritative. Stage5 identity/provenance/scopes/relationships/findings and
language contracts are implemented offline; model/provider/owner qualification
remains pending and is not a dependency for safe operational foundations.

Complete outcome: approved real actions, Open/Waiting/Blocked/Completed states,
identified explicit completion, new related follow-up actions, dates/dependencies
and useful native dashboard plus headless parity. Extend the existing typed
proposal lifecycle, not a second action-approval system. Operational Action CAS
and mixed file/SQLite recovery must join the existing whole-operation transaction
and receipts; prove interruption/older-backup recovery before claiming whole apply.
Completed actions must not be reopened to represent new work.

## First slice: checked operational Action records and reads

Append WorkStore schema V10 with one actions table and stable created-time/id
pagination. No production Action mutation API in this slice: real creation/edits
will be wired only through exact proposals; explicit completion follows later.
Reuse WorkStore ownership, WAL/FULL, foreign/newer refusal, checked startup and
backups. No new database or generic repository/framework.

Freeze typed ActionData: exact title/description, state, optional owner,
related_person/project UUIDs, source UUIDs/thread UUID, optional ISO civil due_on/
follow_up_on dates, dependency UUIDs, parent/follows_up UUIDs and optional Low/Normal/High
priority (unset remains unset for dynamic urgency). Title/owner ≤512 UTF-8 bytes, description ≤64 KiB; UUID collections ≤64,
unique/nonnil; dependency/parent/follows_up self references refuse. Dates must be
valid canonical YYYY-MM-DD including leap days. No cross-vault guesses; relationship
existence/DAG checks belong to approval, not this read-only storage shape.

ActionOrigin retains id, exact creating ProposalStamp, initial ActionData and
created_at_ms. ActionRecord retains immutable origin, version, current data,
updated_at_ms, waiting_since_ms and completed_at_ms. Origin cannot be Completed;
version1 exactly matches origin/time, Waiting starts at creation. Current Waiting
requires a recorded since-time; other states have none. Completed requires a time
between creation/update; other states have none. Versions are positive signed-SQL
range; timestamps are ordered and fit signed-SQL range. Preserve initial bytes.

Expose only action(id) and action_list(state?,limit≤200,before?) plus strict
internal startup checks: bounded JSON, creation/record SHA256, schema/index shape,
indexed identity/version/state/time and semantic records. Semantic corruption
refuses without silently restoring earlier work; physical damage uses existing
backup restoration. Default list is all states,25 entries, immutable created-time/
id order. No UI semantics or automatic state transition is introduced yet.

Acceptance: meaningful synthetic V9 additive upgrade preserving existing work,
exact read/pagination/bytes, startup rehashed structural/hash/index/schema refusal,
valid/invalid dates/UUIDs/clocks/version, backup recovery and no public producer.
Independent read-only review, fresh Store/integrated checks and concise evidence.
A bounded helper may implement only fixed actions module/tests; lead owns migration
integration, proposal extension, workflow/frontend contracts and final integration.
Then add exact typed action changes/review/apply, mixed recovery, explicit Complete,
headless/read-tool access and dashboard in sequential qualified slices.

## Next slice contract: typed Action proposal work

The second slice stores/reviews Action members only. Workflow/frontend creation,
approval and owned AI Rewrite explicitly refuse Action-bearing input until the
complete shared apply/recovery and native review contracts are qualified. This
prevents an empty file-member list from being reported as successful Action apply.

After qualifying the checked foundation, extend ProposalDraft/DraftRequest with
bounded typed Action Create and Replace members. Replace captures the full exact
ActionRecord baseline; identifiers/baselines stay immutable through review edits.
Use optional vault binding only when Markdown targets/source proofs need it;
Action-only requests work without a vault. Old Markdown JSON defaults to no Action
members and keeps its existing vault object. One combined64-member/8MiB budget,
creation replay and exact proposal stamp protect every member. ActionData edits
and temporary proposal comments use the same review lifecycle. Creating/replacing
Completed or reopening completed work refuses; identified Complete is separate.

First qualify stored draft/edit/reject/replay without any application producer;
then extend existing ApplyJournal with checked before/after Action snapshots and
join Action CAS writes, comment cleanup and whole receipt in settle_proposal_apply.
Workflow validates relationships/dependency cycles and current source/before proofs
before admission/effects. Ordinary recovery records retain complete operational
after-state and immutable origins; older/fresh SQLite restoration must import
whole application state without replacing newer work or replaying file effects.
Mixed interrupted application keeps affected reads fenced until reconciliation.
Direct identified completion needs its own narrow idempotent receipt, not an AI
proposal or a fake creation approval; durable completion also survives recovery.
Headless worker/CLI and native dashboard follow these qualified shared contracts.

## First-slice evidence — checked records, 2026-10-04

Implemented V10 Action records/read APIs without a production mutation route.
The meaningful missing-schema/startup-hook RED failed2tests before integration.
A separate real Store RED proved Action SQLCHECK violation silently restored an
older backup; removing four Action-only SQLCHECKs keeps validation semantic.
Independent review then found the populated unexpected-CHECK schema variant;
its fresh quick_check assertion passed before startup wrongly restored backup.
Lead accepted the finding and added a supported brandedV10+ owned-shape precheck
before quick_check. Physical NotADatabase/DatabaseCorrupt errors retain restoration;
older/foreign handling and historical table/classifier behavior remain unchanged.
No remaining actionable finding; independent final17Action tests passed.

Fresh final macOSarm64/Rust1.98.1 locked/offline Action17/0/0,Store241/0/0 and
Storeall-target Clippy passed. `TMPDIR=/private/tmp/brn-actions-root-zkv3jyu2 bash
scripts/verify-end-to-end.sh` passed retirement,format/build/all-target Clippy,
964workspace/0failed/3ignored+52fixtures. Combinednative (`native-ui,native-retrieval,
native-test-support`) passed213/0/0;test-support/shipping all-target Clippy and
shipping build passed. Two shipping startup/restart checks verified V10,empty
Action table,exact BOM/CRLF/Unicode vault bytes and zero credential files.
Earlier963/213 gates predated the review correction and are not final evidence.
Logs/metadata remain in the owned parent above; helper RED/Green/Store logs use
/private/tmp/brn-actions-store-*.log. Only upstream block0.1.6 future warning.

This commit locally integrates the independently reviewed/automated verified
storage foundation; checkpoint publication remains pending. No end-user Action
creation/approval/completion/dashboard or Stage6 completion is claimed. Manual
acceptance follows the first complete shared/UI Action flow. Next: typed draft/
edit/reject/replay, with explicit apply/Rewrite refusal until their extended
protocols and recovery semantics are qualified. Keep empty new serialized vectors
omitted so historical Markdown creation/journal/RewriteOutcome hashes stay exact.
Macmini requirements remain AppleSilicon/CLT,pinnedRust1.98.1,cached locked
libraries,protobuf/Bash/Python3,canonical syntheticTMPDIR and unlockedGUI.
No provider/model/private/original-data/release operations occurred.

## Stored Action review checkpoint — 2026-10-04

Baseline2cea3ebe36535f84f77f44ac136145748df45915. Exact typed Create/Replace members,
optional vault, immutable boxed full baselines, edits/comments/rejection/replay and
one64-member/8MiB budget are implemented. Old Markdown Draft/Edit/RewriteOutcome/
ApplyJournal hardcoded bytes/hashes survive. Meaningful RED exposed empty-file
application and owned AI-job admission; explicit Store/workflow guards now refuse
Action application, creation and owned Rewrite. No Action producer exists yet.
Independent complete review passed67Store+4workflow tests and6actual synthetic CLI
refusal probes, with exact review preserved and no Actions/applies/jobs/provider
or credential-file effects; no actionable finding remained.

A separate public probe reproduced newest malformed Action backup blocking an
older healthy terminal receipt. Regression RED failed1; supportedV10 now checks
complete owned shape/records before quick_check, returning semantic Invalid so
candidates are skipped and mains remain untouched. Qualified V9 Findings precheck
and2regressions are retained. FocusedActions18+Findings16 passed34; independent
work18+Actions18+Findings16 passed52 and a genuine Action B-tree corruption probe
restored exact Completed/v2 while preserving damaged-main and backup bytes.

Fresh macOSarm64/Rust1.98.1 locked/offline `bash scripts/verify-end-to-end.sh`
passed retirement/format/build/all-target Clippy,977workspace/0failed/3ignored+
52fixtures. CombinednativeDesktop passed213/0/0; both native Clippy configurations
and shipping desktop/CLI builds passed. Two shipping startup/restart runs retained
exact BOM/CRLF/Unicode bytes,V10 and zero credential files. Logs:
/private/tmp/brn-actions-root-zkv3jyu2/typed-final-*.log; independent recovery:
/private/tmp/brn-actions-store-39orx1m5/independent-recovery-f_8oetml.
Local56Markdown file/fragment links,format and diff checks passed. Only upstream
block0.1.6 future warning remains. GUI/owner acceptance is pending
for the future complete Action flow; current stored slice has no native producer.
Checkpoint PR/exact-head CI remain pending. Next: extend the existing ApplyJournal
with immutable complete Action after-state, atomic CAS/receipt settlement and
older-backup import; retain workflow/refusal guards until mixed recovery qualifies.
Macmini environment and all live/model/private/release boundaries above remain.


Stored review locally integrated08021fb1a0ab23b923ab0b0b8258fe975cd4c275.
Qualified PR25/8295326 then joins this checkpoint. Independent merge review
verified incoming3source files exactly and preserved all root16Stage5/Stage6B
code/evidence and ownerAGENTS edits; only two docs conflicts needed composition.
Fresh164native-workflow/0failed/2ignored (`--lib --test models --test library
--test ai_tools --test ai_tools_scopes --test knowledge_scopes --test proposals`),
workspace all-target Clippy,format/diff and38doc links passed. Last full977/213
stored-review gates above precede this separately qualified incoming source delta.
PR25 merged-main run37197827195 passed Mac3+UbuntuCore/UI; Ubuntu native retrieval
has three existing unsupported exclusive-install expectations, Windows3 Unix API
failures,overallCIred. Independent analysis found no sharedMac/client-boundary
defect; non-Mac-specific failures remain informational under owner policy.

## Next fixed Store slice: joined Action application and recovery

Baseline7b896ec9a2f52aa516ba21060e90c44fe9c7acf1. Extend existing ApplyJournal with
`action_records: Vec<ActionRecord>` (default/omit-empty): complete ordered after
records, derived exactly once from approved ActionChange members/stamp and captured
started_at_ms. Create retains initial approval/data/time,version1; Replace retains
immutable origin,advances signed revision once and preserves Waiting since-time
only while remaining Waiting. Capture time covers review and every before clock;
validate every exact after binding,overflow and separate bounded snapshot encoding.
Keep the existing total journal/mirror limits: combined proposal accounting bounds
escaped note bytes plus Action JSON; qualify maximal mixtures,not just tiny data.

Admission checks absent Create/full exact Replace in its SQLite transaction before
Applying. Applied settlement changes Actions,whole receipt,review and annotation
cleanup together; NotApplied/Uncertain change no Actions. Replay never repeats or
refreshes Action mutation. No public standalone setter. Recovery import uses the
same transaction: Applied after-state or already-real Replace baseline for earlier
snapshots can reconstruct missing/older work. Immutable origins must match; equal
versions require exact records,newer records win,conflicting forks refuse without
partial import. Reversed inter-proposal import order must remain safe. Existing
journal merge keeps all Action after bindings immutable; mutable review projections
ignore candidate ActionData while preserving IDs/full Replace before-records.
Action-bearing Undo remains explicitly refused until its inverse semantics qualify.

Keep workflow creation/approval/ownedRewrite guards and current UI unchanged in
this Store slice; lead owns mechanical empty-field adaptations across Workflow/
Desktop fixtures and future mirror/domain integration. Meaningful tests cover
Action-only/mixed proof binding,near-limit/legacy hashes,Waiting/time/revision,
stale admission,CAS race,SQL-trigger rollback across all state,terminal replay,
missing/older/reversed import,newer preservation and equal-version forks.
Then independent complete review/finding validation,Store/shared/native checks.
Only after this Store contract qualifies: workflow relationship validation,
vaultless application and whole crash/ordinary-mirror recovery before producers.

## Joined Store checkpoint — 2026-10-04

The fixed contract above is implemented. Thirteen Action application tests cover
ordered after-state/Waiting clocks/revision overflow, mixed and Action-only proofs,
maximum encoded mixtures, stale admission/full CAS, no-effect outcomes, terminal
replay preserving later Completed work, reversed/missing/older imports and whole
SQL-trigger rollback. Legacy Markdown wire/hash fixtures remain unchanged.

Independent review confirmed a concrete recovery defect: checking only after-state
allowed an Applied before-v2/data-A → after-v3/data-B snapshot to replace retained
same-origin v2/data-C. The meaningful regression failed before the correction.
Recovery now validates/imports the real Replace before endpoint first, then imports
Applied after-state in the same transaction. Fresh corrected regression passed;
newer work, missing/older imports and Completed protection retain their behavior.
The reviewer's cached public probe ran corrected code only and is not claimed as
an independent original RED. Final complete review found no remaining defect and
ran56tests (13Action apply+9Action review+34existing apply),all passed. Helper final
Store266/0failed/0ignored and all-target Clippy passed. Evidence:
/private/tmp/brn-actions-store-39orx1m5/action-apply-before-fork-red.log,
action-apply-final-{store,clippy}.log and independent-joined-actions-3_rj10_p.

Fresh final macOSarm64/Rust1.98.1 locked/offline
`TMPDIR=/private/tmp/brn-actions-root-zkv3jyu2 bash scripts/verify-end-to-end.sh`
passed retirement,format/build/all-target Clippy,991workspace/0failed/3ignored and
52fixtures. Combined-native Desktop213/0failed/0ignored,both native all-target
Clippy configurations and shipping desktop/CLI builds passed. Shipping startup/
restart passed twice with V10,exact BOM/CRLF/Unicode bytes and zero credential
files. Root logs:joined-store-*.log; startup:qualified-startup-u7zxriil under that
owned parent. Only upstream block0.1.6 future warning remains. No provider/model,
original/private-data or release operation occurred.

This checkpoint qualifies Store application/recovery only. Workflow creation,
approval and owned Rewrite guards stay in place; Action-bearing Undo refuses.
No user-visible Action flow/dashboard or Stage6 completion is claimed. Native/
owner acceptance will use the first complete shared/UI Action scenario. Next:
validate references/dependency cycles and exact before/source state, then qualify
Action-only vaultless and mixed file/SQLite execution, ordinary mirror retirement,
crash checkpoints and older/fresh restore before enabling producers. Checkpoint
publication/exact-head CI follows the qualified Stage5 language base. Macmini
requirements and open permissions/qualification listed above remain unchanged.

Joined Store locally integratedd3c330f6eb62f9e4f6a208fa9677c4b016718cc0, then
qualified published language main42722526009b6c986271afc75680828bc5287c02 joins.
Independent preservation review verified337/340 tracked mode/blob pairs exact;
only three documentation files differ. The startup conflict retains exact Stage6C
full ActionV10 and FindingsV9 prevalidation. All16knowledge slices, incoming
publication evidence and complete Action evidence remain; ownerAGENTS stays
unstaged. Fresh43local doclinks,diff/conflict checks passed. No production source
delta invalidates the preceding991/213 gates. Language exact-head Mac3+UbuntuShared
passed; main37201599486 passed Mac3+UbuntuCore/UI with known unsupported Ubuntu
exclusive-install and Windows Unix failures,overallCIred. Post-language24tests+
52fixtures,startup2 and tree equality passed. Next publish this checked Stage6
foundation/stored-review/joined-Store deliverable against that merged base, then
qualify workflow recovery before any Action producer.

## Next fixed workflow slice: Action recovery preparation

Baselinea1cc9b429ac70972c826c227d29073122c5724d5. Keep workflow creation,
approval/Rewrite guards and UI unchanged. Qualify ordinary recovery of already
typed Store snapshots before enabling the whole user-visible Action flow.

Allow vault=None only for validated file/source-free journals; mixed/bound records
retain all existing exact vault checks. A terminal Applied mirror reconstructs
Actions through checked Store import. Unsettled zero-file work has no Action SQL
effect before terminal mirror/settlement, so restart/reconciliation settles
NotApplied rather than accepting vacuous empty Applied file proofs. Do not certify
fresh no-effects on restart. Pending/mixed work retains the current-evidence fence.

Temporary mirror retirement must compare ordered Action Create IDs and complete
Replace baselines, while allowing reviewed candidate-data edits. Preserve foreign,
reordered, changed-baseline and malformed occupants; exact lineage can retire after
Applied success. Add meaningful before-fix regressions, vaultless pending/prepared/
uncertain/terminal replay, fresh/older operational restoration and mixed-vault
binding refusal. Lead owns startup/reconciliation; a bounded helper owns only the
retirement comparison/tests behind this fixed contract. Independent full review,
fresh workflow/shared/native qualification precedes local integration. The later
slice still must qualify references/dependency cycles, guarded execution, mixed
crash/repair and read parity before producers, followed by identified Complete and
the dashboard. No MCP,additional database/framework or live/private-data work.

## Workflow recovery preparation checkpoint — 2026-10-04

The fixed slice above is implemented. Valid before-fix fixtures passed1/failed4:
a bound prepared Action-only intent actually settled Applied from empty file
proofs; vaultless pending/terminal/fresh-restore cases refused vault access. One
initial multi-unresolved fixture violated existing exclusive admission and was
corrected before that meaningful RED; it is not claimed as a product defect.
Fresh corrected5tests passed, including Pending/prepared/Uncertain and exact
replay,terminal-mirror-before-SQL reconstruction/comment cleanup,both fresh and
physically damaged/older-backup restoration of a Create→Replace chain,and refusal
of another bound vault before any Action import. Original synthetic file/backup
bytes,immutable origins,Waiting time and zero credential files were retained.

Actual-envelope retirement regression passed1/failed1 before correction: all8
valid foreign binding variants were deleted. Exact ordered Action kind/ID/full
Replace baseline checks now preserve them, while real reviewed candidate edits
retire covered temporaries. New2tests and all20recovery-file tests passed. Final
independent complete review found no defect and ran private copies of5Action+
20recovery tests:25/0failed/0ignored; source hashes stayed unchanged. Evidence:
/private/tmp/brn-independent-action-recovery-jzy622fk; root action-recovery-
red-valid-fixtures.log/action-recovery-green.log and action-retirement-*.log
under /private/tmp/brn-actions-root-zkv3jyu2.

Fresh final macOSarm64/Rust1.98.1 locked/offline end-to-end gate passed998workspace/
0failed/3ignored+52fixtures,retirement/format/build/all-target Clippy. Focused native
workflow `--lib --test proposals --test models` passed148/0failed/2ignored;
combined-native Desktop213/0failed/0ignored,both native all-target Clippy
configurations and shipping desktop/CLI builds passed. Two shipping startup/
restart runs passed,V10,exact BOM/CRLF/Unicode bytes and zero credentials.
Logs:action-recovery-final-*.log; startup:qualified-startup-rpdthaxz under the root
owned parent above. Only upstream block0.1.6 future warning remains.

This is guarded recovery preparation,not whole workflow Action application.
Creation/approval/ownedRewrite remain refused; Action Undo remains refused.
Next validate references/dependency cycles,then qualify source-free/mixed execution,
source/Action CAS drift,crash checkpoints/repair,read fencing and exact native/headless
review before producers. Identified direct Complete and dashboard follow. GUI/owner
acceptance stays pending for the complete Action scenario. No provider/model,
original/private-data or release operation occurred; Macmini requirements remain.

First Stage6 checkpoint PR28 merged5d0e9f5944edce6dd7b350ea8dcedcccca87a541 at
2026-10-04T12:45:41Z after exacta1cc9b4/run37202673215 passed MacCore/UI/Retrieval+
UbuntuSharedCore. Windows Core failed before tests on known Unix-only APIs;
overallCIred. Merged source tree equals qualified candidate. Publication56Store+
4workflow tests,52fixtures,shipping builds/startup2 and66doclinks passed. Post-merge
31Store+4workflow tests,52fixtures and startup2 passed,V10,exact bytes,zero creds.
Logs:/private/tmp/brn-v1-stage6-actions-lxqn9mif,qualified-startup-2hdtn9e4.
Main37203224513 completed/failure:Mac3+UbuntuCore/UI passed;Ubuntu native retrieval
failed10pass/3fail on unchanged exclusive-install expectations,Windows3failed on
Unix APIs before tests. Independent actual-log/hash comparison found no shared
Action defect; evidence:/private/tmp/brn-pr28-main-ci-review-c5vr9poj.

Fresh38local Markdown file/fragment links,format/diff checks passed for this
workflow recovery checkpoint. Local integration precedes exact-head PR/CI/post
qualification; next slice and pending acceptance above remain authoritative.

## Next fixed slice: shared retained Action reads

Prior recovery checkpoint [PR29](https://github.com/ewq100/brn-rust/pull/29)
merged0243c5d6d1d18ff4d99aae1d8203540a3f704206 at2026-10-04T13:13:26Z after
exact9e785a5/run37204244962 passed Mac3+UbuntuShared;Windows Unix APIs failed,
overallCIred. Merged source tree equals candidate. Fresh post25workflow tests,
52fixtures and shipping startup2 passed,V10,exact bytes,zero credentials.
Evidence:/private/tmp/brn-v1-stage6-action-recovery-hzzm7xke/qualified-startup-1diwr4h0.
Main37204848736 passed Mac3+UbuntuCore/UI;Ubuntu installer10pass/3fail and Windows3
Unix failures repeat PR28. Independent actual logs:/tmp/brn-merge-ci-final.7RhFbR/;
no shared Action defect. Macmini needs macOSarm64,pinnedRust1.98.1,locked dependencies;
native owner acceptance and the whole Action scenario remain pending.

Baseline9e785a556360604df3c836d4ae76139178990a5d. Clients need the full exact
Action baseline before constructing typed Replace review input, so expose retained
reads through AppWorker before producers. Preserve Store's existing checked DTOs,
all-state/25-entry default,1–200 limits and immutable created-time/UUID pagination.
`actions::App::action(UUID)` returns the exact record or typed NotFound;
`actions::App::actions(ActionListRequest)` returns the checked page. Reject nil/
invalid cursors before reads. Both require current-evidence reconciliation but no
vault/model/provider; operational history/proposal inspection stays separate.

Fixed worker commands: `Action(UUID)` / `Actions(ActionListRequest)`; events:
boxed `Action(ActionRecord)` / `Actions(ActionPage)` with existing request UUID
correlation. Re-export semantic Action DTOs through workflow,never persistence
handles/SQL/index IDs. Desktop only adds mechanical exhaustive event handling;
the native dashboard follows. Keep all creation/application/Rewrite/Undo guards.

CLI uses `actions show UUID` and `actions list --state open|waiting|blocked|completed|all`
(defaultall),`--limit`(default25) and paired`--before-created-at-ms`/`--before-id`
cursor flags. Validate directly constructed invocations before authority access;
JSON retains exact DTOs, human text quotes all user strings safely. No mutation
subcommands. Qualify real worker/CLI parity,pagination/exact strings,restart,
pending Save/proposal fences,typed NotFound/invalid requests and zero provider/
credential effects with synthetic Store-approved fixtures. Independent review and
fresh shared/native gates precede integration. Then reference/dependency validation
and whole Action application/recovery,identified Complete and dashboard continue.

Implemented shared reads and the fixed CLI contract on baseline0243c5d. Five real
workflow/worker tests cover exact updated origin/baseline,restart,creation-key
pagination/filtering and pending/Uncertain Save/proposal barriers. Four real CLI
process tests and three parser tests cover DTO parity,paired cursors,typed errors,
pre-authority refusal and safe human rendering. Initial tests wrongly expected
Auth::open not to create its empty safe folder and failed to account for JSON
quote escaping; corrected without changing authority rules. One parser fixture
also required explicit ParseFailure handling instead of Debug-bound unwrap.

Independent Sol read-only review privately passed5workflow tests and reproduced
a valid P2: serde_json human rendering leaves DEL/C1 controls raw. Actual CLI
regression failed0pass/1fail before the fix; post-serialization human escaping
now quotes DEL/U0085/U009B and the regression decodes to the exact original DTO.
Fresh4CLIprocess+3parser tests passed; correction review found no remaining defect.
Structured JSON stays exact. Evidence:action-reads-controls-{red,green}.log under
/private/tmp/brn-actions-root-zkv3jyu2; review /var/folders/zb/wm5x0l6j0sqc2503006y96x40000gn/T/brn-action-read-review-m3xosmgk.

Fresh post-correction locked/offline macOSarm64/Rust1.98.1 shared gate passed
1010workspace/0failed/3ignored+52fixtures,retirement,format/build/all-targetClippy.
Native unchanged workflow/Desktop gates passed153/0/2 and213/0/0,both native
Clippy configurations; final CLI feature Clippy/build reran after the display fix.
Shipping native desktop build and startup/restart2 passed,V10,exact synthetic
bytes,zero credential files (qualified-startup-1r1kdmdg). Final51local Markdown
file/fragment links and diff checks passed. Logs:action-reads-final-*.log and
action-reads-postfix-*.log under the owned root parent. Native ignore-event wiring
adds no dashboard behavior. Guarded creation/application/owned Rewrite/Undo stay
unchanged; no producer/provider/model/private-data/release operation occurred.
CLI empty-page/typed-missing manual scenario is in its README; populated owner
acceptance follows approved creation. Exact-head PR/CI/integration qualification
is next; references,whole execution,direct Complete and dashboard remain next.


Shared-read publication PR30 is at1298ee78e6fc788e04e6b76c3634ae1c0c5f1a22
(source tree116f312f equals reviewed90e9193; connected GitHub publication after
CLI credentials expired). Run37208267069 passed both Mac native lanes; Ubuntu
Shared found a new test expectation error: ordinary recovery requires macOS,
so non-Mac reconciliation returns ToolRejected and keeps current reads fenced.
The corrected test retains every shared pre-reconciliation check and the exact
Mac NotApplied/success scenario,with explicit non-Mac refusal/fence assertions.
Independent Luna code/log review confirmed the correction and no shared Action
or Mac defect. Fresh Mac5Action tests and test-targetClippy passed. Original
Windows22Unix diagnostics match PR29 unchanged; overallCIred. Corrected exact-head
applicableCI/merge/post checks remain pending. Publication12tests+52fixtures+
startup2(V10/exact bytes/zero credentials)+51doclinks passed before that CI round.


PR30 mergedb5f7ce857e2fccf8c10eca0090ca253a385ac5f2 at2026-10-04T14:32:29Z after
correctedexact19ab4fe/run37209028073 passed Mac3+UbuntuShared; Windows repeated22
unchangedUnix API build errors,overallCIred. No GitHub bypass. Merged tree equals
qualified candidate; post5workflow+4CLI+3parser tests,52fixtures and startup2 passed,
V10/exact bytes/zero credentials (qualified-startup-7zhle3cl under the publication
parent). Main37209641370 completed5success/4failure: Mac3+UbuntuCore/UI passed;
Ubuntu installer10pass/3fail and Windows3Unix build failures remain. Independent
actual-log/source analysis found no shared Action defect. Next shared reference/
dependency validation and whole application/recovery,then explicit Complete and
dashboard. Current subscription catalog amendment supports bounded Luna-only
qualification; live/native owner acceptance and wholeStage5/6 remain pending.


## Private reference/dependency checkpoint

Baseline13895d9. Keep all creation/apply/ownedRewrite/Undo Action guards closed.
Qualify references before whole execution. related_person/project,sources and
thread are explicit managed Markdown note UUIDs with relation labels; they do not
create another entity registry or alias BRN chat IDs. Later Inbox/person/project
views assemble those approved knowledge concepts through the same boundary.
New UUIDs are compared per optional slot and source membership; only newly added
references require complete unique same-vault inspection plus an already-captured
immutable SourceVersion or exact valid same-draft managed bytes. Historical
unchanged references stay readable; explicit source/history/archive targets are
allowed with proof. Reuse the existing full-before-fingerprint knowledge overlay
for links and Action targets. Approval never silently captures new source authority.

Action dependency,parent and follows_up targets use exact UUIDs from reviewed
Create/Replace after-data or checked Store records. Dependencies and parent
hierarchy must each be acyclic,checked iteratively through reachable after-state;
do not impose one combined DAG or a completed-only follow-up restriction. Store
keeps changed-member full-record CAS; unrelated target completion does not need
a new frozen target-version DTO. Historical reads/replay/Undo remain unchanged.

Implemented private App::validate_action_references; ordinary fresh preflight
calls it after link validation but after the still-closed Action guard. Empty
ordinary Markdown requests return immediately. No public producer was enabled.
The corrected16-test stub RED passed4/failed12 from invalid cases being accepted;
actual16tests passed. Full workflow306/0failed/3ignored before final empty-path
wiring; final affected187/0failed/3ignored and all-targetClippy/format/diff passed.
Independent Sol complete read-only review found no actionable finding,verified
unchanged guards and shared link overlay,checked final logs/source hashes,without
rerunning Cargo. Explicit case-sensitive APFS qualification remains prior evidence
and was not rerun here. Evidence:action-refs-evidence.md,patch/logs under
/private/tmp/brn-v1-stage1-checkpoint-s3nawyyb. Fresh integrated/native gates and
exact-head publication follow; wholeStage6 is not complete. Next whole Action-only
vaultless/mixed execution,CAS/source drift,crash/repair and headless/native exact
review,then identified Complete and dashboard. No provider/model/private data.

Final reference checkpoint joined reviewed native-dialog PR32merge da2ac23.
Fresh locked/offline macOSarm64/Rust1.98.1 gates passed1035workspace/0failed/
3ignored+52fixtures,202focused-native-workflow/0failed/3ignored and216combined-
native-Desktop/0failed/0ignored. Native all-targetClippy in both configurations,
shipping build and startup2 passed,V10/exactBOMCRLFUnicode/zero credentials
(qualified-startup-78o2f_b2). Source hashes match independent review. Evidence:
action-refs-{shared,native-workflow,native-clippy,native-desktop,desktop-clippy,
shipping-clippy,shipping-build,startup}-final.log under the owned parent above.
No user-facing Action producer exists yet; manual acceptance follows whole exact
application. Next whole Action-only/mixed approval and crash/repair,then native
exact typed review,identified Complete and dashboard. Owner-requested screenshot
retention is indexed under docs/ui/screenshots/; Macmini requirements unchanged.

PR33 initial exactbc4bbad/run37213893288 passed both Mac native lanes; Ubuntu
Shared failed61pass/4fail because four new source/overlay tests assumed macOS
coordination. Production already refuses those operations with ToolRejected on
non-Mac. The narrow correction marks only those four and their NoteChange helper
as Mac qualification,retains12shared cases,and adds one explicit non-Mac fresh
source/reference refusal with exact bytes/zero proposals/journals/credentials.
Independent Luna read-only code/log review found no shared defect. Fresh Mac16
reference tests and test-targetClippy passed; final diff passed. Windows repeated
22unchangedUnix errors. Corrected exact-head CI/merge is recorded below. Evidence:
platform-correction-{tests,clippy}.log under /private/tmp/brn-v1-action-references-1xcn7scw.
PR32main37213574797 completed5success/4failure; actual logs repeat Ubuntu native
installer10pass/3fail and Windows22/22/14Unix errors,production paths unchanged.

## Whole Action application slice

PR33 merged7fed132 after correctedexact0882a30/run37214585531 passed
MacCore/UI/Retrieval+UbuntuShared. Windows repeated22Unix errors,overallCIred;
no GitHub bypass. Merged tree47f89319 equals reviewed candidate. Fresh post16
reference tests+52fixtures+startup2 passed,V10/exact bytes/zero credentials.
Main37215168696 completed5success/4failure:Mac3+UbuntuCore/UI passed; actual
failure logs repeat Ubuntu installer10pass/3fail and Windows22/22/14Unix errors
in unchanged production paths. No shared macOS defect was found. Evidence:
/private/tmp/brn-v1-action-references-1xcn7scw/qualified-startup-1jesx6gx and PR33.

Baseline PR33 merge7fed13295a4c9c2e633ef020e981042840f2246c. Admit exact typed
Action Create/Replace and mixed note+Action drafts through existing AppWorker/CLI;
64combined members/8MiB limits remain. Capture exact current Action before records,
validate references and preserve immutable creation replay. Only file/source-free
work may omit vault binding. Native full review/capture must retain and display
all Action data, immutable before/origin and local incomplete typing before guards
open; no separate mutation path, Action Undo or owned AI Rewrite is introduced.

Reuse existing admission,prepared proofs,ordinary mirror and atomic Store receipt.
Zero-file execution deliberately prepares an empty proof vector; restart without
terminal evidence remains NotApplied. Validate exact Action CAS before filesystem
effects and before any terminal Applied mirror, retaining settlement CAS. Mixed
source/CAS drift remains Uncertain/fenced after effects. Explicit Finish requires
source and Action eligibility; Restore remains possible without overwriting
competing Actions. Historical terminal replay preserves newer records/files.
AppWorker's serialized boundary remains the supported mutation owner.

Acceptance: vaultless and bound/source-only Actions,Create/Replace creation replay,
exact edits/comments/group capture,CAS/source refusal,all crash checkpoints,
terminal-mirror-before-SQL restoration,mixed Finish/Restore and current-read fences.
Verify native full fields and incomplete/late acknowledgement guards. Deterministic
synthetic tests precede producer enablement; independent complete review,fresh
shared/native gates and exact-head CI precede integration. Native live/owner
acceptance stays separate. Direct Complete/dashboard are the next deliverable.

## Whole application qualification checkpoint — 2026-10-04

Implemented the fixed slice above on7fed132. Fresh direct/refused paths check
complete Action baselines before effects and Applied mirror authority,retaining
transactional settlement CAS. The meaningful private RED passed5/failed3: the
closed producer and vaultless preparation were not yet implemented; a valid
competing Create also reproduced publication of an Applied mirror before CAS
refusal. All three paths are corrected. Exact creation replay and full Replace
preserve immutable origin/Waiting time; source-only binding and source/CAS drift
stay exact. Zero-file7 and mixed16 crash checkpoints cover ordinary interruption;
unfinished file-free work remains NotApplied,no Actions. Genuine mixed Finish
joins exact Create/Replace after-state; four admitted-Finish CAS races refuse at
repair-mirror/repair-verified. Restore preserves competing Completed records and
exact originals; replay/restart never repeat effects.

Native complete review retains all14fields,invalid raw input,full immutable before
records and late acknowledgements. Independent UI review reproduced blocked
group approval; valid native RED passed6/failed4. The correction captures exact
ordered Action/Markdown members,allows source-free vaultless members with one
bound vault,and preserves arrival/version/navigation/acknowledgement guards.
Independent complete Sol review found no actionable defect and privately passed
12recovery+2repair+2Worker+3CLI+10state tests. Source aggregate92e9c800; actual
per-file hashes:/private/tmp/brn-independent-action-review.GXNbQl/review-hashes.json.

A final actual CLI regression reproduced Activity '.'/omitted Action summaries
(1pass/2fail). The shared approved snapshot now counts Created/Updated Actions
without changing DTOs,paging or loading full bodies into pages. Independent
correction review passed3Activity+3CLI+1timestamp tests,no finding,source aggregate
965f1391. New whole application test suites qualify Mac-only recovery persistence;
shared read/validation/graph coverage stays. An explicit non-Mac source-free Draft/
replay with refused approval/no effects awaits actual Ubuntu CI. Fresh Mac5tests
and test-targetClippy passed; this is not non-Mac execution evidence. Independent
read-only correction review checked actual Unsupported→ToolRejected mapping and
pre-admission ordering,found no defect; three test blobs ce24eea2/010ca138/064d8749.

Fresh final locked/offline macOSarm64/Rust1.98.1 gates passed1059workspace/0failed/
5ignored+52fixtures,retirement/format/build/all-targetClippy;178focused-native-
workflow/0failed/4ignored,229combined-native-Desktop/0failed/0ignored,both native
Clippy configurations,shipping desktop/CLI builds and startup/restart2 passed,
V10/exactBOMCRLFUnicode/zero credentials (qualified-startup-g29j64sx). Four ignored
private crash entry points execute through matrices; the optional case-sensitive
APFS test is prior evidence and was not rerun. Other test counts remain scoped to
their exact commands. Logs:action-application-*-qualified.log,action-platform-*.log,
action-activity-{red,green,clippy}.log under /private/tmp/brn-v1-stage2-checkpoint-dqbmj4oy.
The README synthetic Action JSON and actual CLI commands passed with approval-only
effects,exact data/Waiting origin and process restart/replay,zero credentials:
manual-action-vtu46dn8/results.json. An initial manual harness omitted mkdir/data
and envelope unwrapping; corrected without a production change.

Native Action/owner acceptance remains pending; manual scenarios are in CLI and
Desktop contracts. Actual Luna Settings/model-consent dialogs now render on the
unlocked Mac; three safe originals are indexed in docs/ui/screenshots. Download was
declined,no model request; fresh human Connect is awaiting sign-in,with zero
catalog/inference calls at this checkpoint. Separate owner-requested clickable
sign-in URL/selectable code work is being built as a small UI checkpoint.
Exact-head PR/CI/integration remains next. Then identified Complete/dashboard and
approved follow-up creation continue; no Action-owned Rewrite/Undo,provider assets,
private data or release is claimed. Macmini environment requirements remain.


Whole Action checkpoint PR34 merged5f9033b9/tree3978cedd after exact87f3920/
run37218436437 passed Mac3+UbuntuShared. Windows22 unchanged Unix API errors
keep overallCIred; actual logs/base-source comparison found no shared Mac defect.
Post-merge tree equality,22focused tests (including exercised crash matrices),
52fixtures and shipping startup/restart2 passed,V10/exact bytes/zero credentials
(qualified-startup-c8_5x71s in the publication parent). Merged-main CI is pending.
Owner-requested sign-in URL/code controls are a separate small checkpoint; then
identified Complete/dashboard continue. Native Action acceptance remains pending.


## Identified Complete: next qualified operational slice

Baseline PR34 merge5f9033b9; owner-requested Auth PR35 is qualified separately
and will join before this slice integrates. Explicit user Complete binds operation
UUID and the full identified unfinished ActionRecord. It preserves all Action
content/origin, advances exactly one revision, clears Waiting time and records
monotonic completion/update time. Completed work is never reopened; follow-up
creation remains an exact new approved Action. AI tools do not gain Complete.

Use one narrow typed ActionCompletion request/after receipt in existing WorkStore,
with additive V11 checked storage. Fresh full CAS, bounded bytes/clock/version,
exact immutable replay and transactional record/receipt settlement are required.
The Store transaction holds SQLite write exclusion while workflow publishes exact
ordinary recovery evidence, so a competing writer cannot invalidate the checked
baseline between publication and settlement. No fake proposal, database or generic
workflow/framework. Checked mirror recovery follows ordinary approval import and
precedes AppWorker Ready/current reads; older/fresh SQLite must recover completed
work, reject incompatible forks and preserve newer Completed records.

First qualify the typed Store foundation and restore semantics without opening a
client producer. Lead owns migration/startup integration and subsequent workflow/
worker/client behavior; fixed-interface helper owns only completion module/tests.
Meaningful tests cover all fields/Waiting/clock, exact replay and payload reuse,
CAS/publisher/SQL failures, write exclusion, restart, older/fresh restoration and
semantic corruption refusal. Independent read-only review and fresh shared/native
checks precede integration. Then qualify the ordinary evidence transport and
shared Complete command, CLI/native dashboard with reproducible acceptance.
The application boundary remains workflow/AppWorker; UI/protocol clients never
write files/SQLite or infer real-world completion. No live/data/release action.


Next application contract: retain one immutable typed Complete snapshot in the
existing owned ordinary-recovery directory, sharing its descriptor/ownership/
no-follow/durability/proof mechanics. A specific completion envelope/filename
family does not become a general journal framework. The narrow Store publisher
runs under Immediate write exclusion; installed exact evidence survives a failed
SQLite settlement. On startup import ordinary approval records first, then checked
completion evidence before Ready/current reads. Re-establish artifact durability
before import; incompatible evidence refuses, and uncertainty fences current work
until exact retry/reconciliation. Immutable replay never re-infers completion.

Expose CompleteAction through App/AppWorker with exact operation correlation and
critical-mutation shutdown draining, then CLI and native identified controls.
Source-free operational completion requires no vault/provider. Fresh commands
respect existing current-evidence fences; bound completion replay precedes fresh
eligibility. AI read tools do not gain completion authority. Qualify publication/
settlement/crash/old-backup recovery and no-effect non-Mac refusal before exposing
this command. Native dashboard presentation consumes shared application data;
visual behavior alone stays in desktop. Reproducible explicit Complete/follow-up
acceptance accompanies the first complete shared flow.


Complete storage foundation is implemented and independently reviewed against
9609b7f. Helper10public+1privateSQLite_FULL tests and2V10 direct/backup upgrades
passed. The latter initially submitted a different optional approval-proof value;
fixture replay was corrected to exactSome([]), with no production change.
MissingAPIcompileRED is recorded as structural, not a reproduced runtime defect.
The16-case startup matrix refuses hash/canonical/index/oversize/rehashedsemantic/
retainedAction/schema damage without replacing main; invalid newest backup skips.

Independent Luna read-only complete review found no actionable defect. Reviewed
production blobs14bac328(ActionCompletion),d67f2fec(Action codecs) are unchanged;
final work/mod74a0345 and migrationtestf0cc9e36 differ only by Cargo formatting.
Fresh macOSarm64/Rust1.98.1 locked/offline retirement/format/build/all-targetClippy,
1072workspace/0failed/5ignored+52fixtures,233combined-native/0failed/0ignored,
both native Clippy modes,shipping Desktop/CLI builds and startup/restart2 passed,
V11/exactBOMCRLFUnicode/zero credentials. Four ignored private crash entries run
through subprocess matrices; optional APFS case-sensitive qualification is prior
evidence. Only known upstreamblock0.1.6 futurecompilerwarning remains. Logs
 action-completion-*-final.log and qualified-startup-idiwxuu7 under the owned
publication parent /private/tmp/brn-v1-stage2-checkpoint-dqbmj4oy.

This checkpoint qualifies storage/reconstruction only. CLI/AppWorker/native
Complete stays closed; ordinary evidence publication/startup/current-read fences
and exact retry/drain tests are next. Exact-head CI/integration precedes that
producer. No provider,model,private/original-data or release operation occurred.
Native existing Action/updatedlogin acceptance still awaits an unlocked Mac.

## Shared identified Complete and CLI — active checkpoint

Baseline PR36 merge21442bfb0ff9e15705208f9c1fc224c7de90a5d1/tree8f0a2372.
Its exactheadb991/run37221597895 passedMac3+UbuntuShared; Windows22 unchanged
Unix errors remain informational and overallCIred. Post-merge12publicStore tests,
52fixtures and startup/restart2 passed,V11/exactbytes/zero credentials. Main
37222463218 completed5success/4failure: Mac3+UbuntuCore/UI passed; actual logs
confirm unchanged UbuntuNative10pass/3fail ExclusiveInstallUnavailable and
Windows22/22/14 Unix gaps. No requirement bypass. Rootmain preserved ownerAGENTS.

The next coherent deliverable oncodex/v1-complete-workflow opens exact direct
Complete through App/AppWorker and CLI `actions complete --file PATH`. It retains
one specific immutable checked completion envelope in the existing owned recovery
directory, sharing descriptor/no-follow/ownership/durability/proof mechanics.
Fresh CAS and all bounds precede publication under the existing Immediate Store
transaction. Publication uncertainty or post-publication settlement failure fences
current work and held read tools. Retry/startup re-syncs and imports exact evidence;
approved snapshots import first, completion before credentials/model loading/Ready.
Outer worker ID equals request UUID; admitted durable Complete drains on shutdown.
CLI validates one strict bounded full before-record file before worker startup,
retains that exact prepared input through acknowledgement and identifies an
undelivered successful operation for retry. No UI-only domain logic or AI Complete.

Acceptance covers source-free/vaultless direct completion, all retained fields,
Waiting/timestamp/revision, valid stale CAS, UUID payload reuse, strict input,
unknown publication/settlement/current+held-tool fences, exact resync/retry,
current/older/backup/fresh databases at both published/settled crashes, startup
corruption refusal before Auth/model loading, command correlation and shutdown
draining. New related follow-up is separately approved, never reopens completed
work. Existing full-field proposal review remains unchanged; dashboard is next.

Initial missing-command compileRED is structural. Runtime tests caught invalid
test assumptions (changedv1data violates immutable creation; settling a pending
note approval needs full observations; missing SQLite may restore a backup before
Ready). Fixtures were corrected without production changes. Fresh5private+3worker+
1shutdown tests passed; eight crash/database cases and seven failure boundaries
execute within the private tests. The ignored child is exercised by that matrix.
The transport passed8new+20existing recovery tests; deleting its pre-rename proof
guard reproduced the foreign-stage failure before restoring it. Unknown outcomes
remain explicit; proofs do not promise exclusion of unrestricted uncoordinated
same-UID filesystem writers. No new framework/daemon/database is introduced.

Earlier observation, before final qualification: independent whole review and
final shared/native verification remained pending.
Manual CLI scenario is in its existing Action acceptance contract. Actual updated
Auth controls now run in the owner's unlocked session; safe startup/Settings
originals are retained. A stale CUA observation initially showed startup; the owner
confirmed Ready/Settings and a safe screenshot agrees. ScreenCaptureKit errors
prevented reliable automated interaction; human sign-in was then pending.
No new provider/catalog call had run at that point. Auth screens are excluded.

The owner subsequently confirmed the actual updated app is connected and the
link/code controls work. This establishes changed native/owner acceptance for
PR35. After normal desktop quit, the shipping CLI made both permitted catalog
calls successfully. The captured extraction retained no usable IDs or raw catalog
shape/length; it does not establish Luna unavailability. The bounded round stopped
at2/2catalog calls,0/2logical probes,0/18completions. No further account calls or
claim of actual inference/multilingual model qualification follows from this.

Final whole independent Luna read-only review found no blocking defect in the
ordinary transport, App/startup/fences, worker correlation/drain or prepared CLI.
Its advisory finding was orphan temporary stages after pre-boundary failures:
each is bounded at4MiB, ignored for authority, and preserved; accumulation is an
explicit limitation, not an automatic cleanup permission. Root verified the
conditional same-UID race comment against exclusive installation and post-proof
Unknown behavior; unrestricted uncoordinated same-UID writers are outside the
supported exclusive application ownership model. No review-driven feature scope.

Fresh final macOSarm64/Rust1.98.1 locked/offline shared gate passed1093workspace/
0failed/6ignored+52fixtures; combined native passed233/0/0; both native Clippy
modes and shipping Desktop/CLI builds passed. Two shipping startup/restart checks
passed,V11/exactBOMCRLFUnicode/zero credentials. The additional ignored private
completion child is exercised by eight crash/database cases. Full logs are
complete-workflow-*-final.log under /private/tmp/brn-v1-stage1-checkpoint-s3nawyyb;
startup fixture qualified-startup-07e75hdw. Final malformed-startup witness compares
checked complete records and foreign evidence, rather than incidental SQLite page
bytes after ordinary approval import; no production correction was required.

Manual headless acceptance: follow the CLI's existing Action scenario, export the
full shown unfinished record into a fresh operation request, Complete that exact
file twice, restart and read the unchanged Completed receipt/fields, then approve
a separate new related follow-up. The process/worker tests exercised this flow;
owner/native Dashboard acceptance remains pending. Exact-head CI/integration is
the remaining checkpoint gate; Stage6 is not complete. Next is shared Dashboard
queries and thin native identified Complete/new approved follow-up controls, then
Stage6 AI proposal/read tools. Inbox remains later. No new database/service/tool
framework or original/private-data/release operation was introduced.

## Shared Dashboard query — active contract

Baseline PR37 merge3af4526f7795f749855f4c6165c0351facd56f38/tree2cb6175c.
Exactheadbc2b737/run37225342866 passedMac3+UbuntuShared; actual Windows22 Unix
errors/messages/locations match baseline21442bf, with unchanged failing blobs.
No enforced requirement bypass. Post-merge28focused/0failed/1ignored+52fixtures+
startup/restart2 passed,V11/exactbytes/zero credentials. Main37225862216 completed
5success/4failure: Mac3+UbuntuCore/UI passed. All four actual failed logs match
baseline21442bf: Windows22/22/14 Unix errors before tests and UbuntuNative10pass/
3fail ExclusiveInstallUnavailable/TargetOccupied; failing source unchanged.
Rootmain keeps the owner's unstagedAGENTS change; retained screenshots are now
integrated and the second worktree's duplicate input is preserved in a named stash.

Outcome oncodex/v1-dashboard-query: one specific checked Store snapshot query,
exposed through workflow/AppWorker and CLI `actions dashboard`. No schema/new
service/database, ranking system or competing knowledge authority. Native view
and action/follow-up composition use this interface in the following deliverable.

- Default Active means Open/Waiting/Blocked. Other explicit filters: Open,
  Waiting, Blocked, Completed, Overdue, FollowUp and All.
- Resolve omitted first-page date to OS-local civil today inside workflow using
  the existing pinned chrono clock feature. Canonical dates remain year1–9999
  YYYY-MM-DD. Return the resolved date; any continuation requires it explicitly.
  Overdue means unfinished due_on<as_of; FollowUp means unfinished follow_up_on
  <=as_of. Date counts may overlap. No hidden priority or completion inference.
- Counts cover every checked retained record, independent of filter/cursor/limit.
  Pages use existing immutable created-time/UUID ordering and limit1–200. Each
  query is one database snapshot; pages are fresh observations, not frozen record
  membership across intervening changes. Decode one bounded record at a time.
- Entries retain the complete ActionRecord plus same-snapshot dependency states
  (missing remains explicit), in the Action's dependency order. No state mutation.
  Existing current-evidence fences cover all dashboard data before returning it.
- CLI validates date/filter/page input before opening authority, retains its
  prepared request through acknowledgement and consumes only application DTOs.

Acceptance: counts and matching entries beyond the first25 records; exact date
boundaries/leap days/Completed exclusion; all filters and stable tied cursors;
missing/unfinished/completed dependency observations; corrupt hidden row refusal;
invalid/no-date continuation before effects; existing completion uncertainty
fences; source-free/no-provider worker/CLI parity and restart. Independent whole
read-only review, fresh relevant shared/native gates, exact-head CI and post-merge
checks precede integration. Manual scenario: approve synthetic Open/Waiting/
Blocked Actions with past/today/future dates, inspect all dashboard filters with
limit1, carry returned as_of/cursor, complete one exact Action and refresh counts.
No native/owner acceptance is implied by headless tests. No account call, model
asset, private/original-data operation or release is part of this slice.

The shared Dashboard query is implemented, with Store7/workflow3/CLIprocess5 and
CLIunit7 tests passing. Existing private completion5/0failed/1ignored rechecked
its eight crash cases/failure boundaries and the new Dashboard uncertainty fence.
Focused all-target Store/workflow/CLI Clippy passed; pinned chrono clock builds
locked/offline with no lockfile change. Initial missing Store/workflow API compile
RED was structural, with one fixture-only borrowed-byte correction. CLI process
runtime RED reproduced unknown dashboard command/USAGE before implementing it;
that same scenario now passes through the real AppWorker. Full independent
read-only review is complete; final shared/native/shipping/startup gates passed.
Documentation diff and67local file/fragment links passed. No native controls,
owner acceptance, account call or completeStage6 claim follows from these checks.

Independent Sol whole read-only review found no actionable/advisory defect across
all modified/untracked source, tests and contracts. It validated the transaction,
all-row checks, scalar dependency snapshot, bound records/cursors, global counts,
local/explicit dates, fences and prepared CLI acknowledgement. Reviewed source
blobs1549a239(Store Dashboard),b1a4f1f1(workflow),3a0fb5cb(CLI),75431b22(library)
and testsccb1b9a1/e9f1bfb0/63ef79cf remain unchanged in root's hash comparison.
CLIunit7/0failed/0ignored,private completion5/0failed/1ignored and focused Clippy
passed. Fresh final shared gate passed1106/0failed/6ignored+52fixtures; combined
native passed233/0failed/0ignored. Both native Clippy modes and shipping Desktop/
CLI builds passed. Two V11 startup/restart checks passed with exact BOM/CRLF/
Unicode bytes and zero credentials, fixture qualified-startup-sbh7vzqz. Actual
logs dashboard-*-final.log remain under
/private/tmp/brn-v1-stage1-checkpoint-s3nawyyb. Known upstream block0.1.6 future-
compiler warning remains. Exact-head CI/integration is next; pending native/owner
acceptance does not prevent this shared checkpoint.

## Native Dashboard and identified Complete — next deliverable

Baseline PR38 actualmerge58a1b8f86b654cfa9da2c1bc4b349917884bd2fb/treecd7763de.
Exacthead3582c0e/run37228046322 passedMac3+UbuntuShared; Windows22 decoded errors
match baseline. Post15focused/0failed/0ignored+52fixtures+startup2 passed,V11/
exactbytes/zero credentials. Main37228721623 finished5success/4failure with Mac3+
UbuntuCore/UI passed and actual four failed logs equal previousmain37225862216.
OverallCIred; no shared Mac defect or enforced-requirement bypass.
Scope: guarded Dashboard navigation, eight shared filters, whole-snapshot counts,
older pages with returned date/cursor, complete read-only Action/dependency proof,
and captured explicit Complete. New/follow-up proposal composition follows.

Presentation holds query/view/page/selection generations and workflow DTOs only.
Refresh/filter changes invalidate old detail and counts; stale replies cannot
replace newer observations. Validate each page using the captured shared request.
The modal captures full before record and a fresh operation UUID once; confirmation
rechecks selection/generation/native guards. The outer worker ID equals that UUID.
Admission registers a typed transient attempt before sending, so even synchronous
refusal retains exact input. One completion mutation is admitted at a time.
Retain each attempted request, typed failure and validated terminal receipt across
navigation; a later different capture never destroys an earlier uncertain retry.
Explicit retries reuse the original UUID/full request without requiring selection
or a vault. No error wording proves non-application. Wrong receipt stays pending;
valid global acknowledgement refreshes visible Dashboard and cannot overwrite a
different/newer selection. Late failure cannot regress an acknowledged receipt.
Admitted Complete drains through existing shutdown; no cancellation/write shortcut.

Acceptance: stale/filter/date/cursor/malformed replies; immutable modal and drift
refusal; complete record/origin/ordered dependencies selectable and copyable;
wrong acknowledgement/pending preservation; error/navigation/exact retry and global
receipt correlation; real source-free worker approval→Complete→refresh/restart;
existing draft/comment/review/editor/quit guards; actual 480×480 widgets with all
controls reachable. Whole independent read-only review, fresh shared/native gates,
shipping builds/startup, exact-head CI and post-merge verification. Manual: open
Dashboard after approving synthetic waiting/blocked Actions, inspect every field,
complete one captured Action, refresh and inspect Completed, then restart. Native/
owner acceptance is recorded separately. No live call or original data required.

### Native checkpoint evidence — 2026-10-04

Implemented the fixed thin-client contract. Six state/real-worker and four native
widget tests passed, including actual 480×480 pane reachability/read-only copy,
full shipping modal capture/drift refusal, synchronous submission failure and
same-request copy/retry across navigation. The navigation runtime RED preceded
wiring; missing state APIs were structural RED. The real worker test qualifies
source-free approval→Complete→shutdown drain→restart/exact replay without a vault,
index or provider. Independent Sol whole read-only review found no actionable or
advisory defect; all nine reviewed Rust blob hashes remained exact through gates.

Fresh pinnedRust1.98.1/macOSarm64 locked/offline shared verification passed
1112/0failed/6ignored+52fixtures. Combined native Desktop passed243/0failed/0ignored;
test-support and shipping all-target Clippy, shipping Desktop/CLI builds and two
V11 startup/restart checks passed, preserving exact BOM/CRLF/Unicode and zero
credentials. Logs native-dashboard-*-final.log and qualified-startup-wcle9mhp
remain under /private/tmp/brn-v1-stage2-checkpoint-dqbmj4oy. Only the known upstream
block0.1.6 future-compiler warning remains. Local documentation links/diff checked.

Actual unlocked native observation used fresh owned vaultless fixture
/private/tmp/brn-v1-dashboard-ui-e2q1kcj1 and shipping binary SHA256
a8f5ebcd5d4fc64d61b04bc9d1481250bca151c902e4c61e1c176856071cc149.
Declined the optional model offer. Active showed Open1/Waiting1/Blocked1/Completed1/
Overdue1/FollowUp1; explicitly confirmed the captured Waiting Action once. Refresh
showed1/0/1/2/0/0, keeping Blocked state with its dependency observed Completed.
Completed selection disabled further completion. Normal quit, shipping CLI restart,
full record/immutable-origin/field-byte checks and exact same-operation receipt
replay passed; native restart showed the same counts. Eight safe original JPEGs
are retained in the [screenshot index](../../../ui/screenshots/2026-10-04/INDEX.md).
No account/provider/model download/original-data operation occurred. This records
native observation, not owner acceptance of the whole Action review flow.

Exact-head CI/PR integration follows; new approved Action/follow-up composition
and Stage6 AI read/proposal tools remain next. No completeStage6/V1 claim.

## New Action and related follow-up composition — next fixed deliverable

Baseline PR39 actualmerge783d9dd921f5689a9303c9c1cf344b2d23cb7432/tree4945b5bf.
Exactheada3c1a97/run37231170865 passedMac3+UbuntuShared; Windows22 actual decoded
messages/locations match unchanged baseline. Post10focused/0failed/0ignored+
52fixtures+startup2 passed,V11/exactbytes/zero credentials. Main37231724813 finished5success/4failure:
Mac3+UbuntuCore/UI passed. Windows22/22/14 and UbuntuNative10pass/3fail actual
decoded failures/locations equal baseline; unchanged failed source, overallCIred. Native Dashboard/Complete observation and eight
safe original JPEGs passed; whole Action review/owner acceptance stays pending.

Extend the retained initial DraftForm with one optional typed Action input rather
than a second submission/approval lifecycle. Stable proposal and Action UUIDs are
created once. Reuse SubmittedDraft/prepare/created/failed, exact payload replay,
source correlation, retained-generation/navigation/quit guards and full review.
Raw ActionFields retain all14 values, including incomplete/oversized typing;
Action title is also the proposal title. New Action is source-free/vaultless by
default; optional full saved-source captures use AppWorker ProposalSource(path),
never client filesystem/identity parsing. Keep ordered exact proofs; explicit
same-path recapture replaces that proof only. Accepted proof changes advance
input generation. Current source admission blocks submit/leaving until settled;
stale form/path/binding/read replies cannot replace a new capture. A sourceful
request needs a bound vault and current proofs; shared workflow owns validation.

Creation ACK compares ordered Action kind/UUID/full Replace baselines as well as
existing note/source bindings, allowing later reviewed candidate data. Pending
creation retains its entire submitted payload; later typing remains dirty. Exact
retry keeps proposal/Action UUID and request; an explicit separate form copies
complete raw input/proofs with fresh proposal/Action UUIDs. Pending creation
refuses discard/separation. Dirty Action-only/follow-up fields guard navigation.

Native New Action works without a vault/provider. Dashboard Completed selection
can seed a new Open Action with follows_up set to that exact identified Action;
no completed record is reopened or implicitly changed. Render/copy all raw fields
and complete source proofs, then create ordinary review work. Existing exact
review/edit/comment/approval/Activity remains the sole path to actual Actions.
No generic form/workflow framework, additional database or provider call.

Acceptance: raw-invalid input/guards/exact copies; pending/later typing/unchanged
retry/separation; multiple proofs/stale/same-path recapture and changed-source
refusal; malformed creation bindings remain pending while later review replay
acknowledges; real worker source-free approval/restart/new related follow-up with
zero Actions before approval; actual native fields/Create→Review→Approve→Dashboard.
Meaningful state/worker/widget tests, whole independent review, fresh relevant
shared/native/shipping gates, safe native screenshots/manual scenario, exact-head
CI and post-merge verification. Stage6 AI read/proposal tools follow afterward.

### Composition local qualification — 2026-10-04

The fixed contract is implemented through existing DraftForm/AppWorker/approval.
Meaningful runtime RED proved creation acknowledgement previously ignored Action
bindings; the new native navigation RED proved the missing New Action control.
Three pure form, three real-worker/state and five native/navigation tests cover
full raw retention, exact request/IDs/whole bindings, stale/misbound source proofs,
changed-source refusal, later typing, source-free approval/restart/follow-up,
480×480 reachability/copy and captured Discard drift. No Action exists before
approval; a related follow-up preserves the original Completed record.

Independent Sol whole review found no actionable or advisory defect. Fresh pinned
Rust1.98.1/macOSarm64 locked/offline shared gate passed1118/0failed/6ignored+
52fixtures; combined native passed254/0failed/0ignored. Both native all-target
Clippy modes,shipping Desktop/CLI builds and two V11 startup/restart/exact BOM/
CRLF/Unicode/zero-credential checks passed. Actual logs action-composition-*-final.log
and qualified-startup-_09k21zp remain under
/private/tmp/brn-v1-stage1-checkpoint-s3nawyyb. The first full native run failed two
fixture checks: the newly Draft-rendering probe hid the old sidebar control, and
an animated confirmation missed a simulated click. An optional probe preserves
that sidebar; reduced-motion matches pinned toolkit/existing tests. Assertions
remain intact, final fresh suite passed,shipping source unchanged after review.
Known upstream block0.1.6 future-compiler warning remains.

Actual UI qualification is pending because computer use reports the Mac locked.
Prepared fresh vaultless /private/tmp/brn-v1-action-composition-ui-sptdd5rq bundle
uses shipping binary SHA256
cbc1ab1c8e5d654774d9788123561a6a4f66a47d78c5b9c3934ca50e032c4793,
without test-support. No provider/model/private/original-data or release operation.
Owner acceptance remains separate; this does not complete Stage6/V1. Next: actual
Create→Review→Approve→Dashboard→Completed/new follow-up→restart when available,
checkpoint PR/exact-head CI/integration,then Stage6 AI proposal/read tools.


### Composition integration checkpoint — 2026-10-05

PR40 head2334deb1ca9f5a7bea349fe0cbc718fbafed79fc/treeba3333ef passed MacCore,
MacUI,MacNativeRetrieval and UbuntuShared in run37234485293. Windows Core's22
actual decoded Unix errors/locations match main39. OverallCIred; informational
under owner policy, no sharedMac defect. Fresh normal merge guards found no enabled
protection/rulesets or unmet GitHub requirement. Actual merge
40f1314c248177535d0c315272ae4a3fe7c623b6 has the exact reviewed tree; root/wt1/wt2
fast-forwarded, preserving owner AGENTS edits. Post25focused/0failed/0ignored+
52fixtures+startup2 passed,V11/exactbytes/zero credentials. Logs
 action-composition-post-*.log / qualified-startup-lupils4r remain in the owned
Stage1 parent. Main37235387243 finished5success/4failure: Mac3+UbuntuCore/UI passed.
Independent comparison found exact baseline Windows22/22/14 diagnostics/locations
and UbuntuNative10pass/3fail panic blocks; failed source blobs unchanged. Overall
CIred, no new sharedMac defect. Native observation/owner acceptance remain pending, with the unlocked Mac required only for interaction/screenshots.

## Owned Action-bearing Rewrite — next fixed deliverable

Baseline actual PR40 merge40f1314c248177535d0c315272ae4a3fe7c623b6/treeba3333ef.
Extend the existing strict result with ordered full `action_data: Vec<ActionData>`;
absent/empty retains legacy Markdown wire/hash compatibility. Decode closed typed
ActionData, then validate the complete ProposalEdit through the existing full
review validator. The thin Rig prompt describes complete note/Action data and all14
Action fields; order, IDs, kinds, full Replace baselines and source proof bindings
cannot change. No automatic completion or real mutation.

Remove Store/worker/native Action Rewrite guards only with coherent result tests.
Keep existing vault/tools/provider/model/effort admission. A source-free Action-only
capture has no proposal vault binding: allow it with a currently bound AI vault;
Some(other binding) still refuses. Valid stored vault=None already implies no note
changes or saved source proofs. Manual vaultless creation/approval stays unchanged.
Reuse whole capture hash/version, job/spec identity, replay-before-live, stop/drain,
atomic settlement, stale result and late raw typing guards. No provider calls,
new schema/database/framework/tool bridge or architectural change in this slice.

Acceptance: actual Store admission and real-worker RED; Action-only/mixed full
Create/Replace fields; invalid/partial/count/schema/Completed results refuse as a
whole with no real effects; captured comments/identities/before/source bindings
stay exact; later edits/comments defeat stale output; cancellation/read-lease drain,
restart/replay and legacy hashes retain behavior. Native Rewrite enable/dispatch
and full late Action typing must be witnessed. Independent read-only whole review,
fresh appropriate Store/workflow/shared/native/shipping/startup gates, exact-head
CI, normal merge and post checks. Actual native/live/owner checks remain separate.
Manual: open a clean Action-bearing review with a bound vault and explicit model/
effort, Rewrite, inspect every suggestion/temporary comment, then approve only the
exact intended version; Stop or later edits must preserve current review work.

Then add bounded fixed Action reads/proposal creation through AppWorker, preserving
admission fence and shutdown drain without client event theft or another SQL owner.


### Rewrite implementation and regression evidence — 2026-10-05

Store admission RED failed2, real-worker admission RED failed1 and native Rewrite
navigation RED failed1 before removing the coherent old Action guards. Complete
Action-only/mixed suggestion→exact approval→restart/replay now passes without a
provider/credential call; malformed/missing/unknown/duplicate/partial/count/domain/
Completed results refuse whole work. New Action tests prove later comments/edits
win and Stop waits for retained read leases before terminal settlement/replay.
All14 fields are explicit, including nulls; absence never means clearing a field.
Legacy Markdown omitted/empty Action result vectors retain exact persisted hashes.

A second actual native witness reproduced an existing correlation defect: Store
uses canonical chatgpt/copilot job keys, but desktop compared ChatGPT/Copilot
presentation labels. The fake prior state helper repeated that mismatch. Lead
validated code and the failing native refresh, then shared the existing exact
RewriteRequest::check_replay DTO check with clients and corrected test metadata.
Real canonical keys now accept while wrong job/stamp/model/effort/display-key
replies stay pending. Full late invalid Action widget bytes remain copyable and
conflicting until explicit discard; shipping button dispatch/failed submission and
modal/quit guards pass. No provider logic moved into UI, and no new write path.

Focused Store10/0failed/0ignored,workflow18/0failed/0ignored,nativeAction27/0failed/
0ignored and real Rig synthetic Rewrite wire routes passed before final whole
review/gates. Intermediate test-only missing imports/boxed DTO/Debug formatting
were corrected; they are not product failures. Fresh final independence/gates/
PR/exact-head CI remain next. Actual UI/live/owner qualification is pending, with
no additional live calls authorized in the exhausted catalog round.


### Rewrite local qualification — 2026-10-05

Independent Sol complete read-only review found no actionable or advisory defect,
checking12Rust files plus6contracts/status/plan files and reused validators. All12
reviewed SHA256 values remain exact through fresh pinnedRust1.98.1/macOSarm64
locked/offline verification: shared1128/0failed/6ignored+52fixtures;
combined native257/0failed/0ignored; native test-support and shipping all-target
Clippy,shipping Desktop/CLI builds and startup2 passed,V11/exact BOM/CRLF/Unicode/
zero credentials.75local Markdown file/fragment links,diff and format passed.
Logs action-rewrite-*-final.log / qualified-startup-br67m1a0 remain in the owned
Stage1 parent. Known upstream block0.1.6 future-compiler warning remains.

Implemented and automated verified; PR/exact-head CI/merge/post verification
remain next. Actual native/live/owner acceptance remains pending. No provider/
model/download/private/original-data/release operation occurred. The prior Luna
catalog round is exhausted; no availability or inference claim is made. Manual
Rewrite scenario above and Macmini environment requirements remain. Next: fixed
Action read/proposal callback bridge through AppWorker,then Inbox in roadmap order.


## Read-only Action tools slice — 2026-10-05

Baseline: PR41 actual merge21dd333b78e80802f55dd9819e8a060b7c3d1a49,
exact reviewed tree731e9d60. MacCore/UI/NativeRetrieval and UbuntuShared passed
latest headfc76f74/run37237422223. Windows22 decoded errors/locations match
main40f; overall CI red. Normal expected-head merge satisfied GitHub requirements.
Post30focused/0failed/0ignored+52fixtures+startup2 passed; V11/exact bytes/zero
credentials. Main37238156636 completed5success/4failure: Mac3+UbuntuCore/UI passed;
independent decoded Windows22/22/14 and UbuntuNative10pass/3fail full failures
match main40, with unchanged failed source blobs. OverallCIred; no new sharedMac
defect signature. Actual native/live/owner remains pending.

Outcome: Ask and Rewrite can read complete approved Actions using fixed
read_action(id) and list_actions(state?,limit?,cursor?) Rig tools. Workflow alone
parses UUID/state/opaque checked cursor, calls existing App::action/actions and
preserves immutable origin/full current fields/order/current-evidence checks.
Default lists all states, explicitly labeled on every record; limit1–20/default20,
cursor≤256bytes, complete JSON reply≤1MiB. Reject oversized replies rather than
truncate. Existing Current/Source/History/All note tools remain unchanged.

One narrow private two-variant request/reply on the existing application lane uses
the same shared admission mutex as shutdown. Release it before waiting. Every
admitted reply settles/refuses before Shutdown; no callback enqueues afterward.
Private channels never consume frontend events. Existing note Arc and DrainedTools
forwarding retain Stop/disconnect/model-change/quit read leases. Production calls
use spawn_blocking; no synchronous callback blocks the single-thread chat runtime.
No direct SQL, extra Store attachment/crate/dependency/schema, generic dispatcher,
mutation/Complete/approval/Save/account tool or live/download/private-data call.

Acceptance: actual registered Rig routes (Ask/Rewrite), real-worker full records,
state/page/cursor, strict malformed/nil/missing/overflow/oversize refusal, concurrent
private replies plus public events, current-evidence fence, queued/shutdown admission
and retained Stop/drain/restart with zero knowledge/Action mutations/credentials.
Independent complete review, fresh shared/native/shipping/startup checks and
exact-head CI/normal merge/post follow. Manual with synthetic approved Actions and
a qualified explicit model: ask for waiting work, inspect full origin/current data;
then Rewrite an Action review using evidence, inspect and approve separately.
Actual model/UI/owner qualification remains separate. Next: narrowly bounded
proposal creation through AppWorker, then finish Stage6 and start Inbox.

Independent review identified a concrete fatal-exit cycle: queued private reply
senders lived in the application receiver while local ChatWorker::Drop joined
retained read leases. The bounded actual-worker witness failed with Timeout,
released its synthetic lease safely, then joined; the reviewer finding is valid.
Fallible loop exits now reach the existing drain: close shared admission, refuse
queued private reads and correlated commands, retain discoveries, then join
chat/model work and settle discoveries. The witness passes with Storage refusal
before lease release and one terminal turn. No new workflow abstraction. Existing
note-schema tests now distinguish the five registered tools without weakening
Current/Source/History/All checks; new route coverage includes both Copilot dialects.

Corrected complete independent Sol review is clean; all ten final Rust SHA256s
are pinned. The full-record fixtures use real native approval/completion, so their
module is explicitly macOS-only; the five private RPC/error/fence/lease tests remain
portable. The bounded platform correction was independently checked. After it,
fresh full shared verification passed1139/0failed/6ignored+52fixtures. Native
Desktop257/0/0, nativeWorkflowModels199/0/5, both Clippy modes, shipping Desktop/CLI
builds and shipping startup2 passed; native/shipping production stayed unchanged.
V11/exact BOM/CRLF/Unicode/zero credentials and47local doc links passed. Ignored
entries are private subprocess helpers exercised by parent crash tests. Real model
assets/inference/UI/owner qualification remains separate and pending. PR/CI/merge/
post follow before narrow Action proposal creation. No calls/downloads/private data.
