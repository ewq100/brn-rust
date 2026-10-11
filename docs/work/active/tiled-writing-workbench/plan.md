# Tiled writing workbench implementation plan

**Goal:** Implement the accepted [scope](spec.md) in the existing native application and publish a tested, independently reviewed PR.

**Baseline:** `adc3b273e1985bc56a36d912ad26b34af99e703a` (PR #121); isolated `work/tiled-writing-workbench`. The owner authorized implementation with GPT-6.1 Sol subagents and publication. No additional routine approval is needed.

**Architecture:** Keep SQLite and the shared checked writer authoritative. Persist a typed writing snapshot inside a host-created Message, use existing Comment/Link records for passages, and route explicit rewrite through the current provider runtime with host-enforced proposal-only scope. Compose the existing GPUI note/editor, review, thread and composer into one workspace.

**Stack:** Rust 1.98.1, GPUI Kit 0.7.1, Rig 0.44.0, existing SQLite core and lockfile. No new dependency or schema change.

## Interfaces and ownership

Service helper owns `crates/brn-threads-core/src/writing.rs`, its export, minimal core writer guards/tests, `crates/brn-threads-app/src/writing.rs`, its export and tests. Runtime helper owns `crates/brn-ai/src/threads.rs`, related runtime modules/tests. UI helper owns `crates/brn-desktop/src/threads_ui.rs`, `threads_state.rs`, focused native tests and any small desktop-only writing module. Lead owns plans/contracts, integration and publication. Coordinate before touching another helper's files.

Shared core interface:

```rust
pub const WRITING_REQUEST_ROLE: &str = "writing-request";
pub struct WritingSnapshot {
    pub request_message: String,
    pub thread: String,
    pub note: Record,
    pub comments: Vec<Record>,
    pub messages: Vec<Record>,
}
impl WritingSnapshot {
    pub fn message_text(&self) -> Result<String>;
    pub fn from_message(record: &Record) -> Result<Self>;
    pub fn current_inputs(&self) -> Vec<RevisionInput>;
    pub fn change_request(&self, markdown: &str) -> Result<ChangeRequest>;
}
```

The snapshot is serialized in one Message with role `WRITING_REQUEST_ROLE`; validate record identity/type/thread and the complete bounded payload. `current_inputs` includes note, comments, captured messages and request Message. `change_request` writes only the selected complete Note, preserves its non-Markdown fields, and uses the captured base and inputs. Limit exact snapshot JSON to 128 KiB with an actionable failure, never truncation.

App interface (all writes use checked service and caller-supplied durable idempotency key):

```rust
pub struct WritingComment { pub record: Record, pub addressed: bool }
pub struct WritingCommentInput {
    pub note: String, pub version: u64, pub thread: String,
    pub range: (usize, usize), pub quote: String, pub body: String,
}
// Workspace methods:
fn writing_threads(&self, note: &str) -> AppResult<Vec<Record>>;
fn link_writing_thread(&mut self, key: &str, note: &str, version: u64, thread: &str) -> AppResult<Receipt>;
fn add_writing_comment(&mut self, key: &str, input: &WritingCommentInput) -> AppResult<Record>;
fn writing_comments(&self, note: &str) -> AppResult<Vec<WritingComment>>;
fn snapshot_writing_request(&mut self, key: &str, note: &str, version: u64, thread: &str, comment_ids: &[String]) -> AppResult<WritingSnapshot>;
fn writing_request(&self, message: &str) -> AppResult<WritingSnapshot>;
fn mark_writing_comments_addressed(&mut self, key: &str, request_message: &str, applied_operation: &str, comment_ids: &[String]) -> AppResult<Receipt>;
```

Addressed confirmation is a host-created typed Message referencing the saved request, actual matching Apply receipt, and selected captured comments; derive pending state from that proof and subsequent Undo. Do not overload `Comment.unresolved`, which describes location uncertainty. Comments belong to the note and can be selected across linked discussions; the chat snapshot is only the explicitly selected thread.

Runtime adds `OwnerRequest::RewriteNote { request_message: String }`, with no new mandatory `ThreadRunRequest` field. Load/validate the saved snapshot against canonical stored revisions, consume its exact chat instead of the generic last-40-message truncation, and force one existing target Note plus captured Current inputs in preparation. The snapshot envelope is `{version: 1, snapshot: ...}`. Reject transient sources, use a fixed rewrite command instead of arbitrary caller prompt, and do not post a duplicate synthetic owner message. In rewrite mode all mutable record/source/history reads are refused: the complete bounded snapshot is already present; guide/progress tools remain available. Model tools cannot create host Messages or choose rewrite authority. All modes must preserve the durable review restriction on marked proposals; the core recognizes the saved writing-request input and requires owner review even if maintenance or a content instruction would otherwise allow writes. Core candidate replacement must retain this input. Rewrite runtime rejects unrelated candidate application/refresh and scope expansion. Continue from a saved rewrite must use its saved explicit mode rather than fall back to ordinary maintenance.

## Review focus

1. Two notes/two linked chats: typing and comment anchors never migrate to the wrong destination.
2. Repeated phrases, UTF-8, formatted Markdown and cross-paragraph ranges: exact supported occurrence or explicit refusal; no fuzzy relocation.
3. Note/comment/chat changes while snapshotting or generating: checked input versions, fixed captured context, later additions excluded, old drafts retained.
4. Maintain-ordinary policy, Continue and refresh: rewrite candidates still require exact owner review; no tool-side bypass.
5. Failed startup, lost response, restart, Apply/Undo then confirmation: durable request replay, no duplicates or premature addressed state.

## Task 1 — durable writing services and integrity

- [x] Add focused tests in `crates/brn-threads-app/tests/writing.rs` for multiple same-thread comments without note mutation; exact repeated/UTF-8 anchors; stale note/comment rejection; snapshot replay after later messages; context bound; addressed validation and Undo. Capture expected failures before implementation.
- [x] Implement shared snapshot module and app methods above using existing records, receipt order and checked writes. Reuse active note mapping checks; capture owner/assistant Messages only, without old writing-request payload recursion.
- [x] Add core tests for maintenance/grant refusal on writing-request inputs, owner-reviewed apply/replay, dirty guard/stale refusal, and candidate replacement trying to drop the review marker. Implement central guards without schema changes.
- [x] Run `cargo test -p brn-threads-core -p brn-threads-app --locked --offline`; resolve affected failures and report exact tests.

## Task 2 — explicit proposal-only runtime

- [x] Add tests in a dedicated `threads_writing_tests.rs`: full captured chat beyond 40 Messages is retained or over-limit rejected, later chat excluded, fake/mismatched snapshot rejected, only selected complete Note accepted, immutable inputs cannot be omitted/downgraded, apply never bypasses review under maintenance/Continue/refresh, and provider-independent failure/retry behavior.
- [x] Extend host mode, begin/input construction, prepare/apply/refresh boundaries with shared snapshot type. Keep run/settings fences, interruption, payload bounds, identity replay and ordinary workflows intact. Do not persist another owner message on identical saved-request retry.
- [x] Run `cargo test -p brn-ai --features capability-spike --locked --offline` and strict Clippy for affected runtime targets. No live provider calls in this assignment.

## Task 3 — native tiled workbench

- [x] Add focused native/state tests demonstrating old missing behavior: note-specific chat drafts, multiple passage Add without worker/navigation, selected-thread context, dirty/stale comment retention, review retaining chat/editor, rewrite request scope and retry, addressed control backed by actual receipt.
- [x] Keep `Page::Note` as workspace; reuse one note editor and selected linked thread. Use compact tiled native panels, readable prose, and full conversation/composer. Preserve existing note metadata/source/recovery/Save controls and thread controls. At narrow native sizes give usable pane proportions/access without duplicate editor state.
- [x] Selection capture uses supported rendered/source ranges and keyboard/mouse events. Opening the comment composer must not interrupt continued keyboard selection. Explicit Comment action focuses input. Add comments through Task 1; remove all prototype paragraph shortcut buttons from the intended interface. Preserve comment draft/anchor by note and main composer by note/thread.
- [x] Add document-level Rewrite with AI with included comment count, explicit scope, bounded snapshot failure, and saved-request retry. Start through Task 2 only after durable snapshot succeeds. Keep chat available while review reuses existing whole-candidate controls; show successful receipt Undo and explicit captured-comment resolution.
- [x] Run `cargo test -p brn-desktop --features native-test-support --locked --offline`, native-ui build and strict Clippy. Continue uses saved rewrite mode; ordinary chat and Stop-and-send remain covered.

## Task 4 — integration, independent review and PR

- [x] Update affected target/behavior/status docs for accepted writing behavior, with native/live-provider qualification limits.
- [x] Run formatting, Markdown links, relevant workspace/core/app/runtime tests, native state tests and combined native feature build/Clippy. Build intake helper first where fixture tests require it. Reuse a compatible cache; retain logs in a task-specific temporary evidence folder.
- [x] One independent read-only review of complete actual diff, spec and invariants; fix verified findings and rerun affected checks.
- [ ] Commit only task-owned files; fetch current main, reconcile if necessary; publish branch and PR via available authenticated connector, attach PR to this chat, inspect required CI. Report actual checks and any pending CI without claiming native acceptance or merge.

## Gate record

- Defined: owner accepted tiled flow; selected text comments only, no paragraph buttons.
- Plan review: approved by independent read-only GPT-6.1 Sol review before implementation. Snapshot encoding, canonical revision validation, no synthetic message, source refusal and snapshot-only reads were clarified in the interface contract.
- Implementation: complete. Focused core/app 10 tests, runtime 11 tests and full native widget/state 50 tests pass. Full default workspace/runtime tests, optional retrieval app tests, workspace/runtime strict lint and combined native build pass. Final native and combined build/test/lint gates pass. Candidate is ready for PR publication; exact commands and review dispositions are in [evidence](evidence.md). Publication identity and hosted CI are recorded on the associated PR and handoff.
- Independent code review: approved by a separate GPT-6.1 Sol read-only reviewer after three P1 corrections and regression tests; no remaining blockers.
- Test-first evidence: initial service/UI checks failed to compile because the new APIs did not exist; this is not claimed as a behavioral red test. Runtime initial test invocation overlapped implementation and passed. Final behavioral suites exercise refusal and successful paths.
- Native observation: explicitly deferred from prior session; no automatic claim of acceptance.
- Provider acceptance: not run; synthetic deterministic runtime tests planned.
- Merge/release: outside current delivery.
