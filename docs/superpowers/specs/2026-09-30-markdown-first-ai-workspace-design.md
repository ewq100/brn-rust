# Brain: Markdown-first notes with bounded AI workspaces

Date: 30 September 2026

Status: Design direction and editing behavior (D1) approved; current-only retrieval and archive requirements (D5) recorded from the user. Git integration and other detailed defaults remain proposed for review.

Scope: Design note only. No implementation, data migration, history deletion, or merge is authorized by this document.

Repository baseline inspected: `ewq100/brn-rust`, remote `main` at `18f3891e29e57d4459c663967d26115f0e57787d`.

## 1. Intent and agreed direction

Brain should provide ordinary, independently usable Markdown notes and a capable AI writing workspace. The user needs safe revisions, comparisons, anchored comments, and recovery while writing, but does not want an indefinite archive of every intermediate version.

**Keep the safeguards while working; keep the finished knowledge afterward.**

The agreed direction is Markdown-first content, durable but finite writing sessions, and optional long-term history. It is not SQLite-first with a Markdown export feature. Saving an accepted result updates the actual Markdown file. Another editor can read or edit that file without Brain.

An immutable snapshot must remain unchanged while retained. That does not require retaining it forever. History retention is a separate policy from revision integrity.

**Normal AI questions use current knowledge only. Retaining a note or revision does not grant the AI access to it.** Older saved versions remain available through history, and whole notes can move out of the active vault into an archive. Neither is included in normal AI retrieval; historical access requires an explicit user request with a bounded scope.

## 2. Ownership: one authority for each kind of data

| Data | Authority and lifetime |
| --- | --- |
| Current saved note | One current `.md` file per logical note in the active vault. Lasting content; no export required. |
| Archived note | Ordinary `.md` file outside the active vault, still registered but excluded from normal AI access. |
| Historical saved versions | Accessed through History rather than extra active files. Git is the proposed backend; its policy is separate from temporary workspace retention. |
| Unfinished writing session | Local durable database: starting snapshot, working copy, meaningful checkpoints, candidates, comments, and recovery records. Retained until explicitly finished or discarded. |
| Completed-session recovery | Database or managed recovery store. Finite, configurable retention; optional explicit pinning. |
| Small application metadata | Persistent local records for note identity, vault association, search eligibility, and retained references. These are not all disposable caches. |
| Search indexes and embeddings | Derived caches of eligible current content. Rebuildable, independently budgeted, with obsolete generations removed when no longer in use. |

The database is not a competing master copy of the current saved note. It is authoritative for unfinished work and application-specific state. Losing it must not destroy saved Markdown, but may lose unfinished drafts, review state, permissions, and recovery history. Backups must distinguish those cases.

Proposed storage default: keep application databases and caches outside the note folder, associated with that folder. Do not insert mandatory Brain-specific IDs into Markdown or change existing frontmatter. Metadata-based identity must handle renames conservatively; a path or matching hash alone must not prove note identity. Ambiguous external moves require relinking or create a new identity, never guessed comment reassignment.

## 2.1. Permanent file-status registry, separate from expiring history

Markdown-first does not mean all database records expire. Keep a durable current-state record for each managed note: its internal identity, vault/path association, observed content fingerprint, and application decisions such as review state and search eligibility. Completion or cleanup of a writing session must not delete or reset that record.

Current status is not the same as status history. Brain can retain the latest state without retaining every previous state, full text revision, rejected candidate, or review conversation. A finished review may update a note's current status; later cleanup removes its eligible working history, not the resulting current-state record. Durable records may still be changed or removed through explicit lifecycle and reconciliation rules; “permanent” does not mean immutable or impossible to delete.

Keep three meanings separate: document lifecycle/review state, writing-session state, and retrieval-processing state. Example lifecycle labels such as “needs review” or “reviewed” are illustrative, not an approved status taxonomy. Do not equate a completed writing session, permission to use a file for search, and permission to apply a particular AI rewrite. An approval applies only to the content and purpose actually approved; a content change must not inherit an old version-bound approval automatically.

User decisions and permissions are durable metadata, not reconstructible from Markdown alone under the proposed no-mandatory-frontmatter default. Observations such as an indexed content fingerprint or a pending reindex can be recomputed and reconciled. Losing the registry must not lose the saved note, but may lose its workflow decisions; a rebuild must not invent approval. Include durable metadata in the backup design.

The registry must distinguish active versus archived notes, identify the current content version, and maintain history locators when history is enabled. Archive status overrides any previous normal-search permission. A Git commit records content; it does not confer review status or permission for AI access. Exact status labels and schema remain to be designed.

This is a logical separation, not a requirement for multiple physical database files. A single SQLite database may hold persistent registry tables and separately managed session/history tables, with distinct retention rules. Exact table layout and mapping of the original TypeScript statuses remain implementation-design work; feature parity is not asserted here.

