# Safe Markdown Note Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Follow the current user's delegation preference; inline execution is the repository default. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Open an existing local Markdown note, protect unfinished edits, save to the real file, and safely reconcile external changes or interrupted saves.

**Architecture:** A deep note-editing module in `brn-workflow` coordinates SQLite registry/recovery records and a private macOS filesystem adapter. Desktop and CLI use the same generation-checked interface. Live files, not imported database snapshots, determine managed-note current evidence.

**Tech Stack:** Rust 1.98.1, existing SQLite/rusqlite, SHA-256, UUID/Serde, macOS Foundation file coordination, atomic exchange/exclusive installation, GPUI-kit native UI, and existing subprocess/fake-provider tests.

**Spec:** [Approved design](../../../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md). Read it together with this plan.

## Global Constraints

- Baseline: `main` at `32077d8`; written specification approved on 1 October 2026. Planning only; implementation/verification have not started.
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
- Use explicit disposable data/vault/lock directories and synthetic fixtures. No original-vault access, live provider calls, model acquisition, merge or release.
- Respect the pinned toolchain and lockfiles. Install/restore dependencies only after manifest changes or an actual missing-dependency failure.
- Keep interface changes additive where possible; existing CLI envelopes stay schema version 1 and existing command semantics remain documented.
- Execute in isolation using `using-git-worktrees` at execution time; preserve unrelated changes. Do not create a worktree merely to write this plan.
- Every implementation commit includes `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.

## File ownership and task dependencies

| Files | Responsibility | Task |
| --- | --- | --- |
| `crates/brn-store/src/notes.rs`, `src/lib.rs`, `tests/notes.rs`, `tests/storage.rs` | Typed registry/buffers/intents/receipts, schema migration and recovery retention | 1 |
| `crates/brn-workflow/src/notes/files.rs`, `notes/macos.rs`, `Cargo.toml`, root `Cargo.lock` | Bounded reads, supported filesystem operations, Foundation coordination/presentation, vault ownership | 2 |
| `crates/brn-workflow/src/notes/mod.rs`, `src/lib.rs`, `src/error.rs`, `tests/notes.rs` | Open/state/buffer workflow and error contracts | 3 |
| `crates/brn-workflow/src/notes/save.rs`, `tests/note_recovery.rs` | Original-path exchange, receipts, crash reconciliation and cleanup | 4 |
| `crates/brn-workflow/src/notes/conflicts.rs`, `tests/note_conflicts.rs` | Comparison, confirmed reload/relink, independent save-copy | 5 |
| `crates/brn-workflow/src/notes/eligibility.rs`, `src/lib.rs`, `brn-store/src/workflow.rs`, `tests/flow.rs`, `tests/note_evidence.rs` | Current snapshots, associated imports, provider context/completion guards | 6 |
| `crates/brn/src/cli/notes.rs`, `cli/mod.rs`, `cli/error.rs`, `cli/ask.rs`, `tests/cli_notes.rs`, `README.md` | Agent CLI parity, contextual errors, help and exact-byte inputs | 7 |
| `crates/brn-workflow/src/worker.rs`, `brn-desktop/src/notes.rs`, `src/native.rs`, `src/main.rs`, `README.md` | Note worker actions, native state/editor/conflicts and recovery scheduling | 8 |
| Architecture/crate documentation, task evidence, status/work indexes | Integrated and native qualification, reproducible handoff | 9 |

Task 1 and Task 2 are independently testable. Task 3 consumes both; Task 4 consumes Task 3; Task 5 consumes Task 4; Task 6 consumes Tasks 3-5. Tasks 7 and 8 consume Tasks 3-6 and can be reviewed independently. Task 9 consumes all implementation tasks. This is one cross-surface safety feature, not separately authorized archive/history/AI-review projects.

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
pub enum NoteAvailability { Available, Missing, Conflict, Unsupported, Uncertain }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteView {
    pub id: Uuid,
    pub vault_id: Uuid,
    pub relative_path: PathBuf,
    pub stamp: NoteStamp,
    pub saved: Option<String>,
    pub buffer: String,
    pub availability: NoteAvailability,
    pub search_approval: Approval,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavePhase { Intent, Prepared, Exchanged, Verified, Complete }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileOutcome { NotApplied, Applied, Unknown }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteReceipt {
    pub operation_id: Uuid,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteErrorCode {
    StateChanged, Conflict, Missing, Unsupported, SaveUncertain, Io,
    OperationConflict, WorkspaceBusy,
}
pub type NoteResult<T> = std::result::Result<T, NoteFailure>;
```

