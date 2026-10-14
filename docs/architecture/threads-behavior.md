# BRN Threads behavior and runtime guidance

**Status:** Accepted behavior design with production runtime integration, 11 October 2026. Current implementation/evidence qualifications are recorded below; unverified native or later-scope behavior is not implicitly accepted. The [canonical target](threads-target.md) owns product requirements and integrity guarantees. The [active cleanup plan](../work/active/pre-release-cleanup/plan.md) owns the current assignment. This document makes the thread interaction and runtime instructions concrete without creating another product specification.

## One thread follows one goal

A thread is the continuing context for a question, decision, or piece of work. Its title states that purpose: “Resolve Project A's delivery date,” “Prepare Friday's update,” or “Explain this process.” It holds the conversation and links to the relevant notes, source references, Actions, candidate changes, and completed operations. The same note can participate in several threads without being copied.

Notes remain independently readable knowledge. Actions retain their own progress and completion evidence. A thread does not have to contain every fact about its subject, and the conversation is not the authoritative representation of an Action's due date or a note's current text. Opening a thread loads current linked records and identifies historical revisions when discussing earlier work.

The accepted writing workbench presents the full selected linked thread beside the current note. Passage comments accumulate independently of AI dispatch. **Rewrite with AI** freezes the current saved note, selected pending comments and posted chat as an inspectable request, then proposes a complete note revision for exact review. Chat additions during the run belong to the next request. No text-selection gesture grants write authority, and this command cannot become an automatic maintenance write on Continue. Explicit addressed confirmation follows a successful Apply receipt; generation and dismissal retain pending feedback. [Implementation scope and qualification](../work/active/tiled-writing-workbench/spec.md).

The owner-selected [writing polish](../work/active/writing-polish/spec.md) keeps feedback in the Note tile and chat in its own tile. Read presents formatted text from the current buffer; Write edits that same Markdown source with formatting controls. Rendered passage selection must map exactly to the source being displayed; source changes clear prior selections. Save and comment saves through the checked version/generation boundary and revalidates the captured quote before opening the existing composer. Failed Save preserves the edit and draft; Add comment remains a separate replayable action. Read result changes only candidate presentation and focuses the displayed view; Apply still binds the exact candidate. Passage and overall-feedback drafts retain window-local per-note navigation preservation. Rewrite captures overall feedback in its immutable bounded request with no separate chat Send, including it in agent context and replay identity. Writing proposals and retry/progress/Stop appear in the Note tile; writing request rows and writing continuation controls do not occupy chat. Writing works with Chat closed. If the exact terminal writing run responds without a pending proposal, its same-receipt saved reply/question is readable in the Note tile; overall feedback plus a new Rewrite answers it. Ordinary chat keeps its own interactions and history. Saved rewrite feedback survives restart through the request; unsent writing drafts have no new restart guarantee.

The owner can start a thread from Ask or delegate, a selected note passage, a source import, or an existing Action. The agent can open one when useful work requires an owner decision. Routine processing can attach its outcome to an existing work context and remain quiet.

## State and attention

Chats stay in conversation history after an answer. The owner may Archive or Unarchive them; archive stops the chat's work, fences its live runs, and pauses the saved queue. Unarchive never resumes work. Legacy Resolved conversations appear as archived; Unarchive keeps their identities and restores the internal Open state. Thread IDs remain the storage/API identity. Ordinary chat has no Resolve/Reopen or quick-question auto-resolution.

Attention is separate from that lifecycle:

| Attention reason | What the owner sees | What clears it |
|---|---|---|
| Question | A concrete question and enough context to answer it | An answer or an explicit decision to leave it unresolved |
| Review | The particular candidate and requested decision | Apply, reject, or replace that candidate |
| Conflict | The competing evidence or incompatible change | A recorded resolution or an explicit decision to defer it |
| Blocker | What prevents progress and what the owner can do | The blocking condition is resolved or the work is cancelled |

Keep the underlying reason and linked object, not only a boolean badge. A thread may have several reasons but appears once in **Needs you**. Clearing one reason cannot hide another. Archiving preserves outstanding attention requests and does not complete a linked Action. The interface still shows any Open or Waiting Actions.

