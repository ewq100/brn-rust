# BRN UX rebuild: capability inventory

Baseline: branch `work/ux-rebuild-20261105`, HEAD `ab455c6d`, checkout `/Users/evokessler/repos/brn-ux-rebuild-20261105`. Read-only. No repository file was changed, committed or pushed.

Scope: `crates/brn-desktop/src/*.rs` (non-test) and `crates/brn-writing-ui/src/*.rs`. Test files were read only to confirm behaviour.

Conventions
- Paths: desktop files are `threads_*.rs` under `crates/brn-desktop/src/`. Writing UI files are `wui/` (`crates/brn-writing-ui/src/`). Services are `app/` (`crates/brn-threads-app/src/`) and `core/` (`crates/brn-threads-core/src/`).
- Destination: the spec §4.2 map (`BRN_UX_Rebuild_Spec.md`). Where §4.2 has no entry, the cell says NEEDS DECISION, or names the §6 contract that covers it.
- Verification: the keybinding block (threads_ui.rs:4797-4822), `ThemeId` (wui/theme.rs:6-13), labels (wui/theme.rs:31-37), `TileKind` (threads_layout.rs:4-18), `SettingsSection` (threads_ui.rs:208-216), the Outline dropdown (wui/document.rs:161-202), `pal`/`apply` (wui/theme.rs:242-248) and `ThreadsUi` (threads_ui.rs:373) were checked directly. Other line numbers come from five read-only slice reviews and were not re-checked one by one.

## Counts per area

| Area | Capabilities |
|---|---|
| Shell | 32 |
| Keyboard | 20 |
| Chat | 39 |
| Queue | 24 |
| Recovery | 24 |
| Writing | 27 |
| Comments | 14 |
| Review | 28 |
| Library | 31 |
| Projects | 19 |
| Inbox | 9 |
| Tasks | 10 |
| Search | 10 |
| Settings | 17 |
| Data | 12 |
| Skills | 13 |
| MCP | 11 |
| Web | 6 |
| Appearance | 7 |
| **Total** | **353** |

## Shell

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-001 | CLI flags: --data-dir, --headless-check, --help | main.rs:8-58 | main.rs startup | NEEDS DECISION (CLI only) |
| CAP-002 | Sidebar chat-list toggle (⌘\) | threads_ui.rs:4808; threads_shell.rs:1733-1735 | ToggleThreads handler | RETIRE-CANDIDATE (replaced by §4.1 sidebar) |
| CAP-003 | Nav pill Chats | threads_shell.rs:861-862 | show_tile(Threads) | Sidebar recents |
| CAP-004 | Nav pill Notes | threads_shell.rs:863 | show_tile(Notes) | Library Articles |
| CAP-005 | Nav pill Projects | threads_shell.rs:864 | show_tile(Projects) | Project workspace |
| CAP-006 | Nav pill Tasks | threads_shell.rs:865 | show_tile(Actions) | Inbox Tasks |
| CAP-007 | Nav pill Changes | threads_shell.rs:877-883 | show_tile(History) | Recent changes command |
| CAP-008 | Home pill and ⌘0 | threads_shell.rs:1025-1035; threads_ui.rs:4805 | show_tile(Overview) | NEEDS DECISION (no §4.2 entry) |
| CAP-009 | Launcher button "Search…" and ⌘K | threads_shell.rs:1037-1055; threads_ui.rs:2572-2589, 4799 | open_launcher | NEEDS DECISION (no §4.2 entry) |
| CAP-010 | Launcher results: Chats, Notes, Commands, In chats (max 20) | threads_shell.rs:1396-1491; threads_lists.rs:931-990 | lists::launcher; run_launcher threads_ui.rs:2625-2639 | NEEDS DECISION |
| CAP-011 | Launcher keys: ↑ ↓ ↵ Esc | threads_shell.rs:1486-1541; threads_ui.rs:2590-2624 | move_launcher / run_launcher / close_launcher | NEEDS DECISION |
| CAP-012 | Settings gear and ⌘, | threads_shell.rs:884-897; threads_ui.rs:4810 | show_tile(Settings) | Settings sections |
| CAP-013 | Settings close "Done · Esc" | threads_shell.rs:937-1009, 1663-1672 | relayout(close Settings) | Settings sections |
| CAP-014 | Needs-you badge "● N need(s) you" and menu | threads_shell.rs:1057-1122 | open_thread | Inbox For you |
| CAP-015 | Save status pill "● Unsaved · ⌘S" / "Saved" | threads_shell.rs:1250-1272 | save; show_tile(History) | Primary document canvas (WRITE-02) |
| CAP-016 | Notice banner with Dismiss | threads_shell.rs:1337-1364 | notice.dismiss (threads_notice.rs:89-100) | NEEDS DECISION |
| CAP-017 | Status menu "Last work status" | threads_shell.rs:1320-1327 | show_last_work_notice (threads_ui.rs:993-997) | NEEDS DECISION |
| CAP-018 | "Keys and mouse" help dialog (20 rows) | threads_shell.rs:223-288; threads_tiles.rs:1988-1996 | show_keys | NEEDS DECISION (Settings General is a candidate) |
| CAP-019 | Layout status menu: Layouts and Panels toggles | threads_shell.rs:1283-1319 | apply_layout; relayout(toggle) | RETIRE-CANDIDATE |
| CAP-020 | Layout presets Write, Chat, Solo (⌥1-⌥3, menu) | threads_ui.rs:4811-4813, 1029-1042; threads_layout.rs:203-213 | apply_layout / Layout::apply | RETIRE-CANDIDATE |
| CAP-021 | Open beside and Close other splits | threads_shell.rs:898-933; threads_layout.rs:386-409 | show_beside / open_beside | RETIRE-CANDIDATE |
| CAP-022 | Panel header: title, Split chip, right-click menu | threads_shell.rs:292-397, 471-485 | tile_title | RETIRE-CANDIDATE |
| CAP-023 | Panel ⋯ menu: Maximize, Move, Open beside, Close | threads_shell.rs:199-221, 398-419 | maximize/move/close items; layout.move_by (threads_layout.rs:473) | RETIRE-CANDIDATE |
| CAP-024 | Maximize and restore (button, double-click, ⌘⇧↵) | threads_shell.rs:422-466, 1744-1748; threads_ui.rs:4809 | relayout(toggle_maximize) | RETIRE-CANDIDATE (outcome: full-width reading) |
| CAP-025 | Close panel (×) | threads_shell.rs:448-458 | layout.close | RETIRE-CANDIDATE |
| CAP-026 | Drag header to swap panels | threads_shell.rs:467-470, 621-624 | layout.swap | RETIRE-CANDIDATE |
| CAP-027 | Drag gap to resize panels (8% minimum) | threads_shell.rs:539-546, 575-590 | layout.resize | RETIRE-CANDIDATE |
| CAP-028 | Click to focus a panel; ⌘[ and ⌘] cycle | threads_shell.rs:611-619, 1736-1743; threads_ui.rs:4819-4820 | set_focus / cycle_focus | RETIRE-CANDIDATE (outcome: keyboard panel switching) |
| CAP-029 | Empty panel row and horizontal tile scroll | threads_shell.rs:527-536, 562-574, 659-670 | render only | RETIRE-CANDIDATE |
| CAP-030 | Compact window (under 1000 × 700 pt) sets short_window | threads_shell.rs:15-24, 1647-1648 | compact(); consumer unverified | NEEDS DECISION |
| CAP-031 | Note toolbar and review bar fold by width | threads_shell.rs:551-558; wui/layout.rs:47-118 | NoteFit / ReviewFit | Primary document canvas (automatic) |
| CAP-032 | Window close and ⌘Q run the exit flow | threads_ui.rs:4926-4946, 4985-5003 | request_exit | NEEDS DECISION (Shell has no §4.2 entry) |

