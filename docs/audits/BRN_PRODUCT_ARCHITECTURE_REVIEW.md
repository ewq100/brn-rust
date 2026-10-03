# Independent BRN product and architecture review

**Date:** 2026-10-03\
**Reviewed:** `main@6183cfc1d0619a6ac7150c5fec91fa0b778579bc`\
**Requirements:** [BRN Product Vision](../product/BRN_PRODUCT_VISION.md), plus the owner's resolved rules in this review\
**Proposal challenged:** [Opus product and architecture audit](BRN_PRODUCT_ARCHITECTURE_AUDIT.md)\
**Status:** Advisory architecture review, not an implementation plan or an accepted architecture freeze.

The review was performed against a clean checkout. The only repository change since the Opus audit's baseline was the audit document itself. Verification was static: no tests, live provider calls, model downloads or original-data inspection were performed. This report and its documentation-index entry were saved afterward at the owner's request. No product code, dependencies or architecture contracts were changed.

## A. Overall verdict

**ACCEPT WITH CHANGES.**

The audit proposes the right architecture: keep the current foundation, remove legacy production code, and add the missing product concepts inside existing crates. I found no concrete reason for another redesign.

The changes needed before freezing it concern **proposal recovery, approval boundaries, knowledge identity/provenance, and build dependencies**. These are targeted corrections, not replacement infrastructure.

## B. What Opus got right

Retain all eight choices identified by the owner:

| Choice | Assessment |
| --- | --- |
| Markdown vault | Correct authority for durable knowledge, sources and assets. |
| `brn.sqlite` WorkStore | Correct authority for operational state. Keep migrations, integrity checks, ownership and backup machinery. |
| Disposable `index.sqlite` | Correct. Extend its metadata and relationships rather than adding storage engines. |
| `brn-workflow` | Correct central application layer for approval rules and user commands. |
| Thin `brn-ai` / Rig adapter | Correct. Keep provider details and credentials contained here. |
| Current retrieval implementation | Keep FTS5, vectors, fusion and evidence validation. Qualify multilingual relevance and personal-scale performance. |
| AppWorker ownership | Keep admission, cancellation and joined shutdown. Extend carefully for writes and queued AI work. |
| CLI over the same workflow | Keep it as the headless interface and verification surface. |

These are real foundations in the code: [application ownership](../../crates/brn-workflow/src/app.rs), [WorkStore protection](../../crates/brn-store/src/work/mod.rs), and [fresh evidence validation](../../crates/brn-workflow/src/ai_tools.rs). Relevant evidence at the reviewed commit: `app.rs:27`, `work/mod.rs:92`, and `ai_tools.rs:60`.

Opus also correctly rejects per-word acceptance, graph databases, numeric authority scores, a context planner, and a generic workflow engine.

## C. What Opus got wrong or overcomplicated

### 1. Its proposed apply sequence does not establish atomicity