Working, Interrupted, Failed, and provider-unavailable indicators come from the run and its outcome, rather than additional thread lifecycle states. An interruption only enters Needs you when resuming requires an owner action. Completed routine work appears in **What changed**, with its operation, affected notes, reason, and Undo. Do not create a notification for every tool call, imported source, or successful note update.

## Work within and across threads

**Desktop chat:** One worker runs at a time across provider calls, intake and output publication. Enter sends an idle message and queues messages during work, with each destination saved per item. Shift-Enter adds a line. Waiting items can be edited, removed and reordered; each executes as a separate fresh turn with current context. A claimed pending instruction and its exact owner Message survive restart without reposting. Run start binds the instruction and dispatch claim in the same checked group. Only its exact committed terminal Run/fence can acknowledge the item. Completion never unpauses a paused queue.

**Stop and recovery:** Stop fences the current invocation and pauses the queue before cancellation. Useful visible interrupted text is saved as partial progress. Failure, disconnect and restart preserve pending/waiting text and leave the queue paused until explicit Resume. A retry reuses the pending owner Message identity. Resume invalidates old completion associations; old terminal events cannot resume or re-pause newer work. Composer recovery stores only the latest generation per destination, including Unicode, rather than permanent keystroke revisions.

**Steer:** A separate explicit Steer action stops/fences the active invocation and posts the new instruction atomically, updating a queued pending instruction when applicable. After the old worker finishes and its events are drained, a fresh invocation starts using current settings and authority. Waiting messages remain saved. This is a stop-and-fresh-run handoff, not provider injection. Stop cancels a waiting automatic replacement and leaves its instruction saved. No protected-edit grant is inherited.

**Reading:** Markdown answers stream with stable response identities and selectable text. Per-chat MessageScroller state follows the tail until the reader scrolls away and offers Jump to latest. Quiet is a saved display preference. Quiet on hides intermediate activity across the window while keeping streamed answers, Stop, errors, proposals and attention visible. Quiet off shows a bounded, scrollable monospace trace of tool calls with argument previews, tool outcomes and explicit public provider reasoning summaries. Raw reasoning, encrypted payloads and full returned documents are excluded; write bodies are omitted. Apply outcomes distinguish Applied from NeedsReview, Deferred, Stale and Superseded. The latest trace stays in its own chat until a new run or app restart; answers, meaningful progress and checked receipts retain their existing durable paths. Copy answer and the editor's selection copy remain available.

**Attachments:** Attach file defaults to chat-only retention of readable supported content and meaningful figures, with visible conversion gaps. SQLite stores the converted note/assets, so later reads do not require the original. General lists/search/Markdown export exclude chat-only content; full backup includes it. Keep in BRN changes the same protected note through a version-checked, replayable owner operation, retaining content/assets/links and Undo. Source observations have per-context identities; global and private observations sharing a locator cannot overwrite one another. Agent reads (IDs, ranges, history, sources and links) and typed proposal inputs/references share that context boundary. A run exposed to unpromoted attachments cannot autonomously write their context into general BRN. Explicit general useful-information/full-note intake remains available.

Projects and concurrent provider workers are deferred; the accepted [chat-first brief](../work/active/chats-and-projects/spec.md) records the later project decisions.

Source references use stable source identity where available: a provider identifier, known location, or observed fingerprint. An exact known source or existing Action link can lead directly to the related thread. The agent uses search and current context to judge whether new information belongs in another existing thread. When the relationship is uncertain, preserve that uncertainty rather than forcing a merge. Do not build a semantic deduplication engine or treat similar wording as proof that two sources or goals are identical.

Importing the same source again can identify a prior processing outcome or a newer observation; it does not automatically justify a second note, Action, or attention item. Identity checks are deterministic. Deciding what the information means and where it belongs is agent work.

## Note comments and reviewing changes

A note comment is the same thread with an optional anchor: note ID, revision ID, range, and original quotation. A task thread may have no anchor or link several notes. Keep the original context available after rewriting or deleting the quoted passage.

