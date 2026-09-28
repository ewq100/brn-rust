# Persistent anchored comments implementation plan

> **For agentic workers:** Use `superpowers:executing-plans` for Sol implementation, with Astra architecture/review and Luna narrow checks as requested. Steps use checkboxes for tracking. Routine design choices are authorized; no additional approval pause is required.

**Goal:** Select a passage in a working draft, persist a comment with immutable provenance, resolve/reopen it, and conservatively track its location through editing and restart.

**Architecture:** SQLite remains authoritative. Pure UTF-8 anchor logic is shared by storage validation and the native preview; the serialized workflow worker owns all persistent changes. Comment creation atomically checkpoints the captured working text and inserts its comment. Native edit traces preserve the difference between an untouched passage and one deleted then retyped between saves.

**Tech stack:** Rust 1.98.1; GPUI Kit `=0.6.6`; existing rusqlite, UUID, SHA-256, serde and `similar` dependencies. No CRDT or additional runtime.

**Spec:** Chunk 12 in [roadmap.md](roadmap.md), [architecture-checkpoint.md](architecture-checkpoint.md), [status.md](status.md), and chunk 11's [plan](draft-revisions-plan.md)/[evidence](draft-revisions-evidence.md). Base: main `1b6623a`; branch `feature/anchored-comments` in the managed worktree. The editor experiment proves selection/focus and conservative mapping feasibility; its in-memory whole-document-per-comment representation is not the persistence design.

## Global constraints

- Exact UTF-8, opaque UUID identity, half-open byte ranges at character boundaries; no Unicode or newline normalization.
- Draft text remains limited to `1024 * 1024` bytes. A comment body must contain non-whitespace text and is limited to 64 KiB of UTF-8, with exact entered bytes retained.
- Working text, immutable revisions, comment lifecycle and mapping state are separate concepts. `Resolved` lifecycle never means a passage is located; use `Anchored` for location.
- Acknowledgements happen only after commit. Draft identity, operation ID, expected `DraftStamp`, submitted generation/text and worker request identity prevent stale application.
- No batch generation, candidate adoption, model call, approval/publication, original-vault conversion, merge or release. Existing candidate creation remains independent of comments.
- Preserve existing dirty/switch/close/Cmd-Q/menu guards and provider shutdown behavior. Use disposable native workspaces with no provider configured.

## Concrete design

### Immutable capture and comment lifecycle

`Capture selection` copies draft ID, editor generation, exact full text, selected byte range and exact quote before comment-entry focus changes selection. Reject empty/out-of-bounds/non-character-boundary selections. Add is enabled only while draft ID, generation and exact text still match the capture; text changing and then returning does not silently renew an old capture. The user selects again after any intervening edit.

The explicit button is **Save checkpoint + add comment**. It submits the captured text, current acknowledged stamp, generation, range/quote, body and edit trace. In one transaction, storage verifies CAS, saves the working text, creates an ordinary immutable checkpoint parented to the acknowledged base, maps existing comments, and inserts the new comment anchored exactly to that checkpoint. This applies even to a clean buffer; one intentional revision per successful comment creation makes the original identity unambiguous without a new revision kind. The current draft base advances. Never first save and then issue a separate add operation. Failure rolls back the checkpoint, working copy, comment and operation together.

`DraftComment` contains ID, draft ID, immutable original revision ID, original revision SHA-256, original start/end, original quote, body, `CommentStatus::{Open, Resolved}` and monotonic `status_version: u64`. Original metadata/body are immutable in this slice. Resolve/reopen is a separate CAS write using expected status version and explicit desired state; it changes neither text nor anchors. A desired state equal to current state is a valid no-op receipt with unchanged version. All comments remain visible, with explicit status and actions. Do not hide resolved comments or treat deletion as resolution.

### Schema v5 and integrity

Add focused `comments.rs` plus pure `anchors.rs` in `brn-store`. Create:

