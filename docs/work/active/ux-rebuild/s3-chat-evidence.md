# S3 conversation — evidence

Branch `work/ux-s3-chat`, on `origin/work/ux-rebuild-20261105` at `7dc1c794` (S2 shell merged). Status: implemented and verified (automated); native-observed on synthetic data in Light and Dark at the sizes below. Running and streaming states were not observed natively (no paid provider calls). Not owner-accepted, not merged.

## Commits

| Commit | Content |
|---|---|
| `dde22dc3` | Conversation column, one composer (`threads_composer.rs`), context chip, + menu with skills, activity disclosure, Linked and ⋯ header, start surface, permission card |
| `1faa030e` | Enter submits before the editor inserts a newline; queue Move down; tests for keys, IME, queue, scope, Keep in BRN, permission card |
| `f769ba91` | Search the web launcher command; test that streaming keeps a scrolled-up reader in place |
| `ecd741ee` | Shortcut help names Enter, Shift-Enter and the composer Stop |
| `7581900b` | The icon-only context chip beside a document keeps its size and drops the caret |
| `4ae2e815` | A refused send or queue keeps the text and names the retry (CHAT-02) |
| `c5f679a5` | Jump to latest has an accessible name; test it returns to the tail (CHAT-04) |
| `4b2ba6f0` | Queue rows keep Edit/Remove aligned with fixed-width reorder slots (native defect) |
| `28c5d53d` | Attachment names stay readable in a narrow assistant; coverage truncates first (native defect) |
| this file | Evidence |

## What moved where

Conversation body (`threads_chat.rs`, `threads_writing.rs`):

- Readable column at `metrics::CONVERSATION_WIDTH`, centred in the primary view and full width in the assistant. Assistant replies are flat prose through the existing Markdown renderer; user messages sit in a soft bubble (`RADIUS_BUBBLE`). CAP-089's "centred when maximized" outcome is now always on.
- Copy reply, Save as decision… with an editable preview, Edit note and Capture outcomes… keep their services (CAP-076, CAP-077, CAP-078). Partial and interrupted labels and the writing indicator are unchanged (CAP-086).
- Streaming keeps a scrolled-up reader in place and shows **Jump to latest**, a labelled control.
- Recovery card (Continue, Resume queue, archived reason) is unchanged in the column (CAP-116, CAP-117, CAP-119, CAP-121).

Header, through the S2 header action slot:

| Item | Content | CAP |
|---|---|---|
| Title | Conversation title | — |
| Project pill | Where relevant | — |
| Queue state | e.g. "Queue paused · N waiting" | CAP-107 |
| **Activity** | Last work status, run line, Stop and Quiet. Routine steps, loaded skills, search status and MCP detail move in here. | CAP-047, CAP-084, CAP-085, CAP-102 to CAP-105, CAP-114, CAP-115, CAP-327, CAP-340 |
| **Linked · N** | Opens the existing linked-work content (notes, sources, tasks used) as the supporting pane | CAP-090 |
| **⋯** | Move to project, Archive/Unarchive, Capture useful outcomes, Open note, New chat for this note, the note's chat switcher | CAP-057, CAP-058, CAP-060, CAP-061, CAP-066 |

The old chat status row (`worker-status`, the `resolve-thread` button and the Quiet toggle) is gone from the chat. Its content lives in Activity and ⋯. When the open chat is running, the shell header skips its duplicate work status.

Composer (`threads_composer.rs`, split from `threads_writing.rs`). It is one rounded composer (`RADIUS_COMPOSER`) that grows from 2 to 8 rows and then scrolls internally.

- **Context chip** (§5):
  - Labels: "Saved library", "Project NAME", "This chat only" or the document title. It becomes icon-only, with no caret, in a narrow assistant.
  - Its menu lists what BRN can use: the saved library including protected notes; the files attached to this chat, private until Keep in BRN; what is not used (proposals, scratch, older versions, archived notes); the maintenance setting; and Delegation settings….
  - The wording comes from the same scope retrieval uses, so the chip never claims a narrower scope.
- **+ menu**:
  - Attach document…, Add useful information…, Import full note… (CAP-080, CAP-082, CAP-083).
  - Search the web… opens the existing dialog (CAP-341 to CAP-344; Enter/Escape keys CAP-051 unchanged). Also available as a launcher command.
  - A Skill section (enabled skills, ✓ No skill, Refresh skills) replaces the always-visible skill pill row (CAP-325, CAP-319).
- **Model and effort** controls are populated from the configured capabilities, with their busy reasons (CAP-069, CAP-072, CAP-073). The model collapses to "gpt… · medium" in a narrow assistant.
- **One primary control** by state: Send, Queue, Save queued or Stop (CAP-088). Steer stays an explicit action (CAP-087).
- **Drafts** stay per destination, and the composer clears only after the service accepts (CAP-053 to CAP-055). A refusal keeps the text and names the retry: "…Your message is still in the composer; send it again when ready."

Queue panel (shown only when nonempty):

