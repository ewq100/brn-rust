# Standalone chat implementation plan

**Goal:** Deliver a dependable standalone chat experience with durable chat-only attachments, streaming, a saved message queue, deliberate steering and quiet display, before projects.

**Baseline:** `65119a73a6aaea13764b8bc6290f7bb6dc79ff2d` (PR #128), isolated branch `codex/thread-product-discovery`. Discovery docs are task-owned untracked files. Original checkout/experiments are untouched. The newly merged Copilot/title-bar fixes are included.

**Spec:** [Product Feature Brief](spec.md), stage one only. [Research](research.md) informs polish without assigning additional features.

**Architecture:** Keep the existing SQLite checked service, GPUI Kit renderer, converter and Rig provider runtime. Persist queue/control separately from provider settings, and ephemeral composer recovery separately from meaningful revision history. Retain chat attachments as protected notes with a host-controlled chat context; explicit Keep in BRN changes that visibility on the same record. Ordinary chat history uses Archive instead of Resolve; internal legacy Thread identities remain stable.

**Stack:** Rust 1.98.1, locked dependencies, GPUI Kit 0.7.3, Rig 0.44.0. No new libraries; direct serde/JSON, SHA and UUID imports reuse versions already locked by the repository.

## Authority and gates

- Owner requested chat-first implementation and selected the consequential behavior in the spec. Lead implements in this session; independent reviewers are read-only.
- Projects, release, real-data access, publication and merge are outside this build. The 13 October follow-up authorizes native queue/steering/attachment observation and synthetic selected-provider runs; automated tests, native observation and owner acceptance remain separate evidence.
- Use task-owned synthetic data and the existing compatible build cache only. Preserve provider selection, host grants, exact review, versioned Save, dirty guards, receipts/Undo, source coverage and full backup/restore.
- Defined: satisfied. Plan review: satisfied (two rounds). Implementation, automated verification and independent code review: satisfied. Native observation: satisfied for the scoped synthetic follow-up below; broader qualification remains pending. Owner acceptance: pending. Merge/release: unassigned.

## Shared interfaces and ownership

Lead owns every edit and integration. New focused files: core `chat.rs`, `context.rs`, `upgrade.rs`; app `chat.rs`; native `threads_chat.rs`; behavioral tests beside each component. Existing `types.rs`, `store.rs`, intake, runtime, CLI and tile renderers consume their service APIs.

Core contracts:

```rust
// serde-defaulted/omitted when absent, preserving existing request bytes.
Note.chat: Option<String> // None = general BRN; Some(id) = chat-only
Source.chat: Option<String> // independent observations per context/locator

ChatControl { queue: Vec<QueuedMessage>, pending: Option<QueuedMessage>,
              paused: bool, quiet: bool }
QueuedMessage { id: String, thread: String, text: String,
                message: Option<String>, run: Option<String>, dispatch: Option<String> }
Run.instruction: Option<String> // host-owned exact owner Message identity
Run.queue_claim: Option<String> // immutable claim validated/bound at atomic start
ChatDraft { destination: String, generation: u64, text: String }
RecordData::ChatControl(ChatControl)

Store::records_for_chat(&self, chat: Option<&str>) -> Result<Vec<Record>>
Store::record_visible_in_chat(&self, id: &str, chat: Option<&str>) -> Result<bool>
HostAuthority::for_chat(self, chat: &str, private_context: bool) -> Self
Store::chat_drafts(&self) -> Result<Vec<ChatDraft>>
Store::save_chat_draft(&mut self, destination: &str, generation: u64,
                      text: &str) -> Result<ChatDraft>
Store::upgrade(&Path, &Path) -> Result<()> // explicit external backup folder
```

App contracts in `chat.rs`:

```rust
Workspace::chat_control(&self) -> AppResult<Record>
Workspace::queue_message(&mut self, key: &str, thread: &str,
                         text: &str) -> AppResult<Receipt>
Workspace::edit_queued(&mut self, key: &str, item: &str,
                       text: &str) -> AppResult<Receipt>
Workspace::remove_queued(&mut self, key: &str, item: &str) -> AppResult<Receipt>
Workspace::move_queued(&mut self, key: &str, item: &str,
                       before: Option<&str>) -> AppResult<Receipt>
Workspace::pause_queue(&mut self, key: &str) -> AppResult<Receipt>
Workspace::resume_queue(&mut self, key: &str) -> AppResult<Receipt>
Workspace::recover_queue(&mut self, key: &str) -> AppResult<Receipt>
Workspace::dispatch_queued(&mut self, key: &str) -> AppResult<Option<QueuedMessage>>
Workspace::bind_queued_run(&mut self, key: &str, item: &str,
                          message: &str, run: &str) -> AppResult<Receipt>
Workspace::finish_queued(&mut self, key: &str, item: &str,
                        message: &str, run: &str,
                        terminal_fence: u64,
                        success: bool) -> AppResult<Receipt>
Workspace::archive_chat(&mut self, key: &str, thread: &str) -> AppResult<Receipt>
Workspace::unarchive_chat(&mut self, key: &str, thread: &str) -> AppResult<Receipt>
Workspace::keep_in_brn(&mut self, key: &str, note: &str,
                       version: u64) -> AppResult<ApplyOutcome>
Workspace::attach_full(&mut self, key: &str, thread: &str, title: &str,
                       converted: &ConversionResult) -> AppResult<ApplyOutcome>
```

Owner commands use immutable bound operation identities and the checked writer. Core transaction helpers construct and apply the complete group under one IMMEDIATE transaction; reading mutable queue state outside that transaction is not sufficient. UI clears input only after a successful durable outcome. Model proposals cannot modify chat visibility, queue or recovery controls.

## Review focus

- Stop/Archive/failure/restart cannot advance a queue through a late Finished event; explicit Resume is required.
- A dequeued message survives a crash between posting and starting its run without being posted twice; pending work is distinct from waiting work.
- Private attachment content cannot leak through global search, direct IDs, old revisions, assets, source metadata, unrelated-chat messages or autonomous general-note writes.
- Schema upgrade must preserve the v3 database and produce a verified pre-upgrade backup outside canonical storage before any format/index change; missing backup authority/path fails unchanged.
- Native stream updates must preserve reading position and selectable text; quiet mode must retain Stop, errors and attention, and existing writing review must still work.

## Task 1 — persistent controls and safe format upgrade

- [x] Extend `types.rs` with optional `Note.chat`/`Source.chat`, host-owned `Run.instruction`, ChatControl/QueuedMessage/ChatDraft and the host-only RecordData variant. Old ordinary note/source serialization remains byte-identical. Context/control changes require owner-only configuration authority, beyond maintenance/content grants.
- [x] Add schema 4: singleton ChatControl index, per-context source-locator uniqueness and bounded latest-generation `chat_drafts` recovery table. Recovery drafts do not create permanent keystroke revisions.
- [x] In `upgrade.rs`, validate v3 identity/integrity before writes, refuse live Working runs/dirty edit sessions, require an explicit or remembered usable external backup directory, create an exclusive verified v3 snapshot, then transact the schema/index upgrade. Hold/recheck the checked writer's upgrade preconditions and snapshot consistency against competing writers; no post-snapshot v3 changes may slip into the upgraded state without being backed up. Do not silently choose another backup destination. Keep existing foreign-marker and missing-database refusal.
- [x] Expose explicit `brn ... upgrade --backup-directory ABSOLUTE_FOLDER` using the same core method. Normal open upgrades only with a valid remembered folder; otherwise return the actionable upgrade command. Restore accepts known supported backup formats and validates/upgrades before use without changing the source backup.
- [x] Core tests: preserved old request hashes; wrong/foreign schemas untouched; v3 backup contains exact old records and old schema; unavailable/inside-workspace/colliding destinations fail unchanged; fresh v4 and restored v4 preserve drafts/control.
- [x] Verify: `cargo test --locked --offline -p brn-threads-core` and focused CLI parser/upgrade tests. Commit this coherent service foundation after passing checks.

## Task 2 — queue, pause, recovery and archive service

- [x] Implement app wrappers over checked core transaction helpers. Queue is workspace-serialized, with destinations bound per item. Each queued item is a distinct turn; no automatic merging or old protected-grant reuse.
- [x] Enqueue/edit/remove/reorder are bound/replayable owner actions. Dispatch refuses paused controls, archived destinations or any live workspace Run. It moves an item to pending and creates its owner Message together. Pending retains the exact posted Message identity for retry/recovery; a repeated dispatch cannot repost it.
- [x] Bind prepared Run start and current dispatch claim atomically in the same checked group to the pending item and exact owner Message via `Run.instruction` and `QueuedMessage.run`. `ThreadRunRequest.request_message: Option<String>` plus `queue_claim` supplies the trusted queued Message identity; validate its current role, destination, text and position before start. Existing nonqueued callers supply None. Bind requires the Run's instruction/destination to match and its state to be Working. A crash before binding leaves a recoverable pending Message, not a second send.
- [x] Successful terminal work acknowledges pending and permits the next item. `ThreadResult.terminal_run: Option<(String, u64)>` is emitted only from that invocation's successful checked finish and captures its exact persisted terminal fence. Preserve it in the worker event. `finish_queued` atomically checks expected item/Message/Run, Run.instruction, exact terminal fence and persisted terminal result; caller success alone is insufficient. Receipt replay comes first; a stale tuple/token does not clear or pause newer work. A successful acknowledgement never unpauses controls. Failure/Stop/recovery pause while preserving pending/waiting text. Resume retries a pending saved instruction or starts the next waiting item with current context; no old write grant survives. A posted Message is reused only when it is the exact pending identity, rather than relying on matching prose.
- [x] Archive marks the chat archived, preserves attention/linked Actions, fences its live runs and pauses the queue in one group. Unarchive preserves paused controls and does not restart anything. Treat legacy resolved conversations as archived presentation; unarchive restores Open internally without changing identities.
- [x] Stop-and-send/Steer also updates a pending item's saved Message reference when it supersedes that instruction, preserving later waiting items and avoiding resurrection of the old prompt after restart. The control group must be part of the same writer transaction as the run fence/new instruction.
- [x] Persist composer drafts with monotonically increasing per-destination generations, rejecting delayed clears/writes. Hydrate them at startup and preserve them during navigation and failed send.
- [x] Tests: repeated enqueue/dispatch, immutable-key misuse, restart before/after dispatch/bind, exact message reuse, cross-process competing dispatch, old-run completion after Stop→Resume or Steer and forged success on a noncompleted Run, Stop race with Finished, queued edits/reordering, archived destination, stale draft clear, unfinished Actions/attention preserved. Verify core/app focused tests, then commit.

## Task 3 — durable attachment context and explicit retention

- [x] Factor existing `import_full` to share a scoped full-import builder with `attach_full`; preserve converter coverage, source identity, asset hashes and grouped receipt. Set the new note's chat context and add its chat link atomically. Deduplication distinguishes the same import in different scopes.
- [x] Keep in BRN is a checked, version-bound owner visibility change on that same protected note; preserve content, assets, protection and links. Never duplicate the note or unprotect it. Archive/removal does not promote content.
- [x] In `context.rs`, implement one visibility policy: general current notes plus the selected chat's notes; assets/comments inherit owning-note visibility; sources/links cannot expose an exclusively private owner through another read path; unrelated Message/Run/chat-control contents are unavailable as context. Historical reads respect both current record scope and historical scope.
- [x] General note lists/search/Markdown export exclude chat-only content; full backup includes all canonical content and recovery state. Owner inspection of a selected chat may read its attachment and retained figures after restart without the original.
- [x] A run with private attachments must not autonomously copy information into general BRN. `HostAuthority::for_chat` is host-only, checked against the bound Run's chat, and binds the private-context restriction: maintenance general writes need explicit review/retention, and model-authored visibility changes are rejected. New working notes generated in private context are scoped by the host to that chat. Applicable owner selected-note/decision instructions and exact-candidate review retain their existing bounded authority. Previously archived unpromoted attachments retain the restriction for saved conversation content; removing a chip cannot bypass it.
- [x] Tests: source removed + restart still reads complete converted content/figure; private search exclusion; guessed-ID/history/asset/source/link and input/write bypasses rejected; the same source locator in two chats and general BRN cannot expose a foreign private observation/revision; maintenance cannot generalize private content; Keep is version checked/replayable and makes the same note eligible; backup/restore preserves isolation.
- [x] Verify core/app import, search, export and backup tests; commit.

## Task 4 — runtime context and truthful steering

- [x] Use scoped record reads in `threads.rs`, `threads_history.rs`, source/materialization and captured writing inputs. Validate all proposal inputs/targets against allowed context before preparing/applying, including refresh. No model argument controls the current chat scope.
- [x] Include bounded attachment identities/coverage in the fresh run context and allow exact-version note-range reads. Do not insert unbounded full attachments into every prompt, or reread a missing external original.
- [x] Preserve the nine existing tools, stream/final-answer bounds, provider/model/effort choices and one final assistant Message. Save useful displayed interrupted text as explicitly partial progress, never a completed final answer.
- [x] Implement Steer with the existing safe stop/fence/fresh-invocation handoff on these providers. Show the transition honestly; do not claim provider mid-turn injection. Wait for the old worker before restarting, retain prior accepted results, and recheck current authority/settings.
- [x] Rig mock/provider-contract tests prove private context read/write boundaries, copied Source/input attempts, partial termination, late events after steering and loss of old protected authority. Preserve merged Copilot preflight tests. Verify `cargo test --locked --offline -p brn-ai`; commit.

## Task 5 — polished native chat

- [x] Put queue/composer/attachment behavior in focused `threads_chat.rs`; wire existing tile renderers and shared-service dispatch without replacing the editor or providers.
- [x] Enter sends when idle and queues during work (including another destination); Shift-Enter remains multiline. Separate explicit Steer is available for the active chat. Show waiting cards above composer with edit/remove/reorder, status and explicit Resume. Only successful completion dispatches another item; a late event cannot defeat pause.
- [x] New/continued chats stay in history; remove user-facing Resolve/Reopen and quick-question auto-resolution. Recent/all/archived lists and launcher remain findable; Archive/Unarchive use service methods. Ordinary chat history does not require project controls.
- [x] Add default chat-only Attach file, retaining the existing explicit useful/full general intake choices. File picker/paste-drop paths use the existing converter budgets and supported formats. Show preparing/ready/failed, inspectable coverage/figures and Keep in BRN. Do not clear a failed source/message draft or label a failed conversion usable.
- [x] Reuse pinned Kit's `MessageScroller`/`MessageScrollerState` (virtual rows, tail following, anchor-preserving remeasurement and Jump to latest), with state per chat. Keep stable selectable TextView state per response; update the current Markdown text without recreating its identity on every stream event. Copy answer/selection and current failed-answer Continue stay accessible. No historical prompt editing/branching is introduced.
- [x] Quiet is a persisted presentation preference in ChatControl, not a provider-settings change. It collapses intermediate activity detail while retaining streamed answers, compact truthful status, Stop, errors, proposals and Needs you. Keep keyboard labels/focus, readable themes, reduced-motion behavior and safe links.
- [x] Native state tests: queue/resume/steer, navigation/restart drafts, archive during work, late event safety, private file visibility, Keep/review/Undo, quiet during streaming, long-answer scroll retention and existing comment/rewrite journeys. Verify native compile/tests/strict lint; commit.

## Task 6 — contracts, independent review and candidate evidence

- [x] Update canonical target/behavior/runtime guides, CLI help, desktop README and status for delivered chat semantics and scoped intake only; projects remain explicitly deferred. Keep public Thread IDs/types where renaming them would add migration risk; UI language is Chat.
- [x] Run formatting, Markdown links, core/app/AI/CLI suites, native widget/state suite and strict native Clippy, relevant synthetic end-to-end and headless startup/backup witnesses. Use existing helper/Poppler configuration and name tested source/features.
- [x] One independent read-only whole-diff review covers spec/invariants, context isolation, queue races and migration/recovery. Lead resolves findings and reruns only affected evidence.
- [x] Save concise `evidence.md` with exact candidate, checks, review dispositions and an owner native qualification checklist. Report delivered code and automated proof separately from deferred native observation/owner acceptance. Do not claim merge/release or begin projects.

## Verification commands

```sh
export CARGO_TARGET_DIR=/tmp/brn-ui-repairs-target-20261010
cargo build --locked --offline -p brn-intake --features helper --bin brn-intake-helper
cargo test --locked --offline -p brn-threads-core -p brn-threads-app -p brn-ai -p brn
cargo test --locked --offline -p brn-desktop --features native-ui,native-test-support --bin brn-desktop threads_ui::
cargo clippy --locked --offline -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings
cargo fmt --all -- --check
python3 scripts/check-markdown-links.py
git diff --check
```

Set explicit `BRN_INTAKE_HELPER`, `BRN_PDFTOTEXT` and `BRN_PDFIMAGES` paths when fixture suites require them. Additional commands/logs belong in the evidence record, not as unsupported gate claims here.

The native state invocation must execute a nonzero number of tests (baseline: the existing focus, backup, steering, writing, tiled and Omakase modules). A `native-ui`-only build or zero-test result cannot satisfy that gate. Align `verify-desktop-shell.sh` with CI's `native-test-support` feature if using that helper for evidence.

## Plan review record

Round one: Changes required. Tightened queue completion to an exact pending item/Message/Run plus checked terminal-fence token, including stale/forged completion tests; corrected native commands to enable and explicitly execute widget/state tests. Source-scope and competing-upgrade-writer witnesses included. Round two: Approved by the same read-only reviewer. Lead implements here under the owner's chat-first request and repository workflow.

## Candidate checkpoint

Standalone chat code is complete and independently approved. The final automated gates pass: full workspace storage/CLI gate (484 passing tests), offline end-to-end fixtures, dedicated chat CLI adoption/attachment/queue/Keep witness, production native build, 87 native widget/state tests, strict native Clippy, 36 tooling tests, formatting and Markdown links. Three default-suite ignored entries are isolated child fixtures launched by parent tests; no zero-test result is used to qualify native behavior.

Plan adjustments within the approved behavior: dispatch claims bind atomically at Run start; managed figure URLs receive the same scope guard as typed references and native image maps; Archive/Steer retain displayed partial text in the interruption transaction; queue execution controls cannot be undone into earlier claims; successful non-provider work can dispatch waiting questions. Keep preserves owned figure IDs across multiple source observations and copies borrowed same-chat private figures without promoting the source document. Direct serde/JSON/SHA/UUID dependencies reuse locked libraries.

The task remains isolated and unmerged. Projects, real owner data, publication and release remain outside this build. Native observation and synthetic selected-provider calls are now authorized by the follow-up assignment below. The [evidence and owner checklist](evidence.md) record qualification separately.

## Native end-to-end follow-up, 13 October 2026

Owner now authorizes native app testing and fixes for queue, steering and attachments, including selected-provider runs against fresh synthetic data. This supersedes the earlier deferral for these journeys only. Preserve the owner's original trial workspace. Use a separate explicit native test workspace; no private source files, credentials in evidence, publication, merge or release.

- [x] Observe native attachment import/read/inspection, queue edit/remove/reorder and sequential execution, Steer, Stop/Resume, failure and restart recovery.
- [x] Fix demonstrated defects with focused regression checks and rebuild the production native candidate.
- [x] Retest corrected behavior in the actual app; record exact candidate/data/evidence and independent review for meaningful changes. Owner acceptance remains distinct.

## Quiet display follow-up

Owner rejected the subtle Quiet on/off difference, then specified that Quiet off should show tool calls and agent reasoning with a terminal/CLI experience. Keep streamed answers, Stop, errors, queue and owner decisions available in either mode.

- Reproduce the global status-bar bypass in a native widget test and the actual app.
- Share the Quiet policy across live status displays; give the toggle a selected state and clear labels.
- Add a bounded monospace activity stream with admitted tool calls, argument previews, truthful results and explicit public provider reasoning summaries. Do not expose raw reasoning, encrypted payloads, credentials or full source documents as telemetry. Keep activity separate from saved assistant answers; retain the latest run in its own chat while the app remains open, hiding it in Quiet mode.
- Request public summaries on Responses while preserving selected provider/model/effort. Verify runtime/mock and provider boundary, native rendering/toggle/Stop and scope, lint/build and independent review. Update the test app after native retest; publication, merge and release remain unassigned.


Quiet follow-up candidate `14ec5c38`: implemented and independently approved; 226 AI tests and 92 native tests pass, strict native lint and production build pass, and actual native selected-provider/toggle/independent-scroll checks pass. [Evidence](evidence.md#quiet-off-terminal-activity--13-october-2026) records the bounded public-summary boundary, findings and exact binary. Updated synthetic app is open with Quiet off. Owner acceptance remains pending; hosted CI/integration/release are unassigned.


## Restore dismissed work status

Owner clarified that the “Work … Finished” banner should remain optional but reopenable after ×. Keep the latest completed worker status in memory and expose **Last work status** in the layout menu; × hides it. Restoring must not restart a run or replace a newer error/status. While work runs, do not restore an old completion as current status. No persisted notification history or tile behavior changes are assigned. Verify the real dismiss/menu/restore path, independent review, native build and actual app with synthetic data.


The owner also reported duplicate model controls in the composer and status bar. Keep provider/model/effort controls in the composer; replace the idle status-bar model menu with a plain Idle indicator. Preserve active progress and Stop, existing Settings access and current model/effort configuration.


Status follow-up `4b235b97`: implemented and independently approved, 94 native tests/strict lint/build pass, and actual same-banner dismiss/restore plus nonduplicated model controls were observed. Fresh **BRN Status Test** is open; [evidence](evidence.md#reopen-work-status-and-remove-duplicate-model-controls--13-october-2026) pins the build and one Completed run. Owner acceptance remains pending; integration/release unassigned.


## Contextual note and decision actions

Owner authorized removing Ask/Edit note/Record decision from the composer. Continue using this existing plan (writing-plans workflow) rather than creating a parallel plan. Plain chat/Enter/queue/Steer carry no extra note or decision grant. Remove ComposeMode and its obsolete menu tests.

- Put **Edit note** in the open note's actions. Open a separate instruction dialog bound to its exact saved ID/version, then post an owner instruction in its writing chat and use the existing one-invocation EditNote grant. Preserve the ordinary composer draft. Disable during work, pending paused queue, dirty/recoverable/stale note or review; recheck before dispatch.
- Put **Save as decision** on completed saved assistant replies. Review/edit title and text in a separate dialog, clearly naming BRN retention and protection/confirmation. Save the owner's exact reviewed text through one replayable checked owner group (protected/confirmed Note and link to the originating chat); keep source Message/version provenance. Cancel does nothing; partial/streaming replies do not expose this action. Do not start another model call to save text.
- Implement the decision save in a small shared app module with bound operation identity, source-message/current-input checks, asset visibility checks, replay and Undo. Chat/attachments stay unchanged; saving a decision explicitly retains only the reviewed content in general BRN.
- Use a focused contextual native module for dialog/dispatch behavior; native tests cover removal, exact EditNote target, no incidental grants, preserved drafts, review/cancel/save and failures. Shared app tests cover replay, stale/foreign/partial sources, protection/provenance, visibility, assets and Undo. Reuse current note/review/provider paths.
- Run relevant app/native checks, strict lint/build, independent read-only review and actual synthetic native journeys; rebuild/open the latest app. Publication/merge/release remain unassigned.