- `draft_comments`: the fields above; foreign keys to drafts/revisions, checked lifecycle and nonnegative status version/ranges.
- `draft_comment_anchors`: one current mapping per comment, including the authoritative target draft base/generation/hash and location state/range. Current mappings update atomically with every working-copy save/checkpoint.
- `draft_revision_comment_anchors`: immutable `(comment_id, revision_id)` mapping snapshots for checkpoints made at/after the comment exists. The creation checkpoint gets the new comment's original mapping too. These small records refer to revision text already stored; they do not duplicate whole buffers.
- `comment_results`: operation ID, typed result discriminator and JSON receipt. Capture receipts include saved draft, created comment and complete mapping projection for that submitted snapshot; lifecycle receipts include the historical returned comment. Traced-save receipts likewise freeze their returned mapping projection. Never reconstruct a retry response from mutable current rows.

Location is `Anchored { start, end }`, `Deleted`, or `Ambiguous { reason }`; reason uses the closed enum defined below. Non-anchored states have no current byte range. SQL checks must reject invalid tag/range combinations. Rust read validation checks UUIDs, bounds, exact original quote against original revision, revision hash, draft ownership, checked integer conversions, current target stamp/hash and anchored quote equality. Same-draft revision constraints require application validation even where a simple FK exists. Check stored receipt identities, hashes, payload correspondence and immutable references on retry; do not compare historical mutable fields to today's lifecycle or mapping.

Extend exact schema validation, fresh creation, all v1/v2/v3 upgrade routes, and a transactional v4 upgrade. Retain old draft result decoders and pre-v5 draft operation retry semantics. Existing save/checkpoint public entry points must also update current mappings; there must be no writer that leaves anchors at a stale stamp. Reserve new local operation kinds so the general operation API cannot impersonate them.

### Conservative supported edits, with no quote search reattachment

GPUI's observed `InputEvent::Change` supplies complete new text, not an authoritative edit transaction. Each native event therefore derives a conservative replacement from the longest common UTF-8-character prefix and non-overlapping suffix. A multi-region change is one broad replacement and can lose location; `similar` remains a display diff, never an identity oracle.

Expose `TextEdit { start: usize, end: usize, replacement: String }`, `AnchorState`, `derive_edit(before: &str, after: &str) -> Option<TextEdit>`, and `map_anchor(before: &str, after: &str, state: &AnchorState, original_quote: &str, edit: &TextEdit) -> Result<AnchorState>`. The closed ambiguity-reason enum covers touched, duplicate, missing/unsupported, boundary ambiguity, conflicting snapshot and history-limit cases. Validate each edit against the current intermediate text, character boundaries and the 1 MiB intermediate limit; apply with checked byte arithmetic. A recorded edit must equal the canonical derived replacement for its before/after text, so arbitrary narrower caller claims cannot promote an anchor. No text change leaves its mapping unchanged.

Before mapping a changed document, require exactly one occurrence of the original quote in both previous and next text (count overlapping UTF-8-boundary occurrences with string search, stop after two). Duplicate text becomes `Ambiguous(Duplicate)` even if a human could infer the position. Original capture may select one of several identical passages because it names an exact revision/range; the next changed document gets the conservative duplicate rule. Do not globally search for one surviving occurrence and attach it.

For an already anchored range and a unique quote: a replacement wholly before the range shifts it; one wholly after leaves it; revalidate exact quote and bounds. Insertion exactly at start stays outside and shifts the range; insertion exactly at end stays outside without shifting. Insertion strictly inside or any overlapping replacement becomes ambiguous/touched. A deletion with empty replacement covering the complete original current range becomes `Deleted` when the quote is absent afterward; a missing quote otherwise is ambiguous/touched. Classify actual full deletion before the uniqueness gate's missing-quote branch, but do not claim deletion in a duplicate-identity case. A repeated-character boundary whose minimal replacement could equally straddle the selected range is ambiguous: inspect equivalent left/right minimal replacement alignments and require all permitted alignments to give the same untouched range; otherwise reject the mapping. A single arbitrary prefix/suffix alignment must not decide identity.

An unresolved state stays unresolved through subsequent edits, even if its quote later becomes unique. The only automatic recovery is exact complete-document equality with this comment's original immutable revision, or an immutable checkpoint with a stored `Anchored` mapping for the same comment. This proves content equivalence, not undo intent: retyping the entire exact known document has the same result. Validate full text, hash and quote/range; never hash or quote alone. If identical checkpoint text has conflicting anchored ranges, stay ambiguous. Original identity has priority. This gives deterministic undo recovery without treating arbitrary retyping as proof. Undo/redo each advances editor generation. Undo to another unsaved intermediate buffer may remain explicitly unresolved; document this conservative limit instead of inventing durable editor history.

