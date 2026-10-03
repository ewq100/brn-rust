# BRN Rust --- Rig-First Architecture Reset

**Date:** 2026-10-01\
**Status:** Historical design handoff, superseded by the simple Rig-based notes design

> Preserved during the 2026-10-03 local-document synchronization. This is the
> original 2026-10-01 planning input, not current execution instructions.
> Use the [simple notes roadmap](../work/active/simple-rig-notes/plan.md) and
> [current status](../status.md) for implemented contracts and remaining work.
> Step 4 Rig chat is merged through PR #14; this handoff does not authorize
> another reset, data migration or live/provider checks.

## Decision

BRN is not in production use. There is no installed-base, user-data, or
provider-history compatibility requirement. Do not migrate the existing
Codex App Server architecture. Replace it cleanly.

> **BRN owns product semantics, the current vault, durable user work,
> provenance, permissions, and safe file operations. Rig owns generic AI
> infrastructure.**

Prefer Rig for providers, agents, streaming, tools, hooks, memory,
structured output, embeddings, vector stores, reranking, MCP, telemetry,
and AI test infrastructure. Add custom BRN abstractions only for
BRN-specific semantics.

## First-class providers

Support both immediately:

1.  **ChatGPT/Codex subscription**
2.  **GitHub Copilot subscription**

They must be explicit selectable providers. Never silently fall back
between providers, accounts, subscriptions, or paid API routes.

At design time Rig 0.43.0 is the reference. The planning/implementation
agent must inspect and pin the selected current Rig release and its
exact ChatGPT/Codex and Copilot APIs.

Before production refactoring, create a small qualification harness
proving for both subscriptions: authentication, authenticated
completion, streaming, multi-turn use, cancellation/interruption, tool
calling, structured output, hooks, credential reuse after restart,
reconnect behavior, and cassette/deterministic tests. Resolve the
combined Rust dependency graph before the main rewrite.

## Remove the old provider architecture

Delete/retire the current `brn-provider` Codex App Server transport,
JSON protocol, Codex executable and `codex_home` configuration, App
Server version checks, Codex-specific process supervision, thread
start/resume, account/store identity plumbing, sidecar lifecycle, App
Server-specific production tests, and packaging assumptions.

Historical experiments may remain clearly historical.

Create:

``` text
crates/brn-ai
```

`brn-ai` is a thin Rig integration boundary, not a second AI framework.

## Target architecture

``` text
brn-desktop ─┐
             ├──> brn-workflow ───> brn-ai ───> Rig ───> selected provider
brn CLI ─────┘          │
                        ├──> brn-store
                        └──> brn-retrieval ───> SQLite
                                               ├── FTS5
                                               └── sqlite-vec
```

Desktop and CLI use the same workflow. UI code never directly calls
providers, retrieval databases, or filesystem mutation paths.

## No centralized Context Engine

Do **not** build the previously discussed Context Planner/Context Engine
unless later evidence demonstrates a concrete need.

Normal flow:

``` text
User
 ↓
Rig Agent
 ├── conversation/memory
 ├── skills/instructions
 ├── hooks
 └── BRN tools
      ├── search_vault
      ├── read_note
      ├── list_notes
      └── comments/revisions
```

The main LLM receives normal conversational context. It can understand
follow-ups such as "What if we do that after the editor?" and decides
when it needs vault information.

Use Rig memory/context facilities before inventing custom
summarization/planning infrastructure.

### Deterministic boundary

The model may decide **what it wants to search/read**. Deterministic BRN
code decides **what a tool can return or mutate**.

-   `search_vault` searches only active/current knowledge.
-   `read_note` reads only active/current notes in normal mode.
-   `save_note` enforces BRN safe-write/conflict/recovery rules.
-   archive/history access uses separate explicit capabilities.

Prompt instructions are not enforcement boundaries.

## Vault, archive, and history

Keep the main vault current:

``` text
VAULT
Current source of truth.
Normal AI search/read tools can access it.

ARCHIVE
Retired/obsolete documents.
Normal AI tools cannot access it.

HISTORY
Previous revisions of current documents.
Not part of normal retrieval.
Access only through explicit historical capabilities.
```

If `architecture.md` evolves v1 → v2 → v3, the active vault contains
only current `architecture.md`. Do not preserve old revisions as
separate active searchable notes.

Revision history and archive are different. Archive a document when the
document itself is no longer current. Normal indexes must exclude
archive content.

Historical tools such as `search_archive` or `get_note_history` can be
added explicitly for questions about evolution/history; they should not
participate in ordinary retrieval by default.

