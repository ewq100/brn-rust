# brn-workflow

Shared authoritative application flow for desktop and headless use: imports, eligibility, index lifecycle, grounded answers, saved sessions, drafts and comments. Owns application coordination across adapters.

## Interfaces and source

[Workspace API](src/lib.rs), [worker commands/events](src/worker.rs), [draft workflow](src/drafts.rs), [comment workflow](src/comments.rs), [brn-flow CLI](src/main.rs).

## Simple app owner and read tools

[`App`](src/app.rs) opens one `WorkStore` in a new explicit data folder and
exposes its `OpenReport`, including the restored backup. History and settings
work without a vault. A missing initial vault stays unbound; an unavailable
previously bound vault returns `VaultUnavailable` for reads. The first
`bind_vault` persists the canonical root only after Library refresh and reader
initialization succeed. A different root needs a different data folder. Vault
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
The CLI's simple reads/history/AI actions now use AppWorker; desktop cutover is
separate. Legacy local editing/history remains guarded by Store mode checks.
No simple Markdown Save is added here.

## Owned application and chat lanes

[`AppWorker`](src/app_worker.rs) is the frontend handle:
`start(data_dir, AppConfig)` spawns before any SQLite open, vault scan or model
load. Opening errors arrive as `Failed`; startup emits optional `Restored` then
`Ready`. `submit(uuid, AppCommand)` and `try_event()` /
`recv_event_timeout(timeout)` use `(uuid, AppEvent)` results. Bind, selection,
local status, notes/search, history, validated `RecoverEdit`, model prompt/consent/progress,
account commands and terminal errors all use this seam. Recovery acknowledges
SQLite's unsaved edit, **not** publication to Markdown.

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
cargo test -p brn-workflow --test library --test app --test ai_tools --test models --test app_mode_cli --locked --offline
cargo test -p brn-workflow --features native-retrieval --lib --test models --locked --offline
cargo test -p brn-workflow --test app_worker --test chat_worker --test app --test notes --test note_recovery --locked --offline
cargo test -p brn-workflow --lib simple_worker_tests --locked --offline
```

Native installer/workflow checks use synthetic assets and fake embedding
vectors, not production downloads or ONNX inference qualification.

## Markdown notes

[`notes`](src/notes/mod.rs) re-exports the public editing types from
`brn-store::notes`. `Workspace::open_note(operation_id, vault, relative_path)`
enrolls existing regular, single-link UTF-8 `.md` files (including empty files,
up to 1 MiB). `note(id)` freshly observes disk; `saved` is never substituted
with recovery bytes. Changed content or identity reports Conflict without
rebasing the editing stamp or discarding protected work. Observation tokens
are durable and distinct from the editing baseline.

`save_note_buffer(NoteSubmission)` acknowledges generation-checked SQLite
recovery only; it does not write Markdown. `note_recoveries()` includes clean
notes and unresolved work; `note_recovery(id)` inspects one note (or returns
`None`). Both require no vault access. Note operations return
typed `NoteResult` failures independently of generic workflow errors.

`approve_note_snapshot(operation_id, note_id, current_file_state)` freezes exact
reconciled saved bytes into a dedicated managed source, never the recovery
buffer. Enrollment/copies do not inherit permission. Changed observations and
changed saves withdraw permission; unchanged saves preserve it. Unavailability
excludes a snapshot without treating stored permission as live eligibility.
Reload/relink/accept-current still reset permission even for equal bytes.
Reapproval uses a fresh saved observation and requires an index rebuild when
content/permission epochs changed. Replaying approval returns its old receipt,
not a new permission grant.

[`notes/eligibility.rs`](src/notes/eligibility.rs) owns the live predicate and
`source_projection`: freshly validated documents paired with explicit
Current/Shadowed/Changed/Missing/Unavailable/OwnedElsewhere/Uncertain summaries.
`source_states` delegates to that single observation pass. The mutable
documents-only `sources` wrapper fails with EvidenceStale rather than hiding
excluded rows. Exact-path legacy imports stay immutable but become shadowed;
managed-path imports/approval use the same snapshot helper, with no independent
shadowed-source reapproval. Unrelated legacy imports retain snapshot semantics.
Index/search deliberately use the valid approved subset; changed corpus or
epochs preserve whole-index IndexStale behavior with exclusion reasons.
`brn-flow sources` returns `sources` and `source_states`; worker/native rows
pair those same projections and label their last validated observation.

Provider handoff validates selected evidence and managed prior session evidence
before authentication/resume, again before submission, and at completion.
Streaming is provisional. Completion atomically retains provider status/text
and CurrentAtCompletion/StaleAtCompletion. A stale completed answer returns
EvidenceStale with its historical receipt and confirmed provider outcome,
including same-operation replay without provider access. New operations on
stale threads fail ContextStale and require a fresh conversation. History is
readable and labeled; Unqualified legacy history alone does not block resume.
Storage/integrity failures remain visible rather than becoming exclusions.

Opening the workspace itself does not open the vault. ID-only observations
lazily validate/acquire the registered root and hold ownership until workspace
drop; missing/replaced roots and another BRN owner produce Unavailable or
OwnedElsewhere views with explanatory messages and no fresh saved bytes.
Buffer edits and history remain available offline. A workspace is bound to one
vault; selecting a different root requires a separate data directory. Open
replay binds the caller's exact root/path inputs before filesystem checks and
then returns a fresh view, not stale saved bytes. Alternate case or Unicode
spellings of an already registered file are rejected with typed Conflict naming
the registered path, based on device/inode identity rather than content hashes;
they cannot allocate a second editing buffer.

The owned worker exposes open/observe, recovery list/show/buffer save, Markdown
save/reconcile/compare, confirmed reload/relink, exclusive copy, accept-current
and saved-snapshot approval actions. Outcomes retain submitted requests or
decision preconditions and their operation identity. Failed note terminals remain
`Err` and add typed `note_failure` detail, including phase, known filesystem
outcome and confirmed recovery; post-copy observation errors preserve the
already verified copy's outcome.

`Worker::take_note_notice` drains the Workspace's presenter queue independently
of terminals and progress snapshots. Consumers must schedule workflow observation
in the next idle slot, not read files on the GUI thread or infer eligibility from
a notice. Never drop the adapter/Workspace while holding that queue's mutex:
presenter Drop waits for callbacks that may be waiting for the same mutex.

Critical note-mutation admission is serialized with shutdown before queueing.
Admitted open/save/copy/buffer/reload/relink/accept/approval/reconciliation jobs
are drained and their owner joined, including queued jobs after closing begins.
Existing Import/SetApproval actions are conservatively admitted as critical too,
because managed paths/sources route through the same note-snapshot mutations.
`critical_note_pending` lets native close guard those jobs even without an open
note editor.
Cancellation does not abandon a filesystem handoff. Normal native close waits
asynchronously for durable acknowledgements; defensive shutdown/Drop still joins
the critical owner. The existing bounded detached reaper remains available only
for non-critical local work such as native model loading; provider cancellation
and owned-child joining retain their existing semantics.

Plain enrollment still supports emoji filenames, including variation selectors
and zero-width joiners, regardless of registration order. Copy/relink destination
qualification is not applied to unrelated ordinary opens. Confirmed relink also
vetoes identities belonging to another note's baseline, durable observation or
fresh registered-path observation, including externally moved files.
Pending copy destinations remain reserved by exact path and computable
same-parent name aliases even if an external occupant has a different inode or
unsupported bytes. Enrollment cannot consume the preallocated copy identity's
namespace or prevent later reconciliation.

`save_note(NoteSubmission)` explicitly saves the original Markdown path:
durable recovery intent, exclusive staging, coordinated baseline revalidation,
atomic exchange, installed/displaced identity verification, then a durable
receipt. Equal-generation recovered text is accepted; no-op saves preserve
identity, timestamps and permissions. Successful saves rebase the editing
baseline without overwriting later typing. File flushing requires
`F_FULLFSYNC`; directory durability uses plain `fsync`, with no power-loss claim.
Late external races retain the actual displaced object and report conflict or
uncertainty, never automatic rollback.

Live pre-exchange refusals resolve NotApplied with freshly observed original
baseline proof, even when staging creation failed before its identity was recorded.
The failure result remains the immutable replay result; submitted recovery and
unexpected staging occupants stay protected, but a new original save is allowed.
When live progress proves no exchange, an observed external change remains
Conflict/NotApplied. Its intent stays Unresolved and blocks later original
saves pending explicit resolution; its note view reports Conflict, not an
uncertain execution outcome. An unrecorded artifact is retained as
RetainedUnexpected, never removed by filename. After restart, journal phase alone
cannot establish that exchange was never attempted; missing execution proof
remains uncertain.

`reconcile_note_save(operation_id)` classifies interrupted writes and commits
metadata only: it never retries exchange, creates, renames or unlinks files.
Matching bytes without execution identity proof remain uncertain. Compact
receipts/refusals replay even without a vault or full retained intent.
Artifact cleanup is separate, best-effort bookkeeping after terminal proof
and durable recovery, preserving unexpected occupants and unresolved inputs.
`compare_note(id)` shows exact baseline/local/fresh disk bytes (or deletion)
and an `observed_file_state` token without rebasing or writing files.
`reload_note(op, id, expected, discard)` checks the editing stamp and requires
confirmation before discarding dirty/pending work. Changed identities require
`relink_note(op, id, expected, relative, confirm_identity)` instead; relink
retains local text/generation and never guesses identity from content hashes.
Both decisions establish a new baseline token, invalidating queued old edits.
They do not resolve an unresolved original save.

`accept_note_disk_state(ack_op, save_op, observed_file_state)` explicitly
acknowledges a reviewed, freshly revalidated disk state after an original
save's recorded conflict/uncertainty (use reconciliation first for an interrupted
intent without a result). It cannot acknowledge an active job. This is
metadata-only, preserves local work/generation and protected recovery, and
does not rewrite the original outcome, retire uncertain artifacts or grant
search permission. After a confirmed relink it reviews the current registered
location, leaving the original intent's destination and artifacts unchanged.

`save_note_copy(submission, relative)` reserves an independent destination/new
note ID before staging and installs only with `RENAME_EXCL`. Occupied,
registered-missing and possibly aliased destinations are refused with protected
input; no overwrite fallback exists. Copies never inherit search approval.
Copy receipts identify the source and new target separately. An unresolved
original can be rescued without resolving it or suspending the original's
otherwise unchanged eligibility. Copy restart classification requires the
recorded prepared identity and consumed stage, not matching bytes alone.
Live failures release a copy reservation only when installation was not attempted
and the destination is absent/not the prepared identity, or the exact recorded
stage is proven unconsumed. The latter also permits metadata-only reconciliation
after a collision or transient observation failure. Recorded failures replay
unchanged, including historical Unknown outcomes; submitted recovery and artifacts
remain protected. Missing proof leaves the destination reserved.

Name reservations use canonical Unicode decomposition/full case folding plus
resolved parent identity as conservative **vetoes**, never identity proofs.
Qualified local APFS/HFS volumes advertise the required capabilities; other
volume families and unqualified invisible/control names are rejected.
That rejection applies only to copy/relink candidates. For a registered original
with an unqualified name, validated existing identities can disprove aliasing;
otherwise reservation checks conservatively veto only candidates in the same
resolved parent (or when the original parent cannot be resolved). Copies into
a distinct validated directory remain available, including recovery of the
emoji-named note itself. A conservative veto is Conflict, not an Unsupported
error propagating from the registered name.
Case-sensitive volumes may be deliberately over-rejected. Replays bind their
recorded destination, even after relink or completed-payload pruning.

## Dependencies and features

Depends on `brn-store`, `brn-provider`, `brn-retrieval`. Default features are empty; `native-retrieval` forwards to retrieval `native`.

## Verification

Run from the repository root:

```sh
cargo test -p brn-workflow --locked
cargo test -p brn-workflow --lib worker:: --locked
bash scripts/verify-end-to-end.sh
```

Tests cover flow, drafts, comments and provider-free Markdown note observation,
ownership and recovery. Run the note checks with
`cargo test -p brn-store -p brn-workflow --test notes --locked`.
Save/restart checks are `cargo test -p brn-workflow --test note_recovery --locked`
and `cargo test -p brn-workflow --lib notes:: --locked`; lib tests kill/reap
their own checkpoint-acknowledging children, with no production crash switches.
Conflict/copy decisions are covered by
`cargo test -p brn-workflow --test note_conflicts --locked`; this records the
fixture volume's actual case/normalization equivalence instead of silently
skipping name-collision assertions.
Keep durable authority in store and coordinate provider/retrieval through their
adapters. Preserve evidence validation, operation identity and late-response safety.

Read the [architecture overview](../../docs/architecture/overview.md), [invariants](../../docs/architecture/invariants.md) and [verification guide](../../docs/development/verification.md) before changing contracts.
