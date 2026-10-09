# Bind actual sent evidence to explicit Action completion

Selected P7 slice, 9 October 2026 01:08 UTC. Reuse budget checkout on
`codex/p7-sent-evidence-completion`, baseline qualified ordering4fef278a4cdf9cbb92c8afbc98a606aacb95826e.
Parent PR104 has complete independent/correction review and all local gates passed;
required CI runs. Merge eligible parent normally before this candidate. Main
raw PR103d5f324e is merged; required post-main qualification continues independently.
Owner authorizes remaining small V1 slices until05:00UTC, no new live calls (16/16
spent), no GUI/private data/credentials/sending/account/release/config changes.

## Outcome and reuse

The owner can retain the actual externally sent text through existing Text Inbox
capture, protected Source preparation and exact approval, then explicitly confirm
that displayed Applied Source as the sent version while completing one identified
Action. An already-explicit Action.thread is required for this first slice. Keep
it unchanged; append the selected Source UUID to sources if absent. Approval of a
reply draft or Source alone leaves the Action open. The final owner command means
“I confirm these displayed Source bytes are the actual version I sent, and this
identified Action is complete.” No external sending, AI Complete tool, automatic
draft prefill, inferred delivery/authentication/thread or atomic Source-creation claim.

Reuse exact Text Inbox original/literal fence/Source approval, whole Action CAS,
V11 completion table, publication-before-SQL recovery/envelope, original journals,
source identity/fingerprint and native Dashboard completion captures. Source has
session_id=None; its approval/completion history is independent of chat lifetime.
Session Delete remains unimplemented; do not claim actual deletion qualification.
No new SQL table/schema/recovery engine/dependency/provider/runtime. Pinned Rig
has no role in this direct owner operation. Do not reuse InboxIntakeBinding:
plain Text Sources have no extraction snapshot. Reuse exact Applied Source receipt
pattern in intake_analysis_binding/validate_intake_dependency without that assumption.

## Fixed Store interfaces and transition

Public `SentSourceBinding { source_approval: ApprovalRequest, source: SourceVersion,
note_id: Uuid }`, deny unknown fields. Path is bounded to512UTF8bytes and existing
safe Markdown path rules; complete fingerprint is bounded to existing1MiB note.
Non-nil IDs/valid versions, valid fingerprint/path and compact encoded binding
<=8192bytes. Reuse existing validators. CompleteActionRequest gains
`#[serde(default, skip_serializing_if="Option::is_none")] sent_source:
Option<SentSourceBinding>` after existing fields. Historical omitted/None canonical
JSON/request/envelope hashes stay identical. Some requires unfinished before with
existing thread; validate the exact after data including source-count limits.

WorkStore::sent_source_binding(&SourceVersion)->Result<SentSourceBinding> is read-only:
find the unique exact Applied protected plain Text Source with this path/full hash/
length and original note UUID; mint the binding from its exact Applied journal.
No extraction requirement. Refuse wrong/ambiguous/missing receipt/metadata/type.
WorkStore::validate_sent_source_binding(&SentSourceBinding)->Result<()> checks that
same retained approval and protected Source path/id/full text hash/length. Internal
transaction check is reused at fresh completion before publication and recovery
import; host alone checks current filesystem. No filesystem read in Store.

For Some, completed() appends note_id once, preserves existing thread and all other
Action data/origin, advances existing version/state/times exactly as ordinary
completion. Source limit overflow refuses before publication. Exact operation replay
wins before fresh source eligibility; changed binding under UUID conflicts. Recovery
uses historical Source approval/text proof without requiring its current file or
silently restoring removed/edited Source bytes. Import Source/application records
before completion records; no absent Source lineage invention. Equal-version forks
refuse. Existing None behavior/vaultless completion stays unchanged.

Increase only the Store encoded completion reserve by bounded8KiB for this binding
(verify actual encoded extremes); current recovery envelope4MiB covers existing
~1.6MiB Action pair plus compact binding, so no envelope/schema change is expected.
Update old Rust literals with None only in owned crate; don't rewrite historical JSON.

## Fixed Workflow / CLI boundary

