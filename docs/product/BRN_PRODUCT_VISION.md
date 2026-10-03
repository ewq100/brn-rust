# BRN Product Vision

**Status:** Product vision / requirements source of truth\
**Purpose:** Define what BRN should do from the user's point of view.\
**Audience:** Product owner, architecture reviewers, planning agents, implementation agents, and reviewers.\
**Important:** This document describes the desired product behavior. Existing repository architecture, plans, code, and historical documents are implementation evidence, not higher-priority requirements. If they conflict with this document, surface the conflict instead of silently preserving the old design.

---

## 1. Product definition

BRN is a **chat-first personal work and knowledge assistant** with a Markdown vault underneath it.

It is not primarily:

- a traditional notes editor,
- a search application,
- a task manager,
- a document management system,
- or a generic AI chat client.

It combines those capabilities where needed so that a user can bring in work information, understand what it means, keep current knowledge clean, track unfinished work, create outputs grounded in trusted information, and keep control over every durable change.

The main user interaction is conversation with BRN. The vault is the durable knowledge base. Operational state such as actions and chat sessions may live outside the vault when that is more appropriate.

---

## 2. Core product principle

### Read and investigate autonomously; change durable state only through approval

BRN may autonomously:

- search the active vault,
- inspect related notes,
- follow relationships,
- inspect projects and people,
- inspect actions,
- inspect communication threads,
- compare conflicting evidence,
- use web search,
- delegate bounded work to approved AI subagents,
- reason about stale or contradictory information,
- propose new knowledge,
- propose actions,
- propose reorganizations,
- propose document rewrites,
- propose profile updates.

BRN must not silently make durable changes.

Durable changes require explicit user approval, including:

- creating or updating a Markdown note,
- promoting source information into curated knowledge,
- creating an action,
- changing an action,
- creating or changing a person profile,
- creating or changing a project profile,
- changing archive/current state,
- reorganizing the vault,
- resolving a conflict,
- turning a web fact into permanent knowledge.

A proposal may be edited, approved, or rejected.

---

## 3. Trust model

BRN must never:

1. Invent a fact and present it as known.
2. Silently overwrite approved/current knowledge.
3. Silently create or complete actions.
4. Lose source provenance.
5. Hide material conflicts or uncertainty.
6. Change AI provider or model without the user's knowledge.
7. Permanently delete meaningful information without a recoverable path.
8. Treat AI-generated text as equivalent to approved knowledge.
9. Guess when identity, relationship, source authority, or meaning is materially uncertain.
10. Mark a real-world action complete merely because BRN drafted or approved an output.

If the user's request conflicts with approved current knowledge, BRN should surface the contradiction and ask for clarification rather than blindly producing a misleading result.

---

## 4. Knowledge model

BRN should distinguish at least these conceptual layers.

### 4.1 Source evidence

Source evidence preserves what actually came into BRN.

Examples:

- email,
- Teams conversation,
- meeting notes,
- DOCX,
- PDF,
- PowerPoint,
- imported webpage,
- original user-supplied source material.

Source content should be preserved faithfully enough that the user can later ask:

> "What did Anna actually say?"

and receive the original source wording rather than BRN's rewritten summary.

BRN's interpretation must be separate from source content.

### 4.2 Curated current knowledge

Curated knowledge represents what BRN currently understands as approved/current truth.

Examples:

- current project status,
- current supplier,
- approved decisions,
- current process,
- person profile,
- project profile,
- approved policy,
- approved factual summary.

Current knowledge should be explicit where practical rather than relying only on inference from prose.

### 4.3 Historical / archived knowledge

Superseded and completed material remains available for:

- history,
- audit,
- examples,
- precedents,
- "how did this change?" questions.

Archived material should not normally influence current-state answers.

It may still be used as historical/reference knowledge when relevant, but must be clearly labeled as historical.

### 4.4 Operational state

Some information belongs in BRN's operational system rather than as ordinary Markdown.

Examples:

- open actions,
- waiting actions,
- blocked actions,
- completed actions,
- due dates,
- follow-up dates,
- action dependencies,
- chat sessions,
- Needs Review items,
- proposal state.

Important long-term knowledge must not exist only in opaque BRN state.

---

## 5. Main user journey: incoming request to completed work

Example:

A new email arrives with multiple questions about the Serna project.

The user adds the email to BRN.

BRN should understand:

- this is related to Serna,
- person X is waiting for an answer,
- the email contains specific questions,
- there is likely an open action,
- the email may be part of an existing thread.

During inbox processing BRN should propose:

- converted Markdown source,
- thread relationship,
- project/person relationships,
- one or more actions,
- due/follow-up dates when present,
- possible knowledge updates,
- conflicts with current knowledge.

The user approves, edits, or rejects the proposals.

Later the user asks:

> "Are there any open actions?"

BRN should surface the unanswered Serna request.

The user says:

> "Write an answer."

BRN should:

1. Search relevant current vault knowledge.
2. Inspect related project/person/thread context.
3. Use archived material only when relevant and clearly historical.
4. Use web search if useful.
5. Draft the response.
6. Clearly distinguish:
   - supported vault facts,
   - external/web facts,
   - BRN proposals/inferences,
   - unknown information,
   - conflicting information.

If the user asks something that the vault does not establish, BRN must say so.

Example:

> "The house color has not been decided. The only relevant information I found is a meeting note from two days ago where green versus red was discussed."

The user reviews the reply, adds comments, and presses Rewrite.

BRN rewrites the full proposal.

This may repeat several times.

When satisfied, the user approves the final reply.

The user sends it manually outside BRN.

The action remains open until the user explicitly says the reply was sent/completed.

BRN then marks the action complete.

Important new knowledge from the communication may be proposed for promotion into current project knowledge.

---

## 6. Document creation workflow

Example:

> "Help me write a process for how our company buys paint."

BRN should:

1. Search relevant current vault knowledge.
2. Find analogous processes, roles, policies, job descriptions, and project knowledge.
3. Build a first draft grounded in that evidence.
4. Show lightweight source provenance.
5. Clearly identify where BRN is proposing content rather than reporting known facts.
6. Use web search when helpful.
7. Ask or flag when there is insufficient information.
8. Allow iterative comment -> Rewrite cycles.
9. Approve the resulting document atomically.

If web information is used, the user should be able to see that it came from the web.

Useful external facts may later be proposed for durable knowledge capture.

---

## 7. Inbox and semantic ingestion

The Inbox is the primary v1 intake mechanism.

The user deliberately adds copies of information into BRN.

Supported intake should include, as practical:

- email content,
- Teams conversation content,
- Markdown/text,
- DOCX,
- PDF,
- PowerPoint,
- URLs/webpages,
- other common office documents where useful.

### 7.1 Inbox processing

BRN should:

1. Show what is waiting in the Inbox.
2. Allow individual or batch processing.
3. Convert source content into Markdown.
4. Preserve source meaning and structure as faithfully as practical.
5. Preserve provenance.
6. Extract useful images/charts/diagrams/tables.
7. Interpret important visuals so the AI can understand them later.
8. Suggest note type.
9. Suggest frontmatter/metadata.
10. Suggest project/person relationships.
11. Suggest where the note belongs in the vault.
12. Suggest actions.
13. Suggest project/person/decision updates.
14. Detect likely replacement/version relationships.
15. Detect conflicts with current knowledge.
16. Flag uncertainty instead of guessing.

The user should not need to design the vault structure manually.

### 7.2 Visual material

Example: a DOCX contains an illustration of a man holding a paint can.

The Markdown conversion should preserve the extracted image and include a local interpretation such as:

> Source visual: illustration of a man holding a paint can.

For complex diagrams/charts:

- preserve the visual asset,
- preserve structured data where practical,
- add an interpretation,
- do not silently discard meaningful information.

If BRN cannot preserve meaningful source information reliably, processing is incomplete and the original intake file must not be deleted yet.

### 7.3 Original intake files

