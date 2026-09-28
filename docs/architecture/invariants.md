# Architectural invariants

These constraints summarize the [approved baseline](decisions/2026-09-28-architecture-baseline.md) and subsequent draft/comment contracts. Preserve them when modifying code. Future publication requirements are marked explicitly; their presence here does not mean publication is implemented.

## Authority and exact content

- SQLite owns durable sources, versions, drafts, comments, application sessions and operation records. Indexes and embeddings are derived and rebuildable.
- Acknowledge durable writes only after commit. Corrupt or newer schemas fail visibly; never silently replace a workspace.
- Identity is an opaque ID, not a path or hash. Immutable revisions preserve exact UTF-8 bytes and SHA-256, without silent Unicode or line-ending normalization.
- Evidence and original anchors retain revision identity, hash, exact quote and half-open UTF-8 byte ranges on character boundaries. Imported originals remain unchanged.

## Draft and comment safety

- Mutable working copies, immutable checkpoints and AI candidates are distinct. A generated candidate does not replace current edits or imply approval.
- Correlate asynchronous results with their operation, revision and working-copy generation. Later edits must survive stale saves and responses.
- Preserve original comment evidence permanently. Supported edits can map anchors conservatively; deleted, touched or ambiguous locations must remain explicit. Duplicate text is not proof of identity.
- Anchor mapping and resolve/reopen lifecycle are separate. Undo recovery requires exact validated content and ranges.

## Retrieval and provider boundaries

- Apply version/approval eligibility consistently across search profiles and revalidate evidence against authoritative state. An old index generation cannot grant current eligibility.
- Publish only validated completed index generations. Missing requested native resources report unavailable; do not silently substitute keyword search.
- Provider credentials and authoritative provider conversation history remain provider-owned. Do not inspect, copy or log credential caches or raw authentication payloads.
- Supervise and reap only the app-owned sidecar. Preserve unknown/interrupted outcomes; transport loss is not proof of cancellation.
- Operation IDs bind payload identity. Conflicting reuse fails; uncertain external execution is not silently replayed or described as exactly once.

## Future publication contract

Search approval does not authorize publication. When publication is implemented, approval must bind exact revision bytes/hash and destination, and publication must revalidate those preconditions immediately before a recoverable write. The model must not gain a publication tool.

## Scope and qualification

Use explicit disposable directories and synthetic fixtures for verification. Keep the original vault separate. Automated tests, agent-observed native interactions, subjective user acceptance and release qualification are distinct evidence categories.
