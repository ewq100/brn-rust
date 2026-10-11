# BRN Threads architecture overview

The [canonical Threads target](threads-target.md) owns the detailed architecture. This page is a navigation summary. The [behavior design](threads-behavior.md) supplies concrete thread and packaged runtime-guide defaults. The SQLite Threads core, runtime and native workspace are implemented. Phase 1 hardening, including remembered backup folder, merged through PR #120 at `3a3aa7a`; the current assignment is [pre-release cleanup](../work/active/pre-release-cleanup/plan.md). See [status](../status.md) for verification and acceptance limits.

## One state owner

SQLite owns current records, immutable meaningful revisions, threads, Actions, source observations, change receipts, authorization, and run progress. Small retained note assets may live in the same database. Search indexes are derived and rebuildable.

```mermaid
flowchart TD
    UI["Desktop and CLI"] --> CORE["Shared application service"]
    AGENT["Agent runtime"] --> CORE
    INPUT["External sources"] --> AGENT
    CORE --> DB["SQLite state and history"]
    DB --> SEARCH["Derived search index"]
    SEARCH --> CORE
    DB --> EXPORT["Exports and backups"]
```

One checked change mechanism handles direct owner Save, delegated AI maintenance, reviewed candidates, and Undo. Record versions cover relevant metadata and protection as well as content. Canonical commits are serialized initially; model calls and external processing stay outside transactions.

## Recovery and interaction

Undo records a new compensating operation. Immediate grouped Undo is required. Later conflicting edits can require a reviewed compensation; no whole-database rewind or automatic semantic merge is promised.

Defer autonomous writes to a note with unsaved owner writing. Retain meaningful revisions and comment context; unresolved anchors remain visible. Start with ordinary editor tracking and current/proposed comparison. Add a CRDT or editor fork only after evidence of a concrete need.

## Reuse boundary

Reuse useful GPUI controls, provider integrations, maintained decoders, and retrieval capabilities. The existing crate map is a source of components, not a frozen architecture. The new core replaced mutable-vault coordination, blanket proposal gating, original-copy recovery, temporary comments, and old-state readers.

Full-note import shares the same notes, assets, source references, and protection model as other work. Qualify converter and reader coverage without building a universal source renderer.

## Workspace ownership

| Crate | Responsibility |
|---|---|
| `brn-threads-core` | Authoritative SQLite state, checked changes, authorization, replay, history, Undo and backup/restore |
| `brn-threads-app` | Shared application operations, run coordination, fresh current-note search and publication |
| `brn-threads-intake` | Transient DOCX/PDF conversion, import intent, source coverage and necessary assets |
| `brn-intake` | Bounded extraction protocol and maintained restricted converter helper |
| `brn-ai` | Provider/auth transport, bounded Threads runtime/tools and packaged guidance |
| `brn-retrieval` | Derived keyword/vector indexes and optional explicit local embedding support |
| `brn-desktop` | Headless desktop entry and optional GPUI workspace/state |
| `brn` | Thin CLI over the shared service |

Read the [invariants](invariants.md), [dependency boundaries](dependencies.md) and [verification guide](../development/verification.md) before changing these seams. [Archived reports](../archive/README.md) retain implementation history without assigning old work.
