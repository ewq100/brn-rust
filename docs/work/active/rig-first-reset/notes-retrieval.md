# Current Markdown Notes and SQLite Retrieval Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline execution, or superpowers:subagent-driven-development when explicitly selected. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver basic recoverable Markdown editing and current-only lexical/semantic/hybrid search through shared desktop/headless workflows.

**Architecture:** Store persists registry/permissions/buffers/intents; workflow validates real files and controls saves. Retrieval receives validated snapshots and eligible-version keys, builds disposable SQLite generations and preserves existing passage/RRF contracts.

**Tech Stack:** Rust `1.98.1`, rusqlite `0.40.2`, SHA-256, UUID, FTS5, qualified sqlite-vec/Rig embedding adapter, FastEmbed `7.1.0` when the selected graph requires it, GPUI-kit `0.6.6`.

**Spec:** [Approved reset](../../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md); [master plan](plan.md).

## Global constraints

- Inherit master global constraints, dependencies and commit trailer.
- Exact UTF-8/BOM/line endings/frontmatter; maximum note/submission is `1_048_576` bytes, not characters.
- Markdown authority, separate local buffers/candidates; opening is not search approval.
- Basic atomic replacement only: no NSFileCoordinator/NSFilePresenter/RENAME_SWAP requirement or lossless simultaneous-editing claim.
- Preserve exclusive data-directory ownership and serialize original-path saves per note. Unknown outcomes are not replayed.
- Ordinary tools cannot retrieve archive/history/recovery/old imports as current knowledge.
- No model acquisition, original-vault mutation or live provider call during deterministic checks.

---

## N1: Registry, editing buffers and replay-safe save records

**Prerequisite:** Q1's dependency choices; no live model required.

**Files:**
- Create: `crates/brn-store/src/notes.rs`, `crates/brn-store/tests/notes.rs`.
- Modify: `crates/brn-store/src/lib.rs` migration/open/schema validation and exports, `tests/storage.rs`, `README.md`.

**Consumes:** Existing `Store`, `OperationStatus`, transaction/operation payload binding, hash validation and exact-byte source/version records.

