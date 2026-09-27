# Architecture checkpoint — proposed decision

27 September 2026 · Roadmap 04 · **For user review; not approved**

[Observed dependency versions and licenses](architecture-dependencies.md).

## Recommendation and evidence

Build a local-first, single-user Rust application for Apple Silicon macOS: import knowledge, retrieve attributable passages, draft, review anchored comments and revisions, then approve exact publication content. Preserve the existing TypeScript BRN and original vault. Recommend a thin GPUI view over an independent application core, immutable UTF-8 revisions in SQLite, a supervised Codex App Server adapter, and replaceable local retrieval engines.

This selects boundaries, not trial code for wholesale promotion. Direct Rig subscription integration was not demonstrated; App Server demonstrated the required route. A CRDT/Loro model adds complexity without a collaboration requirement; conservative revision anchors are preferable initially. Another native editor remains an alternative if GPUI fails interaction acceptance.

Observed evidence:

- [Provider lifecycle trial](../experiments/codex-app-server/CHUNK-02.md): managed ChatGPT streaming, read-only tool execution, interruption, separate-process thread resume and proactive refresh; 16 deterministic tests. Natural expiry/revocation and force-crash cleanup remain unverified.
- [Editor trial](../experiments/editor-trial/EVIDENCE.md): native build and 11 model tests; repeated native inspection attempts were blocked by the locked Mac. Rendering, actual input and user acceptance are **not verified**.
- [Retrieval trial](retrieval-trial-evidence.md): keyword, semantic and hybrid profiles share filters/provenance; local reopening and corruption rejection passed on seven synthetic passages. This establishes feasibility, not corpus relevance, production chunking or desktop performance.

## Proposed ownership and UI seam

`desktop` owns GPUI views, focus, selection and transient view state. `core` owns typed commands/events and workflow rules; `store`, `provider` and `retrieval` implement narrow ports. Views send commands with operation IDs, expected revision IDs and a monotonically increasing working-copy generation. Every edit, including undo/redo, advances that generation; responses echo it. Apply a result to current text or selection only when both revision and generation match. Stale AI results may remain separate candidates, but cannot replace the working copy or current selection. UI code never directly writes SQLite, calls a model or builds an index.

GPUI Kit 0.6.6 remains provisional until the editor checklist and user trial pass, or the user explicitly defers that gate. The proposed editing experience is Markdown source with separate diff/review, not formatted in-place editing. Core types contain no GPUI objects; replacing the editor should preserve stored revisions and workflow rules.

## Documents, anchors and authority

SQLite is authoritative for BRN sources, immutable versions/revisions, draft checkpoints, comments, approval records, evidence used by BRN, session associations and operation state. A single storage owner serializes transactions; acknowledge durable writes only after commit. Chunk 06 defines migrations and crash qualification; corrupt or newer schemas fail visibly without replacement.

Use opaque UUIDs for sources, revisions, comments, sessions and operations. Identity is not a path or hash. Each immutable revision stores its parent, exact UTF-8 bytes and SHA-256; do not silently normalize Unicode or line endings. Passage identity includes revision ID, versioned chunker identity and half-open UTF-8 byte range. Evidence also records content hash and exact quote; all offsets must be character boundaries. Imported originals remain unchanged.

A comment retains original revision, range, quote and hash, plus mapping state for each reviewed revision. Safe non-overlapping edits may shift ranges; touched, deleted or ambiguous passages become explicitly unresolved. Undo restores an anchor only when exact revision content and range validate. Preserve original quote access; never guess from duplicate text. Trial full-buffer snapshots are not the production persistence design.

User edits remain a working copy; checkpoints create revisions. AI results create separate candidates tied to frozen source revision and comment batch. Approval binds exact revision bytes/hash and destination; later edits require new approval. Publication revalidates approval, current revision and destination preconditions immediately before its recoverable write. The model has no publication tool.

## Provider ownership

One asynchronous provider actor owns one BRN-launched App Server child, initialization, protocol correlation, bounded transport and shutdown. First integration permits one active turn per session. The UI receives structured streaming/progress/terminal events; raw stderr, tokens and raw authentication payloads are not logged.

