# Interaction patterns

## Keyboard and focus

Implemented shortcuts (menus *BRN*, *View*, *Navigate*):

| Shortcut | Action |
| --- | --- |
| ⌘N | New chat |
| ⌘L | Focus the composer |
| ⌘. | Stop the running request or Rewrite |
| ⌘S | Save to Markdown (note editor) |
| ⌘, | Settings |
| ⌘0 / ⌥⌘0 | Toggle navigator / vault rail |
| ⇧⌘↩ | Focus mode |
| ← → (⇧ for larger steps) | Move a focused divider |
| Esc | Close a dialog |

Rules: every action is reachable by keyboard; focus is always visible (2 pt cyan
ring); dialogs move focus to their safest default, never to a destructive
confirm; closing a dialog returns focus to the invoking control. Planned:
⌘↩ to Ask from the composer, ⌘K command search, ⌘[ / ⌘] for back/forward in
the document pane. VoiceOver support in the toolkit is limited; screen-reader
qualification remains open.

## Long content and resizing

- Long titles truncate with an ellipsis in rows; the full value is available in
  the open document. Paths and UUIDs in `meta` lines may wrap; they are never cut.
- Each pane scrolls independently. Decision bars and composers stay pinned.
- Reading text keeps a maximum width; wide panes add margin, not longer lines.
- Rails auto-collapse on narrow windows (vault first), shown with an amber edge
  and a tooltip; the centre falls back to *Document | Chat* tabs. Minimum window
  480 × 480. Auto-collapse never overwrites saved preferences.
- Surfaces whose headers scroll with content (long forms such as Inbox and
  Needs Review) keep the first actions within one screen at the minimum size.

## Feedback

| Situation | Where | How |
| --- | --- | --- |
| Something is running | Header phase badge (AI tone) + Stop | “Answering · provisional”, “Rewriting · knowledge unchanged” |
| An operation finished | Status strip | One plain sentence: what happened and what did not change |
| A list or surface is loading | In place | Hint text; earlier content stays until replaced |
| Partial results | In place | Keep them, label them “provisional” or “not saved” |
| Counts | Navigator | Only when known |

## Errors, cancellation and recovery

Every failure message answers three questions (vision §30): **what succeeded,
what failed, what is unchanged** — and offers the explicit next step.

- Show errors as `callout(Danger)` next to the thing that failed, with the exact
  workflow message and a retry control that repeats the **same** request.
- Never retry silently with another provider, model or account.
- Stopping keeps partial output visibly provisional; Stop intent survives late
  replies.
- Unsaved typing is never discarded by navigation, closing or failures. If it
  cannot be saved, BRN says so and offers *Copy* before anything closes.
- Uncertain outcomes are named as uncertain (“outcome not inferred”) with a
  *Reconcile* action; recovery is not a second approval.
- Offline or AI unavailable: AI actions are disabled with the reason; browsing,
  local search, notes, Actions and review keep working.

## Confirmations

Use a modal confirmation only for decisions with durable effect: approval,
completion, removal, restore, delete, discarding retained text. The dialog
names the exact object and version, states consequences, and puts the safe
choice first. Do not confirm reversible view changes.
