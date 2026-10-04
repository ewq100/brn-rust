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
