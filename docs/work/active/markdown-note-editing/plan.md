# Safe Markdown Note Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Follow the current user's delegation preference; inline execution is the repository default. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open an existing local Markdown note, protect unfinished edits, save to the real file, and safely reconcile external changes or interrupted saves.

**Architecture:** A deep note-editing module in `brn-workflow` coordinates SQLite registry/recovery records and a private macOS filesystem adapter. Desktop and CLI use the same generation-checked interface. Live files, not imported database snapshots, determine managed-note current evidence.

**Tech Stack:** Rust 1.98.1, existing SQLite/rusqlite, SHA-256, UUID/Serde, macOS Foundation file coordination, atomic exchange/exclusive installation, GPUI-kit native UI, and existing subprocess/fake-provider tests.

**Spec:** [Approved design](../../../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md). Read it together with this plan.

## Global Constraints

- Baseline: `main` at `32077d8`; written specification approved on 1 October 2026. Planning only; implementation/verification have not started.
- Review revision: Opus 5.5/high reviewed `dca2b98`; this update was checked against local `main` at `de0eb45621f9d724cf0fa60be74f36a6ab5c08ae`. Later workspace-shell planning is independent and is not implementation authorization here.
- "Explicit Save/Cmd-S writes Markdown; automatic editing-buffer recovery does not publish."
- "One local vault on this Mac; other local editors may edit its files."
- "Compare, explicit reload/discard, or save a separate copy; no force overwrite or automatic merge."
- "Coordination serializes participating writers, not arbitrary direct writes. A late race can be detected after the submitted file is already installed. No lossless simultaneous-editing claim."
- "One rolling buffer and one most-recent successful-save recovery pair per note. Unresolved work never automatically expires."
- "Empty files are valid; the existing 1 MiB editor limit applies to note and submitted text, measured in bytes."
- "Preserve exact UTF-8 bytes, BOM, line endings, and frontmatter unless deliberately edited."
- "Automatic buffer recovery is scheduled after 500 ms without an edit."
- Required exchange/exclusive-create/durability failures never enable weaker filesystem writes.
- Existing review drafts, original revisions, comments, conversation provenance and operation receipts must survive; no bulk data migration or history purge.
- SQLite owns application metadata and unfinished work, not current saved Markdown; derived indexes cannot grant eligibility.
- Use explicit disposable data/vault directories and synthetic fixtures. Vault ownership uses directory-descriptor locks, not files in a real app-global lock directory or a configurable lock namespace. No original-vault access, live provider calls, model acquisition, merge or release.
- Respect the pinned toolchain and lockfiles. Install/restore dependencies only after manifest changes or an actual missing-dependency failure.
- Keep interface changes additive where possible; existing CLI envelopes stay schema version 1 and existing command semantics remain documented.
- Execute in isolation using `using-git-worktrees` at execution time; preserve unrelated changes. Do not create a worktree merely to write this plan.
- Every implementation commit includes `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.
- Before each task commit, run its targeted checks plus `cargo check --workspace --locked`; update exhaustive matches/constructors in the same task that changes their public types. Do not leave a deliberately broken intermediate commit.

## File ownership and task dependencies

| Files | Responsibility | Task |
| --- | --- | --- |
| `crates/brn-store/src/notes.rs`, `src/lib.rs`, `tests/notes.rs`, `tests/storage.rs` | Typed registry/buffers/intents/receipts, schema migration and recovery retention | 1 |
| `crates/brn-workflow/src/notes/files.rs`, `notes/macos.rs`, `Cargo.toml`, root `Cargo.lock` | Bounded reads, supported filesystem operations, Foundation coordination/presentation, vault ownership | 2 |
| `crates/brn-workflow/src/notes/mod.rs`, `src/lib.rs`, `src/error.rs`, `tests/notes.rs` | Open/state/buffer workflow and error contracts | 3 |
| `crates/brn-workflow/src/notes/save.rs`, `notes/crash_tests.rs`, `tests/note_recovery.rs` | Original-path exchange, receipts, lib-unit process-crash tests, public replay/restart tests and cleanup | 4 |
| `crates/brn-workflow/src/notes/conflicts.rs`, `tests/note_conflicts.rs` | Comparison, confirmed reload/relink, independent save-copy | 5 |
| `crates/brn-workflow/src/notes/eligibility.rs`, `src/lib.rs`, `src/main.rs`, `src/worker.rs`, `brn-store/src/workflow.rs`, `brn/src/cli/error.rs`, `cli/documents.rs`, `brn-desktop/src/native.rs`, `tests/flow.rs`, `tests/note_evidence.rs` | Current snapshots, source-state projections/display, provider context/completion guards and immediate consumer compatibility | 6 |
| `crates/brn/src/cli/notes.rs`, `cli/mod.rs`, `cli/error.rs`, `cli/ask.rs`, `cli/documents.rs`, `tests/cli_notes.rs`, `tests/cli_ask.rs`, `README.md` | Agent CLI parity, current document surfaces, contextual errors, help and exact-byte inputs | 7 |
| `crates/brn-workflow/src/worker.rs`, `brn-desktop/src/notes.rs`, `src/native.rs`, `src/main.rs`, `README.md` | Note worker actions, native state/editor/conflicts and recovery scheduling | 8 |
| Architecture/crate documentation, task evidence, status/work indexes | Integrated and native qualification, reproducible handoff | 9 |

Task 2 consumes Task 1's shared types, although its filesystem checks do not open SQLite. Task 3 consumes both; Task 4 consumes Task 3; Task 5 consumes Task 4; Task 6 consumes Tasks 3-5. Tasks 7 and 8 consume Tasks 3-6 and can be reviewed independently. Task 9 consumes all implementation tasks. This is one cross-surface safety feature, not separately authorized archive/history/AI-review projects.

## Shared planned interface

Define these types in `brn-store::notes` and re-export the public editing types through `brn-workflow::notes`. Keep raw fingerprints/artifact paths out of native view state. All strings below represent exact validated UTF-8; use checked generation conversion into SQLite's signed integer range.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteStamp {
    pub file_state: Uuid,
    pub generation: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteSubmission {
    pub operation_id: Uuid,
    pub note_id: Uuid,
    pub expected: NoteStamp,
    pub generation: u64,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteBufferReceipt {
    pub operation_id: Uuid,
    pub note_id: Uuid,
    pub stamp: NoteStamp,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteAvailability {
    Available, Missing, Conflict, Unsupported, Uncertain, Unavailable, OwnedElsewhere,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteView {
    pub id: Uuid,
    pub vault_id: Uuid,
    pub relative_path: PathBuf,
    pub stamp: NoteStamp,
    pub current_file_state: Option<Uuid>,
    pub saved: Option<String>,
    pub buffer: String,
    pub availability: NoteAvailability,
    pub availability_message: Option<String>,
    pub search_approval: Approval,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavePhase { Intent, Prepared, Exchanged, Verified, Complete }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactCleanup { Pending, Retired, RetainedUnexpected }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOutcome { NotApplied, Applied, Unknown }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteResolution { Unresolved, NotApplied, Applied, AcceptedCurrent }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteReceipt {
    pub operation_id: Uuid,
    pub source_note_id: Uuid,
    pub note_id: Uuid,
    pub submitted_generation: u64,
    pub stamp: NoteStamp,
    pub filesystem_outcome: FileOutcome,
    pub recovery_available: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteFailure {
    pub code: NoteErrorCode,
    pub message: String,
    pub operation_id: Option<Uuid>,
    pub note_id: Option<Uuid>,
    pub phase: Option<SavePhase>,
    pub filesystem_outcome: FileOutcome,
    pub recovery_available: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteRecordedResult { Receipt(NoteReceipt), Failure(NoteFailure) }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteErrorCode {
    StateChanged, Conflict, Missing, Unsupported, SaveUncertain, Io,
    OperationConflict, WorkspaceBusy, VaultBusy, VaultUnavailable, Storage,
}
pub type NoteResult<T> = std::result::Result<T, NoteFailure>;
```

A successful write returns an Applied receipt only after the file and receipt are verified. A verified unchanged save returns NotApplied as an explicitly documented no-op. Conflict/uncertainty is `Err(NoteFailure)`, never an `Ok` receipt that resembles successful saving. Reconciliation can successfully report NotApplied without writing. `NoteView.saved` is a fresh disk observation; stored recovery bytes must never masquerade as that field.

`NoteView.stamp` is the editing buffer's expected baseline/generation. While dirty/conflicted, it does not silently switch to a newly observed external file state. Reload/relink establishes a new baseline explicitly; a successful own save rebases the retained later edits without changing their text.

