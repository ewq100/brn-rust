# Drafts and immutable revisions implementation plan

> Historical record: branch names, commands, approvals and results below describe the recorded task, not new instructions or current authorization. See [current status](../../../status.md) for the present checkout and remaining gaps.

> **For agentic workers:** Use `superpowers:executing-plans` for the Sol implementation, with Astra architecture/review and Luna narrow checks as explicitly requested. Steps use checkboxes for tracking.

**Goal:** Complete the deferred native editor observations, then let a user create/edit a draft, persist its working copy, make immutable checkpoints, reopen and compare revisions, and retain grounded AI answers as separate candidates.

**Architecture:** Add dedicated draft records to the existing SQLite authority and expose them through the existing serialized workflow worker. Keep mutable working text separate from immutable revisions; use draft identity, expected base revision and generation to reject stale writes. The native Markdown editor presents read-only revision review and `similar` diffs without replacing current user edits.

**Tech stack:** Rust 1.98.1, existing rusqlite/UUID/SHA-256/serde dependencies, GPUI Kit 0.6.6, `similar` 2.7 already qualified by the editor trial.

**Spec:** The design below implements chunk 11 of [roadmap.md](../early-checkpoints/roadmap-history.md) and the [approved architecture](../../../architecture/decisions/2026-09-28-architecture-baseline.md), continuing `main` at `5d48737`. Read [status.md](../early-checkpoints/status-history.md), [desktop-usability-evidence.md](../desktop-usability/evidence.md) and [editor trial evidence](../../../../experiments/editor-trial/EVIDENCE.md) with it.

## Authorization and scope

The user explicitly requested architecture/review by Astra, implementation by Sol, narrow checks by Luna, native editor checks before implementation, and autonomous routine decisions through verification, review, commit and feature-branch push. This document records those decisions for review; it does not introduce another permission round. Do not merge or release. The controller completes and records the native prerequisite before dispatching implementation.

This is a single-user, local-first writing slice. Production anchored comments, comment batches, model-driven rewriting, candidate application/restore, approval/publication, graph work and release packaging are outside scope. The existing editor experiment can qualify selection/comment behavior without promoting its snapshot/anchor model into production.

## Design

### Storage and identity

Use schema version 4 with `drafts` and `draft_revisions`, separate from imported `sources`/`versions`. Draft text does not enter retrieval or inherit source approval. Every new draft gets a root checkpoint containing its initial exact text; its working copy starts at generation zero and that root is its base. Draft title must contain non-whitespace text, and draft text is valid UTF-8 up to 1 MiB. Preserve line endings, trailing newline, Unicode and exact bytes; never normalize them. Enforce the bound in the store and UI.

The public data contract is:

```rust
pub const MAX_DRAFT_BYTES: usize = 1024 * 1024;
pub struct DraftStamp { pub base_revision: Uuid, pub generation: u64 }
pub struct Draft {
    pub id: Uuid, pub title: String, pub stamp: DraftStamp,
    pub text: String, pub sha256: [u8; 32],
}
pub enum RevisionKind { Checkpoint, Candidate }
pub struct DraftRevision {
    pub id: Uuid, pub draft_id: Uuid, pub parent_id: Option<Uuid>,
    pub kind: RevisionKind, pub text: String, pub sha256: [u8; 32],
    pub origin_turn: Option<Uuid>,
}
```

All types support the clone/debug/equality traits needed by worker messages and tests. Persist generations as checked nonnegative SQLite integers; reject overflow instead of wrapping. The generation supplied by a new save must exceed the persisted generation; checkpointing unchanged text may retain the generation while advancing the base revision. Every real user edit, including undo/redo, advances the editor generation. Initial programmatic loading is explicitly distinguished from user editing.

`Store` gains these methods, returning its existing `Result` type:

```rust
create_draft(&mut self, op: Uuid, title: &str, text: &str) -> Result<Draft>
drafts(&self) -> Result<Vec<Draft>>
draft(&self, id: Uuid) -> Result<Option<Draft>>
save_draft(&mut self, op: Uuid, id: Uuid, expected: DraftStamp,
           generation: u64, text: &str) -> Result<Draft>
checkpoint_draft(&mut self, op: Uuid, id: Uuid, expected: DraftStamp,
                 generation: u64, text: &str) -> Result<Draft>
draft_revisions(&self, id: Uuid) -> Result<Vec<DraftRevision>>
draft_revision(&self, id: Uuid) -> Result<Option<DraftRevision>>
candidate_from_turn(&mut self, op: Uuid, id: Uuid, parent: Uuid,
                    turn: Uuid) -> Result<DraftRevision>
```

Save/checkpoint compare the complete expected stamp before mutating. Checkpoint atomically persists the submitted working text and a new immutable revision, then advances the base to that revision. Revisions are insert-only through the API; parent IDs must belong to the same draft. Candidate creation validates a completed saved chat turn with an answer, reads its authoritative bytes, records its operation ID as origin, and inserts a candidate under the explicitly frozen parent. It never changes working text, working generation or base. Partial/interrupted/failed turns are rejected. A candidate may branch from a historical revision; current edits cannot invalidate or be overwritten by it.

Bind each operation UUID to kind plus full payload hash using the existing local-operation machinery. Matching retries return the original committed result, even after a later draft edit; store a result snapshot where necessary rather than reading the now-mutated draft. Conflicting reuse fails. Mutation, immutable revision and operation result commit together. Reads verify stored text/hash and parent/draft consistency. Cover fresh databases and every supported v1/v2/v3 upgrade; retain exact schema/integrity validation and safe rejection of foreign/newer/corrupt files. Do not turn v1/v2 migration into a partial multi-transaction upgrade.

### Workflow and comparison

Put storage implementation in `crates/brn-store/src/drafts.rs`; use a small `crates/brn-workflow/src/drafts.rs` for Workspace wrappers and comparison. Existing Workspace owns the store; no view accesses SQLite. Mirror store methods through Workspace using the same argument types and the workflow's `Result<T>`. Add `compare_draft_revisions(&self, draft: Uuid, before: Uuid, after: Uuid) -> Result<String>`: both revisions must belong to the supplied draft, exact-equal bytes produce a clear no-change result, otherwise use `similar::TextDiff::from_lines().unified_diff()` with revision-ID headers. Preserve newline-only differences and the library's missing-final-newline marker.

Extend `worker::Action` and `Outcome` with explicit draft list/open/create/save/checkpoint/revision review/compare/candidate actions and results. Save/checkpoint actions carry operation UUID, draft ID, expected stamp, submitted editor generation and text. Open and review results carry their selected draft/revision IDs; responses never assume that the view selection is unchanged. Keep existing terminal queue and serialized store ownership. Local persistence returns a terminal result after commit; it must not be dropped because a cancellation request arrived after commit.

AI integration is deliberately concrete and bounded: a completed saved answer has an explicit **Save as candidate** action targeting an existing draft and its selected parent revision. It reuses stored provenance and makes no additional provider call. There is no implicit AI-to-editor path and no candidate adoption action in this chunk.

### Native editor state and safety

Add a Drafts page with a draft list, new-draft title input, editable Markdown working copy, **Save working copy**, **Save checkpoint**, visible saved/unsaved status, revision history, read-only revision content and two revision selections for comparison. Reuse the proven scrolling layout, compact row labels and fixed editor size. Full content and diagnostics belong in scrolling bodies. The candidate action may live beside the selected completed saved answer with explicit target draft/parent labels.

Keep the persisted `DraftStamp` and last acknowledged bytes separately from the live editor generation/text. A successful save updates the persisted stamp for the matching selected draft. It clears dirty state only if the editor still matches the submitted generation/text; otherwise newer text remains visible and unsaved and the next save uses the newly acknowledged stamp. A stale completion must never call `set_value` on the live editor or restore an old selection. Checkpoint completion follows the same rule even though it advances the base. An unrelated refresh cannot reload an open dirty draft.