The assumption is that files placed in the Inbox are disposable copies.

After successful semantic conversion and user approval, the original intake copy can be deleted.

If meaningful conversion is incomplete, keep the original and flag the issue.

---

## 8. Imported sources vs working documents

Imported source material should normally be immutable evidence.

If the user wants to rewrite an imported document:

1. preserve the imported source,
2. create a separate working/rewrite proposal,
3. iterate through comments and rewrites,
4. approve the rewritten document as current,
5. archive/hide the original source from normal current-state retrieval while preserving it as evidence/history.

If a newly imported document appears to replace an existing current document, BRN should detect the relationship and propose:

- making the reviewed new version current,
- moving the previous current version to history,
- preserving the imported source as evidence.

---

## 9. Current truth, superseding, conflicts, and staleness

### 9.1 Current truth

Current-state questions should use current approved knowledge by default.

Example:

If the vault contains Serna plan v1, v2, and v3, and v3 is current:

> "What is the Serna plan?"

should use v3 rather than mixing all versions.

### 9.2 Superseding information

If new information appears to supersede current knowledge, BRN should detect it and propose a resolution.

Example:

Current approved knowledge:

> House color: dark red.

New meeting notes:

> The team decided to change it to blue.

BRN should flag the likely supersession and propose updating current knowledge while preserving the previous value as history.

### 9.3 Unresolved conflict

If two current/recent sources disagree and BRN cannot reliably resolve them, it should maintain a visible unresolved conflict.

Example:

- Anna's email says blue.
- meeting notes say green.

BRN should say the color is unresolved rather than choosing one.

### 9.4 Source authority

Source authority depends on context, reliability, and recency.

General principles:

- approved internal decision notes normally outrank older emails,
- a newer email may indicate that an approved decision changed,
- vault sources normally outrank web sources for company/project-specific facts,
- reliable authoritative web facts may challenge incorrect vault knowledge,
- AI-generated proposals do not outrank approved knowledge.

Do not reduce all authority to a simplistic universal numeric score unless there is a concrete need.

### 9.5 Staleness

BRN should proactively flag potentially stale current knowledge.

Example:

> "Supplier decision expected by June 2026"

with no later update by October 2026.

BRN should create a Needs Review item such as:

> "This current knowledge may be outdated. I found no newer decision."

---

## 10. Proactive review and maintenance

BRN should detect issues in three ways.

### 10.1 Event-driven

When new Inbox material is processed, BRN checks for:

- conflicts,
- superseding information,
- likely actions,
- unresolved questions,
- useful knowledge promotions,
- relationship changes,
- identity ambiguity.

### 10.2 During normal work

While chatting or drafting, if BRN encounters something suspicious or contradictory, it should surface it if relevant to the current task.

Unrelated issues should go quietly into Needs Review.

### 10.3 Scheduled maintenance

BRN should periodically scan for:

- stale knowledge,
- unresolved conflicts,
- neglected actions,
- outdated external facts,
- missing consolidation,
- potentially incorrect information,
- knowledge that should be archived/superseded,
- important source facts not yet promoted.

No always-running daemon is required.

When BRN starts, it checks whether scheduled maintenance is due. If a weekly scan was missed while BRN was closed, run it after launch.

Scheduled scans may use web search automatically.

Results go to Needs Review.

---

## 11. Needs Review

BRN should have a first-class non-disruptive review queue.

Possible items:

- conflicting current information,
- stale knowledge,
- uncertain identity,
- uncertain project relationship,
- new information that may supersede current knowledge,
- external fact that contradicts vault knowledge,
- suggested project/person/profile updates,
- suggested consolidation notes,
- unresolved conversion problems.

Non-critical unrelated findings should not interrupt the user's current task.

A small indicator/badge is enough.

If the issue directly affects the current task, BRN should surface it in context.

---

## 12. Actions

Actions are persistent operational state.

### 12.1 Action creation

BRN may propose actions from:

- emails,
- Teams conversations,
- meeting notes,
- documents,
- chat sessions,
- manually stated user requests,
- larger goals that need decomposition.

