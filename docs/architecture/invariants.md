# Architectural invariants

The owner froze the reviewed architecture on 2026-10-03 and amended its client
boundary on 2026-10-04 for future external-agent access. New work follows the [product vision](../product/BRN_PRODUCT_VISION.md), [target ownership](overview.md#frozen-target) and guarantees below. The owner's resolved action, relationship and outbound rules are recorded in the dated [independent review](../audits/BRN_PRODUCT_ARCHITECTURE_REVIEW.md#c-what-opus-got-wrong-or-overcomplicated); its other recommendations remain advisory evidence. [Status](../status.md) distinguishes these requirements from implemented behavior.

## Frozen target guarantees

- Vault Markdown/assets own durable knowledge and sources; WorkStore owns operational state; `index.sqlite` is disposable. Recovery copies and citation snapshots do not compete with current-note authority.
- Preserve exact note bytes, unrelated frontmatter, original source wording and meaningful assets. Keep interpretation and approved knowledge distinct from evidence and tentative findings. Stable identity is not a path or hash; durable provenance survives session deletion.
- AI-created or changed knowledge, profiles, real actions and durable relationships require approval of an exact proposal version. Persisting chats, tentative findings, drafts or Inbox processing state does not itself require a proposal. Explicit user Save and identified Complete/"sent it" commands remain direct actions; follow-up work creates a new related action.
- A proposal uses typed changes and one full-proposal review lifecycle. Comments are temporary review notes, deleted on approval; uncertain anchors are never guessed. Late results cannot replace newer edits.
- Approval binds destinations, source versions and operational record revisions. Apply changes recoverably across files and SQLite; reconcile interruption/restore before exposing affected state. Never claim partial application succeeded or overwrite unexpected external changes during recovery.
- Outbound approval leaves the action open. Sending confirmation binds the actual sent version, preserves it as a Markdown source linked to its thread and completes the identified action. Knowledge promotion remains separate.
- Current queries use approved current knowledge. Explicit source/history access can still reach archived originals. Inferred relationships are derived evidence-backed candidates; approved relationships live in Markdown and rebuild offline without silently invoking AI.
- Keep incomplete intake until meaningful conversion is complete and approved. Trash/undo are recoverable; important superseded knowledge remains readable history. Backup/restore must preserve operational work and reconcile vault receipts.
- Desktop, CLI and future protocol clients use `brn-workflow` / AppWorker as the shared application boundary. Meaningful domain capabilities remain usable headlessly; clients do not implement private knowledge rules or directly access vault/persistence, retrieval, provider or proposal internals. AppWorker owns admission, cancellation and joined work.
- The existing six crates form the V1 core, not a permanent crate-count ceiling. Thin presentation/protocol adapters may surround it when required; they add no competing data authority or domain logic. Internal retrieval replacement preserves client-facing scopes, evidence and authority semantics.
- External knowledge access defaults to Current approved source-of-truth knowledge. Source, History and All require explicit scope. A connection exposes explicit BRN capabilities, never unrestricted filesystem or SQL access. The first future MCP adapter is read-only local stdio over AppWorker; future agent changes use the existing proposal/human-approval boundary.
- This client-boundary amendment adds no MCP implementation, daemon, remote/cloud service, HTTP listener, sync, authentication server or multi-user model to V1.
- Provider/model/account selection is explicit with no automatic fallback; credentials stay protected outside Git, vault, SQLite and logs.

Reopen these guarantees or ownership boundaries only with the owner after a changed requirement or a concrete blocker that the frozen architecture cannot reasonably handle. Follow the [roadmap](../roadmap.md) for delivery, not superseded specifications.

## Current Save and recovery contracts

- Explicit Save/Cmd-S writes Markdown; automatic recovery commits exact unfinished
  work to WorkStore. Preserve UTF-8, BOM, line endings and frontmatter unless
  deliberately edited. Empty files are valid; limits are 1 MiB in bytes.
- Baseline/generation, fresh observation and submitted generation are distinct.
  Acknowledgements preserve later typing. Equal-generation submissions need
  identical bytes; stale or conflicting preconditions fail.
- Use validated directory descriptors, supported regular single-link files,
  containment, root identity and exclusive vault ownership, including overlapping
  roots/aliases. No symlink/hard-link or unsupported-operation fallback.
- Journal intent before sibling staging and coordinated revalidation. Preserve
  attributes; require atomic exchange/exclusive installation, file F_FULLFSYNC,
  directory fsync and exact installed/displaced verification before receipt.
  Missing originals are not recreated; copies never overwrite an occupant.
- Replay binds payload before fresh-state checks and never repeats writes.
  Reconcile from exact prepared/parent/artifact identity proof. Equal bytes alone
  do not establish application. Unexpected displaced objects are retained without
  blind rollback; uncertainty blocks further original Save and current evidence.
- Compare is read-only; reload binds reviewed fresh disk state and confirmed
  discard. Save Copy does not resolve or rebind an uncertain original.
- Keep one rolling buffer and the latest Applied recovery pair. No-op/refusal does
  not refresh that pair. Unresolved work never expires automatically. Artifact
  cleanup requires exact identity proof; compact receipts preserve bound replay.
- Admitted critical mutations drain/join on shutdown. Guarded switch/close/Quit
  flush recovery asynchronously. Dock/system quit cannot veto exit or submit a
  final-hook typing flush; unacknowledged typing may be lost.

## Current storage, retrieval and provider contracts

- WorkStore is checked and backed up at open, with supported additive migrations.
  Corrupt data is moved aside and restored only from validated backups; foreign
  and newer databases remain refused. Old/mixed markers are refused under the
  owner lock before SQLite opens; there is no automatic migration.
- Saved vault bytes remain authoritative. Indexes and embeddings rebuild from
  them; dirty buffers and artifacts do not become current evidence. Current tools
  validate fresh bytes and reject results spanning Save or unresolved application.
- New Ask refreshes before submission and freezes provider/model. UUID replay
  precedes selection/auth gates and never resubmits a Running turn. Finished
  follows durable finalization; PersistenceFailed retains visibly unsaved partial
  text. Transport loss/Stop does not prove upstream cancellation.
- Credentials are owner-only outside Git/data/vault. Saved explicit location wins,
  then the exact data-name credential sibling. Only Connect starts login; device
  codes stay transient and no provider/account/model fallback exists.
- AppWorker owns and joins chat/account/install/read leases before releasing
  authority. Disconnect fences, cancels and joins only its target provider before
  deleting that provider's caches.
- Model activation indexes only with a loaded model and available vault. Download
  needs fresh consent; saved consent/startup never start network work. Default
  builds are explicitly keyword-only and refuse installation.

## Scope and qualification

Use disposable explicit directories and synthetic fixtures. Preserve original
vaults/data and standalone trials. Automated checks, native observation, owner
acceptance and release qualification are distinct. Every domain/workflow
capability must be usable headlessly; visual-only interactions need no CLI command.
Historical immutable-comment, legacy eligibility and publication contracts are
superseded; they remain only in dated task/design evidence.