`current_file_state` identifies a fresh saved-content observation, separately from the editing baseline; it is not proof of an earlier save's execution. Explicit search approval uses this token only after location/outcome reconciliation and never approves `buffer`. Unavailable/owned-elsewhere views have no fresh saved bytes and include an explanatory message. `recovery_available: true` confirms the submitted snapshot is retained; false means that confirmation is absent, not proof that no older work exists. Copy receipts identify both the original source and newly created destination note.

Additional store-owned types: `FileFingerprint { device: u64, inode: u64, len: u64, sha256: [u8; 32] }`; `VaultIdentity { device: u64, inode: u64 }`; `VaultRecord { id: Uuid, root: PathBuf, identity: VaultIdentity }`; `NoteRecovery { note_id: Uuid, stamp: NoteStamp, baseline: String, working: String, pending_operations: Vec<Uuid> }`; `NoteComparison { note_id: Uuid, stamp: NoteStamp, baseline: String, working: String, observed: Option<String>, observed_file_state: Option<Uuid>, availability: NoteAvailability }`; `PreparedFile { relative: PathBuf, fingerprint: FileFingerprint }`; `ArtifactIdentity { device: u64, inode: u64, len: u64, kind: ArtifactKind }`, with `ArtifactKind { Regular, Directory, Symlink, Other }`; `RetainedArtifact { relative: PathBuf, identity: ArtifactIdentity, sha256: Option<[u8; 32]> }`.

`DestinationPrecondition` is an explicit sum type: `Existing { fingerprint: FileFingerprint, baseline_text: String }` for original-path writes, or `Absent { parent: VaultIdentity }` for exclusive copies. `NoteWriteKind { Replace, Copy }` remains persisted explicitly; do not infer it from a nullable text field. `NoteSaveIntent { request: NoteSubmission, target_note_id: Uuid, destination: PathBuf, staging_relative: PathBuf, kind: NoteWriteKind, expected_destination: DestinationPrecondition, staged: Option<PreparedFile>, displaced: Option<RetainedArtifact>, phase: SavePhase, resolution: NoteResolution, acknowledged_by: Option<Uuid>, cleanup: ArtifactCleanup, prior_result: Option<NoteRecordedResult> }`. Allocate and reserve the copy's new `target_note_id` and destination before any filesystem mutation.

Persist the planned non-`.md` staging path in the intent before exclusive creation. `staged` later records its verified prepared identity; a crash between creation/flush and that record leaves a discoverable but unproven artifact, not permission to delete a filename match.

Resolution SQL spellings are checked `unresolved`, `not_applied`, `applied`, `accepted_current`. This field governs the original-write block, not generic operation status or cleanup eligibility. AcceptedCurrent references a separate acknowledgement and keeps the original uncertain/conflict result and recovery protected. Persist tagged success/failure results with payload/result hashes; replay must not lose a failure's phase/outcome or turn acknowledgement into a successful original receipt.

Successful verification is `NoteVerification::Replace { installed: FileFingerprint, displaced: RetainedArtifact, displaced_bytes: Vec<u8> }` or `NoteVerification::Copy { installed: FileFingerprint }`. Only the verified valid starting note fits the normal UTF-8/1 MiB snapshot path. Unexpected displaced content remains an artifact even when non-UTF-8, oversized, unreadable or non-regular; never lose it by trying to convert it into `String`. Derive equality and Serde where persisted. Do not expose raw artifact metadata as current content or provider evidence.

### Generation, ownership and replay rules

| Operation | Exact precondition and effect |
| --- | --- |
| Buffer save | `expected` equals the stored buffer stamp. A higher submitted generation may replace/coalesce working text; an equal generation requires identical text. Lower generations and equal-generation different bytes fail with StateChanged. |
| Markdown save/copy | Accept the same rules, so latest unsaved text can be captured in the intent transaction. Saving an already recovered buffer uses equal expected/submitted generations and identical text; no artificial edit/generation increment. |
| Save completion | Acknowledge only the submitted generation. If a newer buffer exists, retain its text/generation and rebase it onto the verified saved state; never lower the stored generation. |
| Replay | Check operation kind/payload and recorded receipt/failure before fresh-state checks. Replay never captures another buffer, reacquires authority to write, or performs a filesystem mutation. |

Within the intent transaction, reject a second original-path intent while an unresolved original-path intent exists for the note. Use an explicit unresolved-resolution field and a partial uniqueness constraint, not generic `operations.status`: startup marks those operations Interrupted. Independently reserved save-copy operations remain allowed and do not authorize or resolve the original write. Copies suspend eligibility only for their destination, not an otherwise eligible unchanged original.

### Vault lifecycle across processes

One workspace is bound to one registered vault for this slice. Reopening the same root through an alias must validate the stored filesystem identity. Selecting a different root fails explicitly and directs the caller to a separate data directory; do not silently switch or merge registries.

`Workspace::open` opens/recover-checks SQLite without requiring the vault. ID-only note operations load `VaultRecord` from the registry, lazily open and validate the root, and acquire the same directory-descriptor ownership used by `open_note`. Hold ownership for the Workspace/worker lifetime. Missing/replaced roots and another live owner produce VaultUnavailable/VaultBusy on filesystem-dependent mutations. Recovery list/show and historical records remain readable without the root. A fresh-process `notes show` can report unavailable state but never substitute a stored baseline for current saved content.

For search/indexing, report unavailable/owned-elsewhere notes as explicitly excluded; validate the remaining sources. Preserve existing whole-corpus `INDEX_STALE` behavior if eligibility changes invalidate an index, with the exclusion reason surfaced. Do not invent a promise that an old mixed index is always usable. An unavailable vault must not itself prevent database opening, history inspection, or searches whose valid corpus never included that note.

## Task 1: Durable note registry, buffers, intents and receipts

**Files:** Create `crates/brn-store/src/notes.rs`, `crates/brn-store/tests/notes.rs`; modify `crates/brn-store/src/lib.rs` (module exports, V6 initialization/migration, `validate_schema`, local operation kinds), `tests/storage.rs`, and `src/workflow.rs` (additive turn-currentness projection).

**Consumes:** Existing `Store`, SQLite transaction helpers, `operations` payload binding, exact-byte hashes and V1-V5 schema validation.

**Produces:** Shared types above; store methods `enroll_note(op: Uuid, vault: &VaultRecord, relative: &Path, file: FileFingerprint, text: &str) -> NoteResult<NoteRecovery>`, `save_note_buffer(request: &NoteSubmission) -> NoteResult<NoteBufferReceipt>`, `note_recovery(id: Uuid) -> NoteResult<Option<NoteRecovery>>`, `note_vault(id: Uuid) -> NoteResult<VaultRecord>`. Note-specific operations return typed `NoteFailure`, rather than forcing generation/conflict errors into an untyped Invalid message. Existing store/schema interfaces retain their current Result/error contract.

Store save interfaces are `begin_note_save(&NoteSubmission, &Path, NoteWriteKind, &DestinationPrecondition) -> NoteResult<NoteSaveIntent>`, `record_note_prepared(Uuid, &PreparedFile) -> NoteResult<()>`, `mark_note_exchanged(Uuid) -> NoteResult<()>`, `record_note_displaced(Uuid, &RetainedArtifact) -> NoteResult<()>`, `record_note_verification(Uuid, &NoteVerification) -> NoteResult<()>`, `finish_note_save(Uuid, &NoteReceipt) -> NoteResult<()>`, `note_save_intent(Uuid) -> NoteResult<Option<NoteSaveIntent>>`, `note_save_intents() -> NoteResult<Vec<NoteSaveIntent>>`, and `prune_completed_note_payloads(Uuid) -> NoteResult<()>`. Terminal commit atomically reconciles registry/permission/buffer/receipt/resolution state. A copy's parent identity, absent precondition and preallocated identity are durable before installation; the store does not guess directory identities.

Add `note_write_result(&NoteSubmission, &Path, NoteWriteKind) -> NoteResult<Option<NoteRecordedResult>>` to check bound payload/result before root access, even for a refusal recorded without an intent. Add `record_note_write_failure(&NoteSubmission, &Path, NoteWriteKind, &NoteFailure) -> NoteResult<NoteFailure>` to bind a valid attempt, protect its submitted input and record its failure atomically. The internal returned failure is a persistence acknowledgement; the workflow still returns Err. Invalid caller stamps never overwrite valid working text. If recording fails, return Storage with the actual phase/outcome and no invented recovery acknowledgement.