[Audit §12.4](BRN_PRODUCT_ARCHITECTURE_AUDIT.md#124-how-proposals-and-approval-work) writes files sequentially, then commits operational records. A crash after replacing file A but before replacing B leaves a partially applied proposal. An `applying` marker and matching hashes are insufficient recovery proof.

The minimum correction is a **small per-operation apply manifest** containing the exact approved proposal version, preconditions, staged results and recovery material. Reconcile interrupted applications before exposing affected state; never report partial application as success or overwrite unexpected external changes during recovery.

This belongs inside proposal application and vault writing. It does not justify porting the legacy journal or creating a workflow engine.

Atomicity should mean one accepted BRN outcome with recoverable application. Independent file renames and SQLite cannot provide instantaneous, all-at-once visibility to external filesystem readers.

### 2. General proposal should mean one review lifecycle, not arbitrary record mutation

Use a closed set of typed changes: note creation/replacement, moves/trash, assets, and action changes. Add kinds when an actual product slice needs them. Do not expose arbitrary SQL, generic table patches, or configurable transitions.

Automatic persistence of chats, tentative findings, proposal drafts and Inbox processing state does not itself need another proposal. Approval protects **authoritative knowledge and real actions**.

AI-detected actions and chat requests such as “Create an action…” produce proposals. Direct user Complete or explicit “sent it” can complete the identified action without another proposal. Follow-up work creates a new related action rather than reopening completed work. Direct AI-generated creation or updates of notes, actions, profiles and durable relationships still require approval.

### 3. Groups need a small dependency rule

Unrelated consequences should be separate proposals. Effects that must remain consistent, such as installing a replacement document and superseding its predecessor, belong in one proposal.

Approve All applies proposals individually and reports each result. A proposal referring to an unapproved source must wait for that source or retain sufficient approved evidence; it must not create dangling provenance.

### 4. The legacy-removal dependency is inconsistent

[Audit §8.2](BRN_PRODUCT_ARCHITECTURE_AUDIT.md#82-remove-after-replacement-needs-simple-save--proposalscomments-first) requires proposals/comments first; [§14](BRN_PRODUCT_ARCHITECTURE_AUDIT.md#14-migration-strategy) removes legacy immediately after Save. Resolve this as:

**simple safe Save → verify Save/recovery → remove legacy production architecture → Proposal Core.**

Preserving obsolete review behavior is not a prerequisite. Export legacy data only if actual valuable data is found. Existing data folders and trial workspaces remain preserved; blanket deletion of `experiments/` is unnecessary and conflicts with the [repository instructions](../../AGENTS.md).

### 5. Some duplication claims are overstated

Legacy buffers and recovery snapshots are not automatically competing authorities for saved Markdown. The repository explicitly separates those roles and excludes simultaneous workspace modes. Remove duplicate implementations, while retaining recovery bytes and citation snapshots where they serve a concrete purpose. See [documented authority](../architecture/overview.md#managed-note-authority) and [mode exclusion](../../crates/brn-store/src/workspace_mode.rs).

## D. What Opus missed

- **Stale operational proposals.** File hashes need an equivalent record revision check. Otherwise an old “set Waiting” proposal could overwrite a later direct Complete.
- **Exact reviewed version.** Approve and Rewrite must bind the proposal version. Late AI results must not replace newer edits.
- **Backup restoration across stores.** WorkStore currently backs up on open. Restoring an older database can revive a pending proposal while its vault changes remain applied. Retain recovery receipts outside the restored database until covered by a verified backup, reconcile after restore, and add bounded backups during important operational work. [Current behavior](../../crates/brn-store/src/work/mod.rs), lines 115–125 at the reviewed commit.
- **Full-document input.** Notes may be 1 MiB, but `read_note` returns only a 50,000-byte prefix and offers no range parameter. Rewrite needs complete target input or bounded section reads; incomplete input must not produce a destructive whole-document replacement. [Tool contract](../../crates/brn-ai/src/tools.rs), lines 7–14 and 50–54; [note limit](../../crates/brn-store/src/work/mod.rs), lines 21–22.
- **Historical access below retrieval.** `archive/` is rejected by path validation and skipped by scanning. An index flag alone cannot fix this. Separate safe path validation from current/history eligibility. [Path restriction](../../crates/brn-workflow/src/vault/path.rs), line 80; [scanner](../../crates/brn-workflow/src/vault/scan.rs), line 80.
- **Project lifecycle and communication dates.** Document current/superseded status cannot represent Active/Completed projects. Import dates cannot answer “Bob's email last week”; source occurrence dates, thread references and direction are needed where available.
- **Durable provenance independent of sessions.** Capture citation evidence from host-observed tools, not reconstructed model prose. Approved knowledge must retain readable source links after its originating session is deleted.

## E. Final recommended architecture

Keep six production crates:

`Desktop + CLI → brn-workflow → {brn-store, brn-retrieval, brn-ai → Rig}`

No new crate, database, service or agent framework is presently necessary.

| Proposed subsystem | Required? | Smallest sufficient form |
| --- | --- | --- |
| Proposals | Yes | One lifecycle, typed changes, temporary comments, Rewrite, approval and recoverable application. |
| Actions | Yes | SQLite records, dependencies and typed commands. Planning remains agent reasoning. |
| Inbox | Yes | Retained intake copies, operational states and format conversion functions. |
| Needs Review | Yes | Persistent tentative findings with evidence and dismissal/resolution state. No conflict engine. |
| Context | Behavior required | Read-only queries combining notes, actions, communications and review findings. No independent context system. |
| Jobs | Execution support required | Small typed queue on existing owned lanes. No generic scheduler. |
| Relationships/graph | Yes | Derived edges and an exploration view. No graph datastore. |
| Maintenance | Yes | On-launch due check and bounded, interruptible scans producing findings/proposals. |
| Web tools | Yes | Search/fetch tools returning attributable evidence. Supplied-URL fetching alone cannot satisfy the full vision. |
| Subagents | Yes | Bounded approved-model calls with read-only tools, cancellation and budgets. Main agent owns the result. |

Source comparison, identity assessment, planning and consolidation can largely use normal agent reasoning over these tools. Code must enforce approval, preconditions, capability limits and recovery.

### Minimum Markdown conventions

For new BRN-managed notes, use stable `id`, a small `type`, and knowledge lifecycle `status`. Existing plain notes remain readable; introduce metadata through approved changes when needed. Missing or malformed metadata must not silently certify a note as approved current knowledge.

Add conditional fields only when relevant:

- project/person references and evidenced aliases;
- ordinary links for related notes and sources;
- `supersedes` for actual replacement;
- project lifecycle;
- source attribution and available source dates;
- communication thread reference and inbound/outbound direction;
- URL, publisher and retrieval date for web evidence.

Do not require both frontmatter references and body links to encode the same relationship. Do not serialize dynamic actions, conflicts or generated project context into profiles. Preserve unrelated frontmatter and bytes.

Current-state retrieval must distinguish approved current knowledge, source evidence and unclassified material. Source/history access is independent of current-state filtering: following a citation or asking what someone actually said must still reach an archived original. Completed project material should be historical for normal work queries without hiding shared sources or processes used elsewhere.

### Outbound communication

Keep pending and approved outbound drafts in proposal state by default. Approval leaves the action open.

When the user confirms sending, preserve the confirmed content as a Markdown source linked to its thread, with `direction: outbound`, and complete the identified action through a direct command. Bind confirmation to the draft version; if the user changed it externally, capture the actual supplied wording. Knowledge promotion remains a separate proposal.

A thread can be assembled from linked source messages. No separate communication database is required.

### Relationships

Use one derived edge representation distinguishing explicit Markdown links from inferred candidates. Inferred edges carry supporting evidence and become invalid when that evidence changes. Uncertainty goes to Needs Review.

Approved relationships live in Markdown. Losing the index loses only inferred suggestions, not confirmations. Rebuilding it must work offline without silently starting AI calls.

This is purposeful separation between durable relationships and tentative inference, rather than two competing authorities.

## F. Final data ownership table

| Data | Authority |
| --- | --- |
| Approved knowledge, project/person/decision/process notes | Vault Markdown |
| Converted source evidence and confirmed sent communications | Vault Markdown |
| Working profile | Vault Markdown |
| Approved durable relationships and supersession links | Markdown metadata/links |
| Images, diagrams and source assets | Ordinary vault files |
| Actions, dependencies and completion state | `brn.sqlite` |
| Sessions/messages and captured citation evidence | `brn.sqlite` |
| Proposals, approved unsent drafts, comments and unsaved edits | `brn.sqlite` |
| Needs Review, Inbox processing, activity and settings | `brn.sqlite` |
| Passages, embeddings, searchable metadata and explicit/inferred edges | Disposable `index.sqlite` |
| Unprocessed or incompletely converted originals | Intake files retained until conversion and approval permit deletion |
| Trash, bounded undo and apply-recovery material | Ordinary files plus operational receipts |
| Credentials | Existing protected credential directory |

Evidence snapshots and recovery copies are supporting records, not alternative current-note authorities. Important superseded knowledge must remain readable Markdown; short-lived undo copies do not replace history.

Source wording and assets must remain distinguishable from AI interpretation. Approval of conversion does not make omitted meaningful content complete; preserve the original intake copy until conversion is complete and approved.

## G. Final build sequence

Keep the owner's main sequence, with a few dependencies pulled forward:

| Stage | User-visible outcome |
| --- | --- |
| 0 | Vision incorporates resolved rules; conflicting instructions are retired. |
| 1 | Manual Save preserves bytes, detects conflicts, avoids overwriting new destinations and recovers unfinished edits. |
| 2 | Legacy production paths and obsolete tests disappear; historical records and existing data remain preserved. |
| 3 | Narrow capability spikes establish selected-model, effort, retry, native web and image support on actual provider routes. |
| 4 | Complete proposals support editing, comments, Rewrite, individual/group approval, recovery, activity and practical Undo/Trash. |
| 5 | Notes gain minimal identities, source provenance, current/history retrieval, relationship extraction and qualified multilingual search. Basic review findings and session timestamps exist here. |
| 6 | Actions/dashboard support approved creation, direct explicit completion and new related follow-up actions. |
| 7 | Text/email Inbox produces independently reviewable consequences. The small AI queue and identity foundations are available. |
| 8 | Office documents and supplied URLs preserve meaningful content/assets; incomplete conversion retains originals. URL fetching is available here. |
| 9 | Autonomous web research returns attributable evidence and approved durable captures. |
| 10 | Full Needs Review and scheduled maintenance handle stale knowledge, conflicts and neglected work, including web-dependent checks. |
| 11 | Project/person views assemble coherent current context from existing records. |
| 12 | Bounded helper agents extend investigation without independent durable writes. |
| 13 | Session Archive/Restore/Delete, capture warnings and approved working preferences complete session lifecycle. |
| 14 | Graph view exposes existing relationships. |
| 15 | Existing-vault cleanup proposes batches of metadata, organization and supersession changes. |
| 16 | Trusted-user packaging and upgrade/recovery qualification complete delivery. |

Identity foundations precede linked actions/Inbox; full profile views can remain later. Cleanup has no technical dependency on the graph canvas and may move earlier.

Automatic session archive should remain reversible and work offline. The vision requires a capture check before **Delete**, not every automatic archive.

Each stage needs relevant offline verification and user-visible acceptance; provider/native checks remain separate evidence. These are high-level outcomes, not a detailed coding plan or implementation authorization.

## H. Decisions that still genuinely require the owner

The resolved relationship, action and outbound-lifecycle rules are closed.

Remaining owner decisions are chiefly:

1. Which account/provider routes are approved for the intended company data and trusted users.
2. Which external search service is acceptable if native provider search cannot meet the vision.
3. Which helper models and spending limits are approved.
4. Whether any actual legacy data needs retention/export.
5. At packaging time, whether embedding assets are bundled or explicitly downloaded.

These do not require inventing API-key infrastructure, migration machinery or parallel streaming now. Default to many open sessions with queued AI execution.

## I. Architecture freeze recommendation

Use these documents as the authoritative set after reconciliation:

- [Product Vision](../product/BRN_PRODUCT_VISION.md): requirements, including the owner's resolved rules.
- [Architecture overview](../architecture/overview.md): the accepted target, ownership and dependency direction, clearly distinguished from current implementation.
- [Invariants](../architecture/invariants.md): concise approval, provenance, recovery and provider guarantees.
- [Roadmap](../roadmap.md): the high-level build sequence.
- [Status](../status.md): implementation, verification, acceptance and integration evidence.

Update AGENTS.md, the documentation index and crate READMEs to point to that set.

Mark these historical as architecture baselines:

- [2026-09-28 baseline](../architecture/decisions/2026-09-28-architecture-baseline.md).
- [Markdown-first specification](../superpowers/specs/2026-09-30-markdown-first-ai-workspace-design.md).
- [Legacy note-editing specification](../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md).
- [Rig-first reset specification](../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md) and [handoff](../architecture/brn-rig-first-architecture-reset.md).
- [Simple-notes specification](../superpowers/specs/2026-10-02-simple-rig-notes-design.md), preserving its implemented evidence while retiring its conflicting future rules.

Close or move the corresponding completed/superseded active plan packs. Retain applicable shell/presentation decisions. Keep the Opus audit and this review as dated advisory evidence, not implementation specifications.

**Freeze the crate structure, data ownership and product guarantees. Reopen architecture only when a concrete acceptance scenario fails or requirements change. Table layouts, tool parameters and UI details can evolve within that freeze.**