## Keyboard

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-033 | ⌘K open launcher | threads_ui.rs:4799 | open_launcher | NEEDS DECISION |
| CAP-034 | ⌘S save | threads_ui.rs:4800 | save (threads_ui.rs:1436) | Primary document canvas |
| CAP-035 | ⌘↵ send (Ask) | threads_ui.rs:4801; threads_shell.rs:1687-1698 | ask (threads_ui.rs:2138) | Primary conversation workspace |
| CAP-036 | ⌘L focus composer | threads_ui.rs:4802; threads_shell.rs:1705 | FocusAsk | Primary conversation workspace |
| CAP-037 | ⌘N new note | threads_ui.rs:4803; threads_shell.rs:1702 | new_note (threads_ui.rs:1484) | Primary document canvas |
| CAP-038 | ⌘T new chat | threads_ui.rs:4804; threads_shell.rs:1703 | new_thread (threads_ui.rs:1300) | Sidebar "New chat" |
| CAP-039 | ⌘0 Home | threads_ui.rs:4805 | Home handler | NEEDS DECISION |
| CAP-040 | ⌘/ comment on selection | threads_ui.rs:4806 | comment_on_selection (threads_ui.rs:1318) | NEEDS DECISION |
| CAP-041 | ⌘⇧R rewrite with AI | threads_ui.rs:4807; threads_shell.rs:1732 | rewrite_note (threads_ui.rs:1749) | Primary document canvas |
| CAP-042 | ⌘⇧↵ maximize tile | threads_ui.rs:4809 | MaximizeTile | RETIRE-CANDIDATE |
| CAP-043 | ⌘, open settings | threads_ui.rs:4810 | OpenSettings | Settings sections |
| CAP-044 | ⌥1, ⌥2, ⌥3 layout presets | threads_ui.rs:4811-4813 | apply_layout | RETIRE-CANDIDATE |
| CAP-045 | ⌘Q quit | threads_ui.rs:4814, 4926-4946 | request_exit | NEEDS DECISION |
| CAP-046 | ⌘[ and ⌘] previous and next panel (global and Input contexts) | threads_ui.rs:4819-4820 | FocusPreviousTile / FocusNextTile | RETIRE-CANDIDATE |
| CAP-047 | ⌘. stop current work (global and Input contexts) | threads_ui.rs:4821 | cancel (threads_ui.rs:2669) | Per-conversation activity disclosure |
| CAP-048 | Enter sends or queues; Shift-Enter newline (tooltip only) | threads_writing.rs:964-985; threads_ui.rs:607 | ask / enqueue_composer | Primary conversation workspace (CHAT-01); Shift-Enter handler unverified |
| CAP-049 | Escape closes launcher and Settings | threads_shell.rs:1531-1541, 1663-1672 | close_launcher; close Settings | Settings sections; launcher NEEDS DECISION |
| CAP-050 | Declared but unbound: ShowKeys, CloseLauncher, LauncherNext, LauncherPrevious | threads_ui.rs:193; threads_shell.rs:1720-1723, 1761 | listeners only | NEEDS DECISION |
| CAP-051 | Web dialog keys: Enter submits, Escape closes | threads_web_search.rs:185-205 | web search dialog | NEEDS DECISION (Web has no §4.2 entry) |
| CAP-052 | Formatting has no keybinding (no ⌘B or ⌘I) | threads_ui.rs:4797-4822 | n/a | Primary document canvas (gap to confirm) |

## Chat

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-053 | Send an idle message (Enter or Send) | threads_writing.rs:964-985; threads_ui.rs:607, 2138-2188 | ask → ask_with_authority → post_message / start_run | Primary conversation workspace (CHAT-01, CHAT-02) |
| CAP-054 | Composer draft kept per destination, with generation counter | threads_chat.rs:222-239; threads_state.rs:53-77 | save_composer → store.save_chat_draft | Primary conversation (CHAT-02) |
| CAP-055 | Composer destination: new chat, note-linked chat, or open chat | threads_chat.rs:240-253; threads_ui.rs:197-205 | composer_chat | Primary conversation workspace |
| CAP-056 | Send refused in archived chat ("Unarchive this chat before sending") | threads_chat.rs:240-253; threads_ui.rs:2189-2198 | composer_chat | Primary conversation (archived banner) |
| CAP-057 | Archive chat (stops work, pauses queue) | threads_writing.rs:58-68; threads_ui.rs:2199-2232 | resolve_thread → archive_chat | Sidebar recents and all chats |
| CAP-058 | Archived chip and Unarchive banner | threads_writing.rs:23-41, 58-60, 861-876; threads_ui.rs:2233-2243 | reopen_thread → workspace.unarchive_chat | Sidebar archived view |
| CAP-059 | New chat from ⌘T, "+" menu and chat menu | threads_ui.rs:1300-1303; threads_tiles.rs:162-211; threads_writing.rs:116-123 | new_thread | Sidebar "New chat" (§4.1) |
| CAP-060 | New chat for this note | threads_writing.rs:107-114; threads_ui.rs:1655-1679 | new_writing_thread | Primary conversation (note-linked) |
| CAP-061 | Note chat switcher: list the note's chats and select one | threads_writing.rs:84-106; threads_ui.rs:1620-1654 | select_writing_thread | Primary conversation workspace |
| CAP-062 | Recent chats list (10 most recent open; needs-you excluded) | threads_tiles.rs:407-431; threads_lists.rs:474-499 | sidebar() | Sidebar recents |
| CAP-063 | Chat tabs: Recent, All N, Archived N | threads_tiles.rs:441-475 | thread_filter | Sidebar recents and all-chats view |
| CAP-064 | All-chats view: search, Today / This week / Older groups, 200 limit | threads_tiles.rs:476-574; threads_lists.rs:526-620 | all_threads() | Searchable all-chats view |
| CAP-065 | All-chats filters: Open, Needs you, Linked note, Has task | threads_tiles.rs:485-498; threads_lists.rs:526-572 | ThreadFilter::accepts | Searchable all-chats view |
| CAP-066 | Chat row menu: Open, Archive or Unarchive, Open note, Move to project… | threads_tiles.rs:80-131 | open_thread; resolve_thread / reopen_thread; open_note; move_chat_dialog | Sidebar recents (Move: Projects) |
| CAP-067 | Chat row metadata: Needs you, Archived, Task, Note | threads_tiles.rs:412-418, 538-548 | render | Sidebar recents |
| CAP-068 | Chat empty states (no chats, no other open, no open, no matches) | threads_tiles.rs:376-405, 522-530 | render | Sidebar recents and all-chats view |
| CAP-069 | Model picker: ChatGPT or GitHub Copilot, with model list | threads_shell.rs:731-772; threads_writing.rs:346-389 | set_model (threads_ui.rs:2288-2299) | Primary conversation composer (§6.1) |
| CAP-070 | Load GitHub Copilot models (model menu) | threads_shell.rs:773-781 | load_copilot_models | Settings Models |
| CAP-071 | "Provider and model settings…" link | threads_shell.rs:782-792 | show_tile(Settings) | Settings Models |
| CAP-072 | Reasoning effort menu: Low, Medium, High | threads_shell.rs:794-818; threads_writing.rs:939-944 | set_effort (threads_ui.rs:2300) | Primary conversation composer |
| CAP-073 | Busy reasons on model and effort ("Available when the current work finishes") | threads_shell.rs:708-709, 748-752, 795-796 | render | Primary conversation composer |
| CAP-074 | New-chat empty start, including note-linked wording | threads_writing.rs:137-156 | chat_tile | Primary conversation workspace |
| CAP-075 | Chat unavailable state | threads_writing.rs:220-234 | conversation_detail | Primary conversation (§4.3 missing object) |
| CAP-076 | Copy reply | threads_chat.rs:298-312 | clipboard | Primary conversation (CHAT-04) |
| CAP-077 | Save as decision… on an assistant reply (editable title and text) | threads_chat.rs:256-296; threads_context_actions.rs:223-333 | decision_dialog | Primary conversation (CHAT-05) |
| CAP-078 | Edit note: instruction for a saved note (⌘↵) | threads_writing.rs:105-115; threads_context_actions.rs:15-197 | edit_note_dialog → submit_note_edit | Primary conversation (CHAT-05) |
| CAP-079 | Ask in chat about a selection | threads_ui.rs:1368-1386, 4609; wui/writing.rs:506-508, 542 | ask_about_selection | Primary conversation (from canvas) |
| CAP-080 | Attach one document to this chat (menu, drag-drop, Cmd-V) | threads_writing.rs:393-412, 899-910; threads_chat.rs:743-805 | attach_chat_path → convert_file (FullNote) | NEEDS DECISION (chat attachment vs Library All files) |
| CAP-081 | Attachment rows in chat: coverage, Inspect, Keep in BRN | threads_chat.rs:672-736 | open_note; workspace.keep_in_brn | Library All files |
| CAP-082 | Add useful information to chat | threads_writing.rs:911-922; threads_ui.rs:3184-3192 | choose_import(UsefulInformation) | NEEDS DECISION (Inbox Imports or chat) |
| CAP-083 | Import full note into chat | threads_writing.rs:926-931 | choose_import(FullNote) | Library All files |
| CAP-084 | Quiet mode toggle (hides routine activity) | threads_writing.rs:728-748 | workspace.choose_quiet | Primary conversation (CHAT-06) |
| CAP-085 | Activity disclosure with routine steps (max 32 steps, 8192 chars each) | threads_chat.rs:33-111; threads_run_activity.rs:33-67 | record_activity / RunActivity | Per-conversation activity disclosure |
| CAP-086 | Partial or interrupted reply label; "BRN · writing…" indicator | threads_chat.rs:879-933; threads_writing.rs:277-297 | message_row / stream_for | Primary conversation (CHAT-04) |
| CAP-087 | Steer: stop the current reply and send new text | threads_writing.rs:950-955; threads_ui.rs:2084-2137 | stop_and_send / stop_and_post_with_partial | Primary conversation (CHAT-03, explicit) |
| CAP-088 | Send button wording and emphasis (Send, Queue, Save queued) | threads_writing.rs:144-149, 964-985 | render | Primary conversation composer |
| CAP-089 | Transcript centred when the chat is maximized | threads_writing.rs:312-316; threads_chat.rs:329-331 | transcript_centered | RETIRE-CANDIDATE (outcome: readable width) |
| CAP-090 | Linked-work panel: notes, sources and tasks used in this chat | threads_writing.rs:1000-1021, 1049-1087 | linked_tile | NEEDS DECISION (closest: Article Related panel) |
| CAP-091 | Capture and rewrite request transcript rows | threads_chat.rs:809, 976-981 | message_row | NEEDS DECISION (reachability unverified) |