Add store-owned `NoteReconciliation { resolution: NoteResolution, observed_destination: Option<FileFingerprint>, verification: Option<NoteVerification>, result: NoteRecordedResult }` and `reconcile_note_operation(op: Uuid, record: &NoteReconciliation) -> NoteResult<NoteRecordedResult>`. This dedicated transaction checks the bound intent, required proof and existing receipt, and can resolve Pending/Running/Interrupted note-save/copy operations. Applied requires write-kind-specific verification; NotApplied requires the corresponding original/absence proof. AcceptedCurrent is reserved for Task 5's separate acknowledgement, never inferred here. Never relax generic `Store::transition`. Generic `finish_operation` must reject reserved note kinds; unknown execution remains unresolved even if a failure receipt exists. A generic Interrupted operation still cannot become Completed.

- [ ] **1. Write failing exact-buffer/replay tests**, including this test body and a changed-payload variant:

```rust
let data = tempfile::tempdir().unwrap();
let vault = tempfile::tempdir().unwrap();
let (mut store, _) = Store::open(data.path()).unwrap();
let text = "\u{feff}# plan\r\n";
let fingerprint = FileFingerprint {
    device: 1, inode: 3, len: text.len() as u64,
    sha256: sha2::Sha256::digest(text.as_bytes()).into(),
};
let opened = store.enroll_note(
    Uuid::new_v4(), &VaultRecord {
        id: Uuid::new_v4(), root: vault.path().to_owned(),
        identity: VaultIdentity { device: 1, inode: 2 },
    },
    Path::new("plan.md"), fingerprint, text,
).unwrap();
let request = NoteSubmission {
    operation_id: Uuid::new_v4(), note_id: opened.note_id,
    expected: opened.stamp, generation: 1, text: "updated\r\n".into(),
};
let first = store.save_note_buffer(&request).unwrap();
assert_eq!(store.save_note_buffer(&request).unwrap(), first);
drop(store);
let (store, _) = Store::open(data.path()).unwrap();
assert_eq!(store.note_recovery(opened.note_id).unwrap().unwrap().working, "updated\r\n");
```

- [ ] **2. Run red:** `cargo test -p brn-store --test notes --locked`; expected missing note interfaces. Do not count an unrelated failure as red.
- [ ] **3. Implement transactional records.** V6 creates registry, one per-note editing buffer, save intents, last successful recovery pair, shadowed-source associations and compact receipt tables. Bind operation payloads before mutation; reject wrong note/state, invalid UTF-8, oversized bytes, generation overflow, conflicting reuse and stale updates. Store enrollment baseline/working bytes as recovery, not current authority.

```sql
CREATE TABLE note_receipts (
  operation_id TEXT PRIMARY KEY REFERENCES operations(id),
  note_id TEXT NOT NULL,
  result_json BLOB NOT NULL,
  result_sha256 BLOB NOT NULL CHECK(length(result_sha256)=32)
);
```

Include an unresolved-original-write guard in the actual intent DDL:

```sql
CREATE UNIQUE INDEX one_unresolved_original_save
ON note_save_intents(source_note_id)
WHERE write_kind = 'replace' AND resolution = 'unresolved';
```

Every newly created table/index and the additive `chat_turns.evidence_currentness` column must enter the expected-schema builder. Its checked values are `unqualified`, `current_at_completion`, `stale_at_completion`, with existing rows defaulting to `unqualified`. Update every old migration path to include V6 once, and add V5-to-V6 transactional migration; generic startup interruption must not erase dedicated save phases, resolution, destination reservations or cleanup state.

Define `EvidenceCurrentness { Unqualified, CurrentAtCompletion, StaleAtCompletion }` with checked SQL spelling conversion and a default of Unqualified; add it to `ChatTurn` with an additive Serde default. Produce `Store::complete_turn_with_currentness(op: Uuid, status: OperationStatus, text: &str, usage: Option<&str>, currentness: EvidenceCurrentness) -> Result<()>`, committing status/text/currentness atomically. Keep existing `complete_turn` as the existing-semantics wrapper.

- [ ] **4. Add green cases** for V1-V5 upgrade, killed uncommitted migration, corrupt receipt/hash/schema, wrong/equal generations, coalescing buffers, safe completed-payload retirement and protected unresolved payloads. Specifically test that startup-interrupted note saves can reconcile, generic interrupted operations cannot complete, a second unresolved original save is rejected atomically, and an independently reserved copy remains allowed. Reuse the existing subprocess `child_worker`/acknowledgement pattern.
- [ ] **5. Run:** `cargo test -p brn-store --test notes --test storage --test workflow --locked`; expected all selected tests pass. Commit these files with `feat(store): persist managed-note recovery and save intents` and the required trailer.

## Task 2: macOS filesystem and vault-ownership adapter

**Files:** Create `crates/brn-workflow/src/notes/mod.rs` with private adapter module declarations, `notes/files.rs`, `notes/macos.rs`; modify `src/lib.rs` to declare the notes module, `crates/brn-workflow/Cargo.toml` and root `Cargo.lock`. Unit tests live beside the private adapter.

**Consumes:** `FileFingerprint`, `VaultIdentity`, exact UTF-8/1 MiB requirements and the approved concurrency limitation.

**Produces:** Private `MacFiles::open(vault: &VaultRecord, data_dir: &Path, notices: NoteNoticeSink) -> NoteResult<MacFiles>`, `observe(relative: &Path) -> NoteResult<FileObservation>` with `FileObservation { fingerprint: FileFingerprint, text: String }`, coordinated-operation closures, stage/flush/exchange/exclusive-install primitives and notifications. `NoteNoticeSink` is an `Arc<Mutex<NoteNoticeQueue>>`, a coalesced bounded queue defined here; overflow records a RescanRequired notice rather than silently dropping the need to observe. Use the store-owned `PreparedFile` to identify owned staging; do not expose this to views.

Private filesystem methods are `coordinate<T>(&self, relative: &Path, action: impl FnOnce() -> NoteResult<T>) -> NoteResult<T>`, `prepare_replace(op: Uuid, staging: &Path, destination: &Path, bytes: &[u8]) -> NoteResult<PreparedFile>`, `exchange(&PreparedFile, &Path) -> NoteResult<()>`, and `flush_artifact(&Path) -> NoteResult<()>`. Preparation uses the intent's planned staging path, exclusively creates it in the validated destination parent and verifies returned prepared metadata against the intent. Task 5 adds copy-specific counterparts. The coordination accessor runs synchronously while its scope/ownership is held; presenter notifications only enqueue observation hints.

- [ ] **1. Write failing unit tests** for exact bounded observation and ownership:

```rust
let vault = tempfile::tempdir().unwrap();
let data = tempfile::tempdir().unwrap();
std::fs::write(vault.path().join("plan.md"), b"\xef\xbb\xbf# plan\r\n").unwrap();
let root_meta = std::fs::metadata(vault.path()).unwrap();
let registered = VaultRecord {
    id: Uuid::new_v4(), root: vault.path().to_owned(),
    identity: VaultIdentity { device: root_meta.dev(), inode: root_meta.ino() },
};
let notices = Arc::new(Mutex::new(NoteNoticeQueue::default()));
let files = MacFiles::open(&registered, data.path(), notices.clone()).unwrap();
assert_eq!(files.observe(Path::new("plan.md")).unwrap().text.as_bytes(),
           b"\xef\xbb\xbf# plan\r\n");
assert_eq!(MacFiles::open(&registered, data.path(), notices).err().unwrap().code,
           NoteErrorCode::VaultBusy);
```

- [ ] **2. Run red:** `cargo test -p brn-workflow --lib notes::files --locked`; expected missing adapter. Import Unix `MetadataExt` and standard Arc/Mutex in the unit test; fixtures are disposable vault/data directories.
- [ ] **3. Implement the adapter.** Use directory-relative operations, no-follow containment, a `limit + 1` bounded read, UTF-8 validation, SHA-256 and descriptor metadata. Validate single-link regular files before acting. Apply macOS advisory `flock` directly to validated read-only directory descriptors: shared ancestors, exclusive selected root, root-to-leaf order and nonblocking contention reporting. Kernel object identity makes this process-global across data directories and aliases without creating app-global lock files or exposing a test bypass. Use close-on-exec descriptors and retain them for the adapter lifetime. Reject unsupported directory locking, data directories within the vault and changed roots; qualify same-root/nested-root exclusion with subprocesses before later tasks depend on it.

```rust
let mut bytes = Vec::new();
std::io::Read::by_ref(&mut file)
    .take((MAX_IMPORT_BYTES + 1) as u64)
    .read_to_end(&mut bytes)
    .map_err(note_io_failure)?;
if bytes.len() > MAX_IMPORT_BYTES {
    return Err(note_unsupported("note exceeds 1 MiB"));
}
let text = String::from_utf8(bytes).map_err(note_utf8_failure)?;
```

