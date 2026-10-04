# brn-workflow

Shared application flow for desktop and headless use: simple manual Markdown
Save/recovery and Rig chat/read/search through AppWorker. Legacy production
Store/Workspace/worker and brn-flow paths are removed.

## Interfaces and source

[Application](src/app.rs), [commands/events](src/app_worker.rs), [chat lane](src/chat_worker.rs), [editor](src/editor.rs), [proposal review](src/proposals.rs), [proposal application](src/proposal_apply.rs), [activity](src/activity.rs), [file adapter](src/files/mod.rs).

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

`notes` and `search` refresh before returning CLI reads; `note` reads exact
current vault bytes. Call `refresh` on explicit Refresh, focus and application
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
AI Rewrite and native review remain subsequent Stage 4 slices.

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
snapshots; unexpected occupants remain retained. AI Rewrite and native proposal
interaction still need later slices.

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
presentation follows with the complete proposal review interaction.

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
in [brn](../brn/README.md). Native review/Undo interaction and
owner usability acceptance remain pending.

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
Native repair interaction and physical power-loss qualification remain pending.

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