## Queue

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-092 | Enqueue while busy or paused, with confirmation text | threads_chat.rs:254-315; threads_ui.rs:2138-2146 | enqueue_composer → workspace.queue_message | Primary conversation (CHAT-03) |
| CAP-093 | Queue summary: "Queue · N waiting", "Queue paused", "Paused" | threads_chat.rs:103, 123-157, 424, 454-456 | queue_state | Primary conversation (CHAT-03) |
| CAP-094 | Show or hide waiting messages (narrow window) | threads_chat.rs:107-119, 439-441 | queue_expanded (session) | Primary conversation (CHAT-03) |
| CAP-095 | Resume queue (disabled while busy) | threads_chat.rs:343-369, 505; threads_ui.rs:2049-2051, 2141-2142 | resume_chat_queue → resume_queue, dispatch_next | Primary conversation (explicit Resume) |
| CAP-096 | Paused queue warning card | threads_chat.rs:492-506 | composer_controls | Primary conversation |
| CAP-097 | Per-item state: Running, Next, Waiting; other-chat items name their chat | threads_chat.rs:238-246, 561-565 | composer_controls | Primary conversation |
| CAP-098 | Edit a waiting message (disabled while running) | threads_chat.rs:283-291, 597-613; threads_ui.rs:368-371, 1073-1079 | workspace.edit_queued | Primary conversation (CHAT-03) |
| CAP-099 | Remove a waiting message | threads_chat.rs:306-313, 625-641 | workspace.remove_queued | Primary conversation (CHAT-03) |
| CAP-100 | Move a waiting message up (no move-down control) | threads_chat.rs:316-330, 651-667 | workspace.move_queued | Primary conversation (CHAT-03; reorder gap) |
| CAP-101 | Auto-dispatch of the next queued message | threads_chat.rs:316-342; threads_ui.rs:3062-3071 | dispatch_next → dispatch_queued | Primary conversation |
| CAP-102 | Stop current work (Stop, ⌘.) and pause the queue | threads_chat.rs:63-70; threads_ui.rs:2669-2729; threads_shell.rs:1217-1225 | cancel → stop_chat_run / pause_queue | Per-conversation activity disclosure |
| CAP-103 | Stop reply for a saved Working run ("Stop…", asks first) | threads_chat.rs:114-164; threads_ui.rs:4672 | confirm_stop_saved_run → stop_chat_run | Per-conversation activity disclosure |
| CAP-104 | Stop automatic continuation (warning) | threads_ui.rs:2695 | discard_continuation | Primary conversation |
| CAP-105 | Live status: "Stopping…", "Working quietly…", "Activity · status" | threads_chat.rs:25-40 | worker_status | Per-conversation activity disclosure |
| CAP-106 | Status bar: "Work in X · status" (click opens chat) | threads_shell.rs:1170-1216 | open_thread | Per-conversation activity disclosure |
| CAP-107 | Status bar idle and queue text | threads_shell.rs:1227-1248 | queue_state | Per-conversation activity disclosure |
| CAP-108 | Run finish text and tone ("Work ended · status unconfirmed", "Work in X · text") | threads_ui.rs:2836-2946, 2884-2897; finish_tone threads_ui.rs:296 | poll_worker Finished arm | Per-conversation activity; Needs you |
| CAP-109 | Busy notices: one worker at a time | threads_state.rs:104-120 | WorkerKind::busy_notice | Per-conversation activity |
| CAP-110 | Retry saved rewrite | threads_ui.rs:1837-1882; wui/writing.rs:1881-1893 | retry_rewrite | Primary document canvas (writing bar) |
| CAP-111 | Rewriting progress in the document | wui/workbench.rs:46-88; threads_ui.rs:460 | progress | Primary document canvas (§6.6) |
| CAP-112 | Rewrite run outcome line | wui/workbench.rs:185, 245-272, 431-452 | writing_run_outcome | Primary document canvas (§6.6) |
| CAP-113 | Rewrite response in the writing workbench | wui/workbench.rs:346-382 | writing_result | Primary document canvas |
| CAP-114 | Saved rewrite inputs disclosure ("Saved inputs" / "Hide saved inputs") | wui/writing.rs:1857-1866; threads_ui.rs:4648 | ToggleRequest | Per-conversation activity disclosure |
| CAP-115 | Search status for a run ("Note search is {status} for this run") | threads_ui.rs:1959-1964 | run status | Per-conversation activity disclosure |

## Recovery

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-116 | Continue: fresh reply from the saved chat | threads_writing.rs:524-541; threads_state.rs:216-236, 360-463; threads_ui.rs:2659-2667 | continue_run → start_run | Primary conversation (recovery card, §4.3) |
| CAP-117 | Recovery state line: Failed, Interrupted, Stopped, May still be running, Not answered yet | threads_state.rs:361-375 | recovery() | Primary conversation (recovery card) |
| CAP-118 | Recorded reason, detail, next-step and waiting lines | threads_state.rs:304-349, 441-449; threads_writing.rs:435-556 | Recovery::with_notice / label | Primary conversation (recovery card) |
| CAP-119 | Resume queue option on the recovery card | threads_state.rs:378-411 | recovery() | Primary conversation (CHAT-03) |
| CAP-120 | Retry rewrite routed from chat continuation | threads_ui.rs:2662; threads_state.rs:216-236 | retry_rewrite | Primary document canvas (rewrite owned by Note) |
| CAP-121 | Continue unavailable in archived chat ("Unarchive to continue") | threads_state.rs:309-318; threads_writing.rs:543-550 | Recovery::archived | Sidebar archived view and composer banner |
| CAP-122 | Failed worker pauses queue; interruption-state failure message | threads_chat.rs:195-211 | pause_failed_worker → interrupt_chat_run / recover_queue | Primary conversation (recovery card) |
| CAP-123 | Worker lost unexpectedly ("Work stopped unexpectedly…") | threads_ui.rs:3084-3115 | worker_disconnected | Primary conversation (recovery card) |
| CAP-124 | Startup queue recovery when pending or queued items exist | threads_ui.rs:478 | Workspace::recover_queue | Recovery (§4.3: queue stays paused) |
| CAP-125 | Composer recovery saved or failed notices, kept until dismissed | threads_notice.rs:75-100; threads_ui.rs:1155-1167 | NoticeState | Primary conversation (composer) |
| CAP-126 | Notice precedence: composer recovery, then ordinary, then remembered work | threads_notice.rs:26-35, 49-51, 102-108 | NoticeState current / restore | NEEDS DECISION |
| CAP-127 | Recover kept edit | wui/writing.rs:667-672, 697-702; threads_ui.rs:1199-1284 | open_note(recovery) → wui/note.rs:158 | Primary document canvas (WRITE-01) |
| CAP-128 | Read-only until kept edit is recovered (banner) | wui/writing.rs:689-705; wui/native.rs:214 | recovery_blocked | Primary document canvas |
| CAP-129 | Compare open edit with saved | wui/writing.rs:675; threads_ui.rs:3812-3834 | compare_open_edit | Contextual review in document canvas |
| CAP-130 | Discard open edit held elsewhere (confirm) | wui/writing.rs:679; threads_ui.rs:3763-3781 | confirm_discard_open_edit | Primary document canvas (WRITE-03) |
| CAP-131 | Stale-edit banner | wui/writing.rs:729-736; wui/workbench.rs:303 | edit_is_stale | Primary document canvas (WRITE-04) |
| CAP-132 | Saved-version preview | wui/writing.rs:749-766 | render | Primary document canvas |
| CAP-133 | Open current version | wui/writing.rs:717; threads_ui.rs:1199-1284 | open_note | Primary document canvas |
| CAP-134 | Newer-version banner | wui/writing.rs:631, 782-788 | note_banners | Primary document canvas |
| CAP-135 | Edit conflict when a second edit is open | threads_ui.rs:1419-1433 | open_note conflict branch | Primary document canvas (WRITE-04) |
| CAP-136 | Save copy as a new working note ("Recovered {title}") | threads_ui.rs:4580, 4761-4794 | save_copy | Primary document canvas (WRITE-03) |
| CAP-137 | Recovery wording points to Home ("Recover it from Home") | threads_ui.rs:1472-1474 | render | NEEDS DECISION (Home has no §4.2 entry) |
| CAP-138 | Exit preserves exact note text and cancels workers | threads_ui.rs:4974-4984; wui/note.rs:88 | preserve_note_for_exit | Primary document canvas (WRITE-01) |
| CAP-139 | Exit confirm: "Leave without keeping the latest note text?" (Discard and close or quit) | threads_ui.rs:4939-4954, 5004-5033 | confirm_local_discard_and_exit | Primary document canvas (WRITE-03) |

