# BRN Rust desktop delivery roadmap

Prepared: 27 September 2026

Status: proposed delivery roadmap, not an approved detailed implementation specification. Chunk 00's minimal source scaffold is tracked in [status.md](status.md); the private repository and build probe are in place. No desktop product implementation, paid model calls, or deployment have been performed.

## Outcome and constraints

Build an installable, local-first desktop application for importing knowledge, asking grounded questions, drafting documents, reviewing anchored comments and revisions, and approving exact content for publication. Application code is Rust; no Pi, Node, or Python service is required at runtime. Native libraries and platform components may still be dependencies.

Retrieval is switchable between keyword, semantic, hybrid, and graph profiles. BRN owns source identity, versioning, approval status, and the common evidence contract. Retrieval indexes are replaceable derived data. Cloud model access does not make the application cloud-hosted; local retrieval and stored documents remain usable offline where their dependencies are present.

Start with macOS on Apple Silicon and the user's Codex subscription as the first model access route to investigate with Rig. The chunk 01 trial must verify that authentication and integration actually work. Add other platforms and providers after the first complete workflow is proven.

## Working method

- One numbered chunk is one independently reviewable outcome, normally one small pull request once repository publication is authorized. It is not a promise that each chunk finishes in one conversation turn.
- Before implementation of a phase, turn its roadmap items into a reviewed design and exact tasks using the dependency APIs actually verified in the preceding trials.
- Split any chunk that combines unrelated decisions or cannot be reviewed independently. Do not bundle a UI replacement with database migration or a provider change.
- Each chunk records prerequisites, changed files, evidence, limitations, and the next task. Keep a status ledger in the repository so progress does not depend on chat memory.
- Status values: pending, active, blocked, verified, accepted. A compiled feature is not automatically a verified desktop experience.
- Preserve existing TypeScript BRN until a deliberate cutover. The `brn-rust` repository is separate; use disposable test data. No automatic conversion of the user's original vault.
- Run deterministic checks without credentials. Live provider checks are explicitly initiated and bounded. Do not collect production credentials in chat or commit them.
- GitHub should hold durable source and checkpoints; scratch workspaces are not the source of truth. Branches, commits, PRs, CI runs, and release publication follow the user's authorization and account access.

## Cloud and local execution

Cloud is suitable for source editing, backend builds, deterministic tests, indexing benchmarks on an approved sample corpus, database migration tests, and crash/recovery tests, once a Rust-capable environment is available.

The initial inspected chat workspace was Linux and lacked Rust and Cargo. Rust 1.98.1 is pinned for chunk 00, but build verification is recorded separately in [status.md](status.md). Network access is restricted. Running native macOS UI builds has not been demonstrated. Chunk 00 must establish the actual build route; if the current workspace cannot support it, use a configured cloud runner or local environment rather than claiming unexecuted checks passed.

The target desktop is needed early for editor selection, keyboard shortcuts, clipboard, input methods, font scaling, accessibility, file dialogs, credential storage, provider login, and actual perceived performance. Target-OS build runners can automate compilation and packaging, but do not replace human interaction testing. A Linux build does not verify a macOS or Windows app.

Local full development is optional if CI supplies test installers. If editing/building locally, install the selected Rust toolchain and OS build dependencies once. Credentials can remain local. The user's computer need not stay on for work executing on cloud compute. A local executor does need its host available while running tasks. This roadmap does not schedule unattended work between conversation turns.

## Phase A — remove architectural risks

### 00. Reproducible build route
Prerequisite: macOS Apple Silicon selected; identify available repository/runner access.
Deliverable: pinned Rust toolchain, minimal build probe, setup instructions, CI proposal or configuration when authorized.
Done: a clean environment builds and tests a tiny Rust target; target-OS compilation has an identified route. Record unavailable dependencies explicitly.
Where: cloud; target-OS runner or local machine for native build proof.

### 01. Provider and Rig trial
Prerequisite: 00; test Codex subscription access through Rig first.
Deliverable: disposable integration harness for login/configuration, streaming, one read-only tool, cancellation, and history replay.
Done: deterministic fake-provider tests pass, and an authorized live account check proves the required authentication route. Account access and token refresh limitations are recorded.
Where: cloud for fake-provider work; local or otherwise approved environment for login/live checks.