**Produces:** Schema `6`; these store-owned types, re-exported by workflow:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NoteLifecycle { Active, Archived }
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NoteStamp {
    pub file_state: uuid::Uuid,
    pub generation: u64,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaultRecord {
    pub id: uuid::Uuid,
    pub root: std::path::PathBuf,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoteRecord {
    pub id: uuid::Uuid,
    pub vault_id: uuid::Uuid,
    pub relative_path: std::path::PathBuf,
    pub stamp: NoteStamp,
    pub lifecycle: NoteLifecycle,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoteSubmission {
    pub operation_id: uuid::Uuid,
    pub note_id: uuid::Uuid,
    pub expected: NoteStamp,
    pub generation: u64,
    pub text: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SaveOutcome { Applied, Noop, NotApplied, Uncertain }
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SaveReceipt {
    pub operation_id: uuid::Uuid,
    pub note_id: uuid::Uuid,
    pub submitted_generation: u64,
    pub outcome: SaveOutcome,
    pub current_file_state: Option<uuid::Uuid>,
}
```

Private store records contain the observed filesystem identity as opaque bytes, exact hash and observed state ID, baseline/working bytes, pending phase and prepared artifact identity. Do not expose Unix inode/device structures to consumers.

`NoteStamp` is the editor baseline file-state plus last acknowledged buffer generation; `NoteView.disk_state` is the separately fresh observed file-state. Recovery can advance the acknowledged generation without changing the file baseline. Save accepts the already-recovered generation or a newer submission, but rejects older/mismatched generation content. Approval binds the fresh observation, not an outdated buffer stamp.

Planned internal store methods: `register_vault(op: Uuid, root: &Path) -> Result<VaultRecord>`, `note(id: Uuid) -> Result<Option<NoteRecord>>`, `notes(vault: Uuid) -> Result<Vec<NoteRecord>>`, `note_save_receipt(op: Uuid) -> Result<Option<SaveReceipt>>`. Workflow-facing registration/buffer/save transactions consume the types above plus private validated file observations; keep mutation implementations inside this module and reuse `bind_operation`.

- [ ] **1. Red test durable vault identity:** use a temp directory and `Store::open`; register with one operation, reopen and repeat it, then assert same `VaultRecord.id`. Reusing the operation with a different root must return `Error::OperationConflict`, not another vault.

```rust
#[test]
fn vault_registration_replays_without_new_identity() {
    let data = tempfile::tempdir().unwrap();
    let vault = tempfile::tempdir().unwrap();
    let op = uuid::Uuid::new_v4();
    let (mut store, _) = brn_store::Store::open(data.path()).unwrap();
    let first = store.register_vault(op, vault.path()).unwrap();
    drop(store);
    let (mut store, _) = brn_store::Store::open(data.path()).unwrap();
    assert_eq!(store.register_vault(op, vault.path()).unwrap().id, first.id);
}
```

Run `cargo test -p brn-store --locked --test notes vault_registration_replays_without_new_identity`; expect missing interface.

- [ ] **2. Add V6 tables/migration:** `vaults`, `notes`, `note_observations`, `note_buffers`, `note_save_intents`, `note_save_receipts`. Enforce UUID ownership/FKs, `UNIQUE(vault_id, relative_path)`, lifecycle values, nonnegative generations, 32-byte hashes and operation ID/result uniqueness. Store root/path losslessly through validated serialization; reject unsupported path encoding rather than lossy conversion.

Track observations separately from immutable snapshots. Approval records bind a file-state/hash and optional source/version snapshot IDs. Ordinary observation/buffer/save is not an immutable full-text revision per keystroke. Pending save intent includes exact baseline/submission, expected state, submitted generation, prepared artifact identity, phase and known outcome.

- [ ] **3. Extend creation/migration/schema checks together:** new databases create V1-V6; upgrades from each supported old schema end at V6 without resetting originals/drafts/comments. Check `is_local_kind`, existing saved-result validation and startup interrupted operation classification. A generic interrupted operation does not settle a save's file effect.

- [ ] **4. Red/green store cases:** wrong-note state, negative/overflow generation, conflicting operation payload, identical completed replay, interrupted pending intent, corrupted hash, newer schema and migration interruption. Include exact UTF-8 bytes in test fixtures. Retained receipts survive recovery-payload cleanup; pending inputs never enter that cleanup.

Implement recovery compaction transactionally: retain one rolling buffer and the latest completed save's baseline/submission pair per note; remove older completed bulky pairs only after the new pair/receipt is durable. Retain compact payload-bound receipts indefinitely and unresolved inputs without expiry. Test multiple saves plus interrupted new intent, then reopen and verify exactly this retained set.

- [ ] **5. Green/commit:** `cargo test -p brn-store --locked --test notes --test storage`; `cargo check --workspace --locked`. Commit as `feat(store): add current note registry and save journal`.

## N2: Basic file saves, conflict recovery and shared note interface

**Prerequisite:** N1.

**Files:**
- Create: `crates/brn-workflow/src/notes/{mod.rs,files.rs,macos.rs,save.rs,eligibility.rs}`.
- Create: `crates/brn-workflow/tests/{notes.rs,note_recovery.rs}`.
- Modify: workflow `src/lib.rs`, `src/error.rs`, `Cargo.toml`, `README.md`.

**Consumes:** N1 types and transactional store methods; current `MAX_IMPORT_BYTES`, hash and cancellation conventions.

**Produces:** Workflow-owned `NoteView` and these methods on `Workspace`:

```rust
pub enum NoteState { Ready, Changed, Missing, Uncertain }
pub struct NoteView {
    pub note: brn_store::NoteRecord,
    pub disk_state: Option<uuid::Uuid>,
    pub disk: Option<String>,
    pub working: String,
    pub recovered_generation: Option<u64>,
    pub state: NoteState,
}
pub struct CopyReceipt {
    pub operation_id: uuid::Uuid,
    pub source_note_id: uuid::Uuid,
    pub new_note: brn_store::NoteRecord,
    pub submitted_generation: u64,
}
// Each method returns crate::Result<T>.
// register_vault(&mut self, op: Uuid, root: &Path) -> Result<VaultRecord>
// open_note(&mut self, op: Uuid, vault: Uuid, relative: &Path) -> Result<NoteView>
// note_state(&mut self, note: Uuid) -> Result<NoteView>
// recover_note_buffer(&mut self, request: &NoteSubmission) -> Result<NoteView>
// save_note(&mut self, request: &NoteSubmission) -> Result<SaveReceipt>
// note_save_receipt(&self, op: Uuid) -> Result<Option<SaveReceipt>>
// reload_note(&mut self, op: Uuid, note: Uuid, expected: NoteStamp,
//             observed: Uuid, discard_confirmed: bool) -> Result<NoteView>
// copy_note(&mut self, request: &NoteSubmission, relative: &Path) -> Result<CopyReceipt>
// reconcile_note(&mut self, note: Uuid) -> Result<NoteView>
```

Add typed `ErrorKind::{NoteConflict,NoteUnavailable,SaveUncertain}`; propagate them through worker/CLI in N4. Mutation errors are errors, not a success-shaped default receipt.

- [ ] **1. Red exact-save/replay test:** all fixtures are outside the data directory:

```rust
#[test]
fn explicit_save_preserves_exact_bytes_and_replays() {
    let data = tempfile::tempdir().unwrap();
    let vault = tempfile::tempdir().unwrap();
    std::fs::write(vault.path().join("plan.md"), "\u{feff}start\r\n").unwrap();
    let mut w = brn_workflow::Workspace::open(
        data.path(), brn_workflow::Config::default()
    ).unwrap();
    let v = w.register_vault(uuid::Uuid::new_v4(), vault.path()).unwrap();
    let note = w.open_note(uuid::Uuid::new_v4(), v.id, std::path::Path::new("plan.md")).unwrap();
    let request = brn_workflow::NoteSubmission {
        operation_id: uuid::Uuid::new_v4(),
        note_id: note.note.id,
        expected: note.note.stamp,
        generation: note.note.stamp.generation + 1,
        text: "\u{feff}edited\r\n".into(),
    };
    let saved = w.save_note(&request).unwrap();
    assert_eq!(saved.outcome, brn_workflow::SaveOutcome::Applied);
    assert_eq!(std::fs::read(vault.path().join("plan.md")).unwrap(), request.text.as_bytes());
    assert_eq!(w.save_note(&request).unwrap().operation_id, saved.operation_id);
}
```

Run `cargo test -p brn-workflow --locked --test notes explicit_save_preserves_exact_bytes_and_replays`; expect missing note methods.

- [ ] **2. Implement bounded/private file adapter:** validated vault root, contained relative paths, bounded regular single-link reads, checked identity/hash and no-follow access. Reject data directories within vaults, escaped/ambiguous paths, symlink roots/targets and unsupported destinations. Normal raw path strings are not capabilities.

Keep macOS filesystem identity/attribute operations in `macos.rs`, not UI/state types. Support same-directory atomic replacement and exclusive noncolliding copy creation; if those aren't available, return a typed unavailable error. No NSFileCoordinator/RENAME_SWAP or ordinary in-place fallback.

- [ ] **3. Implement save sequence with injected boundaries:** bind and commit intent/recovery; suspend current eligibility; prepare/write/sync temporary file; persist prepared identity; revalidate baseline; atomic replace; sync/verify; commit receipt/new state/buffer acknowledgement. Preserve permissions and supported ACL/xattrs; inability to preserve them fails before replacement or becomes uncertain after possible replacement. A unchanged submission still validates but changes no timestamps/identity.

Temporary names are operation-owned, non-`.md`, and exclusive. Remove only artifacts whose recorded type/identity match; unknown artifacts are not cleanup targets. One original-path save per note; later generations survive receipt updates.

- [ ] **4. Red conflict case:** after opening, externally replace bytes with `"external\n"`; `save_note` must return `ErrorKind::NoteConflict`, disk must remain `"external\n"` and `note_state().working` must retain the submitted edits. Add missing/renamed/replaced identity cases, empty file, exact 1 MiB success, 1 MiB + 1 failure, a multibyte boundary and occupied copy destination.

- [ ] **5. Implement reload/copy/reconciliation:** comparison exposes original working baseline, recoverable edits and fresh disk observation. Reload requires matching editor/disk tokens plus explicit discard. Copy uses exclusive installation and its own `CopyReceipt`/new identity; it never acknowledges the original buffer, overwrites or implicitly approves. Persist the neutral copy result in N1's operation result journal. On restart, inspect journal phase and prepared/destination identity; without proof retain uncertain state and never replay the mutation. Even matching hashes alone do not establish execution provenance.

- [ ] **6. Test interruption/races through a private test seam:** stop child processes after intent, staging, final precheck, replacement, verification and before receipt. Assert durable inputs, no blind second replacement, explicit uncertainty and cleanup exclusions. Inject an external write/delete after precheck: document that replacement can overwrite/recreate; don't assert prevention. Verify subsequent ambiguous state is not announced as a safe concurrent-save success.

- [ ] **7. Green/commit:** `cargo test -p brn-store -p brn-workflow --locked`; `cargo check --workspace --locked`. Commit as `feat(workflow): add basic recoverable Markdown saving`.

## N3: Current-only snapshots and FTS5/BM25

**Prerequisite:** N1/N2 and Q1's SQLite choice.

**Files:**
- Modify: `crates/brn-workflow/src/notes/eligibility.rs`, `src/lib.rs`, `tests/flow.rs`.
- Create: `crates/brn-workflow/tests/note_evidence.rs`.
- Create: `crates/brn-retrieval/src/sqlite.rs`, `tests/fts.rs`.
- Modify: retrieval `src/lib.rs`, `Cargo.toml`, `README.md`, root `Cargo.lock`.

**Consumes:** N2 fresh note observation and existing `Document`, `Evidence`, `Profile`, chunker and RRF.

**Produces:** These workflow and retrieval interfaces:

```rust
// On Workspace:
// approve_note(&mut self, op: Uuid, note: Uuid, observed: Uuid) -> Result<SourceDocument>
// withdraw_note_approval(&mut self, op: Uuid, note: Uuid, observed: Uuid) -> Result<NoteView>
// set_note_lifecycle(&mut self, op: Uuid, note: Uuid,
//                    observed: Uuid, lifecycle: NoteLifecycle) -> Result<NoteView>
// current_documents(&mut self, vault: Uuid) -> Result<Vec<SourceDocument>>
// current_evidence(&mut self, evidence: &Evidence) -> Result<()>

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EligibleVersion {
    pub source_id: String,
    pub version_id: String,
    pub source_hash: String,
}
// On Index:
// search_filtered(&mut self, query: &str, profile: Profile, limit: usize,
//                 eligible: &[EligibleVersion]) -> Result<Vec<Evidence>>
```

The original `Index::search` remains a retrieval-only convenience over its supplied documents; production workflow uses `search_filtered` and authoritative validation. Eligibility is not inferred from the index.

- [ ] **1. Red lexical test** using existing `Index::build`/`Document` helpers:

```rust
#[test]
fn lexical_search_matches_tokens_not_substrings() {
    let root = tempfile::tempdir().unwrap();
    let docs: Vec<_> = [("token", "red blue"), ("substring", "reddish blueprint")]
        .into_iter().map(|(id, text)| brn_retrieval::Document {
            source_id: id.into(), version_id: "v1".into(), title: id.into(),
            text: text.into(), source_hash: brn_retrieval::hash(text.as_bytes()),
        }).collect();
    let mut index = brn_retrieval::Index::build(
        &root.path().join("generation"), &docs, None,
        &std::sync::atomic::AtomicBool::new(false), |_| {}
    ).unwrap();
    let hits = index.search("red", brn_retrieval::Profile::Keyword, 10).unwrap();
    assert_eq!(hits.iter().map(|h| h.source_id.as_str()).collect::<Vec<_>>(), vec!["token"]);
}
```

Run `cargo test -p brn-retrieval --locked --test fts lexical_search_matches_tokens_not_substrings`; expect current substring implementation to fail.

- [ ] **2. Add SQLite generation schema:**

```sql
CREATE TABLE passages (
  passage_id TEXT PRIMARY KEY, source_id TEXT NOT NULL,
  version_id TEXT NOT NULL, source_hash TEXT NOT NULL,
  start_byte INTEGER NOT NULL, end_byte INTEGER NOT NULL, quote TEXT NOT NULL
);
CREATE VIRTUAL TABLE passages_fts USING fts5(
  quote, content='passages', content_rowid='rowid', tokenize='unicode61'
);
```

Populate passage rows and FTS rows in one transaction, reusing the exact chunk ranges/IDs. Confirm bundled FTS5 availability. Search via bound `MATCH` input assembled from quoted Unicode terms with OR semantics, never raw SQL; parameterize eligible source/version/hash keys before ranking. Negate BM25 for existing higher-is-better scores and set `score_kind = "bm25_negated"`. Stable passage-ID tie-break.

Keep published generations immutable: finalize/checkpoint/close SQLite, hash artifacts, then publish COMPLETE/active pointer only after successful validation. Corrupt/incomplete generations never become current. Update manifest format to distinguish SQLite generations; old derived generations report rebuild-needed without rewriting authoritative storage.

- [ ] **3. Wire current approval:** create immutable source/version evidence only on explicit approval, not each save. Map registered note to its snapshot source; suppress associated legacy file imports as independent current sources. Unregistered `.txt`/historical imports remain stored but are not normal current-vault AI knowledge. Document the intentional search/import behavior change and change the deterministic `brn-flow` fixture accordingly.

`withdraw_note_approval` binds the current observation and sets the associated snapshot to existing `Approval::Withdrawn`; eligibility checks both exact note permission and source approval. Existing source-level `set_approval` cannot bypass registration/currentness or revive a stale note. Reapproval rebuilds a snapshot only when bytes/version changed.

Archive is explicit registry lifecycle/exclusion, not an automatic move or a filename heuristic. `set_note_lifecycle(...Archived)` immediately revokes ordinary eligibility without mutating historical bytes. Restore requires fresh explicit approval. Physical archive moves/history browsing are not implicitly added.

- [ ] **4. Add currentness tests:** changed file, missing file, unresolved save, archive and withdrawn approval cannot appear in keyword/semantic/hybrid evidence or reads; old imported copies cannot resurrect approval. Check before generation publication, before returning results, before provider dispatch and at completion. A filtered `limit=1` must return the best eligible passage even if many excluded passages rank higher.

- [ ] **5. Green/commit:** `cargo test -p brn-retrieval -p brn-workflow --locked`; `cargo check --workspace --locked`. Commit as `feat(retrieval): add FTS5 and current-note eligibility`.

## N4: Desktop and headless note editing parity

**Prerequisite:** N1-N3.

**Files:**
- Create: `crates/brn/src/cli/notes.rs`, `crates/brn/tests/cli_notes.rs`.
- Create: `crates/brn-desktop/src/notes.rs`.
- Modify: CLI `mod.rs`, `documents.rs`, `error.rs`, README; workflow `worker.rs`, `main.rs`.
- Modify: desktop `main.rs`, `native/mod.rs`, `native/shell/{centre.rs,vault_rail.rs,header.rs}`, README.
- Modify: `scripts/verify-end-to-end.sh`, `scripts/verify-desktop-shell.sh`.

**Consumes:** N2 note methods/types and N3 approval/lifecycle/current documents.

**Produces:** Shared worker actions/outcomes and CLI commands:

```text
brn vaults register ROOT --operation UUID
brn notes open RELATIVE_PATH --vault UUID --operation UUID
brn notes show NOTE_ID
brn notes recover NOTE_ID --file-state UUID --expected-generation N --generation N --text-file PATH --operation UUID
brn notes save NOTE_ID --file-state UUID --expected-generation N --generation N --text-file PATH --operation UUID
brn notes reload NOTE_ID --file-state UUID --expected-generation N --observed-state UUID --discard --operation UUID
brn notes copy NOTE_ID --to RELATIVE_PATH --file-state UUID --expected-generation N --generation N --text-file PATH --operation UUID
brn notes reconcile NOTE_ID
brn notes approve NOTE_ID --observed-state UUID --operation UUID
brn notes withdraw NOTE_ID --observed-state UUID --operation UUID
brn notes set-state NOTE_ID --observed-state UUID --state active|archived --operation UUID
```

All commands also require the existing `--data-dir`; `--json` retains envelope version `1`. Contents come from bounded UTF-8 files, not command-line secrets or newline normalization. Missing preconditions/invalid options fail before mutations.

- [ ] **1. Red CLI process test:** create disposable `data`/`vault` and `plan.md`; invoke `vaults register` then `notes open`, parse returned IDs/tokens, invoke `notes save` with a new text-file and assert exact disk bytes. Repeat operation and assert no new file mutation. Add JSON error cases for external edit and unknown flags. Use the existing `env!("CARGO_BIN_EXE_brn")` subprocess pattern.

- [ ] **2. Add worker actions/outcomes:** register/open/state/recover/save/reload/copy/reconcile/approve/lifecycle variants call the shared methods only. Carry request operation/note/generation in terminal outcomes. Replace note errors flattened into strings with a worker error DTO carrying `ErrorKind`, message and relevant IDs; update every exhaustive match in the same commit.

- [ ] **3. Add a pure editor-state reducer** in desktop `notes.rs`: `NoteEditor::from_view(view: NoteView)`, `edit(text: String)`, `submission(op: Uuid) -> NoteSubmission`, `acknowledge(receipt: &SaveReceipt) -> Result<(), String>`. It retains dirty later text; receipt baseline/generation changes never call a blanket set-value over newer content.

Red reducer test:

```rust
use brn_workflow::{NoteLifecycle, NoteRecord, NoteStamp, NoteState, NoteView, SaveOutcome, SaveReceipt};

let initial_state = uuid::Uuid::new_v4();
let op = uuid::Uuid::new_v4();
let view = NoteView {
    note: NoteRecord {
        id: uuid::Uuid::new_v4(), vault_id: uuid::Uuid::new_v4(),
        relative_path: "plan.md".into(),
        stamp: NoteStamp { file_state: initial_state, generation: 0 },
        lifecycle: NoteLifecycle::Active,
    },
    disk_state: Some(initial_state), disk: Some("baseline".into()),
    working: "baseline".into(), recovered_generation: None, state: NoteState::Ready,
};
let mut editor = NoteEditor::from_view(view);
editor.edit("submitted".into());
let submitted = editor.submission(op);
editor.edit("typed after save began".into());
editor.acknowledge(&SaveReceipt {
    operation_id: op, note_id: submitted.note_id,
    submitted_generation: submitted.generation,
    outcome: SaveOutcome::Applied, current_file_state: Some(uuid::Uuid::new_v4()),
}).unwrap();
assert_eq!(editor.submission(uuid::Uuid::new_v4()).text, "typed after save began");
```

Put this body in `#[test] fn late_save_ack_preserves_newer_typing()` inside the pure notes module's test module; no native window required. Also test wrong operation/note receipts and copy outcomes never acknowledging the original note. Gate these desktop modules/tests with `native-ui` consistently with the existing optional workflow dependency; the native script below must execute them.

- [ ] **4. Wire native UI:** Choose vault/Open note, explicit Cmd-S, recovery status, conflict comparison, confirmed reload and save-copy. Reuse existing input/editor and draft generation patterns; no file writes on GUI thread. Coalesce recovery after `500 ms` of idle editing; this is scheduling, not durability. Flush before switch/close or request explicit discard. The existing one-item worker queue may delay recovery during AI work: show pending and never claim unacknowledged edits are protected.

- [ ] **5. Wire all current-content surfaces:** vault rail/list/show and AI selection never present historical snapshot bytes as current. Lifecycle controls change registry state; do not claim physical archival moves. Original draft/comment/revision/history views remain explicitly distinct.

- [ ] **6. Green/commit:** `cargo test -p brn -p brn-desktop -p brn-workflow --locked`; `cargo check --workspace --locked`; `bash scripts/verify-desktop-shell.sh --native`. Observe disposable native open/save/external conflict/reload/copy/typing-during-save/restart/close recovery; record unobserved usability/IME/accessibility cases. Commit as `feat(ui): expose shared Markdown note workflows`.

## N5: Local Rig embeddings, sqlite-vec and accepted hybrid search

**Prerequisite:** Q1, N3; separate model-resource authorization for native measurements.

**Files:**
- Create: `crates/brn-retrieval/src/embeddings.rs`, `tests/{sqlite_vec.rs,hybrid.rs}`.
- Modify: retrieval `src/{lib.rs,sqlite.rs,native.rs}`, manifest/README and root lockfile.
- Create: `experiments/retrieval-comparison/{Cargo.toml,Cargo.lock,src/main.rs,src/lance_baseline.rs,README.md}`.
- Modify: `tests/native_smoke.rs`, this task's evidence. Keep old native dependencies until G4 acceptance.

**Consumes:** Existing approved local ONNX MiniLM artifacts/mean pooling/384 dimensions as the comparison baseline, N3 `EligibleVersion`, chunker, SQLite generation and RRF.

**Produces:** Chosen compatible Rig local embedding adapter and sqlite-vec backend; frozen benchmark evidence and explicit G4 acceptance.

- [ ] **1. Red deterministic vector test:** store three known 384-dimensional finite vectors, mark the closest passage ineligible, and assert `search_filtered(...Semantic, 1, eligible)` returns the closest eligible passage, not zero hits or the excluded one. Exercise wrong dimensions, NaN/infinity, malformed extension schema and missing resources as errors. These tests need no model download.

- [ ] **2. Implement the selected integration:** if Q1 qualifies companion crates, use their exact pinned contracts. Otherwise implement Rig's existing `Transport<Local<Embedding>>` around the loaded FastEmbed model, following the inspected `rig-fastembed` transport, not a duplicate provider abstraction. Keep CPU inference off the async event loop; verify cancellation between batches and finite vector count/dimension.

Use local caller-supplied model artifacts, disable automatic acquisition defaults, and fingerprint ONNX/tokenizer/config/pooling/dimension/adapter identity. For vector storage, initialize the selected static sqlite-vec extension for the intended connection, not arbitrary user extension paths; assert successful initialization. Index/vector mappings preserve passage IDs/hashes/ranges and filter eligibility before top-k. If any chosen integration cannot satisfy graph/correctness, stop G4 rather than substitute keyword search.

- [ ] **3. Reuse/test RRF:** retain rank constant `60`, higher-is-better order and passage-ID tie-break. Test overlapping lexical/semantic hits deduplicate by passage ID, excluded hits never contribute, limit `1` is correct, and requesting unavailable semantic/hybrid returns a typed error.

- [ ] **4. Freeze comparison inputs:** synthetic UTF-8 corpus at `100` and `10_000` notes, with up to `50_000` passages; `30` query/relevance cases containing identifiers, paraphrases and exclusion traps. Commit the generator/seed/qrels before measuring. Run old Lance and new SQLite using the same verified local model/artifacts and approved snapshots, on the same named Mac.

The comparison binary prints JSON for recall@10, nDCG@10, p50/p95 lexical/vector/end-to-end query latency, model cold-load time, index build time, peak process RSS, generation disk size and filtered query results. Warm timing performs `5` warmups then `30` measured queries per workload/backend; do not mix cold-load into only one backend. Verify byte-identical provenance for returned passages.

- [ ] **5. Resource/acceptance gates:** missing assets produce an explicit blocked measurement, not a skipped pass. Acquire models only after permission. Require zero eligibility/provenance failures; no unexplained quality/performance regression against the frozen baseline. Any measured trade-off requires explicit recorded acceptance. An unavailable/unqualified Lance baseline cannot be replaced with invented numbers.

- [ ] **6. Green/commit:** `cargo test -p brn-retrieval --locked`; `cargo test -p brn-retrieval --locked --features native`; `cargo check -p brn-desktop --locked --features native-ui,native-retrieval`; run the comparison binary with explicit backend/model/fixture/output paths. Commit as `feat(retrieval): add qualified SQLite semantic and hybrid search`. Deletion of LanceDB remains D1 after accepted G4.
