# Open and safely edit real Markdown notes

Date: 1 October 2026

Status: Four design sections and the written specification approved on 1 October 2026. The [implementation plan](../../work/active/markdown-note-editing/plan.md) subsequently landed through PR #13 at `main@6323e53`; [qualification](../../work/active/markdown-note-editing/evidence.md) is partial and native/user acceptance remains pending.

Supersession: The [Rig-first reset specification](2026-10-01-rig-first-architecture-reset-design.md) replaces this document's stronger coordination/exchange save protocol with basic editor-grade saving for the future reset. The stronger protocol remains implemented until explicitly changed; this documentation integration changes no product behavior. This document retains the earlier approved design as history; its stronger guarantees must not be attributed to the reset.

Inspected baseline: local `main` at `4f059881f5a34d22ffdf1bb53e79954c94c8608f`, with a clean worktree before this documentation change.

This was the first implementation-design slice of the [Markdown-first direction](2026-09-30-markdown-first-ai-workspace-design.md), not authorization to migrate a vault, run a live provider, merge, or release. At the inspected baseline, the architecture was SQLite-first. The current [architecture](../../architecture/overview.md) and [invariants](../../architecture/invariants.md) now describe the implemented live-file authority for managed notes without rewriting historical evidence.

## 1. Outcome and decisions

Open `plan.md` in a chosen local vault, edit it, and save directly to the same Markdown file. Another editor sees the saved content without BRN or its database. External changes and uncertain saves preserve recoverable work and produce explicit states rather than silent overwrite, recreation, or replay.

| Decision approved in this brainstorming | First-version behavior |
| --- | --- |
| Save behavior | Explicit Save/Cmd-S writes Markdown; automatic editing-buffer recovery does not publish. |
| Environment | One local vault on this Mac; other local editors may edit its files. |
| Conflict resolution | Compare, explicit reload/discard, or save a separate copy; no force overwrite or automatic merge. |
| Save protocol | Durable intent, macOS file coordination, preserving atomic exchange, verification, durable receipt. |
| Concurrency limitation | Coordination serializes participating writers, not arbitrary direct writes. A late race can be detected after the submitted file is already installed. No lossless simultaneous-editing claim. |
| Ordinary recovery | One rolling buffer and one most-recent successful-save recovery pair per note. Unresolved work never automatically expires. |
| Current evidence | Changed content loses prior version-bound permission; an obsolete database/index copy cannot stand in for it. |

### Included

Choose a vault, open an existing regular UTF-8 `.md` file, edit, protect unfinished edits, save, recover after restart, compare a conflict, reload, and save a separate copy. Desktop and headless interfaces enforce the same rules. Empty files are valid; the existing 1 MiB editor limit applies to note and submitted text, measured in bytes.

Preserve exact UTF-8 bytes, BOM, line endings, and frontmatter unless deliberately edited. No mandatory IDs, frontmatter conversion, Markdown formatting, or Unicode normalization. Opening and saving unchanged content must not modify the file.

### Excluded

Synced/network vault qualification, multi-Mac collaboration, automatic merging, general new-note creation, broad vault discovery/indexing, AI-review application, archive, Git history, bulk migration, global history-retention policies, backup/restore, and live provider qualification. Save-copy is a recovery operation, not a general note-creation subsystem. Existing standalone drafts and review candidates remain separate.

The rolling ordinary-recovery policy does not authorize deletion of existing versions, conversations, comments, drafts, or pinned history.

## 2. Authority, identities, and the workflow module

| Data | Authority |
| --- | --- |
| Current saved note content | The actual registered Markdown file. |
| Note identity, vault/path association, search permission | Persistent SQLite registry. |
| Unfinished manual edits and their starting content | Durable SQLite editing buffer. |
| Pending save inputs, recovery content, outcomes and receipts | SQLite records plus operation-owned filesystem artifacts until safely reconciled. |
| Explicitly retained original revisions and citations | Existing immutable records, never retargeted to changed content. |
| Search snapshots/indexes | Derived from eligible current content; never a competing content authority. |

A note-editing **module in brn-workflow** provides the shared interface for enrollment/open, state observation, buffer recovery, save, conflict comparison, reload, copy, and reconciliation. Its private macOS filesystem **adapter** owns coordination and filesystem operations. `brn-store` owns validated migrations and transactional records. Desktop and CLI cross this **seam**; neither performs independent file writes or SQL.