## Writing

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-140 | Edit note text in Write mode | wui/writing.rs:376; wui/note.rs:245 | note_changed | Primary document canvas (§6.5) |
| CAP-141 | Write and Read switch | wui/document.rs:94-106; threads_ui.rs:3899-3923 | set_note_reading | Primary document canvas (§6.5) |
| CAP-142 | Save (explicit; ⌘S; Save menu) | wui/writing.rs:155, 1681; threads_ui.rs:1436-1454 | save → wui/note.rs:331 save_note | Primary document canvas (WRITE-03) |
| CAP-143 | Reload saved note | wui/writing.rs:164; threads_ui.rs:4575 | open_note | Primary document canvas |
| CAP-144 | Save text as new note | wui/writing.rs:173, 736, 1675; threads_ui.rs:4761 | save_copy | Primary document canvas |
| CAP-145 | Discard unsaved edit (confirm "Discard unsaved edit?") | wui/writing.rs:285, 1666; threads_ui.rs:3745-3811 | confirm_discard → wui/note.rs:385 | Primary document canvas (WRITE-03) |
| CAP-146 | Unsaved and protected status chips with version | wui/writing.rs:87-98, 1657-1662 | render | Primary document canvas (WRITE-02) |
| CAP-147 | Save outcome copy: saved, stale, generation mismatch, closed, deferred | threads_state.rs:550-565; threads_ui.rs:288-294 | save_message / save_tone | Primary document canvas (WRITE-02, WRITE-04) |
| CAP-148 | New note dialog: title required | threads_ui.rs:1484-1558, 4803; wui/writing.rs:394-397 | new_note | Primary document canvas (new document) |
| CAP-149 | Note ⋯ menu (Save, Reload, Save as new note, Discard, Edit note, Connections, Coverage, Export, Brief, Rewrite, Compare) | wui/writing.rs:131-296; threads_ui.rs:4680-4700 | note_menu_section | Primary document canvas |
| CAP-150 | Protect note and Make working note | wui/writing.rs:241-247; threads_ui.rs:2551-2571 | toggle_protection | NEEDS DECISION (Protected is defined in §5; no §4.2 control) |
| CAP-151 | Outline dropdown of headings; reveal by range; "No headings in this note" | wui/document.rs:161-202, 48-49; threads_ui.rs:1387-1413 | document_controls → reveal | Primary document canvas (READ-01 to READ-04: flat, not collapsible, from editor buffer) |
| CAP-152 | Connections (open tile) | wui/writing.rs:119-122, 217-226; threads_ui.rs:4596 | show_tile(Connections) | Article Related panel |
| CAP-153 | Cut, Copy, Paste, Select all context menu | wui/writing.rs:532-549 | gpui_kit base input | Primary document canvas |
| CAP-154 | Formatting: Heading, Bold, Italic, List, Quote (icons and Format menu) | wui/document.rs:10-47, 112-156; threads_ui.rs:3924-3947; wui/formatting.rs:28 | format_note → format_selection | Primary document canvas (§6.5) |
| CAP-155 | Formatting refusal for code and complex Markdown | wui/formatting.rs:51-53, 104-106, 204-247; threads_ui.rs:3938 | safe_lines | Primary document canvas |
| CAP-156 | Markdown display: inline HTML shown literally; stored images only | wui/markdown.rs:12-33, 41-63 | safe_markdown | Primary document canvas (READ-05) |
| CAP-157 | External links open for http and https only | wui/markdown.rs:36-38, 58-61 | open_url | Primary document canvas (READ-05) |
| CAP-158 | Rewrite with AI (⌘⇧R; menu; writing bar) | wui/writing.rs:194-202, 1915; threads_ui.rs:1749-1815 | rewrite_note → rewrite_note_with_requirements | Primary document canvas (§6.6) |
| CAP-159 | Rewrite again | wui/writing.rs:1750-1757 | rewrite_note | Primary document canvas |
| CAP-160 | Rewrite scope line (whole note, comments, saved context) | wui/writing.rs:1800-1835 | render | Primary document canvas |
| CAP-161 | Blocked-reason line for rewrite | wui/writing.rs:1838-1880 | render | Primary document canvas |
| CAP-162 | Overall feedback field | wui/workbench.rs:384-400; wui/native.rs:255 | WritingPanel.general_feedback | Primary document canvas (§6.6) |
| CAP-163 | Rewrite with selected freshness; Check all requirements | wui/setup.rs:444-450; threads_ui.rs:4242-4294, 4570-4572 | rewrite_with_setup | Primary document canvas |
| CAP-164 | Note title, status and header | threads_shell.rs:300-301 | render | Primary document canvas |
| CAP-165 | Centred reading column when maximized | threads_ui.rs:4447-4448; wui/writing.rs:2692 | reading_column | Primary document canvas (automatic) |
| CAP-166 | Prepare a proposal from selected notes and tasks | threads_ui.rs:1580-1604 | prepare_review_selection | Contextual review in document canvas |

## Comments

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-167 | Comment on selection (⌘/, menu) | wui/writing.rs:181-192; wui/native.rs:29; threads_ui.rs:1318-1366 | comment_on_selection | NEEDS DECISION (§6.6 covers Comment; no §4.2 entry) |
| CAP-168 | Save and comment (selection strip) | wui/writing.rs:489-500; threads_ui.rs:3948-3978 | save_and_comment | NEEDS DECISION |
| CAP-169 | Comment composer: Add comment, Cancel | wui/writing.rs:1208-1228; threads_ui.rs:1693-1748, 4613-4623 | post_comment; cancel draft | NEEDS DECISION (§6.6 side sheet) |
| CAP-170 | Comments toggle (margin shown or hidden, with count) | wui/writing.rs:957-985; threads_ui.rs:3836-3841 | toggle_comments | NEEDS DECISION |
| CAP-171 | Comment margin placement: Wide, Narrow, Stacked | threads_shell.rs:549-550; threads_layout.rs:158-177 | margin_mode | NEEDS DECISION (placement is automatic per §6.6) |
| CAP-172 | Escape collapses stacked comments | wui/native.rs:452-465; threads_ui.rs:3842-3870 | collapse_comments | NEEDS DECISION |
| CAP-173 | Show passage in note (quote button) | wui/writing.rs:1388-1401; threads_ui.rs:1387-1413 | reveal | Primary document canvas (§6.6 anchors) |
| CAP-174 | Include or Exclude a comment from rewrite | wui/writing.rs:1449-1470; threads_ui.rs:4642-4644 | ToggleExcluded | Primary document canvas (§6.6) |
| CAP-175 | Mark addressed; Mark N addressed | wui/writing.rs:1480, 1761-1770; threads_ui.rs:1816-1836 | confirm_addressed | Primary document canvas (§6.6: explicit only) |
| CAP-176 | Comment location status: located, tied to vN, passage not found | threads_state.rs (unlocated_reason, comment_location); wui/layout.rs:133-150 | comment_location | Primary document canvas (§6.6 anchors) |
| CAP-177 | Comment empty and load-error states | wui/writing.rs:1283-1316 | render | Primary document canvas |
| CAP-178 | Highlights for comment anchors in edit and read views | wui/document.rs:214-256, 355-412 | sync_note_marks / sync_read_marks | Primary document canvas |
| CAP-179 | Comment drafts kept per note (session only) | threads_ui.rs:540-552, 1171-1198 | session map | Primary document canvas (WRITE-05) |
| CAP-180 | Comment passage row and note link in linked work | threads_writing.rs:1158-1196 | comment_location | NEEDS DECISION |

