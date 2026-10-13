# Chats and Projects — Product Feature Brief

**Date:** 13 October 2026. **Owner:** Evo. **Status:** Stage-one behavior defined by owner decisions and the explicit request to build chat first. Include durable chat-only attachments, streaming, Queue, Steer and quiet display. Enter queues during work; Stop/failure/restart pause a saved queue until explicit Resume. [Chat-first plan](plan.md) owns implementation. Projects are deferred; this brief supplies no publication or merge authority.

**Origin:** [Thread concept confusion — shared conversation](https://chatgpt.com/share/6ace2dac-e238-83eb-94c1-90a4fa1d8a53), followed by the owner’s decisions in this discovery interview. Recommendations in the shared conversation are background, not additional accepted requirements.

## Feature

Ordinary **Chats**, with optional **Projects** that group related chats, documents and reference material and provide shared context and explicit project instructions. Replace the user-facing Threads/Resolve model with this conversation and organization model.

### Delivery order — owner selected

Deliver this in two independently assessable stages. Stage one must be a useful, dependable chat experience on its own; it must not require project setup or unfinished project controls. Start project implementation after the standalone chat stage meets its agreed acceptance and verification gates. Preserve all project decisions below for that later stage.

| Stage | Outcome and scope |
|---|---|
| **1 — Chats** | New and continued standalone conversations; saved history and findability; several chats linked to the same document; current document/editor/review workflows; chat-only attachments with explicit retention in general BRN; no Resolve/Reopen or quick-question auto-resolution; Archive stops work and preserves progress; actual attention and Actions remain separate. Preserve explicit provider/model/effort selection, drafts, streaming/final-answer integrity, Stop, Stop-and-send, fresh-invocation Continue, failure/retry/recovery and checked writes. Attachments retain readable content and meaningful figures after the original moves or disappears, with visible conversion gaps. |
| **2 — Projects** | Optional grouping; moving existing chats into/between/out of projects; shared document links across projects; project instructions; inherited BRN-knowledge setting; project-only visibility and earlier-project-chat context; deletion preview, unfinished-Action decisions, Trash and relationship restoration. |

Stage one adds files as context for that chat without automatically adding their content to general BRN; the owner explicitly chooses general retention. Existing general-BRN notes remain general knowledge, and existing explicit intake operations keep their contracts. Chat-only visibility must be enforced before attachments are offered. Existing stage-one chats/documents remain usable when projects arrive; adding grouping must not require recreating conversations or duplicating documents.

## Problem

“Thread” currently mixes an individual conversation with a continuing piece of work. Two conversations about the same document appear to be two work items, while the document and its wider context need continuity. The owner wants to talk freely and organize sustained work when useful.

## User outcome

Ask a standalone question without setting up a project or managing completion. For sustained work, open a project, read or write its documents, add references, continue a chat or start a fresh one with relevant project context. Keep project material separate from general BRN knowledge unless deliberately retained there.

## Core user stories

- As the owner, I can start an ordinary chat and later move it into a project, between projects, or back out as a standalone chat.
- As a writer, I can have several distinct chats about the same document, find those conversations from the document, and work beside the editor.
- As a project owner, I can add documents and references, maintain project instructions, and start a fresh chat that can find relevant earlier project discussion.
- As the owner, I can control use of broader BRN knowledge, archive a chat, and delete or restore a project without accidental promotion or lost obligations.

## Expected behavior

- Projects are optional. A new chat is one conversation containing successive messages; a fresh chat is an intentional separate conversation, without a mandatory goal or Resolve step.
- A project brings related chats, documents and references together. Its documents remain directly accessible without opening a conversation. Document links do not require copying the document.
- A fresh project chat prioritizes current project documents, saved decisions and explicit project instructions. It can search relevant earlier chats in that project; it does not need every previous message inserted into each new conversation.
- **Use BRN knowledge** is a project setting inherited by all its chats, with no per-chat override in this scope. The selected normal behavior allows relevant general BRN knowledge within existing permitted access. Switching it off stops new general-BRN lookups. Existing project documents and chat content remain readable and usable, including content previously informed by BRN knowledge.
- Project-only material is not general BRN knowledge and is not automatically available to unrelated projects. Sharing a document grants its content as context where linked, without granting access to other projects’ chats or instructions.
- Project instructions are explicitly set or edited by the owner and apply to every chat in that project. A one-off chat request does not become a standing project instruction. Existing instruction precedence and authority limits continue to apply.
- Discussion is evidence of what was discussed. A proposal is not an accepted document edit or owner decision. Existing source/protection/review rules determine current knowledge and accepted decisions.
- Stage one includes files attached as chat-only context. Keep their readable supported content and meaningful images with the saved chat so they remain usable when the original moves or disappears. Preserve visible conversion gaps. General BRN retention is an explicit owner choice, not an automatic side effect of attaching or archiving; original binaries remain external.
- The owner additionally requested a polished stage-one chat experience with streaming responses, **Queue** for follow-ups after current work, **Steer** for changing the direction of work already in progress, and a **quiet display** with less activity detail. Queue and Steer are distinct behaviors; quiet display is a presentation choice. [Research](research.md) separates documented product precedents from recommendations.
- While BRN is answering, **Enter queues the message**. A separate explicit **Steer** action redirects current work. Existing multiline composition remains available without accidentally sending a message.
- **Stop** stops current work and pauses the queue. Queued messages remain saved and do not run until the owner explicitly resumes the queue. It does not merely skip the active response and advance to the next message.
- **Failure or restart** leaves queued messages saved and paused until explicit owner Resume. Archiving a chat uses the same stop-and-pause behavior; opening or restoring it alone cannot resume work.

## Important concepts and state

| Concept | Product meaning |
|---|---|
| Chat | A conversation, either standalone or in exactly one project. It may link several documents and Actions. |
| Project | An optional grouping and context boundary, with shared owner instructions and a project-wide BRN-knowledge setting. |
| Document / note | One persistent record, which may be linked into several projects. Accepted edits appear everywhere that record is used. |
| General BRN knowledge | Material deliberately available outside project boundaries. Project creation/import and project deletion do not promote material into it. |
| Action | A separate obligation with its existing progress and completion rules. Chat existence, archiving or project restoration cannot complete it. |
| History / Trash | History preserves meaningful work. Trash retains deleted project contents for restoration and excludes them from ordinary active use. |

An existing general BRN document linked into a project remains independently retained. A document created or imported only for a project stays project-only until the owner explicitly keeps/moves it into general BRN or links it elsewhere. Moving a project chat out makes it standalone; deleting its project never does that automatically.

## Lifecycle

1. Create a standalone chat or an optional project. Add/link documents and references; optionally set project instructions. Existing intake choices, full-import protection and conversion coverage rules remain in force.
2. Start or continue chats. Each chat has zero or one project; moving it changes its project context for subsequent work and preserves its conversation history. Reading context or moving membership grants no additional write authority.
3. Answers remain in chat history. There is no Resolve/Reopen lifecycle or quick-answer auto-resolution. Reviews, questions, conflicts, blockers and unfinished Actions supply actual attention requirements.
4. **Archive chat** hides it from the recent list, stops active work and preserves conversation, accepted changes and useful progress. Archive does not complete Actions, approve proposals or erase outstanding review requirements. Finding archived material follows the existing explicit-access rule for archived material. Resuming requires an intentional continuation under current context and authority.
5. **Delete project** first shows the concrete chats and exclusive documents to be removed, the shared/independently retained material to be kept, and unfinished Actions. For each unfinished Action, the owner must choose to keep it outside the project or cancel it before confirming deletion.
6. Confirmed deletion stops work in the removed project, removes its grouping from active use, and moves its chats and exclusive documents to Trash. Shared documents and documents explicitly retained in general BRN survive. Project-only chats and documents never silently become standalone/general material.
7. **Restore project** restores its trashed contents, project settings/instructions and original relationships, while preserving later work. Shared records use their current content. Actions explicitly cancelled during deletion stay cancelled; Actions moved outside the project stay outside. Restoration does not restart AI work or automatically reopen an obligation.

## User decisions

The owner explicitly selected:

1. Project-prioritized context with optional broader BRN knowledge.
2. One project-wide BRN-knowledge setting inherited by every project chat.
3. Shared document links across projects; accepted edits update the same document everywhere.
4. Deletion removes project chats and exclusive documents, with a removal warning; neither is automatically moved into general BRN.
5. Recoverable project deletion through Trash, including relationship restoration.
6. Ordinary chat history and optional archiving, without Resolve/Reopen; attention and Actions stay separate.
7. Automatic search of relevant earlier chats in the same project, preserving the difference between suggestions and decisions.
8. Zero or one project per chat, with movement between project and standalone contexts.
9. An explicit keep-outside-project or cancel choice for unfinished Actions during project deletion; no inferred completion.
10. Turning off BRN knowledge stops new lookups but preserves usability of existing project/chat content.
11. Archiving a chat stops work and preserves progress.
12. Explicit shared project instructions; one-off requests do not become project rules.
13. Restoring a project does not undo the owner’s Action cancellation or movement decisions.
14. Build and qualify the standalone chat experience first, then introduce projects as a separate stage.
15. Include chat-only file attachments in stage one, with an explicit choice to keep their content in general BRN.
16. Stage one should be polished and include streaming, both Queue and Steer, and quiet display. The owner clarified that “quete” meant both queue and quieter display and explicitly requested steering too.
17. Stop ends current work and pauses the queue, keeping queued messages saved until explicit owner resume.
18. After failure or restart, the queue remains saved and paused until explicit owner Resume.
19. While BRN is answering, Enter queues the message; a separate Steer action redirects the current work.
20. Preserve readable attachment content and meaningful images with the chat, including after the original moves or disappears; show conversion gaps and retain an explicit general-BRN choice.

## Trust, approval and autonomy

Project membership and context settings control what material is available; they do not authorize edits to protected documents, new commitments or external effects. Preserve host-owned authority, exact displayed-candidate review, version checks, dirty-edit protection, receipts and Undo. Ordinary-note maintenance remains subject to the existing explicit workspace delegation and current permitted context; projects grant no new standing maintenance authority.

Deleting a project is a deliberate owner operation against the displayed removal set, not ordinary autonomous maintenance. Never hide shared/general records as exclusive contents. Local Undo or Trash restoration does not reverse external delivery or provider usage.

## Failure and uncertainty behavior

Failed changes preserve saved chats, documents, instructions, relationships and owner drafts. Project deletion must not leave an unreported partially removed project or partially applied Action choices. Retrying an uncertain operation checks its recorded outcome rather than deleting twice or duplicating restored records.

If content, membership, settings or Action state changed after the deletion preview, require a refreshed preview before applying the changed removal set. Preserve unsaved writing under existing dirty/stale guards. Failed restoration leaves recoverable contents intact; conflicts with subsequent work remain visible instead of overwriting that work.

Stopping/archive/deletion fences further local commits from the stopped run and preserves accepted results and useful partial progress. It cannot claim that upstream provider processing was necessarily cancelled. Continue uses a fresh invocation with current scope/settings and no inherited protected-write grant. Retained proposals whose inputs are no longer current require the existing refresh/review path.

Missing sources or unavailable provider access stay visible. Earlier chat suggestions and historical claims do not override current documents or confirmed decisions, and the agent does not invent missing context.

## Relationships to existing product behavior

This is a consequential amendment to the [canonical target](../../../architecture/threads-target.md), not a vocabulary-only change. At discovery, the target described projects as ordinary notes; the [behavior contract](../../../architecture/threads-behavior.md) gives threads Open/Resolved state; the [Omakase scope](../omakase-ui/spec.md) exposes resolve/reopen and quick-question auto-resolution. Those summaries and affected runtime guidance must be reconciled during normal planning/integration after brief acceptance.

Project-only visibility also qualifies today’s workspace-wide default note retrieval: material can live in the same authoritative store without being eligible for unrelated project/general retrieval. This applies consistently to desktop, agent and CLI surfaces.

Preserve the [invariants](../../../architecture/invariants.md): one checked write service and authoritative SQLite state, derived search, protection distinct from truth, readable imports with honest coverage, exact review, versioned Save, recovery, Action evidence, export and complete backup. Preserve existing note comments, linked conversations, rewriting and review alongside the editor. This brief does not replace those contracts or claim implementation.

## Examples

- **Japan trip:** separate chats check Hakone-to-Kyoto travel and revise Kyoto activities, both linked to one itinerary. Another fresh chat can find the earlier options.
- **BRN design guide:** several chats plan structure and rewrite an introduction using the same guide and references. “For beginners, in Estonian” is explicitly saved as a project instruction.
- **Travel writing:** the same reference document can be linked here and in Japan trip without a duplicate. Deleting Japan trip preserves that shared record.
- **Mari’s reply:** deleting a project requires a keep/cancel decision for “Send the reply to Mari.” Restoring the project cannot silently revive a cancelled sending obligation.

## Acceptance scenarios

| Stage | Situation | Observable success |
|---|---|---|
| 1 | Ask a standalone question, navigate away, then return after restart | The saved conversation remains findable and can be continued; no project, goal or Resolve step is required. |
| 1 | Receive streamed text and a successful final answer | The existing final-answer contract supplies one saved assistant answer; provisional progress does not become a duplicate or false completed answer. |
| 1 | Discuss the itinerary on two occasions | Two intentionally separate chats are discoverable from the same current document; neither creates another authoritative copy. |
| 1 | Stop work, send a replacement instruction, or recover from failure | Existing run fencing, saved messages/drafts, current-context continuation, selected-provider behavior and checked-operation retry guarantees remain intact. |
| 1 | Press Stop with queued messages waiting | Current work stops; all waiting messages remain saved in a paused queue; none launches automatically, including after the worker reports Finished. Explicit Resume is required. |
| 1 | Press Enter or use Steer during an answer | Enter saves a queued follow-up without interrupting the active answer; the separate Steer action explicitly redirects active work. |
| 1 | A response fails, or BRN restarts with queued messages | Waiting messages survive in a paused queue. Neither recovery nor opening the chat starts them; explicit Resume is required under current context and authority. |
| 1 | Archive a working chat | Local work stops, partial progress survives, the recent-list entry disappears, and linked obligations/review requirements are not marked complete. |
| 1 | Ask for a document rewrite and review it | Existing comments, frozen rewrite inputs, exact-candidate review, stale/dirty guards, Apply and receipt Undo remain usable alongside the chat. |
| 1 | Attach a file to one chat | Its content is available as that chat’s context without becoming general BRN knowledge; an explicit keep-in-BRN operation makes it independently retained. |
| 1 | Restart after moving/deleting an attached original | The saved chat still has supported readable content and retained meaningful images; conversion gaps remain visible. |
| 2 | Start a fresh Japan trip chat | Relevant earlier project discussion is findable; proposed options are distinguishable from the adopted itinerary and decisions. |
| 2 | Turn off BRN knowledge | All subsequent requests in the project stop new general-BRN lookups; existing project/chat content remains usable. |
| 2 | Keep a document only in Japan trip | An unrelated project cannot discover it as general BRN knowledge merely because broader BRN knowledge is enabled. |
| 2 | Edit a shared reference | One accepted edit is visible through both project links, subject to existing protection and review. |
| 2 | Move a stage-one chat to a project, another project, or back out | Conversation history survives without duplication; subsequent work uses the destination context; the chat never belongs to two projects. |
| 2 | Set instructions, then make a one-off request | New project chats use the saved instructions; the one-off request does not alter them. |
| 2 | Preview deletion with shared material and an unfinished Action | The preview distinguishes removed and retained content and requires an explicit keep/cancel Action choice. |
| 2 | Delete and restore Japan trip | Exclusive chats/documents go to Trash without becoming general material; restoration returns project contents and relationships. Shared/general documents survive throughout. |
| 2 | Restore after cancelling Mari’s Action or moving it out | The Action stays cancelled or outside the project, respectively; no sending obligation or AI run restarts automatically. |
| 2 | Deletion fails, its response is lost, or relevant content changes | Saved content/drafts survive; retry recognizes the existing outcome or refreshes the preview; no duplicate deletion, partial success claim or stale approval occurs. |

## Non-goals

Shared team projects, collaboration permissions, chats belonging to multiple projects, per-chat overrides of the project knowledge setting, automatic promotion into general BRN, automatic adoption of chat instructions/decisions, new Action states, autonomous external sending, parallel provider work and a general project-management engine. Permanent Trash purging/retention automation is outside this feature. No native qualification, release or real-data access is assigned here.

Stage one additionally excludes project creation/membership, project settings/instructions, cross-project document sharing and project deletion/Trash. Preserve existing document links and review instead of introducing project controls early.

## Open product questions

None material remain for the scoped stage-one build. Quiet display reduces intermediate activity detail while keeping streamed answers, progress, errors and decisions visible, as clarified during the interview. Queue items remain distinct follow-ups; automatic batching and historical conversation branching are not assigned. Existing partial-progress, checked retry and current-context continuation contracts apply. Low-impact labels and layout follow planning and current UI conventions. Stage-two decisions remain recorded and deferred.

## Implementation considerations

Plan stage one independently around the existing chat, linked-document, review and run foundations, plus the owner’s durable chat-only attachment decision. Qualify the full standalone journey rather than treating a Threads-to-Chats rename as completion. Avoid implementing project membership or project controls merely to anticipate stage two.

For stage two, determine how project membership, independent retention, context eligibility and Trash restoration fit the existing core and service. Context restrictions must cover actual agent reads, relevant-chat search and current-input validation, not only navigation. Plan adopted-schema upgrades and pre-upgrade backups wherever either stage needs them. Reuse the toolchain, lockfile, checked changes, guards, interruption/replay and review foundations; preserve unrelated provider/converter fixes.

Do not let context/index updates, shared-document references, retained candidates or recovery bypass the selected visibility and authority boundaries. Translate the acceptance scenarios into proportionate core/runtime/native checks during implementation; this interview supplies no test or native evidence.

## Handoff

Carry the requested chat-first work through normal technical planning, implementation, independent review and verification. Reconcile the canonical target and affected summaries for chat behavior first, preserving project requirements as deferred scope. Give the owner a concrete standalone chat candidate to assess before introducing projects. Report implementation, verification, native observation, acceptance and merge separately. Publication, merge, release and real-data access require their applicable authority; writing this brief supplies none by itself.

## Owner clarification: detailed display, 13 October 2026

Quiet off shows a terminal-style activity stream with tool calls, bounded argument previews, outcomes and the provider's explicitly public reasoning summaries when supplied. Quiet on hides this activity across the window while retaining streamed answers, compact truthful work/Stop status, errors, proposals and attention. Raw private reasoning and encrypted provider content are not display material. The latest run's bounded in-memory activity remains available in its own chat after completion, until another run or app restart; canonical answers/receipts remain durable.


Owner clarification (13 October): remove the Ask/Edit note/Record decision menu from the composer. Keep plain Enter/queue/Steer free of extra note/decision grants. Put Edit note with the exact open saved note and use a separate instruction dialog; put Save as decision on completed saved replies, with editable title/text review and explicit protected/confirmed BRN retention. Preserve unsent chat drafts, checked replay/Undo and private attachments. Saving reviewed decision text must not require another model call.