Codex owns credentials, managed refresh and authoritative provider conversation history. BRN stores provider/store association, thread/turn IDs and application records, with any displayed transcript clearly a local projection. Do not duplicate, inspect or migrate credential caches. Missing history or changed store/account association requires explicit recovery; never silently start a replacement thread or replay a possibly accepted turn.

Resolve an explicitly selected absolute executable path and validate supported version/schema before readiness. The observed ChatGPT bundle path is not a distribution contract. Sidecar bundling, login UI, signing and credential integration require later qualification under [packaging boundaries](../experiments/codex-app-server/PACKAGING.md).

## Retrieval contract and lifecycle

Use `SearchRequest(query, profile, limit, filters)` and `Evidence(source, revision, passage, hash, byte_range, quote, score_kind, score, generation)`. Defaults select current approved versions; historical/draft/withdrawn selection is explicit. Every adapter applies the same eligibility predicate. Core revalidates returned evidence and current/approval state against authoritative storage before displaying current results or passing context to a model; a stale generation cannot grant eligibility. Record the evidence snapshot used by each answer.

Keyword runs without embeddings. Semantic/hybrid initially use the trial's FastEmbed/LanceDB boundary; keep loaded resources in an owned worker rather than reload on every query. RRF combines ranks, not incomparable raw scores. Production lexical ranking, chunking, candidate limits and relevance thresholds require representative-corpus qualification before chunk 09 acceptance.

Indexes, embeddings and future graphs are derived, rebuildable generations. Manifests record schema/chunker/model/runtime identities and hashes. Build in a new generation; validate and durably publish before atomically changing the active pointer. Cancellation/incomplete construction never activates it. Missing or incompatible requested resources report unavailable; never silently substitute profiles. Source withdrawal/version changes invalidate eligibility immediately, even before rebuilding.

Graph remains required later: its adapter must return supporting passage evidence plus graph provenance and obey identical filters, deletion and version rules. Engine selection waits for roadmap 15; graph does not block initial hybrid use.

## Concurrency, recovery and first production boundary

Use bounded command/event queues, one storage actor, one provider actor and limited retrieval/blocking workers. Coalesce progress; preserve terminal events. Cancellation is cooperative for local work and explicitly requested for provider turns; transport loss is an unknown outcome, not cancellation proof. Shutdown stops submissions, requests cancellation, drains within a deadline, then terminates/reaps the owned child.

Durable operation IDs bind a command payload hash: duplicates return existing state; conflicting reuse fails. Persist pending before external submission. After restart, reconcile when possible; otherwise mark interrupted/unknown and require explicit retry. Never claim exactly-once external execution.

Chunk 05 implements only one window, navigation, typed core commands/events, a selected data directory and a deterministic cancellable background task. No live provider, retrieval integration or authoritative document storage. Acceptance requires observed native launch/responsiveness, cancellation, stale-result rejection and bounded close-during-work behavior; deterministic queue/lifecycle tests complement native checks.

## Approval gate

Roadmap 04 remains open until native editor acceptance **or explicit user deferral**, concrete design approval, and dependency/version/license recording are complete. Approval accepts these boundaries and permits the bounded chunk 05 shell work under the existing authorization to plan, delegate, implement, test and push trial changes. Before coding, record and review the exact implementation plan; routine execution does not need a second permission round. It does not accept unobserved trials or authorize release, merge, account changes or vault migration. Deferring native acceptance preserves GPUI's provisional status and the native gate before chunk 05 acceptance.

## Preparation and review record

Prepared on `trial/architecture-checkpoint` from remote-verified retrieval checkpoint `0e70a90`. Astra drafted the proposal; a separate Astra reviewer checked it against trial evidence and found one important gap in unsaved-edit correlation. The revision-plus-working-copy-generation rule above addresses it, and scoped re-review found no remaining Critical or Important issues. This is approval **for user review**, not user acceptance.

The lead checked local Markdown links, dependency metadata, secret patterns and `git diff --check`. Only documentation changed, so build/test suites were not rerun for this proposal; prior executable verification remains linked above. Computer-use again reported the Mac locked on 2026-09-27. No native interaction result or architecture acceptance is claimed.
