# Architectural invariants

These constraints summarize the [approved baseline](decisions/2026-09-28-architecture-baseline.md) and subsequent draft/comment/managed-note contracts. Preserve them when modifying code. Future publication requirements are marked explicitly; their presence here does not mean publication is implemented.

> **Transition:** the [simple Rig-based notes app](../superpowers/specs/2026-10-02-simple-rig-notes-design.md) replaces these invariants step by step ([roadmap](../work/active/simple-rig-notes/plan.md)). New code in `brn_store::work` and `brn_workflow::vault` follows that specification; the invariants below still describe the existing code until the cleanup step removes it.

## Authority and exact content

- SQLite owns durable imported sources, versions, drafts, comments, application sessions and operation records. For managed notes, the live registered Markdown file owns current saved content; SQLite owns registry/permission and unfinished work, not a competing saved-content authority. Search snapshots, indexes and embeddings are derived and rebuildable.
- Acknowledge durable writes only after commit. Corrupt or newer schemas fail visibly; never silently replace a workspace.
- Identity is an opaque ID, not a path or hash. Immutable revisions preserve exact UTF-8 bytes and SHA-256, without silent Unicode or line-ending normalization.
- Evidence and original anchors retain revision identity, hash, exact quote and half-open UTF-8 byte ranges on character boundaries. Imported originals remain unchanged.

## Managed Markdown note safety

- Explicit Save/Cmd-S writes Markdown; automatic buffer recovery only commits exact unfinished work to SQLite. Preserve UTF-8 bytes, BOM, line endings and frontmatter unless deliberately edited. Empty files are valid; note/submission limits are 1 MiB in bytes.
- Keep editing baseline/generation, fresh observed file state and submitted save generation distinct. Acknowledgements never erase later typing or lower generations. Equal-generation submissions require identical bytes; stale/conflicting preconditions fail.
- Resolve paths through validated directory descriptors. Require supported local regular single-link files, containment, root identity and exclusive vault ownership, including aliases/overlapping roots. No symlink/hard-link or unsupported-operation fallback.
- Save commits intent before exclusive same-filesystem staging, coordinated revalidation, preserving atomic exchange, required durability, installed/displaced verification and receipt. Missing originals are not recreated. Copies reserve independent identity/destination and use exclusive installation, never overwrite.
- Required file `F_FULLFSYNC`, directory `libc::fsync`, coordination, exchange, exclusive-create or attribute failures remain explicit. Never weaken writes. Participation in coordination does not prove lossless arbitrary simultaneous editing; preserve unexpected displaced objects without blind rollback.
- Bound operation replay precedes fresh-state validation and never repeats filesystem writes. Startup Interrupted is not a file outcome; dedicated reconciliation requires exact artifact/identity proof, not matching content alone.
- Compare is read-only. Reload/discard and relink require explicit confirmation and current preconditions. Accept-current is a separate acknowledgement retaining the original failure/recovery; save-copy does not resolve an uncertain original.
- Keep one rolling buffer and the most-recent Applied-save recovery pair. No-op/NotApplied does not refresh that pair. Unresolved work never expires automatically; cleanup requires known resolution, identity-bound artifact proof and durable recovery. Preserve compact receipts, historical provenance and unresolved dependencies.
- Presenter notices are hints for worker observation, not content/eligibility proof. Admitted critical note jobs drain/join on shutdown. Guarded close/quit/switch routes flush recovery asynchronously; Dock/system termination cannot be vetoed by pinned GPUI and submits no final-hook flush. Unacknowledged typing may be lost.

## Draft and comment safety

- Mutable working copies, immutable checkpoints and AI candidates are distinct. A generated candidate does not replace current edits or imply approval.
- Correlate asynchronous results with their operation, revision and working-copy generation. Later edits must survive stale saves and responses.
- Preserve original comment evidence permanently. Supported edits can map anchors conservatively; deleted, touched or ambiguous locations must remain explicit. Duplicate text is not proof of identity.
- Anchor mapping and resolve/reopen lifecycle are separate. Undo recovery requires exact validated content and ranges.

## Retrieval and provider boundaries

- Apply version/approval eligibility consistently across search profiles and revalidate evidence against authoritative state. An old index generation cannot grant current eligibility.
- Managed-note eligibility requires a supported existing file at its reconciled location, no unresolved original save, and fresh exact filesystem/content identity matching the approved snapshot. Dirty buffers, artifacts, shadowed imports and unapproved copies cannot become current evidence. Revalidate at index publication, search return and provider submission/completion.
- Changed content withdraws version-bound search permission; verified unchanged saves preserve it. Explicit reload/relink/accept-current reset permission even for equal bytes. Unavailability excludes a source without treating stored approval as live eligibility. Current surfaces expose exclusions, not old bytes as current; whole-index staleness remains possible.
- Legacy AI is retired: all retained Workspace ask entries return typed
  LegacyAiRetired before storage/network mutation or callbacks, even for UUID
  replay. Legacy history wire fields stay readable; never translate old thread
  IDs into Rig resume requests.
- Simple AppWorker refreshes before new Ask, validates current read evidence,
  freezes provider/model and resolves UUID replay before vault/auth/selection
  gates. Running never resubmits. Finished follows durable finalization;
  PersistenceFailed is visibly unsaved in-memory partial text.
- Publish only validated completed index generations. Missing requested native resources report unavailable; do not silently substitute keyword search.
- Simple credentials are protected owner-only files outside Git/data/vault.
  AppConfig None honors the saved explicit location, then the exact
  `<data-name>.credentials` sibling. Only Connect starts login; no fallback.
  Device codes exist only on the transient login surface, never SQLite/logs.
- AppWorker owns and joins chat/account/install/read leases before releasing
  authority. Disconnect fences/cancels/joins only the target provider before
  deleting its caches. No production sidecar exists. Transport loss/Stop never
  establishes upstream cancellation or no billing.
- Startup/model activation index only with both a loaded model and available
  vault. Download needs fresh consent; saved approval never starts a request.
  Default builds are explicitly keyword-only and refuse model installation.
- Operation IDs bind payload identity. Conflicting reuse fails; uncertain external execution is not silently replayed or described as exactly once.

## Future publication contract

Search approval does not authorize publication. When publication is implemented, approval must bind exact revision bytes/hash and destination, and publication must revalidate those preconditions immediately before a recoverable write. The model must not gain a publication tool.

## Scope and qualification

Use explicit disposable directories and synthetic fixtures for verification. Keep the original vault separate. Automated tests, agent-observed native interactions, subjective user acceptance and release qualification are distinct evidence categories.

Every new domain/workflow capability should be usable headlessly. A visual-only interaction does not require a CLI command.