## Review

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-181 | Open rewrite proposal (Review rewrite; Show in note) | wui/workbench.rs:154-156; threads_writing.rs:685-688; threads_ui.rs:2326-2389 | open_review | Contextual review in document canvas (§6.7) |
| CAP-182 | Changes and Result view toggle | wui/writing.rs:2114-2135; threads_ui.rs:3871-3898 | set_review_reading | Contextual review (§6.7) |
| CAP-183 | Apply or Approve the exact candidate | wui/writing.rs:2195-2212, 2783-2789; threads_ui.rs:2401-2476 | apply_review → review_apply | Contextual review (REVIEW-01) |
| CAP-184 | Apply disabled reasons | wui/writing.rs:2185-2190; wui/review.rs:148-159 | exact_candidate guard | Contextual review |
| CAP-185 | Edit final text, Use this text, Cancel edit | wui/writing.rs:322-328, 2162-2166, 2249-2262; threads_ui.rs:2390-2400 | begin_edit / use text / cancel | Contextual review (REVIEW-01) |
| CAP-186 | Prepare selection for review | wui/writing.rs:2222-2231; threads_ui.rs:1580-1604 | prepare_review_selection | Contextual review |
| CAP-187 | Include or Omit a dependent group ("Include note: X", "Omit task: X") | wui/writing.rs:2413-2440; threads_ui.rs:4658 | review group toggle | Contextual review (§6.7 whole group) |
| CAP-188 | Showing: note selector ("Show <title>") | wui/writing.rs:2475-2500; threads_ui.rs:4749-4760 | select_review_note | Contextual review |
| CAP-189 | Keep for later (refused while edited) | wui/writing.rs:345, 2170; threads_ui.rs:4740-4748 | keep_review | Inbox For you |
| CAP-190 | Dismiss proposal ("The note is unchanged.") | wui/writing.rs:354, 2176; threads_ui.rs:2517-2540 | dismiss_review | Contextual review |
| CAP-191 | Proposal label and counts ("Review changes to…", +a −r) | wui/writing.rs:2053-2090, 2764-2812; threads_state.rs:471-506 | review_summary / review_label | Contextual review |
| CAP-192 | Protected chip and Approve wording | wui/writing.rs:2093-2108; threads_writing.rs:216-223, 697-705 | render | Contextual review (§5) |
| CAP-193 | Compare side by side | wui/writing.rs:313-319, 2599; threads_ui.rs:4597 | show_tile(Compare) | Contextual review (§6.7 two-column on demand) |
| CAP-194 | Sources used, reads and requirement claims panel | wui/references.rs:24-120 | sources_used | Contextual review (§6.7) |
| CAP-195 | Private-source disclosure | wui/writing.rs:2355-2369 | render | Contextual review (§5) |
| CAP-196 | Proposal unavailable here | wui/writing.rs:1985-1992, 2601-2609 | render | Contextual review |
| CAP-197 | Task facts in a proposal | wui/writing.rs:2376-2400; app action_review_text | action_review_text | Inbox Tasks (contextual task card) |
| CAP-198 | Needs-you card in chat: proposal, question or stale review; Apply, Dismiss, Show in note | threads_writing.rs:319-362, 558-716; threads_ui.rs:2422-2431 | attention_card / card_apply / card_dismiss | Inbox For you |
| CAP-199 | Rewrite-ready card in workbench ("Rewrite ready · N comments"; Dismiss) | wui/workbench.rs:90-150 | writing_proposals / card_dismiss | Inbox For you |
| CAP-200 | Proposal outcome copy: deferred, stale, needs review, superseded | threads_state.rs:541-548; threads_ui.rs:278-286 | outcome_message / apply_tone | Inbox For you or contextual review |
| CAP-201 | Undo a change (receipt) | threads_ui.rs:2541-2550; threads_tiles.rs:1376-1387 | undo_receipt → workspace.undo | Recent changes command (contextual document history) |
| CAP-202 | Applied and undone copy, with stopped-work note and refresh follow-up | threads_state.rs:508-539 | applied_message / refresh_hint | Recent changes command |
| CAP-203 | Changes list (newest first; background excluded) and Background group | threads_tiles.rs:1464-1499; threads_lists.rs:324-369 | history (session) | Recent changes command |
| CAP-204 | "May need refresh" items and "Open refresh follow-up" | threads_tiles.rs:1390-1459 | refresh_records; refresh_followup | Library menu and contextual views |
| CAP-205 | Capture useful outcomes: entry pill | threads_writing.rs:51-55; threads_capture.rs:8-78 | capture_outcomes | NEEDS DECISION (proposals go to Inbox For you) |
| CAP-206 | Capture confirmation: "Capture useful outcomes?" (Cancel, Propose outcomes) | threads_capture.rs:53-89 | save_capture_request / start_capture_request | NEEDS DECISION |
| CAP-207 | Retry capture | threads_capture.rs:90-107 | retry_capture (caller not found) | NEEDS DECISION (unverified) |
| CAP-208 | Review bar folds by width | wui/layout.rs:91-118 | ReviewFit | Contextual review (automatic) |

## Library

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-209 | Notes title search (100-title cap) | threads_tiles.rs:626-662; threads_lists.rs:755-767; threads_search.rs:127-137 | search_changed / filter_notes | Library Articles |
| CAP-210 | Notes list views: Recent (12), Most linked (12), All A–Z (150 cap), Protected and imported (12) | threads_tiles.rs:715-752; threads_lists.rs:690-740 | notes_view | Library Articles (Protected and imported: Library All files) |
| CAP-211 | A–Z letter pills with counts | threads_tiles.rs:678-714; threads_lists.rs:632-639 | notes_letter | Library Articles |
| CAP-212 | Linked to open chats list (12) | threads_tiles.rs:719-724; threads_lists.rs:770-781 | notes_linked_to_open | Article Related panel |
| CAP-213 | Note row menu: Open, New chat about this note | threads_tiles.rs:135-158 | open_note; new_writing_thread | Library Articles |
| CAP-214 | Note row metadata: Imported, inbound link count, Protected chip | threads_tiles.rs:582-600; threads_lists.rs:622-631 | NoteRow | Library Articles |
| CAP-215 | Library empty state | threads_tiles.rs:663-676 | render | Library Articles |
| CAP-216 | Topics section and Create Topics note | threads_connections.rs:322-394 | topics; create_topics → workspace.create_topics_note | Library Articles (§4.2 topics) |
| CAP-217 | Knowledge check: run and summary (25 findings per kind) | threads_connections.rs:396-426, 549-589, 643-645 | run_knowledge_check → knowledge_triage | Library menu and contextual views |
| CAP-218 | Knowledge findings: filters and paging (8 per page) | threads_connections.rs:590-642, 670-697 | knowledge.filter / knowledge.page | Library menu and contextual views |
| CAP-219 | Inspect finding and open affected note | threads_connections.rs:428-476, 654-725 | knowledge_action Inspect / Open | Library menu and contextual views |
| CAP-220 | Review existing pending fix | threads_connections.rs:456, 493, 748-757 | open_review | Contextual review in document canvas |
| CAP-221 | Draft follow-up in chat from a finding (starts no AI request) | threads_connections.rs:457, 494-534, 759-761 | draft_knowledge_follow_up | Library menu and contextual views (lands in chat composer) |
| CAP-222 | Unavailable reference state in a finding | threads_connections.rs:738-747 | render | Library menu and contextual views |
| CAP-223 | Connections tile: Direct or Two hops; Sources toggle; relation filters; open and recenter | threads_connections.rs:96-320 | refresh_connections → request_connections | Article Related panel |
| CAP-224 | Connections count, footer and failure state | threads_connections.rs:231-265, 316-318 | refresh_connections | Article Related panel |
| CAP-225 | Import full document as a Library note (DOCX, PDF, HTML, XLSX, PPTX, Markdown, text, EML) | threads_ui.rs:3184-3283 | choose_import(FullNote) → import_full_scoped_with_conversion | Library All files |
| CAP-226 | Brief and references form (opened from note menu) | wui/writing.rs:231; threads_ui.rs:3993-4000; wui/setup.rs | load_writing_setup | NEEDS DECISION (references: Library All files) |
| CAP-227 | Recheck availability and keep edits; reload saved (discard edits); close | wui/setup.rs:302-310; threads_ui.rs:4001-4082, 4480-4488 | keep_edits_after_recheck; replace_writing_setup | Library All files |
| CAP-228 | Save brief and references | wui/setup.rs:432; threads_ui.rs:4083-4108 | save_writing_setup → set_writing_setup | Library All files |
| CAP-229 | Reference purpose: Requirements, Evidence, Style | wui/setup.rs:250-255, 365-381; threads_ui.rs:4512-4523 | draft.references | Library All files |
| CAP-230 | Must be current for next rewrite | wui/setup.rs:384-393; threads_ui.rs:4524-4533 | must_be_current | Library All files |
| CAP-231 | Remove reference; Open reference as Note | wui/setup.rs:395-408; threads_ui.rs:4534-4566 | draft.references / open_note_in_context | Library All files |
| CAP-232 | Import a file as reference (Stop import) | wui/setup.rs:326, 436-441; threads_ui.rs:4567-4569, 4295-4434 | import_writing_reference | Library All files |
| CAP-233 | Find an existing Note as reference: search, attach, next results | wui/setup.rs:458-490; threads_ui.rs:4109-4221, 4493-4511 | search_writing_references / attach_match | Library All files |
| CAP-234 | Coverage label "Original import" and coverage details | wui/setup.rs:257-271; wui/writing.rs:263, 841-849; threads_ui.rs:4600-4603 | import_label; show_coverage | Library All files |
| CAP-235 | Import summary banner (Full imported reference, Useful information) and Open original | wui/writing.rs:794-850 | note_banners | Library All files |
| CAP-236 | Import report details | wui/native.rs:587 | import_report_details | Library All files |
| CAP-237 | Reference availability: Attached, Pending Save, Must be current | wui/setup.rs:335-353 | resolve | Library All files |
| CAP-238 | Reference removed, pending Save notice | wui/setup.rs:414-424 | render | Library All files |
| CAP-239 | Source row with Open and Copy reference | threads_writing.rs:1098-1148 | render RecordData::Source; open_with_system / open_url | Library All files (§6.4 evidence) |