Define `note_io_failure(std::io::Error) -> NoteFailure`, `note_unsupported(&str) -> NoteFailure`, and `note_utf8_failure(std::string::FromUtf8Error) -> NoteFailure` in this module; use context enrichment at the workflow call site. Verify volume capabilities via `getattrlist`, use `renameatx_np` exchange/exclusive flags rather than ordinary rename, and preserve mode/ACL/xattrs before committing replacement. Flush prepared files and containing directories; propagate every failure.

Use target-macOS dependencies matching inspected lockfile releases: `libc = "=0.2.189"`, `objc2 = "=0.6.4"`, `objc2-foundation = "=0.3.2"`, `block2 = "=0.6.2"`. Foundation feature names include `NSFileCoordinator`, `NSFilePresenter`, `NSOperation` (which supplies `NSOperationQueue`), `NSURL`, `NSString`, `NSError` and `block2`; verify concrete accessor/protocol signatures at the Task 2 compile gate. After manifest edits, refresh the lockfile through Cargo before the locked checks; version presence alone is not API qualification.

Keep Foundation ownership/registration in `macos.rs`, register/unregister presenters deterministically, and pass the app's presenter into its coordinator. Presenter callbacks run on a dedicated serial operation queue, not the GUI thread or owned SQLite worker, and enqueue only `NoteFileNotice { vault_id: Uuid, relative_path: Option<PathBuf>, kind: NoteNoticeKind }`, with `NoteNoticeKind { Changed, Moved, Deleted, RescanRequired }`. They never perform a save or wait for the worker. Non-macOS note writes report unsupported; existing commands retain their supported Unix behavior.

Preserve mode/ACL/xattrs, not the original modification time or a broad `COPYFILE_STAT` bundle. Changed saves must advance observable modification metadata; unchanged saves preserve it. Qualify file `F_FULLFSYNC` and directory synchronization on the supported filesystem and propagate unsupported/failing required operations. Do not assume `F_FULLFSYNC` is accepted on directory descriptors or claim untested power-failure durability from process-kill tests.

- [ ] **4. Exercise physical primitives** against empty/exact-limit/over-limit/invalid-UTF-8 inputs, links, escaping paths, nested/aliased ownership roots, external atomic replacement, missing exchange targets, exclusive-create collisions, coordination errors and attribute/durability failures. Verify the displaced inode/bytes survive exchange. Test observer lifecycle and that callback delivery does not synchronously perform workflow writes.
- [ ] **5. Run:** `cargo test -p brn-workflow --lib notes:: --locked`. Commit as `feat(workflow): coordinate supported local note files`.

## Task 3: Shared open/state/buffer workflow

**Files:** Create `crates/brn-workflow/tests/notes.rs`; modify `src/notes/mod.rs`, `src/lib.rs`, `src/error.rs`.

**Consumes:** Task 1 store methods and Task 2 adapter. Integration tests and CLI subprocesses use the production directory-descriptor locking path on disposable vaults; no inaccessible private constructor or global lock-directory override.

**Produces:** `Workspace::open_note(op: Uuid, vault: &Path, relative: &Path) -> NoteResult<NoteView>`, `note(id: Uuid) -> NoteResult<NoteView>`, `save_note_buffer(request: NoteSubmission) -> NoteResult<NoteBufferReceipt>`, `note_recoveries() -> NoteResult<Vec<NoteRecovery>>`. Keep the selected adapter/root lifetime owned by `Workspace`; no second SQLite connection for autosave.

Define private `acquire_note_vault(note_id: Uuid) -> NoteResult<()>` from `Store::note_vault` and use it for every ID-only filesystem-dependent operation, including reconciliation, approval and evidence validation. Root identity is checked before using stored paths. Maintain the shared notice queue in Workspace; Task 8 exposes its lightweight draining route through the worker. Buffer mutations and metadata-only recovery inspection do not require a present/owned vault.

- [ ] **1. Write failing provider-free workflow tests**, using this fresh setup in each isolated test:

```rust
let data = tempfile::tempdir().unwrap();
let vault = tempfile::tempdir().unwrap();
std::fs::write(vault.path().join("plan.md"), b"# base\r\n").unwrap();
let mut w = Workspace::open(data.path(), Config::default()).unwrap();
let opened = w.open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md")).unwrap();
let recovered = w.save_note_buffer(NoteSubmission {
    operation_id: Uuid::new_v4(), note_id: opened.id,
    expected: opened.stamp, generation: 1, text: "# draft\r\n".into(),
}).unwrap();
assert_eq!(recovered.stamp.generation, 1);
assert_eq!(w.note(opened.id).unwrap().buffer, "# draft\r\n");
assert_eq!(std::fs::read(vault.path().join("plan.md")).unwrap(), b"# base\r\n");
```

- [ ] **2. Run red:** `cargo test -p brn-workflow --test notes --locked`; expected missing workflow methods.
- [ ] **3. Implement enrollment/state observation**, requiring an existing regular UTF-8 `.md` note, loading protected buffers without replacing their baseline, updating observation IDs conservatively and preserving opaque note identity. Read actual disk bytes for `saved`; represent missing/unsupported/ambiguous states explicitly. Add typed note-code conversion, retaining `NoteFailure` context rather than flattening it into English matching.

```rust
pub fn save_note_buffer(&mut self, request: NoteSubmission) -> NoteResult<NoteBufferReceipt> {
    self.store.save_note_buffer(&request)
}
```

The store checks operation replay before validating the stored editing baseline/generation; it does not require conflicted disk content to match that baseline. Recovery must still work after external changes. Map note SQL failures to Storage, retaining IDs/phase and known filesystem outcome; do not classify generation/operation conflicts by matching Invalid-message wording. Keep the new note-specific failure contract separate from existing generic store errors.

- [ ] **4. Verify reopening acknowledged buffers**, empty notes, explicit unsupported/unavailable/owned-elsewhere states, changed disk observations, stale buffer mutations, duplicate operations and recovery during external deletion. Drop/reopen Workspace and call `note(id)` without another `open_note` to prove lazy root acquisition. Test missing/replaced roots, a second BRN owner, read-only recovery/history access and rejection of selecting a different vault. Operation replay must precede current-state/root checks.
- [ ] **5. Run:** `cargo test -p brn-store -p brn-workflow --test notes --locked`; commit as `feat(workflow): open notes with generation-checked recovery buffers`.

## Task 4: Original-path save and crash reconciliation

**Files:** Create `crates/brn-workflow/src/notes/save.rs`, `notes/crash_tests.rs`, `tests/note_recovery.rs`; modify `notes/mod.rs`, store intent/receipt methods in `brn-store/src/notes.rs`.

**Consumes:** `NoteSubmission`, expected fingerprints/baseline recovery and Task 2 coordinated filesystem primitives.

**Produces:** `Workspace::save_note(request: NoteSubmission) -> NoteResult<NoteReceipt>`, `reconcile_note_save(op: Uuid) -> NoteResult<NoteReceipt>`. Private save phases expose `cfg(test)` hooks to lib unit tests; they are not assumed accessible from integration tests and are never production environment-controlled crash switches.

The private helpers use `NoteSaveObservations { destination: Option<FileObservation>, staging: Option<FileObservation>, retained: Option<RetainedArtifact> }` and `RecoveryDecision { NotApplied, Applied, MatchingContentUnproven, Conflict, Uncertain }`, defined in `save.rs`. Missing paths alone become `None`; unreadable/unexpected objects produce explicit failures with preserved artifacts. MatchingContentUnproven produces SaveUncertain with FileOutcome::Unknown, not successful application. Signatures: `load_note_intent(Uuid) -> NoteResult<NoteSaveIntent>`, `observe_note_intent(&NoteSaveIntent) -> NoteResult<NoteSaveObservations>`, `classify_note_save(&NoteSaveIntent, &NoteSaveObservations) -> NoteResult<RecoveryDecision>`, and `commit_note_reconciliation(NoteSaveIntent, RecoveryDecision) -> NoteResult<NoteReceipt>`.

- [ ] **1. Write failing same-file save and stale-baseline tests**:

```rust
let data = tempfile::tempdir().unwrap();
let vault = tempfile::tempdir().unwrap();
let path = vault.path().join("plan.md");
std::fs::write(&path, b"base").unwrap();
let mut w = Workspace::open(data.path(), Config::default()).unwrap();
let opened = w.open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md")).unwrap();
let request = NoteSubmission {
    operation_id: Uuid::new_v4(), note_id: opened.id,
    expected: opened.stamp, generation: 1, text: "mine".into(),
};
std::fs::write(&path, b"external").unwrap();
let failure = w.save_note(request).unwrap_err();
assert_eq!(failure.code, NoteErrorCode::Conflict);
assert_eq!(std::fs::read(&path).unwrap(), b"external");
```