## Markdown and SQLite authority

Continue the Markdown-first direction:

-   current Markdown file = authority for saved note content
-   SQLite = application metadata, operations, recovery, conversations,
    permissions, and derived indexes
-   retrieval/vector indexes = rebuildable derived data
-   Rig memory, summaries, embeddings, and vector rows are never
    authoritative note content

## SQLite retrieval from the start

Use SQLite as the unified local data technology:

-   application tables for BRN state
-   **FTS5/BM25** for lexical retrieval
-   **sqlite-vec**, preferably through Rig's SQLite vector integration,
    for semantic retrieval

``` text
Query
 ├──> SQLite FTS5/BM25 ─┐
 └──> embedding ─> sqlite-vec ─┤
                               ├──> fusion ─> optional rerank ─> evidence
```

Do not preserve LanceDB by default. Remove it if SQLite + sqlite-vec
meets correctness and practical performance requirements for a personal
local vault. Reintroduce a separate vector DB only if benchmarks
demonstrate a real need.

`rig-sqlite` is a vector integration, not a replacement for `brn-store`.

## Embeddings

Use Rig's embedding abstraction and prefer a stable **local embedding
model**.

Benefits: offline search, no indexing API cost, vault content need not
leave the Mac for indexing, and changing conversational provider does
not rebuild the semantic index.

``` text
Conversational LLM:
- ChatGPT/Codex subscription
- GitHub Copilot subscription
- future Rig providers

Semantic index:
- one stable local embedding model
```

Use Rig's local/FastEmbed integration if compatible with the pinned
release. Do not keep custom FastEmbed machinery merely because it
exists. If dependency conflicts remain, use the thinnest possible
adapter implementing Rig's embedding interface.

## Retrieval behavior

Replace current custom substring lexical scoring with FTS5/BM25.
Semantic retrieval uses Rig embeddings + sqlite-vec. Hybrid retrieval
combines both.

A small BRN-specific fusion layer is acceptable; start with
reciprocal-rank fusion unless representative tests justify something
else. Evaluate Rig reranking only if it materially improves quality.

Every passage retains source/note and passage provenance. Semantic
retrieval cannot bypass active-vault rules.

## Conversations and memory

Redesign conversations from scratch; do not preserve Codex App Server
thread compatibility.

BRN owns durable conversation identity. Rig owns generic message/model
execution and memory/context mechanics.

Persist enough for restart, multi-turn continuation, provider/model per
turn, tool calls/results where needed, evidence/source references, usage
where available, and execution outcome.

Provider-native thread IDs are not BRN conversation identity. Provider
choice is explicit. Use Rig memory before building custom history
truncation/summarization/retrieval.

## Skills, tools, hooks

Use the Rig agent directly.

**Skills/instructions** teach the model how BRN works and how to use
tools. They are guidance, not permission enforcement.

Initial narrow tools:

``` text
search_vault
read_note
list_notes
list_comments
read_comment
create_revision_candidate
```

Do not expose broad escape hatches such as arbitrary shell, raw SQL,
arbitrary file writes, or running the CLI as a model-controlled bypass.
Rig tools call shared BRN workflows.

Use **Rig hooks** for run/operation tracking, cancellation, telemetry,
tool policy, request/result observation, safe retry policy, and
run-scoped state. Never automatically retry a potentially effectful
operation when its previous outcome is uncertain.

## Structured output and AI writing

Use Rig structured output/extraction for generated artifacts. Example:

``` text
RevisionProposal
├── base_revision
├── addressed_comments[]
├── edits[]
│   ├── target/range
│   ├── replacement
│   └── rationale
└── unresolved_comments[]
```

Rig/model generates candidates. BRN validates them against
frozen/current state and stores them separately. The user decides
whether to adopt them. AI output never automatically overwrites current
notes or implies approval/publication.

## Safe Markdown editing stays BRN-specific

Rig does not replace BRN's safe editing protocol. BRN remains
responsible for exact bytes, editing buffers, conflict detection, safe
writes, recovery, operation identity, external-editor conflicts,
explicit save semantics, and note identity.

AI-requested mutations go through the same workflow rules as desktop/CLI
actions.

## Provider/auth configuration

Replace Codex-specific config with:

``` text
AIConfig
├── provider: ChatGPT/Codex | GitHub Copilot
├── model
├── authentication/account reference
├── embedding configuration
└── runtime policy
```

Require explicit connect/disconnect, visible provider/account identity,
safe credential reuse/refresh, reconnect-required state, no raw secret
logging, no silent fallback, and no silent account switching.