Keep filesystem details out of desktop state contracts. Reuse exact-byte hashing, operation payload binding, generation-aware acknowledgements, and store durability conventions. Do not turn standalone draft-save into Markdown publication or insert an extra pass-through crate.

### Persistent records

The logical records are:

- Vault: opaque ID, explicitly selected canonical root and observed root filesystem identity.
- Managed note: opaque ID, vault ID, relative path, observed file-state ID and fingerprint, reconciliation state, and optional current search-snapshot association.
- Editing buffer: note ID, expected starting file-state ID, exact starting bytes, exact working bytes, and monotonically increasing generation.
- Save intent: operation ID and payload digest, note and buffer identities, expected destination state, exact starting/submitted bytes, staging path/identity, phase, observed displaced state, and known outcome.
- Completed receipt: operation identity, bound payload digest, submitted generation, recorded outcome and file-state identity. It remains replay-safe after obsolete recovery bytes are removed.

File-state IDs are opaque application tokens, not paths or content hashes. Fingerprints include exact SHA-256 and filesystem identity; size and modification times are hints, not substitutes for reading bytes. A caller supplies the expected file state and buffer generation. Stale or wrong-note tokens fail without mutation.

### Location and ownership

Keep the database and persistent recovery content in the explicit application data directory, outside the selected vault. Reject a data directory inside the vault. Short-lived staging/displaced artifacts must be on the destination filesystem; use operation-owned non-`.md` names and exclude them from all current-content surfaces. Never delete an unexpected artifact based on its filename alone.

Retain exclusive workspace ownership. Add an application-global per-vault process lock, keyed by the selected root's filesystem identity, so separate BRN data directories cannot independently write the same vault. Also reject overlapping owned roots: shared ancestry locks and an exclusive selected-root lock, acquired in consistent ancestor order, prevent selecting a nested vault to bypass ownership. Root changes require reconciliation; aliases must not defeat ownership.

Resolve note operations relative to validated directory handles. Reject observed symlinks, hard-linked files, escaped paths, non-regular targets, and unsupported volumes. Require advertised atomic exchange and exclusive-create support and successful required durability operations; no ordinary-rename or in-place-write fallback.

Do not infer note identity from a matching hash or filename. External moves, a newly occupied path, or ambiguous replacement require explicit reconciliation/relinking. A missing registered note remains missing; selecting the same path must not silently adopt a different occupant.

## 3. Open, edit, and ordinary recovery

Opening reads bounded exact bytes from the file and records a conservative observation. It grants neither search permission nor AI approval. When a protected buffer exists, show the recovered work and its original baseline alongside the observed disk state; do not silently replace either with the other.

Automatic buffer recovery is scheduled after 500 ms without an edit. Coalesce queued redundant updates without losing the latest generation, a live save's inputs, or unresolved recovery content. The timer is a scheduling target, not a durability acknowledgement or a bound on worker backlog. Show pending/failed recovery honestly; only an acknowledged SQLite commit establishes recoverability.

Flush the latest buffer before switching/closing, or require explicit discard. If the flush fails or remains pending, do not claim the work is protected. Closing does not write Markdown, complete an AI review, or start deletion of unfinished work. Unacknowledged keystrokes can be lost in a process crash.

Only one save may mutate a note's original destination at a time. Typing during a save is allowed. Receipts acknowledge the submitted generation only, preserving later text and its dirty/recovery state. A late acknowledgement from an old note or operation cannot switch the editor or overwrite later edits.

For ordinary completed saves, retain one latest starting/submitted recovery pair and the latest necessary editing buffer. Older completed payloads are removable only after their outcome is known and required recovery content is durably retained. Compact operation receipts remain; repeated operation IDs never cause another filesystem write. Unresolved work is not eligible for rolling cleanup and may exceed the normal bound. This is not a total-vault budget or backup promise.

## 4. Save protocol

Bind each operation ID to the note, destination, expected state, buffer generations, and submitted bytes. Conflicting ID reuse fails; identical reuse reports its existing receipt or unresolved outcome without writing again.