- [ ] **2. Run red:** `cargo test -p brn-workflow --test note_recovery --locked`; expected missing save method.
- [ ] **3. Implement the spec's six ordered phases**, with durable intents/temporary eligibility suspension before staging, recorded prepared identity before exchange, actual displaced-state verification, recovery retention and terminal receipt after verification. Capture the submission as recovery even when a detected external conflict prevents writing. Identical replay is handled before stale-state checks.

Use `note_write_result` before root acquisition/fresh-state checks, and the shared equal/higher-generation rules; test saving a recovered buffer without editing it again. Valid external/root/conflict refusals use `record_note_write_failure` so replay and submitted recovery remain inspectable even without a staging intent. Enforce the unresolved-original gate inside `begin_note_save`, not just in UI. After exchange returns, record Exchanged and displaced identity; a crash before that journal update is still classified from the prepared/destination pair. Uncertain exchange errors and mismatched displaced objects never become known-not-applied merely because the phase still says Prepared.

```rust
pub fn reconcile_note_save(&mut self, op: Uuid) -> NoteResult<NoteReceipt> {
    let intent = self.load_note_intent(op)?;
    if let Some(result) = intent.prior_result.clone() {
        return match result {
            NoteRecordedResult::Receipt(receipt) => Ok(receipt),
            NoteRecordedResult::Failure(failure) => Err(failure),
        };
    }
    self.acquire_note_vault(intent.request.note_id)?;
    let observations = self.observe_note_intent(&intent)?;
    let decision = classify_note_save(&intent, &observations)?;
    self.commit_note_reconciliation(intent, decision)
}
```

Define these private operations in `save.rs`; `classify_note_save` implements every restart-table row. Observation is read-only; `commit_note_reconciliation` uses the dedicated note-specific Interrupted transition and performs only store bookkeeping. No reconciliation branch invokes exchange, ordinary rename, creation or unlink. Unknown/mismatched artifacts remain retained with phase/outcome context.

Keep cleanup distinct: `cleanup_note_artifacts(op: Uuid) -> NoteResult<()>` verifies the terminal outcome, durably retained recovery bytes, exact artifact ownership/identity and references before unlinking. Journal Pending before deletion; sync the parent and mark Retired afterward. A crash after unlink but before Retired is handled by observing absence and committing metadata, not recreating/deleting anything during save reconciliation. Unexpected occupants become RetainedUnexpected and remain untouched. Known terminal NotApplied attempts can coalesce obsolete payloads only after the latest working/baseline recovery is confirmed and no protected dependency references them. Unresolved/uncertain artifacts and conflict inputs remain protected.

- [ ] **4. Add lib-unit process-crash coverage** in `notes/crash_tests.rs`, declared under `cfg(test)` from `notes/mod.rs`. Relaunch the lib test executable with `--exact notes::crash_tests::child_worker --nocapture`; only that child reads its fixture/hook environment, installs the private test hook and prints/flushed `ACK:<phase>`. Parent validates the exact acknowledgement, kills that specific Child, waits/reaps, reopens and classifies. Keep public-API replay/reopen tests in `tests/note_recovery.rs`; they do not call private hooks.

| Durable checkpoint acknowledged before kill | Required restart assertion |
| --- | --- |
| Before intent commit | No filesystem mutation; prior acknowledged recovery remains. |
| Intent committed, no stage | Original unchanged; NotApplied without replay. |
| Stage synced, prepared identity not committed | Unknown artifact/proof is retained; never assume it is safe to delete or apply. |
| Prepared identity committed | Unexchanged pair yields NotApplied. |
| Exchange returned, Exchanged not committed | Verified exchanged identities yield Applied even though the journal still says Prepared. |
| Exchange/directory sync completed | Applied only with exact displaced/installed proof; otherwise Conflict/Uncertain. |
| Displaced recovery retained, receipt absent | Complete bookkeeping without rewriting either file. |
| Receipt committed | Original receipt replays without filesystem effects. |
| Cleanup Pending, unlink absent/present | Preserve an unexpected object; retire observed absence metadata-only. |
| Obsolete payload retirement | Current recovery, unresolved dependencies and replay receipts survive. |

Add post-precheck content/deletion/replacement hooks, non-UTF-8/oversized displaced-object cases and equal-content-with-missing-proof cases. Never rollback unexpected displaced objects.
- [ ] **5. Verify no-op timestamp/identity/permission preservation**, equal-generation recovered saves, later typing, blocking a second unresolved original intent, same-ID replay after later edits, known-not-applied coalescing and protected uncertain payloads. Run `cargo test -p brn-workflow --lib notes::crash_tests --locked` and `cargo test -p brn-workflow --test note_recovery --locked`; commit as `feat(workflow): journal and reconcile Markdown saves`.

## Task 5: Conflict comparison, confirmed reload/relink and exclusive save-copy

**Files:** Create `crates/brn-workflow/src/notes/conflicts.rs`, `tests/note_conflicts.rs`; modify `notes/mod.rs`, `notes/save.rs`, and store note transactions.

**Consumes:** Tasks 3-4, `NoteComparison`, `NoteRecovery`, expected note stamps and the existing-destination versus expected-absence protocol distinction.

**Produces:** `compare_note(id: Uuid) -> NoteResult<NoteComparison>`, `reload_note(op: Uuid, id: Uuid, expected: NoteStamp, discard: bool) -> NoteResult<NoteView>`, `relink_note(op: Uuid, id: Uuid, expected: NoteStamp, relative: &Path, confirm_identity: bool) -> NoteResult<NoteView>`, `save_note_copy(request: NoteSubmission, relative: &Path) -> NoteResult<NoteReceipt>`.

Also produce `accept_note_disk_state(ack_op: Uuid, save_op: Uuid, observed_file_state: Uuid) -> NoteResult<NoteView>` for explicitly resolving the writing block after an uncertain/late-conflict outcome. It reobserves the exact reviewed state, records a separate acknowledgement, preserves all uncertain/displaced recovery and the original recorded outcome, and establishes a new baseline without any file write or search approval. It cannot run while the original job is active. A new operation is permitted only after this explicit reconciliation; ordinary reload alone does not silently resolve an uncertain save. Replay of the original operation still returns its original failure/uncertainty, never a rewritten success.

Its store transaction sets resolution AcceptedCurrent and `acknowledged_by` only after checking the bound original operation and fresh observation. Keep local text/generation and the original starting bytes in protected recovery; selecting the reviewed disk baseline does not automatically merge, discard or publish that text. An acknowledgement does not make previously uncertain artifacts eligible for rolling cleanup.

- [ ] **1. Write failing collision/discard tests**:

```rust
let data = tempfile::tempdir().unwrap();
let vault = tempfile::tempdir().unwrap();
std::fs::write(vault.path().join("plan.md"), b"base").unwrap();
let mut w = Workspace::open(data.path(), Config::default()).unwrap();
let opened = w.open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md")).unwrap();
std::fs::write(vault.path().join("other.md"), b"occupied").unwrap();
let request = NoteSubmission {
    operation_id: Uuid::new_v4(), note_id: opened.id,
    expected: opened.stamp, generation: 1, text: "recover me".into(),
};
let failure = w.save_note_copy(request, Path::new("other.md")).unwrap_err();
assert_eq!(failure.code, NoteErrorCode::Conflict);
assert_eq!(std::fs::read(vault.path().join("other.md")).unwrap(), b"occupied");
let dirty = w.note(opened.id).unwrap();
assert_eq!(dirty.buffer, "recover me");
assert!(w.reload_note(Uuid::new_v4(), opened.id, dirty.stamp, false).is_err());
```

- [ ] **2. Run red:** `cargo test -p brn-workflow --test note_conflicts --locked`.
- [ ] **3. Implement read-only three-way comparison and explicit decisions.** Reload validates caller buffer generation, reobserves disk, and creates a new baseline only after discard confirmation. Ambiguous moves/replacements require explicit relink/identity confirmation, never a hash-based guess or historical comment reassignment. An unrelated occupant at a registered path remains a conflict; do not auto-enroll a new identity at that reserved path. This slice does not add note retirement.

```rust
if files.aliases_original(relative, &original_relative)?
    || registry_path_is_reserved(relative)? {
    let failure = note_copy_conflict("copy needs a distinct unregistered path");
    let recorded = store.record_note_write_failure(
        &request, relative, NoteWriteKind::Copy, &failure,
    )?;
    return Err(recorded);
}
let parent = files.parent_identity(relative)?;
let intent = store.begin_note_save(
    &request, relative, NoteWriteKind::Copy, &DestinationPrecondition::Absent { parent },
)?;
let prepared = files.prepare_copy(
    request.operation_id, &intent.staging_relative, relative, request.text.as_bytes(),
)?;
store.record_note_prepared(request.operation_id, &prepared)?;
files.install_exclusive(&prepared, relative)?;
```