## 3. Approved writing experience — D1

| Stage | Approved behavior |
| --- | --- |
| Ordinary editing | Manual edits save directly to the Markdown file, with bounded recovery. No formal AI review is required. |
| Start an AI review | Freeze the starting file bytes and identity, then create a durable working session. The saved file remains visible in other editors. |
| Edit and discuss | During that review, manual edits and AI candidates remain in the working session. Autosave protects the draft without publishing it to the file. |
| Accept a suggestion | Apply it to the review draft, not automatically to the Markdown file. Rejected alternatives stay separate until cleanup. |
| Apply and finish | Approve an exact final snapshot, verify the destination has not changed, and save that snapshot to Markdown through the application workflow. |
| Finish successfully | After the file save and recovery bookkeeping are confirmed, close the session and start its retention clock. |
| Pause or close Brain | Keep unfinished work. Closing a tab, quitting, inactivity, or restarting does not mean approval, completion, or deletion. |

The interface must distinguish **Draft saved in Brain** from **Saved to Markdown**. Reopening a completed note starts a fresh session from the file; it does not depend on the full previous editing history.

D1 approved by the user on 30 September 2026: ordinary manual editing saves directly to Markdown; an explicit AI review stages both manual edits and AI suggestions until Apply and finish. Accepting an individual suggestion changes the review draft only. Closing Brain preserves unfinished work. This approval settles editing behavior, not the remaining defaults, implementation, or migration.

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
| Short-term recovery snapshots | Bounded recovery for ordinary saves and completed reviews; coalesce repetitive autosaves and identical content. |
| Saved-version history and archived notes | Not deleted by completed-workspace expiry. Propose retaining meaningful saved versions in Git and archived Markdown until a separate explicit deletion/history-retention decision. Git remains a proposed backend. |
| Unpinned completed history budget | Provisional 100 MiB per vault across completed-session and prior-content recovery. Oldest eligible records may expire sooner when this limit is reached. |
| Pinned content | Keep until explicitly unpinned or deleted. Show its separate storage cost. |
| Obsolete retrieval data | Remove after replacement is validated and outstanding users have released it; budget separately from writing history. |

The proposed seven-day/100 MiB limit applies to temporary workspace and recovery data, not to saved-version Git history, archived Markdown, or the durable status registry. Those storage categories must be reported separately. Archiving is not a disk-space reduction or secure-erasure promise.

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

## 6.1. Current knowledge, archive, and explicit history — D5

The user requires normal questions to use the current version of a plan, not superseded plans. Historical analysis must be possible only when deliberately requested. The following distinction is required:

| Surface | Meaning | Normal AI access |
| --- | --- | --- |
| Active vault | Current saved versions of active logical notes. For a plan at version 3, there is one current `plan.md`, not automatically generated `plan-v1.md`, `plan-v2.md`, and `plan-v3.md` alongside each other. | Eligible only when the exact current content is permitted for the request. |
| Archive | Whole notes deliberately retired from the active vault, retaining their last saved Markdown and identity. | Excluded, even if previously approved or still present in an obsolete index. |
| Version history | Older saved states of active or archived notes. Shown in History, not materialized as ordinary active notes. | Excluded unless the user explicitly requests historical access. |
| Review workspace | Unfinished drafts, comments, and AI candidates. | Only within the explicitly selected writing/review session, never as general current knowledge. |

Git stores committed snapshots separately from the checked-out working files, so historical versions do not require extra Markdown copies in the active folder [G1]. Brain may expose archive and history under one navigation area, but they remain different lifecycles. Explicitly saving an old baseline as a separate reference is possible; it must be labeled appropriately, not silently promoted to a second current version of the same plan. Existing files with similar names or dates must not be automatically deleted, merged, or superseded by a filename heuristic.

“Current” means the user's designated current saved content, not an assertion that every statement is factually up to date. Brain must not infer semantic obsolescence merely from a file's age. Version 4 still under review does not replace saved version 3 for ordinary questions. Once version 4 is applied, version 3 loses current eligibility immediately; indexing lag or missing new approval must produce an unavailable/reindex-needed result, not fallback to version 3.

### Archive and restore lifecycle

Archive is an explicit, recoverable application operation. First persist the intent and make the note ineligible for normal AI use; then move its current Markdown from the active vault to the configured archive, reconcile its registry location, and record the archive event in history when enabled. Preserve its identity and retained historical references. Do not overwrite an occupied destination or silently resolve an unfinished review: require the user to finish, keep a protected paused session, or explicitly discard it. Interrupted archive operations remain excluded until reconciled; do not announce completion before the move and durable state are confirmed.