### 02. Desktop editor trial
Prerequisite: 00.
Deliverable: small runnable desktop trial: open a long Markdown document, select a passage, attach a comment, edit near it, display a revision diff.
Candidate: GPUI Kit for native Markdown editing/review. If formatted in-place editing is essential, compare an appropriate Rust editor before selecting the framework. Consider Loro only if stable positions/history justify its integration cost.
Done: user tries the selection/comment flow on the target OS; resizing, clipboard, Unicode, undo, and deleted anchor behavior are understood. Select one UI and document model.
Where: implementation in cloud where builds permit; early local interaction test required.

### 03. Retrieval adapter trial
Prerequisite: 00; approved sample documents.
Deliverable: common query/evidence types plus keyword and semantic/hybrid adapters; fixed comparison queries and expected sources. Trial LanceDB and FastEmbed; evaluate Swiftide only where it reduces ingestion work.
Done: changing a profile does not alter caller code; all methods return source/version/passage references and obey the same filters. Record quality, latency, disk use, and model download needs.
Where: cloud on sample data; local performance check later.

### 04. Architecture checkpoint
Prerequisite: 01–03.
Deliverable: concise design fixing the chosen UI, provider seam, document model, persistence responsibilities, and retrieval contract; dependency versions and licenses recorded.
Done: critical uncertainties resolved or explicitly deferred; user approves the concrete design before product implementation. Graph remains a required later capability, not a dependency blocking useful hybrid retrieval.

## Phase B — first usable desktop workflow

### 05. Desktop shell and application core
Prerequisite: 04.
Deliverable: one app window, navigation, typed commands/events, background task ownership, selected data directory.
Done: window opens on the target OS; UI stays responsive while a deterministic background operation runs. Closing during work has defined behavior.

### 06. Authoritative storage and recovery
Prerequisite: 05.
Deliverable: SQLite migrations, source/version records, session/message records, operation IDs and state transitions.
Done: acknowledged data survives restart; repeated request IDs do not start new work; uncertain in-flight work is marked interrupted after a crash; corrupt/newer databases are not silently replaced.

### 07. Import and version tracking
Prerequisite: 06.
Deliverable: Markdown/text import, stable IDs, source hashes, ingestion status, duplicate/change handling.
Done: importing twice does not duplicate unchanged content; changed input creates an identifiable version. Originals stay intact. Rich PDF/Office extraction is a later extension.

### 08. Index construction and management
Prerequisite: 03, 07.
Deliverable: background chunk/embed/index work, progress, cancellation, versioned index manifests, rebuild commands.
Done: interrupted construction cannot become the active index; documents added, changed, or withdrawn produce consistent searchable generations. Missing required indexes are reported, not silently substituted.

### 09. Search and profile switching
Prerequisite: 08.
Deliverable: keyword, semantic, and hybrid profiles; source previews; default and per-query selection.
Done: selection chooses the requested method, filters apply consistently, incompatible scores are merged using a defined policy, and evidence points to the correct source version.

### 10. Grounded chat
Prerequisite: 01, 06, 09.
Deliverable: streamed answers using retrieved evidence, clickable source references, usage where available, cancellation and session reopening.
Done: import → search → ask → inspect sources → close/reopen succeeds. Provider failure and client/UI detachment do not duplicate requests or falsely report success.
Milestone: first useful desktop build for personal trial.

## Phase C — writing and review

### 11. Drafts and immutable revisions
Prerequisite: 02, 06, 10.
Deliverable: create/edit draft, save checkpoints, reopen revisions, compare revisions with a reused diff library.
Done: saved text survives restart; AI results are separate candidate revisions; ongoing user edits are never overwritten silently.

### 12. Anchored comments
Prerequisite: 11.
Deliverable: select/comment, resolve/reopen, link to original revision and quote; map comments through supported edits.
Done: insertion, deletion, duplicate passages, Unicode, and undo behave predictably. Ambiguous or deleted anchors are displayed explicitly instead of attaching to the wrong passage.