A successful write returns an Applied receipt only after the file and receipt are verified. A verified unchanged save returns NotApplied as an explicitly documented no-op. Conflict/uncertainty is `Err(NoteFailure)`, never an `Ok` receipt that resembles successful saving. Reconciliation can successfully report NotApplied without writing. `NoteView.saved` is a fresh disk observation; stored recovery bytes must never masquerade as that field.

`NoteView.stamp` is the editing buffer's expected baseline/generation. While dirty/conflicted, it does not silently switch to a newly observed external file state. Reload/relink establishes a new baseline explicitly; a successful own save rebases the retained later edits without changing their text.

Additional store-owned types: `FileFingerprint { device: u64, inode: u64, len: u64, sha256: [u8; 32] }`; `VaultIdentity { device: u64, inode: u64 }`; `NoteRecovery { note_id: Uuid, stamp: NoteStamp, baseline: String, working: String, pending_operations: Vec<Uuid> }`; `NoteComparison { note_id: Uuid, stamp: NoteStamp, baseline: String, working: String, observed: Option<String>, availability: NoteAvailability }`; `PreparedFile { relative: PathBuf, fingerprint: FileFingerprint }`; `NoteWriteKind { Replace, Copy }`; `NoteSaveIntent { request: NoteSubmission, destination: PathBuf, kind: NoteWriteKind, baseline: FileFingerprint, baseline_text: String, staged: Option<PreparedFile>, phase: SavePhase, prior_receipt: Option<NoteReceipt> }`. Derive exact equality and Serde where persisted. Source permission remains the existing `Approval` type. Do not re-export raw prepared-file/intent metadata to native view state.

## Task 1: Durable note registry, buffers, intents and receipts

**Files:** Create `crates/brn-store/src/notes.rs`, `crates/brn-store/tests/notes.rs`; modify `crates/brn-store/src/lib.rs` (module exports, V6 initialization/migration, `validate_schema`, local operation kinds), `tests/storage.rs`, and `src/workflow.rs` (additive turn-currentness projection).

**Consumes:** Existing `Store`, SQLite transaction helpers, `operations` payload binding, exact-byte hashes and V1-V5 schema validation.

**Produces:** Shared types above; store methods `enroll_note(op: Uuid, root: &Path, vault: VaultIdentity, relative: &Path, file: FileFingerprint, text: &str) -> Result<NoteRecovery>`, `save_note_buffer(request: &NoteSubmission) -> Result<NoteBufferReceipt>`, `note_recovery(id: Uuid) -> Result<Option<NoteRecovery>>`. Transactional intent/prepared/verification/receipt methods used by Task 4 remain crate-facing storage operations with typed records, not filesystem calls. Store methods never return a purported fresh `NoteView.saved`.

Store save interfaces are `begin_note_save(&NoteSubmission, &Path, NoteWriteKind) -> Result<NoteSaveIntent>`, `record_note_prepared(Uuid, &PreparedFile) -> Result<()>`, `record_note_verification(Uuid, Option<&str>) -> Result<()>`, `finish_note_save(Uuid, &NoteReceipt) -> Result<()>`, `note_save_intent(Uuid) -> Result<Option<NoteSaveIntent>>`, `note_save_intents() -> Result<Vec<NoteSaveIntent>>`, and `prune_completed_note_payloads(Uuid) -> Result<()>`. Verification carries displaced text for exchange and no displaced text for copy; terminal commit must atomically reconcile registry/permission/buffer/receipt state.

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
    Uuid::new_v4(), vault.path(), VaultIdentity { device: 1, inode: 2 },
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

Every newly created table/index and the additive `chat_turns.evidence_currentness` column must enter the expected-schema builder. Its checked values are `unqualified`, `current_at_completion`, `stale_at_completion`, with existing rows defaulting to `unqualified`. Update every old migration path to include V6 once, and add V5-to-V6 transactional migration; generic startup interruption must not erase dedicated save phases.