Implement `registry_path_is_reserved(&Path) -> NoteResult<bool>` and `note_copy_conflict(&str) -> NoteFailure`; adapter `aliases_original(&Path, &Path) -> NoteResult<bool>`, `parent_identity(&Path) -> NoteResult<VaultIdentity>`, `prepare_copy(op: Uuid, staging: &Path, destination: &Path, bytes: &[u8]) -> NoteResult<PreparedFile>` and `install_exclusive(&PreparedFile, &Path) -> NoteResult<()>` handle real filesystem identities and `RENAME_EXCL`. Check recorded results before fresh validation; operational collision/alias refusals protect valid submitted input through `record_note_write_failure`, not an unrecorded early return.

For existing paths compare validated device/inode identities. For absent targets, check every registered/reserved path, including missing notes and pending copies, using a conservative canonical-Unicode/full-case-fold component key plus resolved parent identity. These keys only veto possibly aliased destinations; they never prove logical note identity. Qualify their conservatism against the supported volume's name equivalence, reject unqualified ambiguous names, and do not use lowercase-only or lexical equality as an exact filesystem rule. Over-rejecting an ambiguous copy destination is preferable to recreating an original under different spelling.

Persist destination reservation, explicit Absent precondition and new target note ID before staging; record prepared identity before installation. A confirmed copy enrolls that preallocated identity and receipt transactionally. Reconciliation/replay cannot allocate another ID or manufacture an original-path save receipt.

- [ ] **4. Cover changing comparison observations, missing originals, occupied/registered-missing destinations, no-force-overwrite, relink confirmation and metadata-only uncertain-outcome acknowledgement. On case-insensitive/normalization-insensitive supported fixtures, reject `Plan.md` versus registered missing `plan.md` and NFC versus NFD equivalents; record actual volume semantics instead of silently skipping the assertion.** Run copy crash cases through the lib hook harness before/after exclusive installation/receipt. Assert source/target receipt IDs, no displaced-file requirement, retained original uncertainty, no fresh identity on replay, and no search permission inherited by a copy.
- [ ] **5. Run:** `cargo test -p brn-workflow --test note_conflicts --test note_recovery --locked` and the lib `notes::crash_tests` selector for copy transitions; commit as `feat(workflow): preserve conflicts and save exclusive recovery copies`.

## Task 6: Current evidence, shadowed imports and provider context

**Files:** Create `crates/brn-workflow/src/notes/eligibility.rs`, `tests/note_evidence.rs`; modify `src/lib.rs` (`sources`, `documents`, import/approval/build/search/evidence/ask paths), `src/error.rs`, `src/main.rs` (`brn-flow` source display), `src/worker.rs` (source-state refresh compatibility), `brn-store/src/notes.rs`, `brn-store/src/workflow.rs`, `brn/src/cli/error.rs`, `brn/src/cli/documents.rs`, `brn-desktop/src/native.rs` (source-state presentation), and `tests/flow.rs`. Update existing `ChatTurn` constructors/JSON projections and exhaustive matches in the task that changes them.

**Consumes:** Note registry, shadowed-source associations, live observation, unresolved-intent gate and additive persisted turn currentness from Task 1.

**Produces:** Private `validate_current_note_evidence(&Evidence) -> Result<()>`, `validate_current_session(session: Uuid) -> Result<()>`, and store-backed content/approval epochs used consistently before index publication, search return, provider submission and turn completion. Add typed workflow `EvidenceStale`/`ContextStale` categories; retain actual provider outcome independently.

Produce `Workspace::approve_note_snapshot(op: Uuid, id: Uuid, expected_file_state: Uuid) -> NoteResult<NoteSearchReceipt>`, with store-owned/re-exported `NoteSearchReceipt { operation_id: Uuid, note_id: Uuid, file_state: Uuid, source_id: Uuid, version_id: Uuid }`. It revalidates the reconciled saved file, freezes exact bytes and grants approval only to that state, never the recovery buffer. Existing import/approval of managed paths must use the same helper; shadowed source IDs cannot be independently reapproved.

Add `source_states(&mut self) -> Result<Vec<SourceStateSummary>>`, workflow-owned `SourceStateSummary { source_id: Uuid, title: String, note_id: Option<Uuid>, current_state: SourceCurrentState, message: Option<String> }` and `SourceCurrentState { Current, Shadowed, Changed, Missing, Unavailable, OwnedElsewhere, Uncertain }`. Use the shared predicate for sources, worker refresh, CLI display and search diagnostics. Expected per-note unavailability is explicit state, not fresh content or a broad catch; database/integrity failures still fail visibly.

Observation persists permission withdrawal, so make observation-dependent `sources(&mut self) -> Result<Vec<SourceDocument>>`, private `documents` and `validate_evidence(&mut self, evidence: &Evidence) -> Result<()>` mutable and update their callers, CLI document signatures and worker `refresh` in this task. Add `source_projection(&mut self) -> Result<(Vec<SourceDocument>, Vec<SourceStateSummary>)>` as the shared single-pass observation/projection; `source_states` delegates to it. Pair freshly validated documents with state summaries in CLI/`brn-flow` display and worker refresh/outcomes/native rows. The old documents-only `sources` wrapper must fail explicitly if it would hide invalid managed sources; index/search paths deliberately consume the validated subset plus surfaced exclusions.

`documents list` retains legacy fields for valid rows and adds current-state metadata; invalid managed rows are labeled with state/reason and no current bytes. `documents show` of a known shadowed/changed/unavailable managed snapshot fails with EvidenceStale, not NotFound merely because it was omitted from the valid subset. Explicit history/revision access preserves originals. Do not quietly drop unavailable notes or present stored approval as live eligibility.

- [ ] **1. Write a failing regression for the old imported-copy path**:

```rust
let data = tempfile::tempdir().unwrap();
let vault = tempfile::tempdir().unwrap();
let path = vault.path().join("plan.md");
std::fs::write(&path, b"oldterm").unwrap();
let mut w = Workspace::open(data.path(), Config::default()).unwrap();
let cancel = std::sync::atomic::AtomicBool::new(false);
let imported = w.import_file(&cancel, Uuid::new_v4(), &path, Approval::Approved).unwrap();
w.build_index(&cancel, |_| {}).unwrap();
let opened = w.open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md")).unwrap();
let approved = w.approve_note_snapshot(
    Uuid::new_v4(), opened.id, opened.current_file_state.unwrap(),
).unwrap();
w.build_index(&cancel, |_| {}).unwrap();
let prior = w.search("oldterm", brn_retrieval::Profile::Keyword).unwrap();
assert!(!prior.evidence.is_empty());
let old_hit = prior.evidence[0].clone();
std::fs::write(&path, b"new current content").unwrap();
assert_eq!(w.validate_evidence(&old_hit).unwrap_err().kind, ErrorKind::EvidenceStale);
assert_eq!(w.search("oldterm", brn_retrieval::Profile::Keyword).unwrap_err().kind,
           ErrorKind::IndexStale);
assert_eq!(w.set_approval(&cancel, Uuid::new_v4(),
    imported.source_id, imported.version_id, Approval::Approved).unwrap_err().kind,
    ErrorKind::EvidenceStale);
assert_ne!(approved.source_id, imported.source_id);
```

Enrollment never inherits search permission. The positive current snapshot belongs to a dedicated managed source; original import identities/provenance stay unchanged. The real keyword index and successful prior query establish the validation precondition; a missing index/profile failure cannot satisfy this regression.

- [ ] **2. Run red:** `cargo test -p brn-workflow --test note_evidence --locked`; current snapshot-only behavior must fail the new live-state assertions.
- [ ] **3. Implement one live predicate**, not per-profile copies. Reconcile exact registered-path import associations conservatively; suppress independent eligibility, preserve immutable originals, and prevent import/approval bypass. Build snapshots only on explicit current import/approval, not ordinary save. Current documents either validate their snapshot against disk or return changed/unavailable; note state reads live content independently of permission.

```rust
let found = self.search(query, profile)?;
if let Some(id) = session {
    self.validate_current_session(id)?;
}
for evidence in &found.evidence {
    self.validate_evidence(evidence)?;
}
```

Run session validation before thread resume/provider access and selected-evidence validation again immediately before turn submission. At completion revalidate and commit provider terminal status/text/currentness atomically with `complete_turn_with_currentness`. Return `AskFailure` with `EvidenceStale`, recorded Completed and provider Confirmed(Completed) when applicable. If that commit fails, do not invent a recorded status or forget an already confirmed provider completion. Same-operation replay returns the recorded stale outcome without resubmission; revalidate currentness before labeling an old receipt current. Historical display remains labeled; streaming is provisional.