| Step | Action and required durable state |
| --- | --- |
| 1. Prepare intent | Commit the exact expected/submitted recovery content and operation intent. Suspend this note's current-search eligibility before any filesystem mutation. |
| 2. Prepare staging | Create a unique operation-owned staging file on the same filesystem, write exact submitted bytes, preserve supported destination attributes, and flush the file and directory. Persist the prepared file's identity before exchange. |
| 3. Revalidate | Inside macOS file coordination, reread the current destination and verify root/location, regular single-link type, filesystem identity and exact bytes against the expected state. Any observed change, deletion, unsupported state or access error stops the save. |
| 4. Exchange | Atomically exchange staging and the existing destination using the macOS exchange operation and supported no-follow/path-containment protections. A missing destination fails; do not recreate it through a fallback. |
| 5. Verify | Complete required file/directory durability operations. Verify the installed destination against the prepared identity/bytes and the displaced file against the expected starting identity/bytes. Preserve displaced content before retiring its artifact. |
| 6. Commit receipt | Commit the reconciled file-state identity, buffer acknowledgement, permission invalidation when content changed, recovery pair and terminal receipt. Only then acknowledge Saved to Markdown. |

Preserve destination permissions, ACLs and supported extended attributes rather than silently replacing them with staging defaults. Attribute-preservation or durability failure must surface explicitly, with its phase and known outcome. If replacement has happened but bookkeeping/durability cannot be confirmed, report uncertain, not failed-with-no-effect or successful.

An unchanged submission still revalidates the expected file state, then completes as a no-op without changing bytes, file identity, timestamps, or existing exact-version permission.

### External writers and late races

Use `NSFileCoordinator` for reads/writes and a registered `NSFilePresenter` for relevant notifications. Presenter callbacks must not silently publish a recovered buffer, discard dirty work, or run lengthy workflow operations synchronously. Coordination errors never permit an uncoordinated fallback.

`RENAME_SWAP` retains the actual displaced filesystem object and fails when the destination is absent. It does **not** compare content hashes. If a non-participating writer races after the precheck, the proposed file may already occupy the original path when verification discovers a conflict. Preserve the displaced object and all known snapshots, surface the conflict, and block another original-path save. Do not announce normal success or automatically swap back: rollback itself could destroy newer work.

A writer retaining an open descriptor can continue modifying a displaced file, and namespace mutations can occur outside coordination. Consequently, preserved artifacts do not establish lossless arbitrary simultaneous editing. Unexpected types/identities, unstable reads, late changes, or unreadable displaced content remain explicit conflict/uncertain states; never traverse or remove an unexpected displaced directory or link as routine cleanup.

### Restart reconciliation

The existing store marks pending/running operations interrupted at startup. That is not a file-save outcome. Retain the dedicated save-intent phases and reconcile them through the shared workflow before re-enabling eligibility or cleanup.

| Observation against durable intent and artifact identities | Result |
| --- | --- |
| Destination matches the original state, with either no staging created or the recorded prepared artifact still unexchanged | Consistent with not applied; retain the submitted buffer. |
| Destination matches the recorded prepared identity/bytes and displaced artifact matches the original identity/bytes | Applied; finish metadata reconciliation without rewriting the file. |
| Copy intent expects absence, destination matches its recorded prepared identity/bytes, and the staging path was consumed | Copy applied; finish its independent metadata reconciliation without recreating or rewriting either note. |
| Terminal receipt is already present | Return the recorded outcome; separately observe whether the file has since changed. |
| Content matches but artifact/identity proof is missing | Matching content is present; execution provenance remains uncertain. |
| Destination missing/different, artifacts unexpected, or reads/durability uncertain | Conflict/uncertain; retain work and require explicit resolution. |

Reconciliation may commit observations and receipts, but never repeats a filesystem write. Neither an old receipt nor a matching hash implies the recorded content is still current. No cleanup of unresolved operations or unknown artifacts.

## 5. Conflict handling and save-copy

Show the exact starting snapshot, recoverable local edits, and freshly observed disk content or deletion. Comparison is read-only and identifies its observed file state; subsequent changes invalidate that observation.

Reload requires the caller's current buffer/state preconditions and explicit confirmation before discarding local work. Reread the file, establish the new baseline, and only then replace the editor buffer. Dirty/pending work never auto-reloads. A clean editor may refresh an unambiguous content change; ambiguous identity/location changes require reconciliation.

For manual merging, copy/retain local edits, reload the disk version, incorporate the wanted changes, and save against that new baseline. There is no acknowledgement that authorizes blind overwrite of the old baseline.

Save-copy uses a user-chosen different `.md` path within the selected vault, with an existing validated parent. It must not alias the original, reuse a missing registered note's path, or overwrite any occupied destination. Use durable intent plus an exclusive atomic installation, not check-then-ordinary-rename. A confirmed copy receives a new note identity and no automatic search permission.

An uncertain original save can be rescued by a separately journaled copy without resolving or modifying the original operation. Both outcomes remain independently visible. Failed/uncertain copies preserve the input and follow the same no-replay/no-blind-cleanup rules.

