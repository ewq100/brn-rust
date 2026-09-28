# Delivery roadmap

This document defines future outcomes and prerequisites. It is not an approved implementation plan or authorization to start work. [Current status](status.md) owns implementation, verification and integration state; the [original annotated roadmap](work/completed/early-checkpoints/roadmap-history.md) preserves earlier milestone specifications and history.

## Outcome and constraints

Build a local-first, single-user Rust desktop application for importing knowledge, asking grounded questions, drafting, reviewing comments/revisions and approving exact publication content. Start with macOS on Apple Silicon. Keep the original TypeScript application and vault separate. Runtime application services remain Rust; native libraries may be dependencies.

## Existing foundations

Milestones 00–12 cover build/provider/editor/retrieval trials, the accepted architecture baseline, desktop/storage, import/search/chat, drafts and anchored comments. Their bounded implementations and observations are indexed in [completed work](work/completed/README.md). This does not close broader corpus-quality, user acceptance or release qualification gaps.

## Next writing and review outcomes

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


## Starting the next task

The next sequential outcome is milestone 13. Define its concrete design, frozen batch identity, candidate behavior and failure/recovery acceptance before implementation. Follow the [development workflow](development/workflow.md); update current status as work proceeds. Candidate adoption must be explicitly scoped rather than inferred from retaining an AI candidate.
