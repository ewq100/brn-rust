# BRN Threads desktop

Launch the native build with a fresh explicit absolute data directory:

```sh
cargo run -p brn-desktop --features native-ui -- --data-dir /absolute/fresh/threads
```

The window is an Omakase tiled workspace: a top bar (launcher ⌘K, Write/Chat/Solo layouts, Needs-you badge), resizable tiles and a status bar (run state and Stop, note save state, layout, theme, keys). Drag gaps to resize, drag headers to swap, double-click a header to maximize, right-click it for the tile menu. Five themes (Sammal, Läänemeri, Raba, Virmalised, Paber) are saved in Settings once no run is working.

The chat list shows Needs you, recent chats, an all-chats count and an archived count. Archived chats remain findable in history and ⌘K; Unarchive is explicit and leaves work paused. Answered chats stay in history. The notes tile offers filter/search, Recent, Linked, Protected & imported and A–Z sections. Notes are typed in place with explicit Save; recovery buffers survive restart and delayed generations cannot overwrite typing. Comments are highlighted with margin cards; Rewrite with AI and in-place tracked-change review apply exactly the shown candidate, then offer Mark addressed and receipt Undo. What changed exposes receipts and checked Undo.

The chat composer sends with ↵ (⇧↵ new line); its mode pill grants Edit the open note or Record decision for one request only; model and effort pills change the explicit Settings. Intake (＋) offers useful information or a protected full readable note. DOCX and EML use the sibling `brn-intake-helper`; PDF uses configured Poppler `pdftotext` and `pdfimages`. Markdown/text remain verbatim. Import gaps are visible; originals stay external. Managed figures render only retained SQLite asset bytes. Export and backup publish to fresh destinations.

The first-use delegation choice (Maintain ordinary notes or Review changes first) appears in the new-thread chat, the overview and Settings › Delegation; manual work stays available before choosing. Stop and send saves the instruction and fences old writes before cancelling; its replacement waits for the old worker to finish. Configuration and policy changes fence working runs; Continue invokes the provider fresh using BRN-owned state.

Implementation: [view and services wiring](src/threads_ui.rs), [shell](src/threads_shell.rs), [navigation tiles](src/threads_tiles.rs), [writing tiles](src/threads_writing.rs), pure [layout](src/threads_layout.rs), [lists](src/threads_lists.rs), [tracked changes](src/threads_diff.rs), [themes](src/threads_theme.rs) and [state rules](src/threads_state.rs), and the [shared app](../brn-threads-app/src/lib.rs). Headless widget tests do not prove pointer feel, typing, focus, dialogs, IME, selection or rendering; see the [Omakase evidence](../../docs/work/active/omakase-ui/evidence.md) for the owner qualification checklist.

### Standalone chat

Enter sends when idle and queues while work runs; Shift-Enter adds a line. Steer redirects the current invocation through a stopped fresh run. Stop pauses the queue; failures and restart preserve waiting messages until Resume. Queue cards support edit, remove and move earlier. Composer drafts survive navigation/restart.

Answers stream in a per-chat MessageScroller with tail following, selectable Markdown, Copy and Jump to latest. Quiet on hides intermediate activity throughout the window while retaining streamed answers, status, Stop, errors and reviews. Quiet off shows a bounded terminal-style tool trace with argument previews, truthful outcomes and public provider reasoning summaries; the latest trace remains in its own chat until a new run or app restart. Archive preserves chat history, attention and Actions and stops work; Unarchive does not resume it.

Attach file retains converted readable content and meaningful figures for this chat with visible coverage limitations. The original may move or disappear. Inspect shows the saved note and figures; Keep in BRN retains that same protected note in general knowledge with version checks and Undo. Projects remain deferred. Native interaction qualification is separate from widget/state tests.


The status bar shows Idle or current work, without repeating the composer's provider/model/effort controls. The latest completed worker banner can be hidden with × and restored from Layout → Last work status during the same app session. Restore never restarts work or replaces a newer visible status/error; answers and saved changes remain in their existing history.