## 6. Search and conversation consistency

A current-content identity and permission are separate. A changed save or observed external content change invalidates the previous version-bound permission. While a save/reconciliation is unresolved, eligibility is blocked even if an old index still exists. A verified not-applied save can restore eligibility only when the original approved identity/content remain valid.

One workflow eligibility predicate applies to approval, index construction, keyword/semantic/hybrid results and provider handoff: the file exists at its reconciled location, is supported, has no unresolved save, and its current exact content identity matches the approved snapshot. Read/revalidate bounded file bytes; notifications alone cannot prove correctness. Recheck before publishing an index generation and before returning evidence.

Do not create immutable full-text history on every keystroke or ordinary save. Freeze a search snapshot on an explicit import/approval operation; that snapshot is derived evidence, not the current note authority. A changed live note leaves the old snapshot historical and ineligible until an explicit current snapshot/approval and index rebuild.

### Existing imported copies and other surfaces

Enrollment conservatively identifies imported file-origin records associated with the explicitly selected registered path. Suppress their independent current eligibility; keep original source/version IDs, operation receipts, bytes and citations unchanged. Do not deduplicate or reassign identities by matching text. Ambiguous associations require user reconciliation, not guessed reassignment.

Importing a managed path or changing its search permission must go through the live-note checks; it cannot resurrect a shadowed snapshot. Current document/list/show surfaces must return a freshly validated snapshot or an explicit changed/unavailable state, never old bytes labeled current. The note-state surface can show actual saved bytes and recovered work without granting search permission. Explicit revision/history surfaces retain immutable originals.

Unrelated legacy imports keep their existing snapshot semantics. This slice does not claim to migrate every source in an old workspace to current-only vault knowledge. It does guarantee that an enrolled note and its associated imported copies cannot expose its old version as current evidence.

Recovery buffers, staging/displaced artifacts, prior-save recovery and copied notes are not automatically approved/indexed. Missing approval or indexing lag produces reapproval/reindex-needed or unavailable, never fallback to an older version. Optional native profiles enforce the same predicate and retain their existing unavailable-without-fallback behavior.

### Conversations

Before resuming a provider thread as current, validate all application-supplied prior evidence. If a managed-note version is superseded, missing, excluded or uncertain, reject current-mode resume with stale-context information and require a fresh conversation. Do not silently replay old quotes or answers into the new context. Preserve the old conversation for explicitly labeled historical reading.

Validate selected evidence immediately before submission and again at answer completion. Streaming text is provisional. If a source changes during a turn, preserve the received answer as stale history, not a successful current answer. Keep local outcome/currentness distinct from provider outcome: a completed provider response can have stale evidence, and already-sent content cannot be unsent. Preserve operation replay semantics without resubmitting the turn.

Keep provider execution separate from the vault; do not add a filesystem/archive tool route around eligibility. No live provider call is needed for deterministic qualification.

## 7. Desktop and headless contracts

Add Choose vault, Open Markdown note, the note editor, Save/Cmd-S, conflict comparison/reload, Save a separate copy and recovery inspection/reconciliation. Do not describe database buffer recovery as saving to Markdown.

| Display state | Meaning |
| --- | --- |
| Unsaved | Editor generation is not yet durably recovered or saved to disk. |
| Recoverable in BRN | The latest acknowledged buffer is durable; it may differ from Markdown. |
| Saving | A submitted generation is undergoing the file protocol; later typing remains separate. |
| Saved to Markdown | The submitted file state and terminal receipt were verified at completion. |
| External conflict | Observed external state prevents normal save; local work is retained. |
| Save outcome uncertain | Filesystem effect/bookkeeping cannot be confirmed; do not retry blindly. |

The headless interface exposes note open/show, buffer save, Markdown save, recovery list/show/reconcile, compare, confirmed reload and save-copy. Mutating commands accept operation IDs and explicit caller-supplied file-state/buffer-generation preconditions. Read text through bounded UTF-8 inputs; do not infer the caller's expected state by fetching the latest state inside a mutation.

Use the existing versioned JSON-envelope conventions. Note receipts identify the note, operation, submitted generation, recorded file state, known filesystem outcome and recovery availability. Add typed categories for stale note state, file conflict/missing/unsupported, uncertain save and stale evidence/context; preserve operation-conflict and ordinary I/O distinctions. Errors after a possible write carry recoverable identifiers and the known phase/outcome, not success-shaped defaults. Existing command shapes must not silently change their meaning.