- Summary and paused card (CAP-093, CAP-096) and per-item state (CAP-097).
- Edit and Remove (CAP-098, CAP-099). Move up, plus the new **Move down** (CAP-100's reorder gap is closed). All controls are keyboard-reachable buttons; nothing is drag-only.
- Every row has the same fixed reorder slots, so Edit and Remove line up even when an arrow is absent.
- In a short window it collapses to one row with a disclosure (CAP-094). Resume is explicit (CAP-095), and auto-dispatch is unchanged (CAP-101).

Attachments (§6.8): rows show the name, coverage, Inspect and **Keep in BRN…** (CAP-081). Keep in BRN asks first, names "Destination: Saved library", and stays undoable in Recent changes. In a narrow assistant the name keeps at least 140 pt and the coverage note wraps or truncates first.

Permissions (§6.11, `threads_mcp.rs`, conversation part only):

- Exact-input review is an inline warning card above the composer: tool, destination, chat scope and exact input, with explicit **Deny** and **Allow once** (CAP-336 to CAP-338).
- An allowed input binds once to the message in the composer; it is never saved and queued sends never use it.
- Deny sends nothing and says so.

New-chat start surface (§4.3, absorbing Home's first-use content):

- Greeting "What are we working on?" with one sentence, then the composer and three starting actions: Attach a document, New document, Search the web.
- While unchosen, the delegation card "How should BRN update your notes?" appears (CAP-074, CAP-008 outcome).
- A paused queue stays visible here.

## Keys

| Key | Behaviour |
|---|---|
| Enter | Sends when idle and queues while running. It submits before the editor inserts a newline (CAP-048). |
| Shift-Enter | Inserts a newline; the inventory found no handler, so this is now implemented and tested |
| IME composition | Never submits |
| ⌘↵ | Kept; it queues or sends exactly once |
| ⌘. | Stop; kept, and shown in the Activity disclosure and the composer Stop |
| Escape | Closes the chip, +, model and ⋯ menus; it never stops work or discards a draft |

Shortcut help gained Send / new line and Stop rows.

## Shared-file hooks (lead-owned files, minimal edits)

- `threads_shell.rs`:
  - `one_line` is now `pub(super)`.
  - Web search command icon.
  - Shortcut-help rows for Send / new line and Stop.
  - The header skips `work_status` when the open chat is running, so the status is not shown twice.
- `threads_lists.rs`: `Command::WebSearch` "Search the web…" in the launcher.
- `threads_ui.rs`: send/steer/enqueue failure wording and composer creation.
- `threads_skills.rs`: removed the composer `skill_selector` pill row; its actions live in the + menu.

No shell model, sidebar or layout change.

## Tests

None were removed. Adapted to the new presentation without weakening the assertions:

| Test file | Adaptation |
|---|---|
| `threads_steering_tests.rs` | The `stop-work` id is now `run-stop`. Quiet and run activity are asserted inside the `chat-activity` disclosure. `worker-status` must be absent from the chat. `resolve-thread` is now the `chat-menu` Archive item. |
| `threads_omakase_tests.rs` | Archive is reached through the ⋯ menu |
| Shell keys test | Expects the added help rows |

New tests:

| Test | Covers |
|---|---|
| `shift_enter_adds_a_line_enter_queues_once_and_ime_composition_never_submits` | Shift-Enter, Enter and IME |
| `command_enter_in_the_composer_queues_exactly_once` | ⌘↵ |
| `a_refused_send_keeps_the_text_and_names_the_retry` | CHAT-02 |
| `queue_items_move_down_and_up_and_editing_keeps_the_composer_draft` | Queue reorder; also asserts Edit is aligned across rows |
| `the_context_chip_names_the_real_scope` | Context chip |
| `keep_in_brn_names_the_destination_before_it_promotes` | Keep in BRN |
| `a_narrow_assistant_keeps_the_attachment_name_readable` | Assistant widths 300–380. Fails on the old layout: the name shrank to 6 pt at 340. |
| `the_new_chat_start_has_a_greeting_the_composer_three_actions_and_the_delegation_choice` | Start surface |
| `the_activity_disclosure_names_the_last_recorded_run` | Activity disclosure |
| `streaming_keeps_the_reading_position_when_scrolled_up` | Streaming position and the Jump to latest label |
| `the_permission_card_names_the_request_and_allow_or_deny_is_explicit` (`threads_mcp_tests.rs`) | Permission card |

## Commands and results

Environment: Rust 1.98.1, `--locked --jobs 4`, fresh `TMPDIR` under `/private/tmp`, and document helpers built per `scripts/verify-desktop-shell.sh`. Results at `28c5d53d`:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | ok |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | ok |
| `cargo clippy -p brn-writing-ui --features native-ui,native-test-support --all-targets -- -D warnings` | ok |
| `cargo test -p brn-writing-ui --features native-ui,native-test-support` | 48 + 12 passed, 0 failed |
| `cargo test -p brn-desktop --features native-ui,native-test-support` | 399 passed, 0 failed, 1 ignored |
| `cargo build -p brn-desktop --features native-ui` | ok |

## Native observation

Setup:

- The app copy `/private/tmp/brn-s3-app/brn-desktop-s3` ran with its own `--data-dir` under `/private/tmp`.
- Synthetic chats: Atlas kickoff (project Atlas), Budget review, Workshop agenda, Release planning (an answer with a table and code, a linked protected note and the attachment Venue contract). Synthetic note: Launch checklist. Four messages queued and paused.
- Theme was switched through Settings → Appearance; the OS appearance was not changed. Only this window was captured.

Captures are in `/Users/evokessler/repos/brn-ux-rebuild-artifacts/shots/s3/`. "Final" means the build at `28c5d53d`; Dark VE-01 and the Dark menus come from the `4b2ba6f0` build, whose 1280 layout is unchanged.

| File | Shows |
|---|---|
| `ve01-light-1280-conversation.png`, `ve01-dark-1280-conversation.png` | VE-01 at 1280×900: answer with references, paused queue of four with aligned Edit/Remove/↑/↓, attachment row, composer with "Saved library", model, effort and Queue. Header shows Queue paused · 4 waiting, Activity, Linked · 1 and ⋯. |
| `light-1280-context-chip-menu.png`, `dark-1280-context-chip-menu.png` | "BRN can use" scope, including the attached file and Not used |
| `light-1280-plus-menu.png`, `dark-1280-plus-menu.png` | Attach, useful information, import, Search the web…, Skill section |
| `light-1280-linked-disclosure.png`, `dark-1280-linked-disclosure.png` | Linked work: Venue contract, Protected, v1, Open note |
| `light-1280-new-chat-start.png`, `dark-1280-new-chat-start.png` | Start surface with the paused queue, composer, three actions and the delegation card |
| `light-1280-document-with-assistant.png` | Note with the docked assistant at 1280 |
| `light-900-assistant-sheet.png`, `dark-900-assistant-sheet.png` | 900×600 sheet over Launch checklist: collapsed "Paused · 4 wai…" with Resume queue, full attachment name, icon-only chip, "gpt… · medium", Queue |
| `light-1280-enter-queued-paused.png`, `light-1280-shift-enter-two-lines.png`, `light-1280-conversation-references.png`, `light-900-new-chat-start.png`, `light-900-assistant-sheet-before-chip-fix.png` | Previous helper, earlier build (before `7581900b`/`4b2ba6f0`): Enter queued while paused, Shift-Enter giving two lines, and the chip defect before its fix |

Defects found natively and fixed, each with a test that fails without the fix:

1. Queue rows: Edit/Remove shifted when a row lacked ↑ or ↓ (`4b2ba6f0`).
2. In the 900×600 sheet the attachment name collapsed to "A…" beside "Coverage not independently checked" (`28c5d53d`).

## Acceptance cases exercised

| AC | Native (synthetic data) | Automated only |
|---|---|---|
| AC-02 chat and draft isolation | Composer bound to each chat while switching between chats | Per-chat drafts, clear-once-after-accept, refused-send retry (`quiet_display_keeps_stream_and_stop_and_saved_drafts_survive_reopening`, `queue_items_move_down_and_up_and_editing_keeps_the_composer_draft`, `a_refused_send_keeps_the_text_and_names_the_retry`, context-action draft tests) |
| AC-03 queued messages | Paused queue in the chat, the start surface and the sheet. Edit/Remove/↑/↓ visible and aligned. Enter queued while paused (previous helper's capture). | Edit, reorder both ways, remove, dispatch order (`queue_items_move_down_and_up…`, `enter_queues_while_working_and_stop_pauses_saved_queue_across_late_finish`, `editing_a_queued_message_does_not_redirect_the_next_chat_send`) |
| AC-04 Stop, Resume, Steer | Resume queue control present | Stop pauses the queue, explicit Resume, Steer persistence and partial answers (`stop_and_send_click_is_durable…`, `archive_and_steer_preserve_partial_answers…`, `failed_steering_persistence_retains_text_worker_and_controls`) |
| AC-05 provider and model | Model and effort controls show the configured "ChatGPT · gpt-6.1-sol" and "medium"; no run was started | `the_model_control_shows_display_names`, busy reasons |
| AC-28 failures | — | Failed and interrupted finish text, refused stop/undo banners, MCP refusals (`a_failed_reply_*`, `a_refused_*`, `nf1_*`) |
| AC-31 keyboard and focus | Escape closes the chip and + menus. Shift-Enter newline (previous helper). Controls reachable by accessible name. | Enter/Shift-Enter/IME/⌘↵ tests, Jump to latest label, queue controls as buttons |
| VE-01 | Light and Dark at 1280×900 | — |

## Gaps

- **Running and streaming states were not observed natively.** Doing so needs a provider run, and paid calls were out of scope. Covered only by automated tests: the run line, Stop, Steer, streaming with Jump to latest, and the permission card with Allow/Deny.
- AC-04 and AC-28 were not exercised natively.
- Focus contention: another slice's app was taking focus during capture. The guarded driver stopped and retried, and every capture was checked by eye.
- After an accessibility "press" on the note header's Assistant toggle, its tooltip ("Ask about this document beside it") stayed visible over the sheet header until another control was used. The toggle belongs to the S2 shell, and this was not seen with a real pointer. It is recorded for the lead and was not changed here.