Make stale replay exact: an existing StaleAtCompletion turn, or a recorded completed turn whose managed evidence is now ineligible, returns `Err(AskFailure)` with EvidenceStale and its original receipt/provider outcome, before any authentication/submission. A new operation resuming a stale thread returns ContextStale. Check the managed/associated subset of prior application evidence; do not invalidate unrelated legacy sessions just because old rows default to Unqualified. History is readable and labeled, not implicitly replayed into a fresh provider thread.

- [ ] **4. Add positive approval/reapproval/no-op cases**: explicitly approved unchanged content stays searchable across an unchanged save; changed manual content withdraws permission; explicit approval of the new observed state plus rebuild returns only new content; old-version approval fails exactly. Assert stale/unavailable/shadowed document state and no old bytes in a current show response.

Use fake providers for changes before resume, after search/before submission and mid-turn. Assert EvidenceStale/ContextStale, provider call counts, preserved stale answer, truthful Completed provider outcome and failure-envelope replay. Validator-level checks are model-independent but do not qualify actual semantic/hybrid search; native cases require a real eligible index and resources, and a ProfileUnavailable/IndexMissing error cannot count as a stale-evidence pass.
- [ ] **5. Run:** `cargo test -p brn-workflow --test note_evidence --test flow --locked` and `cargo test -p brn --test cli_core --test cli_ask --locked`; keep exhaustive CLI mappings in this task, not Task 7. Run the global workspace check and `cargo check -p brn-desktop --features native-ui --locked` for changed native consumers; commit as `fix(workflow): gate note evidence on current files`.

## Task 7: Agent CLI note operations and typed contextual errors

**Files:** Create `crates/brn/src/cli/notes.rs`, `crates/brn/tests/cli_notes.rs`; modify `cli/mod.rs`, `cli/error.rs`, `cli/ask.rs`, `cli/documents.rs`, `tests/cli_ask.rs`, `README.md`. Preserve the source-state/error compatibility wired in Task 6. Do not add business logic to `brn-flow`; the agent CLI provides headless parity.

**Consumes:** All workflow note methods, `NoteResult`, stamps/receipts and evidence/context categories.

**Produces:** Exact commands below, existing JSON envelope version 1, operational exit 1 for typed note failures, and existing usage/cancellation/timeout codes.

```text
brn notes open PATH --vault DIR [--operation UUID]
brn notes show NOTE_ID
brn notes buffer save NOTE_ID --base-file-state UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
brn notes save NOTE_ID --base-file-state UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
brn notes recovery list
brn notes recovery show NOTE_ID
brn notes recovery reconcile --operation UUID
brn notes recovery accept-current --save-operation UUID --file-state UUID --keep-recovery [--operation UUID]
brn notes compare NOTE_ID
brn notes reload NOTE_ID --base-file-state UUID --expected-generation N --discard-local-edits [--operation UUID]
brn notes relink NOTE_ID --path PATH --base-file-state UUID --expected-generation N --confirm-identity [--operation UUID]
brn notes save-copy NOTE_ID --path PATH --base-file-state UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
brn notes approve-for-search NOTE_ID --file-state UUID [--operation UUID]
```

Vault roots and `notes open PATH` are explicit absolute paths; paths for relink/copy are vault-relative. Parse UUIDs/generations/flags and read exact bounded text before opening the workspace. A mutation never substitutes a newly fetched stamp for caller input. `--operation` is required for selecting a particular recovery reconciliation, not a new operation ID.

For `accept-current`, `--save-operation` names the unresolved original operation, while optional `--operation` identifies the separate acknowledgement. Require the reviewed observation token and `--keep-recovery`; no deletion, implicit discard, filesystem write or approval occurs. A stale token fails. Approval requires `current_file_state`, not the possibly older editing stamp.

- [ ] **1. Write failing subprocess acceptance** using `Command::new(env!("CARGO_BIN_EXE_brn"))`, null stdin and exactly-one-envelope decoding as in existing draft CLI tests:

```rust
let data = tempfile::tempdir().unwrap();
let vault = tempfile::tempdir().unwrap();
let path = vault.path().join("plan.md");
std::fs::write(&path, b"# plan\r\n").unwrap();
let output = Command::new(env!("CARGO_BIN_EXE_brn"))
    .args(["notes", "open", path.to_str().unwrap(),
           "--vault", vault.path().to_str().unwrap(),
           "--data-dir", data.path().to_str().unwrap(), "--json"])
    .stdin(Stdio::null()).output().unwrap();
assert!(output.status.success());
let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
assert_eq!(envelope["schema_version"], 1);
assert_eq!(envelope["ok"], true);
```

Use fresh temporary directories and exact contents in each test. Seed only through the shared workflow; release it before CLI subprocesses acquire ownership.

- [ ] **2. Run red:** `cargo test -p brn --test cli_notes --locked`; expected unrecognized note commands.
- [ ] **3. Implement parsing/dispatch/help and one shared bounded text reader** extracted from `cli/drafts.rs` if needed, preserving preparation-before-open guards. Map note errors to `NOTE_STATE_CHANGED`, `NOTE_CONFLICT`, `NOTE_MISSING`, `NOTE_UNSUPPORTED`, `NOTE_SAVE_UNCERTAIN`, `NOTE_IO_ERROR`, `NOTE_STORAGE_ERROR`, `VAULT_BUSY`, `VAULT_UNAVAILABLE`, existing operation/workspace codes and the Task 6 `EVIDENCE_STALE`/`CONTEXT_STALE` mappings. Use `CliFailure.context` for IDs, phase, outcome and submitted recovery availability; do not inspect message wording.

```rust
let receipt = workspace.save_note(NoteSubmission {
    operation_id: operation.unwrap_or_else(Uuid::new_v4),
    note_id, expected: NoteStamp { file_state, generation: expected_generation },
    generation, text,
}).map_err(note_cli_failure)?;
```

Define `note_cli_failure(NoteFailure) -> CliFailure` in `cli/notes.rs`; return the actual operation receipt and state, not success when only input preparation or recovery succeeded. Add extraction files to the task only when needed; document them in evidence.

- [ ] **4. Exercise open/buffer/save/reopen/recovery/compare/reload/relink/copy/approval parity** across separate subprocesses, including show/save/reconcile without another open, equal-generation recovered saves, missing/busy roots with inspectable recovery, acceptance preserving uncertain artifacts, stale tokens and all contextual error shapes. Cover exact original/copy IDs, replay after later edits, invalid-input no-initialization, signal-after-durable-write honesty, quiet broken pipes and help-before-validation.

Add direct document list/show assertions after an external edit/deletion and for shadowed imports: the list labels state/reason, show exits 1 with EVIDENCE_STALE and never emits old content as current. Assert approval/reapproval and rebuilt positive searches. Add a CLI ask replay case for StaleAtCompletion: exit 1/EVIDENCE_STALE, original receipt/Completed provider context, preserved historical answer, no provider launch; new stale-session attempts use CONTEXT_STALE.
- [ ] **5. Run:** `cargo test -p brn --test cli_notes --test cli_ask --test cli_core --test cli_signals --test cli_drafts_save --locked`; commit as `feat(brn): expose safe Markdown note operations`.

## Task 8: Native note editor and generation-safe worker integration

**Files:** Create `crates/brn-desktop/src/notes.rs`; modify `brn-workflow/src/worker.rs`, `brn-desktop/src/native.rs`, `src/main.rs`, `README.md`; unit tests in note editor/worker modules.

**Consumes:** Note workflow methods, `NoteView`, `NoteSubmission`, `NoteBufferReceipt`, `NoteReceipt`, `NoteFailure`, `NoteFileNotice`, source-state summaries, comparison/recovery interfaces.

**Produces:** Note worker actions/outcomes, additive typed note-failure detail on failed worker terminals, a `NoteEditor` transient state module, and native Choose vault/Open/edit/Save/conflict/recovery controls. Never wrap an error in a successful worker outcome.

- [ ] **1. Write failing editor-state tests** for typing after save submission:

```rust
let note_id = Uuid::new_v4();
let file_state = Uuid::new_v4();
let opened = NoteView {
    id: note_id, vault_id: Uuid::new_v4(), relative_path: PathBuf::from("plan.md"),
    stamp: NoteStamp { file_state, generation: 0 },
    current_file_state: Some(file_state),
    saved: Some("base".into()), buffer: "base".into(),
    availability: NoteAvailability::Available, search_approval: Approval::Draft,
    availability_message: None,
};
let mut editor = NoteEditor::new(opened);
editor.edit("first".into()).unwrap();
let submitted = editor.begin_save(Uuid::new_v4()).unwrap();
editor.edit("later".into()).unwrap();
let receipt = NoteReceipt {
    operation_id: submitted.operation_id, source_note_id: note_id, note_id,
    submitted_generation: submitted.generation,
    stamp: NoteStamp { file_state: Uuid::new_v4(), generation: submitted.generation },
    filesystem_outcome: FileOutcome::Applied, recovery_available: true,
};
editor.acknowledge(submitted.operation_id, receipt).unwrap();
assert_eq!(editor.text(), "later");
assert!(editor.dirty());
```

