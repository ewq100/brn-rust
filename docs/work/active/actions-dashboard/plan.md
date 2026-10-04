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