Current mappings are persisted on explicit saves, and checkpoint mapping snapshots survive process restart. Working previews use the same rules and the loaded original/checkpoint recovery references. Restart restores acknowledged text and mapping; it does not restore native keyboard undo history or unsaved edits.

### Validated edit traces and save races

A save cannot map only the acknowledged and final texts: deletion/reinsertion and several distinct edits can happen before Save. `EditTrace` is either `Steps(Vec<TextEdit>)` or `HistoryLost`. `DraftEditor` retains compact replacements since its acknowledged buffer, not a full buffer per keystroke. Limit to 4096 steps and 8 MiB of accumulated replacement bytes; once exceeded use `HistoryLost` for that unsaved interval. A temporarily oversized editor buffer also enters HistoryLost so shrinking back below the limit remains saveable. That fallback persists exact final text and maps all existing comments to explicit history-limit ambiguity, except exact immutable-snapshot recovery. Never silently fall back to endpoint quote matching.

A submitted snapshot freezes the exact trace prefix, text, generation and expected stamp. Further typing appends a separate suffix while the worker runs. Successful acknowledgement installs the returned persisted draft/mappings, drops only the acknowledged prefix, then replays the suffix for current preview. Failure keeps the original trace and text retryable. An old/unrelated result cannot clear the composer, replace text, move selection or install old mappings. For a new comment acknowledged after more typing, map it forward through that same suffix before displaying its current location. A full snapshot return is not permission to overwrite newer live bytes. Freeze composer generation/body too; clear the composer only if it still equals the successful submitted capture/body. Preserve later comment typing.

Storage replays the trace from the CAS-protected old draft and requires it to reconstruct the submitted text exactly; reject malformed traces atomically. `HistoryLost` accepts final text under the same size/CAS rules but grants no inferred identity. Legacy callers without a trace use one conservative canonical endpoint replacement; the native UI always supplies its trace. Trace and fallback tag are part of operation payload binding. A no-text-change operation with a nonempty trace still replays its intermediate states; exact snapshot recovery remains the only resurrection exception.

Programmatic editor loads/discard must synchronize pure state and GPUI without recording spurious user changes. Explicit Discard restores acknowledged mappings and clears traces/capture; normal page navigation preserves them. Closing or switching during a pending capture/save remains blocked. Nonblank unsaved comment text also participates in close/switch protection, with an explicit Discard comment action; page navigation retains it. Comment lifecycle requests may run while text is dirty; they apply only lifecycle fields on completion, preserving local mapped ranges and edit traces.

### Interfaces and ownership

Public types live in `brn-store` and are re-exported through workflow as existing draft types are. Use owned request/receipt structs to avoid oversized positional APIs:

- `CommentCapture { op, draft_id, expected: DraftStamp, generation, text, edits: EditTrace, range: std::ops::Range<usize>, quote: String, body: String }`.
- `DraftCommentView { comment: DraftComment, anchor: AnchorState }` and `DraftComments { draft: Draft, comments: Vec<DraftCommentView> }` for a matching persisted snapshot.
- `CommentCreated { op, submitted_generation, saved: DraftComments, comment_id }`.
- `CommentStatusChange { op, draft_id, comment_id, expected_status_version, status }` and `CommentStatusChanged { op: Uuid, comment: DraftComment }`.
- `DraftWriteWithComments { op, draft_id, expected, generation, text, edits, checkpoint: bool }` returning `DraftComments` in a frozen operation receipt. Keep old save/checkpoint method signatures as compatibility wrappers, with the same mapping update path and old-operation replay support.
- Store methods `draft_comments(draft_id: Uuid) -> Result<DraftComments>`, `create_draft_comment(request: CommentCapture) -> Result<CommentCreated>`, `set_comment_status(request: CommentStatusChange) -> Result<CommentStatusChanged>`, and `write_draft_with_comments(request: DraftWriteWithComments) -> Result<DraftComments>`; `comment_anchor_snapshots(draft_id: Uuid) -> Result<Vec<CommentAnchorSnapshot>>` returns `CommentAnchorSnapshot { revision: DraftRevision, anchors: Vec<(Uuid, AnchorState)> }` for immutable checkpoint recovery. Checkpoint recovery includes only that draft and bounded text already subject to the existing revision limit; load once on open/after checkpoint, not on every keystroke.

