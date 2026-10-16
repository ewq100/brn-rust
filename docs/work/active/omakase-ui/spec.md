# Omakase native UI rebuild — accepted scope

**Date:** 12 October 2026. **Owner request:** rebuild the native UI in the accepted Omakase style (prototype variant B) with full mouse support and five lead-chosen themes; start the build and publish a PR. Do not merge, release, push to main, use real data or make live provider calls. Native interaction is not part of this task; headless native tests only, plus one owner UI qualification checklist.

This is presentation and navigation over the existing checked services, plus quick-question auto-resolve and reply-to-reopen. It does not change checked writes, versioned Save, dirty/stale guards, host-owned authority/grants, exact-candidate review, receipt Undo, rewrite snapshots, comment anchoring, Stop-and-send, cancellation/recovery, export/backup, intake or delegation policy. No new dependencies or SQL schema changes.

## Shell

- **Top bar:** brand, launcher field (⌘K), **Write / Chat / Solo** layout buttons, “N need you” badge with a menu of those threads. No numbered workspaces.
- **Tiles** with gaps, square corners, monospace chrome and system-font prose. Each header has a title, tile controls, maximize and close. Mouse: drag a gap to resize, drag a header onto another tile to swap, double-click a header to maximize/restore, right-click a header for Maximize / Move left / Move right / Close / Open tile. Tile kinds: threads, note, chat, linked work, notes, all threads, overview (⌘0 home equivalent), Actions, What changed, Settings, compare.
- **Layouts:** Write = threads · note · chat; Chat = threads · chat · linked work; Solo = focused tile only. Opening a tile inserts it after the focused tile; a narrow window scrolls tiles horizontally.
- **Status bar:** run state + Stop, current note save state (click saves), layout menu, ~~theme menu~~, keys/mouse help. Workspace notices appear in a dismissible line above it. *Superseded by owner decision (UI polish, open question 4): the status bar has no theme control; theme lives in Settings › General and ⌘K ([UI polish plan](../ui-polish/plan.md)).*
- **Launcher ⌘K:** notes, threads (including resolved, marked ✓) and commands (new thread/note, rewrite, save, layouts, themes, open tiles, What changed, Actions, Settings, keys). Menus and launcher rows show shortcuts.

## Themes

~~Selectable from the status bar, launcher and Settings › General.~~ Selectable from Settings › General and the launcher (⌘K); owner decision, UI polish open question 4 ([UI polish plan](../ui-polish/plan.md)). Hover/active/selection/add/delete/comment tints are derived per theme. Text, muted and accent colors must reach 4.5:1 against tile and alternate backgrounds; the only adjustment needed is **Raba** accent `d9534b` → `df6d66` (4.0 → 4.5:1).

| Theme | paper | tile | alt | border | ink | muted | accent |
|---|---|---|---|---|---|---|---|
| Sammal (default) | f7f5ee | fffdf7 | f1f5ed | 9cad91 | 283526 | 56634f | 416438 |
| Läänemeri | eef2f5 | fbfcfd | e6edf2 | 9fb2c1 | 1d2b36 | 52646f | 2f6690 |
| Raba (dark) | 1c1a16 | 24221c | 2b2921 | 4d4a3c | e6e0cc | a39e86 | df6d66 |
| Virmalised (dark) | 0f1419 | 151b22 | 1b232c | 2f3b47 | d6e2ea | 8796a3 | 5fd6a4 |
| Paber | f2f1ed | ffffff | eceae4 | 8a8a85 | 111111 | 555550 | 111111 (white primary text) |

**Persistence.** The Settings record gains a serde-defaulted optional `theme` (old records and serialized forms unchanged; no SQL schema change). Every Settings write bumps its version and running invocations are bound to that version, so a theme change uses a dedicated owner settings update that is deferred, in the same writer transaction, while any run is Working. The UI applies the theme immediately and saves it once no run is working. Layout is session state.

## Threads and notes