`PrepareSentCompletionRequest { before: Box<ActionRecord>, source_path: String }`,
strict and pure validate before workspace: unfinished exact Action, thread present,
source path safe/<=512. Host mints the new completion operation UUID.
`SentCompletionPreview { request: CompleteActionRequest, source: ProposalSource,
title: String }` and `validate_for(&PrepareSentCompletionRequest)` prove exact before,
selected path/fingerprint, nonempty bounded title, mandatory sent binding, complete
Source text length/hash and source UUID/classification observations. No text clipping.

App::prepare_sent_action_completion(&mut self,&request)->Result<SentCompletionPreview>:
current/uncertain/root fence, exact current Action before, guarded saved Source read,
read-only Store binding, unique saved identity/path, valid immutable original/body;
return complete review without effects. App::complete_action revalidates Some source
approval/current exact full fingerprint/unique ID/original before publication; None
unchanged. Terminal replay/restart uses retained historical proof first. Source changed
or ambiguous/missing refuses without new effects. Source is already retained if a
completion fails; do not claim Action definitely open for uncertain/changed outcomes.
Show “completion not confirmed; inspect current Action and retained attempt.”

AppCommand::PrepareSentActionCompletion(request) -> boxed
AppEvent::SentActionCompletionPrepared(preview), shared owner headless query.
CLI `actions prepare-sent-completion --file REQUEST.json`; use existing strict
regular/nonblocking bounded Action JSON reader and validation before state/credentials.
Existing `actions complete --file` accepts extended exact request; no inferred run.

## Fixed native ownership and behavior

Add a retained Dashboard “Sent Source path” input, never inferred from a draft.
Owner first captures/approves actual text with existing Inbox. Explicit read/prepare
uses selected exact Action plus path. Display full saved Source and Action baseline
with the returned approval/identity/fingerprint; final separately labelled confirmation
asserts actual sent version + exact identified completion. Existing ordinary Complete
remains independent and does not silently acquire a previously prepared sent binding.

AiState preparation captures dashboard view/page/selection/generation, before and
path. Wrong/late/malformed replies cannot replace preview or settle another intent;
validate complete preview at boundary. Navigation/selection/current changes block
stale confirmation; retained path, attempts/errors and exact retry remain recoverable.
Existing CompletionCapture/confirm/receipt checks carry optional binding exactly;
no automatic retry or Source re-import. Source read only has no OperationalWork effect.

## Acceptance / checks / resources

1. Approve an original reply draft, supply a different actual sent text through Text
   Source capture/approval, then bind only actual Source to completion. Draft/Source
   approval alone does not complete. Existing thread/other fields/origin unchanged.
2. Refuse changed Action/Source, wrong receipt/type, duplicate/missing identity,
   missing thread, count/size overflow and malformed input; no effects/publication.
3. Same exact completion replay retains time/IDs after restart and Source edit/removal;
   changed binding conflicts. Source/original bytes unchanged by completion.
4. Publication-before-SQL crash and post-publication failure settle from existing
   evidence, including older/fresh DB+Source journals, without inference/duplicate
   effects or resurrecting external Source edits/removal. Keep ordinary witnesses.
5. Default/omitted/None canonical old bytes/hashes unchanged; actual large encoded
   bounds work without truncated full-size crash/recovery/Undo qualification.
6. CLI process preparation/explicit completion/restart/replay/refusal and no inference;
   native state and real headless widgets full Source/exact confirmation, wrong/late
   ack, navigation/changed selection/error/retry/ordinary Complete isolation.
7. One complete independent read-only review; final applicable Store/Workflow/CLI/
   Desktop/default/native/Clippy/shipping/fixtures and actual required CI. Normal
   protected merge/resulting-main verification. GUI expected/pending in ONE morningtask.

Lead keeps selected model/effort and owns Workflow, CLI, shared docs/cases/integration.
One bounded Sol helper owns Store-only code/tests/README; one owns Desktop-only
code/tests/README. <=2active helpers, no recursion, no provider/GUI. Fix interfaces
before edits. Store helper owns sole Cargo target/budgets first for Store checks;
root/Desktop no Cargo until explicit release. Desktop helper may prepare against
fixed Workflow interfaces, then wait. Root owns all other literal None updates.
Stop/reassess on real authority/recovery conflict; routine technical choices proceed.