Workspace wrappers contain no GPUI. Worker `Action`/`Outcome` variants carry operation ID, draft ID, submitted generation and matching target stamp; failures retain worker-job correlation. Refresh/open returns draft and comments as one coherent projection. If later asynchronous list results target an older stamp, ignore/reload them, not the editor. Lifecycle responses update only their matching comment/status version and cannot regress a newer lifecycle acknowledgement. No model/provider action is added.

The native page offers Capture selection, a comment composer with captured quote, Save checkpoint + add comment, visible open/resolved status, Resolve/Reopen, Show passage only for a validated current anchor, and Show original revision for every comment. Before Show passage, compare current editor text/generation to the projection and revalidate range/quote; use `selected_range`/`set_selected_range` as the experiment does. Original review is read-only and never restores editor text. Show the reason for deleted/ambiguous anchors and retain a scrollable exact quote. Reuse the proven stable-ID quote scroller, wrapping and nonshrinking panes. Keep all actions reachable at 800×600.

## Review focus

- Dirty capture, typing during commit and delayed acknowledgement: no comment on wrong bytes; newer text stays dirty (Tasks 1–3).
- Delete then retype, duplicate passages and repeated-character edit alignment: never quietly choose a surviving occurrence (Task 1).
- Undo, redo and restart: recover only from exact immutable snapshots; lifecycle survives independently (Tasks 1–3).
- Old operation replay after later saves/resolve/reopen: return original receipt without reapplying state or mutating mappings (Task 1).
- Corrupt/mismatched original metadata, migration failure and invalid traces: fail atomically and retain prior data (Task 1).

## Sequenced implementation tasks

### Task 1: Storage and mapping authority

#### 1a. Pure mapping and edit-trace contract

**Files:** create `crates/brn-store/src/anchors.rs`; expose types in `crates/brn-store/src/lib.rs`; tests colocated in the pure module or `crates/brn-store/tests/anchors.rs`.

**Interfaces:** implement the mapping/trace types and functions above without SQL or GPUI. Add a pure replay helper taking initial text, trace, final text, comment projections and immutable recovery references; return validated final mappings. Deriving edits and replay share exactly one mapping implementation.

- [x] Write failing tests for insert/delete strictly before and after a range; insertion at both boundaries; overlap and whole-range deletion; duplicate originals and new duplicates; overlapping quote occurrences; Unicode/emoji/combining marks, CRLF and final newline. Assert exact byte offsets and location tags.
- [x] Add repeated-character alignment regressions (e.g. selecting a unique multi-character quote adjacent to a run that can move a minimal insertion across its boundary); every competing identity yields ambiguity. Add broad multi-region replacement conservative failure, no-op preservation and invalid boundary/range checks.
- [x] Add delete/reinsert trace, original/checkpoint exact undo recovery, redo invalidation, conflicting snapshot range, HistoryLost, malformed reconstruction, intermediate oversize and step/byte limit cases. A unique quote in a different document must never resurrect an unresolved anchor.
- [x] Run `cargo +1.98.1 test -p brn-store --locked --offline` and record expected missing-feature failures.
- [x] Implement the pure contract, with bounded scans and checked arithmetic. Do not use a fuzzy/diff alignment to assign identity.
- [x] Rerun the focused mapping tests and request Astra review of the mapping rules before persistence wiring.

#### 1b. Durable comments, draft integration and migration

**Files:** create `crates/brn-store/src/comments.rs`, `crates/brn-store/tests/comments.rs`; modify `crates/brn-store/src/drafts.rs` and `crates/brn-store/src/lib.rs`.

**Interfaces:** produce the schema-v5 store APIs/receipts above. Refactor the existing draft transaction internals just enough to share checkpoint/save work inside comment creation; do not nest transactions or invoke a separately committed public save.

