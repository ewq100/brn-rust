# Brain: Markdown-first notes with bounded AI workspaces

Date: 30 September 2026

Status: Design direction approved in conversation; detailed defaults proposed for review.

Scope: Design note only. No implementation, data migration, history deletion, or merge is authorized by this document.

Repository baseline inspected: `ewq100/brn-rust`, remote `main` at `18f3891e29e57d4459c663967d26115f0e57787d`.

## 1. Intent and agreed direction

Brain should provide ordinary, independently usable Markdown notes and a capable AI writing workspace. The user needs safe revisions, comparisons, anchored comments, and recovery while writing, but does not want an indefinite archive of every intermediate version.

**Keep the safeguards while working; keep the finished knowledge afterward.**

The agreed direction is Markdown-first content, durable but finite writing sessions, and optional long-term history. It is not SQLite-first with a Markdown export feature. Saving an accepted result updates the actual Markdown file. Another editor can read or edit that file without Brain.

An immutable snapshot must remain unchanged while retained. That does not require retaining it forever. History retention is a separate policy from revision integrity.

## 2. Ownership: one authority for each kind of data

| Data | Authority and lifetime |
| --- | --- |
| Current saved note | Ordinary `.md` file in a user-selected folder. Lasting content; no export required. |
| Unfinished writing session | Local durable database: starting snapshot, working copy, meaningful checkpoints, candidates, comments, and recovery records. Retained until explicitly finished or discarded. |
| Completed-session recovery | Database or managed recovery store. Finite, configurable retention; optional explicit pinning. |
| Small application metadata | Persistent local records for note identity, vault association, search eligibility, and retained references. These are not all disposable caches. |
| Search indexes and embeddings | Derived caches of eligible current content. Rebuildable, independently budgeted, with obsolete generations removed when no longer in use. |

The database is not a competing master copy of the current saved note. It is authoritative for unfinished work and application-specific state. Losing it must not destroy saved Markdown, but may lose unfinished drafts, review state, permissions, and recovery history. Backups must distinguish those cases.

Proposed storage default: keep application databases and caches outside the note folder, associated with that folder. Do not insert mandatory Brain-specific IDs into Markdown or change existing frontmatter. Metadata-based identity must handle renames conservatively; a path or matching hash alone must not prove note identity. Ambiguous external moves require relinking or create a new identity, never guessed comment reassignment.

## 3. Proposed writing experience — decision D1 remains open

| Stage | Proposed behavior |
| --- | --- |
| Ordinary editing | Manual edits save directly to the Markdown file, with bounded recovery. No formal AI review is required. |
| Start an AI review | Freeze the starting file bytes and identity, then create a durable working session. The saved file remains visible in other editors. |
| Edit and discuss | During that review, manual edits and AI candidates remain in the working session. Autosave protects the draft without publishing it to the file. |
| Accept a suggestion | Apply it to the review draft, not automatically to the Markdown file. Rejected alternatives stay separate until cleanup. |
| Apply and finish | Approve an exact final snapshot, verify the destination has not changed, and save that snapshot to Markdown through the application workflow. |
| Finish successfully | After the file save and recovery bookkeeping are confirmed, close the session and start its retention clock. |
| Pause or close Brain | Keep unfinished work. Closing a tab, quitting, inactivity, or restarting does not mean approval, completion, or deletion. |

The interface must distinguish **Draft saved in Brain** from **Saved to Markdown**. Reopening a completed note starts a fresh session from the file; it does not depend on the full previous editing history.

D1: Confirm whether ordinary editing should save directly while a formal AI review stages all its edits, as above, or whether every editing session should wait for an explicit final approval. The former is the recommendation. Neither choice permits an unapproved AI candidate to overwrite the note.

## 4. Safe file handoff and external edits

The application, not the model, owns file writes. Approval binds the exact proposed bytes, target note, and expected starting file state. A later draft change invalidates that approval.

Before replacing a file, reconcile its current state with the session's starting state. When they differ, stop automatic replacement and preserve both versions. Initial conflict handling should offer comparison and explicit resolution or saving to a different file; automatic merging is not required. External deletion must not silently recreate a note, and a newly occupied destination must not be overwritten.