## Projects

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-240 | Projects page with Active and Trash scope | threads_projects.rs:229-257; threads_shell.rs:507, 864 | projects_tile; projects_trash | Project workspace |
| CAP-241 | New project (dialog; Enter creates; Cancel) | threads_projects.rs:253-265, 517-627 | project_name_dialog → apply_project(Create) | Project workspace |
| CAP-242 | Project rows and empty states | threads_projects.rs:281-320 | render | Project workspace |
| CAP-243 | Project header: title, BRN knowledge status, instructions status | threads_projects.rs:352-420 | render | Project workspace (§6.3) |
| CAP-244 | Project instructions and settings dialog (title, instructions, Save ⌘↵) | threads_projects.rs:628-756 | project_settings_dialog → ProjectCommand::Configure | Project workspace (§6.3 settings) |
| CAP-245 | Use BRN knowledge switch | threads_projects.rs:716-734 | ProjectCommand::Configure | Project workspace (§5, §6.3) |
| CAP-246 | New chat in project | threads_projects.rs:398-410, 517-627 | ProjectCommand::CreateChat | Project workspace |
| CAP-247 | New document in project | threads_projects.rs:181-190, 517-627 | ProjectCommand::CreateDocument | Project workspace |
| CAP-248 | Import file into project | threads_projects.rs:191-209 | choose_import(FullNote) with attaching_chat | Project workspace (§6.3) |
| CAP-249 | Link document dialog (unarchived, non-superseded notes) | threads_projects.rs:210-217, 841-930 | ProjectCommand::LinkDocument | Project workspace |
| CAP-250 | Project documents: open, Keep in BRN, Unlink | threads_projects.rs:93-126, 436-485 | open_project_note; keep_in_brn; UnlinkDocument | Project workspace (§6.3) |
| CAP-251 | Project chats list (Archived label) | threads_projects.rs:497-515 | open_thread | Project workspace |
| CAP-252 | Move chat to a project or Standalone (history stays) | threads_projects.rs:763-832; threads_tiles.rs:122-131 | move_chat_dialog → ProjectCommand::MoveChat | NEEDS DECISION (Sidebar or Project workspace) |
| CAP-253 | Move project to Trash (preview of moved items; per-task choice) | threads_projects.rs:930-1041; other_items_line threads_projects.rs:39-79 | project_delete_dialog → ProjectCommand::Delete | Settings Data or project menu (§6.3, §4.2) |
| CAP-254 | Trash scope: Restore project (cancelled tasks stay cancelled) | threads_projects.rs:23, 112-150, 367-390 | ProjectCommand::Restore | Settings Data or project menu (§6.3) |
| CAP-255 | Projects ⋯ menu: Instructions, New document, Import, Link, Move to Trash | threads_projects.rs:162-226 | projects_menu_section | Project workspace |
| CAP-256 | Project edit during a running reply keeps partial progress; stops the run when the receipt ends it | threads_projects.rs:117-143 | apply_project | Project workspace (§6.3 run fencing) |
| CAP-257 | Shared document shows its host project when open | threads_ui.rs:409, 1243-1248; threads_projects.rs:93-111 | note_project | Project workspace (§6.3) |
| CAP-258 | Chats in archived projects hidden from chat lists | threads_lists.rs:434-438 | project_for_chat | Sidebar recents (§4.1) |

## Inbox

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-259 | Document inbox explanation and limits (manual scan; no subfolders; 256 entries; 16 MiB) | threads_inbox.rs:5-23 | inbox_settings → workspace.inbox() | Inbox Imports |
| CAP-260 | Choose General inbox folder and confirm destination | threads_inbox.rs:31-38, 60-132 | choose_inbox_folder → choose_inbox(General) | Inbox Imports |
| CAP-261 | Scan inbox (manual; background) | threads_inbox.rs:40-45, 134-197 | start_inbox_scan | Inbox Imports |
| CAP-262 | Inbox entries: name, status, detail; no-folder message | threads_inbox.rs:49-56 | workspace.inbox() | Inbox Imports (status shows raw enum name, threads_inbox.rs:51) |
| CAP-263 | Stop inbox scan | threads_ui.rs:2680-2690 | cancel → worker.cancel | Inbox Imports (button location unverified) |
| CAP-264 | Conversion progress and stop ("Reading the full document…", "Import stopped") | threads_ui.rs:2965, 3036-3042, 3384-3387 | convert_file / Converted arm | Inbox Imports |
| CAP-265 | Import pauses queue when conversion does not complete | threads_ui.rs:3047-3050 | pause_queue | Inbox Imports |
| CAP-266 | Useful-information intake creates a chat with the source | threads_ui.rs:3116-3183 | useful_intake → source_intake_chat, start_run | NEEDS DECISION (Standalone Chat or Inbox Imports) |
| CAP-267 | Reference conversion stopped; no Note attached | threads_ui.rs:3042 | Converted arm | Primary document canvas |

## Tasks

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-268 | Task buckets: Suggested, Overdue, Today, Upcoming, Check date, Waiting, Open, Done, Cancelled | threads_lists.rs:9-95; threads_tiles.rs:1254-1286 | follow_up_group | Inbox Tasks |
| CAP-269 | Tasks empty state and "Follow-ups as of" | threads_tiles.rs:1237-1253 | render | Inbox Tasks |
| CAP-270 | Done and Cancelled collapsed (session) | threads_tiles.rs:1207-1234, 1261; threads_ui.rs:417-418 | open_sections | Inbox Tasks |
| CAP-271 | Task row: description and facts (state, due, owner, waiting on, follow-up, evidence) | threads_tiles.rs:1054-1093, 1190-1200 | RecordData::Action | Inbox Tasks and contextual task cards |
| CAP-272 | Main task action: Accept task or Confirm done | threads_tiles.rs:1094-1115; threads_lists.rs:381-419 | change(step.reason()) | Inbox Tasks |
| CAP-273 | Task ⋯ actions: Edit details, Waiting, Mark open, Confirm done, Cancel task (no confirmation) | threads_tiles.rs:1116-1158 | change | Inbox Tasks |
| CAP-274 | Edit details dialog "Task follow-up details" (owner, waiting on, date) | threads_tiles.rs:942-1049 | workspace.owner_change | Inbox Tasks |
| CAP-275 | Confirm done writes evidence "Owner confirmed completion in BRN." | threads_tiles.rs:1066-1073 | next() | Inbox Tasks |
| CAP-276 | Refresh pill opens Tasks | threads_tiles.rs:1431-1433 | show_tile(Actions) | Inbox Tasks |
| CAP-277 | Task rows inside linked work | threads_writing.rs:1092; threads_tiles.rs:1054 | action_row | Inbox Tasks and contextual task cards |

## Search

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-278 | Note search with previous and next paging | threads_search.rs:127-191, 238-250, 336-400 | run_search → request_page | Library Articles |
| CAP-279 | Note-hit status line ("N matches · first–last"; "with meaning") | threads_search.rs:427-447 | search_results | Library Articles |
| CAP-280 | Note hit opens at match; facts (heading, passages, imported, protected, inbound links) | threads_search.rs:474-503 | open_note + reveal | Library Articles |
| CAP-281 | Archived label on hits | threads_search.rs:72-80, 494-498 | archived_chip | Library Articles and all chats |
| CAP-282 | Chat-message matches ("In chats") with paging | threads_search.rs:551-635, 252-269 | Lanes::Chats | Searchable all-chats view |
| CAP-283 | Open chat message at its match | threads_search.rs:272-281, 598-617 | open_chat_message | Searchable all-chats view |
| CAP-284 | Empty note search state | threads_search.rs:525-537 | render | Library Articles |
| CAP-285 | Search loading and failure ("Preparing search…", "Searching…", "Search failed: …") | threads_search.rs:193-236, 418-470, 572-576 | search_answered | Library Articles |
| CAP-286 | Search index preparation at startup (derived index) | threads_search.rs:100-125 | prepare | Library Articles |
| CAP-287 | ⌘K launcher "In chats" matches (label-only fallback) | threads_search.rs:285-328; threads_shell.rs:1381-1394 | launcher_search_changed | NEEDS DECISION (launcher) |