Define `EvidenceCurrentness { Unqualified, CurrentAtCompletion, StaleAtCompletion }` with checked SQL spelling conversion and a default of Unqualified; add it to `ChatTurn` with an additive Serde default. Produce `Store::complete_turn_with_currentness(op: Uuid, status: OperationStatus, text: &str, usage: Option<&str>, currentness: EvidenceCurrentness) -> Result<()>`, committing status/text/currentness atomically. Keep existing `complete_turn` as the existing-semantics wrapper.

- [ ] **4. Add green cases** for V1-V5 upgrade, killed uncommitted migration, corrupt receipt/hash/schema, wrong generation, coalescing buffers, safe completed-payload retirement and protected unresolved payloads. Reuse the existing subprocess `child_worker`/acknowledgement pattern.
- [ ] **5. Run:** `cargo test -p brn-store --test notes --test storage --test workflow --locked`; expected all selected tests pass. Commit these files with `feat(store): persist managed-note recovery and save intents` and the required trailer.

## Task 2: macOS filesystem and vault-ownership adapter

**Files:** Create `crates/brn-workflow/src/notes/mod.rs` with private adapter module declarations, `notes/files.rs`, `notes/macos.rs`; modify `src/lib.rs` to declare the notes module, `crates/brn-workflow/Cargo.toml` and root `Cargo.lock`. Unit tests live beside the private adapter.

**Consumes:** `FileFingerprint`, `VaultIdentity`, exact UTF-8/1 MiB requirements and the approved concurrency limitation.

**Produces:** Private `MacFiles::open(root: &Path, data_dir: &Path, lock_dir: &Path) -> NoteResult<MacFiles>`, `observe(relative: &Path) -> NoteResult<FileObservation>` with `FileObservation { fingerprint: FileFingerprint, text: String }`, coordinated-operation closures, stage/flush/exchange/exclusive-install primitives and notifications. Use the store-owned `PreparedFile` to identify owned staging; do not expose this to views.

Private filesystem methods are `coordinate<T>(&self, relative: &Path, action: impl FnOnce() -> NoteResult<T>) -> NoteResult<T>`, `prepare_replace(Uuid, &Path, &[u8]) -> NoteResult<PreparedFile>`, `exchange(&PreparedFile, &Path) -> NoteResult<()>`, and `flush_artifact(&Path) -> NoteResult<()>`. Task 5 adds copy-specific counterparts. The coordination accessor runs synchronously while its scope/ownership is held; presenter notifications only enqueue observation hints.

- [ ] **1. Write failing unit tests** for exact bounded observation and ownership:

```rust
let vault = tempfile::tempdir().unwrap();
let data = tempfile::tempdir().unwrap();
let locks = tempfile::tempdir().unwrap();
std::fs::write(vault.path().join("plan.md"), b"\xef\xbb\xbf# plan\r\n").unwrap();
let files = MacFiles::open(vault.path(), data.path(), locks.path()).unwrap();
assert_eq!(files.observe(Path::new("plan.md")).unwrap().text.as_bytes(),
           b"\xef\xbb\xbf# plan\r\n");
assert!(MacFiles::open(vault.path(), data.path(), locks.path()).is_err());
```

- [ ] **2. Run red:** `cargo test -p brn-workflow --lib notes::files --locked`; expected missing adapter. Keep root/lock fixtures private and disposable.
- [ ] **3. Implement the adapter.** Use directory-relative operations, no-follow containment, a `limit + 1` bounded read, UTF-8 validation, SHA-256 and descriptor metadata. Validate single-link regular files before acting. Acquire shared ancestry/exclusive-root locks in root-to-leaf order; retain file handles for the adapter lifetime. Reject data directories within the vault and changed roots.

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

Use target-macOS dependencies matching inspected lockfile releases: `libc = "=0.2.189"`, `objc2 = "=0.6.4"`, `objc2-foundation = "=0.3.2"`, `block2 = "=0.6.2"`. Enable Foundation's concrete `NSFileCoordinator`, `NSFilePresenter`, `NSOperationQueue`, `NSURL`, `NSString`, `NSError` support and required block support in the manifest. Keep Foundation ownership/registration in `macos.rs`, register/unregister presenters deterministically, and invoke workflow closures only through coordinated accessors. Non-macOS note writes report unsupported; existing commands keep compiling on their supported Unix targets.

