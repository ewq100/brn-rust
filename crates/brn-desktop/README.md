# BRN Threads desktop

Launch the native build with a fresh explicit absolute data directory:

```sh
cargo run -p brn-desktop --features native-ui -- --data-dir /absolute/fresh/threads
```

The window is an Omakase tiled workspace: a top bar (launcher ⌘K, Write/Chat/Solo layouts, Needs-you badge), resizable tiles and a status bar (run state and Stop, note save state, layout, theme, keys). Drag gaps to resize, drag headers to swap, double-click a header to maximize, right-click it for the tile menu. Five themes (Sammal, Läänemeri, Raba, Virmalised, Paber) are saved in Settings once no run is working.

The chat list shows Needs you, recent chats, an all-chats count and an archived count. Archived chats remain findable in history and ⌘K; Unarchive is explicit and leaves work paused. Answered chats stay in history. The notes tile offers filter/search, Recent, Linked, Protected & imported and A–Z sections. Notes are typed in place with explicit Save; recovery buffers survive restart and delayed generations cannot overwrite typing. Comments are highlighted with margin cards; Rewrite with AI and in-place tracked-change review apply exactly the shown candidate, then offer Mark addressed and receipt Undo. What changed exposes receipts and checked Undo.

The chat composer sends with ↵ (⇧↵ new line); ordinary messages carry no extra note or decision grant; model and effort pills change the explicit Settings. Edit note appears with an open saved note and collects a separate instruction for that exact note/version. Save as decision appears on completed saved replies and opens an editable review before retaining the exact text as a protected, confirmed BRN note. Both actions preserve unsent chat drafts. Intake (＋) offers useful information or a protected full readable note. All eight kinds use the captured Settings converter: stock `brn-xberg-helper` (default) or optional `brn-xberg-helper-brn`, with independent AI conversion Off by default. Both are installed siblings; missing/wrong helpers refuse without fallback. Import gaps are visible; originals stay external. Managed figures render only retained SQLite asset bytes. Export and backup publish to fresh destinations.

The first-use delegation choice (Maintain ordinary notes or Review changes first) appears in the new-thread chat, the overview and Settings › Delegation; manual work stays available before choosing. Stop and send saves the instruction and fences old writes before cancelling; its replacement waits for the old worker to finish. Configuration and policy changes fence working runs; Continue invokes the provider fresh using BRN-owned state.

Implementation: [view and services wiring](src/threads_ui.rs), [shell](src/threads_shell.rs), [navigation tiles](src/threads_tiles.rs), [chat and linked tiles](src/threads_writing.rs), [writing panel](../brn-writing-ui/src/native.rs) and [document/review views](../brn-writing-ui/src/writing.rs), pure [layout](src/threads_layout.rs), [lists](src/threads_lists.rs), [tracked changes](../brn-writing-ui/src/diff.rs), [themes](../brn-writing-ui/src/theme.rs) and [state rules](src/threads_state.rs), and the [shared app](../brn-threads-app/src/lib.rs). Headless widget tests do not prove pointer feel, typing, focus, dialogs, IME, selection or rendering; see the [Omakase evidence](../../docs/work/active/omakase-ui/evidence.md) for the owner qualification checklist.

Toolkit snapshots require an observed control; a plain `.id(...)` does not register one. Check the pinned toolkit's API before asserting state: `label()` may be an explicit accessibility label rather than visible text, and `disabled() == None` means no native disabled flag was exposed. Use supported control observations; these remain distinct from actual Mac accessibility or rendering checks.

### Standalone chat

Enter sends when idle and queues while work runs; Shift-Enter adds a line. Steer redirects the current invocation through a stopped fresh run. Stop pauses the queue; failures and restart preserve waiting messages until Resume. Queue cards support edit, remove and move earlier. Composer drafts survive navigation/restart, including a Note composer adopted by its first writing conversation. Adoption persists the destination and clears the old recovery atomically; an empty displayed destination also restores the text in its composer. Stale or occupied destinations keep the original draft separately with a visible failure notice identifying the Note. That recovery attention survives background status until the same Note hands off successfully or the owner dismisses it; completed work remains available through Last work status.

Answers stream in a per-chat MessageScroller with tail following, selectable Markdown, Copy and Jump to latest. Quiet on hides intermediate activity throughout the window while retaining streamed answers, status, Stop, errors and reviews. Quiet off shows a bounded terminal-style tool trace with argument previews, truthful outcomes and public provider reasoning summaries; the latest trace remains in its own chat until a new run or app restart. Archive preserves chat history, attention and Actions and stops work; Unarchive does not resume it.

Attach file retains converted readable content and meaningful figures for this chat with visible coverage limitations. The original may move or disappear. Inspect shows the saved note and figures; Keep in BRN retains that same protected note in general knowledge with version checks and Undo. Projects navigation creates groups, chats and documents, links existing documents, imports private files, edits shared instructions and inherited BRN-knowledge settings, and moves chats via their context menu. Delete reviews the concrete removal/retention set and unfinished-task choices; Trash restores recoverable contents without restarting work. Shared writing uses the project selected when opening the document. Adopted schema3–9 workspaces require a verified external pre-upgrade backup before upgrading to current schema10. Source rollback does not downgrade the database; preserve the pre-upgrade snapshot. Native interaction qualification is separate from widget/state tests.


The status bar shows Idle or current work, without repeating the composer's provider/model/effort controls. The latest completed worker banner can be hidden with × and restored from Layout → Last work status during the same app session. Restore never restarts work or replaces a newer visible status/error; answers and saved changes remain in their existing history.

The MCP Settings endpoint is a discovery draft, separate from the configured active
connection. Changing it hides old metadata/errors and refuses stale approval; late
discovery is admitted only for the same canonical endpoint and context. Discover
and inspect the new destination before approving it. Exact runtime input review
and the checked service remain authoritative.
