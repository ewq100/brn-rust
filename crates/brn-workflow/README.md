# brn-workflow

Shared client-facing application boundary for desktop, CLI and future protocol
adapters: manual Markdown Save/recovery, knowledge operations and Rig chat/read/
search through AppWorker. Legacy production Store/Workspace/worker and brn-flow
paths are removed.

The [architecture client boundary](../../docs/architecture/overview.md#client-and-protocol-boundary)
permits a future thin read-only stdio MCP adapter. It maps capabilities to this
crate's commands/events; it does not open vault/SQLite files or implement ranking,
knowledge scopes, parsing, relationships, provider or proposal rules. External
search/read/list default to Current; other scopes are explicit. Retrieval remains
replaceable inside the core. No MCP implementation, daemon or remote service is
part of this amendment.

Search results expose workflow-owned `NoteHit` evidence (path, whole-note hash,
byte range, exact quote and relative score). SQLite passage IDs stay inside
retrieval for ranking/fusion and are not client identifiers. Domain scope and
relationship DTOs retain their current meaning; adapters must not depend on
retrieval storage details.

## Interfaces and source

[Application](src/app.rs), [commands/events](src/app_worker.rs), [chat lane](src/chat_worker.rs), [editor](src/editor.rs), [proposal review](src/proposals.rs), [proposal application](src/proposal_apply.rs), [durable provenance](src/knowledge/provenance.rs), [activity](src/activity.rs), [file adapter](src/files/mod.rs).

## Simple app owner and read tools

[`App`](src/app.rs) opens one `WorkStore` in a new explicit data folder and
exposes its `OpenReport`, including the restored backup. History and settings
work without a vault. A missing initial vault stays unbound; an unavailable
previously bound vault returns `VaultUnavailable` for reads. The first
`bind_vault` persists the canonical root only after Library/reader initialization
succeeds. It refreshes saved knowledge when no durable change is unresolved;
otherwise current evidence remains fenced for reconciliation. A different root needs a different data folder. Vault
and data folders cannot overlap.

Credentials must be absolute, outside Git repositories, the data folder and
the vault. `default_credentials_dir(data_dir)` supplies a canonical sibling
`<data-name>.credentials` (desktop `BRN-simple.credentials`); `Auth::open`
checks only that safe folder, never provider caches. There is no default
provider/model. `AppConfig.credentials_dir: Option<PathBuf>` uses an explicit
location when supplied, otherwise the owner's saved `ai.credentials_dir`, then
the safe sibling. The non-secret absolute location is persisted only by App's
owning lane; frontends never open an extra WorkStore to read settings.
Selection is validated and
saved atomically in one `ai.selection` setting. Explicit Copilot discovery
results go through `record_models`; `validate_selection` checks membership
without network or cache access.

`effort`/`select_effort` expose an explicit low/medium/high choice in `ai.effort`
without provider access. Fresh Ask requires a captured effort before admission;
setting changes cannot alter the owned turn. Paired history records that choice.
Existing unknown-effort turns remain readable and replayable before current
settings, vault or credential checks, without another provider request.

`notes` and `search` refresh before returning CLI reads; `note` reads exact
current knowledge bytes. Their `*_scoped` variants and AppWorker `ScopedNotes`,
`ScopedNote` and `ScopedSearch` explicitly select Current, Source, History or All.
Current excludes sources and historical notes; Source includes original sources
in any state; History includes historical knowledge and sources. All includes
every eligible classified note. Call `refresh` on explicit Refresh, focus and application
writes. `embed_pending(batch)` provides bounded-batch progress for the owned
application lane. [`AiTools`](src/ai_tools.rs) is `Send + Sync`, with a mutexed,
retrieval-owned read-only reader and a clone of Library's one
[`SharedEmbedder`](src/library.rs). Both use `search_index`: absent models flag
**every mode** keyword-only; present models keep Keyword non-downgraded, embed
Semantic once and fuse Hybrid's top 50 keyword/semantic passages. Model
identity/dimension mismatches are errors, never fallback.

Every tool revalidates visibility and fresh bytes. Search checks the hash,
UTF-8 range and exact quote before exposing any candidate; stale/removed
notes return safe `IndexStale`. Unsafe arguments return `ToolRejected`.
Reads preserve the exact UTF-8 prefix up to 50,000 bytes, including BOM/CRLF.
Lists validate component-only folders and note-path cursors, use exclusive
path-sorted keyset pagination and return at most 200 entries. `"work"` never
matches `"workshop"`. These APIs never write vault files.

## Typed proposal review foundation

[`knowledge`](src/knowledge.rs) exposes saved-current `NoteIdentity` inspection and
`PrepareNoteIdentity`, a read-only full-source preparation command. Explicit
nonnil note/proposal UUIDs and a title produce a complete ordinary Replace
`DraftRequest`, including exact before/source proofs. Preparation creates no
proposal/editor record and writes no vault content; existing CreateProposal and
exact approval admit/apply the reviewed bytes. Already identified or unsupported
metadata refuses reassignment. Ordinary fresh replacement preserves a recognized
target ID; direct manual Save and exact Undo retain their byte semantics.

`IdentityInventory` scans and reads fresh saved Markdown across current and archive
paths, including unmanaged notes. It reports duplicate UUID paths and incomplete
metadata/file inspection instead of trusting index timestamps or guessing.
`ResolveNoteIdentity` returns explicit Unique/Absent/Ambiguous/Incomplete with
observed matches/issues; unreadable evidence cannot certify uniqueness or absence.
These observations are not an atomic vault snapshot; later durable work still
captures and checks exact source versions. `EvidenceNote` opens explicit saved
source/history text, including top-level archives, through the separate read-only
`EvidencePath`. It applies existing bounds, UTF-8 and non-symlink checks without
opening an editor or changing current mutation authority. All three commands
respect unresolved Save/application fences.

Refresh reads full saved bytes, including archives, and derives UUID/classification
into disposable BRNI V3 rows. Optional `brn_kind: knowledge|source` and
`brn_state: current|history` default to current knowledge; top-level archives
always count as history. Malformed managed identity, classification or provenance is reported in
`RefreshReport.unreadable` and excluded from all scoped queries. Unreadable
evidence files/folders are reported and stale rows removed; unknown subtrees
keep identity resolution incomplete without blocking readable current notes.
Equal size/mtime never substitutes for bytes; equal hashes retain passages/vectors.
Scopes filter before ranking/limits, then returned passages and list rows recheck
exact saved class/hash/quote. Explicit EvidenceNote still reads malformed original
text. Queries never stamp metadata, open editors or mutate sources. AiTools maps
`brn_ai::ReadScope` explicitly to KnowledgeScope for the same scoped search/list/
read checks. Existing read calls default to Current. Owned chat/Rewrite leases
forward scoped calls and retain authority until every blocking read drains.
Native browsing/search controls use the same scoped commands; non-current
openings expose full read-only evidence while direct editing keeps its existing
authority.

`NoteProvenance` reads durable `brn_provenance` citations from ordinary Markdown,
then freshly resolves each source UUID across current/archive evidence. Matched,
Changed, Absent, Ambiguous and Incomplete outcomes retain the saved exact quote;
paths/hashes never substitute for logical identity. `CaptureCitation` binds a
caller-supplied saved hash and UTF-8 byte range to full coordinated source bytes.
`PrepareNoteProvenance` adds captured citations to complete ordinary Replace
review input, preserving old references and all unrelated target bytes. It
creates no editor/review record and refuses self-source preparation. All three
commands respect unresolved application/Save fences.

Archived evidence can supply read-only proposal source bindings; writable
destinations remain current VaultPath. Fresh normal approval checks newly added
or changed citations against unique identities, exact quotes and captured source
fingerprints, including full edits/Rewrite. Unchanged historical references need
not still match a current source; exact Undo and completed replay preserve their
existing authority. Sources remain checked throughout application/recovery.
Approved quotes survive index loss and inspection from a fresh operational store
with only the copied ordinary Markdown. Actual session Delete qualification and
native provenance convenience follow their roadmap slices.

`NoteLinks` inspects ordinary CommonMark inline/reference links in saved body
bytes, using Store's exact frontmatter body offset and pinned markdown 1.0.0.
Occurrence/used-definition proofs retain full-note UTF-8 ranges and exact quotes.
Contained relative Markdown paths and stable `brn://note/UUID` targets resolve
from a fresh complete identity inventory, with explicit unresolved/non-note/
external outcomes. Sources retain UUID/hash and separate identity certainty;
targets recheck saved bytes before reporting resolution. Inspection respects
current-evidence fences, never opens an editor/writes Markdown or starts network
work, and survives restart/index loss from ordinary Markdown alone. It refuses
4096-link/4 MiB destination-and-quote overflow instead of silently truncating.
These are observations, not a transactional vault snapshot.

`PrepareNoteLink(LinkRequest)` returns complete ordinary Replace review input
for one additive stable UUID link. The current consumer and saved target must
already have freshly unique managed identities and valid metadata; source/history
targets are permitted. The request binds the selected target's full hash, and the
draft captures both full source versions. Preparation preserves every original
byte, escapes the literal single-line label and confirms the exact appended AST
link outside unfinished code/HTML. Self-links and existing resolved links are
refused. No proposal or editor is admitted and no vault bytes change.

Fresh ordinary approval rechecks newly introduced stable UUID targets after
review edits or Rewrite. A saved target needs its exact captured source binding;
a target created/replaced in the same complete draft uses the exact reviewed
bytes and normal destination/before proofs. After-state identity inspection
accounts for filesystem namespace aliases and refuses absent, removed, ambiguous
or invalid targets before Applying. Unchanged historical UUID links are compared
without the public extraction-output caps; exact Undo and completed replay retain
their authority. This convenience command prepares links to saved targets; it
does not coordinate cross-proposal creation dependencies.

`Relationships(RelationshipRequest)` refreshes saved metadata, uses one fresh
identity inventory and rebuilds disposable explicit-link and inferred-provenance
edges. Each unique, eligible managed endpoint retains UUID/path/full hash; exact
source link occurrences/definitions or target provenance quotes remain attached.
Repeated proofs coalesce by directed endpoint pair and origin; self-links and
external/unmanaged targets do not create note-to-note edges. Inferred provenance
is a candidate, never an approved durable relationship. Changed/ambiguous or
incompletely inspected identities cannot become guessed edges. Endpoint bytes
are rechecked before caching; issues and duplicate paths remain visible.

Pages contain 1–200 edges, a matching total and offset, ordered by source path,
target path and origin. Existing scopes filter both endpoints before pagination;
Current is default, and All explicitly includes cross-scope connections. Every
workflow query derives from saved Markdown offline rather than trusting an old
target resolution. The cache is neither authority nor a transactional vault
snapshot. No AI, network, proposal admission or Markdown write occurs. Native
relationship controls follow separately; graph canvas
remains its later roadmap stage.

`proposal_source(path)` / AppWorker `ProposalSource` return the complete saved
text and trusted file fingerprint for initial review composition. The capture
uses current-evidence fences and the same visible Markdown/path/file checks as
proposal creation. It creates no editor/proposal record and writes no vault
content; creation subsequently checks the captured version. This full-note query
does not substitute a truncated AI read-tool excerpt for the proposed before text.

`proposals::DraftRequest` supplies typed Create/Replace/Trash intent. Workflow
captures trusted vault/parent/before identities and exact bytes, checks expected
source fingerprints and refuses occupied or aliased destinations. Creation replay
precedes fresh-vault checks and returns existing edited review work; a conflicting
initial payload cannot reuse its UUID. Review text/comments are operational work,
never current vault evidence.

Full-text edits, anchored or whole-proposal comments, explicit reattachment,
rejection and imported captured Rewrite results use one version. Changing an
anchored target leaves its old range unresolved; late results after newer edits,
comments or rejection fail. AppWorker owns and drains admitted review mutations.
Read/list/edit/comment work remains available without current vault access.
`validate_review_edit` provides pure full-result validation using Store's bounded
typed rules before the frontend queues an edit. Owned AI Rewrite uses the
lifecycle below. Native exact individual/captured-group approval and activity use
these same DTOs; native Undo/repair confirmations use the existing exact previews.

The current-evidence fence covers pending/Uncertain Save and proposal journals.
It refuses tools, note/list/search, refresh and embedding, including startup and
model activation; retained tools cannot read through the fence. Settling a Save
cannot clear a proposal's uncertainty. Pending proposals also refuse new Save,
Save Copy and reload, while keeping editor recovery and existing proposal review
readable.

`proposal_apply` exposes exact individual and captured-group approval through
AppWorker and the CLI. Preflight checks reviewed vault/parents/targets/sources and
all editor namespace/identity aliases. It fences retained read leases before
intent. All Create/Replace members are staged before effects; Trash retains its
original. Complete prepared proofs reach SQLite and a bounded, hash-checked
ordinary recovery receipt before coordinated exchange/exclusive installation.
Whole installed/displaced proofs and required file/directory sync precede one
Applied result. No-effect refusal is certified only in the fresh path before a
namespace attempt; interrupted/mixed proofs remain Uncertain. Replay writes
nothing. Group membership and versions are explicit; each proposal is a separate
unit and later members stop on refusal. Later editor typing keeps its old baseline
and buffer as a visible conflict until explicit reload.

Recovery receipts remain in the data folder across SQLite backups. Every startup
imports newest terminal evidence before earlier history and unresolved work,
then classifies pending file proofs before binding current evidence. Trusted
historical completion preserves subsequent owner bytes. Reconciliation checks
current sources for incomplete work, recognizes sources replaced/trashed by that
proposal, and never repeats installation. Approved annotation cleanup covers
current/prior journals, ordinary snapshots and compatible proof-checked temporary
snapshots; unexpected occupants remain retained. Native exact approval and
reconciliation use these same workflow rules. `ProposalRecovery` returns small
checked pending/uncertain summaries, validating one journal at a time;
`ProposalApply(UUID)` loads only the identified full historical snapshot. Paged
native Activity never retains every historical proposal body. Explicit CLI
`applies` remains the full operational inspection command.

## Owned AI Rewrite

`proposal_rewrite::RewriteRequest` binds a job UUID, exact proposal stamp, explicit
provider/model, low/medium/high effort and event generation. AppWorker's
`StartProposalRewrite` replays history before current-vault/account access, then
preflights the bound vault and read tools. Fresh admission captures the complete
proposal/comments on the checked owned AI lane. `ProposalRewrite` reads safe
job history offline. No automatic provider/model fallback or retry occurs.
Running replay returns AlreadyRunning history, preserving the original owned
generation; it does not subscribe a new presentation generation to that job.

Ask and Rewrite share one active owned job, cancellation registry and read-tool
lease drain. Stop withdraws queued Rewrite before admission or cancels active
work; provider disconnect and shutdown join retained reads before releasing
credentials/ownership. A consumed successful result can win a later Stop.
Running jobs become Interrupted at restart without resubmission.

The Rig adapter bounds full capture and buffered output to 50 MiB encoded JSON;
it never truncates the full proposal or emits raw Rewrite deltas. Strict output
contains exactly title and every member's full text/null. WorkStore additionally
enforces the decoded 1 MiB/member and 8 MiB aggregate review limits. Malformed,
partial or oversized results settle a safe failure without editing. Validated
result and terminal job commit together against the captured stamp/hash; later
review changes settle Stale. Rewrite changes operational review only. Temporary
comments/prompt/raw output are not copied into chat or durable job metadata.

`activity::ActivityRequest` projects successful Applied journals into readable
history through AppWorker and `brn activity list`. Default pages contain 20 entries
(1–100 allowed), newest approval time then operation UUID first; an existing
Applied UUID is the exclusive older-page cursor. Entries include the title,
Create/Replace/Trash path summary and operation/proposal/group/session identities.
The recorded time is approval admission, not an exact completion time. Note bodies,
comments, file proofs and raw tool transcripts are absent. One checked journal is
decoded at a time, keeping bodies out of the full-history ordering metadata.
Drafts/refusals/uncertain work cannot claim a successful durable change. Historical
activity stays readable when current evidence is fenced or its vault unavailable,
and ordinary recovery restores it without repeating old effects. Native activity
presentation uses the same paged projection and exact journal snapshots.

## Explicit Undo and Trash restoration

`preview_proposal_undo` returns the complete inverse of one Applied operation as
operational review data; it does not claim current file eligibility. `undo_proposal`
uses a fresh caller UUID and an immutable whole/single-Trash scope. The shared
AppWorker `PreviewProposalUndo` / `UndoProposal` commands return a typed preview or
existing `ProposalApplied` receipt. Admitted Undo drains on shutdown; AI has no
direct Undo tool. Whole Undo never silently selects a subset. Explicit single
Trash restoration can preserve later edits to other members of a mixed operation.

Fresh preflight checks the same bound root, parents, targets and editor aliases as
approval, plus every exact retained original. Preparation flushes and borrows its
inode instead of copying it, preserving original bytes/mode/ACL/xattrs. The same
whole prepared proof, durable mirror, coordinated installation, strict settlement
and recovery protocol applies. Unknown partial work stays fenced; reconciliation
and UUID replay never repeat namespace effects. A clean editor whose baseline/text
exactly matches the restored original may remain bound; dirty or incompatible
recovery refuses. Tokens, generations and queued later typing remain untouched.

Activity includes the source operation and optional original Trash member index.
The existing 64-member/8 MiB proposal limits apply. Old retained Trash is eligible
while its proofs hold; no timer/count silently purges it. CLI commands are described
in [brn](../brn/README.md). Native Activity shows the full historical inverse and
Applied snapshots expose restoration by original Trash member index. Current
eligibility is checked only on explicit confirmation. Native owner usability
acceptance remains pending.

## Explicit interrupted-operation repair

`preview_proposal_repair` returns the full approved draft, exact current
Before/Applied member phases and a capture hash. `repair_proposal` binds a fresh
attempt UUID, that hash and Finish/Restore direction. AppWorker exposes
`PreviewProposalRepair` / `RepairProposal` with typed previews/receipts; admitted
repair drains on shutdown. CLI [commands](../brn/README.md) use the same boundary.
AI has no direct repair tool.

Admission checks the bound vault, complete file pairs and retained editor aliases.
Finish checks sources against arbitrary current phases and installs only remaining
approved changes; Restore returns applied members to exact originals while
preserving unrelated external sources. A durable repair mirror precedes namespace
effects. Each coordinated move proves its current pair, uses exclusive installation
or exchange, flushes surviving files and parents, and proves the complete endpoint.
Unknown destinations/staging, dirty aliases or changed capture refuse. Comments
are deleted only on Applied completion; restored Draft comments remain for review.

The original first Uncertain receipt/proofs remain immutable until known whole
settlement. Up to 64 bounded attempts retain their exact requests and outcomes;
the latest outcome settles with the whole receipt. Replay, startup and reconciliation
never repeat namespace changes. Known terminal repair requires every exact Before
or Applied pair, including staging proofs; older/missing SQLite restores the same
checked history. Temporary repair snapshots retire only against compatible checked
canonical history. Editor baselines/generations and later queued typing stay intact.
Native repair offers explicit full Finish/Restore confirmations, displaying the
observed phases, complete approved draft and matching recorded comments. Each
attempt retains its exact hash/direction and reports the actual optional outcome;
failed attempts remain inspectable without automatic retry. Native owner
usability acceptance and physical power-loss qualification remain pending.

## Explicit model installation

[`models`](src/models.rs) exposes the pinned source, approximately 87 MiB cost
and destination. `prepare_model_download(consent, target)` persists
`model.download_decision` as approved/declined. Only a **fresh explicit
approval** creates a non-cloneable `ModelInstallRequest`; startup, search and
persisted approval never download. A later explicit action can override decline.
The default target is `<simple-data-dir>/models/minilm`; targets cannot overlap
the vault or a repository.

The worker consumes the request's synchronous `install(cancel, progress)` on an owned
blocking job, not on the GUI/application lane. It owns job lifetime,
cancellation and events: report `ModelDownloaded` on install success, drain
and detach idle chat/tool handles, then call `App::activate_model` on the
application lane. External tool handles cause `ToolsBusy`, rather than swapping
an active snapshot. Activation loads once, replaces both shared adapters and
invalidates vectors by model identity; subsequent bounded `embed_pending`
calls rebuild them. Emit `ModelInstalled` only after successful activation.
Invalid installed models remain `ModelInvalid`, not model absence.
Worker indexing is scheduled only with a loaded model and an available bound
vault, including between batches. Startup and activation still succeed without
a vault; missing indexing prerequisites do not produce a later correlated
failure. Binding an available vault or refreshing a restored bound vault resumes
bounded indexing. Actual embedding/index errors remain explicit `Failed` events.

Headless/default builds are keyword-only and ignore any saved `model.directory`
without changing the setting or assets. Only an explicitly supplied model
directory or Download action returns typed `SemanticUnavailableInBuild`
(`SEMANTIC_UNAVAILABLE_IN_BUILD`). Unsupported Download does not change consent,
and these builds do not offer an automatic download prompt. Explicit decline
still persists without network. Native builds enable `native-retrieval` and
continue to honor saved model directories and fresh consent.
The CLI and desktop simple editing/recovery/reads/history/AI actions use
AppWorker. Old and mixed authorities remain refused without migration.

## Simple manual Markdown editing

[`editor`](src/editor.rs) provides `open_editor`, `recover_editor`, `save_editor`,
`reload_editor` and `reconcile_editor` through AppWorker. `EditorView` separates
fresh saved bytes/fingerprint from the protected WorkStore buffer and exposes
conflicts and pending operations. Recovery remains readable when the vault is
unavailable. UTF-8, BOM, line endings and frontmatter remain exact; empty notes
are valid and text is limited to 1 MiB.

Save journals the generation-bound input before coordinated file work. The
shared private macOS adapter validates root/parent/file identity and vault
ownership, preserves attributes, uses durable sibling staging and atomic
exchange, and never recreates a missing original. Save Copy installs exclusively
at a distinct unused Markdown destination. A copy does not resolve or rebind the
original. Required coordination/durability failures have no weaker fallback.

Replay never repeats filesystem writes. Reconciliation checks prepared,
destination-parent and installed/displaced identities; matching text alone
cannot prove application. Unresolved saves fence current knowledge reads and AI tools.
Applied completion preserves later typing. One recent Applied recovery pair and
compact settled receipts remain after identity-proven artifact cleanup;
unexpected artifacts and unresolved payloads stay protected. Reload binds the
exact reviewed disk observation and requires confirmed discard of dirty text.
Already-admitted recovery/save/reload/reconcile commands drain on shutdown.
Stage 1 automated checks passed; native usability remains pending in
[status](../../docs/status.md).

## Owned application and chat lanes

[`AppWorker`](src/app_worker.rs) is the frontend handle:
`start(data_dir, AppConfig)` spawns before any SQLite open, vault scan or model
load. Opening errors arrive as `Failed`; startup emits optional `Restored` then
`Ready`. `submit(uuid, AppCommand)` and `try_event()` /
`recv_event_timeout(timeout)` use `(uuid, AppEvent)` results. Bind, selection,
local status, editor/recovery/save/reload/reconcile, notes/search, history,
model prompt/consent/progress, account commands and terminal errors all use this
seam. Recovery acknowledges SQLite's unfinished work; explicit Save writes Markdown.

The private [`ChatWorker`](src/chat_worker.rs) owns its runtime, attached
`ChatStore` and `Arc<Auth>`. Keeping admission private prevents frontends from
bypassing App refresh or discovery membership checks. App refreshes before
each **new** Ask, requires an available bound vault and explicit valid
selection, and checks conversation existence locally. Prior UUID replay comes
first: terminal replay is history-only even with an unavailable vault or
selection no longer in discovery; a Running record is `AlreadyRunning`, never
resubmitted. Different payloads/generations conflict. Outer submission UUID
must equal Ask/account operation UUID. Durable replay matches the recorded
question, conversation, provider and model; generation is a transient
navigation correlation, not persisted history.

Conversation projections include known `created_at_ms` and nullable
`last_activity_at_ms`; turns include nullable `started_at_ms`/`finished_at_ms`.
These are Unix milliseconds from checked WorkStore chat transactions. Historical
unknowns remain null. New admission and genuine finalization update activity;
reads, exact replay and startup interruption do not. Recovery does not invent a
finish time, and a nonpersisted partial has no trusted time. Existing
Conversations/Turns/Turn commands carry the same values without a new workflow.

`AppCommand::Turn(uuid)` / `AppEvent::Turn(Option<WorkTurn>)` is an owner-lane
lookup for CLI replay projection. Query Selection explicitly, but use the
recorded provider/model during replay even if current selection is obsolete.
`WorkflowError::recorded_ai_failure` projects the closed persisted AI category
to safe typed errors, rather than frontend wording classification.

One turn is active. Dispatch continues while turn/auth futures await. Stop,
account actions and installer cancellation bypass application work; chat and
transient account events are forwarded independently of scans/loading.
Disconnect fences its provider, cancels and joins only that provider's jobs,
then removes caches after clients/tools drain. Other-provider account work
continues. Cancel Connect is not Disconnect. Explicit discovery is recorded on
the application lane **before** its successful Models event is forwarded.
Account/model operation UUIDs cannot be reused to start another job.

`app::model_history` retains the last 20 earlier terminal text pairs, including
failed/interrupted partials. The chat lane snapshots them after the previous
turn's commit; Running turns are excluded. Rig receives no tool/provider
metadata and omits empty assistant text while keeping its question.
Every chat event includes turn UUID and generation; navigation must filter
stale events without preventing the worker from persisting their conversation.
`Finished` follows the terminal commit. `PersistenceFailed` contains the
in-memory partial and safe error: it does **not** establish saved text.

Installation progress uses the submission UUID. `ModelDownloaded` precedes
activation; active turns and all queued/running blocking tool leases must drain
before idle-only tool detachment. Loading happens once on the app lane.
Cancellation before model publication leaves the old model intact (synchronous
loading itself cannot be preempted). Tools are reattached before
`ModelInstalled`; vectors rebuild in batches of at most 16 between commands.
Indexing progress uses its startup/refresh/download submission UUID.
Activation failure remains visible and reinstalls the prior tool adapter.

Call `shutdown()` and handle its error: it cancels and joins admitted turn,
account and install work before releasing App/owner, including failed terminal
persistence or credential finalization. Repeated shutdown retains the outcome.
Drop also cancels/joins, never delegates to a detached reaper. Stop cannot
guarantee upstream cancellation or no billing; OS-killed processes lose
uncommitted stream text. Private `cfg(test)` external-operation adapters are
not provider features, public registries or CLI flags.

Focused offline checks:

```sh
cargo test -p brn-workflow --test library --test app --test ai_tools --test models --locked --offline
cargo test -p brn-workflow --features native-retrieval --lib --test models --locked --offline
cargo test -p brn-workflow --test app_worker --test chat_worker --test app --locked --offline
cargo test -p brn-workflow --lib simple_worker_tests --locked --offline
```

Native installer/workflow checks use synthetic assets and fake embedding
vectors, not production downloads or ONNX inference qualification.

## Dependencies and features

Only store, retrieval and AI are workspace dependencies. Default builds are
keyword-only; `native-retrieval` enables the explicit local embedding/installer
path. Private macOS files/coordinator types enforce manual Save without a generic
repository or workflow framework. Default and optional native checks remain
separate; see [verification](../../docs/development/verification.md).