- [ ] **4. Exercise physical primitives** against empty/exact-limit/over-limit/invalid-UTF-8 inputs, links, escaping paths, nested/aliased ownership roots, external atomic replacement, missing exchange targets, exclusive-create collisions, coordination errors and attribute/durability failures. Verify the displaced inode/bytes survive exchange. Test observer lifecycle and that callback delivery does not synchronously perform workflow writes.
- [ ] **5. Run:** `cargo test -p brn-workflow --lib notes:: --locked`. Commit as `feat(workflow): coordinate supported local note files`.

## Task 3: Shared open/state/buffer workflow

**Files:** Create `crates/brn-workflow/tests/notes.rs`; modify `src/notes/mod.rs`, `src/lib.rs`, `src/error.rs`.

**Consumes:** Task 1 store methods and Task 2 adapter. Production lock location is application-global; tests inject a disposable lock directory through a private constructor, never a public bypass.

**Produces:** `Workspace::open_note(op: Uuid, vault: &Path, relative: &Path) -> NoteResult<NoteView>`, `note(id: Uuid) -> NoteResult<NoteView>`, `save_note_buffer(request: NoteSubmission) -> NoteResult<NoteBufferReceipt>`, `note_recoveries() -> NoteResult<Vec<NoteRecovery>>`. Keep the selected adapter/root lifetime owned by `Workspace`; no second SQLite connection for autosave.

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
- [ ] **3. Implement enrollment/state observation**, loading protected buffers without replacing their baseline, updating observation IDs conservatively and preserving opaque note identity. Read actual disk bytes for `saved`; represent missing/unsupported/ambiguous states explicitly. Add typed note-code conversion, retaining `NoteFailure` context rather than flattening it into English matching.

```rust
pub fn save_note_buffer(&mut self, request: NoteSubmission) -> NoteResult<NoteBufferReceipt> {
    self.store.save_note_buffer(&request).map_err(store_note_failure)
}
```

The store checks operation replay before validating the stored editing baseline/generation; it does not require conflicted disk content to match that baseline. Recovery must still work after external changes. `store_note_failure(brn_store::Error) -> NoteFailure` preserves typed operation/workspace conflicts and I/O failures. Enrich a new failure with the supplied note/operation IDs at the call site without changing its known outcome.

- [ ] **4. Verify reopening acknowledged buffers**, empty notes, explicit unsupported states, changed disk observations, stale buffer mutations, duplicate operations and recovery during external deletion. Operation replay must be checked before current-state preconditions so a receipt can be replayed after later edits.
- [ ] **5. Run:** `cargo test -p brn-store -p brn-workflow --test notes --locked`; commit as `feat(workflow): open notes with generation-checked recovery buffers`.

## Task 4: Original-path save and crash reconciliation

**Files:** Create `crates/brn-workflow/src/notes/save.rs`, `tests/note_recovery.rs`; modify `notes/mod.rs`, store intent/receipt methods in `brn-store/src/notes.rs`.

**Consumes:** `NoteSubmission`, expected fingerprints/baseline recovery and Task 2 coordinated filesystem primitives.

**Produces:** `Workspace::save_note(request: NoteSubmission) -> NoteResult<NoteReceipt>`, `reconcile_note_save(op: Uuid) -> NoteResult<NoteReceipt>`. Private save phases expose test-only hooks, not production environment-controlled crash switches.

The private helpers below use `NoteSaveObservations { destination: Option<FileObservation>, staging: Option<FileObservation> }` and `RecoveryDecision { NotApplied, Applied, Conflict, Uncertain }`, defined in `save.rs`. Missing paths alone become `None`; unreadable or unexpected objects produce explicit failures with preserved artifacts. Their signatures are `load_note_intent(Uuid) -> NoteResult<NoteSaveIntent>`, `observe_note_intent(&NoteSaveIntent) -> NoteResult<NoteSaveObservations>`, `classify_note_save(&NoteSaveIntent, &NoteSaveObservations) -> NoteResult<RecoveryDecision>`, and `commit_note_reconciliation(NoteSaveIntent, RecoveryDecision) -> NoteResult<NoteReceipt>`.

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