Removing/rebuilding old index entries follows the eligibility change, not the other way around. There must be no window in which a completed archive is still returned as current because a worker has not re-indexed it. For external moves/deletions, validate existence, location, identity, and content at retrieval time as well as using file-change notifications.

Restore from archive makes the selected note a candidate to become active again through an explicit operation with collision and content checks; do not infer fresh search permission from its former state. Restoring a historical version should create a reviewed candidate and, on application, a new current saved version/checkpoint. Do not rewind the entire vault, disturb other notes, or silently discard newer content. Browsing historical content does not restore it.

### Enforce the boundary outside the model

The default retrieval scope is current active content only. Apply the same eligibility predicate to keyword, semantic, hybrid, and future graph retrieval and revalidate exact evidence immediately before sending it to a provider. Exclude the archive, Git objects, backups, expiring workspaces, and history excerpts from ordinary indexing; separate collections or on-demand historical indexes can serve explicit history requests.

Do not implement this as “give the model everything and ask it to prefer the newest.” Normal answering tools must not provide an unrestricted filesystem, shell, Git, or archive-search route around the filter. The application owns any history lookup and returns only authorized note/version evidence. Archive links, graph neighbors, cached answers, retained source quotes, and AI summaries must not become back doors that reintroduce excluded sources as current evidence.

A current answer must be marked with the source content identities/knowledge generation it used. If a relevant source changes or is archived during an in-flight turn, do not silently present the result as current: cancel/recompute or clearly mark it stale and require a fresh current query. Already sent content cannot be unsent.

Historical provider conversations also need separation. A conversation that already contains version 1 cannot be made current-only merely by filtering its next search. Use an isolated historical context, and create a fresh provider context with only revalidated current evidence when returning to current mode or when a formerly supplied source becomes ineligible. Do not blindly replay old answers, quotes, or historical summaries into it. This rule governs Brain-supplied context and tools; it is not a promise to erase provider-side records, user-supplied text, or a model's prior knowledge.

### Explicit historical questions

An explicit request such as “Compare the last three saved versions of this plan” authorizes a bounded historical operation for that plan. Requests to inspect archived material can likewise scope notes, folders, dates, or selected results. Permission is for the requested historical task, not a lasting global switch. An ordinary or ambiguous “What is happening with the plan?” does not authorize broadening into the archive, even when current results are empty.

Historical results must show note identity, saved version/commit, recorded date, and active/archived/historical status. A recorded date is not proof of when a real-world decision took effect. Historical lookup may read selected Git versions without checking out old files over the live vault [G2]. Any temporary search data for history remains isolated from the current corpus and is eligible for cleanup after use. Historical analysis must cite the versions actually read and distinguish them from the current plan; it must not promise to reconstruct rejected drafts already expired from the writing workspace.

## 6.2. Git across active and archived notes — proposed D6

Recommendation: retain saved-version Git history when a note is archived. Archiving changes visibility and current eligibility, not the right to keep or inspect its past. The same history backend can serve active and archived notes; no separate Git repository per archived note is required.

One proposed local layout is a library root containing sibling `Vault/` and `Archive/` folders, with one Git repository at the library root. Only `Vault/` is a candidate root for current retrieval, further restricted by registry state and permissions. SQLite and caches stay in external application storage under proposed D3. This layout is a recommendation, not approval to initialize, relocate, or alter an existing user repository. For an existing repository or an external archive directory, history must remain resolvable by verified identity/location records; moving the file must not silently break its history.

Use Git for meaningful saved checkpoints, not every keystroke, rejected candidate, or review transcript. Record the pre-change content when needed and the exact content successfully applied to Markdown. Git failures must be visible and recoverable without undoing a successful file save; retain sufficient recovery data until the history outcome is resolved. Do not sweep unrelated staged files into a Brain commit. Treat commit IDs as locators, not status/approval records, and verify any content loaded through a locator. Implementation must account for filters, line endings, external Git operations, and missing/rewritten history without guessing provenance.

Normal Git maintenance preserves referenced history; it does not automatically remove a note's old versions because it was moved or because the workspace retention window elapsed [G3]. Report history storage separately and never describe archive as purging history. Per-note permanent deletion, history rewriting, or a strict history-size cap is a separate design/authorization decision, especially when copies or remotes exist. No automatic remote push is proposed. Keeping local history is not a complete backup of the notes, metadata, or unfinished work.

## 7. Remaining choices and proposed defaults