- Vocabulary is **thread** everywhere; every conversation is a thread.
- **Resolve only.** Resolved threads leave the sidebar (count only, opening the all-threads tile), remain findable in ⌘K and the all-threads tile, and reopen with a recorded reason when the owner replies or adds feedback. Resolving never completes linked Actions.
- **Quick questions auto-resolve** through the checked resolve path, with a distinct recorded reason, when the run completed successfully and the thread is standalone (no links), has exactly one owner message, no attention, no pending candidate that references it and no receipt bound to its runs that wrote anything except its own messages, runs or thread record.
- **Sidebar (threads tile):** Needs you (all attention threads with reasons, plus recoverable edits and pending reviews), Open (10 most recently active), “all threads · N →”, resolved count, and links to notes, Actions, What changed and Settings.
- **All-threads tile:** filter-as-you-type; Today / This week / Older (UTC dates); filters all/open/resolved/needs you/linked note/has Action; right-click Open, Resolve/Reopen, Open note. No stale-thread nagging.
- **Notes tile:** filter (titles plus the existing current-note keyword search), sections Recent · Linked to open threads · Protected & imported · All A–Z by letter; right-click Open / New thread about this note. Lists are capped with explicit counts so about 5,000 notes never render as one flat list. No folders or tags.

## Chat tile

Conversation rhythm: owner turns marked, BRN as prose, one monospace activity/run line with Stop, proposal cards (diff preview, Show in note, Dismiss, Apply/Approve of that exact candidate), attention cards, existing Continue and Stop-and-send. Header: ✓ Resolve / Reopen and a ▾ menu of threads for the current note plus “New thread for this note”. The composer lives in the chat tile: ↵ sends, ⇧↵ inserts a newline, ⌘↵ also sends; ＋ offers the existing two intake choices; a mode pill maps exactly to existing grants (Ask · read only → none; Edit *open note* · this request → `EditNote{id, version}`; Record decision · this request → `RecordDecision`) and resets to Ask after sending; model and effort pills change the explicit Settings value through the existing configure path (disabled while work runs; no fallback). “New thread” (＋, ⌘K, ⌘T) is standalone; opening a note selects its linked thread or a new thread for that note.

## Note tile and review

- Always-editable note text (no Read/Edit toggle). Header: title, status chip (protected/working vN, unsaved), 💬 margin toggle, ⋯ menu (Save, Discard edit, Reload saved note, Save text as a new note, Protect/Make working note, Export, Coverage details, Comment on selection, Rewrite with AI).
- Comments: persistent highlights on the saved text via editor range decorations; Word-style margin cards with include-in-rewrite, Show passage and Mark addressed after Apply; selecting text shows a selection strip (Comment ⌘/, Ask in thread) and the editor right-click menu offers the same. The comment composer opens in the margin. With an unsaved edit, Comment is disabled with that reason.
- Foot bar: “N comments · M thread messages” and **Rewrite with AI**; unsaved state with Save/Discard; candidate state while reviewing; “Applied vN · Mark K comments addressed · Undo” after Apply. Saved rewrite inputs and Retry remain available.
- Review in place: the whole-note candidate appears in the note tile as inline tracked changes (word-level for light paragraph edits, whole-paragraph delete/insert blocks for heavy rewrites) with a sticky bar: inputs ⓘ, Compare (side-by-side tile), Edit final text (creates a new candidate), Dismiss, Apply/Approve, Keep for later. Approval binds the exact displayed candidate; no per-hunk acceptance. Multi-member group selection remains.
- Linked work tile: sources (Open/Copy reference), linked note excerpts, passage context and linked Actions. Actions, What changed (with Undo), Settings (General/theme, Models, Delegation, Data) and the overview (Needs you, first-use delegation, recover edits, review candidates, recent changes) are tiles.

## Keys

⌘K launcher, ⌘S save, ↵/⌘↵ send, ⌘. stop, ⌘/ comment, ⌘⇧R rewrite, ⌘N new note, ⌘T new thread, ⌘L focus composer, ⌘\ toggle threads tile, ⌘[ ⌘] focus tile, ⌘⇧↵ maximize, ⌘, Settings, ⌥1/2/3 layouts, ⌘0 overview. ⌘[, ⌘] and ⌘. are also bound inside text inputs so the global meaning wins over toolkit indent/code actions.

## Feasibility decisions (GPUI Kit 0.7.1)

No new editor. Highlights and tracked changes use the toolkit editor's range/text decoration collections; the diff view is a read-only editor showing the candidate interleaved with deleted text. The toolkit does not expose selection pixel bounds or hover offsets on the editor, so the selection toolbar is a strip above the text instead of floating at the selection, and comment hover cards are replaced by margin cards that reveal their passage. Intake is unchanged (one-file picker, two choices).