Actions become real only after approval.

### 12.2 Action states

At minimum:

- Open
- Waiting
- Blocked
- Completed

### 12.3 Action fields

Support as needed:

- title/description,
- status,
- owner,
- related person,
- related project,
- source/thread,
- due date,
- follow-up date,
- waiting since,
- dependencies,
- parent/sub-actions,
- optional priority,
- completion time.

### 12.4 Completion

BRN never assumes a real-world action is complete.

Example:

- BRN drafts reply.
- user approves reply.
- action remains open.
- user sends email manually.
- user tells BRN "sent it."
- action becomes completed.

### 12.5 Follow-up work

If a completed action receives new follow-up work, create a new related action rather than reopening the completed action.

### 12.6 Waiting

Actions may be in Waiting state.

Example:

> Waiting for Anna's final paint specification.

When new material from Anna/the thread arrives, BRN should notice that the waiting condition may have changed.

### 12.7 Dependencies

BRN should understand dependencies.

Example:

- Ask Anna for specification.
- Send purchasing request, blocked by Anna's specification.
- Place order, blocked by purchasing approval.

Planning suggestions should account for blockers.

### 12.8 Larger goals

BRN may propose breaking a larger goal into smaller dependent actions.

Nothing becomes an actual task until approved.

### 12.9 Priority

Priority is optional.

The user may set High / Normal / Low.

BRN may propose a priority.

Otherwise practical urgency may be inferred dynamically from:

- due date,
- follow-up date,
- who is waiting,
- project importance,
- blockers,
- age.

### 12.10 Action storage

Use a hybrid model:

- ordinary actions live primarily in BRN's operational system,
- complex/important actions may have a related Markdown note when useful.

Do not create one Markdown file per trivial task by default.

### 12.11 Recurring actions

Not required for v1.

---

## 13. Projects

Projects are first-class knowledge concepts.

A project should have:

### Durable approved Markdown

Examples:

- current project summary,
- goals,
- key decisions,
- current state,
- important durable project facts.

### Dynamically generated operational context

Examples:

- open actions,
- waiting/blocked actions,
- recent communications,
- recent changes,
- unresolved conflicts,
- related people,
- relevant source documents,
- milestones/dates.

BRN should be able to answer:

> "What is the current state of Serna?"

with one coherent picture.

Projects have a lifecycle:

- Active
- Completed
- Archived/history

Completed/archived projects should not normally pollute current-state answers.

Archived project material may still be used as historical examples or precedents when appropriate.

---

## 14. People

People are first-class knowledge concepts.

BRN should be able to resolve references across sources:

- Anna Smith
- Anna S.
- anna@company.com
- Teams identity

when evidence is sufficient.

If identity is uncertain, BRN should not guess.

People profiles may contain:

- full name,
- aliases,
- known identifiers,
- role,
- company/team,
- projects,
- important commitments,
- relationships,
- open/waiting actions,
- related threads/source notes.

BRN should be able to answer:

> "What am I waiting for from Anna?"

and:

> "Show me everything important involving Anna and Serna."

---

## 15. Consolidation notes

BRN may proactively propose creating durable consolidation notes when scattered information would benefit from one.

Examples:

- project profile,
- person profile,
- decision note,
- process note,
- topic summary.

Creation requires approval.

---

## 16. Note types and structure

BRN should use a small explicit set of note types rather than treating every file as totally generic.

Initial candidate types:

- project
- person
- decision
- process
- source
- meeting
- topic
- draft

Keep the set small.

Add new types only when a real workflow requires them.

BRN should manage:

- note type,
- frontmatter,
- status fields,
- current/superseded state,
- relationships,
- structural metadata

automatically.

The user should not need to maintain metadata manually.

---

## 17. Vault organization

BRN should help organize the vault.

When new content arrives, BRN should propose where it belongs based on existing context.

If the current vault structure is inconsistent, BRN may propose structural improvements.

Example:

> "I recommend creating company/processes/purchasing and moving these three related notes there."

No reorganization happens without approval.