```rust
pub fn reconcile_note_save(&mut self, op: Uuid) -> NoteResult<NoteReceipt> {
    let intent = self.load_note_intent(op)?;
    let observations = self.observe_note_intent(&intent)?;
    let decision = classify_note_save(&intent, &observations)?;
    self.commit_note_reconciliation(intent, decision)
}
```

Define these four private operations in `save.rs`; `classify_note_save` implements every restart-table row in the spec. Observation is read-only; `commit_note_reconciliation` performs only store bookkeeping. No reconciliation branch invokes exchange, ordinary rename or target creation. Unknown/mismatched artifacts remain retained and fail with phase/outcome context.

- [ ] **4. Add process-crash coverage** using the existing test-executable child pattern: child acknowledges one named hook, parent kills that exact `Child`, waits, reopens, and verifies file/artifact/buffer/receipt state. Cover before/after intent commit, staging flush, prepared identity commit, exchange, directory flush, displaced retention, receipt commit and safe payload cleanup. Add post-precheck race hooks for content writes, deletion and replacement; never automatically rollback unexpected displaced files.
- [ ] **5. Verify no-op timestamp/identity/permission preservation**, stale generations, continued typing via two recovery generations, same-ID replay after later edits and completed-payload retirement. Run `cargo test -p brn-workflow --test note_recovery --locked`; commit as `feat(workflow): journal and reconcile Markdown saves`.

## Task 5: Conflict comparison, confirmed reload/relink and exclusive save-copy

**Files:** Create `crates/brn-workflow/src/notes/conflicts.rs`, `tests/note_conflicts.rs`; modify `notes/mod.rs`, `notes/save.rs`, and store note transactions.

**Consumes:** Tasks 3-4, `NoteComparison`, `NoteRecovery`, expected note stamps and the existing-destination versus expected-absence protocol distinction.

**Produces:** `compare_note(id: Uuid) -> NoteResult<NoteComparison>`, `reload_note(op: Uuid, id: Uuid, expected: NoteStamp, discard: bool) -> NoteResult<NoteView>`, `relink_note(op: Uuid, id: Uuid, expected: NoteStamp, relative: &Path, confirm_identity: bool) -> NoteResult<NoteView>`, `save_note_copy(request: NoteSubmission, relative: &Path) -> NoteResult<NoteReceipt>`.

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
- [ ] **3. Implement read-only three-way comparison and explicit decisions.** Reload validates caller buffer generation, reobserves disk, and creates a new baseline only after discard confirmation. For ambiguous moves/replacements, explicit relink/identity confirmation never guesses from hashes or retargets historical comments. An unrelated occupant can instead open as a separate note identity; the original stays missing.

```rust
if relative == original_relative || registry_path_is_reserved(relative)? {
    return Err(note_copy_conflict("copy needs a distinct unregistered path"));
}
let prepared = files.prepare_copy(request.operation_id, relative, request.text.as_bytes())?;
files.install_exclusive(&prepared, relative)?;
```

Implement `registry_path_is_reserved(&Path) -> NoteResult<bool>` and `note_copy_conflict(&str) -> NoteFailure` in this workflow; adapter `prepare_copy(Uuid, &Path, &[u8]) -> NoteResult<PreparedFile>` and `install_exclusive(&PreparedFile, &Path) -> NoteResult<()>` use durable prepared identity and `RENAME_EXCL`. Persist the copy intent before this code, record prepared identity before installation, and verify/commit afterward. A successful receipt names the new note identity; an original uncertain save remains unresolved.

- [ ] **4. Cover changing comparison observations, missing originals, aliased destinations, occupied/registered-missing destinations, no-force-overwrite, relink confirmation, and copy crashes before/after exclusive installation/receipt.** Reconciliation recognizes installed copies without needing a displaced file.
- [ ] **5. Run:** `cargo test -p brn-workflow --test note_conflicts --test note_recovery --locked`; commit as `feat(workflow): preserve conflicts and save exclusive recovery copies`.

## Task 6: Current evidence, shadowed imports and provider context

**Files:** Create `crates/brn-workflow/src/notes/eligibility.rs`, `tests/note_evidence.rs`; modify `src/lib.rs` (`sources`, `documents`, import/approval/build/search/evidence/ask paths), `brn-store/src/notes.rs`, `brn-store/src/workflow.rs`, and `tests/flow.rs`.