Unsaved changes block draft switching, new-draft creation that would replace the editor, and normal window close, with visible guidance to save first or explicitly discard. A discard action reloads only the acknowledged working copy and requires an explicit UI decision. Allow navigation to other pages without destroying the editor. While the worker is busy, users may keep typing but local actions remain disabled with the existing progress status; a dirty close is still blocked. Closing during a submitted save is blocked until its terminal acknowledgement is handled.

GPUI 0.6.6's bundled `Window::on_window_should_close` supports veto; `on_app_quit` explicitly cannot veto. Register close protection at the former and preserve existing provider shutdown in the latter. Define an application-owned Quit action with `gpui_kit::actions!`, bind Cmd-Q, and expose that action in the application menu; its handler uses the same dirty/pending-save guard before calling `quit`. Verify it works while the editor owns focus. Ordinary keyboard/menu quit must not silently discard edits. OS termination paths that bypass the action, like force termination, do not imply unsaved text was persisted; report their observed limits. Do not attempt to implement a veto from the non-cancellable shutdown observer or label a draft saved before commit.

## Global constraints

- Rust 1.98.1; GPUI Kit `=0.6.6`; reuse `similar` 2.7 and existing dependencies.
- Exact UTF-8 text; maximum draft content `1024 * 1024` bytes; opaque UUID identity.
- Mutable working text and immutable checkpoint/candidate records stay separate.
- Draft/base-revision/generation checks protect user edits, including undo/redo.
- SQLite remains authoritative and the workflow worker its sole native owner.
- No original-vault migration, unsolicited live call, merge, release or credential changes.

## Review focus

- Save acknowledgement after more typing: acknowledge the saved snapshot without clearing/replacing later edits (Task 3 state tests).
- Old or conflicting operation retry after subsequent edits: return the original result or reject conflict; never reapply stale bytes (Task 1 tests).
- Checkpoint/candidate with another draft's parent or incomplete answer: reject atomically (Tasks 1–2 tests).
- Unicode, empty text, CRLF, final-newline-only change and over-limit text: preserve valid bytes, show accurate diff, reject oversized saves without damaging old work (Tasks 1–2 tests).
- Dirty draft switch/close, quit during save, and save error: keep buffer recoverable and never claim persistence before acknowledgement (Task 3 tests/native checks).

## Sequenced implementation tasks

### Task 1: Durable draft authority and schema v4

**Files:** create `crates/brn-store/src/drafts.rs`, `crates/brn-store/tests/drafts.rs`; modify `crates/brn-store/src/lib.rs` and migration tests there.

**Interfaces:** produce the store types/methods specified above. Existing operation records, `encode_args`, SHA-256 and transaction helpers remain the foundation; extend reserved local-operation kinds.

- [x] Write failing tests `draft_working_copy_and_checkpoints_survive_reopen`, `stale_stamp_is_rejected_without_changes`, `retry_returns_original_result_after_newer_save`, `conflicting_operation_reuse_is_rejected`, and `candidate_preserves_working_copy_and_requires_completed_turn`. Assert exact text/hash, root/parent IDs, preserved previous checkpoint bytes, generation/base state and unchanged working text for candidate creation.
- [x] Add boundary tests for empty content, Unicode/CRLF/final newline, wrong-draft parent, overflow, oversized save and failure rollback. Assert no new revision or operation result appears on failed writes. Add populated v3 migration preservation and v3 injected rollback tests; retain v1/v2 migration coverage.
- [x] Run `cargo test -p brn-store --locked --offline` to observe the missing-feature failures before implementing.
- [x] Implement schema/migrations and transactional API. Use persisted result snapshots for mutable-return idempotency. Keep exact schema validation in sync with all migration routes.
- [x] Run `cargo test -p brn-store --locked --offline`; all storage and new draft tests pass. Have the controller review this boundary before native wiring.

### Task 2: Workflow, worker and revision comparison

**Files:** create `crates/brn-workflow/src/drafts.rs`, `crates/brn-workflow/tests/drafts.rs`; modify workflow `src/lib.rs`, `src/worker.rs`, `Cargo.toml`, and root `Cargo.lock`.