| ID | Choice | Recommendation / status |
| --- | --- | --- |
| D1 | When do edits reach Markdown? | Approved 30 September 2026: direct saves outside review; staged edits inside an explicit AI review, applied only on final approval. |
| D2 | How much closed workspace/recovery history? | Seven-day maximum age and provisional 100 MiB per-vault budget, with optional pinning. Proposed; configurable. Excludes archived notes and saved-version Git history. |
| D3 | Where is Brain-only state? | Outside the note folder; preserve existing Markdown/frontmatter. Proposed. Full identity/relink behavior belongs in the implementation design. |
| D4 | Multi-device scope? | Markdown remains portable; active Brain sessions do not sync in the first implementation. No shared live SQLite database or simultaneous-writer guarantee. Proposed initial scope. |
| D5 | Current knowledge vs archive/history? | User requirement recorded 30 September 2026: only current active content in normal AI retrieval; whole-note archive and older saved versions excluded; explicit scoped historical access. |
| D6 | How is saved-version history stored? | Git-backed meaningful checkpoints, retained through archiving; one local library repository can cover active and archive folders. Backend, exact layout, and long-term retention policy remain proposed. |

The original vault, current database contents, and existing history remain untouched. Existing drafts must not be mistaken for accepted notes during a later migration. Migration requires a separate approved plan, a backup, collision handling, exact-content validation, and a rollback path. Automatic conversion, cleanup, or relocation is outside this task.

## 8. Relationship to the current Rust architecture

The inspected baseline makes SQLite authoritative for sources and revisions and requires original comment evidence to be preserved permanently [S1, S3]. It already separates working copies, immutable checkpoints, and candidates [S2, S3]. File publication and backup/restore are not established current capabilities in the status record [S1].

This note proposes a new authority and retention contract, not a configuration toggle. Reuse the existing review concepts, exact-byte validation, stale-response protection, and shared desktop/headless workflow. Add or adapt a file/vault boundary, safe publication and external-change handling, archive lifecycle, current-only retrieval/context isolation, retention, and migration. Git history is a proposed additional boundary, not an implemented capability. Exact crate boundaries are left to the implementation plan, not decided by this note.

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
| File-status durability | Completed-session cleanup preserves current note identity, review state, and eligibility; content changes do not inherit stale approvals; registry loss never fabricates permissions. |
| Evidence | Retained quotes remain interpretable; missing old full revisions are explicit; no dangling or silently retargeted references. |
| Search | External edits/deletions and approval changes cannot grant stale evidence eligibility; indexes can be rebuilt from allowed content. |
| Current-only evidence | After plan v3 is current, normal keyword/semantic/hybrid queries cannot receive v1/v2; absent v3 indexes never cause fallback. Include future graph paths in the same contract. |
| Archive exclusion | Archive during and between searches: old index entries, direct tools, graph links, caches, and returned answers cannot silently bypass exclusion. Crash/restart during the move preserves data and the exclusion intent. |
| Explicit historical scope | Default current requests do not widen when empty. A scoped history request accesses only allowed material, labels versions/dates, and does not repopulate the current index. |
| Context isolation | An old/current or history provider thread is not blindly reused after evidence becomes excluded. Current mode receives freshly revalidated evidence and no historical replay. |
| History continuity | When Git is enabled, archive/restore preserves exact history lookup; missing history is explicit; reading a past version does not alter live notes. |
| Migration and rollback | Verified copies are produced without modifying the original vault or silently promoting unaccepted drafts. |
| Workflow parity | Desktop and headless paths enforce the same save, approval, conflict, and cleanup rules. |

These are future acceptance criteria, not checks already passed. This note is grounded in repository documentation, not a fresh application test run.

## Sources and decision provenance

The Markdown-first direction and D1 editing behavior were explicitly accepted by the user in this conversation on 30 September 2026. The user also raised the original TypeScript app's permanent file-status database; section 2.1 clarifies that durable file metadata remains part of this design. No exact TypeScript status schema or migration mapping has been established by this note.

The user subsequently required an archive outside the main vault, one current version per plan, exclusion of obsolete material from ordinary AI questions, and explicit optional historical analysis. Sections 6.1 and D5 record those requirements. Git retention through archiving and the library layout in section 6.2 are recommendations, not a confirmed backend decision. Numeric retention defaults and other remaining choices are proposals for review.

Repository references are pinned to the inspected remote commit, not unverified local work or other branches:

- [S1: Current status](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/docs/status.md).
- [S2: Architecture overview](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/docs/architecture/overview.md).
- [S3: Architectural invariants](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/docs/architecture/invariants.md).
- [S4: Development workflow](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/docs/development/workflow.md).
- [S5: Agent guide](https://github.com/ewq100/brn-rust/blob/18f3891e29e57d4459c663967d26115f0e57787d/AGENTS.md).

Public technical references consulted 30 September 2026:

- [G1: Git snapshots and the working tree](https://git-scm.com/book/en/v2/Getting-Started-What-is-Git%3F).
- [G2: Read historical objects with git show](https://git-scm.com/docs/git-show).
- [G3: Git maintenance and retained objects](https://git-scm.com/docs/git-gc).
