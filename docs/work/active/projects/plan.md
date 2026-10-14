# Projects implementation plan

**Goal:** Deliver stage two of the accepted [Chats and Projects brief](../chats-and-projects/spec.md).

**Authority:** Owner requested implementation on 14 October 2026 and confirmed Projects. Lead implements; independent reviewers are read-only. Release, real-data access and merge are unassigned. Current instruction to implement the existing decisions supersedes optional skill handoff ceremonies.

**Baseline:** `85d291cddfa0f5b5a73e3d4f3b2b96c844445839`, fetched origin/main; isolated `codex/projects-feature`. Original checkout and experiments remain untouched. Standalone chat and the latest navigation/writing polish are merged in this baseline.

**Architecture:** One new host-owned Project record owns membership, explicit instructions, inherited knowledge setting and recoverable deletion metadata. Existing chat IDs and Note/Source scope fields remain stable; a scope may name a conversation or project. Project document links grant document/source/asset access without exposing other projects' conversations. Private retention is independent of original scope identity: explicit project document membership survives unlink from the original project. Sticky Thread.private_context and a monotonic context_epoch preserve private history and invalidate changed membership without coupling review to attention writes. Host-owned Action.chat keeps new private obligations private even when their documents are shared; the explicit Keep decision records independent retention. All commands build and apply through the checked writer in one IMMEDIATE transaction. Context changes fence affected runs and pause pending queues. Schema 5 refuses old readers and requires the existing verified external snapshot path for adopted schema 3/4 upgrades.

**Stack:** Repository Rust 1.98.1, locked dependencies, GPUI Kit 0.7.3, Rig 0.44.0. No new dependencies.

## Constraints and review focus

- Exactly zero or one project per chat; moving preserves saved conversation, drafts and attachments, while changing subsequent context.
- Notes are one record; sharing and edits do not duplicate or implicitly promote content. General search/export excludes private material.
- All exact-ID, historical, source, asset, search, connection and candidate-input paths obey project scope and knowledge-off. Existing linked project documents remain readable with knowledge off.
- Project instructions are owner configuration, not model-authored standing grants. Existing note/review/protection/Undo guarantees remain.
- Deletion preview binds the concrete current removal and retention set and unfinished Actions. Apply is atomic and stale/dirty work fails unchanged. Keep/cancel choices survive restoration.
- Restore preserves later shared-document edits, relationships and settings, leaves cancelled/outside Actions unchanged and never starts work.

## Task 1 — canonical model and checked project commands

Files: core `types.rs`, `projects.rs`, `store.rs`, `upgrade.rs`, `restore.rs`, `lib.rs`; app `projects.rs`, `lib.rs`, `intake.rs`; core/app project integration tests.

- [x] Add `Project` and Trash metadata, serialization defaults, owner-only Configure gate, membership consistency validation, schema 5 and backed-up adoption of schemas 3/4.
- [x] Add core `ProjectCommand` create/configure/move/link/unlink/create-document operations. Expose app `project_command(key, command) -> Receipt`, `projects(include_trash)`, `project_delete_preview(id)`, and project import using existing scoped conversion.
- [x] Add deletion preview containing snapshot identity, removed/retained records and unfinished Actions. `Delete { preview, actions }` checks exact freshness, requires one Keep/Cancel choice per unfinished Action, archives exclusive contents atomically, fences runs and pauses the queue. `Restore` restores original archive relationships with exact post-delete versions and refuses conflicts.
- [x] Tests: replay/key misuse, competing membership changes, protected content/dirty guards, shared retained notes, cancellation/keep decisions, stale preview, restart/backup/restore and schema 4 backup/refusal.

## Task 2 — context, retrieval and runtime

Files: core `context.rs`, `reader.rs`, checked writer; app `search.rs`; AI `threads.rs` and runtime tests.

- [x] Extend one context predicate for project-linked documents, sources/assets/history, project-only exclusion, membership and knowledge-off. Do not expose foreign project instructions/chats through document sharing.
- [x] Search considers allowed project scope lanes and rechecks current canonical access, including knowledge-off and current settings changes; connections count only visible links.
- [x] Fresh run context prioritizes bounded project document identities/instructions; read tool supports query-filtered earlier project messages labelled discussion, not decisions. Context/membership changes invalidate running work and old candidate inputs.
- [x] Tests: unrelated/global/private leak attempts through IDs/history/source/asset/search/write, project instructions, earlier chat discovery, movement and late completion.

## Task 3 — usable native and CLI surfaces

Files: CLI command parser/dispatcher, desktop `threads_projects.rs`, tile layout/rendering, chat/note navigation, focused native project tests.

- [x] Add Projects navigation and project tile: create, rename, explicit instructions, inherited knowledge toggle, documents, chats, start chat, move chat, share/link/import/create document.
- [x] Add concrete deletion review with removed/retained content and per-Action Keep/Cancel choices; Trash and Restore. Preserve editor buffers and drafts, and report failed/stale operations without clearing them.
- [x] CLI routes the same commands and previews through Workspace. General list/search stays general; explicit project inspection does not promote material.
- [x] Native/service journeys cover creating, moving, linking one note across projects, knowledge setting, deletion review and restoration.

## Task 4 — verification, review and candidate handoff

- [x] Update canonical target, behavior/invariants, runtime guidance and status with implemented scope.
- [x] Run formatting, Markdown links, core/app/AI/CLI suites, native build/state tests/strict lint and fresh synthetic CLI witness with backup/reopen. Record exact commands and limits.
- [x] Independent read-only complete-diff review against brief, invariants and verified candidate; fix supported findings and rerun affected checks.
- [x] Save candidate evidence and concrete owner qualification checks. Report implemented, verified, native-observed, owner-accepted and merged separately. Publication/integration follows current authority; do not claim release or merge.

## Independent review corrections

Plan review approved the accepted scope. Whole-diff code review found six supported issues, addressed with regression journeys: schema-4 snapshot validation; mutable attention version in proposal context inputs; lost provenance after leaving a project; active exclusive cancelled Actions; unlinking a privately retained document shared elsewhere; and native writing conversations inheriting the wrong project. The implementation keeps immutable scope identities and explicit project document membership rather than transferring/copying content on unlink. Current gate receipts and limits are in [evidence](evidence.md).

A second review found missing Configure enforcement for context_epoch, terminal Action Trash coverage, retained-source access after unlink and native public-answer progress preservation. Each is corrected and regression-tested. Final coverage also binds Action privacy to its host-assigned origin, requires unfinished choices for shared documents and preserves explicit independent Keep decisions.

**Candidate gate:** implemented and verified at `3569e6cb`, independently APPROVED at tree `b53af0c9dfbf6dd1cddef10ab183a66dca31bc4b`; representative native walkthrough observed. Owner acceptance, hosted CI/integration and release remain separate as recorded in evidence.