Use the editor toolkit's position tracking for supported edits. Store a reliable updated mapping with the associated note change. Across moves or substantial rewrites, attachment is best effort: show an unresolved location when no reliable mapping exists. An agent can suggest a relocation; it must not silently invent certainty. Neither CRDT adoption nor perfect semantic anchors are prerequisites.

The thread workspace shows the current note and the proposed final note with understandable change highlighting. The owner can edit that final candidate before accepting it. Start with whole-note revisions and explicit Action changes as review groups, not arbitrary hunk combinations. Approval binds the actual displayed candidate. Editing its contents creates a replacement identity and invalidates an older approval.

Allow one active candidate per note. Another thread can refer to it or request a refresh. If current state changes, report Stale and reread before generating a replacement. Ordinary delegated work may then apply under its existing scope; a manually reviewed replacement needs applicable authority for the changed result.

Unsaved human writing defers a grouped autonomous change in its entirety when any member is guarded. Do not quietly apply its other members. Human Save still checks the editor's base version. The [target's change and recovery contract](threads-target.md#a-single-mechanism-for-changes-and-review) governs durable preparation, idempotent replay, dirty-buffer coordination, and compensating Undo; thread presentation does not create a second write path.

## Runtime guidance is deliberately small

The development agent follows the repository's [AGENTS.md](../../AGENTS.md). The shipped BRN agent needs separate product guidance. The production bundle contains these paths:

| Packaged file | Responsibility |
|---|---|
| `agent/BRN.md` | Short base behavior shared by every run |
| `agent/skills/intake/SKILL.md` | Interpret supplied sources and honor the chosen import intent |
| `agent/skills/maintain-notes/SKILL.md` | Maintain current ordinary knowledge and related work |
| `agent/skills/resolve-conflict/SKILL.md` | Investigate disagreement and prepare the owner's decision |
| `agent/skills/prepare-reply/SKILL.md` | Produce a supported reply draft without claiming delivery |

Use YAML `name` and `description` followed by Markdown instructions. The description states what the skill does and when it is useful. This follows the [Agent Skills format specification](https://agentskills.io/specification). A compact catalog supplies the metadata; the relevant full instructions are loaded when needed. The format does not itself execute anything or make a model obey it.

BRN loads the four-entry catalog from a fixed packaged table using `include_str!`. The runtime puts the base guide and skill descriptions in the run's preamble through Rig's agent builder. Registered `read_skill(id)` returns packaged text for an allowlisted skill ID and accepts no filesystem path. Tests validate the small frontmatter contract. No runtime filesystem discovery, frontmatter framework, remote marketplace, or self-modifying prompt system is needed. A generic document loader or model-information catalog is not automatically a skill loader. The adapter is implemented and covered by recorded runtime/tool checks.

Record the instruction bundle version or content hash and loaded skill identities in Run. This supports diagnosis when guidance changes without storing every raw tool transcript. Keep the actual bundled versions available through the application build/repository. Loading a skill gives instructions; it cannot expand tools, data access, provider choice, or mutation authority.

### Shared base guide

The packaged [base guide](../../agent/BRN.md) is the runtime instruction source. It keeps routine delegated work quiet and recoverable, separates source claims from owner decisions, preserves protected material, requires shared tools and truthful receipts, and distinguishes a reply draft from delivery. Current owner instructions outrank saved preferences/defaults within host-granted capabilities; a single instruction does not silently become standing policy.

### What belongs in each skill

**Intake** selects useful information by default, locates current related notes, preserves source references and qualifications, and avoids one-note-per-input behavior. For Import full note, it preserves substantive wording and structure within supported formats, records coverage gaps, and prepares a protected reference. It uses maintained converters rather than asking the model to invent missing content. Original files remain external; temporary material follows the cleanup contract.

**Maintain notes** applies the ordinary wiki policy: read related current knowledge, choose update/create/thread based on stable subject and reuse, and maintain coherent entity/project, concept/process or synthesis pages as writing conventions. It retains evidence attribution, dates, constraints, uncertainty and direct input versions, with supported Link relationships. Knowledge queries save useful synthesis only within authority; bounded topic reviews inspect duplicates, missing support, contradictory evidence and affected dependencies. Personal writing and drafts remain supported. Protection is independent of page role, and a generated page cannot displace a protected decision. Archive superseded ordinary material only when justified; keep routine success in What changed. Packaged bundle `brn-threads-guidance-3` recorded this guidance change; [controlled comparison](../work/active/ordinary-wiki/evidence.md) distinguishes it from measured model outcomes. Bundle `brn-threads-guidance-4` adds discovery through `search_notes` (2–3 reformulations, exact totals, cursors) and `note_connections`, the recommended relation names `source`, `supersedes`, `contradicts` and `related` (others stay allowed), and the host-named root Topics note ([knowledge and retrieval plan](../work/active/knowledge-retrieval/plan.md)).

**Resolve conflict** gathers the competing records, distinguishes reporting from authority, and states the smallest decision needed. It proposes a concrete change when useful and applies a clear owner answer without demanding a redundant approval. It preserves unresolved qualifications and treats protection changes as real operations, not shortcuts.

**Prepare reply** drafts from supported current facts, identifies missing consequential information, and links the draft to its source and relevant Action. It may finish an internal drafting task when the requested draft is saved. It does not complete a human obligation, claim that the recipient agreed, or send externally in the first release.

Each skill should include one ordinary success example and a few relevant boundaries. New semantic behavior should usually require guidance and scenario examples, not new persistent workflow types.

## Tools, authority, and continuation

Expose a small capability vocabulary: search/read current records; inspect an available source; prepare/apply a grouped change; add thread context or attention; inspect history; and request compensation. Mutating thread and Action operations use the same service as note writes. Exact Rust signatures belong in the core implementation, rather than another duplicated schema here.

Tool outcomes are structured: Applied with receipt, Needs review with candidate, Deferred with guarded members, Stale with changed versions, Rejected with reason, Interrupted, or Failed. Preparation returns a durable candidate identity. The host records request identity before dispatch, preparation itself is replayable, and apply retries return the existing receipt. The agent responds to these outcomes rather than guessing from prose or regenerating IDs after a timeout. Change inputs default to Current and must still match their active version when Apply commits. Use Historical only for an intentional accepted-revision citation; it does not assert current truth. A stale input needs a refreshed candidate, not broader authority.

Host-only `ToolContext` carries the applicable grant. A grant binds a trusted instruction or review to target IDs and the allowed operation: changing A's deadline does not authorize archiving A or changing protected B. A record's presence in thread context or its mention inside an imported source confers no permission. The model can propose targets; the host enforces the actual bound scope. A clear owner instruction can authorize the concrete change without another click.

After a crash, an unfinished run becomes Interrupted. Continue starts a fresh run from deliberately retained conversation, current linked records, operation/candidate receipts, useful progress, and source references. Recheck current authority and versions. Avoid opaque persisted runtime checkpoints containing raw fetched bodies. Deliberately retained excerpts, source metadata, and structured receipts are legitimate records; blanket rejection of all tool-derived information is unnecessary.

## Two concrete journeys

### Email to a supported reply

1. The owner supplies Mari's email about Project A. BRN records the source reference, loads intake guidance, and finds the existing project thread, current notes, and Open delivery Action.
2. It updates the ordinary project summary with the email's supported information and creates a Suggested Action if the message requests new work. The atomic result appears in What changed.
3. The email asks for 15 November while a protected decision records 22 November. BRN keeps that decision unchanged and adds one Conflict reason: “Keep 22 November or accept the requested 15 November?” The thread shows both sources.
4. The owner answers, “Keep 22 November and draft a reply explaining it.” The run has authority for that instruction. It records the resolution and prepares the reply using the unchanged decision, without another approval question.
5. The reply draft is saved in the thread for copying to Outlook, with source and decision links. The internal drafting result can be complete; the app never claims that the email was sent.
6. The drafting discussion may be archived while the delivery Action remains Open or Waiting for its real outcome. Sending a draft outside BRN is not inferred from closing the thread. Completion requires applicable owner confirmation or execution evidence.

### Full process-description import

The owner chooses Import full note for a DOCX process description. BRN converts it into one readable protected note, retaining section order, numbered steps, roles, prerequisites, exceptions, warnings, necessary tables and diagrams, and meaningful references. It records the external source and conversion coverage. Repeated page furniture may be removed; substantive paragraphs cannot become a summary.

The owner can read the whole process offline with section navigation and ask questions in a linked thread. A working checklist or summary can be created separately without rewriting the protected reference. If a necessary diagram or section could not be read, the saved note is visibly Partial with the location identified. Mathematical equation support is outside the initial scope and is not a release gate; representative proof documents should reflect the owner's actual use. Importing a process does not automatically adopt it as the owner's policy.

## Qualification status

The core, provider runtime, packaged guides and Phase 0/1 hardening are implemented and merged through PR #120. [Status](../status.md) records the active cleanup, deferred native observation and remaining recovery/owner acceptance. Adjust semantic guidance using representative observed outcomes and keep deterministic integrity checks in the shared service.

## Implemented response and authority contract

Production uses published Rig 0.44.0. Streamed text/events update provisional UI. The successful structured final response replaces that preview and supplies the one host-owned assistant Message. Intermediate tool-loop text is not a second saved answer. Failed, interrupted, token-truncated or unknown termination remains partial progress; EOF cannot create completion. Preview and final answer both have a 128 KiB bound. Invalid/unknown calls and malformed arguments never dispatch, including other otherwise valid calls in the same proposed batch. Retry budgets are explicitly zero, tool rounds default to eight with a bounded final turn, and unknown finish reasons are rejected.

Ordinary working-note maintenance remains permitted after the owner explicitly selects it and while it is enabled. Fresh or legacy unchosen Settings remain pending: show the two policy choices on Home/Settings, retain manual work and scoped direct instructions, and require a choice before Resume. Review-first selection produces candidates for owner review; it does not grant standing application. Provider configuration never selects delegation. A protected content edit uses the contextual **Edit note** dialog: target ID, displayed version, separate owner instruction, content-edit class, actual owner Message identity and current run/settings fence. Dirty, recoverable or stale notes and pending/busy work refuse that action; ordinary chat/queue/Steer add no extra grant. **Save as decision** on a completed saved reply lets the owner review/edit title and text, then explicitly retains the exact reviewed content in general BRN as a protected, confirmed note linked to its chat, with source-message/version provenance, checked replay and Undo. It neither invokes a model nor promotes the chat or its documents; private figures must be retained explicitly or removed from the reviewed text. Cancel makes no change. CLI `ask --edit-note ID --base VERSION` and `ask --record-decision yes` keep their scoped invocation grants; the latter permits one new confirmed protected decision and binds its concrete target once prepared. The text may be normal English or Estonian; there is no magic phrase. Sources, quotations and historical messages cannot create these host modes. The grant expires with that invocation; Continue carries no old grant. Existing deterministic policy still invalidates prior confirmation when accepted content changes; a content grant is not a new confirmation or permission to unprotect/archive.

Every run registers nine tools: `read_records`, `read_note`, `read_source`, `read_skill`, `prepare_changes`, `apply_changes`, `retain_progress`, `read_history`, `refresh_candidate`. When the host supplies a search handle (the desktop and CLI do), it also registers `search_notes` and `note_connections`, bound to the run's chat; without one they are not offered and nothing substitutes for them. It also owns payload-free activity labels. Long notes use exact-version UTF-8 byte ranges, bounded sections or continuations. History lists at most 32 accepted revisions per page and reads exact historical passages up to 128 KiB; it is not current truth. Refresh atomically prepares a fresh immutable candidate through checked service authority, retires the stale candidate, checks current bases/inputs and grants no extra apply permission. Conversation Messages, settings, runs and receipt authority are host-owned.

[Runtime evidence](../archive/threads-rebuild/evidence.md#rig-044-runtime-integration) distinguishes automated/live success from the remaining locked-Mac visual gate and separately pending owner acceptance.
