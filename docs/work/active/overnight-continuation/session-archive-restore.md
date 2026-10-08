# Manual reversible session Archive/Restore

## Outcome, baseline and scope

Baseline merged main `b4e3a59a201b3e719df91278426d44388576959f`, PR94.
Branch `codex/p9-session-archive-restore` in the existing budget checkout.
Product Vision section 19.3 requires manual Archive and Restore; P9 is independent
of converters. Let the owner organize older conversations and reopen exactly the
same history and recorded budgets. Archive changes organization, never deletes
messages or captured knowledge, Actions, Sources, proposals, Findings or receipts.
Automatic 30-day archival, Delete/warnings, preferences and broad UX redesign are
separate slices. No inference is required to implement or qualify this behavior.

Reuse/adapt existing rusqlite Immediate transactions, checked operational records,
attached ChatStore, serialized ChatWorker admission/drain, AppWorker correlation,
strict CLI JSON readers and native history widgets. Build only the missing
lifecycle metadata/projection and controls. SQLite supplies atomicity; no new
dependency, scheduler, general workflow engine or duplicate history is needed.

## Fixed Store contract

Keep WorkConversation, WorkTurn and canonical AI captures/questions/recovery
envelopes unchanged. Add strict serializable types in `work/conversations.rs`:

- `ConversationState { Active, Archived }` and `ConversationFilter { Active, Archived, All }`, snake_case.
- `ConversationStamp { id: Uuid, version: u64 }`.
- `ConversationLifecycle { stamp: ConversationStamp, state: ConversationState }`.
- `ConversationSummary { conversation: WorkConversation, lifecycle: ConversationLifecycle }`.
- `ConversationLifecycleRequest { operation_id: Uuid, expected: ConversationStamp, target: ConversationState }`.
- `ConversationLifecycleReceipt { request: ConversationLifecycleRequest, before: ConversationLifecycle, after: ConversationLifecycle }`.
- `ConversationLifecycleResult { receipt: ConversationLifecycleReceipt, current: ConversationLifecycle }`.

Structs deny unknown fields. Validate nonnil UUIDs and positive SQLite-range
versions before mutation; checked increment refuses overflow. New same-state
transitions refuse. All existing/new conversations begin Active/version 1. The
next migration adds lifecycle rows and a checked exact operation/receipt table;
existing chat bytes and timestamps are untouched. Validate complete row coverage,
receipt identity/hash/transition and retained ordered version chain at open.

WorkStore and attached ChatStore expose `conversation_lifecycle(id)`,
`conversation_summaries(filter)` and `set_conversation_lifecycle(request)`.
The mutation uses one Immediate transaction: exact operation replay first,
otherwise exact expected version, opposite state and busy checks, then metadata
and receipt together. Replaying Archive after a later Restore returns the original
receipt plus current Active state and never repeats the transition. Changed input
for the same operation UUID refuses. No creation/activity timestamp changes.

Fresh Ask and Inbox turn admission check Active inside their existing transaction;
fresh Inbox reservation also checks a supplied conversation before provider work.
Fresh Rewrite checks the captured proposal's session inside its admission
transaction. Proposals without a session or with legacy session IDs absent from
conversations keep existing eligibility; never fabricate a conversation. Exact
historical Ask/Inbox/Rewrite replay precedes lifecycle restrictions. Archive
independently refuses target Running messages or Running Rewrite work. A retained
reservation without a turn does not permanently prevent archival. Recover/settle
existing work remains possible; archived sessions require Restore for new work.

## Workflow and concurrency

Reexport Store DTOs through a focused workflow module. Add commands/events:
`ConversationSummaries(filter)`, `ConversationLifecycle(id)` and
`SetConversationLifecycle(request)`; replies carry the matching filter/summaries,
lifecycle or `ConversationLifecycleChanged(result)`. Mutation outer correlation
must equal operation_id, and accepted commands are critical shutdown work.

Route mutation through a private ChatWorker command/reply, serialized with actual
Ask/Inbox/Rewrite admission. Conservatively refuse lifecycle mutation while any
owned AI job is active or draining, reusing the existing whole-lane busy boundary;
no new per-session job ownership registry is necessary for this slice. Store
transaction checks independently close cross-connection admission races. A busy
refusal does not cancel anything. Never wait for an active job to finish while
blocking the AppWorker lane it needs for tools. Cancellation keeps the fence until
all owned jobs/read/proposal leases and durable settlement finish.

Archive winning a race makes later fresh admission refuse with ContextStale and
"Restore this archived session before continuing." Admission winning makes
Archive refuse ToolsBusy. Translate Store errors deliberately, without exposing
SQL internals. Read-only evidence/history and captured proposal review/approval
remain available. Exact operation replay is read-only even while another job is
busy; distinguish replay from new transition before the live busy refusal.

## CLI and native behavior

CLI: `conversations list [--state active|archived|all]`, default Active;
`conversations show UUID`; `conversations archive --file REQUEST.json` and
`conversations restore --file REQUEST.json`. Subcommand must match target. Preserve
existing conversation/turn objects and add separate lifecycle metadata to JSON.
Validate bounded regular-file JSON, domain values and duplicate/unknown fields
before opening application state. Never infer expected version from a fresh read.

Native history rail: Active/Archived selectors, default Active, and explicit
Archive/Restore for the selected session. Archived history is readable and shows
"Archived — restore to continue"; new Ask/Inbox/Rewrite is disabled until Restore.
Keep selected history visible after acknowledgement. Preserve composer, editor,
proposal review and partial/error text; no automatic new session or inference.
Pending state captures operation, exact stamp, selected session and navigation/list
generation. Freeze lifecycle/filter/session navigation while pending. Validate
receipt and correlation; stale acknowledgements cannot navigate, change filters
or replace owner work. Only matching nonolder metadata can update. Guard list
responses against stale generations; old compatibility Conversations remains an
all-session query while new controls use the explicit filtered projection.

## Acceptance and qualification

1. Legacy migration, Active defaults and Archive/restart/Restore preserve identical
   UUID, turns, budgets, creation/activity times and every captured outcome.
2. Exact operation replay after opposite later transition, stale expected version,
   changed UUID input, invalid metadata and rollback never lose history or override
   newer state. Ordinary backup restoration preserves its recorded checkpoint.
3. Both admission race orders for Ask, Inbox reservation/turn and Rewrite;
   archived fresh admission refuses but saved replay remains readable without
   provider/vault access. Stop/timeout with blocked leases keeps archive fenced.
4. CLI malformed requests leave state unopened; native pending/stale list/receipt
   behavior preserves owner buffers and exposes readable Archived/Restore controls.
5. Meaningful Store/workflow/CLI/native behavioral tests, one complete independent
   read-only review, applicable final default/native/shipping gates, actual required
   CI, normal protected merge and resulting-main verification. Reuse unchanged
   relevant evidence and serialize Cargo across checkouts.
6. Add the complete expected interactive journey to the single morning UI task;
   no interactive observation or owner acceptance is claimed overnight.

Lead keeps the selected model/effort and owns integration. At most two active
helpers, no recursive delegation; fixed Store and desktop file ownership permits
bounded implementation while lead owns workflow/CLI/shared docs. Reassess only
for a concrete integrity defect or unresolved product requirement, then continue
independent authorized work. No live calls, private data, GUI, optional download,
port, release, destructive data operation or unrelated merge belongs to this slice.
