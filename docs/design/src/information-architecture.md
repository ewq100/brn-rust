# Information architecture and navigation

## Window regions

```text
┌ Header: brn · vault name · phase badge ·········· Stop · Focus · Vault ┐
├ Navigator ┬ Centre: document pane ┃ chat ─────────────┬ Vault rail ──┤
│ New chat  │ (a surface or note)   ┃ transcript        │ identity     │
│ Workspace │                       ┃                   │ scope        │
│ Chats     │                       ┃ composer          │ notes, index │
│ Review    │                       ┃                   │              │
│ Settings  │                       ┃                   │              │
├───────────┴───────────────────────┸───────────────────┴──────────────┤
└ Status strip: last event · (future) model and budget                 ┘
```

- **Header** — identity and global state only: brand, vault name, a phase badge
  (Ready / Answering · provisional / Rewriting · knowledge unchanged / Workspace
  unavailable), Stop while work runs, Focus, Vault toggle, and Settings when the
  navigator is collapsed.
- **Navigator (left rail)** — *where work lives*. Groups, in order:
  1. **New chat** (⌘N).
  2. **Workspace** — the operational surfaces, each with a count only when the
     count is already known: *Dashboard* (future *Today*), *Inbox*,
     *Needs Review*, *Activity*; later *Projects*, *People*, *Graph*.
  3. **Chats** — active sessions, newest first, with last-activity age; later a
     link to all chats and the archive.
  4. **Review** — proposals awaiting a decision, each with a state badge;
     decided proposals collapse under *Decided (n)*. Small create commands
     (*Proposal*, *Action*) sit here because creating means drafting for review.
  5. **Settings** (⌘,) pinned to the bottom.
- **Centre** — chat is always present. Opening a surface or note shows it in the
  **document pane** beside the chat (resizable divider). With nothing open, the
  chat takes the whole centre. Narrow windows switch to *Document | Chat* tabs.
- **Vault rail (right)** — *what you know*: vault identity, retrieval scope
  (Current / Source / History / All, with the scope's meaning), relationships,
  notes in that scope, recovered edits and index state.
- **Status strip** — the last event in a sentence, and persistent workspace
  notices (startup failure, restored backup, layout warning).

## Full-product navigation model

| Surface | Opens in | Reached from | Status |
| --- | --- | --- | --- |
| Chat (current session) | Centre | Navigator *Chats*, ⌘N, ⌘L | Implemented |
| Dashboard → **Today** | Document pane | Navigator | Dashboard implemented; Today future |
| Inbox | Document pane | Navigator | Implemented (text formats) |
| Needs Review | Document pane | Navigator; in-context links from answers | Implemented (subset) |
| Proposal review | Document pane | Navigator *Review*; answer actions; Inbox group | Implemented |
| Note (current, editable) | Document pane | Vault rail; search results; citations | Implemented |
| Evidence (Source / History / All, read only) | Document pane | Vault rail with scope; citations | Implemented |
| Activity and recovery | Document pane | Navigator | Implemented |
| Project view | Document pane | Navigator *Projects*; project notes; answers | Future design |
| Person view | Document pane | Navigator *People*; person notes; answers | Future design |
| Graph | Document pane (can expand with Focus) | Navigator *Graph*; *Relationships* | Future design |
| All chats and archive | Document pane | Navigator *Chats* | Future design |
| Settings | Modal dialog | Navigator footer, ⌘, , header gear | Implemented |

Rules:

- **Surfaces never replace the chat.** The owner can always ask about what is
  open. Focus (⇧⌘↩) hides both rails for reading or writing.
- **One surface at a time in the document pane.** Opening another surface
  replaces it after the existing guards (unsaved edits, unacknowledged review
  text) are satisfied; navigation never discards typing.
- **Evidence inspection returns.** Opening a citation or claim shows the
  evidence in the document pane; *Close* returns to the previous surface
  (future: an explicit *Back to …* label).
- **Settings is global.** It never changes the open document, chat or unsent
  composer text.
- **Counts are honest.** A navigator count appears only when the number is
  known from loaded data; no count is not “zero”.

## Today (future home)

*Dashboard* becomes **Today** when work planning lands: count tiles, BRN’s
suggested focus with reasons (vision §26), waiting-on-others, Inbox and Needs
Review summaries. The chat remains the default centre; Today is the first
surface in the navigator and the natural answer to “What should I focus on?”.
See the [prototype](prototypes.md).
