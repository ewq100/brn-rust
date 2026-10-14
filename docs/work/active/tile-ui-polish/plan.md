# Reusable workspace areas implementation plan

**Goal:** Ordinary navigation reuses main and companion areas; only explicit Open beside adds a split.

**Authority/spec:** Owner accepted this behavior on 14 October 2026 and requested implementation and polish. This plan records that accepted scope. Baseline `89a1a944edd79069ce2968111daf013607f5ec14`, isolated branch `codex/tile-ui-polish`. Unrelated retrieval edits remain in the owner's checkout.

**Architecture:** Keep existing singleton tile content and checked services. Give layout slots explicit Navigation, Main, Companion and Beside roles; route ordinary opens through replacement and explicit opens through insertion. Settings is a dismissible surface retaining the underlying arrangement. No persistence/schema/dependency changes.

**Tech stack:** Rust 1.98.1, locked GPUI Kit 0.7.3 / GPUI 0.3.9.

## Behavior and constraints

- Chats, note browsing and all-chats browsing reuse the navigation area, with persistent mouse navigation to switch between them. Notes, overview, Actions and history use Main. Chat uses Companion alongside a main Note, otherwise Main. Linked work, Connections and Compare use Companion.
- An already open kind is focused and revealed, never duplicated. Explicit Open beside adds a previously unopened kind adjacent to focus; those deliberate splits persist until closed or reset. Singleton note/chat content is retained; multiple independent note editors are outside this change.
- Settings opens in a separate dismissible workspace view; Close/Escape returns to it without changing its arrangement. Ordinary navigation dismisses Settings. Solo becomes a reversible focus mode using the same maximize mechanism, rather than deleting the arrangement. Write/Chat explicitly reset arrangements.
- Keep resizing/reordering and add an obvious Open beside menu plus Close other splits. Make navigation and overflow recovery reachable with mouse and keyboard; reveal focused panels on horizontal overflow. Keep important content readable at narrow widths.
- Changing visible areas must preserve unsaved notes, per-chat drafts, exact review candidates, pending changes and running work. Opening another note must not silently replace an existing chat selection; explicit chat selection/new-chat actions still change it.
- All writes remain through existing checked services. No live providers or real user data. No release or merge authority assumed.

## Review focus

1. Repeated normal navigation after explicit splits: no further accumulation or duplicate kinds.
2. Hidden dirty note/review editors: preserve input and move keyboard focus to visible content.
3. Different note with an existing conversation draft: preserve conversation identity and text.
4. Narrow windows and far-off splits: focus must actually reveal the destination.
5. Settings and Solo dismissal: restore the existing arrangement and usable keyboard focus.

## Implementation

- [x] Layout (`threads_layout.rs`): add slot roles, bounded `open`, explicit `open_beside`, Settings surface state, reversible Solo, and `close_other_splits`. Unit journeys cover replacement, deduplication, resets, mixed navigation and split operations, focus/close and resize.
- [x] Shell/navigation (`threads_shell.rs`, `threads_tiles.rs`, `threads_lists.rs`, `threads_ui.rs`): add persistent navigation controls, explicit split labels, visible Open beside menu and Settings surface. Retain one scroll handle and reveal focused tile after layout. Preserve editor input before replacing/hiding content and fix note navigation so it doesn't retarget an existing chat.
- [x] Native tests: exercise actual mouse menus, 30+ normal navigation transitions, explicit split/close, offscreen reveal, draft and exact-review preservation, Settings/Escape and reversible Solo. Update only tests whose old tile-accumulation/context-switch expectations are intentionally replaced.
- [x] Update canonical target and status to describe the new accepted navigation contract and evidence limits.
- [x] Verify pinned offline native build/tests, strict native lint, formatting, Markdown links and diff whitespace. Independently review the complete diff and fix supported findings. Inspect native synthetic UI if available; otherwise explicitly retain native observation as pending.

## Gates

- Defined: accepted by owner in chat.
- Plan review: approved by independent read-only reviewer.
- Implementation and independent code review: complete; 111 tests, strict lint and native build pass, with checks recorded in [evidence](evidence.md).
- Native observation: representative synthetic navigation/draft recovery/Solo observed. Owner acceptance of the implemented UI remains pending.
- Publication / merge / release: outside this implementation request.