## Settings

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-288 | Settings sections: General, Models, Delegation, Documents, Data, Skills, MCP | threads_ui.rs:208-216; threads_tiles.rs:2000-2032 | settings_section (session) | Settings sections (§4.2) |
| CAP-289 | Provider choice: Codex or ChatGPT, or GitHub Copilot | threads_tiles.rs:1694-1710 | provider_form.provider → configure | Settings Models |
| CAP-290 | Explicit model field | threads_tiles.rs:1711-1714; threads_ui.rs:438, 513-517 | configure | Settings Models |
| CAP-291 | Reasoning effort: Low, Medium, High | threads_tiles.rs:1715-1724 | effort → configure | Settings Models |
| CAP-292 | Codex authentication file (optional) | threads_tiles.rs:1725-1730; threads_ui.rs:439, 518-522 | auth_file | Settings Models |
| CAP-293 | Copilot credentials folder (optional; default ~/.config/brn/copilot, mode 0700) | threads_tiles.rs:1731-1736; threads_ui.rs:255-267, 440, 523-527 | credentials_dir | Settings Models |
| CAP-294 | Helper sub-agent: Off, Codex or Copilot; helper model and effort | threads_tiles.rs:1762-1820; threads_ui.rs:441, 528-537 | helper settings | Settings Models |
| CAP-295 | Connect GitHub Copilot (sign-in code copied; URL opened) | threads_tiles.rs:1740-1757; threads_ui.rs:2808-2826, 3577-3693 | connect_copilot → copilot_worker | Settings Models |
| CAP-296 | Save settings (refused during an active run; restored on failure) | threads_tiles.rs:1821-1850; threads_ui.rs:2311-2315, 3694-3734 | configure | Settings Models |
| CAP-297 | Settings write rules: empty provider, model or effort rejected | app/src/settings.rs:80-88; core/src/store.rs:872-880 | configure_workspace | Settings Models |
| CAP-298 | Delegation choice: Maintain ordinary notes, or Review changes first | threads_tiles.rs:1508-1532, 2061; threads_ui.rs:3735-3742 | choose_delegation | Settings Delegation |
| CAP-299 | Delegation status copy | threads_state.rs:40-51 | delegation_status | Settings Delegation |
| CAP-300 | Maintenance Running or Paused switch | threads_tiles.rs:1623-1655 | workspace.pause | Settings Delegation |
| CAP-301 | First-use delegation card (Home or chat) until a choice is made | threads_tiles.rs:1533-1591; threads_ui.rs:1043-1057 | choose_delegation | NEEDS DECISION (§4.3: shown when relevant) |
| CAP-302 | Document converter: Xberg or Xberg + BRN layer | threads_documents.rs:27-64 | choose_document_conversion | Settings Documents |
| CAP-303 | AI conversion: Off (default) or On | threads_documents.rs:65-71 | choose_document_conversion | Settings Documents |
| CAP-304 | Conversion report summary; applies to future imports only | threads_documents.rs:14-24, 48, 57 | report_outcome | Settings Documents |

## Data

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-305 | Create full backup in the saved folder | threads_tiles.rs:1885-1892; threads_ui.rs:3392-3580 | choose_output(backup) → backup_to_saved_directory | Settings Data |
| CAP-306 | Choose or change backup folder | threads_tiles.rs:1858-1900; threads_ui.rs:3457-3516 | choose_backup_folder → choose_backup_directory | Settings Data |
| CAP-307 | Backup and export status lines and result copy | threads_tiles.rs:1902, 1918; threads_ui.rs:2921-2922, 3541-3547 | backup_status / export_status | Settings Data |
| CAP-308 | Export Markdown snapshot (Settings Data) | threads_tiles.rs:1903-1918; threads_ui.rs:3416-3456, 3517-3580 | export_now | Settings Data |
| CAP-309 | Export Markdown snapshot from the note menu | wui/writing.rs:253; threads_ui.rs:3392-3412, 4599 | choose_output(false) | Document ⋯ menu (§4.2 Export) |
| CAP-310 | Restore: choose backup file | threads_restore.rs:23-35, 94-140 | restore_source_chosen | Settings Data |
| CAP-311 | Restore: choose recovery folder (new folder outside the workspace) | threads_restore.rs:164-218, 347-365 | restore_parent_chosen / recovery_destination | Settings Data |
| CAP-312 | Restore: validate in a private copy; review; "Restore this copy" | threads_restore.rs:201-264 | Store::prepare_restore → publish | Settings Data |
| CAP-313 | Restore: cancel validation and cancel restore | threads_restore.rs:275-297 | cancel_restore / discard_restore | Settings Data |
| CAP-314 | Restore: exclusions shown (skill files and original documents are not in the backup) | threads_restore.rs:61 | render | Settings Data |
| CAP-315 | Open recovered workspace in a new window ("Saved queues stay paused until Resume") | threads_restore.rs:298-324 | open_restored_workspace | Settings Data |
| CAP-316 | Restore status and failure states | threads_restore.rs:219-244, 265-274 | restore_prepared / restore_finished | Settings Data |

## Skills

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-317 | Instruction skills panel with scope (general chats or a project) | threads_skills.rs:256-378, 44-107 | skills_settings / refresh_skills | Settings Skills |
| CAP-318 | Install workspace-local instruction-only package | threads_skills.rs:184-207, 172-182 | install_skill | Settings Skills |
| CAP-319 | Refresh packages (Settings and composer) | threads_skills.rs:109-112, 246-251, 269-270 | force_refresh_skills | Settings Skills |
| CAP-320 | Skill rows: origin, enabled or disabled, approved digest, stale and unavailable states | threads_skills.rs:288-309 | refresh_skills | Settings Skills |
| CAP-321 | Inspect skill (preview up to 8192 bytes) | threads_skills.rs:148-158, 312, 366-376 | skill_action Inspect | Settings Skills |
| CAP-322 | Enable and Disable skill | threads_skills.rs:136-146, 313-321 | set_skill_enabled | Settings Skills |
| CAP-323 | Enable current version (stale digest) | threads_skills.rs:323-328 | skill_action AdoptCurrent | Settings Skills |
| CAP-324 | Remove workspace-wide package (no confirmation) | threads_skills.rs:147, 330-336 | remove_skill | Settings Skills |
| CAP-325 | Composer skill selector (None, or enabled skills) | threads_skills.rs:209-243 | skill_action Select / Clear | Primary conversation composer |
| CAP-326 | Scope-change guard ("Skill scope changed…") | threads_skills.rs:122-134 | skill_action | Settings Skills |
| CAP-327 | Loaded skill details in per-conversation activity | threads_skills.rs:379-388, 411-443 | loaded_skill_details | Per-conversation activity disclosure |
| CAP-328 | Policy change warning: interrupts active runs | threads_skills.rs:268 | render | Settings Skills |
| CAP-329 | Non-macOS/Linux platform notice | threads_skills.rs:405-408 | render | Settings Skills |

