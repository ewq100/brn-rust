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
22unchangedUnix errors. Corrected exact-head CI/merge remains pending. Evidence:
platform-correction-{tests,clippy}.log under /private/tmp/brn-v1-action-references-1xcn7scw.
PR32main37213574797 completed5success/4failure; actual logs repeat Ubuntu native
installer10pass/3fail and Windows22/22/14Unix errors,production paths unchanged.