Replay returns the original recorded receipt or unresolved state, not a fresh write or a claim that the old outcome is still the current file. Desktop ignores unrelated/late receipts while preserving their durable outcomes.

## 8. Acceptance and verification

Use synthetic notes in disposable explicit vault/data directories. Never access the original vault or existing trial workspaces. Add deterministic failure injection and subprocess termination points, with normal workflow interfaces as the test surface and a private filesystem seam for controlled races.

| Scenario | Required assertion |
| --- | --- |
| Exact open/save | Unchanged UTF-8, BOM, CRLF, Unicode and frontmatter round-trip byte-for-byte; empty notes work; no mandatory metadata is inserted. |
| Limits and unsupported paths | Exactly 1 MiB is accepted; larger input is rejected with bounded reads. Links, escapes, unsupported volumes and attribute/durability failures surface explicitly without unsafe fallback. |
| Ordinary editing | Save changes the actual file; recovered buffers alone do not. No AI approval is needed. |
| Generations and ownership | Late typing survives save/recovery acknowledgements. Separate BRN workspaces cannot concurrently own the same, aliased, or overlapping vault roots. |
| Observed external change | Edit, deletion, replacement and ambiguous moves block normal save, preserve work and invalidate stale current evidence. |
| Exchange race | Inject edits/deletion/replacement after precheck. Preserve displaced objects and surface conflict/uncertainty; missing destination is not recreated and no blind rollback occurs. |
| Crash boundaries | Terminate before/after intent commit, staging flush/identity record, exchange, directory durability, displaced-content retention, receipt commit and cleanup. Restart reconciles without a filesystem replay. |
| Replay and cleanup | Same-operation retries do not write; conflicting reuse fails. Rolling payload cleanup preserves receipts and unresolved dependencies. |
| Reload and copy | Reload needs current preconditions and explicit discard. Copy refuses occupied/aliased/registered-missing destinations and has its own crash/replay coverage. |
| Search | Old database/index copies fail eligibility across every profile; approval does not transfer to changed bytes. Changes during indexing/search/provider handoff are revalidated. |
| Conversations | Fake-provider tests reject superseded-context resume and mark mid-turn source changes stale while preserving the actual provider outcome. |

The native acceptance demonstration opens disposable `plan.md`, edits and saves, verifies identical bytes from another editor, then changes it externally and observes that BRN blocks silent overwrite. Also exercise deletion, save-copy collision, typing during an in-flight save, quit/restart with acknowledged edits and interrupted-save recovery. Record separately which observations were agent-observed and user-accepted.

Select checks from [verification](../../development/verification.md): targeted store/workflow/CLI tests, storage process-crash checks, integrated workflow checks, optional native-retrieval checks, and native desktop interaction. Do not duplicate overlapping full suites. A headless test or native build is not native usability evidence; optional native resources and actual filesystem/coordination qualification must be reported honestly.

Before implementation, create its plan/evidence under [active work](../../work/active/README.md) using the [development workflow](../../development/workflow.md). Update implemented architecture/crate contracts with the code, not merely because this design exists.

## 9. Approval and technical sources

The user selected explicit Save, a local vault, no force overwrite, and coordinated journaled exchange with its concurrency limitation. The user then approved ownership/scope, save/recovery, current evidence, and interaction/acceptance separately on 1 October 2026. The visual companion was declined; review remains text-only.

The user explicitly approved the written specification committed at `32077d8` and requested implementation planning. This document records intended behavior, not passing checks, native usability, an implemented Markdown editor, or approval of the broader design's remaining history/archive defaults.

Technical grounding:

- [Apple: ensuring safe read/write operations with file coordinators](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileCoordinators/FileCoordinators.html): coordinator/presenter participation and lifecycle.
- Local macOS `man 2 rename`, inspected 1 October 2026: `RENAME_SWAP`, `RENAME_EXCL`, no-follow/containment flags, filesystem capability checks and missing-destination behavior. Atomic exchange is not content-hash compare-and-replace.
- [Workflow implementation](../../../crates/brn-workflow/src/lib.rs): current import, snapshot eligibility/evidence validation and provider-thread resume.
- [Store implementation](../../../crates/brn-store/src/lib.rs): exclusive workspace ownership, durability and interrupted-operation marking.
- [Desktop draft state](../../../crates/brn-desktop/src/drafts.rs): generation-aware submission and acknowledgements that preserve later edits.