**Consumes:** Note registry, shadowed-source associations, live observation, unresolved-intent gate and additive persisted turn currentness from Task 1.

**Produces:** Private `validate_current_note_evidence(&Evidence) -> Result<()>`, `validate_current_session(session: Uuid) -> Result<()>`, and store-backed content/approval epochs used consistently before index publication, search return, provider submission and turn completion. Add typed workflow `EvidenceStale`/`ContextStale` categories; retain actual provider outcome independently.

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
w.open_note(Uuid::new_v4(), vault.path(), Path::new("plan.md")).unwrap();
std::fs::write(&path, b"new current content").unwrap();
assert!(w.search("oldterm", brn_retrieval::Profile::Keyword).is_err());
assert!(w.set_approval(&cancel, Uuid::new_v4(),
    imported.source_id, imported.version_id, Approval::Approved).is_err());
```

Do not assume enrollment inherits search permission.

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

- [ ] **4. Add fake-provider tests** for changes before resume, after search/before submission and mid-turn; verify no stale current-success envelope, no reuse of historical thread context, preserved stale answer and honest provider outcome. Test all profiles through the shared eligibility seam; actual native profile coverage remains resource-qualified.
- [ ] **5. Run:** `cargo test -p brn-store -p brn-workflow --locked` once for this cross-cutting change; commit as `fix(workflow): gate note evidence on current files`.

## Task 7: Agent CLI note operations and typed contextual errors

**Files:** Create `crates/brn/src/cli/notes.rs`, `crates/brn/tests/cli_notes.rs`; modify `cli/mod.rs`, `cli/error.rs`, `cli/ask.rs`, `README.md`. Do not add business logic to `brn-flow`; the agent CLI provides headless parity.

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
brn notes compare NOTE_ID
brn notes reload NOTE_ID --base-file-state UUID --expected-generation N --discard-local-edits [--operation UUID]
brn notes relink NOTE_ID --path PATH --base-file-state UUID --expected-generation N --confirm-identity [--operation UUID]
brn notes save-copy NOTE_ID --path PATH --base-file-state UUID --expected-generation N --generation N --text-file PATH [--operation UUID]
```

Vault roots and `notes open PATH` are explicit absolute paths; paths for relink/copy are vault-relative. Parse UUIDs/generations/flags and read exact bounded text before opening the workspace. A mutation never substitutes a newly fetched stamp for caller input. `--operation` is required for selecting a particular recovery reconciliation, not a new operation ID.

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
- [ ] **3. Implement parsing/dispatch/help and one shared bounded text reader** extracted from `cli/drafts.rs` if needed, preserving existing semantics and preparation-before-open guards. Map note errors to `NOTE_STATE_CHANGED`, `NOTE_CONFLICT`, `NOTE_MISSING`, `NOTE_UNSUPPORTED`, `NOTE_SAVE_UNCERTAIN`, `NOTE_IO_ERROR`, existing operation/workspace codes, and `EVIDENCE_STALE`/`CONTEXT_STALE`. Use `CliFailure.context` for note/operation/phase/outcome/recovery availability; do not inspect message wording.

```rust
let receipt = workspace.save_note(NoteSubmission {
    operation_id: operation.unwrap_or_else(Uuid::new_v4),
    note_id, expected: NoteStamp { file_state, generation: expected_generation },
    generation, text,
}).map_err(note_cli_failure)?;
```

Define `note_cli_failure(NoteFailure) -> CliFailure` in `cli/notes.rs`; return the actual operation receipt and state, not success when only input preparation or recovery succeeded. Add extraction files to the task only when needed; document them in evidence.

- [ ] **4. Exercise create/open/buffer/save/reopen/recovery/compare/reload/relink/copy parity**, stale inputs, all contextual error shapes, replay after later edits, invalid-input no-initialization, busy ownership, signal-after-durable-write honesty, quiet broken pipes and help-before-validation.
- [ ] **5. Run:** `cargo test -p brn --test cli_notes --test cli_core --test cli_signals --test cli_drafts_save --locked`; commit as `feat(brn): expose safe Markdown note operations`.

## Task 8: Native note editor and generation-safe worker integration

**Files:** Create `crates/brn-desktop/src/notes.rs`; modify `brn-workflow/src/worker.rs`, `brn-desktop/src/native.rs`, `src/main.rs`, `README.md`; unit tests in note editor/worker modules.

