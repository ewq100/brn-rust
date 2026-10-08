# Information architecture and navigation

## Window regions

Minimal by default (owner direction D16): sidebar and chat only; the vault rail
opens on demand.

```text
┌ Header: ▯ brn · (phase badge only when not Ready) ······ Stop · ⤢ · ▯ ┐
├ Sidebar ──┬ Centre: document pane ┃ chat ─────────────┬ (Vault rail) ┤
│ New chat  │ (a surface or note)   ┃    question  ▐    │ closed by    │
│ Dashboard │                       ┃ answer            │ default      │
│ Inbox  3  │                       ┃ model · effort    │              │
│ Needs Rev │                       ┃ ┌ composer ─────┐ │              │
│ Review    │                       ┃ │ model▾ think▾ │ │              │
│ Chats     │                       ┃ └───────────────┘ │              │
│ Settings  │                       ┃                   │              │
└───────────┴───────────────────────┸───────────────────┴──────────────┘
  (status strip appears only for a real notice)
```

- **Header** — sidebar toggle, brand, a phase badge only when not Ready
  (Answering · provisional / Rewriting · knowledge unchanged / Workspace
  unavailable), Stop while work runs, Focus, vault toggle, and Settings when the
  sidebar is hidden.
- **Sidebar (left)** — *where work lives*, in order:
  1. **New chat** (⌘N).
  2. **Needs you** (D24) — one count for everything waiting for the owner. It
     opens a queue grouped as Due (Actions), Decide (proposals, with *+ Proposal*
     / *+ Action*), Sort (Inbox) and Check (Needs Review findings); each group
     links to its full view (Dashboard, Inbox, Needs Review).
  3. **Chats** — titles only; last activity is in the tooltip.
  4. **History** (Activity plus decided proposals) and **Settings** (⌘,) pinned
     to the bottom.
- **Centre** — chat is always present, as a centred reading column. With no
  conversation the greeting and composer sit together in the middle. Opening a
  surface or note shows it in the **document pane** beside the chat. Narrow
  windows switch to *Document | Chat* tabs.
- **Composer** — the question box, the **model** and **thinking-effort**
  pickers (D17), Search (no AI) and Ask / Stop.
- **Vault rail (right, closed by default)** — the Markdown vault: notes in the
  selected retrieval scope (Current / Source / History / All), relationships,
  recovered edits and index state. Open it with the header toggle or ⌥⌘0.
- **Status strip** — only when there is something to say: a last event worth
  reading, startup failure, restored backup or layout warning.

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