Saving spans a filesystem and a database; the design must not treat them as a single database transaction. Use a recoverable write protocol with durable intent, preserved recovery content, an operation identity, and post-write reconciliation. Cleanup must not run until an operation has a known safe outcome. Restart must distinguish not-applied, applied, and conflicting or uncertain outcomes without blindly repeating a write.

A last-moment hash comparison followed by replacement is not, by itself, a complete concurrent-writer guarantee. The implementation plan must specify and test the macOS coordination strategy and the limits for non-cooperating editors or sync clients. No claim of lossless simultaneous editing is made here. Current scope is one Brain writer on one Mac; live collaborative editing is excluded.

Normal manual saves need the same conflict and recovery protections, even though they do not need AI approval.

## 5. Proposed retention defaults — D2

These numbers are recommendations, not previously approved requirements.

| Data | Proposed default |
| --- | --- |
| Unfinished sessions, unresolved save outcomes, live operations | Never expire automatically. Surface storage use and allow explicit safe resolution or discard. |
| Unpinned completed sessions | Up to seven days after successful completion. Includes intermediate drafts, rejected candidates, resolved review comments, and local review chat. |
| Previous saved content | Bounded recovery for ordinary saves and completed reviews; coalesce repetitive autosaves and identical content. |
| Unpinned completed history budget | Provisional 100 MiB per vault across completed-session and prior-content recovery. Oldest eligible records may expire sooner when this limit is reached. |
| Pinned content | Keep until explicitly unpinned or deleted. Show its separate storage cost. |
| Obsolete retrieval data | Remove after replacement is validated and outstanding users have released it; budget separately from writing history. |

Seven days is a maximum automatic retention age, not a guarantee of seven complete days under the size limit. Active and pinned data are excluded from automatic eviction and can exceed that limit; report that honestly. An unfinished session's redundant autosaves can be coalesced without losing its current draft, required anchors, live-operation inputs, or explicitly retained checkpoints.

Completion must not silently dispose of unresolved comments. Ask the user to resolve them, keep them with a pinned review, save their useful content into Markdown, or explicitly discard them. A pinned comment carries the exact evidence needed to understand it; a pin is not just a pointer to content that cleanup may remove.

Offer an explicit purge of eligible completed history, with a preview of what will be lost. Do not delete active work or pinned dependencies as an incidental effect. Once purged, exact rejected wording and review discussions are unavailable unless separately retained. Regular backups remain separate from this recovery window.

Cleanup must remove unneeded dependent records and eventually reclaim on-disk space, not merely hide history in the interface. A retained item must not indefinitely keep unrelated sessions or obsolete indexes alive. Cleanup must also be restart-safe and must not allow expired operation IDs or late AI responses to replay old actions.

## 6. Useful evidence without an unlimited archive

Review chat expires with its unpinned completed session by default. General saved conversations are a separate product concern; this note does not authorize deleting all existing chat history.

Offer **Save decision to note** for useful rationale. Lasting facts, decisions, and source references should be explicitly written into the Markdown where appropriate, rather than preserved only in a hidden conversation.

A deliberately retained comment or answer must retain enough exact evidence to remain interpretable: quoted text, source identity/path, content fingerprint, and the relevant range or location. Do not silently retarget historical citations to changed current paragraphs. When exact full historical navigation is promised, pin the required revision; otherwise label the quote as historical and explain that the original full revision was not retained. An excerpt alone does not prove the full old file is still recoverable.

Current search must read/revalidate eligible current file content, not silently search an old database master. File changes, withdrawal, deletion, and renames must invalidate or reconcile stale retrieval state. Saving to disk is not permission to send content to an AI provider. Proposed conservative default: external content changes invalidate prior version-bound search approval until the new content is approved.

The existing provider boundary keeps provider credentials and authoritative provider conversation history provider-owned [S3]. Deleting Brain's local session data must not be described as deleting provider-side history, backups, or synced copies. This design promises application cleanup, not secure erasure everywhere.

