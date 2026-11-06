# BRN UX Rebuild Specification

**Specification:** BRN-UX-002<br>
**Version:** 1.3<br>
**Date:** 6 November 2026 (1.3: chat simplification from [#249](https://github.com/ewq100/brn-rust/issues/249); 1.2: 5 November 2026)<br>
**Owner:** Evo<br>
**Audience:** The build agent, implementation agents and independent reviewers<br>
**Status:** The simple ChatGPT-inspired visual direction is approved. This package defines its implementation target, including light and dark appearance. Native implementation and acceptance remain separate work.

## 1 Purpose and approved direction

Rebuild BRN's desktop user experience around a quiet sidebar, one dominant workspace, clear writing surfaces and tools that appear when needed. Preserve BRN's supported capabilities and checked application services while replacing the current tiled presentation. Keep GPUI Kit as the native UI foundation.

Evo approved the reference concept with the words “exactly what I need” and “simple clean.” Treat that concept as the selected direction. The task is to carry its simplicity through the complete application, including the less visible workflows and failure states.

The main task must remain obvious. In a conversation, read and respond. In a document, read and write. In the Library, find and understand an article. A source, comment, permission request or agent trace supports that task without becoming a permanent competing workspace.

### 1.1 Required outcomes

- **UX-01:** Use one compact navigation sidebar and one primary content area. Keep only one message composer visible for the active conversation or writing context.
- **UX-02:** Make Chat, document writing and the article Library feel like parts of the same application. Preserve identities, drafts and context when moving between them.
- **UX-03:** Provide complete **System, Light and Dark** appearance choices. Dark mode is a full product theme, including editing, review, overlays, selection and failures.
- **UX-04:** Preserve all supported capabilities. Relocate and simplify their presentation; do not silently remove functionality because it is absent from the concept.
- **UX-05:** Preserve current service rules for scope, protection, delegation, saves, revisions, recovery, exact-candidate Apply and receipt-based Undo.
- **UX-06:** Finish the complete UX migration. A new sidebar around untouched legacy tile screens is an intermediate state, not completion.
- **UX-07:** Upgrade the active desktop and reusable writing UI to **GPUI Kit `=0.9.0`** as part of the rebuild. Complete the compatibility migration and dependency validation before expanding the new shell across the app.
- **UX-08:** Make long articles comfortable to read and navigate. Add a collapsible heading outline and consistent document presentation in both themes, using Mado as a reading reference within the selected BRN design.

### 1.2 Meaning of requirement words

**Must** identifies a completion requirement. **Should** identifies the expected default; document a concrete reason for a different implementation. Numeric dimensions are design targets in logical pixels or points, not permission to clip content or ignore platform accessibility. Service integrity requirements take precedence over a convenient visual shortcut.

## 2 Reference package and authority

Read these files in order:

1. `README_START_HERE.md` for the handoff and verification limits.
2. This specification for product behavior and completion requirements.
3. `BRN_Design_Reference.pdf` for light and dark screen compositions.
4. `BRN_Interactive_Reference.html` for the chat to canvas to Library transitions.
5. `BRN_Design_Tokens.json` for named appearance and geometry values.
6. `BUILD_AGENT_PROMPT.txt` for the receiving agent's starting instruction.
7. [Reading-quality fixture instructions](fixtures/README.md) and the [sample article](fixtures/reading-quality-article.md) for outline and document-presentation acceptance.

The written behavior in this specification governs edge cases. The PDF and interactive reference govern visual hierarchy, density and interaction direction. The token file governs the named starting values. Resolve a conflict in that order, record the decision, and keep the package coherent. The reference content and model names are examples, not production defaults.

The interactive reference uses a temporary in-memory simulation. It is not a GPUI application, persistence implementation, model connection or test harness. Its sample imports, history and responses must not be copied as production behavior. The PDF contains drawn design plates, not native screenshots. Browser/native rendering of the reference was not verified in this environment; source syntax and document rendering were checked separately.

This handoff lives at `docs/product/ux-rebuild/`. The canonical target, architecture invariants and behavior contracts retain their existing ownership; this package defines the selected UX migration. Reconcile any required canonical documentation changes during implementation. Do not introduce a second product spec authority or copy long permanent instructions into every agent prompt.

### 2.1 Relationship to previous design work

The selected concept replaces open exploration of unrelated visual directions. Keep applicable repository and task approval gates. User approval of a direction does not establish that a named Council met a score gate, that native behavior passed, or that a release was approved. If the current task retains a visual review gate, complete it honestly on this direction, including dark mode. No Council score is claimed by this package.

The related [Wiki Library specification](BRN_Wiki_Library_Spec.md) is included in this directory and remains relevant to article consolidation and evidence. This document defines the new interface around that behavior. It does not authorize bulk consolidation of the owner's real library during development.

### 2.2 Mado reading reference — revision 1.2

Evo supplied the Mado recommendation from the [shared design discussion](https://chatgpt.com/s/t_6aec0987aaa08191beba222ebba48e3c). Its required decisions are recorded here so the build agent does not need access to that conversation. Mado's official product page illustrates reading-first Markdown presentation, a heading outline and document export. It is a product reference for the article surface, not a replacement for the approved ChatGPT-inspired shell or the required GPUI Kit 0.9.0 implementation. [T15]

| Idea | BRN requirement | Scope |
|---|---|---|
| Reading first | Open Library articles in Read mode; keep editing, assistant, sources and comments available when needed. | Required; reinforces §§6.2–6.5 |
| Heading outline | Provide collapsible contents navigation with distinct targets and current-section indication. | Required; highest-priority addition in this revision, §6.5.1 |
| Consistent presentation | Use deliberate heading hierarchy, paragraph spacing, readable width, tables, images and citations in Light and Dark. | Required; §§8–10 and AC-42 |
| Polished HTML/PDF snapshots | Consider export of one explicitly selected article version, with contents navigation and working source links. | Later work, §15.1 |
| Small publishing adapters | Consider one explicit export action and one destination per adapter. | Later work, §15.1 |

This reference does not establish an available Mado source library or embeddable SDK. No Mado code, runtime or connector dependency is required. Use the existing BRN writing lifecycle and GPUI Kit building blocks. SQLite remains the authority for accepted content; Markdown is an input/rendering or export representation, not a second authoritative store. [R4] [R6]

The PDF and HTML retain the approved visual baseline. They do not yet depict or simulate the new outline; the written rules, tokens and fixture in revision 1.2 define that addition. The fixture is synthetic source material for native verification, not a claim that the feature has been implemented or observed.

## 3 Grounded starting point

The read-only repository inventory used `ewq100/brn-rust` main at **`e749a83e207c5b6d3c081d9f7d1fa6d6efed088c`**. Reconcile the receiving agent's actual branch, HEAD, work in progress, task instructions and open changes before implementation. Source findings are not a native acceptance result.

At the inspected baseline, desktop and reusable writing UI pin **GPUI Kit `=0.8.0`**. On 5 November 2026, Evo explicitly added an upgrade to **GPUI Kit `=0.9.0`** to this handoff. Version 0.8.0 is historical context, not the rebuild target. The upgrade is required and already authorized within the assigned UX rebuild; do not request a second approval merely to perform it. Target 0.9.0 explicitly rather than a floating latest version, and reconcile a checkout that has already completed this upgrade. [R1] [R2] [T13]

The desktop currently presents a tiled workspace, with Chats, Notes, Projects, Tasks, Changes and Settings, plus contextual Compare, Connections and linked-work surfaces. Existing capabilities include queued chat, independent document writing, project attachments, manual document scanning, reviewed changes, tasks, history and recovery. These must be mapped into the new interface. [R1] [R3]

SQLite owns accepted records. Desktop, CLI and agents use shared checked services. The reusable writing module already owns important document and review lifecycles. Keep those boundaries. This is a full UX rebuild, not a mandate to replace storage, the agent runtime, the editor engine or the service layer. [R4] [R5]

## 4 Navigation and information architecture

### 4.1 Persistent sidebar

Default expanded width is 224. Use a slightly differentiated neutral background, compact typography, small line icons and a quiet selected row. The sidebar contains:

- **New chat**
- **Search**
- **Library**
- **Inbox**, with an attention count when applicable
- **Projects**, followed by recent or pinned projects
- **Recent chats**, with a route to all and archived conversations
- The user/workspace area at the bottom, opening Settings and workspace-level commands

New chat starts outside a project unless invoked from a project context. Do not silently inherit a private project's scope from an unrelated last-open document. The project destination and knowledge setting must be apparent before sending.

Allow pinning or existing equivalent organization without requiring it. Keep long names readable through ellipsis plus a full accessible name and tooltip. Context menus hold rename, move, archive and deletion actions, with keyboard access. Essential primary navigation is visible without hover.

Do not add a second global Chat / Write / Library tab bar. Writing is entered by opening or creating a document. Local view controls inside a document or Inbox are allowed when they describe that object's state.

### 4.2 Destination map

| Existing capability | Destination in the rebuilt UX |
|---|---|
| Chat list, all chats, archived chats | Sidebar recents and a searchable all-chats view |
| Standalone Chat | Primary conversation workspace |
| Note, read view and Write | Primary document canvas |
| Notes, topics and indexes | Library Articles with existing topic navigation |
| References, full imports, unindexed notes and personal writing | Library All files and appropriate contextual lists |
| Projects | Project workspace with chats, documents, instructions and knowledge settings |
| Needs you and pending review | Inbox For you, linked to the exact conversation or document |
| Document intake folder and scanning | Inbox Imports, with explicit folder/destination setup and Scan |
| Tasks and follow-ups | Inbox Tasks and contextual task cards in their originating work |
| Global Changes and operation receipts | Recent changes command and Library/Inbox menu entry; contextual document history |
| Compare and candidate review | Contextual review in the document canvas |
| Connections and related material | Article Related panel; existing bounded relationship views remain reachable |
| Knowledge checks and repair follow-ups | Library menu and contextual affected-record views |
| Agent activity and latest work status | Per-conversation activity disclosure; active errors/review remain visible |
| General, Models, Delegation, Documents, Data, Skills and MCP | Settings sections, opened from the workspace menu |
| Export, backup, restore and recoverable Trash | Settings Data or the appropriate project/document menu |

The map is a minimum. Produce an implementation inventory of every currently supported route, menu command, shortcut, setting and visible state. For each, record old entry, new entry, service owner, implementation status and evidence. Cosmetic tile controls such as swapping tile headers can retire because the selected layout replaces that mechanism. The user outcome they served, such as focusing a document or opening related work, must remain available.

### 4.3 Default and restored state

On first use, present a calm New chat surface with a short welcome heading and the composer. Do not repeat the composer's + actions as a row of starting buttons; attaching, creating a document and tool access stay in the + menu (§6.1.1). Existing manual work must remain available before selecting a maintenance delegation. Provider setup or the existing delegation choice appears when relevant, without a permanent warning dashboard.

On later launches, restore the last valid workspace and safe saved layout. Restore an article's draft through the existing recovery mechanism. Restore queued work in its correct paused state. Opening the application or unarchiving a chat must not implicitly resume an agent run.

If the prior object is missing or no longer accessible, explain that locally and open a safe destination. Keep any recoverable text accessible. Do not silently redirect a private document to a general library copy.

## 5 Knowledge and permission language

The approved visual concept used “Approved library” as example copy. The implementation must use **Saved library** or **Current knowledge** unless an actual reviewed-only filter is active. Current BRN permits ordinary notes to be maintained under an explicitly chosen standing delegation. Saved does not mean personally reviewed. [R4] [R6]

| Term | Meaning and UI treatment |
|---|---|
| Saved | The service confirmed the current content was durably saved. It does not assert personal review. |
| Owner-confirmed | A specific revision has the applicable recorded owner confirmation. Show only when supported by actual state. |
| Protected | Ordinary maintenance cannot change the material. Protection does not certify factual truth. |
| Personal edit | The owner's current editor buffer, with its own save/recovery state. |
| Proposed change | An immutable candidate awaiting the applicable review. It is not current saved knowledge. |
| Delegated maintenance | An authorized ordinary-note change through the same checked service, under the selected delegation. |
| Conversation or project reference | Context available within that scope; attaching it does not grant protected write permission or general publication. |

Default retrieval keeps current ordinary and protected notes within the permitted scope. Pending candidates, scratch, older revisions and archived notes must not enter current retrieval merely because the interface can display them. Owner history search and explicit historical views remain separately available. [R6]

Use one consistent source of authorization truth. Do not add blanket manual approval for every ordinary delegated update, and do not silently broaden delegation. Opening a document, selecting a passage, adding a reference or saying something in ordinary chat does not itself authorize a protected edit.

The primary composer should show a concise context chip, such as “Saved library,” “Project BRN,” “This chat only,” or the active document title. Opening the chip reveals the real scope, selected references and relevant knowledge setting. Avoid claiming “Only this document” while broader retrieval is enabled.

## 6 Screen and journey contracts

### 6.1 Conversation workspace

The conversation is centered within a readable column. Assistant messages are predominantly flat prose; user messages may use a soft rounded bubble. Render lists, tables, code and citations through the existing maintained renderer. A document or task result may have one purposeful card. Do not wrap every paragraph or tool event in a card.

The header identifies the conversation and project where relevant. Keep the model choice near the composer rather than repeating it in the header and status area. Populate choices from actual supported provider/account capabilities. An unavailable selection produces a clear state; no silent fallback or invented model list.

The bottom composer contains the multiline input and one control row: one compact + action, a quiet context chip showing the actual scope, one combined provider/model/effort control and one primary Send/Queue/Stop control appropriate to state. The input starts compact, grows to a bounded height and then scrolls internally. Explanations about attachments appear when attachments are present, not as a permanent paragraph above an empty input. The conversation remains reachable above it at the minimum window size.

**CHAT-01:** Enter sends when idle and queues while running; Shift-Enter inserts a newline. Do not submit an active IME composition. Preserve current keyboard behavior where platform conventions require an adaptation.

**CHAT-02:** After the service accepts the intended send or queue operation, clear the live composer once. If the operation fails, retain the text and offer a safe retry. Bind the draft and action to the correct thread; navigation cannot redirect an in-flight submission.

**CHAT-03:** While running, show a concise status and Stop. The queue expands only when nonempty and supports existing edit, remove and reorder actions. Stop, interruption, failure and restart preserve waiting items and their paused state. Resume and Steer remain explicit. Unarchive does not resume.

**CHAT-04:** Stream without moving the reading position when the owner has scrolled up. Show Jump to latest when appropriate. Keep text selection, copy and source navigation usable. Collapse routine tool activity; always surface required permissions, failures and review outcomes.

**CHAT-05:** Preserve explicit Edit note, Save as decision and Capture outcomes actions. Edit note binds a specific saved note/version and instruction. Save as decision shows editable title and text before creating the exact protected confirmed note. Captured outcomes remain proposals with their proper scope. These actions must preserve unsent drafts.

**CHAT-06:** Quiet mode, if supported, hides routine intermediate traces while preserving answers, Stop, actionable errors, permissions and reviews. Activity reports real events and receipts, not guessed progress or fabricated success. Its card has a bounded height; longer steps and details scroll inside it and never paint over the attachments or composer below.

### 6.1.1 Composer tools, web access, skills and model choice

Version 1.3 replaces earlier requirements that kept a separate web query dialog, a persistent skill picker and built-in skill disable controls ([#249](https://github.com/ewq100/brn-rust/issues/249)). Do not reintroduce them.

**+ menu.** One compact + action holds Attach document, Add useful information, Import full note, New document and the **Allow web access** toggle. Approved local MCP tools keep their exact-input review entries. It holds no skill picker, “No skill” or “Refresh skills.”

**CHAT-07 Agent web access.** The agent may call a web-search tool within the same chat and run when useful; an ordinary request needs no query form. The run shows a brief “Searching the web…” activity and the answer cites its web sources close to that answer. **Allow web access** is retained per conversation; new chats start Allowed when the selected subscription route supports search. Allowed permits, but never forces, a search. Off blocks web access for the main agent and every delegated tool path; enforcement is central in the tool runtime, not prompt text. Unavailable (for example GitHub Copilot, or an unsupported account path) is stated separately from Off, with no fallback to another provider, account or billing route. Search reuses the existing subscription-search adapter; only the agent's query is sent, never the whole chat, document or library. Search activity, sources, failures and cancellation belong to the originating chat and run: Stop cancels owned search, and late results never appear in another chat or overwrite a newer run. A search that is refused or fails is reported in Activity as failed, never as a success, and the answer then has no Web sources. Web results are evidence, never instructions.

**CHAT-08 Skills.** BRN's built-in skills are always available for automatic discovery and loading; a normal message can use an appropriate skill without manual selection, and the owner cannot disable built-ins. Typing / at the start of a word opens a searchable list built from the actual skill catalogue with short descriptions; a shortened description ends at a whole word with an ellipsis, and a row wraps it to two lines rather than cutting it again in a narrow chat. The list stays inside the chat area: it opens below the centred New chat composer and above a docked one, scrolls when long, and keeps the keyboard-selected row in view. Choosing a command inserts an explicit request for the current message only; it never becomes a persistent mode for the project, other chats or later messages, and the agent may still use other relevant skills. Ordinary slashes in prose, URLs and paths are not commands. Removing the unsent command removes only that request. Existing workspaces with saved disabled built-in bindings or an old persistent selection are reconciled by the service. Installed packages keep truthful validity and availability; loading any skill grants no tool, edit or publication permission.

**CHAT-09 Model picker.** The closed control shows a provider indicator, short model name, current effort where supported and a caret. Opening it shows connected providers and Favorites on the left, searchable models in the middle and the selected model's supported reasoning levels on the right; on a narrow surface the reasoning section moves below the model list. Choices come from actual provider/account/model capability data, never a shared history of model names, and only supported efforts appear. Provider/account/model identity stays together, including in Favorites; the route is clear when a model is offered by several providers. Model and effort change together: an unsupported earlier effort never survives a model switch, and any adjustment is visible. Each run records the provider/model/effort it used; later changes never relabel earlier replies or alter an in-flight request.

### 6.2 Chat to document

A document card contains a title, content type and meaningful state such as “Proposed update” or “Saved document.” Opening it selects the same canonical document; it does not create a copy. Preserve the originating conversation and return position.

The document becomes the primary surface. On the first transition into writing, navigation may collapse to its rail. Show the related assistant when the owner opened a proposal from chat. Opening a Library article defaults to reading with the assistant closed. Preserve later explicit pane choices; Apply, Undo and a re-render must not reset them.

Exactly one composer is visible in a writing workspace. If the assistant is closed, show a quiet “Ask about this document” affordance. It opens the existing contextual assistant or creates/adopts the correct writing conversation through existing services. It must not discard an unsent document prompt or merge it into an occupied conversation draft.

### 6.3 Project workspace

Use a compact project header, a short description when present, and clear local sections for documents and conversations. Expose **New document**, **Add document** and **New chat** beside the relevant lists, including empty states. Project instructions and the inherited/overridden knowledge setting are discoverable in the header or a settings sheet.

Retain linking of shared documents and private file import. Show the active host project when a shared document opens. One linked Note retains one identity, while writing context and references remain scoped to the host. Moving a conversation must preserve private provenance and apply existing run/queue fencing.

Deletion reviews the concrete removal, retention and unfinished-task choices. Keep recoverable Trash/Restore. Restoring a project must not resume runs or revive tasks explicitly cancelled during deletion. A clean interface is not a reason to remove these decisions or silently choose them. [R6]

### 6.4 Library

Open to useful topics and consolidated current articles. An article row shows title, a short useful description and restrained metadata. Avoid a spreadsheet of internal IDs, hashes and processing flags.

Use **Articles** as the default local view and **All files** as the secondary route to supported notes, personal writing, full imported references and source material. Preserve search and existing scope rules. “All files” does not promise an archive of original binaries that BRN never retained.

Topic indexes remain ordinary linked Notes. Do not require a new database object, inferred taxonomy or title naming convention to make the Library usable. One article can appear under several relevant topics without copying its content.

Keep existing Recent, Linked, Protected/imported and A-Z capabilities available through compact filters or menus. Avoid showing every filter by default. Show applicable search scope and result type. Opening a result preserves its actual identity and version.

Article reading provides related articles, supporting sources, last change, review attention and history on demand. Sources must open their recorded evidence or state the actual availability limit. Source date and import date must not be conflated. Contextual Connections remain available; a global graph does not become the default interface.

Knowledge checks retain the ability to inspect affected records, broken links, stale material, coverage gaps, similar/duplicate content and recorded conflicts. A result can lead to a reviewed repair or a follow-up. Avoid presenting a heuristic as proof or auto-repairing protected material.

### 6.5 Document reading and direct editing

The same underlying document supports readable presentation and editing. Reuse the existing Markdown buffer and editor capabilities. A clean reading surface does not imply a new rich-text engine or lossless WYSIWYG conversion.

Use a small local Read / Edit control or the existing equivalent. Keep formatting actions contextual and compact. The title, actual save state, comments, sources and review state remain reachable. A document can be written with Chat closed.

**WRITE-01:** Persist/recover personal drafts with the existing edit-session identity. Pane changes, theme changes, resizing, navigation and conversation switching must preserve text, selection and recovery state.

**WRITE-02:** Show **Unsaved**, **Saving**, **Saved**, **Save failed** or **Conflict** based on actual service state. Never retain a green Saved label after typing. Recovery autosave is not the same as committing current knowledge.

**WRITE-03:** Keep explicit Save. Do not silently change product save semantics to automatic canonical commits. If navigation safely preserves a recovery buffer, avoid a redundant modal. If an operation cannot preserve it, offer Save, Discard and Cancel, with their concrete effects.

**WRITE-04:** A save checks the current base version and edit generation. On a conflict, preserve the draft, explain that the saved article changed, and open an appropriate comparison or recovery route. Never silently overwrite a newer human or service update.

**WRITE-05:** Keep document-specific state separate: editor buffer, draft prompt, references, proposal, selection and applied status must not leak into an unrelated document or host context. Existing comments are Note-based: an intentionally shared Note can expose the same comments in its authorized hosts. Preserve that visibility rule and the current access boundary; do not promise new per-project comment privacy. Scoped writing setup, references and unsent assistant drafts remain distinct. The shared-Note comment privacy question requires a separate explicit product decision if changed. [R9]

### 6.5.1 Long-article reading and outline

**READ-01 — Local, optional contents.** Provide a labeled **Outline** toggle in the document toolbar and an accessible command in the existing command surface. On first opening an article, keep the outline closed until requested. Remember explicit visibility and any heading-fold choices by stable document identity. It is independent of the assistant. Outline navigation is required in Read mode; Edit mode may reuse existing editor navigation only where its range APIs support it. Do not create a second editor, buffer or document copy to supply this feature.

**READ-02 — Real headings and exact targets.** Build the outline from the structure used to render the displayed article. Support heading levels 1–6, nested and skipped levels, Unicode, repeated names and long labels. Modest indentation communicates hierarchy; wrap long labels or make their full text accessible. Fenced code must not create headings. Repeated titles have distinct targets within the displayed content generation; a title string or guessed slug is not sufficient identity. A document without headings shows “No headings yet” without implying a load failure. Collapsing the outline is required; folding individual branches is optional.

**READ-03 — Predictable navigation.** Clicking or pressing Enter on an entry reveals its heading below the fixed document toolbar. Indicate the current section with accessible selected/current semantics and more than color alone. Scrolling updates that indication without moving keyboard focus or jumping the document. Keyboard users can reach the toggle, move through headings, activate an entry and close the panel; expose any optional folding controls. A docked outline is a normal nonmodal region. In the compact temporary view, selecting a heading returns to the document at that section; Escape dismisses it and returns focus to the invoking control. Do not introduce an unreviewed global shortcut.

**READ-04 — Correct document and version.** Bind heading targets to the actual displayed document identity and content generation: committed revision, historical revision, or unsaved buffer as applicable. Rename/removal, Read/Edit transitions, saved changes, Apply, Undo and reopened documents must refresh the outline before old targets can activate. Discard stale asynchronous results. If an entry no longer resolves, refresh or show its unavailable state rather than guessing a similarly named section. History or proposal views must identify the outlined surface; when no single document surface is unambiguous, temporarily hide the outline. Navigation and theme/pane changes preserve unsaved text, assistant drafts, selection and the appropriate reading position; preserve explicit pane preferences separately from temporary layout fallbacks.

**READ-05 — Reading quality.** Use the document styles in §10 for prose, headings, lists, quotes, code, tables, images and citations. Tables and code may scroll locally; ordinary paragraphs must wrap inside the reading column. Images keep their aspect ratio and have useful alternative text or a clear unavailable state. Citation links open the actual recorded source or explain the availability limit, then allow a return to the reading position. Use the [long-article fixture](fixtures/reading-quality-article.md) and its [setup instructions](fixtures/README.md) to inspect these requirements in both themes. Existing source permissions, comment anchors and review semantics continue to apply.

### 6.6 Comments and rewriting

Selecting a passage exposes **Comment** and the existing appropriate actions. A comment retains the original quote, note, revision and range. Comments can be inspected from a compact side sheet or contextual list. Do not require a permanently visible comment column.

Overall feedback and selected passage comments remain available for an explicit **Rewrite** request. The control must distinguish Rewrite from ordinary chat. Sending a general question about a document must not silently initiate a protected write.

Rewrite freezes its saved document, selected comments, overall feedback and applicable posted context according to existing services. Display progress in document work even when chat is closed. Late output from an obsolete request must not replace newer typing or a newer proposal.

Show attached, moved, removed or unresolved anchors honestly. An ambiguous anchor remains unresolved with its quote visible. Do not guess a new passage by similar words. Mark addressed remains an explicit action or accurately described choice; accepting a rewrite must not silently claim every comment is resolved.

### 6.7 Reviewing changes

Use the document canvas for review. Default to an in-context prose comparison with clear **Added** and **Removed** meaning, restrained highlighting and normal reading typography. Offer a two-column comparison on demand when width permits. Keep code-oriented monospace patches for material where that presentation helps.

The review header identifies the affected document or coherent group, number of changes, relevant sources and whether the saved version is unchanged. Offer **Apply**, **Discard** and applicable candidate editing. For a dependent group, describe the entire effect; do not offer arbitrary partial approval that the service cannot commit safely.

**REVIEW-01:** Apply binds the exact displayed candidate and current preconditions. A regenerated or edited candidate needs its own current review. A stale version, dirty guard, scope change or protected-state conflict refuses safely and preserves useful work.

**REVIEW-02:** After a confirmed Apply, show the actual outcome and receipt-based **Undo**. Discard leaves the saved article unchanged. The UI must not report Applied while the commit is pending or has failed. If Apply commits but later attention cleanup or Mark addressed fails, retain Applied, the committed content, actual receipt and Undo; report the follow-up failure separately. Do not invite a duplicate Apply. [R9]

**REVIEW-03:** Undo is a checked compensation. Later changes must not be overwritten to recreate an old screenshot. If automatic Undo is no longer safe, show the conflict and existing reviewed-compensation route.

**REVIEW-04:** Separate editor Undo from saved-operation Undo. The normal editing shortcut affects the editor buffer; receipt Undo is clearly tied to a saved operation. A local Undo cannot claim to reverse an external send or provider usage.

### 6.8 Inbox and intake

Inbox is a quiet attention destination with local **For you**, **Imports** and **Tasks** views. Use counts for actual actionable items, not every historical message or imported record. Each item opens its exact originating context and carries the real underlying state.

For you includes reviews, blocked work and decisions needing the owner. Routine successful delegated maintenance can appear in activity/history without creating artificial approval work. Distinguish “needs your review,” “failed,” and “waiting on someone else.”

Imports provides capture/paste and existing file-intake routes, choosing between useful-information extraction and a protected full readable note where supported. Preserve coverage warnings, meaningful figures and source attribution. A partial summary cannot be called a complete imported document.

The current folder inbox is **manual**. Preserve separately confirmed folder and General-knowledge destination setup, explicit **Scan**, per-item progress, cancellation, failures and retry. Moving this control into Inbox does not authorize a background watcher, automatic general publication or a new schedule. [R1] [R7]

Conversation attachments remain private to their permitted context until explicit **Keep in BRN**. Promotion uses the same retained Note and checked service with Undo. Show the destination before promotion. Attaching or viewing material does not change its general-search eligibility.

An empty Inbox offers a calm caught-up state and relevant capture action. A failed conversion keeps retry and safe diagnostic details available. Do not erase the filename or target scope when an error occurs.

### 6.9 Tasks and follow ups

Retain task state, reported owner, waiting-on context, separate follow-up and due dates, checked detail editing and explicit completion. Put these in a compact detail panel when the task opens; show only meaningful summary fields in a list.

Task state is independent of conversation state. Archiving a chat does not complete a task. Saving task details or approving a proposed task does not perform the real-world action or schedule an external reminder. Invalid dates and concurrent changes leave the form open with the user's input intact.

### 6.10 Search and contextual details

One global Search entry opens a keyboard-accessible result surface for supported objects. Clearly distinguish current-knowledge results from historical conversation/archive results and identify scope and type. A word found only in a current item's text finds it too, with the matching passage; title matches come first and late results never move the selection. Search must not widen access or promote candidates into current knowledge.

Sources, version history, related records, comparison and global changes should use contextual panels or full focused views where density requires them. Preserve back navigation and reading position. Drawers, menus and dialogs have actual focus and dismissal behavior.

Web search in chat is an agent tool governed by CHAT-07 (§6.1.1); there is no separate query dialog in the normal chat flow. Do not automatically send the entire open document, chat or library to a web search service. Web results stay cited and untrusted. [R1]

### 6.11 Settings and first use

Settings uses a readable local list of sections: Appearance, General, Models, Delegation, Documents, Data, Skills and MCP, combining items only when their meanings remain clear. Reuse existing services and constraints.

Appearance offers System / Light / Dark as the primary choice. Existing additional palettes can remain in an advanced appearance area if still supported; they do not clutter the main shell. Model configuration preserves actual provider/account/model/effort and helper choices, consistent with the composer picker (CHAT-09). Show unavailable credentials or capabilities truthfully.

Delegation retains the explicit Maintain ordinary notes / Review changes first choice and the unchosen state. Documents retains converter and AI-conversion controls. Data retains export, full backup, restore, data location and recoverable deletion flows. Skills show the built-in catalogue as always available, with no Enable/Disable controls for built-ins (CHAT-08); installed packages keep install, inspect, enable, remove and truthful availability. MCP preserves discovery, enablement, actual availability and exact required approval.

After restore, show the real configuration exceptions: local inbox authorization and its ledger require fresh folder/destination confirmation; installed instruction-package bytes are excluded from canonical backup, so missing selected packages remain visibly unavailable; transient MCP execution grants are not restored. Do not silently reinstall packages or reconstruct grants. [R6] [R10]

Permissions appear as concrete requests naming the action, target and relevant scope/input. Keep Allow and Deny reachable in the work context. Routine traces may collapse; an active permission request must not disappear inside them. Changing MCP discovery input must not retain a misleading approval for an earlier destination.

## 7 State and recovery model

Represent independent facts separately. Do not reduce a document, run, queue and proposal to a single busy boolean.

| State axis | Required distinctions | Presentation rule |
|---|---|---|
| Document buffer | Clean, dirty, saving, save failed, conflict, recovery available | Save status belongs to the document. |
| Proposal | Absent, preparing, ready, stale, applying, applied, discarded, blocked | Proposed text never looks like an already saved result. |
| Agent run | Idle, queued, working, waiting for permission, stopping, interrupted, failed, completed | Show the next valid action and actual outcome. |
| Message queue | Empty, waiting, dispatching, paused | Keep ordering and explicit Resume. |
| Context | General, project, private conversation, selected document/reference | Label scope; separate context from authority. |
| Appearance | Preference System/Light/Dark plus resolved light/dark | Appearance is independent of document and agent work. |
| Layout | Primary view, navigation preference, assistant visibility, panel widths | Preserve the owner's choice within usable width constraints. |

Each document and conversation has stable state ownership. Scope drafts by the actual host/document/thread identities already used by the service; do not use the display title as a key. Bind asynchronous callbacks to the correct operation and current context. Stale work cannot update a newly selected record.

Keep expected failures local and actionable. “Couldn't save” should preserve the draft and offer Retry/details. “Document changed” should offer comparison. “Model unavailable” should preserve the prompt and explain how to choose a valid option. “Original source unavailable” should preserve the retained note and explain its evidence limit.

Restore errors, interrupted work and missing assets must not produce endless spinners, empty panels without explanation, or success badges. Preserve operation identity across retry so an ambiguous response cannot duplicate a commit or send.

## 8 Layout and sizing

Use logical pixels/points and native scaling. The main desktop acceptance viewports are **1280 × 900**, **1440 × 960** and **900 × 600** usable client areas. Test larger text/scaling separately. Mobile parity is not part of this desktop rebuild.

| Element | Starting value | Constraint |
|---|---:|---|
| Expanded navigation | 224 | Adjustable roughly 208–280; names remain accessible. |
| Collapsed navigation rail | 56 | Labeled icon actions via accessible names/tooltips. |
| Application header | 56 | Native titlebar integration may alter physical chrome. |
| Conversation reading column | 640–720 | Centered with flexible side space. |
| Document reading column | 680–760 | Do not stretch prose across an ultrawide window. |
| Docked article outline | 200 | Useful range 180–240; counts separately from the document region. |
| Assistant pane | 320 | Useful range 280–400; collapsible and resizable. |
| Document region in split view | At least 560 | Includes inner margins; preserve readable prose. |
| Page inset | 32–40 | Reduce to 20–24 where needed in a small window. |
| Standard control height | 32–36 | Fine-pointer desktop; clear effective hit targets. |
| Compact icon control | 30–32 | Avoid adjacent tiny targets and clipped focus. |
| Sidebar row | 36 | Touch use, if shipped, needs enlarged effective targets. |
| Composer | 2–8 text rows | Cap growth; retain access to conversation and Send/Stop. |
| Major divider | 1 | Neutral and visible without a heavy frame around every region. |

Compute split availability from remaining usable width after navigation, scaling and padding. Do not assume a screen-name breakpoint guarantees enough room. When document plus assistant cannot fit their usable widths, collapse navigation first if the owner has not deliberately expanded it; then use a contextual assistant sheet or a local document/assistant switch. Preserve work when moving between layouts.

At 900 × 600, writing should open with the document dominant and supporting panes closed by default. Opening the assistant must not squeeze the document into a narrow unreadable strip. Give the owner a clear return to the document.

The outline is a local contents region beside the document, not another global navigation destination. Dock it only when the remaining document region still meets the 560 minimum, including its inner margins. Count navigation, outline, assistant, dividers and usable window width before choosing the layout. Never shrink text or margins below the stated reading rules just to keep four panes visible.

When the requested outline cannot fit, use a temporary sheet or local contents view. At the minimum window size, opening Outline, assistant, sources or comments must show the newly requested supporting view coherently; temporarily hide an incompatible supporting view and preserve its draft, selection and scroll state. Do not stack sheets or duplicate the composer. Selecting an outline entry returns to the document at the selected section. A responsive fallback does not overwrite the owner's stored visibility or width preference; restore the preferred arrangement when space permits.

At wide sizes, expanded navigation, document and assistant may coexist if the minimums hold. Changing theme, applying a change or receiving a stream event must not reset pane widths. Avoid horizontal scrolling in ordinary prose; wide tables and code blocks may scroll within their own region.

Use native titlebar behavior and platform window controls. The mockup's decorative window dots are explanatory chrome, not a requirement to draw fake macOS controls on every platform.

## 9 Light and dark appearance

### 9.1 Preference behavior

**THEME-01:** New workspaces default to System. Light and Dark are explicit overrides. System follows the current operating-system appearance and relevant live appearance changes. If the platform cannot deliver a live change event, re-read at a supported lifecycle point and document the actual limitation.

**THEME-02:** Apply a selection immediately to all BRN-owned surfaces in the current workspace, including already-open menus/dialogs where supported. Persist through an appropriate presentation-preference path that does not cancel or fence agent work merely because color changed. Existing theme persistence may defer while work runs; improve it safely or make pending persistence explicit. Never report a durable preference save that has not occurred.

**THEME-03:** Theme changes retain editor text, unsent messages, selection, focus, scroll position, pending proposals, recovery buffers and run state. They are presentation changes.

**THEME-04:** Resolve tokens before painting the first normal frame to avoid a bright flash during dark startup. Theme-dependent caches, syntax colors, Markdown and virtualized rows must update together.

### 9.2 Semantic palette

Use the same token names in both themes. The values below are the selected starting palette; `BRN_Design_Tokens.json` is the machine-readable companion. Maintain the relationships if adjusting a value to meet measured contrast.

| Token | Light | Dark | Use |
|---|---|---|---|
| canvas | `#FFFFFF` | `#202020` | Primary workspace |
| sidebar | `#F5F5F3` | `#181818` | Navigation |
| surface_subtle | `#FAFAF9` | `#252525` | Composer/supporting surface |
| surface_raised | `#FFFFFF` | `#2A2A28` | Menus/dialogs |
| hover | `#EAEAE6` | `#30302E` | Hover/quiet selection |
| text | `#242424` | `#EBEBEB` | Primary text |
| text_muted | `#64645F` | `#B3B3AB` | Readable metadata |
| placeholder | `#70706A` | `#A6A69F` | Input placeholders |
| divider | `#DEDED9` | `#3E3E3A` | Decorative structure |
| control_border | `#85857E` | `#818179` | Boundary needed to identify a control |
| primary | `#292A28` | `#EEEEE9` | Main action fill |
| on_primary | `#FFFFFF` | `#232421` | Main action text/icon |
| focus | `#526A9C` | `#9FB7E5` | Visible keyboard focus |
| link | `#315CA0` | `#ABC7F5` | Links, with a text affordance |
| selection | `#D7E5FB` | `#3B5071` | Text selection under normal text |
| success_text | `#346B54` | `#A4D1B9` | Success/added state label |
| success_surface | `#EDF5EE` | `#263A2F` | Added change background |
| warning_text | `#7C5C22` | `#DEC18A` | Review/waiting state label |
| warning_surface | `#F9F3E7` | `#393226` | Review attention surface |
| danger_text | `#A63535` | `#F0B4B4` | Error/removed state label |
| danger_surface | `#FCEEEE` | `#412828` | Error/removed background |

Dark mode uses distinct dark surfaces and readable muted text. Do not invert light screenshots, leave white editor islands, or turn all secondary information into barely visible gray. Use restrained neutral primary actions. Reserve status colors for meaning.

Measure normal text, metadata and placeholders at **4.5:1 or better** against their actual background; use **3:1 or better** for necessary control/icon/focus information. Decorative separators need not meet the control threshold. These are measurable native design targets derived from WCAG guidance, not a claim of formal web conformance. [T8] [T9]

Additions, removals, errors, protection and selection must also have labels, shapes or text treatment. Color alone is insufficient. Retain system text selection conventions and visible keyboard focus. Respect reduced-motion preferences for nonessential animation. [T10] [T11] [T12]

### 9.3 Complete theme coverage

Check navigation, composer, provider/model menus, chat Markdown, tables/code, article reading, native editor, comments, selected passages, added/removed text, review bars, comparison views, permission prompts, file/import dialogs, tooltips, context menus, search, task forms, settings, empty/loading/error states, scrollbars and recovery/restore surfaces.

OS-owned file pickers may follow platform appearance independently; do not substitute a fake in-app picker to make a screenshot match. Verify their usability and handoff on each shipped platform.

## 10 Typography spacing and components

Use the native platform system font for chrome and prose, with an appropriate installed fallback. Use monospace only for code, technical identifiers when necessary, and optional activity details. Most of the app must read like a writing application.

| Role | Starting size | Weight and line spacing |
|---|---:|---|
| Metadata | 11 | Regular; never lower to fit clutter |
| Secondary label | 12 | Regular |
| Sidebar | 13 | Regular; selected may be medium |
| Controls and chat body | 14 | Regular; comfortable line spacing |
| Document body | 16 | Regular; approximately 1.7 line height |
| Section heading | 18 | Medium |
| Conversation emphasis | 20 | Medium |
| Article/welcome heading | 28 | Medium; no oversized marketing headline |

Use a shared spacing scale of **4, 8, 12, 16, 20, 24, 32, 40 and 48**. Typical control radius is 6–8, a meaningful result card 10, user bubble 16–18 and composer 16–18. Rounded corners should help distinguish controls, not turn every layout container into a card.

### 10.1 Article presentation

Apply one shared set of document styles to the reading view. Start Markdown H1–H6 at **28, 22, 18, 16, 16 and 16**, with medium weight and approximately 1.3 line height; lower levels retain their actual semantic level even where the visual size matches. Use **12** between paragraphs and **24 before / 8 after** headings, adapting first-child spacing and measured wrapping. The document body remains **16 / 1.7**. Avoid duplicating the same title as both a large application heading and a second decorative document title; preserve the actual stored heading content and its outline target.

Lists retain readable indentation and line spacing. Quotes use a quiet divider and text that meets the theme's contrast requirement. Code uses a readable monospace style, preserves whitespace and offers the existing copy behavior. Syntax coloring, where supported, must remain legible in both themes without introducing a second highlighting engine solely for this redesign.

Tables have distinguishable headers, aligned values, readable cell padding and useful wrapping. Keep a wide table's horizontal scrolling within the table region and reachable by keyboard. Avoid shrinking all document text to fit a table. Preserve the renderer's supported selection and copy behavior, and test that the reader can continue vertically after interacting with a wide block.

Images fit the reading column, preserve aspect ratio and retain captions where present. Do not invert photographs or arbitrary imported figures in Dark mode; give transparent or low-contrast artwork a suitable neutral backing if needed. Missing images show a bounded placeholder with available alternative text and a truthful status, without preventing the rest of the article from opening. Citations and ordinary links remain identifiable, keyboard reachable and readable in both themes. These are presentation requirements; never invent missing source excerpts or rewrite accepted document content to improve its appearance.

### 10.2 Shared controls

Use the toolkit's line-icon family consistently, generally 16–18 in compact chrome. Put a text label on primary actions and meaningful accessible names on icon-only controls. Native focus and selected states need a visible difference in both themes. Do not use tooltip text as the sole accessible name.

Tables are for genuinely tabular information. Library navigation is a readable article list. Use a divider between major panes, restrained row separators and whitespace within content. Avoid permanently visible diagnostics, duplicated status bars, decorative gradients, large colored section headers and placeholder statistics.

## 11 Keyboard focus and interaction quality

Every supported daily journey must work without a mouse. Preserve native text selection, clipboard, context menus, undo conventions and input-method composition. Focus order follows the visible task, not invisible mounted panels.

| Interaction | Required behavior |
|---|---|
| Global search/launcher | Use the existing platform-aware command binding; the concept shows Cmd/Ctrl+K. |
| Save | Cmd/Ctrl+S saves the relevant active editable document through the checked service. |
| Composer Enter | Send or Queue, except while IME composition or a modal form owns the event. |
| Shift+Enter | Insert a newline. |
| Escape | Close the top transient surface; never silently discard a draft or stop a run. |
| Document local Undo | Affect the editor buffer; do not substitute receipt compensation. |
| Open/close drawer or dialog | Move focus appropriately and restore it to the initiating control or a valid successor. |
| Navigation or source return | Restore the relevant reading position and logical focus. |
| Resize split pane | Offer an accessible alternative to pointer dragging where supported. |

Inspect the current keymap before assigning new shortcuts. Keep macOS Command labels and Linux/Windows Control labels accurate. Declaring a shortcut in accessibility metadata does not register its action.

Modal decisions trap focus; a normal document/assistant split does not. Focus indicators must not be clipped by parent containers or obscured by a composer or review bar. Announce significant save, failure and permission outcomes without announcing every streamed token. Make key actions available without hover.

Verify responsive input, list navigation and article opening with a representative synthetic library of 5,000 active notes and long document/conversation fixtures. Reuse virtualized lists and measured text wrapping; avoid synchronous whole-library work on the UI thread. Record the device, baseline and candidate behavior so a serious responsiveness regression is visible. Do not claim a frame-rate or latency target was achieved without measurement.

Reduced motion removes nonessential transitions. Any animation should help orientation and remain brief; no looping decorative motion. Large text/scaling must reflow prose and controls rather than clip labels or compress hit targets.

## 12 GPUI implementation guidance

The following is a capability map for the selected GPUI Kit 0.9.0 target. Check the versioned APIs and existing uses first; later public documentation may differ. Prefer an existing working component before adding a custom one.

| Surface | Toolkit direction | BRN responsibility |
|---|---|---|
| Navigation | Sidebar/groups/menu items/toggle | Routes, active identity, persistence and collapse policy |
| Lists and search | List/virtual-list and existing search/command surface | Scope, stable rows, ordering and correct actions |
| Conversation | Existing MessageScroller and formatted text | Message lifecycle, streaming, citations and queue |
| Composer | Pinned multiline Input/Textarea equivalent | Send/Queue, references, draft persistence and run control |
| Article reading | GPUI Base TextView/Markdown renderer through the pinned kit | Shared document styles, sources, assets, access and record/revision identity |
| Article outline | Toolkit list/tree or simple navigation rows beside the existing renderer | Parsed headings, current section, exact generation-bound targets, focus and compact fallback |
| Writing | Existing brn-writing-ui editor | Buffer, comments, formatting, saves and recovery |
| Split layout | Resizable panels and normal GPUI layout | Usable minimums and remembered layout |
| Menus/dialogs | Maintained toolkit controls under the proper root | Actions, focus, approval meaning and errors |
| Review | Existing prose diff/review widgets | Exact candidate, checked Apply/Discard and receipt Undo |
| Appearance | Pinned toolkit theme API plus BRN semantic tokens | System preference, persistence and complete coverage |

TextView renders formatted text; it is not a complete rich-text editor. A toolkit patch Diff displays supplied changes; it does not own proposal generation or BRN transactions. Keep those distinctions in implementation plans. [T1] [T2] [T3] [T4]

GPUI Base TextView documents Markdown rendering, selection, tables, images, links, styling and source/rendered range mapping. Reconcile those APIs with the actual pinned 0.9.0 code before using them. Reuse the existing parser/structure and retained view state for heading extraction and reveal/scroll coordination through a narrow BRN adapter. The outline is an application feature, not a claimed ready-made toolkit TOC. Do not add an independent Markdown parser or regex heading scan that can disagree with the displayed document. Keep one applicable selection layer and the existing lifecycle owner; integrate within the normal toolkit root. [T16]

The toolkit theme source exposes Light and Dark modes; System is an application preference resolved against OS appearance. Do not invent `ThemeMode::System`. Use and verify the 0.9.0 APIs for theme update, system appearance synchronization and appearance observation. Confirm that BRN's update path synchronizes all component themes; direct global field mutation alone is not sufficient evidence. [T5] [T6]

Keep a single BRN semantic token layer shared by desktop and reusable writing UI. Do not scatter raw colors across screen modules or assume the toolkit retains custom spacing values that it does not own. App text scaling and theme color are distinct controls. Invalidate wrapping/measurement caches when typography changes.

The HTML reference is design material. Do not embed it as a webview to avoid building the GPUI interface. Do not change frameworks, introduce a second editor or rebuild maintained controls solely to reproduce the mockup.

### 12.1 Required GPUI Kit 0.9.0 migration

Make the dependency migration an early, reviewable change with a working app before broad layout changes. Inspect the actual manifests, enabled features, aliases, patches and dependency graph. Update all applicable active-workspace GPUI Kit pins, including `brn-desktop` and `brn-writing-ui`, to `=0.9.0`; update necessary compatible companion/backend entries and `Cargo.lock` coherently. Verify the resolved graph and avoid incompatible mixed UI type versions. The selected 0.9.0 release uses gpui-pre 0.3.11 by default and gpui-fast 0.1.13 for the optional backend; retain BRN's chosen backend and features. [T13]

Read the official 0.9.0 release notes and the intervening 0.8.1 migration notes when starting from the inspected 0.8.0 pin. Address the relevant changes in production code and existing tests: [T13] [T14]

- Public theme structs are now `#[non_exhaustive]`. Replace external struct literals, including struct-update literals, with supported defaults/constructors and field assignments.
- `Window::text_system()` now returns an `Rc`-based window text system that is not `Send`/`Sync`; preserve UI-thread ownership. Review affected unwind boundaries because `DisplayMap` no longer implements the unwind-safety marker traits.
- Migrate affected keyboard/modifier and scrolling APIs from the intervening backend update. Where animation tests depend on time, use the supported executor test clock rather than wall-clock sleeps.
- Preserve the existing writing engine, event identity, selection, comments, review and save/recovery contracts while adapting APIs. If adopting `InputEditsEvent`, account for its delivery alongside `InputEvent::Change` so one edit does not duplicate dirty-state, save or history work. Use new toolkit capabilities only where they serve the approved UX; the upgrade does not require adopting every new feature.

Run the relevant existing build, strict lint and focused desktop/writing checks with the updated lockfile and native features, then qualify the actual app through AC-29 to AC-34 and the affected save/review journeys. Verify both light and dark themes, focus, scrolling, selection/IME, dialogs and Apply/Undo. Record dependency resolution and exact-build evidence. Report a concrete unresolved compatibility issue accurately; do not call the rebuild complete on the old pin or infer a native pass from compilation. Broader dependency upgrades remain outside this instruction unless required by this migration and justified in the change.

## 13 Code and state ownership

Separate presentation state from durable domain state. Reuse current reducers, lifecycle owners and shared services instead of constructing another general framework.

- The application shell owns route selection, pane visibility, transient overlays and presentation preferences.
- Each conversation owns its selection, scroll position, composer recovery binding and presentation of run/queue state.
- The document host owns the active scoped writing context; the reusable writing module owns its existing edit/review lifecycle and input state.
- Shared checked services own accepted content, authorization, versions, operation identity, queue dispatch, receipts and recovery contracts.
- Agent tools and existing skills perform semantic interpretation and article maintenance; layout code must not invent a deterministic semantic consolidation engine.

Bind UI actions to typed existing commands/events. A button should not duplicate canonical save or approval logic. Background completions must validate the relevant record/context/operation before changing visible state. Avoid direct canonical SQL from screen code and duplicate truth in a view model.

### 13.1 Existing entry points to inspect

| Concern | Current paths under the inspected repository |
|---|---|
| Host and services | `crates/brn-desktop/src/threads_ui.rs` |
| Shell/navigation/layout | `threads_shell.rs`, `threads_layout.rs`, `threads_lists.rs`, `threads_components.rs` in that directory |
| Chat/run/activity | `threads_chat.rs`, `threads_composer.rs`, `threads_model_picker.rs`, `threads_writing.rs`, `threads_worker.rs`, `threads_run_activity.rs`, `threads_notice.rs` |
| Projects | `threads_projects.rs` |
| Search/knowledge | `threads_search.rs`, `threads_search_state.rs`, `threads_connections.rs` |
| Intake/recovery | `threads_documents.rs`, `threads_inbox.rs`, `threads_capture.rs`, `threads_restore.rs` |
| Skills/MCP | `threads_skills.rs`, `threads_mcp.rs` |
| Settings/tasks/changes | `threads_tiles.rs` and their service owners |
| Shared writing | `crates/brn-writing-ui/src/` including `native.rs`, `writing.rs`, `workbench.rs`, `note.rs`, `review.rs`, `document.rs`, `setup.rs`, `references.rs`, `formatting.rs`, `diff.rs`, `theme.rs` |

These are navigation aids for the baseline, not a requirement to preserve the same filenames or crate count. Retire obsolete tile-specific mechanisms when their replacement is complete. Keep old and new presentation paths only for a bounded migration period if needed; avoid a permanent second UX.

## 14 Delivery sequence

Work in reviewable slices with a single integration owner and clear file ownership. Keep the app runnable after each slice. Use the repository's existing task/evidence workflow and preserve unrelated work.

1. **Reconcile and inventory.** Record actual HEAD, pin, active gates, current routes and supported behaviors. Capture a small native baseline on synthetic data when available. Complete the preservation map and identify differences from this baseline. Determine whether an active gate blocks implementation. If it does, satisfy that gate on the selected direction before step 2; final native review remains a separate gate.
2. **Upgrade GPUI Kit, then establish tokens and shell.** Complete and check the required 0.9.0 migration in §12.1 as a distinct early change. Build paired neutral themes and System/Light/Dark behavior. Implement sidebar, routing, modal host, focus and responsive pane policy. Validate real small-window layout in both themes before spreading the pattern.
3. **Complete conversation UX.** Move existing chat, streaming, context, queue, run control, explicit writing/decision/capture actions and errors into the new shell. Verify continuity, cancellation and retry.
4. **Complete reading and writing.** Integrate the existing writing module into the canvas. Add the collapsible article outline, shared reading styles and long-article fixture checks in both themes. Preserve direct edits, comments, independent Rewrite, review, exact Apply/Discard, history and Undo.
5. **Complete Library and projects.** Implement article-first navigation, All files, sources/related/checks, project documents/chats/settings, private attachments and explicit promotion.
6. **Complete Inbox and secondary flows.** Migrate attention, manual imports, tasks, global changes, settings, skills/MCP, export/backup/restore, Trash and empty/error/recovery states.
7. **Retire the old UX coherently.** Remove unreachable tile chrome, dead actions and duplicated presentation state only after every retained capability has a verified destination. Update shortcuts, help and behavior documentation together.
8. **Qualify the whole experience.** Run the acceptance journeys below, review representative native screenshots in both themes and at minimum size, resolve concrete findings, and produce a clear handoff.

Start with the approved design, then improve specific observed problems while preserving its hierarchy. Do not restart a broad style exploration or add a navigation item for every backend module.

Use existing tests for behavior and integrity. Add focused regression checks for real missing or broken behavior; avoid tests that only restate CSS-like constants or duplicate the toolkit. Native observation remains necessary for focus, typing, selection, IME, scrolling and perceived layout. Source compilation and a screenshot alone cannot establish them. [R8]

Where the configured Council workflow applies, have actual independent reviewers inspect the same final visual set. Name the real models and missing access honestly. Use the owner's current cost/provider preferences; do not invent participation or routine paid model calls. Implementation and review agents should have bounded ownership and concise evidence handoffs.

## 15 Scope and completion boundaries

Complete UX coverage includes the screens and workflows above, not only the hero conversation. It does not require a new database, frontend toolkit, provider gateway, orchestration framework, scheduler, automatic folder watcher, full original-file archive or general graph product.

Use synthetic workspaces for rebuild trials. Respect supported-schema backup/restore rules and current real-data authorization. A UI migration must not silently reset, reclassify, merge or delete owner knowledge. Source rollback does not downgrade a database.

Do not hard-code sample records, model choices, statuses, source links or success messages. Every production control must perform its supported action or state its actual unavailable condition with a useful next step. A disabled or hidden feature does not count as migrated.

The receiving agent's task controls implementation, merge, publication and release authority. This package records a design and build target; it does not claim a merge, release, Council pass or owner native acceptance. Complete all work already authorized before requesting any genuinely necessary final approval.

### 15.1 Later export and publishing ideas

Mado's document export and connectors are useful product references for a later BRN publishing feature. They do not add implementation or acceptance obligations to this rebuild. [T15]

- **Polished article snapshots:** a later design may export one explicitly selected, identified article revision as HTML or PDF, with a table of contents, working source links and a clear treatment of unavailable assets. A snapshot must not silently substitute a newer revision or become the authoritative stored article.
- **Small destination adapters:** a later design may use one explicit export action, one destination and a narrow adapter contract. Decide the content/version, destination, credentials, failure reporting and publication authority when that feature is assigned. No Mado JavaScript runtime, connector contract, automatic publication or new external integration is required now.

The currently supported Markdown/assets export and full backup/restore remain required, including AC-35. Deferring new presentation exports must not remove or weaken those existing workflows.

## 16 Acceptance and verification

The acceptance checklist appended below is part of this specification. Record Pass, Partial, Fail, Blocked or Not run for every applicable case, with the exact build, platform, theme, fixture and evidence. Mark a capability unavailable only after checking the actual baseline and task scope; do not use Not applicable to hide an unfinished migration.

### Authority and test setup

The existing repository and wiki authority model governs these checks. **Current** or **Saved** means committed content; it does not mean the owner personally reviewed it. Ordinary knowledge may be maintained within an explicitly selected delegation scope. Review-required changes and protected documents retain their applicable gates. A cleaner interface must make these distinctions understandable without adding an owner-approval step to every ordinary update.

Use an isolated synthetic BRN workspace with two projects and separate conversations, ordinary current articles, a review-required article, a protected article, project-only attachments, retained sources, and a multi-note proposal. Include the [long-article reading fixture](fixtures/reading-quality-article.md), a missing external source, and controlled failure fixtures. Follow the [fixture setup](fixtures/README.md) through existing import/edit services; repository Markdown is not a substitute for actual stored document identity. Exercise tasks against harmless test targets. Do not use real external side effects to prove the design.

Reuse existing tests and native journeys where they establish the required behaviour. These 42 cases are acceptance obligations, not a requirement to write one new test per case. One coherent native journey may supply evidence for several cases. Add targeted tests for meaningful uncovered risks, especially persistence, isolation, atomicity, and interruption.

Evidence codes:

- **N — Native interaction:** observe the actual GPUI app, recording the relevant actions and result. Screenshots or short recordings support the observation.
- **B — Behaviour check:** a focused existing or new test of observable state transitions, commands, or queue behaviour.
- **S — Service/storage check:** verify durable data, identity, permissions, revisions, or transaction outcomes through the actual service boundary.
- **V — Visual evidence:** native screenshots with recorded build, theme, window dimensions, and fixture state.
- **A — Accessibility evidence:** keyboard observation and native accessibility information; use an actual assistive technology session for claims about that technology.
- **R — Receipt/log evidence:** the resulting task, proposal, intake, or run record, with sensitive content omitted.

All cases begin **Not run**. Evidence requirements below describe what must be established, not a demand to duplicate existing coverage.

### Core journeys

| ID | Scenario and steps | Observable pass conditions | Evidence |
| --- | --- | --- | --- |
| AC-01 | **Workspace continuity.** Open BRN, visit chat, a project, Library, and Inbox; change pane visibility; reopen the app. | Each retained destination is reachable. The last supported workspace restores with correct identity. Pane choices survive ordinary rerenders. There is one primary canvas and at most one visible composer. Startup restores state without inventing a new task or implicit inbox scan. | N, B |
| AC-02 | **Chat and draft isolation.** Type in conversation A, switch to B, return, send, then reopen both. | Unsent text belongs to its conversation. Sending creates one message and clears the live composer. Existing messages remain ordered and persist; no draft or response appears in the other conversation. | N, B, S |
| AC-03 | **Queued messages.** While a run is active, queue several messages; edit one, reorder pending items, remove one, then allow dispatch. | The visible queue reflects the exact pending text and order. Only eligible pending messages can be reordered. Dispatch follows that order once, without sending removed or superseded text. Keyboard users can perform the same actions without drag. | N, B, R |
| AC-04 | **Stop, Resume, and Steer.** Stop an active run with queued work, resume where supported, and steer an active run with a new instruction. | Stop reports its actual acknowledgement and any already-completed effects; queued work does not silently restart a stopped run. Resume preserves the supported continuation context. Steer visibly identifies the intended run and whether delivery was accepted or remains pending. Unsupported continuation is not presented as a successful resume. No duplicate tool effect or message results. | N, B, R |
| AC-05 | **Provider and model selection.** Select two available provider/model combinations at the actual supported settings scope; start a run and revisit the selection. | Choices come from actual configured capabilities. The run records the selection used. A later selection does not relabel an already-running request. The baseline uses workspace settings; keep that scope and its current fencing behavior unless separately changed. Unavailable capabilities are explained rather than faked. | N, B, R |
| AC-06 | **Chat to document canvas.** Open a document card, open/close its assistant, return to the conversation, and reopen the document. | The same document, conversation, source links, history, and relevant proposal appear. The document receives the main working area. There is only one composer, and its active document/reference context is accurate. | N, B, S |
| AC-07 | **Project attachments and Keep in BRN.** Attach a source to a conversation in project A; inspect another conversation, project B and global knowledge; explicitly choose Keep in BRN; repeat the retention action. | Before retention, the attachment stays within its documented conversation/project scope and is not silently promoted into global knowledge. Keep in BRN clearly identifies the retained object and destination, preserves provenance, and is idempotent under the existing identity rules. Keeping source material does not falsely mark an article personally reviewed. | N, S, R |
| AC-08 | **Articles and All files.** Browse consolidated articles, open one, switch to All files, and open a related source or supported file type. | Articles remain the default knowledge experience. All supported retained items remain discoverable through their mapped route. Opening an item preserves its identity and available provenance; the new UI does not manufacture a separate article for every source file. | N, S, V |
| AC-09 | **Search and scope.** Search from global and project contexts for a current article, conversation, retained file, and project-only attachment. | Results identify object type and scope, open the correct item, and preserve useful navigation context. Project-only material does not leak into another project's results. Current committed knowledge is not excluded merely because its valid update was delegated rather than personally reviewed. | N, B, S |
| AC-10 | **Direct editing and saving.** Edit an article; toggle panes, change theme, resize, navigate away/back, save, and reopen it. | Draft text survives supported transitions. Save status distinguishes unsaved, saving, saved, and failed states according to the service result. Reopened text equals the committed text, and no other article receives the draft. | N, B, S |
| AC-11 | **Passage comments and rewrite.** Select a passage, add a comment, request a rewrite, edit nearby text, then reopen the document. | The comment and proposed rewrite remain associated with the intended passage. If the anchor cannot be resolved, the UI says so instead of highlighting an unrelated range. Selection and comment context do not leak to another document. | N, B, S |
| AC-12 | **Review-required and protected edits.** Propose a change to a review-required article and a protected article; inspect it, discard one, and attempt an authorised apply for the other. | The preview clearly differs from current saved content. Discard removes the pending suggestion without changing the current article. Apply obeys the actual review and protection rules; ordinary delegation alone cannot bypass a protected edit gate. No proposed text is presented as already committed. | N, S, R |
| AC-13 | **Delegated ordinary maintenance.** Select an explicit ordinary-knowledge maintenance scope and run one eligible update plus one out-of-scope update. | The eligible ordinary change may commit without an unnecessary owner-review step. The out-of-scope change waits or is denied under the policy. The resulting version/receipt attributes the change accurately; Current/Saved does not imply personal review. | N, S, R |
| AC-14 | **Apply, reopen, and Undo.** Apply a valid proposal, reopen the article and app, then use the supported Undo-change route. | The exact reviewed change is durable, identity and history remain intact, and Undo remains discoverable within the documented retention rules. Repeat with a post-commit attention/comment-cleanup failure: Applied and receipt Undo remain valid, while the cleanup failure is shown separately. Undo restores the expected prior content through the service; its receipt and current status agree. Editor text undo is not confused with reversing a committed proposal. | N, S, R |
| AC-15 | **Manual Inbox Scan.** Place supported files in the configured intake location; invoke Scan; inspect a success and failure; scan unchanged input again. | Processing starts through the manual Scan action. The redesign introduces no automatic folder watcher or implicit launch scan. The receipt distinguishes processed, skipped, and failed inputs. Retry is available for failures, and repeated scanning does not create uncontrolled duplicate sources or articles. | N, B, S, R |
| AC-16 | **Tasks and completion receipts.** Create or open a task, execute a harmless supported action, complete another manually, and inspect a failed/stopped execution. | Task and run states are distinct. Completion reflects the actual outcome or an explicit manual completion. Receipts identify what happened and the relevant target; partial, stopped, and failed work is not described as successful completion. Links back to the originating task/conversation work. | N, S, R |
| AC-17 | **History and restoration.** Open a current article's history, inspect an earlier version, return to current, and restore through the supported route. | Historical content is clearly labelled. Previewing history does not change current content. Restoration retains prior history, respects protection/review rules, and produces an accurate current version and receipt. Comments/proposals are reconciled or explicitly marked stale where necessary. | N, S, R |
| AC-18 | **Settings and retained capabilities.** Visit the settings and advanced routes mapped from the old UI; change representative preferences, restart, and reset a preference. | Existing supported configuration remains reachable. Actual preferences persist at their documented scope. Model names, providers, theme options, and supported actions are not hard-coded from concept data. Unimplemented sample controls are not shipped as working features. | N, B, S |

### Failure, authority, and isolation checks

| ID | Scenario and steps | Observable pass conditions | Evidence |
| --- | --- | --- | --- |
| AC-19 | **Multi-note proposal atomicity.** Apply one proposal affecting several notes; repeat with an injected validation/write failure on a later target. | The declared application set commits together or leaves all current targets unchanged. No partially committed set is exposed as success. If recovery is needed, the proposal remains non-final until reconciled; its final state and receipt agree with every target. This guarantee concerns the BRN note transaction, not unrelated external effects. | S, B, R; N for the user-visible result |
| AC-20 | **Stale proposal conflict.** Generate a multi-note proposal, change one target through another valid route, then apply the original proposal. | The stale base is detected before commit. No target in the application set changes. The UI identifies the conflict and offers the supported refresh/rebase/review path. A regenerated change is reviewed against its actual current base; there is no silent overwrite or hidden force-apply. | N, S, R |
| AC-21 | **Undo after later work.** Apply a proposal, make a later direct edit or another valid update, then request Undo of the earlier proposal. | Undo preserves later work when the service can safely reverse the earlier change; otherwise it reports a conflict and leaves current content intact. It never blindly restores a stale whole-document snapshot over newer changes. Only the intended targets are affected. | N, S, R |
| AC-22 | **Authority changes during a run.** Start delegated ordinary maintenance, then revoke/narrow the scope or protect a target before commit. | Commit rechecks the effective authority. The run does not rely on stale permission, a model choice, or a linked document as edit authorisation. A blocked change is visible and recoverable without falsely marking the task complete. | S, B, R; N for the blocked state |
| AC-23 | **Concrete permission flow.** Trigger a supported permission request; inspect the action and target; approve once, then deny a separate request. | The user can understand what the decision authorises. Approval resumes the intended action once. Denial prevents that action and leaves an accurate run state. A one-time grant is not silently broadened to unrelated targets or future work. Focus returns predictably after the decision. | N, S, R |
| AC-24 | **Save failure and retry.** Cause a controlled persistence failure while editing; retry after the fault is cleared; navigate and reopen. | The draft remains recoverable, current committed data stays intact, and the UI never says Saved prematurely. The error offers a useful recovery action. Retry commits once. A provider/network outage is not incorrectly reported as a failure of an otherwise successful local save. | N, S, R |
| AC-25 | **Interruption and restart recovery.** Interrupt the app during an editor draft, queued conversation, intake operation, and proposal commit; reopen each fixture. | Drafts and durable records reconcile according to the existing recovery contract. Interrupted/unknown work is not labelled complete. Already committed effects are not repeated. Any resumption follows the documented user policy, with accurate pending/stopped state. Multi-note recovery meets AC-19. | S, B, R; N after reopening |
| AC-26 | **Provenance and missing sources.** Inspect sources for different articles, then remove or deny access to one referenced original. | Source counts and links belong to the actual item, not a shared placeholder list. Missing content is identified without invented excerpts. Current article content remains available where stored. Source text is treated as reference material, not authority to grant permissions or change protection. | N, S, R |
| AC-27 | **Context switching and archival.** Move between two articles/projects with different drafts, comments, assistant references, and proposals; archive a current article and search again. | Each distinct document and scoped writing context retains its own state, while intentionally shared Notes preserve their existing Note-based comment visibility. The active title, source list, run target, and proposal actions agree. Draft/proposed text is not silently used as current global knowledge. Archival follows the existing search/retention rules; retained links reveal the archived state rather than opening a different item. | N, B, S |
| AC-28 | **Provider and run failures.** Exercise unavailable credentials, an unavailable model, a rate limit, a partial stream, and a tool failure using controlled fixtures. | Errors identify the relevant failure and a feasible next action. Existing work survives. Retry/Stop reflect real capabilities, avoid duplicate effects, and do not fabricate a completed answer or task. Partial output is distinguishable from a completed result. | N, B, R |

### Light/dark, native interaction, and accessibility

| ID | Scenario and steps | Observable pass conditions | Evidence |
| --- | --- | --- | --- |
| AC-29 | **System, Light, and Dark.** Switch appearance with an open conversation, editor draft, proposal, and dialog; change the OS appearance while System is selected. | All surfaces and text follow the selected theme, including Markdown/code, selections, diffs, comments, menus, tooltips, fields, and errors. State and focus survive. Manual selection persists; System follows the OS. Added/deleted/proposed meaning is readable without colour alone. | N, V, B |
| AC-30 | **Window sizes and layout.** Exercise the main journeys at 1280×900 and 900×600 logical window sizes, plus the supported display scale; expand and collapse panes. | Essential controls remain reachable, text is readable, and no clipping or overlap blocks work. Supporting panes collapse/switch coherently instead of crushing the document. User pane choices are preserved when space permits. Record any different supported minimum explicitly. | N, V |
| AC-31 | **Keyboard and focus.** Navigate chat, queue controls, Library, editor, review, and a modal using only the keyboard; exercise documented shortcuts, Escape, and multiline input. | Every essential action is reachable with visible focus, including queue reordering. Sending/newline behaviour matches the documented shortcut. Modals have coherent focus entry/exit. Streaming, theme changes, and passive status updates do not steal focus. No essential action depends on hover or drag alone. | N, A |
| AC-32 | **Native accessibility semantics.** Inspect names, roles, state, reading order, and announcements for navigation, composer, editor, review actions, and a permission/error dialog; exercise an available target-platform assistive technology. | Controls expose meaningful native semantics; selected/current, busy, error, and disabled states are understandable. Announcements are useful without reading every token. Report assistive-technology results only for sessions actually observed; an accessibility-tree dump alone is not a screen-reader pass. | A, N |
| AC-33 | **Native text and long content.** Use Unicode/IME composition, selection, clipboard, undo/redo, scrolling, and keyboard movement in a long document and a long conversation. | Text is neither lost nor duplicated. Composition is not submitted prematurely. Selection, comments, wrapped text, scrolling, and focus remain coherent in both themes. Existing native editor behaviours remain available; the redesign does not replace them with a less capable imitation. | N, B; V for visible defects |
| AC-34 | **Actual app, toolkit migration and declared platform gates.** Verify the resolved GPUI Kit 0.9.0 dependency and lockfile, build and launch the real application on the declared release targets, run the representative chat–canvas–review–reopen journey and inspect startup/recovery. | Desktop and reusable writing consumers use the required 0.9.0 target with compatible UI dependencies. The app launches and paints correctly, executes the new UI against the real service boundary, and satisfies the active repository/platform gates. Evidence comes from the native application, not the HTML concept or a browser reconstruction. A target that could not be observed remains unverified. | N, V, relevant existing dependency/build/test evidence |

### Other retained workflows

| ID | Scenario and steps | Observable pass conditions | Evidence |
| --- | --- | --- | --- |
| AC-35 | **Export and backup restore.** Export a Markdown/assets snapshot and create a full backup to fresh test destinations; inject a publication failure; restore a valid backup into an isolated target. | Export and full backup are identified separately. Complete retained data/assets restore according to current contracts. Partial publication is not called successful. Source data and the backup stay intact; supported schema upgrades obey backup-first admission. Restored local inbox folder authorization/ledger requires fresh setup; missing selected instruction-package bytes stay visibly unavailable; transient MCP grants are not restored or silently re-authorized. | N, S, R |
| AC-36 | **Project movement and deletion.** Move a chat between projects; delete a synthetic project containing a shared document, private files and unfinished tasks; inspect the concrete choices and restore it. | Membership changes fence affected work without leaking private context. The reviewed removed/retained/task set matches the actual operation. Dirty/stale work refuses safely. Shared documents retain identity; cancelled or kept-outside tasks do not silently revive; restore never resumes runs. | N, S, R |
| AC-37 | **Web and MCP boundaries.** With web access Allowed, ask a question that needs current information (CHAT-07); compare Off and an unavailable route. Where the existing optional MCP feature is configured, change its discovery endpoint before a prior result returns, then inspect a permission request. | Only the agent's query is sent; Off and Unavailable dispatch nothing; results stay cited and untrusted. Stale discovery cannot authorize the new endpoint. Exact reviewed input and current scope bind any allowed MCP action; disabled/unavailable integration stays truthful. No new connector scope is required for this case. | N, B, S, R |
| AC-38 | **Intake intent and coverage.** Use representative currently supported files, including retained figures; compare useful-information intake with full-note import and a controlled partial conversion. | The chosen intent is explicit. A full note remains protected and readable with retained assets. Coverage gaps remain visible; a summary is not labelled a complete import. Converter/AI settings and failures reflect the actual configuration, with no hidden fallback. | N, S, R |
| AC-39 | **Explicit writing decision and capture actions.** Use Edit note on a saved version, Save as decision on a completed reply, and Capture outcomes in a posted conversation, while preserving an unsent draft. | Each action retains its distinct authority. The decision preview allows the exact title/text review before a protected confirmed note is saved. Capture outputs remain correctly scoped proposals; draft text survives. A general chat or attachment does not supply the same grant. | N, B, S, R |
| AC-40 | **Knowledge checks and useful navigation.** Open representative broken-link, stale-input and recorded-conflict results; inspect the affected records, related article view and offered repair/follow-up. | Existing checks remain reachable. Actual record identities, evidence and limitations are visible. A repair follows existing authority and review rules. Historical citations retain their meaning, and heuristics are not presented as proof. | N, B, S, R |

### Long-article reading additions

| ID | Scenario and steps | Observable pass conditions | Evidence |
| --- | --- | --- | --- |
| AC-41 | **Outline navigation and version correctness.** Open the supplied fixture; navigate nested, skipped, duplicate, Unicode and long headings by pointer and keyboard. Scroll, switch documents, rename/remove a heading in an unsaved draft, then save, view history, Apply a proposal, reopen and Undo through existing supported flows. Also use a no-heading variant. | Entries describe the displayed content and reach distinct correct targets below the toolbar. Code fences create no headings. Current-section updates do not steal focus. Old async results/anchors cannot navigate another document or generation. Empty state, optional folding, close/focus return and visibility preferences work. Read/Edit transitions and changes preserve draft and selection state. Existing conflict/review/Undo rules still govern changes. | N, B, S, A; V with VE-08 |
| AC-42 | **Long-article presentation and compact supporting views.** Read the fixture's prose, nested headings, lists, code, tables, image/caption and citations in Light and Dark at 1280×900 and 900×600. Open Outline, assistant, sources and comments in turn; exercise a controlled missing-image/source condition. | Body width, hierarchy, spacing, contrast, selection and links remain readable. Ordinary prose wraps; wide tables/code scroll locally with keyboard access. Images fit without distortion or theme inversion, and unavailable content is truthful. Supporting views follow §8 without stacked sheets, hidden essential controls or duplicated composers. Returning from a source or compact outline restores the intended position; unsent work and pane preferences survive. | N, V, A; S for source identity/access |

### Chat simplification additions (#249)

These checks come from [#249](https://github.com/ewq100/brn-rust/issues/249) and apply §6.1.1.

| ID | Scenario | Pass condition | Evidence |
|---|---|---|---|
| CHAT-A1 | **Clean start.** Open a new chat and an existing conversation. | One composer; no duplicate starting-action row; one combined model control; accurate, inspectable context. | N, V, B |
| CHAT-A2 | **Automatic web.** Ask an ordinary question that needs current information with web Allowed. | The agent can search within the same run and the answer cites its web sources; no query dialog. | N, B, R |
| CHAT-A3 | **Web Off.** Turn web access Off for one chat, send, reopen. | Main-agent and delegated paths cannot dispatch a search; the per-chat state persists. | B, S |
| CHAT-A4 | **Ownership and cancellation.** Stop during a search; switch chats while a result is pending. | Stop cancels owned search; late results never reach another chat or a newer run. | B, S |
| CHAT-A5 | **Availability.** Select an unsupported provider or account route. | Unavailable is stated honestly and differs from Off; no fallback connection. | N, B |
| CHAT-A6 | **Automatic skills.** Send a normal message suited to a built-in skill. | The agent can discover and load the skill without a / command. | B, R |
| CHAT-A7 | **Existing workspaces.** Open a workspace with a disabled built-in binding and an old manual selection. | Built-ins are available; no hidden persistent selection remains. | S, B |
| CHAT-A8 | **Message-specific command.** Type /, choose a command, send, then send another message. | Completion works; the explicit request belongs only to that message and run and does not disable other skills. | B, R, N |
| CHAT-A9 | **Skill controls.** Inspect the + menu, composer and Settings. | No built-in disable or persistent selection controls. | N, B |
| CHAT-A10 | **Model route.** Search models, filter by provider, favorite a model, select it. | Provider/account/model identity is preserved, including in Favorites. | B, N |
| CHAT-A11 | **Reasoning.** Switch between models with different reasoning support. | Only supported efforts appear; no stale unsupported value is submitted; earlier run labels are unchanged. | B, R |
| CHAT-A12 | **Existing journeys.** Attachments/Keep, scope inspection, draft recovery, Send/Queue/Stop/Steer, queue editing, Apply/Discard/Undo. | Outcomes unchanged. | B, N |
| CHAT-A13 | **Keyboard.** / selection, Escape, model-picker navigation, Enter/Shift-Enter, IME composition. | Behaves as specified. | A, B |
| CHAT-A14 | **Native layout.** New chat, active conversation, + menu, / menu, model picker and inline web activity in Light and Dark at 1280×900 and 900×600. | Before/after evidence without clipping, crowding or unreadable states. | N, V |
| CHAT-A15 | **Handoff.** Spec, acceptance evidence and PR linked to #249. | Implemented, tested, native-observed, owner-accepted and merged reported separately. | — |

### Minimum representative visual evidence

Use native captures from the same candidate build and stable sample data. Capture the selected content at a useful scroll position; do not shrink the entire app to make one image contain every state. Keep an evidence index with theme, logical window size, display scale, platform, build/ref, and linked acceptance IDs.

This is a minimum representative set, not a screenshot of every state in every theme. Several rows may share a coherent journey. Add a capture only to resolve a concrete visual risk or a relevant regression.

| Evidence set | State to capture | Theme and size | What the capture establishes |
| --- | --- | --- | --- |
| VE-01 | Conversation with an answer, references, active context, and composer | Light and Dark; 1280×900 | Quiet hierarchy, readable content, correct scope, and one composer. |
| VE-02 | Document canvas with pending review, old/new text, assistant, and Apply/Discard | Light and Dark; 1280×900 | Document priority, legible change semantics, and an unmistakable pending state. |
| VE-03 | Article Library with a current delegated-update example and access to All files | Light and Dark; 1280×900 | Consolidated-article structure and truthful Current/Saved wording without false personal-review claims. |
| VE-04 | Direct-edit draft with a passage comment and a useful pane configuration | Light and Dark; 900×600 | Readable editing, truthful unsaved state, and usable minimum-size layout. |
| VE-05 | One concrete permission dialog; one representative provider or persistence error | Permission in Light; error in Dark; 1280×900 | Opaque, readable transient surfaces and clear decision/recovery actions. |
| VE-06 | Manual Inbox Scan result; task completion receipt | Scan in Light; receipt in Dark; 1280×900 | Accurate processing and completion feedback without a new watcher or false success. |
| VE-07 | Settings with the appearance choice and visible keyboard focus | Either theme; 900×600 | Discoverable System/Light/Dark selection, focus visibility, and compact settings layout. |
| VE-08 | Supplied long article with the outline/current section and representative table, image and citation states; compact contents fallback | Light and Dark; 1280×900 and 900×600 | Reading hierarchy and contrast, outline/current-section presentation, readable block content and document-first supporting-view layout. Capture useful scroll positions, not one shrunken whole-page image. |

These captures support visual review; they do not alone prove saving, approval enforcement, transaction atomicity, or accessibility. Use the required behavioural/native evidence for those claims.

### Completion statuses and access-limited evidence

Record each case with an ID, result, candidate build/ref, platform, relevant fixture, evidence links, and any remaining gap.

- **Not run:** no acceptance evidence yet.
- **In progress:** validation is active; no pass claim.
- **Pass:** every required part of the case is established for the stated target/build, including native observation where required.
- **Partial:** some evidence passes, but a required part remains unobserved or incomplete. State the missing part explicitly.
- **Fail:** an observed result contradicts a pass condition. Include a reproducible symptom and affected scope.
- **Blocked:** a required check cannot currently be performed; name the access, environment, fixture, or dependency limitation.
- **Not applicable:** use only with a documented baseline/scope reason. It cannot remove a feature or gate explicitly required by this specification.

Keep implementation progress separate from acceptance. “Implemented,” “compiles,” “automated checks pass,” “visually reviewed,” and “native journey passed” are different claims. Do not average a serious data-integrity, authority, or draft-loss failure into an overall design score.

**When native access is limited, work may continue.** Complete the authorised implementation, inspection, focused automated checks, build packaging, and a reproducible native handoff. Record native cases as Partial or Blocked and state exactly what was and was not observed. A locked Mac, unavailable display, browser-only preview, simulated view tree, or screenshot from another build does not establish a native pass. Do not bypass access controls or replace the missing evidence with a claim of success.

The rebuild is accepted only when the retained-feature mapping is complete, required cases have the necessary evidence, material failures are resolved, and required native/platform gates have been observed. If native observation is still unavailable, the accurate handoff is **implementation complete with native acceptance pending**, followed by the exact remaining checks.


## 17 Definition of done

The full rebuild is complete only when:

- The required GPUI Kit 0.9.0 migration is implemented, the lockfile and active consumers agree, and relevant compatibility checks pass.
- Every supported behavior in the preservation inventory has a working, discoverable destination and a recorded verification result.
- The final app uses the selected shell throughout; legacy tiles or dialogs do not remain as unexplained islands.
- Light, Dark and System work across all BRN-owned surfaces and preserve work during appearance changes.
- Long articles meet the outline and reading-quality contracts in AC-41/AC-42, including version correctness and compact layout in both themes.
- Chat, writing, sources, projects, review, queue controls, tasks and recovery work together with correct identity and scope.
- Relevant deterministic checks and required native/platform acceptance pass for each declared release target. Otherwise report the distinct interim status “implementation complete with native acceptance pending.”
- Blocking independent review findings affecting usability or data integrity are resolved. Any remaining nonblocking limitation is explicitly documented.
- The agent supplies changed paths/commits, a concise behavior summary, screenshots tied to the final build, acceptance results, remaining limits and current merge/release status.

User-facing success is simple: Evo can open BRN, understand the current task immediately, write comfortably, find useful knowledge, and trust that his work and context survive the transition to the new interface.

## 18 Sources and supporting references

Repository links are pinned to the inspected baseline. Toolkit documentation is a capability reference and must be reconciled against the required GPUI Kit 0.9.0 target. The geometry, palette and information architecture in this specification are selected design decisions, not claims of measured native performance.

[R1]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/crates/brn-desktop/README.md "Desktop behavior and qualification boundaries"
[R2]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/crates/brn-desktop/Cargo.toml "Pinned desktop dependencies"
[R3]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/crates/brn-desktop/src/threads_layout.rs "Current tile and layout inventory"
[R4]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/docs/architecture/threads-target.md "Canonical target and delegation"
[R5]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/crates/brn-writing-ui/README.md "Reusable writing boundaries"
[R6]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/docs/architecture/invariants.md "Shared service and data integrity contracts"
[R7]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/crates/brn-desktop/src/threads_inbox.rs "Manual document inbox"
[R8]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/docs/development/verification.md "Verification and native evidence"
[R9]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/docs/architecture/threads-behavior.md "Shared writing context and review outcomes"
[R10]: https://github.com/ewq100/brn-rust/blob/e749a83e207c5b6d3c081d9f7d1fa6d6efed088c/crates/brn-threads-app/README.md "Application service and restoration contracts"
[T1]: https://gpui-kit.com/component/sidebar/ "Sidebar components"
[T2]: https://gpui-kit.com/component/textarea/ "Multiline input"
[T3]: https://gpui-kit.com/component/text-view/ "Formatted text rendering"
[T4]: https://gpui-kit.com/component/resizable/ "Resizable panels"
[T5]: https://gpui-kit.com/component/theme/ "Theme configuration"
[T6]: https://github.com/longbridge/gpui-kit/blob/v0.9.0/crates/component/src/theme/mod.rs "GPUI Kit 0.9.0 theme implementation"
[T7]: https://gpui-kit.com/docs/accessibility/ "Toolkit accessibility"
[T8]: https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html "Text contrast guidance"
[T9]: https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html "Control contrast guidance"
[T10]: https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html "Meaning beyond color"
[T11]: https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html "Visible focus"
[T12]: https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html "Reduced interaction motion"
[T13]: https://github.com/longbridge/gpui-kit/releases/tag/v0.9.0 "GPUI Kit 0.9.0 release and breaking changes"
[T14]: https://github.com/longbridge/gpui-kit/releases/tag/v0.8.1 "Intervening GPUI backend migration notes"
[T15]: https://madomd.com/ "Mado product reference for reading, outline and document export"
[T16]: https://gpui-kit.com/base/text-view/ "GPUI Base TextView rendering, selection, styling and range mapping"

Prior related requirement: [BRN Wiki Library Specification](BRN_Wiki_Library_Spec.md). It supplies the related article consolidation and evidence requirements; this specification supplies the selected interface. The repository-local copy replaces the original ChatGPT file link for build-agent access. Current canonical contracts and task authority retain their existing ownership.