BRN should also support initial analysis/cleanup of an existing messy vault:

- infer note types,
- infer relationships,
- identify duplicate/superseded information,
- identify projects/people,
- propose folders/structure,
- identify archive candidates.

---

## 18. Relationships and graph

BRN should proactively maintain meaningful relationships between notes.

Examples:

- project ↔ person profile,
- project ↔ decision,
- project ↔ source email/thread,
- current document ↔ previous/source version,
- process ↔ related process,
- meeting ↔ decision.

For v1, the visual graph can focus on **relationships between Markdown notes**.

It does not need every action or internal database record to become a graph node.

Primary v1 graph use case:

> visually explore how notes are connected.

Graph relationships should be proactively maintained by BRN.

Clear relationships may be added automatically.

Uncertain relationships should be surfaced for confirmation.

---

## 19. Chat

Chat is the primary interaction model.

### 19.1 Session model

Chat behaves more like Codex sessions than one permanent conversation.

Requirements:

- multiple sessions at the same time,
- resume sessions,
- new sessions start relatively fresh,
- sessions do not automatically become durable truth,
- vault knowledge is shared long-term memory across sessions.

### 19.2 Capturing session outcomes

Knowledge capture works both ways:

- the user can explicitly say "save this decision" / "create an action",
- BRN can proactively propose important outcomes it notices.

Possible session outcomes:

- new action,
- new decision,
- project update,
- person update,
- new note,
- preference,
- conflict.

### 19.3 Session lifecycle

- Active sessions remain visible.
- Automatically archive after 30 days of inactivity.
- Manual Archive.
- Restore.
- Delete.

Before deleting a session, BRN should check for likely uncaptured durable outcomes and warn the user.

Deleting a session must not delete durable knowledge already captured from it.

---

## 20. Search and retrieval

Search is important infrastructure, but not the main user-facing workflow.

The user expects to ask:

> "Show me the mail from Bob last week about hammers."

rather than operate a complex search UI.

Search should support:

- current knowledge retrieval,
- source retrieval,
- person/project context,
- historical retrieval when explicitly relevant,
- relationship-aware retrieval where useful.

Current-state answers should avoid archived/superseded material unless relevant.

Approximate target scale:

- up to ~5,000 active notes,
- potentially much larger archive/history.

Prefer simple personal-scale indexing that can be rebuilt rather than enterprise infrastructure.

---

## 21. Provenance

Normal answers should stay readable.

Factual claims should have lightweight source indicators that can be clicked/expanded.

BRN should distinguish:

- supported by current vault,
- supported by source evidence,
- supported by web,
- inferred/proposed by BRN,
- unknown,
- conflicting.

The user should be able to inspect where an important claim came from without every answer becoming visually heavy.

---

## 22. Web usage

BRN may use the web automatically when useful.

Examples:

- document drafting,
- fact verification,
- normal chat,
- inbox analysis,
- scheduled maintenance.

Web information may challenge vault knowledge.

Example:

If the vault says the United States uses EUR and reliable external sources say USD, BRN should flag the issue.

Useful external facts may be proposed for durable capture.

Durable web-derived knowledge should preserve provenance such as:

- source URL,
- source/publisher,
- retrieval date,
- context.

If later scheduled maintenance finds that approved external knowledge may be stale, BRN should flag it and propose an update rather than silently replacing it.

---

## 23. Writing and review

Most substantial writing should follow:

**chat/request -> full proposal -> user comments -> Rewrite -> review -> Approve**

Approval is atomic at the proposal level.

The user does not need per-word/per-change Git-style acceptance for v1.

If one operation produces several unrelated consequences, use separate atomic proposals grouped in one review screen.

Example inbox review:

- import/convert source,
- create action,
- update project,
- update person profile,
- connect thread.

Each may be approved/rejected separately.

Provide an optional **Approve all** for the batch.

---

## 24. Manual note editing

Manual note editing is secondary.

BRN should support:

- viewing notes,
- small corrections,
- adding comments,
- reviewing proposals,
- approving changes.