**Consumes:** Note workflow methods, `NoteView`, `NoteSubmission`, `NoteBufferReceipt`, `NoteReceipt`, `NoteFailure`, comparison/recovery interfaces.

**Produces:** Note worker actions/outcomes, additive typed note-failure detail on failed worker terminals, a `NoteEditor` transient state module, and native Choose vault/Open/edit/Save/conflict/recovery controls. Never wrap an error in a successful worker outcome.

- [ ] **1. Write failing editor-state tests** for typing after save submission:

```rust
let note_id = Uuid::new_v4();
let file_state = Uuid::new_v4();
let opened = NoteView {
    id: note_id, vault_id: Uuid::new_v4(), relative_path: PathBuf::from("plan.md"),
    stamp: NoteStamp { file_state, generation: 0 },
    saved: Some("base".into()), buffer: "base".into(),
    availability: NoteAvailability::Available, search_approval: Approval::Draft,
};
let mut editor = NoteEditor::new(opened);
editor.edit("first".into()).unwrap();
let submitted = editor.begin_save(Uuid::new_v4()).unwrap();
editor.edit("later".into()).unwrap();
let receipt = NoteReceipt {
    operation_id: submitted.operation_id, note_id,
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
    || receipt.note_id != pending.note_id
    || receipt.submitted_generation != pending.generation {
    return Err(note_state_changed("receipt does not match submitted save"));
}
self.acknowledged = receipt.stamp;
self.pending = None;
```

Define `pending: Option<NoteSubmission>` and `acknowledged: NoteStamp` in `NoteEditor`, and `note_state_changed(&str) -> NoteFailure` locally. Acknowledgement updates the persisted baseline/stamp while retaining newer text rather than calling native `set_value` with the submitted text. Preserve the remaining later-generation recovery submission when an older receipt arrives.

Handle a save-copy receipt separately: it identifies the new note, not a successful original-path save. Keep the original buffer and any uncertain original operation protected; do not feed that receipt into original-note acknowledgement or silently switch away from later edits.

The existing worker rejects submissions while busy and has a one-item channel. Keep a coalesced latest pending recovery submission in UI state, dispatch when idle, show recovery pending honestly, and flush/retry before close/switch. Do not block the GUI thread or create another Store. Long provider work may delay recovery; user-visible pending status and close guards must reflect that rather than promising a 500 ms durability deadline.

- [ ] **4. Add Choose vault/Open Markdown note and Cmd-S**, using the app's existing GPUI-kit styling. Render distinct Unsaved/Recoverable/Saving/Saved/Conflict/Uncertain states, read-only comparison with deletion, confirmed reload/relink and copy destination selection. Presenter/parent-directory notifications enqueue observation rather than mutate buffers. Observe again on focus and before actions; notifications are not eligibility proof.
- [ ] **5. Run:** `cargo test -p brn-workflow --lib worker:: --locked` and `cargo test -p brn-desktop --features native-ui notes:: --locked`; then `cargo build -p brn-desktop --features native-ui --locked`. Check actual BOM/CRLF/Unicode round trips through the native editor, not merely store strings. If GPUI editor normalization changes bytes, preserve unchanged text/line-ending representation in the editing module and add regression coverage before claiming exactness. Commit as `feat(desktop): edit and recover local Markdown notes`.

## Task 9: Integrated qualification and documented handoff

**Files:** Update `docs/architecture/overview.md`, `invariants.md`, affected crate READMEs, `docs/status.md`, this task's `evidence.md`, and active/completed indexes when the task actually closes.

**Consumes:** Tasks 1-8, all approved spec acceptance criteria and repository verification/evidence conventions.

**Produces:** Measured verification/qualification, accurate implemented authority documentation, and a reproducible handoff. No unobserved native acceptance or merge/release claim.

- [ ] **1. Run smallest combined suites** after fixing any remaining integration regressions:

```sh
cargo fmt --all -- --check
cargo test -p brn-store -p brn-workflow -p brn --locked
bash scripts/verify-end-to-end.sh
```

The integrated script already performs workspace build/Clippy/tests and its fixture. Do not rerun overlapping full suites through storage/desktop scripts; run their additional targeted subprocess/native checks directly where necessary.

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