## 7. Remaining choices and proposed defaults

| ID | Choice | Recommendation / status |
| --- | --- | --- |
| D1 | When do edits reach Markdown? | Direct saves outside review; staged edits inside an explicit AI review. Needs a user decision now. |
| D2 | How much closed history? | Seven-day maximum age and provisional 100 MiB per-vault budget, with optional pinning. Proposed; configurable. |
| D3 | Where is Brain-only state? | Outside the note folder; preserve existing Markdown/frontmatter. Proposed. Full identity/relink behavior belongs in the implementation design. |
| D4 | Multi-device scope? | Markdown remains portable; active Brain sessions do not sync in the first implementation. No shared live SQLite database or simultaneous-writer guarantee. Proposed initial scope. |

The original vault, current database contents, and existing history remain untouched. Existing drafts must not be mistaken for accepted notes during a later migration. Migration requires a separate approved plan, a backup, collision handling, exact-content validation, and a rollback path. Automatic conversion, cleanup, or relocation is outside this task.

## 8. Relationship to the current Rust architecture

The inspected baseline makes SQLite authoritative for sources and revisions and requires original comment evidence to be preserved permanently [S1, S3]. It already separates working copies, immutable checkpoints, and candidates [S2, S3]. File publication and backup/restore are not established current capabilities in the status record [S1].

This note proposes a new authority and retention contract, not a configuration toggle. Reuse the existing review concepts, exact-byte validation, stale-response protection, and shared desktop/headless workflow. Add or adapt a file/vault boundary, safe publication and external-change handling, retention, and migration. Exact crate boundaries are left to the implementation plan, not decided by this note.

Before implementation, explicitly supersede the conflicting SQLite-authority and permanent-evidence requirements in the agent guide and architecture documentation. Preserve the old dated decisions as history. Do not quietly reinterpret old invariants or mark the new behavior implemented merely because this design note exists.

No application source files or existing architecture contracts are changed by this design-note task. Detailed specification approval precedes an implementation plan; neither is a release or data-migration approval.

## 9. Acceptance checks for a later implementation

| Check | Required result |
| --- | --- |
| Independent notes | Saved notes open correctly in another Markdown editor with Brain closed and without its database. |
| Draft visibility | Other editors see the saved file, not an unapproved review candidate; Brain clearly indicates where a draft is saved. |
| Crash recovery | Acknowledged unfinished work survives restart; uncertain saves reconcile without loss or blind replay. |
| External changes | Edits, deletion, rename, and destination collisions are exercised; conflicting content is preserved and surfaced. |
| Exact application | Only the exact approved snapshot is applied; later user edits and late AI results cannot replace it silently. |
| Completion vs pause | Closing, inactivity, cancellation, and failed saves do not start deletion of unfinished work. |
| Cleanup | Eligible history expires by age/budget, active and pinned dependencies survive, and disk-space reclamation is measured. |
| Evidence | Retained quotes remain interpretable; missing old full revisions are explicit; no dangling or silently retargeted references. |
| Search | External edits/deletions and approval changes cannot grant stale evidence eligibility; indexes can be rebuilt from allowed content. |
| Migration and rollback | Verified copies are produced without modifying the original vault or silently promoting unaccepted drafts. |
| Workflow parity | Desktop and headless paths enforce the same save, approval, conflict, and cleanup rules. |

These are future acceptance criteria, not checks already passed. This note is grounded in repository documentation, not a fresh application test run.

## Sources and decision provenance

The Markdown-first direction was explicitly accepted by the user in this conversation on 30 September 2026. Numeric retention defaults and the remaining choices above are proposals for review.

Repository references are pinned to the inspected remote commit, not unverified local work or other branches:

- [S1: Current status](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/docs/status.md).
- [S2: Architecture overview](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/docs/architecture/overview.md).
- [S3: Architectural invariants](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/docs/architecture/invariants.md).
- [S4: Development workflow](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/docs/development/workflow.md).
- [S5: Agent guide](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/AGENTS.md).