It does not need to become a full Obsidian/VS Code-class editor in v1.

---

## 25. Home experience

BRN should be chat-first.

Default opening experience:

- prominent chat,
- easy access to dashboard,
- compact visibility into important operational state.

Dashboard should include, as appropriate:

- open actions,
- waiting/blocked work,
- Inbox,
- Needs Review,
- active projects,
- overdue/follow-up items.

No macOS system notifications are required in v1.

Attention stays inside BRN.

---

## 26. Work planning

BRN may suggest what to focus on.

If the user asks:

> "What should I focus on today?"

BRN may consider:

- open actions,
- due dates,
- follow-up dates,
- who is waiting,
- blockers,
- project importance,
- unresolved review items,
- recent communications.

It should explain why it suggested the order.

The user decides priorities.

---

## 27. Calendar and dates

For v1:

- recognize dates/times/deadlines,
- attach them to actions/projects,
- support due and follow-up dates.

Do not build an internal calendar.

Do not require direct Outlook/Google Calendar integration in v1.

Calendar connectors may come later.

---

## 28. AI providers, models, and subagents

The user explicitly selects:

- provider,
- main model,
- thinking/reasoning effort.

Example choices may include:

- GPT-6.1 Sol via ChatGPT,
- GPT-6 Astra via Copilot.

BRN should keep using the selected provider/model until the user changes it.

No silent provider/model fallback.

Transient failures may be retried a small number of times on the same provider/model.

BRN may automatically delegate bounded helper tasks to approved subagents/models.

Example helper work:

- retrieval,
- classification,
- inbox triage,
- bounded research,
- evidence gathering.

Example cheaper/faster helper model: GPT-6 Luna.

Rules:

- main agent owns the conversation and final result,
- subagents do narrow delegated work,
- subagents do not independently modify durable BRN state.

---

## 29. AI activity visibility

Normal use should show a lightweight activity trail.

Example:

> Searching vault -> checking 8 sources -> web verification -> drafting

The user may expand for more detail.

Do not expose raw agent/tool transcripts by default.

---

## 30. Failure behavior

BRN should fail clearly.

Examples:

- provider rate limit,
- provider unavailable,
- web search failure,
- document conversion failure,
- corrupted file,
- invalid subagent result.

Behavior:

1. retry the same operation/provider a small number of times when the error appears transient,
2. never silently switch provider/model,
3. preserve useful partial output where safe,
4. show what succeeded,
5. show what failed,
6. state what remains unchanged.

---

## 31. Offline behavior

Without internet/AI provider access, BRN should still support:

- vault browsing,
- local note viewing/editing,
- existing actions,
- project/person views,
- local search over existing indexes,
- review of existing proposals/data where possible.

AI-dependent capabilities should clearly show that AI is unavailable rather than breaking the entire app.

---

## 32. Vault ownership and external edits

The vault remains a normal Markdown folder.

BRN is the expected primary writer/organizer.

Supported:

- inspect/copy/backup vault outside BRN,
- external edits while BRN is closed,
- detect those edits at next scan/open.

Not required:

- robust simultaneous live editing with Obsidian/VS Code/other editors,
- complex cross-editor coordination.

If a file changes externally while BRN is open and before BRN saves, BRN should detect the conflict and avoid silently overwriting it.

---

## 33. Version history and recovery

BRN does not need a full version-control UI.

Required:

- show proposed changes before approval,
- approved change becomes current,
- important superseded knowledge can move to history/archive,
- simple recent undo/recovery where practical,
- deeper historical recovery may rely on Git/Time Machine.

---

## 34. Backup and internal data recovery

The Markdown vault must remain independently readable.

BRN should automatically back up internal state.

If internal storage is corrupted:

- detect it,
- restore the newest valid backup where possible,
- tell the user what happened.

Derived search/index data should be disposable and rebuildable.

Important operational state such as actions, sessions, proposals, and review state should be recoverable where practical.

---

## 35. Deletion and trash

Deleting important knowledge should be recoverable.

Default behavior:

- move to trash/archive first,
- permanent deletion requires a separate explicit action.

---

## 36. Portability

If BRN disappeared tomorrow, most long-term knowledge should remain understandable.

Requirements:

- Markdown notes are ordinary files,
- images/assets are ordinary files,
- frontmatter and links use simple documented conventions,
- long-term knowledge is not trapped only in SQLite,
- operational state may use SQLite,
- eventually support export of actions/sessions/other operational data.

---

## 37. Personal working profile

BRN should maintain durable knowledge about the user's working preferences separately from project facts.

Examples:

- prefer concise supplier emails,
- language preferences,
- preferred process-document structure,
- recurring terminology,
- formatting preferences.

BRN may infer possible preferences from repeated approved outputs and propose them for approval.

Only approved/final outputs should teach persistent style/preferences.

Precedence:

**current explicit instruction > saved preference > default behavior**

A one-off instruction does not automatically rewrite durable preferences.

---

## 38. Language support

Mixed-language usage is normal.

At minimum, BRN should comfortably support mixed English and Estonian content and conversations.

BRN should normally answer in the language the user is currently using unless asked otherwise.

---

## 39. Installation and user model

v1 targets:

- macOS,
- manual installation for the owner and a small trusted group,
- single user per installation,
- separate vault/state/credentials per user,
- no collaboration required in v1,
- no mobile client required in v1.

The app should have:

- reproducible app bundle/build,
- understandable setup,
- clear vault/data locations,
- provider sign-in,
- safe upgrades that preserve data.

Full public distribution/App Store-grade onboarding is not required.

---

## 40. Sync

BRN operational state may remain local to one Mac in v1.

The Markdown vault may be synced externally with tools such as:

- iCloud,
- Dropbox,
- Git,

but BRN does not need its own multi-device sync subsystem in v1.

---

## 41. Security

For v1:

- rely on macOS account protection and disk encryption,
- no separate BRN PIN/password required,
- no BRN-managed vault encryption required,
- store AI credentials safely,
- never expose credentials in logs, errors, fixtures, or vault files.

Relevant vault information may be sent to the explicitly selected corporate-approved AI provider without per-request confirmation.

---

## 42. Activity history

BRN should keep a human-readable history of important approved durable changes.

Examples:

- updated Serna project,
- created action,
- archived supplier process,
- changed Anna profile,
- promoted web fact into current knowledge.

The history should show, where practical:

- what changed,
- when,
- what proposal/session caused it,
- what object/note was affected.

This is separate from raw technical logs.

---

## 43. Note relationship graph

v1 should include a visual knowledge graph.

Initial scope:

- graph Markdown notes,
- show meaningful note-to-note relationships,
- support visual exploration of how knowledge is connected.

It does not need:

- every action as a graph node,
- graph-based conflict analysis,
- graph-native storage.

The graph is a view over relationships, not necessarily the primary datastore.

---

## 44. User-scale performance target

Design for personal scale.

Expected active set:

- up to roughly 5,000 active notes.

Archive/history may be larger.

Prefer:

- simple rebuildable indexes,
- understandable local storage,
- maintainable architecture,
- deterministic recovery.

Avoid enterprise-scale infrastructure unless a concrete requirement justifies it.

---

## 45. Product quality priority

For v1:

1. correctness,
2. trust,
3. data safety,
4. provenance,
5. predictable approval behavior,
6. reliable actions/inbox/project workflows,
7. recovery,
8. usability,
9. visual polish.

A rougher UI is acceptable if the product is reliable and safe.

---

## 46. v1 scope commitment

The user wants the full product capabilities in this document to define v1 rather than postponing major pieces into an undefined later generation.

Implementation should still happen incrementally and in reviewable slices.

BRN should not be considered ready for daily reliance until the v1 workflows are present and validated.

This does not mean every slice must be built at once.

It means the complete v1 target remains visible and intentional.

---

## 47. Explicit v1 non-goals

Do not add unless a later requirement changes this document:

- direct email sending,
- direct Teams sending,
- direct Outlook/Gmail/Teams ingestion connectors,
- internal calendar system,
- background macOS daemon,
- macOS system notifications,
- mobile client,
- multi-user collaboration,
- BRN-managed multi-device sync,
- recurring actions,
- enterprise scalability,
- full Git-style history UI,
- full professional Markdown IDE/editor,
- simultaneous multi-editor coordination,
- silent provider fallback,
- automatic durable writes by AI,
- public App Store-grade distribution.

---

## 48. Product acceptance examples

The following scenarios should eventually work end to end.

### Scenario A: Email request

1. Import email asking multiple Serna questions.
2. BRN connects it to the existing thread/project/person.
3. BRN proposes an action.
4. User approves.
5. Later asks for open actions.
6. BRN shows the request.
7. User asks for reply.
8. BRN drafts grounded answer.
9. Unknown facts are labeled unknown.
10. Conflicts are surfaced.
11. User comments.
12. BRN rewrites.
13. User approves.
14. User sends manually.
15. User says "sent."
16. Action completes.
17. Important new knowledge is proposed for project update.

### Scenario B: New process document

1. User asks BRN to draft a paint-purchasing process.
2. BRN searches current vault.
3. BRN finds related processes, roles, and evidence.
4. BRN optionally uses web search.
5. BRN distinguishes known vs proposed vs external content.
6. User iterates through comments and Rewrite.
7. User approves final process.
8. Durable knowledge and relationships are updated through proposals.

### Scenario C: Inbox DOCX

1. User drops a DOCX into Inbox.
2. BRN converts it semantically to Markdown.
3. Images/tables/diagrams are preserved and interpreted.
4. BRN proposes note type, metadata, relationships, destination, and actions.
5. User reviews proposals.
6. User approves all or selected items.
7. Markdown becomes durable.
8. Disposable intake copy is removed.
9. If conversion is incomplete, original is retained and issue flagged.

### Scenario D: Conflict

1. Current project says dark red.
2. New source says blue.
3. BRN detects likely supersession.
4. BRN proposes update.
5. User approves.
6. Blue becomes current.
7. Dark red remains historical.

### Scenario E: Unresolved conflict

1. Anna says blue.
2. Meeting note says green.
3. Neither clearly outranks the other.
4. BRN marks the fact unresolved.
5. Answers say it is unresolved.
6. Needs Review contains the conflict.
7. User resolves it.

### Scenario F: Work planning

1. User asks what to focus on today.
2. BRN looks at open, waiting, blocked, overdue, due, follow-up, and project context.
3. BRN suggests priorities.
4. BRN explains why.
5. User decides.

### Scenario G: Session capture

1. User has a long BRN session.
2. Session produces decisions/actions/project updates.
3. BRN proposes durable captures.
4. User approves selected outcomes.
5. Session later archives.
6. Durable knowledge remains independent of the session.

---

## 49. Rule for architecture and implementation agents

Architecture must serve these product behaviors.

Do not preserve complexity merely because it already exists.

Do not rewrite working components merely because a different design is theoretically cleaner.

For every subsystem, ask:

1. Which user requirement does this serve?
2. Is it technically necessary for safety/correctness?
3. Is there a simpler way to meet the same requirement?
4. Is the complexity current or speculative?
5. Can the user-visible behavior be verified?
6. Can the subsystem be replaced internally without breaking product behavior?

Prefer simple, well-bounded components with clear ownership and rebuildable derived state.

---

## 50. Rule for audits

When auditing the repository against this vision:

- treat this file as the requirements source of truth,
- treat current code/docs/plans as evidence of implementation state,
- identify conflicts explicitly,
- distinguish implemented vs verified vs user-accepted,
- do not infer "required" from historical architecture,
- do not modify code during the audit,
- do not recommend a rewrite unless a concrete product/technical problem justifies it,
- identify what should be kept,
- identify what should be simplified,
- identify what should be removed or deferred,
- identify what is missing,
- explain conclusions in plain language for a non-software product owner,
- cite concrete repository files/modules for technical findings.