- [x] Write failing tests for atomic dirty-buffer capture with exact original quote/revision/hash; clean capture; two comments on separate checkpoints; empty/non-boundary/out-of-bounds/body-limit cases; stale base/generation; and forced failure after checkpoint insertion with complete rollback.
- [x] Add save/checkpoint tests replaying multiple edits, deletion/reinsertion, Unicode and HistoryLost. Assert working mappings and immutable checkpoint mappings commit together. Exercise existing untraced public save/checkpoint calls so they cannot bypass mapping updates.
- [x] Test resolve/reopen CAS and no-op, cross-draft/comment IDs, generation/status overflow, original tampering, wrong hash/range, and invalid serialized receipt identities. Lifecycle writes cannot alter text, provenance or location.
- [x] Test identical operation replay after later text and lifecycle writes; changed request/trace/op-kind conflicts fail. Validate old schema-v4 draft retry receipts remain usable. All replay tests assert database rows remain unchanged.
- [x] Add populated v4 migration/reopen plus injected migration rollback, and retain v1/v2/v3/newer/corrupt schema coverage. Add a separate-process reopen fixture for comments/mappings/status after acknowledged saves.
- [x] Run store tests to confirm missing behavior, implement the transaction/migration contract, then run `cargo +1.98.1 test -p brn-store --locked --offline` to green. Astra reviews schema, transaction and retry integrity before native wiring.

### Task 2: Workflow and worker

**Files:** create `crates/brn-workflow/src/comments.rs`, `crates/brn-workflow/tests/comments.rs`; modify workflow `src/lib.rs` and `src/worker.rs`.

**Interfaces:** consume Task 1 store APIs with matching Workspace wrapper signatures. Worker actions carry the request structs above; outcomes carry matching receipts. Open/refresh and traced write outcomes include the matching draft/comment projection and immutable recovery references. Existing untraced worker callers remain supported.

- [x] Write failing worker tests for capture/save/status/reopen with no provider configured, stamp-matched projections, operation/job correlation and original revision ownership. Existing candidate behavior remains separate and unchanged.
- [x] Run `cargo +1.98.1 test -p brn-workflow --locked --offline` to demonstrate missing behavior; implement wrappers and worker messages without UI or model dependencies.
- [x] Rerun the workflow suite to green and request controller/Astra boundary review before native wiring.

### Task 3: Native state and persistent review UI

**Files:** create focused `crates/brn-desktop/src/comments.rs`; modify desktop `src/drafts.rs`, `src/native.rs` and module declarations.

**Interfaces:** consume Task 2 messages; produce pure native state for captures, trace prefixes/suffixes and preview mappings. Native GPUI code owns focus and entities only.

- [x] Write pure native tests for stale capture after edit/undo, add acknowledgement after further typing, save suffix replay, failed submission/worker failure preserving trace and composer, unrelated results ignored, stale list projection, resolve completion during dirty editing, trace overflow, discard, composer typing during commit and guarded dirty-composer/pending close/switch. Assert no acknowledgement calls for replacing live text or selection.
- [x] Run focused native tests to demonstrate missing behavior; implement pure state first, then wire the UI controls and preview described above. Preserve all existing draft lifecycle guards and scrolling patterns.
- [x] Run native desktop tests/build/all-target Clippy; commands use `--features native-retrieval --locked --offline`, pinned toolchain and the controller's shared target directory. Keep build jobs bounded and use `CARGO_INCREMENTAL=0` if the shared cache requires it.
- [x] Controller exercises fresh disposable native data: keyboard and mouse capture, Unicode clipboard quote/body, dirty capture checkpoint, two comments, Show passage/original, before/after insertions, full deletion, duplicate paste, undo/redo exact recovery, resolve/reopen, additional typing during a save, narrow-window and long-quote scrolling. Confirm actual SQLite provenance through read-only inspection and then restart a separate process to verify exact text/status/location.

### Task 4: Evidence and full review

**Files:** create `docs/anchored-comments-evidence.md`; update `docs/status.md`, `docs/roadmap.md`, and relevant launcher/use instructions.

- [x] Run `bash scripts/verify-end-to-end.sh`, provider harness offline tests, editor trial tests, native desktop test/build/Clippy, launcher regressions and `git diff --check`. Rerun only affected checks after corrections.
- [x] Record command results, disposable workspace paths, native observations, separate-process reopening and conservative mapping/undo limits. Distinguish agent observation from subjective user acceptance and do not claim keyboard undo history persists across restart.
- [x] Obtain independent Astra review of the full change, especially edit ambiguity, replay receipts, migrations, trace/ack races and lifecycle/location independence. Sol resolves material findings; Luna runs bounded checks as directed.
- [x] Controller commits and pushes the reviewed feature branch only under the existing task authorization, and verifies remote SHA. No merge or release.