### 13. Revision from a comment batch
Prerequisite: 12.
Deliverable: freeze source revision and selected comments, generate a candidate, compare changes, preserve evidence and batch identity.
Done: edits during generation are retained; interrupted work is recoverable; generated changes do not automatically count as user approval or semantically completed comments.

### 14. Approval and publication
Prerequisite: 13.
Deliverable: approve exact bytes/version and destination; recoverable Markdown publication with conflict detection.
Done: destination changes are detected; injected failures do not lose the old version or claim a partial publication succeeded. Publishing remains outside the model's tool registry.
Milestone: complete import → grounded draft → comments → revision → approval workflow.

## Phase D — graph profile and dependable release

### 15. Graph engine qualification
Prerequisite: 03, 09; can begin after Phase B without blocking Phase C.
Deliverable: compare Cognee-RS and, if useful, GraphRAG-rs on the shared evidence contract and fixed query set.
Done: meaningful entity/relationship extraction, provenance, source filtering, source deletion/version updates, indexing cost, and native packaging are demonstrated. A demo using fallback embeddings is not sufficient evidence.

### 16. Graph profile integration
Prerequisite: 15 and selected engine.
Deliverable: graph and optionally graph-hybrid profiles behind BRN's retrieval boundary; index build status and explicit selection.
Done: switching profiles preserves UI and agent interfaces; evidence includes supporting passages and graph provenance. Automatic routing cannot override explicit user selection. Original sources remain authoritative.

### 17. Backup and restore
Prerequisite: 14; include graph metadata if 16 is complete.
Deliverable: consistent backup, restore rehearsal, portability and schema-upgrade checks. Indexes may rebuild; authoritative drafts/comments/approvals must restore.
Done: restore to a clean directory preserves user work and evidence links. Upgrade failure has a documented recovery path.

### 18. Packaging and target-OS acceptance
Prerequisite: 10 for early test installers; 14, 16, 17 for full planned release.
Deliverable: installer/application bundle, model resource handling, credential integration, installation/update instructions, target-OS smoke checks.
Done: install on a clean target machine and complete the full workflow; no development toolchain or Node/Python service required at runtime. Validate graphics, input, offline reopening, sleep/wake, and uninstall data policy. Signing/notarization, if required for distribution, uses the user's release credentials through the appropriate secure workflow.

## Scope and sequencing

Critical path: 00 → trials 01/02/03 → 04 → 05/06 → 07/08/09 → 10 → 11/12/13/14 → 17/18. Graph qualification/integration 15/16 follows the retrieval contract and joins before the full planned release. Installer work starts early enough to supply desktop trials.

Useful delivery checkpoints: editor trial at 02; retrieval trial at 03; first useful app at 10; writing/review app at 14; full modular graph-enabled release at 18. Graph is part of the intended scope even though an earlier usable build ships without it.

Defer multi-user collaboration, sync, plugin marketplaces, generic agent orchestration, mobile clients, and multiple OS targets until the initial desktop workflow is reliable. Reconsider richer document formats separately.

## Immediate next step

Verify chunk 00's Rust build route, then execute only Phase A initially, beginning with Codex/Rig access and the target-OS editor trial. Do not install every candidate framework in the main app before the trials. Re-estimate scope after Phase A using actual build and integration evidence rather than a guessed total duration.

## References

- Existing BRN: https://github.com/ewq100/brn
- Rig: https://github.com/0xPlaygrounds/rig
- GPUI Kit: https://github.com/longbridge/gpui-kit
- Swiftide: https://github.com/bosun-ai/swiftide
- LanceDB Rust: https://docs.rs/lancedb/latest/lancedb/
- FastEmbed: https://github.com/anush008/fastembed-rs
- Loro: https://github.com/loro-dev/loro
- Cognee-RS: https://github.com/topoteretes/cognee-rs
- GraphRAG-rs: https://github.com/automataIA/graphrag-rs
- Cloud environment configuration: https://learn.chatgpt.com/docs/environments/cloud-environment

Dependency choices remain candidates until the trials pass. Prior research established documented capabilities, not successful integration or measured quality on the user's corpus.