## MCP

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-330 | MCP settings: anonymous local client; endpoint (default http://127.0.0.1:8077/mcp) | threads_mcp.rs:44-48, 444-457 | McpView.endpoint (view state) | Settings MCP |
| CAP-331 | Discover local metadata (no tool call) | threads_mcp.rs:145-224, 458-463 | discover_mcp → Workspace::discover_mcp | Settings MCP |
| CAP-332 | Approve known read-only tool for this scope | threads_mcp.rs:226-289, 501-508 | approve_mcp_tool → configure_mcp | Settings MCP |
| CAP-333 | MCP status line: enabled, scope authorized, transient grant | threads_mcp.rs:113-144, 511-515 | refresh_mcp / mcp_status | Settings MCP |
| CAP-334 | Disable or disconnect MCP (fenced) | threads_mcp.rs:290-300, 464-468 | disable_mcp | Settings MCP |
| CAP-335 | MCP discovery and policy errors | threads_mcp.rs:469-487 | McpViewError | Settings MCP |
| CAP-336 | Review exact input for a tool (idle chat, approved tool) | threads_mcp.rs:301-337, 518-526 | begin_mcp_input | NEEDS DECISION (per-conversation; §4.2 has Settings MCP only) |
| CAP-337 | Authorize exact input once for the current request | threads_mcp.rs:338-421, 528-531 | review_mcp_input | NEEDS DECISION |
| CAP-338 | Send with reviewed input; queued sends refused | threads_mcp.rs:422-443 | take_mcp_review | NEEDS DECISION |
| CAP-339 | MCP authorization refusal before a run | threads_ui.rs:1949-1956, 1981-1990 | run authorization check | NEEDS DECISION |
| CAP-340 | Per-chat MCP status detail | threads_mcp.rs:553-580 | mcp_detail | Per-conversation activity disclosure |

## Web

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-341 | Search the web dialog (ephemeral; only the query is sent) | threads_web_search.rs:137-211; threads_writing.rs:44-46 ("Search web…") | open_web_search | NEEDS DECISION (no §4.2 entry) |
| CAP-342 | Run web search, Search and Stop search | threads_web_search.rs:51-99, 196-207 | start_web_search / stop_web_search | NEEDS DECISION |
| CAP-343 | Web results: provider, model, answer, source links (untrusted evidence) | threads_web_search.rs:100-164 | poll_web_search | NEEDS DECISION |
| CAP-344 | Web search failure | threads_web_search.rs:133-155 | field_error | NEEDS DECISION |
| CAP-345 | Settings change during a search stops it | threads_web_search.rs:38-50, 100-116 | reconcile_web_settings | Settings Models |
| CAP-346 | Web search uses saved provider, model, effort and credentials | threads_web_search.rs:21-36 | selection compare | Settings Models |

## Appearance

| ID | Capability | Entry point | Service owner | New-UX destination |
|---|---|---|---|---|
| CAP-347 | Theme Sammal (light; default) | threads_tiles.rs:1931-1983 (settings-theme-sammal at 1956); wui/theme.rs:33 | choose_theme → theme::apply | Settings General |
| CAP-348 | Theme Läänemeri (light) | threads_tiles.rs:1931-1983; wui/theme.rs:34 | choose_theme → theme::apply | Settings General |
| CAP-349 | Theme Raba (dark) | threads_tiles.rs:1931-1983; wui/theme.rs:35 | choose_theme → theme::apply | Settings General |
| CAP-350 | Theme Virmalised (dark) | threads_tiles.rs:1931-1983; wui/theme.rs:36 | choose_theme → theme::apply | Settings General |
| CAP-351 | Theme Paber (light) | threads_tiles.rs:1931-1983; wui/theme.rs:37 | choose_theme → theme::apply | Settings General |
| CAP-352 | Theme command in ⌘K ("Theme: {label} · {description}") | threads_lists.rs:803, 829; threads_ui.rs:2651 | Command::Theme | Settings General |
| CAP-353 | Theme persistence: settings.theme in SQLite; restored at startup | core/src/types.rs:325; core/src/store.rs:922-925; app/src/settings.rs:169-175; threads_ui.rs:492-493, 2244-2258 | choose_theme → store.choose_theme | Settings General |

## Architecture notes

Root view
- `ThreadsUi` (threads_ui.rs:373) is the single host. It owns `writing: WritingPanel`, `workspace: Workspace` (brn-threads-app), `data_dir`, the in-memory `layout`, the `theme` and session state (notices, launcher, open sections, drafts, the active worker).
- Startup (threads_ui.rs:478-500) recovers the queue, applies the saved theme (492-493) and creates the writing panel (500).
- threads_shell.rs builds the chrome: status bar (Work, Queue and Save pills), launcher button, nav pills, the layout menu and the tile row. Key contexts are `BrnLauncher` (threads_shell.rs:1517) and `BrnWorkspace` (1659). Action listeners sit at threads_shell.rs:1679-1761.
- Tile dispatch (threads_shell.rs:488-509) maps each `TileKind` to a body: threads_tiles.rs (Threads, AllThreads, Notes, Overview, Actions, History, Settings), `chat_tile`, `note_tile` and `compare_tile` (threads_writing.rs and wui), `connections_tile` (threads_connections.rs) and `projects_tile` (threads_projects.rs).

Tiles and layout
- `TileKind` (threads_layout.rs:4-18) has 13 variants. `Layout` holds visible tiles, weights, areas (Navigation, Main, Companion), focus, the maximized tile, the preset and `settings_open`.
- `show_tile` (threads_ui.rs:1019-1028), `apply_layout` (1029-1042) and `relayout` (1032-1038) mutate it. The default is the Write preset with Chat focused (threads_ui.rs:669).
- Layout is memory only. No save or load was found in the desktop crate. Settings section and open disclosure sections are session state too.

Theme and colour
- The enum, labels, palettes and `pal()` live in brn-writing-ui: `ThemeId` (wui/theme.rs:6-13), labels (31-37), `palette()` (55-116), tokens (131-207), `pal()` (242), `apply()` (248).
- The desktop includes it through `#[path = "threads_theme.rs"] mod theme;` (threads_ui.rs:82-83). threads_theme.rs only re-exports the writing crate module.
- Five fixed themes. No System or automatic option exists; no OS appearance query was found.
- Persistence: `choose_theme` (threads_ui.rs:2244) → `theme::apply` → `persist_theme` (2258) → `workspace.choose_theme` (app/src/settings.rs:169-175) → store (core/src/store.rs:922-925) into `settings.theme` (core/src/types.rs:325). Startup reads it at threads_ui.rs:492-493. Saves wait while a run is active (threads_ui.rs:2262-2282).

Writing workbench embedding
- `wui/native.rs` is the embed surface. `WritingPanel` is created once (threads_ui.rs:500), `WritingRequest` (native.rs:316-396) is an immediate, non-durable request enum, and `WritingView` is built each render (native.rs:407).
- The host builds the view at threads_ui.rs:4435-4469 from records, history, candidates, buffers, threads, reviews, requests, the worker and progress. Requests are dispatched at threads_ui.rs:4471-4672.
- `note.rs`, `review.rs`, `references.rs` and `setup.rs` are compiled through `#[path]` declarations in native.rs:16-24, not lib.rs. Their lifecycles stay in the writing crate; saves and approvals go through app/core services.
- `writing.rs` (about 2,900 lines) holds the note and review panels and menus. `workbench.rs` holds rewrite progress and proposal cards. `document.rs` holds the note toolbar, Outline, formatting and highlights.

Persistence
- Records, history and candidates are SQLite, owned by the brn-threads-app `Workspace` (threads_ui.rs:822-825). main.rs:122 asserts `threads.sqlite3`.
- Settings are written through `configure` (app/src/settings.rs:80-88; core/src/store.rs:872-880). Theme is written as above.
- Chat drafts use `store.save_chat_draft` with a generation counter (threads_chat.rs:222-239). Recovery buffers live in the store (threads_ui.rs:825).
- Session only, lost on restart: notices (threads_notice.rs:18-23), launcher state, open disclosure sections, `queue_expanded`, `notes_letter`, `thread_filter`, comment and feedback drafts, export and backup status (threads_ui.rs:392-466).

Presentation-only versus state or service glue
- Presentation: threads_layout.rs, threads_shell.rs, threads_tiles.rs, threads_components.rs, threads_lists.rs (list shaping over records), threads_state.rs (wording and pure rules; its tests are at lines 594 onward), threads_notice.rs (session notice channel), threads_run_activity.rs (bounded labels). In the writing crate: components.rs, layout.rs, markdown.rs, diff.rs, theme.rs.
- Glue or owners (they call services or change worker and dispatch state): threads_ui.rs (host, dispatch, worker finish, import, export, backup, configure, exit), threads_worker.rs, threads_chat.rs (send, queue, stop, acknowledge), threads_projects.rs (ProjectCommand), threads_search.rs and threads_search_state.rs (search handle), threads_connections.rs (graph and knowledge triage), threads_inbox.rs, threads_documents.rs, threads_capture.rs, threads_restore.rs, threads_skills.rs, threads_mcp.rs, threads_web_search.rs, threads_context_actions.rs.
- Mixed: threads_writing.rs (rendering plus queue, steer, attach and edit calls), writing.rs (large render with session state), note.rs and review.rs (lifecycle).

Not found in the checkout
- A System or automatic appearance option (UX-03 requires System, Light and Dark).
- A document-level Trash. Only project Trash exists (threads_projects.rs, Trash scope and Restore project).
- Queue move-down or reorder, and a cancel-edit control for a queued message.
- A "Saving" label in the writing UI. Save outcome copy is in threads_state.rs:550-565.
- A Shift-Enter key handler (tooltip text only, threads_writing.rs:980-984).
- A retry-capture caller (threads_capture.rs:90).
- A skills empty state and a dedicated permission state beyond disabled-reason text.

Limits of this inventory
- Slice reviews were read-only. Service bodies in app/ and core/ were cited by name, not read in full.
- Labels for launcher commands and Home sections come from slice reads, not from my own read.