Inspect current Rig ChatGPT and Copilot authentication implementations
before deciding credential persistence. Do not invent authentication Rig
already supplies.

## Keep `brn-ai` thin

Good:

``` text
brn-workflow → brn-ai → Rig → provider
```

Avoid:

``` text
brn-workflow → ProviderPort → ProviderManager → CompletionService → RigAdapter → Rig
```

Repository rule:

> **Before creating generic AI infrastructure, inspect the pinned Rig
> release and companion crates. Prefer Rig unless BRN has a concrete
> product-specific requirement Rig does not satisfy.**

## Rig capability audit

Planning must inspect the pinned release and classify capabilities.
Initial direction:

  Capability                       Direction
  -------------------------------- -----------------------------
  Provider/model abstraction       Use now
  ChatGPT/Codex subscription       Use now
  GitHub Copilot subscription      Use now
  Agent, streaming, hooks, tools   Use now
  Structured output                Use now
  Memory/messages                  Use now
  Embeddings                       Use now
  SQLite vector integration        Use now
  Reranking                        Evaluate
  Cassette/testing                 Use now
  Telemetry                        Use now
  MCP                              Later unless required
  PDF/EPUB, audio, image           Later
  Graph integrations               Graph phase
  ECS runtime                      Only with demonstrated need

## Implementation sequence

Use small reviewable PRs:

1.  **PR A --- Architecture reset:** update architecture/docs; Rig-first
    ownership; vault/archive/history; SQLite direction; old App Server
    architecture historical.
2.  **PR B --- Rig qualification:** pin Rig; prove ChatGPT/Codex +
    Copilot subscriptions; streaming/tools/hooks/structured output; auth
    reuse; deterministic/cassette tests; dependency resolution. **Both
    subscriptions are a gate.**
3.  **PR C --- `brn-ai`:** thin crate, provider/account config, both
    providers, model selection, streaming mapping.
4.  **PR D --- Conversations + Rig memory:** new durable BRN
    conversation identity, restart continuation,
    provider/model/tool/evidence metadata.
5.  **PR E --- Replace ask path:** Rig agent, safe vault tools, preserve
    BRN operation/error honesty, remove Codex-thread assumptions.
6.  **PR F --- Remove App Server:** delete old
    provider/config/sidecar/dependencies/tests.
7.  **PR G --- FTS5:** BM25 lexical search, current-vault-only indexing,
    archive exclusion/provenance tests.
8.  **PR H --- Local embeddings + sqlite-vec:** Rig embedding
    abstraction, local model, semantic index, benchmarks; remove LanceDB
    if accepted.
9.  **PR I --- Hybrid retrieval:** fusion, representative quality tests,
    optional reranker experiment.
10. **PR J --- Tools + structured output:** formal tool registry and
    typed revision-candidate generation.
11. **PR K --- Observability/testing:** Rig telemetry/cassettes,
    cancellation/interruption, provider parity, secret-redaction tests.
12. **PR L --- Cleanup:** dead code/dependencies, inspect `brn-core`,
    simplify crates, update docs, full verification, size/dependency
    report.

### Parallelization

After PR B, PR C and PR G can run in parallel. PR D and PR H can also
proceed largely independently. PR E depends on C plus core D work; F
follows E; I depends on G+H; J follows stable agent/conversation/tool
boundaries; L is last.

## Acceptance gates

The reset is complete only when:

-   ChatGPT/Codex subscription works end-to-end through Rig.
-   GitHub Copilot subscription works end-to-end through Rig.
-   explicit provider selection works with no silent fallback.
-   restart continuation works.
-   streaming/cancellation outcomes are represented honestly.
-   active vault search cannot return archive/history.
-   FTS5 lexical search and sqlite-vec semantic search work from the
    same SQLite-based architecture.
-   local embeddings are rebuildable.
-   hybrid retrieval preserves provenance.
-   safe Markdown writes remain behind BRN workflow rules.
-   AI-generated revisions remain separate candidates.
-   deterministic/cassette tests cover both provider paths.
-   obsolete App Server and unnecessary LanceDB/custom AI infrastructure
    are removed.
-   architecture/status/roadmap documentation describes the new system.

## Planning-agent instruction

Turn this design into a concrete implementation plan against the current
repository. Inspect the repository and the pinned/current Rig source
before choosing exact APIs. Prefer deletion/replacement over
compatibility shims. Identify dependency/version conflicts early. Break
work into reviewable PRs with exact files, tests, commands, acceptance
gates, and dependencies. Do not implement product code until the
resulting plan has been reviewed.