The receipt acknowledges only `submitted.generation`. `NoteEditor` methods are `new(NoteView)`, `edit(String) -> NoteResult<()>`, `begin_save(Uuid) -> NoteResult<NoteSubmission>`, `acknowledge(Uuid, NoteReceipt) -> NoteResult<()>`, `text() -> &str`, `dirty() -> bool`. Add recovery acknowledgement consuming `NoteBufferReceipt` and confirmed-discard methods with the same operation/generation checks.

- [ ] **2. Run red:** `cargo test -p brn-desktop --features native-ui notes:: --locked`; expected missing note editor. Keep this module native-feature gated like existing drafts.
- [ ] **3. Wire `worker::Action` variants for every workflow note operation**, retaining submitted operation/stamp/generation in outcomes and `NoteFailure` on failed terminals. Handle failures without dropping durable outcome context. Preserve single-owned-connection/worker ownership and existing provider cancellation semantics.

```rust
let pending = self.pending.as_ref().ok_or_else(|| note_state_changed("no pending save"))?;
if receipt.operation_id != pending.operation_id
    || receipt.source_note_id != pending.note_id
    || receipt.note_id != pending.note_id
    || receipt.submitted_generation != pending.generation {
    return Err(note_state_changed("receipt does not match submitted save"));
}
self.acknowledged = receipt.stamp;
self.pending = None;
```

Define `pending: Option<NoteSubmission>` and `acknowledged: NoteStamp` in `NoteEditor`, and `note_state_changed(&str) -> NoteFailure` locally. Acknowledgement updates the persisted baseline/stamp while retaining newer text rather than calling native `set_value` with the submitted text. Preserve the remaining later-generation recovery submission when an older receipt arrives.

Handle a save-copy receipt separately: require the pending copy operation/generation and matching `source_note_id`, with a distinct destination `note_id`. Validate the returned destination view before displaying it. Keep the original buffer and any uncertain original operation protected; do not feed that receipt into original-note acknowledgement or silently switch away from later edits.

The existing worker rejects submissions while busy and has a one-item channel. Keep a coalesced latest pending recovery submission in UI state, dispatch when idle, show recovery pending honestly, and flush/retry before close/switch. Do not block the GUI thread or create another Store. Long provider work may delay recovery; user-visible pending status and close guards must reflect that rather than promising a 500 ms durability deadline.

Define `Worker::take_note_notice() -> Option<NoteFileNotice>` as a lightweight drain of the Workspace's shared notice queue installed during worker startup. The native polling loop drains it independently of job terminals/snapshot-generation changes, marks matching notes for observation, and coalesces an ObserveNote job for the next idle slot. It never reads files on the GUI thread. RescanRequired schedules all relevant registered-note observations; moved/deleted events never retarget buffers automatically.

All window-close, Quit/menu/keybinding and note-switch paths must consult a note close guard as well as the existing draft guard. While a note save/copy/recovery commit or flush is pending, defer close, dispatch the latest recoverable buffer when idle, consume its durable acknowledgement, and only then close. On failure keep the window/work accessible or require explicit discard; an uncertain operation remains durably protected and visibly uncertain. Test a queued recovery and in-flight save at each close route.

Add a critical note-mutation admission flag to worker phase state, set before queueing save/copy/buffer/reload/relink/acknowledgement/approval mutations. Shutdown must drain admitted critical note jobs and join their owner, not discard them because `closing` is set or send them down the existing 250 ms detached-reaper path. Retain the bounded reaper for non-note native model loading. Normal UI close waits asynchronously for acknowledgements, so it never invokes a blocking shutdown while a critical note job is active; defensive Drop/headless shutdown must still retain/join that owner. Cancellation cannot abandon an already-started filesystem handoff without a durable known/uncertain outcome.

- [ ] **4. Add Choose vault/Open Markdown note and Cmd-S**, using existing GPUI-kit styling. Render Unsaved/Recoverable/Saving/Saved/Conflict/Uncertain and root-unavailable/owned-elsewhere messages, comparison with deletion/retained displacement, confirmed reload/relink, explicit accept-current with recovery retained, and copy selection. Display source-state labels in the existing source list as well as the note editor. Existing native import/search-approval controls must route managed notes through the shared saved-snapshot helper and clearly remain separate from Save. Observe again on focus/actions; notifications are not eligibility proof.
- [ ] **5. Add queued/in-flight worker shutdown and close/retry cases.** Pause a critical job using the lib-test hook, request shutdown from a separate test thread, assert it cannot finish before the job is released, then assert its committed recovery/receipt and actual file outcome after join. Test close while busy, failed flush retaining the window, successful retry, and a later edit arriving during an earlier flush. Verify notices arrive while another job is active and coalesce into an observation only when idle.

Run `cargo test -p brn-workflow --lib worker:: --locked` and `cargo test -p brn-desktop --features native-ui notes:: --locked`; then `cargo build -p brn-desktop --features native-ui --locked`. Check actual BOM/CRLF/Unicode round trips through the native editor, not merely store strings. If GPUI editor normalization changes bytes, preserve unchanged text/line-ending representation in the editing module and add regression coverage before claiming exactness. Commit as `feat(desktop): edit and recover local Markdown notes`.

## Task 9: Integrated qualification and documented handoff

**Files:** Update `docs/architecture/overview.md`, `invariants.md`, affected crate READMEs, `docs/status.md`, this task's `evidence.md`, and active/completed indexes when the task actually closes.

**Consumes:** Tasks 1-8, all approved spec acceptance criteria and repository verification/evidence conventions.

**Produces:** Measured verification/qualification, accurate implemented authority documentation, and a reproducible handoff. No unobserved native acceptance or merge/release claim.

- [ ] **1. Run integrated verification once** after all targeted task gates have passed:

```sh
bash scripts/verify-end-to-end.sh
```

The integrated script already performs workspace format/build/Clippy/tests and its fixture, including default lib crash tests. Do not pre-run the same package suites immediately before it or rerun overlapping full suites through storage/desktop scripts. Run only additional native/resource-qualified cases separately. The sample desktop `--headless-check` is not qualification of the integrated note worker or native editor.

- [ ] **2. Qualify optional native paths separately**:

```sh
cargo test -p brn-desktop --features native-ui --locked
cargo clippy -p brn-desktop --features native-ui --all-targets --locked -- -D warnings
cargo test -p brn-workflow --features native-retrieval --locked
```

Missing model resources remain reported as resource-gated/unavailable, not passed semantic/hybrid behavior. No model download without current authorization.

- [ ] **3. Observe the disposable native acceptance sequence:** save `plan.md`, verify exact bytes in another editor, externally edit/delete/replace it, observe blocked save and retained buffer, save a noncolliding copy, type during save, quit/restart and exercise a test-induced interrupted save. Record which editor participated in coordination, filesystem/OS/toolchain/features, and actual concurrency limitations. Never use the original vault.
- [ ] **4. Update implemented contracts and evidence**, distinguishing automated passes, native observations, user acceptance and remaining qualification. Record commands/commit/dirty state, failures and skipped cases. Move this task to completed only when its implementation scope/evidence are actually complete; design approval is not native acceptance.
- [ ] **5. Run documentation link checks and `git diff --check`**; commit the final qualification record as `docs: record safe Markdown editing qualification`. Do not merge, push or release without separate authorization.

## Plan self-review and execution gate

Coverage: spec sections 1-3 map to Tasks 1-3/8; original saves/restart to Task 4; conflict/reload/relink/copy to Task 5; eligibility/import aliases/conversations to Task 6; shared desktop/headless contracts to Tasks 7-8; measured acceptance/handoff to Task 9. Every task consumes the global constraints.

Self-review must verify type/member consistency, exact-byte/native editor behavior, startup interruption versus dedicated save phases, operation replay before fresh-state validation, current evidence on every related surface, and copy-specific versus exchange-specific recovery. The implementation snippets are targeted contracts/algorithms, not prewritten substitutes for the red-green cycles.

Written spec approved; implementation has not been selected or started. After plan review, offer inline execution with checkpoints or explicitly authorized subagent-driven execution. Use the chosen execution skill and isolated worktree at that time.