**Interfaces:** consume Task 1 types/methods; produce Workspace wrappers and `compare_draft_revisions` as above, plus correlated worker actions/outcomes.

- [x] Write failing tests `revision_diff_preserves_newline_only_changes`, `comparison_rejects_cross_draft_revisions`, `worker_saves_checkpoints_and_reopens_without_provider`, and `candidate_is_separate_from_concurrent_user_edits`. The candidate test advances the working copy after freezing its parent, then asserts both saved edits and candidate provenance survive reopening.
- [x] Run `cargo test -p brn-workflow --locked --offline` and observe the missing behavior.
- [x] Add `similar = "2.7"`, resolve the lockfile using cached dependencies, implement the wrappers/diff and typed worker messages. Return operation/draft/stamp/generation information needed by the UI; never replace editor text in worker code.
- [x] Run `cargo test -p brn-workflow --locked --offline`; existing retrieval/provider behavior and all new workflow/worker cases pass without live provider access.

### Task 3: Native draft editing and safe review

**Files:** create focused `crates/brn-desktop/src/drafts.rs` for editor-state logic; modify `crates/brn-desktop/src/native.rs` and module declarations as needed. Reuse pinned GPUI APIs rather than adopting the experimental full-buffer history model.

**Interfaces:** consume Task 2 worker messages. Produce a pure editor-state helper for persisted stamp, generation, dirty state and pending submitted snapshot, with native view code owning the GPUI entity only.

- [x] Write pure state tests for `save_ack_preserves_later_edits`, `checkpoint_ack_advances_base_without_replacing_newer_text`, `unrelated_draft_result_is_ignored`, `save_failure_keeps_dirty_text`, `undo_advances_generation`, and dirty switch/close/pending-save guards. Assert next-save expected stamp after an older successful acknowledgement.
- [x] Run the focused native-UI test target and observe missing behavior before implementing the state helper and UI.
- [x] Implement Drafts page, explicit working-copy/checkpoint saves, read-only revision view, two-revision comparison and completed-answer candidate creation. Clearly label candidate origin, target and parent. Never add implicit adoption or revision restore.
- [x] Register normal close protection and an application-owned Cmd-Q/menu Quit action sharing the dirty/pending-save guard; verify editor focus does not bypass it. Preserve existing shutdown/cancellation semantics. Make dirty/pending/saved/error feedback visible and keep controls reachable at 800×600.
- [x] Run native build, tests and Clippy with the same pinned/offline shared-target setup as existing usability evidence. The controller exercises real typing, clipboard, undo/redo, a save with further typing, revision inspection, diff, unsaved switch/close protection, restart, and candidate preservation on disposable data.

### Task 4: Final evidence, review and branch publication

**Files:** create `docs/work/completed/draft-revisions/evidence.md`; update `docs/work/completed/early-checkpoints/status-history.md`, `docs/work/completed/early-checkpoints/roadmap-history.md`, and relevant launch/use instructions. Native editor prerequisite evidence is owned by the controller.

- [x] Run `bash scripts/verify-end-to-end.sh`, store/workflow draft tests, `cargo +1.98.1 test --manifest-path experiments/codex-app-server/Cargo.toml --locked --offline`, and the existing editor trial tests. No live call is needed for regression checks.
- [x] Run native desktop build/test/all-target Clippy with `--features native-retrieval --locked --offline`, launcher regressions and `git diff --check`. Re-run only affected checks after corrections.
- [x] Exercise a separate process restart after acknowledged working-copy and checkpoint writes; assert working text, both immutable checkpoints and a candidate reopen exactly. Record command/native observations and remaining limits, distinguishing agent observation from user acceptance.
- [x] Obtain independent Astra review of storage transactions, migration routes, idempotency, stale-result safety and UI lifecycle. Sol resolves material findings; Luna checks bounded commands/docs as directed.
- Publication: commit the reviewed implementation and evidence on `feature/draft-revisions`, push that branch, and verify its remote SHA in the final task record. Do not merge or release.
